// Copyright (c) 2026 Nicolás Correa
// SPDX-License-Identifier: GPL-3.0-or-later

use super::commands::bw_command;
use anyhow::{Context, Result};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
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

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
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
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(cache_dir())
        .context("could not create the cache directory")?;
    // mode() applies at creation, so the token is never briefly world-readable.
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .context("could not create the session file")?;
    f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    f.write_all(key.as_bytes())?;
    std::fs::write(session_time_file(), ts.to_string())?;
    Ok(ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_session_is_private_from_creation() {
        let dir = std::env::temp_dir().join(format!("bw-tui-test-{}", std::process::id()));
        // SAFETY: only test touching XDG_CACHE_HOME; no other thread reads it concurrently.
        unsafe { std::env::set_var("XDG_CACHE_HOME", &dir) };
        save_session("tok").expect("save session");
        let mode =
            |p: &std::path::Path| std::fs::metadata(p).expect("stat").permissions().mode() & 0o777;
        assert_eq!(mode(&dir.join("bw-tui")), 0o700);
        assert_eq!(mode(&session_file()), 0o600);
        assert_eq!(
            std::fs::read_to_string(session_file()).expect("read session"),
            "tok"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
