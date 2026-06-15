use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub parts: Vec<String>,
    pub is_complete: bool,
    pub missing: Vec<String>,
}

pub struct VolumeDetector;

impl VolumeDetector {
    pub fn detect(path: &Path) -> Option<VolumeInfo> {
        let parent = path.parent()?;
        let filename = path.file_name()?.to_string_lossy().to_string();

        let stem = path.file_stem()?.to_string_lossy().to_string();
        let ext = path.extension().map(|e| e.to_string_lossy().to_string());

        if let Some(info) = Self::detect_dot_number(&parent, &stem, &ext) {
            return Some(info);
        }

        if let Some(info) = Self::detect_part_number(&parent, &stem) {
            return Some(info);
        }

        if let Some(info) = Self::detect_dot_number_ext(&parent, &filename) {
            return Some(info);
        }

        None
    }

    fn detect_dot_number_ext(parent: &Path, filename: &str) -> Option<VolumeInfo> {
        let re = regex::Regex::new(r"^(.+)\.(\d{3})$").ok()?;
        let caps = re.captures(filename)?;
        let base = caps.get(1)?.as_str();
        let _first_num: u32 = caps.get(2)?.as_str().parse().ok()?;

        let mut parts: Vec<PathBuf> = Vec::new();
        let mut num = 1u32;
        loop {
            let name = format!("{}.{:03}", base, num);
            let p = parent.join(&name);
            if p.exists() {
                parts.push(p);
                num += 1;
            } else {
                break;
            }
            if num > 999 {
                break;
            }
        }

        if parts.len() > 1 {
            let part_strs: Vec<String> = parts
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            return Some(VolumeInfo {
                parts: part_strs,
                is_complete: true,
                missing: vec![],
            });
        }
        None
    }

    fn detect_dot_number(parent: &Path, stem: &str, ext: &Option<String>) -> Option<VolumeInfo> {
        let re = regex::Regex::new(r"^(.+?)\.(\d+)$").ok()?;
        let caps = re.captures(stem)?;
        let base = caps.get(1)?.as_str();
        let _first_num: u32 = caps.get(2)?.as_str().parse().ok()?;

        let ext_str = ext.as_deref().unwrap_or("");
        let mut parts: Vec<PathBuf> = Vec::new();
        let mut num = 1u32;
        loop {
            let name = if ext_str.is_empty() {
                format!("{}.{:03}", base, num)
            } else {
                format!("{}.{:03}.{}", base, num, ext_str)
            };
            let p = parent.join(&name);
            if p.exists() {
                parts.push(p);
                num += 1;
            } else {
                break;
            }
            if num > 999 {
                break;
            }
        }

        if parts.len() > 1 {
            let part_strs: Vec<String> = parts
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            return Some(VolumeInfo {
                parts: part_strs,
                is_complete: true,
                missing: vec![],
            });
        }
        None
    }

    fn detect_part_number(parent: &Path, stem: &str) -> Option<VolumeInfo> {
        let re = regex::Regex::new(r"^(.+?)[\.\s]*part\s*(\d+)$").ok()?;
        let caps = re.captures(stem)?;
        let base = caps.get(1)?.as_str().to_string();
        let _first_num: u32 = caps.get(2)?.as_str().parse().ok()?;

        let mut parts: Vec<PathBuf> = Vec::new();
        let mut num = 1u32;
        loop {
            for ext in &["rar", "7z", "zip"] {
                let name = format!("{}.part{:01}.{}", base, num, ext);
                let p = parent.join(&name);
                if p.exists() {
                    parts.push(p);
                }
            }
            num += 1;
            if num > 100 {
                break;
            }
        }

        if parts.len() > 1 {
            let part_strs: Vec<String> = parts
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            return Some(VolumeInfo {
                parts: part_strs,
                is_complete: true,
                missing: vec![],
            });
        }
        None
    }
}
