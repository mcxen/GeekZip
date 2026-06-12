use serde::{Deserialize, Serialize};

const INTERFERENCE: &[&str] = &[
    "删除", "去掉", "勿", "不要", "取消", "移除",
    "delete", "remove", "cancel", "drop", "del",
    "复制", "copy", "副本", "拷贝",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanResult {
    pub original: String,
    pub cleaned: String,
    pub removed: Vec<String>,
}

pub struct FilenameCleaner;

impl FilenameCleaner {
    pub fn clean(name: &str) -> CleanResult {
        let mut cleaned = name.to_string();
        let mut removed = Vec::new();

        for pattern in INTERFERENCE {
            if cleaned.contains(pattern) {
                cleaned = cleaned.replace(pattern, "");
                removed.push(pattern.to_string());
            }
        }

        cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
        let cleaned = cleaned.trim().to_string();

        CleanResult {
            original: name.to_string(),
            cleaned,
            removed,
        }
    }

    pub fn restore_extension(name: &str, format_ext: &str) -> String {
        let has_dot = name.contains('.');
        if has_dot {
            name.to_string()
        } else {
            format!("{}.{}", name, format_ext)
        }
    }
}