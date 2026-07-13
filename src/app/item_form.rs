use super::events::BwEvent;
use super::App;
use crate::bw::{self, Item};

#[derive(Clone, Copy, PartialEq)]
pub enum ItemFormField {
    Name,
    Username,
    Password,
}

impl ItemFormField {
    pub(super) fn next(self) -> Self {
        match self {
            ItemFormField::Name => ItemFormField::Username,
            ItemFormField::Username => ItemFormField::Password,
            ItemFormField::Password => ItemFormField::Name,
        }
    }

    pub(super) fn prev(self) -> Self {
        match self {
            ItemFormField::Name => ItemFormField::Password,
            ItemFormField::Username => ItemFormField::Name,
            ItemFormField::Password => ItemFormField::Username,
        }
    }
}

pub enum ItemFormMode {
    Create,
    Edit { id: String },
}

pub struct ItemForm {
    pub mode: ItemFormMode,
    pub focus: ItemFormField,
    pub name: String,
    pub username: String,
    pub password: String,
    pub password_revealed: bool,
    pub generator_open: bool,
    pub error: Option<String>,
}

impl ItemForm {
    fn new() -> Self {
        Self {
            mode: ItemFormMode::Create,
            focus: ItemFormField::Name,
            name: String::new(),
            username: String::new(),
            password: String::new(),
            password_revealed: false,
            generator_open: false,
            error: None,
        }
    }

    fn for_editing_login(item: &Item) -> Self {
        Self {
            mode: ItemFormMode::Edit { id: item.id.clone() },
            focus: ItemFormField::Name,
            name: item.name.clone(),
            username: item.username().unwrap_or_default().to_string(),
            password: String::new(),
            password_revealed: false,
            generator_open: false,
            error: None,
        }
    }

    pub(super) fn focused_field_mut(&mut self) -> &mut String {
        match self.focus {
            ItemFormField::Name => &mut self.name,
            ItemFormField::Username => &mut self.username,
            ItemFormField::Password => &mut self.password,
        }
    }
}

impl App {
    pub fn open_create_form(&mut self) {
        self.item_form = Some(ItemForm::new());
    }

    pub fn open_edit_form(&mut self) {
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        if item.item_type != 1 {
            self.set_status("⚠️ Editing is only supported for logins right now");
            return;
        }
        self.item_form = Some(ItemForm::for_editing_login(&item));
    }

    pub fn open_item_form_password_picker(&mut self) {
        if let Some(form) = &mut self.item_form {
            form.generator_open = true;
        }
    }

    pub fn close_item_form_password_picker(&mut self) {
        if let Some(form) = &mut self.item_form {
            form.generator_open = false;
        }
    }

    pub fn confirm_item_form_password_picker(&mut self) {
        match self.generator.result.clone() {
            Some(password) => {
                if let Some(form) = &mut self.item_form {
                    form.password = password;
                    form.generator_open = false;
                }
            }
            None => self.generate_password(),
        }
    }

    pub fn reveal_current_password_in_item_form(&mut self) {
        if self.busy {
            return;
        }
        let Some(form) = &self.item_form else {
            return;
        };
        let ItemFormMode::Edit { id } = &form.mode else {
            return;
        };
        let id = id.clone();
        let Some(session) = self.session.clone() else {
            return;
        };
        self.busy = true;
        self.busy_label = Some("Fetching current password…".to_string());
        self.spawn(move || BwEvent::ItemFormPasswordRevealed(bw::get_password(&id, &session)));
    }

    pub fn submit_item_form(&mut self) {
        if self.busy {
            return;
        }
        let Some(form) = self.item_form.as_mut() else {
            return;
        };
        let name = form.name.trim().to_string();
        let username = (!form.username.is_empty()).then(|| form.username.clone());
        let password = (!form.password.is_empty()).then(|| form.password.clone());
        let editing_id = match &form.mode {
            ItemFormMode::Create => None,
            ItemFormMode::Edit { id } => Some(id.clone()),
        };

        if name.is_empty() {
            form.error = Some("Name is required".to_string());
            return;
        }
        let Some(session) = self.session.clone() else {
            return;
        };

        form.error = None;
        self.busy = true;

        match editing_id {
            Some(id) => {
                self.busy_label = Some("Saving item…".to_string());
                let patch = bw::ItemPatch {
                    name,
                    notes: None,
                    folder_id: None,
                    login: Some(bw::NewLogin { username, password }),
                    card: None,
                };
                self.spawn(move || BwEvent::ItemEdited(bw::edit_item(&id, &patch, &session)));
            }
            None => {
                self.busy_label = Some("Creating item…".to_string());
                let new_item = bw::NewItem {
                    folder_id: None,
                    item_type: 1,
                    name,
                    notes: None,
                    login: Some(bw::NewLogin { username, password }),
                    card: None,
                    secure_note: None,
                };
                self.spawn(move || BwEvent::ItemCreated(bw::create_item(&new_item, &session)));
            }
        }
    }
}
