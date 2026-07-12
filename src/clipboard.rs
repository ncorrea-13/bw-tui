use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

static IS_WSL: OnceLock<bool> = OnceLock::new();

pub fn is_wsl() -> bool {
    *IS_WSL.get_or_init(|| {
        if std::env::var_os("WSL_DISTRO_NAME").is_some() || std::env::var_os("WSL_INTEROP").is_some()
        {
            return true;
        }
        std::fs::read_to_string("/proc/version")
            .map(|v| v.to_ascii_lowercase().contains("microsoft"))
            .unwrap_or(false)
    })
}

pub fn copy(text: &str) -> Result<()> {
    if is_wsl() {
        let mut child = Command::new("clip.exe")
            .stdin(Stdio::piped())
            .spawn()
            .context("could not run clip.exe (is Windows interop enabled?)")?;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(text.as_bytes())
            .context("could not write to clip.exe")?;
        child.wait().context("clip.exe failed")?;
        return Ok(());
    }
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .context("could not run wl-copy (are you in a Wayland session?)")?;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(text.as_bytes())
        .context("could not write to wl-copy")?;
    child.wait().context("wl-copy failed")?;
    Ok(())
}

pub fn notify(message: &str) {
    let _ = Command::new("notify-send")
        .arg(message)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn current_clipboard() -> Option<String> {
    let out = Command::new("wl-paste")
        .arg("-n")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

fn cliphist_contains(text: &str) -> bool {
    let Ok(out) = Command::new("cliphist").arg("list").output() else {
        return false;
    };
    String::from_utf8_lossy(&out.stdout).contains(text)
}

fn cliphist_delete(text: &str) {
    let _ = Command::new("cliphist")
        .args(["delete-query", text])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

pub fn autoclear_note(secs: u64) -> String {
    if is_wsl() {
        String::new()
    } else {
        format!(" (clears in {secs}s)")
    }
}

pub fn spawn_autoclear(secret: String, label: &'static str) {
    if is_wsl() {
        return;
    }
    let autoclear_secs = crate::config::get().clipboard_clear_secs;
    thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(autoclear_secs);
        let mut deleted_from_history = false;
        while std::time::Instant::now() < deadline {
            if !deleted_from_history && cliphist_contains(&secret) {
                cliphist_delete(&secret);
                deleted_from_history = true;
            }
            thread::sleep(Duration::from_millis(300));
        }
        cliphist_delete(&secret);
        if current_clipboard().as_deref() == Some(secret.as_str()) {
            let _ = Command::new("wl-copy")
                .arg("--clear")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            notify(&format!("🧹 Clipboard cleared ({label})."));
        }
    });
}
