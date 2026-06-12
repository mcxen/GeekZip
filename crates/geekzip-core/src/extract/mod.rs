use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::format::{detect_format, ArchiveFormat};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOptions {
    pub target_dir: Option<String>,
    pub create_subfolder: bool,
    pub overwrite: OverwritePolicy,
    pub password: Option<String>,
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

        let result = match info.format {
            ArchiveFormat::Zip => Self::extract_zip(path, &target_dir, opts)?,
            ArchiveFormat::Tar => Self::extract_tar(path, &target_dir)?,
            ArchiveFormat::TarGz => Self::extract_targz(path, &target_dir)?,
            ArchiveFormat::TarBz2 => Self::extract_tarbz2(path, &target_dir)?,
            ArchiveFormat::TarXz => Self::extract_tarxz(path, &target_dir)?,
            ArchiveFormat::Gz => Self::extract_gz(path, &target_dir)?,
            ArchiveFormat::Bz2 => Self::extract_bz2(path, &target_dir)?,
            ArchiveFormat::Xz => Self::extract_xz(path, &target_dir)?,
            ArchiveFormat::Zstd => Self::extract_zstd(path, &target_dir)?,
            ArchiveFormat::SevenZ => Self::extract_7z(path, &target_dir, opts)?,
            ArchiveFormat::Lz4 => Self::extract_lz4(path, &target_dir)?,
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
            password_used: opts.password.clone(),
        })
    }

    fn resolve_target_dir(path: &Path, opts: &ExtractOptions) -> Result<PathBuf> {
        if let Some(ref dir) = opts.target_dir {
            return Ok(PathBuf::from(dir));
        }

        let parent = path.parent().unwrap_or(Path::new("."));
        if opts.create_subfolder {
            let stem = path.file_stem()
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

    fn extract_zip(path: &Path, target: &Path, opts: &ExtractOptions) -> Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        if let Some(ref pwd) = opts.password {
            archive = Self::try_zip_password(path, pwd)?;
        }

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
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

        Self::collect_files(target)
    }

    fn try_zip_password(path: &Path, pwd: &str) -> Result<zip::ZipArchive<fs::File>> {
        let file = fs::File::open(path)?;
        let archive = zip::ZipArchive::new(file)?;
        Ok(archive)
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

        let stem = path.file_stem()
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

        let stem = path.file_stem()
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

        let stem = path.file_stem()
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

        let stem = path.file_stem()
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

        let stem = path.file_stem()
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