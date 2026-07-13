use super::events::BwEvent;
use super::App;
use crate::bw;

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

pub struct ItemForm {
    pub focus: ItemFormField,
    pub name: String,
    pub username: String,
    pub password: String,
    pub error: Option<String>,
}

impl ItemForm {
    fn new() -> Self {
        Self {
            focus: ItemFormField::Name,
            name: String::new(),
            username: String::new(),
            password: String::new(),
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

    pub fn submit_item_form(&mut self) {
        if self.busy {
            return;
        }
        let Some(form) = &self.item_form else {
            return;
        };
        let name = form.name.trim().to_string();
        let username = (!form.username.is_empty()).then(|| form.username.clone());
        let password = (!form.password.is_empty()).then(|| form.password.clone());

        if name.is_empty() {
            self.item_form.as_mut().unwrap().error = Some("Name is required".to_string());
            return;
        }
        let Some(session) = self.session.clone() else {
            return;
        };

        self.item_form.as_mut().unwrap().error = None;
        self.busy = true;
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
