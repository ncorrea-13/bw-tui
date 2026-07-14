use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct UriData {
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginData {
    pub username: Option<String>,
    #[allow(dead_code)]
    pub password: Option<String>,
    pub totp: Option<String>,
    pub uris: Option<Vec<UriData>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CardData {
    #[serde(rename = "cardholderName")]
    pub cardholder_name: Option<String>,
    pub brand: Option<String>,
    pub number: Option<String>,
    #[serde(rename = "expMonth")]
    pub exp_month: Option<String>,
    #[serde(rename = "expYear")]
    pub exp_year: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityData {
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomField {
    pub name: Option<String>,
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub field_type: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: u8,
    pub login: Option<LoginData>,
    pub card: Option<CardData>,
    pub identity: Option<IdentityData>,
    pub fields: Option<Vec<CustomField>>,
    pub notes: Option<String>,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
}

impl Item {
    pub fn username(&self) -> Option<&str> {
        self.login.as_ref()?.username.as_deref()
    }

    pub fn has_totp(&self) -> bool {
        self.login.as_ref().and_then(|l| l.totp.as_ref()).is_some()
    }

    pub fn first_uri(&self) -> Option<&str> {
        self.login.as_ref()?.uris.as_ref()?.first()?.uri.as_deref()
    }

    pub fn type_label(&self) -> &'static str {
        match self.item_type {
            1 => "login",
            2 => "note",
            3 => "card",
            4 => "identity",
            _ => "?",
        }
    }

    pub fn type_icon(&self) -> &'static str {
        match self.item_type {
            1 => "\u{f084}",
            2 => "\u{f249}",
            3 => "\u{f09d}",
            4 => "\u{f2bb}",
            _ => "?",
        }
    }

    pub fn identity_summary(&self) -> Option<String> {
        let i = self.identity.as_ref()?;
        let name = [i.first_name.as_deref(), i.last_name.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        let mut parts = vec![];
        if !name.is_empty() {
            parts.push(name);
        }
        if let Some(email) = &i.email {
            parts.push(email.clone());
        }
        if let Some(phone) = &i.phone {
            parts.push(phone.clone());
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" · "))
        }
    }

    pub fn visible_fields(&self) -> Vec<(String, String)> {
        self.fields
            .as_ref()
            .map(|fields| {
                fields
                    .iter()
                    .map(|f| {
                        let name = f.name.clone().unwrap_or_else(|| "(unnamed)".to_string());
                        let value = match f.field_type {
                            1 => "••••••••".to_string(),
                            _ => f.value.clone().unwrap_or_default(),
                        };
                        (name, value)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ---- Create/edit payloads -------------------------------------------------

#[derive(Debug, Clone, Default, Serialize)]
pub struct NewLogin {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct NewIdentity {
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct NewCard {
    #[serde(rename = "cardholderName")]
    pub cardholder_name: Option<String>,
    pub brand: Option<String>,
    pub number: Option<String>,
    #[serde(rename = "expMonth")]
    pub exp_month: Option<String>,
    #[serde(rename = "expYear")]
    pub exp_year: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecureNoteData {
    #[serde(rename = "type")]
    pub note_type: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct NewItem {
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
    #[serde(rename = "type")]
    pub item_type: u8,
    pub name: String,
    pub notes: Option<String>,
    pub login: Option<NewLogin>,
    pub card: Option<NewCard>,
    pub identity: Option<NewIdentity>,
    #[serde(rename = "secureNote")]
    pub secure_note: Option<SecureNoteData>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemPatch {
    pub name: String,
    pub notes: Option<String>,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
    pub login: Option<NewLogin>,
    pub card: Option<NewCard>,
    pub identity: Option<NewIdentity>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Folder {
    pub id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Status {
    #[serde(rename = "serverUrl")]
    pub server_url: Option<String>,
    #[serde(rename = "lastSync")]
    pub last_sync: Option<String>,
    #[serde(rename = "userEmail")]
    pub user_email: Option<String>,
    pub status: String,
}

pub enum LoginOutcome {
    Success(String),
    TwoFactorRequired,
}

pub struct VaultLoad {
    pub key: String,
    pub ts: u64,
    pub items: Vec<Item>,
    pub folders: Vec<Folder>,
}

pub struct ItemsLoad {
    pub items: Vec<Item>,
    pub folders: Vec<Folder>,
}

pub struct SyncLoad {
    pub status: Option<Status>,
    pub items: Vec<Item>,
    pub folders: Vec<Folder>,
}

pub enum LoginFlowResult {
    LoggedIn(VaultLoad),
    TwoFactorRequired,
}

pub enum StartOutcome {
    Vault(VaultLoad),
    NeedsServerConfig(Status),
    NeedsUnlock(Status),
    Error(anyhow::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GenerateOptions {
    pub length: u8,
    pub uppercase: bool,
    pub lowercase: bool,
    pub numbers: bool,
    pub special: bool,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            length: 20,
            uppercase: true,
            lowercase: true,
            numbers: true,
            special: false,
        }
    }
}
