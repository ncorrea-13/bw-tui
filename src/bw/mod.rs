mod commands;
mod model;
mod session;

pub use commands::*;
pub use model::*;
pub use session::*;

use anyhow::Result;

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
            let status = status().ok();
            Ok(LoginFlowResult::LoggedIn(VaultLoad {
                key,
                ts,
                items,
                folders,
                status,
            }))
        }
    }
}

pub fn unlock_and_load(password: &str) -> Result<VaultLoad> {
    let key = unlock(password)?;
    let ts = save_session(&key)?;
    let items = list_items(&key)?;
    let folders = list_folders(&key).unwrap_or_default();
    let status = status().ok();
    Ok(VaultLoad {
        key,
        ts,
        items,
        folders,
        status,
    })
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
    Ok(SyncLoad {
        status,
        items,
        folders,
    })
}

pub fn compute_start() -> StartOutcome {
    if let Some((key, ts)) = load_cached_session() {
        if let Ok(items) = list_items(&key) {
            let folders = list_folders(&key).unwrap_or_default();
            let status = status().ok();
            return StartOutcome::Vault(VaultLoad {
                key,
                ts,
                items,
                folders,
                status,
            });
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
