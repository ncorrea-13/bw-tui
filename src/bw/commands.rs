use super::model::*;
use super::session::clear_cached_session;
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) fn bw_command() -> Command {
    let mut parts = crate::config::get().bw_cmd.split_whitespace();
    let program = parts.next().unwrap_or("bw");
    let mut cmd = Command::new(program);
    cmd.args(parts);
    cmd
}

pub fn unlock(password: &str) -> Result<String> {
    const ENV_VAR: &str = "BW_TUI_PASSWORD";
    let out = bw_command()
        .args(["unlock", "--raw", "--passwordenv", ENV_VAR])
        .env(ENV_VAR, password)
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw unlock`")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("bw unlock failed: {}", stderr.trim());
    }
    let key = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if key.is_empty() {
        bail!("wrong master password");
    }
    Ok(key)
}

pub fn list_items(session: &str) -> Result<Vec<Item>> {
    let out = bw_command()
        .args(["list", "items", "--session", session])
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw list items`")?;
    if !out.status.success() {
        bail!(
            "bw list items failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let items: Vec<Item> =
        serde_json::from_slice(&out.stdout).context("could not parse bw's response")?;
    Ok(items)
}

pub fn create_item(new_item: &NewItem, session: &str) -> Result<Item> {
    let new_item_json = serde_json::to_string(new_item).context("could not encode the new item")?;
    let new_item_base64 = STANDARD.encode(new_item_json);
    let out = bw_command()
        .args(["create", "item", &new_item_base64, "--session", session])
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw create item`")?;
    if !out.status.success() {
        bail!(
            "could not create the item: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    serde_json::from_slice(&out.stdout).context("could not parse the created item")
}

pub fn get_item(id: &str, session: &str) -> Result<serde_json::Value> {
    let out = bw_command()
        .args(["get", "item", id, "--session", session])
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw get item`")?;
    if !out.status.success() {
        bail!(
            "could not get the item: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    serde_json::from_slice(&out.stdout).context("could not parse the item")
}

pub fn edit_item(id: &str, patch: &ItemPatch, session: &str) -> Result<Item> {
    let mut raw_item = get_item(id, session)?;
    let item_fields = raw_item.as_object_mut().context("unexpected item shape from bw")?;

    item_fields.insert("name".to_string(), serde_json::json!(patch.name));
    item_fields.insert("notes".to_string(), serde_json::json!(patch.notes));
    item_fields.insert("folderId".to_string(), serde_json::json!(patch.folder_id));

    if let Some(login_patch) = &patch.login {
        let login_fields = item_fields
            .entry("login".to_string())
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .context("unexpected login shape from bw")?;
        if let Some(username) = &login_patch.username {
            login_fields.insert("username".to_string(), serde_json::json!(username));
        }
        if let Some(password) = &login_patch.password {
            login_fields.insert("password".to_string(), serde_json::json!(password));
        }
    }

    if let Some(card_patch) = &patch.card {
        let card_fields = item_fields
            .entry("card".to_string())
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .context("unexpected card shape from bw")?;
        if let Some(cardholder_name) = &card_patch.cardholder_name {
            card_fields.insert("cardholderName".to_string(), serde_json::json!(cardholder_name));
        }
        if let Some(brand) = &card_patch.brand {
            card_fields.insert("brand".to_string(), serde_json::json!(brand));
        }
        if let Some(number) = &card_patch.number {
            card_fields.insert("number".to_string(), serde_json::json!(number));
        }
        if let Some(exp_month) = &card_patch.exp_month {
            card_fields.insert("expMonth".to_string(), serde_json::json!(exp_month));
        }
        if let Some(exp_year) = &card_patch.exp_year {
            card_fields.insert("expYear".to_string(), serde_json::json!(exp_year));
        }
        if let Some(code) = &card_patch.code {
            card_fields.insert("code".to_string(), serde_json::json!(code));
        }
    }

    if let Some(identity_patch) = &patch.identity {
        let identity_fields = item_fields
            .entry("identity".to_string())
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .context("unexpected identity shape from bw")?;
        if let Some(first_name) = &identity_patch.first_name {
            identity_fields.insert("firstName".to_string(), serde_json::json!(first_name));
        }
        if let Some(last_name) = &identity_patch.last_name {
            identity_fields.insert("lastName".to_string(), serde_json::json!(last_name));
        }
        if let Some(email) = &identity_patch.email {
            identity_fields.insert("email".to_string(), serde_json::json!(email));
        }
        if let Some(phone) = &identity_patch.phone {
            identity_fields.insert("phone".to_string(), serde_json::json!(phone));
        }
    }

    let edited_item_json = serde_json::to_string(&raw_item).context("could not encode the edited item")?;
    let edited_item_base64 = STANDARD.encode(edited_item_json);
    let out = bw_command()
        .args(["edit", "item", id, &edited_item_base64, "--session", session])
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw edit item`")?;
    if !out.status.success() {
        bail!(
            "could not edit the item: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    serde_json::from_slice(&out.stdout).context("could not parse the edited item")
}

pub fn get_password(id: &str, session: &str) -> Result<String> {
    let out = bw_command()
        .args(["get", "password", id, "--session", session])
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw get password`")?;
    if !out.status.success() {
        bail!(
            "could not get the password: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn get_totp(id: &str, session: &str) -> Result<String> {
    let out = bw_command()
        .args(["get", "totp", id, "--session", session])
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw get totp`")?;
    if !out.status.success() {
        bail!(
            "could not get the TOTP code: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn status() -> Result<Status> {
    let out = bw_command()
        .arg("status")
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw status`")?;
    if !out.status.success() {
        bail!(
            "bw status failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    serde_json::from_slice(&out.stdout).context("could not parse `bw status`")
}

pub fn config_server(url: &str) -> Result<()> {
    let out = bw_command()
        .args(["config", "server", url])
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw config server`")?;
    if !out.status.success() {
        bail!(
            "could not configure the server: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

// `bw`'s master-password/two-step prompts open /dev/tty directly instead of
// reading from stdin, so redirecting stdin to null doesn't stop them. Run
// against a raw-mode ratatui screen (which already owns that same tty) and a
// prompt we never intended to trigger just hangs forever with no way to
// answer it. Bound the wait so a stuck prompt surfaces as "needs 2FA"
// (first attempt, no code yet) or a clear timeout error (second attempt)
// instead of freezing the UI.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(20);

pub fn login(email: &str, password: &str, two_factor: Option<(&str, &str)>) -> Result<LoginOutcome> {
    const ENV_VAR: &str = "BW_TUI_PASSWORD";
    let mut args = vec![
        "login".to_string(),
        email.to_string(),
        "--raw".to_string(),
        "--passwordenv".to_string(),
        ENV_VAR.to_string(),
    ];
    if let Some((method, code)) = two_factor {
        args.push("--method".to_string());
        args.push(method.to_string());
        args.push("--code".to_string());
        args.push(code.to_string());
    }

    let mut child = bw_command()
        .args(&args)
        .env(ENV_VAR, password)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("could not run `bw login`")?;

    let deadline = Instant::now() + LOGIN_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().context("could not check on `bw login`")? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            if two_factor.is_none() {
                return Ok(LoginOutcome::TwoFactorRequired);
            }
            bail!("bw login timed out waiting for a response");
        }
        std::thread::sleep(Duration::from_millis(100));
    };

    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut s) = child.stdout.take() {
        let _ = s.read_to_string(&mut stdout);
    }
    if let Some(mut s) = child.stderr.take() {
        let _ = s.read_to_string(&mut stderr);
    }

    if !status.success() {
        let stderr_lower = stderr.to_lowercase();
        if stderr_lower.contains("two-step") || stderr_lower.contains("two factor") || stderr_lower.contains("2fa") {
            return Ok(LoginOutcome::TwoFactorRequired);
        }
        bail!("bw login failed: {}", stderr.trim());
    }
    let key = stdout.trim().to_string();
    if key.is_empty() {
        bail!("could not log in");
    }
    Ok(LoginOutcome::Success(key))
}

pub fn logout() -> Result<()> {
    let out = bw_command()
        .arg("logout")
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw logout`")?;
    if !out.status.success() {
        bail!(
            "could not log out: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    clear_cached_session();
    Ok(())
}

pub fn sync(session: &str) -> Result<()> {
    let out = bw_command()
        .args(["sync", "--session", session])
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw sync`")?;
    if !out.status.success() {
        bail!(
            "could not sync: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

pub fn list_folders(session: &str) -> Result<Vec<Folder>> {
    let out = bw_command()
        .args(["list", "folders", "--session", session])
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw list folders`")?;
    if !out.status.success() {
        bail!(
            "could not list folders: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    serde_json::from_slice(&out.stdout).context("could not parse bw's response")
}

pub fn generate(opts: &GenerateOptions) -> Result<String> {
    let mut args = vec!["generate".to_string(), "--length".to_string(), opts.length.max(5).to_string()];
    if opts.uppercase {
        args.push("--uppercase".to_string());
    }
    if opts.lowercase {
        args.push("--lowercase".to_string());
    }
    if opts.numbers {
        args.push("--number".to_string());
    }
    if opts.special {
        args.push("--special".to_string());
    }
    let out = bw_command()
        .args(&args)
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw generate`")?;
    if !out.status.success() {
        bail!(
            "could not generate a password: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
