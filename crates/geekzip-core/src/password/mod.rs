use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResult {
    pub password: String,
    pub source: PasswordSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PasswordSource {
    Filename,
    History,
    Builtin,
    Dictionary,
    Manual,
}

pub struct PasswordEngine;

impl PasswordEngine {
    pub fn extract_from_filename(filename: &str) -> Option<String> {
        let patterns = [
            r"[\[【]\s*密码\s*[：:]\s*([^\]】]+)\s*[\]】]",
            r"[\[【]\s*pwd\s*[：:]\s*([^\]】]+)\s*[\]】]",
            r"[\[【]\s*pass(?:word)?\s*[：:]\s*([^\]】]+)\s*[\]】]",
            r"密码\s*[：:]\s*(\S+)",
            r"[pP]wd\s*[：:]\s*(\S+)",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(caps) = re.captures(filename) {
                    if let Some(m) = caps.get(1) {
                        let pwd = m.as_str().trim().to_string();
                        if !pwd.is_empty() {
                            return Some(pwd);
                        }
                    }
                }
            }
        }
        None
    }
}

pub static BUILTIN_PASSWORDS: &[&str] = &[
    "1234", "12345", "123456", "1234567", "12345678",
    "0000", "1111", "6666", "8888", "9999",
    "password", "admin", "test", "abcd", "qwer",
    "abc123", "111111", "666666", "888888", "000000",
];