use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const AUTOCLEAR_SECS: u64 = 9; // matches reference/bitwarden-tui.sh

pub fn copy(text: &str) -> Result<()> {
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

/// Spawns a background thread that, mirroring reference/bitwarden-tui.sh,
/// scrubs `secret` out of cliphist as soon as it lands there and then, after
/// AUTOCLEAR_SECS, clears the live clipboard if it still holds the secret.
/// Runs off the UI thread so the TUI stays interactive while it waits.
pub fn spawn_autoclear(secret: String, label: &'static str) {
    thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(AUTOCLEAR_SECS);
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
