use super::commands::bw_command;
use anyhow::{Context, Result};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

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
    #[allow(clippy::unwrap_used, reason = "system clock is never before UNIX_EPOCH")]
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
