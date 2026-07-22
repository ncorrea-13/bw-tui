use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

static IS_WSL: OnceLock<bool> = OnceLock::new();
static HAS_WIN32YANK: OnceLock<bool> = OnceLock::new();

pub fn is_wsl() -> bool {
    *IS_WSL.get_or_init(|| {
        if std::env::var_os("WSL_DISTRO_NAME").is_some()
            || std::env::var_os("WSL_INTEROP").is_some()
        {
            return true;
        }
        std::fs::read_to_string("/proc/version")
            .map(|v| v.to_ascii_lowercase().contains("microsoft"))
            .unwrap_or(false)
    })
}

fn has_win32yank() -> bool {
    *HAS_WIN32YANK.get_or_init(|| {
        Command::new("win32yank.exe")
            .arg("-h")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    })
}

pub fn copy(text: &str) -> Result<()> {
    let (program, args, hint): (&str, &[&str], &str) = if is_wsl() {
        if has_win32yank() {
            ("win32yank.exe", &["-i"], "is win32yank.exe on PATH?")
        } else {
            ("clip.exe", &[], "is Windows interop enabled?")
        }
    } else {
        ("wl-copy", &[], "are you in a Wayland session?")
    };
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| format!("could not run {program} ({hint})"))?;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(text.as_bytes())
        .with_context(|| format!("could not write to {program}"))?;
    child.wait().with_context(|| format!("{program} failed"))?;
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
    if is_wsl() {
        if !has_win32yank() {
            return None;
        }
        let out = Command::new("win32yank.exe")
            .arg("-o")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        return Some(
            String::from_utf8_lossy(&out.stdout)
                .trim_end_matches(['\r', '\n'])
                .to_string(),
        );
    }
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
    if is_wsl() && !has_win32yank() {
        String::new()
    } else {
        format!(" (clears in {secs}s)")
    }
}

pub fn spawn_autoclear(secret: String, label: &'static str) {
    if is_wsl() && !has_win32yank() {
        return;
    }
    let autoclear_secs = crate::config::get().clipboard_clear_secs;
    thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(autoclear_secs);
        // cliphist is a Wayland clipboard-history tool; there's no equivalent
        // to purge from Windows' native history, so it's skipped under WSL.
        let use_cliphist = !is_wsl();
        let mut deleted_from_history = false;
        while std::time::Instant::now() < deadline {
            if use_cliphist && !deleted_from_history && cliphist_contains(&secret) {
                cliphist_delete(&secret);
                deleted_from_history = true;
            }
            thread::sleep(Duration::from_millis(300));
        }
        if use_cliphist {
            cliphist_delete(&secret);
        }
        if current_clipboard().as_deref() == Some(secret.as_str()) {
            if is_wsl() {
                if let Ok(mut child) = Command::new("win32yank.exe")
                    .arg("-i")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    drop(child.stdin.take());
                    let _ = child.wait();
                }
            } else {
                let _ = Command::new("wl-copy")
                    .arg("--clear")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            notify(&format!("🧹 Clipboard cleared ({label})."));
        }
    });
}
