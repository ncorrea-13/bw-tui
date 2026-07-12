use crate::bw::{self, Folder, GenerateOptions, Item, LoginOutcome, Status, MAX_SESSION_AGE_SECS};
use crate::clipboard;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq)]
pub enum LoginField {
    Email,
    Password,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TwoFactorMethod {
    Authenticator,
    Email,
}

impl TwoFactorMethod {
    fn code(self) -> &'static str {
        match self {
            TwoFactorMethod::Authenticator => "0",
            TwoFactorMethod::Email => "1",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TwoFactorMethod::Authenticator => "Authenticator app",
            TwoFactorMethod::Email => "Email",
        }
    }

    fn toggled(self) -> Self {
        match self {
            TwoFactorMethod::Authenticator => TwoFactorMethod::Email,
            TwoFactorMethod::Email => TwoFactorMethod::Authenticator,
        }
    }
}

pub enum Screen {
    ServerConfig {
        url: String,
        error: Option<String>,
        busy: bool,
    },
    Login {
        email: String,
        password: String,
        focus: LoginField,
        awaiting_2fa: bool,
        code: String,
        method: TwoFactorMethod,
        error: Option<String>,
        busy: bool,
    },
    /// Vault is already authenticated (bw status == "locked") — just needs the
    /// master password, same as reference/bitwarden-tui.sh's `bw unlock`.
    Unlock {
        email: Option<String>,
        password: String,
        error: Option<String>,
        busy: bool,
        relock_message: Option<String>,
    },
    Main,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Vault,
    Generator,
    Account,
}

impl Tab {
    fn next(self) -> Self {
        match self {
            Tab::Vault => Tab::Generator,
            Tab::Generator => Tab::Account,
            Tab::Account => Tab::Vault,
        }
    }
    fn prev(self) -> Self {
        match self {
            Tab::Vault => Tab::Account,
            Tab::Generator => Tab::Vault,
            Tab::Account => Tab::Generator,
        }
    }
}

/// Vault tab input mode, vim-style: Normal for single-key motions/actions,
/// Search (entered with `/`) for typing into the fuzzy filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VaultMode {
    Normal,
    Search,
}

#[derive(Default)]
pub struct GeneratorState {
    pub opts: GenerateOptions,
    pub result: Option<String>,
    pub error: Option<String>,
}


pub struct StatusMsg {
    pub text: String,
    pub shown_at: Instant,
}

pub struct App {
    pub screen: Screen,
    pub tab: Tab,
    pub session: Option<String>,
    pub session_started: u64,
    pub items: Vec<Item>,
    pub filtered: Vec<usize>,
    pub folders: Vec<Folder>,
    /// 0 = "All", 1 = "No folder", 2.. = folders[i-2]
    pub folder_index: usize,
    pub vault_mode: VaultMode,
    pending_g: bool,
    pub query: String,
    pub selected: usize,
    pub status: Option<StatusMsg>,
    pub should_quit: bool,
    pub reveal: Option<(String, String)>,
    pub server_status: Option<Status>,
    pub generator: GeneratorState,
    pub confirm_logout: bool,
    matcher: SkimMatcherV2,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Unlock {
                email: None,
                password: String::new(),
                error: None,
                busy: false,
                relock_message: None,
            },
            tab: Tab::Vault,
            session: None,
            session_started: 0,
            items: Vec::new(),
            filtered: Vec::new(),
            folders: Vec::new(),
            folder_index: 0,
            vault_mode: VaultMode::Normal,
            pending_g: false,
            query: String::new(),
            selected: 0,
            status: None,
            should_quit: false,
            reveal: None,
            server_status: None,
            generator: GeneratorState::default(),
            confirm_logout: false,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Decides the first screen: reuse a still-valid cached session (same
    /// files reference/bitwarden-tui.sh writes), or ask `bw status` whether
    /// we need server config + login, or just an unlock.
    pub fn start(&mut self) {
        if let Some((key, ts)) = bw::load_cached_session() {
            if let Ok(items) = bw::list_items(&key) {
                self.enter_vault(key, ts, items);
                return;
            }
            bw::clear_cached_session();
        }

        match bw::status() {
            Ok(status) => {
                self.screen = match status.status.as_str() {
                    "unauthenticated" => Screen::ServerConfig {
                        url: status.server_url.clone().unwrap_or_default(),
                        error: None,
                        busy: false,
                    },
                    _ => Screen::Unlock {
                        email: status.user_email.clone(),
                        password: String::new(),
                        error: None,
                        busy: false,
                        relock_message: None,
                    },
                };
                self.server_status = Some(status);
            }
            Err(e) => self.set_status(format!("⚠️ {e}")),
        }
    }

    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status = Some(StatusMsg {
            text: text.into(),
            shown_at: Instant::now(),
        });
    }

