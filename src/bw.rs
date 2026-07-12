use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_SESSION_AGE_SECS: u64 = 1200; // 20 minutes, matches reference/bitwarden-tui.sh

#[derive(Debug, Clone, Deserialize)]
pub struct UriData {
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginData {
    pub username: Option<String>,
    #[allow(dead_code)]
    pub password: Option<String>,
    pub totp: Option<String>,
    pub uris: Option<Vec<UriData>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CardData {
    #[serde(rename = "cardholderName")]
    pub cardholder_name: Option<String>,
    pub brand: Option<String>,
    pub number: Option<String>,
    #[serde(rename = "expMonth")]
    pub exp_month: Option<String>,
    #[serde(rename = "expYear")]
    pub exp_year: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityData {
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomField {
    pub name: Option<String>,
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub field_type: u8, // 0 text, 1 hidden, 2 boolean, 3 linked
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: u8,
    pub login: Option<LoginData>,
    pub card: Option<CardData>,
    pub identity: Option<IdentityData>,
    pub fields: Option<Vec<CustomField>>,
    pub notes: Option<String>,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
}

impl Item {
    pub fn username(&self) -> Option<&str> {
        self.login.as_ref()?.username.as_deref()
    }

    pub fn has_totp(&self) -> bool {
        self.login.as_ref().and_then(|l| l.totp.as_ref()).is_some()
    }

    pub fn first_uri(&self) -> Option<&str> {
        self.login.as_ref()?.uris.as_ref()?.first()?.uri.as_deref()
    }

    pub fn type_label(&self) -> &'static str {
        match self.item_type {
            1 => "login",
            2 => "note",
            3 => "card",
            4 => "identity",
            _ => "?",
        }
    }

    pub fn card_summary(&self) -> Option<String> {
        let c = self.card.as_ref()?;
        let brand = c.brand.as_deref().unwrap_or("Card");
        let last4 = c
            .number
            .as_ref()
            .map(|n| n.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<String>())
            .unwrap_or_else(|| "????".to_string());
        let exp = match (&c.exp_month, &c.exp_year) {
            (Some(m), Some(y)) => format!(" (expires {m}/{y})"),
            _ => String::new(),
        };
        let holder = c
            .cardholder_name
            .as_ref()
            .map(|h| format!(" — {h}"))
            .unwrap_or_default();
        Some(format!("{brand} •••• {last4}{exp}{holder}"))
    }

    pub fn identity_summary(&self) -> Option<String> {
        let i = self.identity.as_ref()?;
        let name = [i.first_name.as_deref(), i.last_name.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        let mut parts = vec![];
        if !name.is_empty() {
            parts.push(name);
        }
        if let Some(email) = &i.email {
            parts.push(email.clone());
        }
        if let Some(phone) = &i.phone {
            parts.push(phone.clone());
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" · "))
        }
    }

    pub fn visible_fields(&self) -> Vec<(String, String)> {
        self.fields
            .as_ref()
            .map(|fields| {
                fields
                    .iter()
                    .map(|f| {
                        let name = f.name.clone().unwrap_or_else(|| "(unnamed)".to_string());
                        let value = match f.field_type {
                            1 => "••••••••".to_string(),
                            _ => f.value.clone().unwrap_or_default(),
                        };
                        (name, value)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Folder {
    pub id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Status {
    #[serde(rename = "serverUrl")]
    pub server_url: Option<String>,
    #[serde(rename = "lastSync")]
    pub last_sync: Option<String>,
    #[serde(rename = "userEmail")]
    pub user_email: Option<String>,
    pub status: String, // "unauthenticated" | "locked" | "unlocked"
}

pub enum LoginOutcome {
    Success(String),
    TwoFactorRequired,
}

#[derive(Debug, Clone)]
pub struct GenerateOptions {
    pub length: u8,
    pub uppercase: bool,
    pub lowercase: bool,
    pub numbers: bool,
    pub special: bool,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            length: 20,
            uppercase: true,
            lowercase: true,
            numbers: true,
            special: false,
        }
    }
}

fn cache_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME is not set");
    PathBuf::from(home).join(".cache")
}

fn session_file() -> PathBuf {
    cache_dir().join("bw_session")
}

fn session_time_file() -> PathBuf {
    cache_dir().join("bw_session_time")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Reads the cached session from disk if present and not older than
/// MAX_SESSION_AGE_SECS. Does not validate against the bw server; callers
/// should confirm with `list_items`.
pub fn load_cached_session() -> Option<(String, u64)> {
    let key = std::fs::read_to_string(session_file()).ok()?;
    let key = key.trim().to_string();
    if key.is_empty() {
        return None;
    }
    let ts: u64 = std::fs::read_to_string(session_time_file())
        .ok()?
        .trim()
        .parse()
        .ok()?;
    if now_secs().saturating_sub(ts) > MAX_SESSION_AGE_SECS {
        return None;
    }
    Some((key, ts))
}

pub fn clear_cached_session() {
    let _ = Command::new("bw")
        .arg("lock")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = std::fs::remove_file(session_file());
    let _ = std::fs::remove_file(session_time_file());
}

pub fn save_session(key: &str) -> Result<u64> {
    let ts = now_secs();
    let path = session_file();
    let mut f = std::fs::File::create(&path).context("could not create the session file")?;
    f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    f.write_all(key.as_bytes())?;
    std::fs::write(session_time_file(), ts.to_string())?;
    Ok(ts)
}

/// Unlocks the vault using the master password, without ever putting the
/// password on the command line (visible via `ps`). Returns the raw session key.
pub fn unlock(password: &str) -> Result<String> {
    const ENV_VAR: &str = "BW_TUI_PASSWORD";
    let out = Command::new("bw")
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
    let out = Command::new("bw")
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

pub fn get_password(id: &str, session: &str) -> Result<String> {
    let out = Command::new("bw")
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
    let out = Command::new("bw")
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

/// Reads `bw status`, which reflects login state from bw's own config, not
/// our session cache. Used to decide between the server-config, login, and
/// unlock screens at startup.
pub fn status() -> Result<Status> {
    let out = Command::new("bw")
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
    let out = Command::new("bw")
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

/// Full login (email + master password, optionally with a 2FA method/code),
/// as opposed to `unlock` which assumes the CLI is already authenticated.
/// `two_factor` is (method, code), e.g. ("0", "123456") for an authenticator app.
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

    let out = Command::new("bw")
        .args(&args)
        .env(ENV_VAR, password)
        .stdin(Stdio::null())
        .output()
        .context("could not run `bw login`")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
        if stderr.contains("two-step") || stderr.contains("two factor") || stderr.contains("2fa") {
            return Ok(LoginOutcome::TwoFactorRequired);
        }
        bail!(
            "bw login failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let key = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if key.is_empty() {
        bail!("could not log in");
    }
    Ok(LoginOutcome::Success(key))
}

pub fn logout() -> Result<()> {
    let out = Command::new("bw")
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
    let out = Command::new("bw")
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
    let out = Command::new("bw")
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
    let out = Command::new("bw")
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
