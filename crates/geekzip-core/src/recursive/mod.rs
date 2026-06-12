use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::extract::{ExtractEngine, ExtractOptions, ExtractResult};
use crate::format::detect_format;
use crate::safety::SafetyGuard;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveResult {
    pub results: Vec<ExtractResult>,
    pub total_layers: u32,
    pub total_files: usize,
}

pub struct RecursiveExtractor {
    max_depth: u32,
    safety: SafetyGuard,
}

impl RecursiveExtractor {
    pub fn new(max_depth: u32) -> Self {
        Self {
            max_depth,
            safety: SafetyGuard::default(),
        }
    }

    pub fn extract_recursive(&self, path: &Path, opts: &ExtractOptions) -> Result<RecursiveResult> {
        let mut results = Vec::new();
        let mut seen = HashSet::new();
        let canonical = std::fs::canonicalize(path)?;
        seen.insert(canonical);

        self.extract_recursive_inner(path, opts, 0, &mut results, &mut seen)?;

        let total_layers = results.len() as u32;
        let total_files = results.iter().map(|r| r.files.len()).sum();

        Ok(RecursiveResult {
            results,
            total_layers,
            total_files,
        })
    }

    fn extract_recursive_inner(
        &self,
        path: &Path,
        opts: &ExtractOptions,
        depth: u32,
        results: &mut Vec<ExtractResult>,
        seen: &mut HashSet<PathBuf>,
    ) -> Result<()> {
        if depth >= self.max_depth {
            bail!("Max recursion depth ({}) exceeded", self.max_depth);
        }

        let canonical = std::fs::canonicalize(path)?;
        if seen.contains(&canonical) {
            bail!("Circular archive detected: {:?}", path);
        }
        seen.insert(canonical);

        self.safety.check_file_size(path)?;

        let result = ExtractEngine::extract(path, opts)?;
        results.push(result.clone());

        for file_path in result.files.iter() {
            let fp = PathBuf::from(file_path);
            let file_info = detect_format(&fp);
            if file_info.format != crate::format::ArchiveFormat::Unknown {
                if let Ok(canonical_sub) = std::fs::canonicalize(&fp) {
                    if seen.contains(&canonical_sub) {
                        continue;
                    }
                }
                let sub_opts = ExtractOptions {
                    target_dir: Some(fp.parent().unwrap_or(Path::new(".")).to_string_lossy().to_string()),
                    create_subfolder: true,
                    delete_after: opts.delete_after,
                    ..opts.clone()
                };
                let _ = self.extract_recursive_inner(&fp, &sub_opts, depth + 1, results, seen);
            }
        }

        Ok(())
    }
}