    pub fn on_tick(&mut self) {
        if let Some(s) = &self.status
            && s.shown_at.elapsed() > Duration::from_secs(4) {
                self.status = None;
            }
        if matches!(self.screen, Screen::Main) && self.session_age() > MAX_SESSION_AGE_SECS {
            self.relock("🔒 Session expired, enter your master password again:");
        }
    }

    pub fn session_age(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now.saturating_sub(self.session_started)
    }

    pub fn session_remaining(&self) -> u64 {
        MAX_SESSION_AGE_SECS.saturating_sub(self.session_age())
    }

    fn enter_vault(&mut self, key: String, ts: u64, items: Vec<Item>) {
        self.folders = bw::list_folders(&key).unwrap_or_default();
        self.session = Some(key);
        self.session_started = ts;
        self.items = items;
        self.folder_index = 0;
        self.vault_mode = VaultMode::Normal;
        self.reveal = None;
        self.refilter();
        self.screen = Screen::Main;
        self.tab = Tab::Vault;
    }

    fn relock(&mut self, message: &str) {
        let email = self.server_status.as_ref().and_then(|s| s.user_email.clone());
        bw::clear_cached_session();
        self.session = None;
        self.items.clear();
        self.filtered.clear();
        self.folders.clear();
        self.query.clear();
        self.reveal = None;
        self.screen = Screen::Unlock {
            email,
            password: String::new(),
            error: None,
            busy: false,
            relock_message: Some(message.to_string()),
        };
    }

    // ---- Server config -------------------------------------------------

