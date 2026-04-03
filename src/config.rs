use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub name: String,
    #[serde(default)]
    pub caption: String,
    pub executable: String,
    pub parameters: String,
    pub working_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub entries: Vec<RunConfig>,
    #[serde(default = "default_overlay_x")]
    pub overlay_x: f32,
    #[serde(default = "default_overlay_y")]
    pub overlay_y: f32,
    #[serde(default = "default_overlay_width")]
    pub overlay_width: f32,
    #[serde(default = "default_hotkey_key")]
    pub hotkey_key: String,
    #[serde(default = "default_true")]
    pub hotkey_alt: bool,
    #[serde(default)]
    pub hotkey_ctrl: bool,
}

fn default_overlay_x() -> f32 {
    100.0
}

fn default_overlay_y() -> f32 {
    100.0
}

fn default_overlay_width() -> f32 {
    400.0
}

fn default_hotkey_key() -> String {
    "F10".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            overlay_x: default_overlay_x(),
            overlay_y: default_overlay_y(),
            overlay_width: default_overlay_width(),
            hotkey_key: default_hotkey_key(),
            hotkey_alt: true,
            hotkey_ctrl: false,
        }
    }
}

pub fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("keykoff");
    path.push("config.json");
    path
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        return AppConfig::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, json)?;
    Ok(())
}
