use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn bw_command() -> Command {
    let mut parts = crate::config::get().bw_cmd.split_whitespace();
    let program = parts.next().unwrap_or("bw");
    let mut cmd = Command::new(program);
    cmd.args(parts);
    cmd
}

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
    pub code: Option<String>,
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
    pub field_type: u8,
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

// ---- Create/edit payloads -------------------------------------------------

#[derive(Debug, Clone, Default, Serialize)]
pub struct NewLogin {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct NewCard {
    #[serde(rename = "cardholderName")]
    pub cardholder_name: Option<String>,
    pub brand: Option<String>,
    pub number: Option<String>,
    #[serde(rename = "expMonth")]
    pub exp_month: Option<String>,
    #[serde(rename = "expYear")]
    pub exp_year: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecureNoteData {
    #[serde(rename = "type")]
    pub note_type: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct NewItem {
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
    #[serde(rename = "type")]
    pub item_type: u8,
    pub name: String,
    pub notes: Option<String>,
    pub login: Option<NewLogin>,
    pub card: Option<NewCard>,
    #[serde(rename = "secureNote")]
    pub secure_note: Option<SecureNoteData>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemPatch {
    pub name: String,
    pub notes: Option<String>,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
    pub login: Option<NewLogin>,
    pub card: Option<NewCard>,
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
    pub status: String,
}

pub enum LoginOutcome {
    Success(String),
    TwoFactorRequired,
}

pub struct VaultLoad {
    pub key: String,
    pub ts: u64,
    pub items: Vec<Item>,
    pub folders: Vec<Folder>,
}

pub struct ItemsLoad {
    pub items: Vec<Item>,
    pub folders: Vec<Folder>,
}

pub struct SyncLoad {
    pub status: Option<Status>,
    pub items: Vec<Item>,
    pub folders: Vec<Folder>,
}

pub enum LoginFlowResult {
    LoggedIn(VaultLoad),
    TwoFactorRequired,
}

pub enum StartOutcome {
    Vault(VaultLoad),
    NeedsServerConfig(Status),
    NeedsUnlock(Status),
    Error(anyhow::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg).join("bw-tui");
    }
    let home = std::env::var("HOME").expect("HOME is not set");
    PathBuf::from(home).join(".cache").join("bw-tui")
}

fn session_file() -> PathBuf {
    cache_dir().join("session")
}

fn session_time_file() -> PathBuf {
    cache_dir().join("session_time")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

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
    if now_secs().saturating_sub(ts) > crate::config::get().session_max_age_secs {
        return None;
    }
    Some((key, ts))
}

pub fn clear_cached_session() {
    let _ = bw_command()
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
    std::fs::create_dir_all(cache_dir()).context("could not create the cache directory")?;
    let mut f = std::fs::File::create(&path).context("could not create the session file")?;
    f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    f.write_all(key.as_bytes())?;
    std::fs::write(session_time_file(), ts.to_string())?;
    Ok(ts)
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

    let out = bw_command()
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

// ---- Chained operations ---------------------------------------------------
//
// Each of these glues together several blocking `bw` invocations into one
// call that a background thread can run start-to-finish, so the UI thread
// only ever waits on a channel instead of a subprocess.

pub fn login_and_load(
    email: &str,
    password: &str,
    two_factor: Option<(&str, &str)>,
) -> Result<LoginFlowResult> {
    match login(email, password, two_factor)? {
        LoginOutcome::TwoFactorRequired => Ok(LoginFlowResult::TwoFactorRequired),
        LoginOutcome::Success(key) => {
            let ts = save_session(&key)?;
            let items = list_items(&key)?;
            let folders = list_folders(&key).unwrap_or_default();
            Ok(LoginFlowResult::LoggedIn(VaultLoad { key, ts, items, folders }))
        }
    }
}

pub fn unlock_and_load(password: &str) -> Result<VaultLoad> {
    let key = unlock(password)?;
    let ts = save_session(&key)?;
    let items = list_items(&key)?;
    let folders = list_folders(&key).unwrap_or_default();
    Ok(VaultLoad { key, ts, items, folders })
}

pub fn refresh_items(session: &str) -> Result<ItemsLoad> {
    let items = list_items(session)?;
    let folders = list_folders(session).unwrap_or_default();
    Ok(ItemsLoad { items, folders })
}

pub fn sync_and_refresh(session: &str) -> Result<SyncLoad> {
    sync(session)?;
    let status = status().ok();
    let items = list_items(session)?;
    let folders = list_folders(session).unwrap_or_default();
    Ok(SyncLoad { status, items, folders })
}

pub fn compute_start() -> StartOutcome {
    if let Some((key, ts)) = load_cached_session() {
        if let Ok(items) = list_items(&key) {
            let folders = list_folders(&key).unwrap_or_default();
            return StartOutcome::Vault(VaultLoad { key, ts, items, folders });
        }
        clear_cached_session();
    }

    match status() {
        Ok(s) if s.status == "unauthenticated" => StartOutcome::NeedsServerConfig(s),
        Ok(s) => StartOutcome::NeedsUnlock(s),
        Err(e) => StartOutcome::Error(e),
    }
}

pub fn logout_and_restart() -> Result<StartOutcome> {
    logout()?;
    Ok(compute_start())
}
