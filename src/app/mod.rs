mod events;
mod input;
#[cfg(test)]
mod tests;

use crate::bw::{self, Folder, GenerateOptions, Item, Status};
use crate::config;
use crate::clipboard;
use events::BwEvent;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

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
    Unlock {
        email: Option<String>,
        password: String,
        error: Option<String>,
        busy: bool,
        relock_message: Option<String>,
    },
    Loading,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VaultMode {
    Normal,
    Search,
}

pub struct GeneratorState {
    pub opts: GenerateOptions,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ItemFormField {
    Name,
    Username,
    Password,
}

impl ItemFormField {
    fn next(self) -> Self {
        match self {
            ItemFormField::Name => ItemFormField::Username,
            ItemFormField::Username => ItemFormField::Password,
            ItemFormField::Password => ItemFormField::Name,
        }
    }

    fn prev(self) -> Self {
        match self {
            ItemFormField::Name => ItemFormField::Password,
            ItemFormField::Username => ItemFormField::Name,
            ItemFormField::Password => ItemFormField::Username,
        }
    }
}

pub struct ItemForm {
    pub focus: ItemFormField,
    pub name: String,
    pub username: String,
    pub password: String,
    pub error: Option<String>,
    pub busy: bool,
}

impl ItemForm {
    fn new() -> Self {
        Self {
            focus: ItemFormField::Name,
            name: String::new(),
            username: String::new(),
            password: String::new(),
            error: None,
            busy: false,
        }
    }

    fn focused_field_mut(&mut self) -> &mut String {
        match self.focus {
            ItemFormField::Name => &mut self.name,
            ItemFormField::Username => &mut self.username,
            ItemFormField::Password => &mut self.password,
        }
    }
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
    pub folder_index: usize,
    pub show_folders: bool,
    pub vault_mode: VaultMode,
    pending_g: bool,
    pub query: String,
    pub selected: usize,
    pub status: Option<StatusMsg>,
    pub should_quit: bool,
    pub reveal: Option<(String, String)>,
    pub reveal_cvv: Option<String>,
    pub detail_open: bool,
    pub item_form: Option<ItemForm>,
    pub server_status: Option<Status>,
    pub generator: GeneratorState,
    pub confirm_logout: bool,
    pub busy: bool,
    pub busy_label: Option<String>,
    tick_count: u64,
    matcher: SkimMatcherV2,
    bw_tx: Sender<BwEvent>,
    bw_rx: Receiver<BwEvent>,
}

impl App {
    pub fn new() -> Self {
        let (bw_tx, bw_rx) = mpsc::channel();
        Self {
            screen: Screen::Loading,
            tab: Tab::Vault,
            session: None,
            session_started: 0,
            items: Vec::new(),
            filtered: Vec::new(),
            folders: Vec::new(),
            folder_index: 0,
            show_folders: false,
            vault_mode: VaultMode::Normal,
            pending_g: false,
            query: String::new(),
            selected: 0,
            status: None,
            should_quit: false,
            reveal: None,
            reveal_cvv: None,
            detail_open: false,
            item_form: None,
            server_status: None,
            generator: GeneratorState {
                opts: config::get().generator.clone(),
                result: None,
                error: None,
            },
            confirm_logout: false,
            busy: false,
            busy_label: None,
            tick_count: 0,
            matcher: SkimMatcherV2::default(),
            bw_tx,
            bw_rx,
        }
    }

    fn spawn(&self, f: impl FnOnce() -> BwEvent + Send + 'static) {
        let tx = self.bw_tx.clone();
        thread::spawn(move || {
            let _ = tx.send(f());
        });
    }

    pub fn start(&mut self) {
        self.screen = Screen::Loading;
        self.spawn(|| BwEvent::Started(bw::compute_start()));
    }

    fn apply_start_outcome(&mut self, outcome: bw::StartOutcome) {
        match outcome {
            bw::StartOutcome::Vault(load) => {
                self.enter_vault(load.key, load.ts, load.items, load.folders);
            }
            bw::StartOutcome::NeedsServerConfig(status) => {
                self.session = None;
                self.items.clear();
                self.filtered.clear();
                self.folders.clear();
                self.screen = Screen::ServerConfig {
                    url: status.server_url.clone().unwrap_or_default(),
                    error: None,
                    busy: false,
                };
                self.server_status = Some(status);
            }
            bw::StartOutcome::NeedsUnlock(status) => {
                self.session = None;
                self.items.clear();
                self.filtered.clear();
                self.folders.clear();
                self.screen = Screen::Unlock {
                    email: status.user_email.clone(),
                    password: String::new(),
                    error: None,
                    busy: false,
                    relock_message: None,
                };
                self.server_status = Some(status);
            }
            bw::StartOutcome::Error(e) => self.set_status(format!("⚠️ {e}")),
        }
    }

    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status = Some(StatusMsg {
            text: text.into(),
            shown_at: Instant::now(),
        });
    }

