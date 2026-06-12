use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub language: String,
    pub hdr_enabled: bool,
    pub theme: String,
    pub default_extract_path: String,
    pub default_overwrite: String,
    pub default_create_subfolder: bool,
    pub default_open_after_extract: bool,
    pub default_delete_after_extract: bool,
    pub default_delete_intermediate: bool,
    pub recursive_enabled: bool,
    pub recursive_max_depth: u32,
    pub single_file_size_limit: u64,
    pub total_extract_size_limit: u64,
    pub password_timeout_seconds: u32,
    pub auto_save_passwords: bool,
    pub use_builtin_passwords: bool,
    pub use_password_dictionary: bool,
    pub password_dictionary_path: Option<String>,
    pub show_completion_notification: bool,
    pub show_error_notification: bool,
    pub enable_watcher: bool,
    pub watch_paths: Vec<String>,
    pub max_concurrent_tasks: u32,
    pub max_threads: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "zh".to_string(),
            hdr_enabled: true,
            theme: "dark".to_string(),
            default_extract_path: "current".to_string(),
            default_overwrite: "rename".to_string(),
            default_create_subfolder: true,
            default_open_after_extract: false,
            default_delete_after_extract: false,
            default_delete_intermediate: false,
            recursive_enabled: true,
            recursive_max_depth: 10,
            single_file_size_limit: 10 * 1024 * 1024 * 1024,
            total_extract_size_limit: 50 * 1024 * 1024 * 1024,
            password_timeout_seconds: 30,
            auto_save_passwords: true,
            use_builtin_passwords: true,
            use_password_dictionary: false,
            password_dictionary_path: None,
            show_completion_notification: true,
            show_error_notification: true,
            enable_watcher: false,
            watch_paths: vec![],
            max_concurrent_tasks: 4,
            max_threads: 8,
        }
    }
}

impl AppSettings {
    pub fn config_path() -> PathBuf {
        let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.push("geekzip");
        dir.push("settings.json");
        dir
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let data = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, data).map_err(|e| e.to_string())
    }
}