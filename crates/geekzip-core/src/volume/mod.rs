use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub parts: Vec<String>,
    pub is_complete: bool,
    pub missing: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_obfuscated_dot_number_volumes() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("archive.zip.001.中文混淆");
        let second = temp.path().join("archive.zip.002.中文混淆");
        std::fs::write(&first, b"one").unwrap();
        std::fs::write(&second, b"two").unwrap();
        std::fs::write(temp.path().join("other.txt"), b"ignore").unwrap();

        let info = VolumeDetector::detect(&second).unwrap();

        assert!(info.is_complete);
        assert_eq!(info.parts.len(), 2);
        assert_eq!(info.parts[0], first.to_string_lossy().to_string());
        assert_eq!(info.parts[1], second.to_string_lossy().to_string());
    }

    #[test]
    fn reports_missing_obfuscated_volume_parts() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("archive.zip.001.删除");
        let third = temp.path().join("archive.zip.003.删除");
        std::fs::write(&first, b"one").unwrap();
        std::fs::write(&third, b"three").unwrap();

        let info = VolumeDetector::detect(&first).unwrap();

        assert!(!info.is_complete);
        assert_eq!(info.missing, vec!["archive.zip.002"]);
    }
}

pub struct VolumeDetector;

impl VolumeDetector {
    pub fn detect(path: &Path) -> Option<VolumeInfo> {
        let parent = path.parent()?;
        let filename = path.file_name()?.to_string_lossy().to_string();

        if let Some(info) = Self::detect_dot_number_suffix(parent, &filename) {
            return Some(info);
        }

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

    fn detect_dot_number_suffix(parent: &Path, filename: &str) -> Option<VolumeInfo> {
        let re = regex::Regex::new(r"^(.+?)\.(\d{3})(?:\..+)?$").ok()?;
        let caps = re.captures(filename)?;
        let base = caps.get(1)?.as_str();
        let _selected_num: u32 = caps.get(2)?.as_str().parse().ok()?;

        let mut parts = BTreeMap::new();
        for entry in std::fs::read_dir(parent).ok()? {
            let entry = entry.ok()?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Some(caps) = re.captures(&name) else {
                continue;
            };
            if caps.get(1)?.as_str() != base {
                continue;
            }
            let num: u32 = caps.get(2)?.as_str().parse().ok()?;
            parts.insert(num, entry.path());
        }

        Self::build_info(parts, |num| format!("{}.{:03}", base, num))
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

        Self::build_info_from_paths(parts, |_| String::new())
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

        Self::build_info_from_paths(parts, |_| String::new())
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

        Self::build_info_from_paths(parts, |_| String::new())
    }

    fn build_info_from_paths<F>(parts: Vec<PathBuf>, missing_name: F) -> Option<VolumeInfo>
    where
        F: Fn(u32) -> String,
    {
        if parts.len() <= 1 {
            return None;
        }
        let numbered = parts
            .into_iter()
            .enumerate()
            .map(|(index, path)| ((index + 1) as u32, path))
            .collect();
        Self::build_info(numbered, missing_name)
    }

    fn build_info<F>(parts: BTreeMap<u32, PathBuf>, missing_name: F) -> Option<VolumeInfo>
    where
        F: Fn(u32) -> String,
    {
        if parts.len() <= 1 {
            return None;
        }

        let max = *parts.keys().next_back()?;
        let missing = (1..=max)
            .filter(|num| !parts.contains_key(num))
            .map(missing_name)
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();
        let part_strs = parts
            .values()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        Some(VolumeInfo {
            parts: part_strs,
            is_complete: missing.is_empty(),
            missing,
        })
    }
}
