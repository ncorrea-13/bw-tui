use crate::bw::GenerateOptions;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub bw_cmd: String,
    pub session_max_age_secs: u64,
    pub clipboard_clear_secs: u64,
    pub generator: GenerateOptions,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bw_cmd: "bw".to_string(),
            session_max_age_secs: 1200,
            clipboard_clear_secs: 9,
            generator: GenerateOptions::default(),
        }
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn get() -> &'static Config {
    CONFIG.get_or_init(load_or_create)
}

fn config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("bw-tui");
    }
    let home = std::env::var("HOME").expect("HOME is not set");
    PathBuf::from(home).join(".config").join("bw-tui")
}

fn config_file() -> PathBuf {
    config_dir().join("config.json")
}

fn load_or_create() -> Config {
    let path = config_file();
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("⚠️ could not parse {}: {e}, using defaults", path.display());
                Config::default()
            }
        },
        Err(_) => {
            let cfg = Config::default();
            if let Err(e) = write_default(&path, &cfg) {
                eprintln!("⚠️ could not create {}: {e}", path.display());
            }
            cfg
        }
    }
}

fn write_default(path: &Path, cfg: &Config) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(cfg).expect("Config is always serializable");
    std::fs::write(path, json)
}