    fn confirm_server_config(&mut self) {
        let Screen::ServerConfig { url, .. } = &self.screen else {
            return;
        };
        let url = url.trim().to_string();
        if !url.is_empty() {
            if let Screen::ServerConfig { busy, .. } = &mut self.screen {
                *busy = true;
            }
            if let Err(e) = bw::config_server(&url) {
                self.screen = Screen::ServerConfig {
                    url,
                    error: Some(e.to_string()),
                    busy: false,
                };
                return;
            }
        }
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

    // ---- Login / unlock --------------------------------------------------

    fn try_login(&mut self) {
        let Screen::Login {
            email,
            password,
            awaiting_2fa,
            code,
            method,
            ..
        } = &self.screen
        else {
            return;
        };
        let email = email.clone();
        let password = password.clone();
        let method = *method;
        let two_factor = awaiting_2fa.then(|| (method.code().to_string(), code.clone()));

        if let Screen::Login { busy, .. } = &mut self.screen {
            *busy = true;
        }

        let tf_ref = two_factor.as_ref().map(|(m, c)| (m.as_str(), c.as_str()));
        let result = bw::login(&email, &password, tf_ref);

        match result {
            Ok(LoginOutcome::Success(key)) => {
                let outcome = bw::save_session(&key).and_then(|ts| {
                    let items = bw::list_items(&key)?;
                    Ok((ts, items))
                });
                match outcome {
                    Ok((ts, items)) => {
                        self.enter_vault(key, ts, items);
                        self.set_status("🔓 Logged in");
                    }
                    Err(e) => {
                        self.screen = Screen::Login {
                            email,
                            password: String::new(),
                            focus: LoginField::Password,
                            awaiting_2fa: false,
                            code: String::new(),
                            method: TwoFactorMethod::Authenticator,
                            error: Some(e.to_string()),
                            busy: false,
                        };
                    }
                }
            }
            Ok(LoginOutcome::TwoFactorRequired) => {
                self.screen = Screen::Login {
                    email,
                    password,
                    focus: LoginField::Password,
                    awaiting_2fa: true,
                    code: String::new(),
                    method,
                    error: Some("Enter the verification code".to_string()),
                    busy: false,
                };
            }
            Err(e) => {
                self.screen = Screen::Login {
                    email,
                    password: String::new(),
                    focus: LoginField::Password,
                    awaiting_2fa: false,
                    code: String::new(),
                    method: TwoFactorMethod::Authenticator,
                    error: Some(e.to_string()),
                    busy: false,
                };
            }
        }
    }

    fn try_unlock(&mut self) {
        let Screen::Unlock { password, .. } = &self.screen else {
            return;
        };
        let password = password.clone();
        if password.is_empty() {
            return;
        }
        if let Screen::Unlock { busy, .. } = &mut self.screen {
            *busy = true;
        }

        let result = bw::unlock(&password).and_then(|key| {
            let ts = bw::save_session(&key)?;
            let items = bw::list_items(&key)?;
            Ok((key, ts, items))
        });

        match result {
            Ok((key, ts, items)) => {
                self.enter_vault(key, ts, items);
                self.set_status("🔓 Vault unlocked");
            }
            Err(e) => {
                let email = if let Screen::Unlock { email, .. } = &self.screen {
                    email.clone()
                } else {
                    None
                };
                self.screen = Screen::Unlock {
                    email,
                    password: String::new(),
                    error: Some(e.to_string()),
                    busy: false,
                    relock_message: None,
                };
            }
        }
    }

    // ---- Vault tab ---------------------------------------------------

    pub fn refresh_items(&mut self) {
        let Some(session) = self.session.clone() else {
            return;
        };
        match bw::list_items(&session) {
            Ok(items) => {
                self.items = items;
                self.folders = bw::list_folders(&session).unwrap_or_default();
                self.refilter();
                self.set_status("🔄 List refreshed");
            }
            Err(e) => self.set_status(format!("⚠️ Could not refresh: {e}")),
        }
    }

    pub fn cycle_folder(&mut self, delta: i32) {
        let total = self.folders.len() + 2; // All + No folder + N folders
        let new = (self.folder_index as i32 + delta).rem_euclid(total as i32);
        self.folder_index = new as usize;
        self.refilter();
    }

    pub fn folder_label(&self, index: usize) -> String {
        match index {
            0 => "All".to_string(),
            1 => "No folder".to_string(),
            i => self
                .folders
                .get(i - 2)
                .map(|f| f.name.clone())
                .unwrap_or_default(),
        }
    }

    pub fn refilter(&mut self) {
        let folder_index = self.folder_index;
        let folder_id_filter: Option<Option<&str>> = match folder_index {
            0 => None,
            1 => Some(None),
            i => self.folders.get(i - 2).map(|f| Some(f.id.as_deref().unwrap_or(""))),
        };

        let candidates = self.items.iter().enumerate().filter(|(_, item)| match folder_id_filter {
            None => true,
            Some(None) => item.folder_id.is_none(),
            Some(Some(id)) => item.folder_id.as_deref() == Some(id),
        });

        if self.query.is_empty() {
            self.filtered = candidates.map(|(i, _)| i).collect();
        } else {
            let mut scored: Vec<(i64, usize)> = candidates
                .filter_map(|(i, item)| {
                    let haystack = match item.username() {
                        Some(u) => format!("{} {}", item.name, u),
                        None => item.name.clone(),
                    };
                    self.matcher
                        .fuzzy_match(&haystack, &self.query)
                        .map(|score| (score, i))
                })
                .collect();
            scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
            self.filtered = scored.into_iter().map(|(_, i)| i).collect();
        }
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
        self.reveal = None;
    }

    pub fn selected_item(&self) -> Option<&Item> {
        self.filtered.get(self.selected).map(|&i| &self.items[i])
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.filtered.is_empty() {
            return;
        }
        let len = self.filtered.len() as i32;
        let new = (self.selected as i32 + delta).rem_euclid(len);
        self.selected = new as usize;
        self.reveal = None;
    }

    pub fn copy_password(&mut self) {
        let Some(session) = self.session.clone() else {
            return;
        };
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        match bw::get_password(&item.id, &session) {
            Ok(pw) if !pw.is_empty() => {
                if let Err(e) = clipboard::copy(&pw) {
                    self.set_status(format!("⚠️ {e}"));
                    return;
                }
                clipboard::notify(&format!("✅ Password copied: {}", item.name));
                self.set_status(format!(
                    "✅ Password for '{}' copied (clears in 9s)",
                    item.name
                ));
                clipboard::spawn_autoclear(pw, "password");
            }
            Ok(_) => self.set_status("⚠️ This item has no password"),
            Err(e) => self.set_status(format!("⚠️ {e}")),
        }
    }

    pub fn copy_username(&mut self) {
        let Some(item) = self.selected_item() else {
            return;
        };
        let Some(username) = item.username().map(|s| s.to_string()) else {
            self.set_status("⚠️ This item has no username");
            return;
        };
        let name = item.name.clone();
        if let Err(e) = clipboard::copy(&username) {
            self.set_status(format!("⚠️ {e}"));
            return;
        }
        self.set_status(format!("✅ Username for '{name}' copied"));
    }

    pub fn copy_totp(&mut self) {
        let Some(session) = self.session.clone() else {
            return;
        };
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        if !item.has_totp() {
            self.set_status("⚠️ This item has no TOTP");
            return;
        }
        match bw::get_totp(&item.id, &session) {
            Ok(code) if !code.is_empty() => {
                if let Err(e) = clipboard::copy(&code) {
                    self.set_status(format!("⚠️ {e}"));
                    return;
                }
                self.set_status(format!(
                    "✅ TOTP code for '{}' copied (clears in 9s)",
                    item.name
                ));
                clipboard::spawn_autoclear(code, "TOTP");
            }
            Ok(_) => self.set_status("⚠️ Could not generate the TOTP code"),
            Err(e) => self.set_status(format!("⚠️ {e}")),
        }
    }

    pub fn toggle_reveal(&mut self) {
        let Some(session) = self.session.clone() else {
            return;
        };
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        if self.reveal.as_ref().is_some_and(|(id, _)| id == &item.id) {
            self.reveal = None;
            return;
        }
        match bw::get_password(&item.id, &session) {
            Ok(pw) => self.reveal = Some((item.id.clone(), pw)),
            Err(e) => self.set_status(format!("⚠️ {e}")),
        }
    }

    // ---- Generator tab -------------------------------------------------

    pub fn generate_password(&mut self) {
        match bw::generate(&self.generator.opts) {
            Ok(pw) => {
                self.generator.result = Some(pw);
                self.generator.error = None;
            }
            Err(e) => self.generator.error = Some(e.to_string()),
        }
    }

    pub fn copy_generated(&mut self) {
        let Some(pw) = self.generator.result.clone() else {
            return;
        };
        if let Err(e) = clipboard::copy(&pw) {
            self.set_status(format!("⚠️ {e}"));
            return;
        }
        self.set_status("✅ Generated password copied (clears in 9s)");
        clipboard::spawn_autoclear(pw, "generated");
    }

    // ---- Account tab -----------------------------------------------------

    pub fn sync_now(&mut self) {
        let Some(session) = self.session.clone() else {
            return;
        };
        match bw::sync(&session) {
            Ok(()) => {
                if let Ok(status) = bw::status() {
                    self.server_status = Some(status);
                }
                self.refresh_items();
                self.set_status("🔄 Synced with server");
            }
            Err(e) => self.set_status(format!("⚠️ {e}")),
        }
    }

    pub fn lock_now(&mut self) {
        self.relock("🔒 Vault locked, enter your master password:");
    }

    pub fn logout_now(&mut self) {
        self.confirm_logout = false;
        match bw::logout() {
            Ok(()) => {
                self.session = None;
                self.items.clear();
                self.filtered.clear();
                self.folders.clear();
                self.start();
            }
            Err(e) => self.set_status(format!("⚠️ {e}")),
        }
    }

    // ---- Input handling ----------------------------------------------

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match &mut self.screen {
            Screen::ServerConfig { url, busy, .. } => {
                if *busy {
                    return;
                }
                match key.code {
                    KeyCode::Esc => self.should_quit = true,
                    KeyCode::Enter => self.confirm_server_config(),
                    KeyCode::Backspace => {
                        url.pop();
                    }
                    KeyCode::Char(c) => url.push(c),
                    _ => {}
                }
            }
            Screen::Login {
                email,
                password,
                focus,
                awaiting_2fa,
                code,
                method,
                error,
                busy,
            } => {
                if *busy {
                    return;
                }
                if *awaiting_2fa {
                    match key.code {
                        KeyCode::Esc => self.should_quit = true,
                        KeyCode::Tab => *method = method.toggled(),
                        KeyCode::Enter => {
                            *error = None;
                            self.try_login();
                        }
                        KeyCode::Backspace => {
                            code.pop();
                        }
                        KeyCode::Char(c) => code.push(c),
                        _ => {}
                    }
                    return;
                }
                match key.code {
                    KeyCode::Esc => self.should_quit = true,
                    KeyCode::Tab => {
                        *focus = match focus {
                            LoginField::Email => LoginField::Password,
                            LoginField::Password => LoginField::Email,
                        }
                    }
                    KeyCode::Enter => match focus {
                        LoginField::Email => *focus = LoginField::Password,
                        LoginField::Password => {
                            *error = None;
                            self.try_login();
                        }
                    },
                    KeyCode::Backspace => match focus {
                        LoginField::Email => {
                            email.pop();
                        }
                        LoginField::Password => {
                            password.pop();
                        }
                    },
                    KeyCode::Char(c) => match focus {
                        LoginField::Email => email.push(c),
                        LoginField::Password => password.push(c),
                    },
                    _ => {}
                }
            }
            Screen::Unlock {
                password,
                error,
                busy,
                ..
            } => {
                if *busy {
                    return;
                }
                match key.code {
                    KeyCode::Esc => self.should_quit = true,
                    KeyCode::Enter => {
                        *error = None;
                        self.try_unlock();
                    }
                    KeyCode::Backspace => {
                        password.pop();
                    }
                    KeyCode::Char(c) => password.push(c),
                    _ => {}
                }
            }
            Screen::Main => self.handle_main_key(key),
        }
    }

