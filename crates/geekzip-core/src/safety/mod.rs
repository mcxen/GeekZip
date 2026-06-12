use anyhow::{bail, Result};
use std::path::Path;

const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024 * 1024; // 10GB
const DEFAULT_MAX_TOTAL_SIZE: u64 = 50 * 1024 * 1024 * 1024; // 50GB
const DEFAULT_MAX_COMPRESSION_RATIO: f64 = 1000.0;

pub struct SafetyGuard {
    pub max_file_size: u64,
    pub max_total_size: u64,
    pub max_compression_ratio: f64,
}

impl Default for SafetyGuard {
    fn default() -> Self {
        Self {
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            max_total_size: DEFAULT_MAX_TOTAL_SIZE,
            max_compression_ratio: DEFAULT_MAX_COMPRESSION_RATIO,
        }
    }
}

impl SafetyGuard {
    pub fn check_file_size(&self, path: &Path) -> Result<()> {
        let metadata = std::fs::metadata(path)?;
        let size = metadata.len();
        if size > self.max_file_size {
            bail!(
                "File too large: {} bytes (limit: {} bytes)",
                size,
                self.max_file_size
            );
        }
        Ok(())
    }

    pub fn check_compression_ratio(&self, compressed: u64, uncompressed: u64) -> Result<()> {
        if compressed == 0 {
            return Ok(());
        }
        let ratio = uncompressed as f64 / compressed as f64;
        if ratio > self.max_compression_ratio {
            bail!(
                "Suspicious compression ratio: {:.1}x (limit: {:.0}x). Possible zip bomb.",
                ratio,
                self.max_compression_ratio
            );
        }
        Ok(())
    }

    pub fn check_path_traversal(entry_path: &str) -> bool {
        entry_path.contains("..") || entry_path.starts_with('/')
    }
}