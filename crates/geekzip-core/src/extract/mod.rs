use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::format::{detect_format, ArchiveFormat};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOptions {
    pub target_dir: Option<String>,
    pub create_subfolder: bool,
    pub overwrite: OverwritePolicy,
    pub password: Option<String>,
    #[serde(default)]
    pub password_candidates: Vec<String>,
    pub delete_after: bool,
    pub open_after: bool,
    pub verify: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OverwritePolicy {
    Skip,
    Overwrite,
    Rename,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            target_dir: None,
            create_subfolder: true,
            overwrite: OverwritePolicy::Rename,
            password: None,
            password_candidates: Vec::new(),
            delete_after: false,
            open_after: false,
            verify: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub source: String,
    pub target_dir: String,
    pub format: String,
    pub files: Vec<String>,
    pub file_count: usize,
    pub total_size: u64,
    pub elapsed_ms: u64,
    pub password_used: Option<String>,
}

pub struct ExtractEngine;

impl ExtractEngine {
    pub fn extract(path: &Path, opts: &ExtractOptions) -> Result<ExtractResult> {
        let start = std::time::Instant::now();
        let info = detect_format(path);

        if info.format == ArchiveFormat::Unknown {
            bail!("Unknown archive format: {:?}", path);
        }

        let target_dir = Self::resolve_target_dir(path, opts)?;
        fs::create_dir_all(&target_dir)?;

        let (result, password_used) = match info.format {
            ArchiveFormat::Zip => Self::extract_zip(path, &target_dir, opts)?,
            ArchiveFormat::Tar => (Self::extract_tar(path, &target_dir)?, None),
            ArchiveFormat::TarGz => (Self::extract_targz(path, &target_dir)?, None),
            ArchiveFormat::TarBz2 => (Self::extract_tarbz2(path, &target_dir)?, None),
            ArchiveFormat::TarXz => (Self::extract_tarxz(path, &target_dir)?, None),
            ArchiveFormat::Gz => (Self::extract_gz(path, &target_dir)?, None),
            ArchiveFormat::Bz2 => (Self::extract_bz2(path, &target_dir)?, None),
            ArchiveFormat::Xz => (Self::extract_xz(path, &target_dir)?, None),
            ArchiveFormat::Zstd => (Self::extract_zstd(path, &target_dir)?, None),
            ArchiveFormat::SevenZ => (Self::extract_7z(path, &target_dir, opts)?, None),
            ArchiveFormat::Lz4 => (Self::extract_lz4(path, &target_dir)?, None),
            _ => bail!("Unsupported format: {:?}", info.format.name()),
        };

        if opts.delete_after {
            let _ = fs::remove_file(path);
        }

        let elapsed = start.elapsed();

        Ok(ExtractResult {
            source: path.to_string_lossy().to_string(),
            target_dir: target_dir.to_string_lossy().to_string(),
            format: info.format.name().to_string(),
            files: result,
            file_count: 0,
            total_size: 0,
            elapsed_ms: elapsed.as_millis() as u64,
            password_used,
        })
    }

    fn resolve_target_dir(path: &Path, opts: &ExtractOptions) -> Result<PathBuf> {
        if let Some(ref dir) = opts.target_dir {
            return Ok(PathBuf::from(dir));
        }

        let parent = path.parent().unwrap_or(Path::new("."));
        if opts.create_subfolder {
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let subfolder = parent.join(stem);
            Ok(subfolder)
        } else {
            Ok(parent.to_path_buf())
        }
    }

    fn collect_files(dir: &Path) -> Result<Vec<String>> {
        let mut files = Vec::new();
        if dir.exists() {
            for entry in walkdir::WalkDir::new(dir) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    files.push(entry.path().to_string_lossy().to_string());
                }
            }
        }
        Ok(files)
    }

    fn extract_zip(
        path: &Path,
        target: &Path,
        opts: &ExtractOptions,
    ) -> Result<(Vec<String>, Option<String>)> {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let encrypted = (0..archive.len()).any(|index| {
            archive
                .by_index_raw(index)
                .map(|entry| entry.encrypted())
                .unwrap_or(false)
        });
        let password_used = if encrypted {
            Some(Self::find_zip_password(path, opts)?)
        } else {
            None
        };

        for i in 0..archive.len() {
            let mut entry = if let Some(password) = password_used.as_ref() {
                archive.by_index_decrypt(i, password.as_bytes())?
            } else {
                archive.by_index(i)?
            };
            let entry_path = match entry.enclosed_name() {
                Some(p) => p.to_owned(),
                None => continue,
            };
            let out_path = target.join(&entry_path);

            if entry.is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut out_file = fs::File::create(&out_path)?;
                std::io::copy(&mut entry, &mut out_file)?;
            }
        }

        Ok((Self::collect_files(target)?, password_used))
    }

    fn find_zip_password(path: &Path, opts: &ExtractOptions) -> Result<String> {
        let mut candidates = Vec::new();
        if let Some(password) = opts.password.as_ref() {
            candidates.push(password.clone());
        }
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            if let Some(password) = crate::password::PasswordEngine::extract_from_filename(name) {
                candidates.push(password);
            }
        }
        candidates.extend(opts.password_candidates.iter().cloned());
        candidates.extend(
            crate::password::BUILTIN_PASSWORDS
                .iter()
                .map(|value| value.to_string()),
        );

        let mut seen = HashSet::new();
        candidates.retain(|password| !password.is_empty() && seen.insert(password.clone()));

        for password in candidates {
            if Self::validate_zip_password(path, &password).is_ok() {
                return Ok(password);
            }
        }
        bail!("Password required: no matching manual, filename, vault, or built-in password")
    }

    fn validate_zip_password(path: &Path, password: &str) -> Result<()> {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        for index in 0..archive.len() {
            let encrypted = archive.by_index_raw(index)?.encrypted();
            if !encrypted {
                continue;
            }
            let mut entry = archive.by_index_decrypt(index, password.as_bytes())?;
            if entry.is_dir() {
                continue;
            }
            let mut sink = Vec::new();
            entry.read_to_end(&mut sink)?;
            return Ok(());
        }
        bail!("Archive has no encrypted file entries")
    }

    fn extract_tar(path: &Path, target: &Path) -> Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let mut archive = tar::Archive::new(file);
        archive.unpack(target)?;
        Self::collect_files(target)
    }

    fn extract_targz(path: &Path, target: &Path) -> Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);
        archive.unpack(target)?;
        Self::collect_files(target)
    }

    fn extract_tarbz2(path: &Path, target: &Path) -> Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let bz2 = bzip2::read::BzDecoder::new(file);
        let mut archive = tar::Archive::new(bz2);
        archive.unpack(target)?;
        Self::collect_files(target)
    }

    fn extract_tarxz(path: &Path, target: &Path) -> Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let xz = xz2::read::XzDecoder::new(file);
        let mut archive = tar::Archive::new(xz);
        archive.unpack(target)?;
        Self::collect_files(target)
    }

    fn extract_gz(path: &Path, target: &Path) -> Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let mut gz = flate2::read::GzDecoder::new(file);

        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let out_path = target.join(stem);

        let mut out_file = fs::File::create(&out_path)?;
        std::io::copy(&mut gz, &mut out_file)?;

        Ok(vec![out_path.to_string_lossy().to_string()])
    }

    fn extract_bz2(path: &Path, target: &Path) -> Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let mut bz2 = bzip2::read::BzDecoder::new(file);

        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let out_path = target.join(stem);

        let mut out_file = fs::File::create(&out_path)?;
        std::io::copy(&mut bz2, &mut out_file)?;

        Ok(vec![out_path.to_string_lossy().to_string()])
    }

    fn extract_xz(path: &Path, target: &Path) -> Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let mut xz = xz2::read::XzDecoder::new(file);

        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let out_path = target.join(stem);

        let mut out_file = fs::File::create(&out_path)?;
        std::io::copy(&mut xz, &mut out_file)?;

        Ok(vec![out_path.to_string_lossy().to_string()])
    }

    fn extract_zstd(path: &Path, target: &Path) -> Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let mut zst = zstd::Decoder::new(file)?;

        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let out_path = target.join(stem);

        let mut out_file = fs::File::create(&out_path)?;
        std::io::copy(&mut zst, &mut out_file)?;

        Ok(vec![out_path.to_string_lossy().to_string()])
    }

    fn extract_lz4(path: &Path, target: &Path) -> Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let mut lz4 = lz4_flex::frame::FrameDecoder::new(file);

        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let out_path = target.join(stem);

        let mut out_file = fs::File::create(&out_path)?;
        std::io::copy(&mut lz4, &mut out_file)?;

        Ok(vec![out_path.to_string_lossy().to_string()])
    }

    fn extract_7z(path: &Path, target: &Path, _opts: &ExtractOptions) -> Result<Vec<String>> {
        sevenz_rust::decompress_file(path, target)
            .with_context(|| format!("Failed to extract 7z: {:?}", path))?;
        Self::collect_files(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn extracts_aes_zip_with_password_candidate() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("protected.zip");
        let output = temp.path().join("output");
        let file = fs::File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .with_aes_encryption(zip::AesMode::Aes256, "vault-secret");
        writer.start_file("hello.txt", options).unwrap();
        writer.write_all(b"password vault works").unwrap();
        writer.finish().unwrap();

        let opts = ExtractOptions {
            target_dir: Some(output.to_string_lossy().to_string()),
            create_subfolder: false,
            password_candidates: vec!["wrong".into(), "vault-secret".into()],
            ..ExtractOptions::default()
        };
        let result = ExtractEngine::extract(&archive_path, &opts).unwrap();
        assert_eq!(result.password_used.as_deref(), Some("vault-secret"));
        assert_eq!(
            fs::read(output.join("hello.txt")).unwrap(),
            b"password vault works"
        );
    }

    #[test]
    fn rejects_aes_zip_when_passwords_do_not_match() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("protected.zip");
        let file = fs::File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .with_aes_encryption(zip::AesMode::Aes256, "correct");
        writer.start_file("hello.txt", options).unwrap();
        writer.write_all(b"secret").unwrap();
        writer.finish().unwrap();

        let opts = ExtractOptions {
            password_candidates: vec!["wrong".into()],
            ..ExtractOptions::default()
        };
        let error = ExtractEngine::extract(&archive_path, &opts)
            .unwrap_err()
            .to_string();
        assert!(error.contains("no matching"));
    }
}
