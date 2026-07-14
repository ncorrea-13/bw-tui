use super::events::BwEvent;
use super::App;
use crate::bw::{self, Item};

#[derive(Clone, Copy, PartialEq)]
pub enum ItemKind {
    Login,
    Note,
    Card,
    Identity,
}

impl ItemKind {
    fn from_item_type(item_type: u8) -> Self {
        match item_type {
            2 => ItemKind::Note,
            3 => ItemKind::Card,
            4 => ItemKind::Identity,
            _ => ItemKind::Login,
        }
    }

    fn item_type(self) -> u8 {
        match self {
            ItemKind::Login => 1,
            ItemKind::Note => 2,
            ItemKind::Card => 3,
            ItemKind::Identity => 4,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ItemKind::Login => "login",
            ItemKind::Note => "note",
            ItemKind::Card => "card",
            ItemKind::Identity => "identity",
        }
    }

    fn next(self) -> Self {
        match self {
            ItemKind::Login => ItemKind::Note,
            ItemKind::Note => ItemKind::Card,
            ItemKind::Card => ItemKind::Identity,
            ItemKind::Identity => ItemKind::Login,
        }
    }

    fn prev(self) -> Self {
        match self {
            ItemKind::Login => ItemKind::Identity,
            ItemKind::Note => ItemKind::Login,
            ItemKind::Card => ItemKind::Note,
            ItemKind::Identity => ItemKind::Card,
        }
    }

    pub fn fields(self) -> &'static [ItemFormField] {
        match self {
            ItemKind::Login => {
                &[ItemFormField::Name, ItemFormField::Username, ItemFormField::Password, ItemFormField::Notes]
            }
            ItemKind::Note => &[ItemFormField::Name, ItemFormField::Notes],
            ItemKind::Card => &[
                ItemFormField::Name,
                ItemFormField::CardholderName,
                ItemFormField::Brand,
                ItemFormField::Number,
                ItemFormField::ExpMonth,
                ItemFormField::ExpYear,
                ItemFormField::Code,
                ItemFormField::Notes,
            ],
            ItemKind::Identity => &[
                ItemFormField::Name,
                ItemFormField::FirstName,
                ItemFormField::LastName,
                ItemFormField::Email,
                ItemFormField::Phone,
                ItemFormField::Notes,
            ],
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ItemFormField {
    Name,
    Username,
    Password,
    Notes,
    CardholderName,
    Brand,
    Number,
    ExpMonth,
    ExpYear,
    Code,
    FirstName,
    LastName,
    Email,
    Phone,
}

impl ItemFormField {
    pub fn label(self) -> &'static str {
        match self {
            ItemFormField::Name => "Name",
            ItemFormField::Username => "Username",
            ItemFormField::Password => "Password",
            ItemFormField::Notes => "Notes",
            ItemFormField::CardholderName => "Cardholder",
            ItemFormField::Brand => "Brand",
            ItemFormField::Number => "Number",
            ItemFormField::ExpMonth => "Exp. month",
            ItemFormField::ExpYear => "Exp. year",
            ItemFormField::Code => "CVV",
            ItemFormField::FirstName => "First name",
            ItemFormField::LastName => "Last name",
            ItemFormField::Email => "Email",
            ItemFormField::Phone => "Phone",
        }
    }

    pub fn max_len(self) -> Option<usize> {
        match self {
            ItemFormField::ExpMonth => Some(2),
            ItemFormField::ExpYear => Some(4),
            ItemFormField::Code => Some(3),
            _ => None,
        }
    }
}

pub enum ItemFormMode {
    Create,
    Edit { id: String },
}

pub struct ItemForm {
    pub mode: ItemFormMode,
    pub kind: ItemKind,
    pub focus: ItemFormField,
    pub name: String,
    pub username: String,
    pub password: String,
    pub password_revealed: bool,
    pub notes: String,
    pub cardholder_name: String,
    pub brand: String,
    pub number: String,
    pub exp_month: String,
    pub exp_year: String,
    pub code: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub generator_open: bool,
    pub error: Option<String>,
}

impl ItemForm {
    fn new() -> Self {
        Self {
            mode: ItemFormMode::Create,
            kind: ItemKind::Login,
            focus: ItemFormField::Name,
            name: String::new(),
            username: String::new(),
            password: String::new(),
            password_revealed: false,
            notes: String::new(),
            cardholder_name: String::new(),
            brand: String::new(),
            number: String::new(),
            exp_month: String::new(),
            exp_year: String::new(),
            code: String::new(),
            first_name: String::new(),
            last_name: String::new(),
            email: String::new(),
            phone: String::new(),
            generator_open: false,
            error: None,
        }
    }