    fn handle_main_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Tab => {
                self.tab = self.tab.next();
                return;
            }
            KeyCode::BackTab => {
                self.tab = self.tab.prev();
                return;
            }
            _ => {}
        }

        match self.tab {
            Tab::Vault => match self.vault_mode {
                VaultMode::Search => match key.code {
                    KeyCode::Esc => {
                        self.query.clear();
                        self.refilter();
                        self.vault_mode = VaultMode::Normal;
                    }
                    KeyCode::Enter => self.vault_mode = VaultMode::Normal,
                    KeyCode::Backspace => {
                        self.query.pop();
                        self.refilter();
                    }
                    KeyCode::Char(c) => {
                        self.query.push(c);
                        self.refilter();
                    }
                    _ => {}
                },
                VaultMode::Normal => {
                    if key.code != KeyCode::Char('g') {
                        self.pending_g = false;
                    }
                    match key.code {
                        KeyCode::Esc => {
                            if !self.query.is_empty() {
                                self.query.clear();
                                self.refilter();
                            } else {
                                self.should_quit = true;
                            }
                        }
                        KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Char('/') => self.vault_mode = VaultMode::Search,
                        KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
                        KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
                        KeyCode::Char('g') => {
                            if self.pending_g {
                                self.pending_g = false;
                                self.selected = 0;
                                self.reveal = None;
                            } else {
                                self.pending_g = true;
                            }
                        }
                        KeyCode::Char('G') => {
                            self.selected = self.filtered.len().saturating_sub(1);
                            self.reveal = None;
                        }
                        KeyCode::Char('h') | KeyCode::Left => self.cycle_folder(-1),
                        KeyCode::Char('l') | KeyCode::Right => self.cycle_folder(1),
                        KeyCode::Enter => self.copy_password(),
                        KeyCode::Char('u') => self.copy_username(),
                        KeyCode::Char('t') => self.copy_totp(),
                        KeyCode::Char('r') => self.toggle_reveal(),
                        KeyCode::Char('R') | KeyCode::F(5) => self.refresh_items(),
                        _ => {}
                    }
                }
            },
            Tab::Generator => match key.code {
                KeyCode::Esc => self.should_quit = true,
                KeyCode::Up => {
                    self.generator.opts.length = self.generator.opts.length.saturating_add(1).min(128)
                }
                KeyCode::Down => {
                    self.generator.opts.length = self.generator.opts.length.saturating_sub(1).max(5)
                }
                KeyCode::Char('u') => self.generator.opts.uppercase = !self.generator.opts.uppercase,
                KeyCode::Char('l') => self.generator.opts.lowercase = !self.generator.opts.lowercase,
                KeyCode::Char('n') => self.generator.opts.numbers = !self.generator.opts.numbers,
                KeyCode::Char('s') => self.generator.opts.special = !self.generator.opts.special,
                KeyCode::Enter => self.generate_password(),
                KeyCode::Char('c') => self.copy_generated(),
                _ => {}
            },
            Tab::Account => {
                if self.confirm_logout {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => self.logout_now(),
                        _ => self.confirm_logout = false,
                    }
                    return;
                }
                match key.code {
                    KeyCode::Esc => self.should_quit = true,
                    KeyCode::Char('s') => self.sync_now(),
                    KeyCode::Char('l') => self.lock_now(),
                    KeyCode::Char('o') => self.confirm_logout = true,
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn item(name: &str, folder_id: Option<&str>) -> Item {
        Item {
            id: name.to_string(),
            name: name.to_string(),
            item_type: 1,
            login: None,
            card: None,
            identity: None,
            fields: None,
            notes: None,
            folder_id: folder_id.map(str::to_string),
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn vault_app(items: Vec<Item>, folders: Vec<Folder>) -> App {
        let mut app = App::new();
        app.screen = Screen::Main;
        app.tab = Tab::Vault;
        app.items = items;
        app.folders = folders;
        app.refilter();
        app
    }

    #[test]
    fn move_selection_wraps_around() {
        let mut app = vault_app(vec![item("Alpha", None), item("Beta", None), item("Gamma", None)], vec![]);
        assert_eq!(app.selected, 0);
        app.move_selection(-1);
        assert_eq!(app.selected, 2);
        app.move_selection(1);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn vim_jk_navigate_list() {
        let mut app = vault_app(vec![item("Alpha", None), item("Beta", None), item("Gamma", None)], vec![]);
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.selected, 1);
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.selected, 2);
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.selected, 0, "j should wrap past the last item");
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.selected, 2, "k should wrap before the first item");
    }

    #[test]
    fn vim_gg_and_g_jump_top_bottom() {
        let mut app = vault_app(vec![item("Alpha", None), item("Beta", None), item("Gamma", None)], vec![]);
        app.selected = 1;
        app.handle_key(key(KeyCode::Char('g')));
        assert_eq!(app.selected, 1, "a single 'g' should not jump yet");
        app.handle_key(key(KeyCode::Char('g')));
        assert_eq!(app.selected, 0, "'gg' should jump to the top");
        app.handle_key(key(KeyCode::Char('G')));
        assert_eq!(app.selected, 2, "'G' should jump to the bottom");
    }

    #[test]
    fn vim_g_sequence_breaks_on_other_key() {
        let mut app = vault_app(vec![item("Alpha", None), item("Beta", None), item("Gamma", None)], vec![]);
        app.selected = 1;
        app.handle_key(key(KeyCode::Char('g')));
        app.handle_key(key(KeyCode::Char('j'))); // interrupts the gg sequence
        assert_eq!(app.selected, 2);
        app.handle_key(key(KeyCode::Char('g')));
        assert_eq!(app.selected, 2, "the interrupted 'g' should not have jumped");
    }

    #[test]
    fn slash_enters_search_mode_and_filters() {
        let mut app = vault_app(vec![item("Netflix", None), item("Amazon", None)], vec![]);
        assert_eq!(app.vault_mode, VaultMode::Normal);
        app.handle_key(key(KeyCode::Char('/')));
        assert_eq!(app.vault_mode, VaultMode::Search);
        for c in "net".chars() {
            app.handle_key(key(KeyCode::Char(c)));
        }
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(app.items[app.filtered[0]].name, "Netflix");

        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.vault_mode, VaultMode::Normal, "Enter should confirm and return to Normal mode");
        assert_eq!(app.query, "net", "the query should survive confirming the search");

        app.handle_key(key(KeyCode::Esc));
        assert!(app.query.is_empty(), "Esc in Normal mode should clear an active filter");
        assert_eq!(app.filtered.len(), 2);
    }

    #[test]
    fn search_esc_cancels_and_clears_query() {
        let mut app = vault_app(vec![item("Netflix", None), item("Amazon", None)], vec![]);
        app.handle_key(key(KeyCode::Char('/')));
        app.handle_key(key(KeyCode::Char('n')));
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.vault_mode, VaultMode::Normal);
        assert!(app.query.is_empty());
        assert_eq!(app.filtered.len(), 2);
    }

    #[test]
    fn vim_hl_cycle_folders() {
        let folders = vec![
            Folder { id: Some("f1".into()), name: "Work".into() },
            Folder { id: Some("f2".into()), name: "Personal".into() },
        ];
        let items = vec![item("A", Some("f1")), item("B", Some("f2")), item("C", None)];
        let mut app = vault_app(items, folders);
        assert_eq!(app.folder_index, 0); // All

        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.folder_index, 1); // No folder
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(app.items[app.filtered[0]].name, "C");

        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.folder_index, 2); // Work
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(app.items[app.filtered[0]].name, "A");

        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.folder_index, 1);
    }

    #[test]
    fn q_and_esc_quit_in_normal_mode() {
        let mut app = vault_app(vec![item("Alpha", None)], vec![]);
        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }
}
