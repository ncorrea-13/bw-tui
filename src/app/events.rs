use super::{App, LoginField, Screen, TwoFactorMethod};
use crate::bw;
use crate::clipboard;

pub enum BwEvent {
    Started(bw::StartOutcome),
    ServerConfigured { url: String, result: anyhow::Result<()> },
    LoggedIn(anyhow::Result<bw::LoginFlowResult>),
    Unlocked(anyhow::Result<bw::VaultLoad>),
    ItemsRefreshed(anyhow::Result<bw::ItemsLoad>),
    PasswordCopied { item_name: String, result: anyhow::Result<String> },
    TotpCopied { item_name: String, result: anyhow::Result<String> },
    Revealed { item_id: String, result: anyhow::Result<String> },
    Generated(anyhow::Result<String>),
    Synced(anyhow::Result<bw::SyncLoad>),
    LoggedOut(anyhow::Result<bw::StartOutcome>),
    ItemCreated(anyhow::Result<bw::Item>),
}

impl App {
    pub(super) fn apply_bw_event(&mut self, event: BwEvent) {
        match event {
            BwEvent::Started(outcome) => self.apply_start_outcome(outcome),
            BwEvent::ServerConfigured { url, result } => match result {
                Ok(()) => {
                    self.screen = Screen::Login {
                        email: String::new(),
                        password: String::new(),
                        focus: LoginField::Email,
                        awaiting_2fa: false,
                        code: String::new(),
                        method: TwoFactorMethod::Authenticator,
                        error: None,
                        busy: false,
                    };
                }
                Err(e) => {
                    self.screen = Screen::ServerConfig {
                        url,
                        error: Some(e.to_string()),
                        busy: false,
                    };
                }
            },
            BwEvent::LoggedIn(result) => {
                if !matches!(self.screen, Screen::Login { .. }) {
                    return;
                }
                match result {
                    Ok(bw::LoginFlowResult::LoggedIn(load)) => {
                        self.enter_vault(load.key, load.ts, load.items, load.folders);
                        self.set_status("🔓 Logged in");
                    }
                    Ok(bw::LoginFlowResult::TwoFactorRequired) => {
                        if let Screen::Login { focus, awaiting_2fa, code, error, busy, .. } = &mut self.screen {
                            *focus = LoginField::Password;
                            *awaiting_2fa = true;
                            code.clear();
                            *error = Some("Enter the verification code".to_string());
                            *busy = false;
                        }
                    }
                    Err(e) => {
                        if let Screen::Login { password, focus, awaiting_2fa, code, method, error, busy, .. } = &mut self.screen {
                            password.clear();
                            *focus = LoginField::Password;
                            *awaiting_2fa = false;
                            code.clear();
                            *method = TwoFactorMethod::Authenticator;
                            *error = Some(e.to_string());
                            *busy = false;
                        }
                    }
                }
            }
            BwEvent::Unlocked(result) => {
                if !matches!(self.screen, Screen::Unlock { .. }) {
                    return;
                }
                match result {
                    Ok(load) => {
                        self.enter_vault(load.key, load.ts, load.items, load.folders);
                        self.set_status("🔓 Vault unlocked");
                    }
                    Err(e) => {
                        if let Screen::Unlock { password, error, busy, .. } = &mut self.screen {
                            password.clear();
                            *error = Some(e.to_string());
                            *busy = false;
                        }
                    }
                }
            }
            BwEvent::ItemsRefreshed(result) => {
                self.busy = false;
                self.busy_label = None;
                match result {
                    Ok(load) => {
                        self.items = load.items;
                        self.folders = load.folders;
                        self.refilter();
                        self.set_status("🔄 List refreshed");
                    }
                    Err(e) => self.set_status(format!("⚠️ Could not refresh: {e}")),
                }
            }
            BwEvent::PasswordCopied { item_name, result } => {
                self.busy = false;
                self.busy_label = None;
                match result {
                    Ok(pw) if !pw.is_empty() => {
                        if let Err(e) = clipboard::copy(&pw) {
                            self.set_status(format!("⚠️ {e}"));
                            return;
                        }
                        clipboard::notify(&format!("✅ Password copied: {item_name}"));
                        let secs = crate::config::get().clipboard_clear_secs;
                        let note = clipboard::autoclear_note(secs);
                        self.set_status(format!("✅ Password for '{item_name}' copied{note}"));
                        clipboard::spawn_autoclear(pw, "password");
                    }
                    Ok(_) => self.set_status("⚠️ This item has no password"),
                    Err(e) => self.set_status(format!("⚠️ {e}")),
                }
            }
            BwEvent::TotpCopied { item_name, result } => {
                self.busy = false;
                self.busy_label = None;
                match result {
                    Ok(code) if !code.is_empty() => {
                        if let Err(e) = clipboard::copy(&code) {
                            self.set_status(format!("⚠️ {e}"));
                            return;
                        }
                        let secs = crate::config::get().clipboard_clear_secs;
                        let note = clipboard::autoclear_note(secs);
                        self.set_status(format!("✅ TOTP code for '{item_name}' copied{note}"));
                        clipboard::spawn_autoclear(code, "TOTP");
                    }
                    Ok(_) => self.set_status("⚠️ Could not generate the TOTP code"),
                    Err(e) => self.set_status(format!("⚠️ {e}")),
                }
            }
            BwEvent::Revealed { item_id, result } => {
                self.busy = false;
                self.busy_label = None;
                match result {
                    Ok(pw) => self.reveal = Some((item_id, pw)),
                    Err(e) => self.set_status(format!("⚠️ {e}")),
                }
            }
            BwEvent::Generated(result) => {
                self.busy = false;
                self.busy_label = None;
                match result {
                    Ok(pw) => {
                        self.generator.result = Some(pw);
                        self.generator.error = None;
                    }
                    Err(e) => self.generator.error = Some(e.to_string()),
                }
            }
            BwEvent::Synced(result) => {
                self.busy = false;
                self.busy_label = None;
                match result {
                    Ok(load) => {
                        if let Some(status) = load.status {
                            self.server_status = Some(status);
                        }
                        self.items = load.items;
                        self.folders = load.folders;
                        self.refilter();
                        self.set_status("🔄 Synced with server");
                    }
                    Err(e) => self.set_status(format!("⚠️ {e}")),
                }
            }
            BwEvent::LoggedOut(result) => {
                self.busy = false;
                self.busy_label = None;
                match result {
                    Ok(outcome) => self.apply_start_outcome(outcome),
                    Err(e) => self.set_status(format!("⚠️ {e}")),
                }
            }
            BwEvent::ItemCreated(result) => {
                self.busy = false;
                self.busy_label = None;
                match result {
                    Ok(item) => {
                        let created_id = item.id.clone();
                        self.items.push(item);
                        self.refilter();
                        if let Some(pos) = self.filtered.iter().position(|&i| self.items[i].id == created_id) {
                            self.selected = pos;
                        }
                        self.item_form = None;
                        self.set_status("✅ Item created");
                    }
                    Err(e) => {
                        if let Some(form) = &mut self.item_form {
                            form.error = Some(e.to_string());
                        }
                    }
                }
            }
        }
    }
}