    fn for_editing(item: &Item) -> Self {
        let card = item.card.as_ref();
        let identity = item.identity.as_ref();
        Self {
            mode: ItemFormMode::Edit { id: item.id.clone() },
            kind: ItemKind::from_item_type(item.item_type),
            focus: ItemFormField::Name,
            name: item.name.clone(),
            username: item.username().unwrap_or_default().to_string(),
            password: String::new(),
            password_revealed: false,
            notes: item.notes.clone().unwrap_or_default(),
            cardholder_name: card.and_then(|c| c.cardholder_name.clone()).unwrap_or_default(),
            brand: card.and_then(|c| c.brand.clone()).unwrap_or_default(),
            number: card.and_then(|c| c.number.clone()).unwrap_or_default(),
            exp_month: card.and_then(|c| c.exp_month.clone()).unwrap_or_default(),
            exp_year: card.and_then(|c| c.exp_year.clone()).unwrap_or_default(),
            code: card.and_then(|c| c.code.clone()).unwrap_or_default(),
            first_name: identity.and_then(|i| i.first_name.clone()).unwrap_or_default(),
            last_name: identity.and_then(|i| i.last_name.clone()).unwrap_or_default(),
            email: identity.and_then(|i| i.email.clone()).unwrap_or_default(),
            phone: identity.and_then(|i| i.phone.clone()).unwrap_or_default(),
            generator_open: false,
            error: None,
        }
    }

    pub(super) fn cycle_focus(&mut self, delta: i32) {
        let fields = self.kind.fields();
        let current = fields.iter().position(|&f| f == self.focus).unwrap_or(0);
        let len = fields.len() as i32;
        let new_pos = (current as i32 + delta).rem_euclid(len);
        self.focus = fields[new_pos as usize];
    }

    pub(super) fn focused_field_mut(&mut self) -> &mut String {
        match self.focus {
            ItemFormField::Name => &mut self.name,
            ItemFormField::Username => &mut self.username,
            ItemFormField::Password => &mut self.password,
            ItemFormField::Notes => &mut self.notes,
            ItemFormField::CardholderName => &mut self.cardholder_name,
            ItemFormField::Brand => &mut self.brand,
            ItemFormField::Number => &mut self.number,
            ItemFormField::ExpMonth => &mut self.exp_month,
            ItemFormField::ExpYear => &mut self.exp_year,
            ItemFormField::Code => &mut self.code,
            ItemFormField::FirstName => &mut self.first_name,
            ItemFormField::LastName => &mut self.last_name,
            ItemFormField::Email => &mut self.email,
            ItemFormField::Phone => &mut self.phone,
        }
    }

    pub fn field_value(&self, field: ItemFormField) -> &str {
        match field {
            ItemFormField::Name => &self.name,
            ItemFormField::Username => &self.username,
            ItemFormField::Password => &self.password,
            ItemFormField::Notes => &self.notes,
            ItemFormField::CardholderName => &self.cardholder_name,
            ItemFormField::Brand => &self.brand,
            ItemFormField::Number => &self.number,
            ItemFormField::ExpMonth => &self.exp_month,
            ItemFormField::ExpYear => &self.exp_year,
            ItemFormField::Code => &self.code,
            ItemFormField::FirstName => &self.first_name,
            ItemFormField::LastName => &self.last_name,
            ItemFormField::Email => &self.email,
            ItemFormField::Phone => &self.phone,
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
        if !matches!(item.item_type, 1..=4) {
            self.set_status("\u{f071} Editing is not supported for this item type");
            return;
        }
        self.item_form = Some(ItemForm::for_editing(&item));
    }

    pub fn cycle_item_kind(&mut self, delta: i32) {
        let Some(form) = &mut self.item_form else {
            return;
        };
        if !matches!(form.mode, ItemFormMode::Create) {
            return;
        }
        form.kind = if delta > 0 { form.kind.next() } else { form.kind.prev() };
        let fields = form.kind.fields();
        if !fields.contains(&form.focus) {
            form.focus = fields[0];
        }
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
        let opt = |s: &str| (!s.is_empty()).then(|| s.to_string());

        let kind = form.kind;
        let name = form.name.trim().to_string();
        let notes = opt(&form.notes);
        let login = matches!(kind, ItemKind::Login)
            .then(|| bw::NewLogin { username: opt(&form.username), password: opt(&form.password) });
        let card = matches!(kind, ItemKind::Card).then(|| bw::NewCard {
            cardholder_name: opt(&form.cardholder_name),
            brand: opt(&form.brand),
            number: opt(&form.number),
            exp_month: opt(&form.exp_month),
            exp_year: opt(&form.exp_year),
            code: opt(&form.code),
        });
        let identity = matches!(kind, ItemKind::Identity).then(|| bw::NewIdentity {
            first_name: opt(&form.first_name),
            last_name: opt(&form.last_name),
            email: opt(&form.email),
            phone: opt(&form.phone),
        });
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
                let patch = bw::ItemPatch { name, notes, folder_id: None, login, card, identity };
                self.spawn(move || BwEvent::ItemEdited(bw::edit_item(&id, &patch, &session)));
            }
            None => {
                self.busy_label = Some("Creating item…".to_string());
                let secure_note = matches!(kind, ItemKind::Note).then_some(bw::SecureNoteData { note_type: 0 });
                let new_item = bw::NewItem {
                    folder_id: None,
                    item_type: kind.item_type(),
                    name,
                    notes,
                    login,
                    card,
                    identity,
                    secure_note,
                };
                self.spawn(move || BwEvent::ItemCreated(bw::create_item(&new_item, &session)));
            }
        }
    }
}