    pub fn spinner(&self) -> &'static str {
        SPINNER[(self.tick_count as usize) % SPINNER.len()]
    }

    fn poll_bw_events(&mut self) {
        while let Ok(event) = self.bw_rx.try_recv() {
            self.apply_bw_event(event);
        }
    }

    pub fn on_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.poll_bw_events();
        if let Some(s) = &self.status
            && s.shown_at.elapsed() > Duration::from_secs(4) {
                self.status = None;
            }
        if matches!(self.screen, Screen::Main) && self.session_age() > config::get().session_max_age_secs {
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
        config::get().session_max_age_secs.saturating_sub(self.session_age())
    }

    fn enter_vault(&mut self, key: String, ts: u64, items: Vec<Item>, folders: Vec<Folder>) {
        self.folders = folders;
        self.session = Some(key);
        self.session_started = ts;
        self.items = items;
        self.folder_index = 0;
        self.show_folders = false;
        self.vault_mode = VaultMode::Normal;
        self.reveal = None;
        self.reveal_cvv = None;
        self.detail_open = false;
        self.refilter();
        self.screen = Screen::Main;
        self.tab = Tab::Vault;
    }

    fn relock(&mut self, message: &str) {
        let email = self.server_status.as_ref().and_then(|s| s.user_email.clone());
        thread::spawn(bw::clear_cached_session);
        self.session = None;
        self.items.clear();
        self.filtered.clear();
        self.folders.clear();
        self.query.clear();
        self.reveal = None;
        self.reveal_cvv = None;
        self.detail_open = false;
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
        let Screen::ServerConfig { url, busy, .. } = &mut self.screen else {
            return;
        };
        if *busy {
            return;
        }
        let url = url.trim().to_string();
        if url.is_empty() {
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
            return;
        }
        if let Screen::ServerConfig { busy, .. } = &mut self.screen {
            *busy = true;
        }
        self.spawn(move || {
            let result = bw::config_server(&url);
            BwEvent::ServerConfigured { url, result }
        });
    }

    // ---- Login / unlock --------------------------------------------------

    fn try_login(&mut self) {
        let Screen::Login { email, password, awaiting_2fa, code, method, busy, .. } = &mut self.screen
        else {
            return;
        };
        if *busy {
            return;
        }
        let email = email.clone();
        let password = password.clone();
        let method = *method;
        let two_factor = awaiting_2fa.then(|| (method.code().to_string(), code.clone()));
        *busy = true;

        self.spawn(move || {
            let tf_ref = two_factor.as_ref().map(|(m, c)| (m.as_str(), c.as_str()));
            let result = bw::login_and_load(&email, &password, tf_ref);
            BwEvent::LoggedIn(result)
        });
    }

    fn try_unlock(&mut self) {
        let Screen::Unlock { password, busy, .. } = &mut self.screen else {
            return;
        };
        if *busy || password.is_empty() {
            return;
        }
        let password = password.clone();
        *busy = true;

        self.spawn(move || BwEvent::Unlocked(bw::unlock_and_load(&password)));
    }

    // ---- Vault tab ---------------------------------------------------

    pub fn open_create_form(&mut self) {
        self.item_form = Some(ItemForm::new());
    }

    pub fn refresh_items(&mut self) {
        if self.busy {
            return;
        }
        let Some(session) = self.session.clone() else {
            return;
        };
        self.busy = true;
        self.busy_label = Some("Refreshing…".to_string());
        self.spawn(move || BwEvent::ItemsRefreshed(bw::refresh_items(&session)));
    }

    pub fn toggle_folder_bar(&mut self) {
        self.show_folders = !self.show_folders;
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
        self.reveal_cvv = None;
        self.detail_open = false;
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
        self.reveal_cvv = None;
        self.detail_open = false;
    }

    pub fn copy_password(&mut self) {
        if self.busy {
            return;
        }
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        if item.item_type != 1 {
            self.set_status("⚠️ This item has no password");
            return;
        }
        let Some(session) = self.session.clone() else {
            return;
        };
        self.busy = true;
        self.busy_label = Some(format!("Copying password for '{}'…", item.name));
        self.spawn(move || {
            let result = bw::get_password(&item.id, &session);
            BwEvent::PasswordCopied { item_name: item.name, result }
        });
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

    pub fn copy_primary_secret(&mut self) {
        let Some(item) = self.selected_item() else {
            return;
        };
        match item.item_type {
            1 => self.copy_password(),
            2 => self.copy_notes(),
            3 => self.copy_card_number(),
            _ => self.set_status("⚠️ Nothing to copy for this item"),
        }
    }

    pub fn copy_card_number(&mut self) {
        let Some(item) = self.selected_item() else {
            return;
        };
        let Some(number) = item.card.as_ref().and_then(|c| c.number.clone()) else {
            self.set_status("⚠️ This card has no number on file");
            return;
        };
        let name = item.name.clone();
        if let Err(e) = clipboard::copy(&number) {
            self.set_status(format!("⚠️ {e}"));
            return;
        }
        let secs = config::get().clipboard_clear_secs;
        let note = clipboard::autoclear_note(secs);
        self.set_status(format!("✅ Card number for '{name}' copied{note}"));
        clipboard::spawn_autoclear(number, "card number");
    }

    pub fn copy_notes(&mut self) {
        let Some(item) = self.selected_item() else {
            return;
        };
        let Some(notes) = item.notes.clone().filter(|n| !n.is_empty()) else {
            self.set_status("⚠️ This item has no notes");
            return;
        };
        let name = item.name.clone();
        if let Err(e) = clipboard::copy(&notes) {
            self.set_status(format!("⚠️ {e}"));
            return;
        }
        let secs = config::get().clipboard_clear_secs;
        let note = clipboard::autoclear_note(secs);
        self.set_status(format!("✅ Notes for '{name}' copied{note}"));
        clipboard::spawn_autoclear(notes, "notes");
    }

    pub fn copy_totp(&mut self) {
        if self.busy {
            return;
        }
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
        self.busy = true;
        self.busy_label = Some(format!("Fetching TOTP for '{}'…", item.name));
        self.spawn(move || {
            let result = bw::get_totp(&item.id, &session);
            BwEvent::TotpCopied { item_name: item.name, result }
        });
    }

    pub fn toggle_reveal(&mut self) {
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        if self.reveal.as_ref().is_some_and(|(id, _)| id == &item.id) {
            self.reveal = None;
            self.reveal_cvv = None;
            return;
        }
        match item.item_type {
            1 => {
                if self.busy {
                    return;
                }
                let Some(session) = self.session.clone() else {
                    return;
                };
                self.busy = true;
                self.busy_label = Some("Revealing password…".to_string());
                self.spawn(move || {
                    let result = bw::get_password(&item.id, &session);
                    BwEvent::Revealed { item_id: item.id, result }
                });
            }
            3 => {
                let Some(number) = item.card.as_ref().and_then(|c| c.number.clone()) else {
                    self.set_status("⚠️ This card has no number on file");
                    return;
                };
                self.reveal_cvv = item.card.as_ref().and_then(|c| c.code.clone());
                self.reveal = Some((item.id, number));
            }
            _ => self.set_status("⚠️ Nothing to reveal for this item"),
        }
    }

    // ---- Generator tab -------------------------------------------------

    pub fn generate_password(&mut self) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.busy_label = Some("Generating…".to_string());
        let opts = self.generator.opts.clone();
        self.spawn(move || BwEvent::Generated(bw::generate(&opts)));
    }

    pub fn copy_generated(&mut self) {
        let Some(pw) = self.generator.result.clone() else {
            return;
        };
        if let Err(e) = clipboard::copy(&pw) {
            self.set_status(format!("⚠️ {e}"));
            return;
        }
        let secs = config::get().clipboard_clear_secs;
        let note = clipboard::autoclear_note(secs);
        self.set_status(format!("✅ Generated password copied{note}"));
        clipboard::spawn_autoclear(pw, "generated");
    }

    // ---- Account tab -----------------------------------------------------

    pub fn sync_now(&mut self) {
        if self.busy {
            return;
        }
        let Some(session) = self.session.clone() else {
            return;
        };
        self.busy = true;
        self.busy_label = Some("Syncing…".to_string());
        self.spawn(move || BwEvent::Synced(bw::sync_and_refresh(&session)));
    }

    pub fn lock_now(&mut self) {
        self.relock("🔒 Vault locked, enter your master password:");
    }

    pub fn logout_now(&mut self) {
        self.confirm_logout = false;
        if self.busy {
            return;
        }
        self.busy = true;
        self.busy_label = Some("Logging out…".to_string());
        self.spawn(|| BwEvent::LoggedOut(bw::logout_and_restart()));
    }
}
