use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crate::format::{detect_format, ArchiveFormat};
use crate::task::{OperationControl, ProgressCallback, ProgressReader, ProgressUpdate};
use crate::volume::VolumeInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOptions {
    pub target_dir: Option<String>,
    #[serde(default)]
    pub default_target_dir: Option<String>,
    pub create_subfolder: bool,
    pub overwrite: OverwritePolicy,
    pub password: Option<String>,
    #[serde(default)]
    pub password_candidates: Vec<String>,
    pub delete_after: bool,
    pub open_after: bool,
    pub verify: bool,
    #[serde(default)]
    pub rename_prefixes: Vec<String>,
    #[serde(default)]
    pub flatten_single_root: bool,
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
            default_target_dir: None,
            create_subfolder: true,
            overwrite: OverwritePolicy::Rename,
            password: None,
            password_candidates: Vec::new(),
            delete_after: false,
            open_after: false,
            verify: false,
            rename_prefixes: Vec::new(),
            flatten_single_root: false,
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
        Self::extract_with_progress(path, opts, OperationControl::new(), None)
    }

    pub fn extract_with_progress(
        path: &Path,
        opts: &ExtractOptions,
        control: OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<ExtractResult> {
        let start = std::time::Instant::now();
        if let Some(volume) = crate::volume::VolumeDetector::detect(path) {
            return Self::extract_volume(path, opts, start, volume, &control, progress);
        }

        let target_dir = Self::resolve_target_dir(path, opts)?;
        let total = fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or_default();
        Self::emit_progress(progress, "准备解压", Some(path), 0, total, 0, 1);
        let (result, format, password_used) =
            Self::extract_archive(path, &target_dir, opts, &control, progress)?;

        if opts.delete_after {
            let _ = fs::remove_file(path);
        }

        let elapsed = start.elapsed();

        Ok(ExtractResult {
            source: path.to_string_lossy().to_string(),
            target_dir: target_dir.to_string_lossy().to_string(),
            format,
            files: result,
            file_count: 0,
            total_size: 0,
            elapsed_ms: elapsed.as_millis() as u64,
            password_used,
        })
    }

    fn extract_archive(
        path: &Path,
        target_dir: &Path,
        opts: &ExtractOptions,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<(Vec<String>, String, Option<String>)> {
        let info = detect_format(path);
        if info.format == ArchiveFormat::Unknown {
            bail!("Unknown archive format: {:?}", path);
        }

        fs::create_dir_all(&target_dir)?;

        let (_files, password_used) = match info.format {
            ArchiveFormat::Zip => Self::extract_zip(path, target_dir, opts, control, progress)?,
            ArchiveFormat::Tar => (
                Self::extract_tar(path, target_dir, control, progress)?,
                None,
            ),
            ArchiveFormat::TarGz => (
                Self::extract_targz(path, target_dir, control, progress)?,
                None,
            ),
            ArchiveFormat::TarBz2 => (
                Self::extract_tarbz2(path, target_dir, control, progress)?,
                None,
            ),
            ArchiveFormat::TarXz => (
                Self::extract_tarxz(path, target_dir, control, progress)?,
                None,
            ),
            ArchiveFormat::Gz => (Self::extract_gz(path, target_dir, control, progress)?, None),
            ArchiveFormat::Bz2 => (
                Self::extract_bz2(path, target_dir, control, progress)?,
                None,
            ),
            ArchiveFormat::Xz => (Self::extract_xz(path, target_dir, control, progress)?, None),
            ArchiveFormat::Zstd => (
                Self::extract_zstd(path, target_dir, control, progress)?,
                None,
            ),
            ArchiveFormat::SevenZ => (
                Self::extract_7z(path, target_dir, opts, control, progress)?,
                None,
            ),
            ArchiveFormat::Lz4 => (
                Self::extract_lz4(path, target_dir, control, progress)?,
                None,
            ),
            _ => bail!("Unsupported format: {:?}", info.format.name()),
        };
        let result = Self::post_process(target_dir, opts)?;

        Ok((result, info.format.name().to_string(), password_used))
    }

    fn extract_volume(
        path: &Path,
        opts: &ExtractOptions,
        start: std::time::Instant,
        volume: VolumeInfo,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<ExtractResult> {
        if !volume.is_complete {
            bail!(
                "Incomplete volume archive, missing: {}",
                volume.missing.join(", ")
            );
        }
        let base_name = Self::volume_base_name(path)?;
        let base_path = path.with_file_name(&base_name);
        let target_dir = Self::resolve_target_dir(&base_path, opts)?;
        let temp = tempfile::tempdir()?;
        let combined = temp.path().join(base_name);
        let mut output = fs::File::create(&combined)?;
        let total = volume
            .parts
            .iter()
            .filter_map(|part| fs::metadata(part).ok().map(|metadata| metadata.len()))
            .sum::<u64>();
        let mut done = 0u64;
        for part in &volume.parts {
            let mut input = fs::File::open(Path::new(part))?;
            let mut buffer = [0u8; 128 * 1024];
            loop {
                control.wait_if_paused()?;
                let read = input.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                output.write_all(&buffer[..read])?;
                done += read as u64;
                Self::emit_progress(
                    progress,
                    "合并分卷",
                    Some(Path::new(part)),
                    done,
                    total,
                    0,
                    volume.parts.len(),
                );
            }
        }
        drop(output);

        let (result, format, password_used) =
            Self::extract_archive(&combined, &target_dir, opts, control, progress)?;
        if opts.delete_after {
            for part in &volume.parts {
                let _ = fs::remove_file(Path::new(part));
            }
        }
        let elapsed = start.elapsed();
        Ok(ExtractResult {
            source: path.to_string_lossy().to_string(),
            target_dir: target_dir.to_string_lossy().to_string(),
            format,
            files: result,
            file_count: 0,
            total_size: 0,
            elapsed_ms: elapsed.as_millis() as u64,
            password_used,
        })
    }

    fn volume_base_name(path: &Path) -> Result<String> {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("Cannot determine volume base name"))?;
        let re = regex::Regex::new(r"^(.+)\.\d{3}(?:\..+)?$")?;
        if let Some(caps) = re.captures(filename) {
            return Ok(caps.get(1).unwrap().as_str().to_string());
        }
        Ok(filename.to_string())
    }

    fn resolve_target_dir(path: &Path, opts: &ExtractOptions) -> Result<PathBuf> {
        if let Some(ref dir) = opts.target_dir {
            return Ok(PathBuf::from(dir));
        }

        let parent = opts
            .default_target_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")).to_path_buf());
        if opts.create_subfolder {
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let subfolder = parent.join(stem);
            Ok(subfolder)
        } else {
            Ok(parent)
        }
    }

    fn post_process(target: &Path, opts: &ExtractOptions) -> Result<Vec<String>> {
        if opts.flatten_single_root {
            Self::flatten_single_root(target)?;
        }
        if !opts.rename_prefixes.is_empty() {
            Self::rename_prefixes(target, &opts.rename_prefixes)?;
        }
        Self::collect_files(target)
    }

    fn flatten_single_root(target: &Path) -> Result<()> {
        let entries = fs::read_dir(target)?.collect::<std::result::Result<Vec<_>, _>>()?;
        if entries.len() != 1 || !entries[0].file_type()?.is_dir() {
            return Ok(());
        }

        let root = entries[0].path();
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let destination = Self::unique_path(&target.join(entry.file_name()));
            fs::rename(entry.path(), destination)?;
        }
        let _ = fs::remove_dir(&root);
        Ok(())
    }

    fn rename_prefixes(target: &Path, prefixes: &[String]) -> Result<()> {
        let mut entries = walkdir::WalkDir::new(target)
            .min_depth(1)
            .contents_first(true)
            .into_iter()
            .collect::<std::result::Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.depth()));

        for entry in entries {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let cleaned = Self::strip_prefixes(name, prefixes);
            if cleaned == name || cleaned.is_empty() {
                continue;
            }
            let destination = Self::unique_path(&path.with_file_name(cleaned));
            fs::rename(path, destination)?;
        }
        Ok(())
    }

    fn strip_prefixes(name: &str, prefixes: &[String]) -> String {
        let mut cleaned = name.to_string();
        loop {
            let mut changed = false;
            for prefix in prefixes {
                let prefix = prefix.trim();
                if prefix.is_empty() {
                    continue;
                }
                if let Some(rest) = cleaned.strip_prefix(prefix) {
                    cleaned = rest
                        .trim_start_matches(&[' ', '_', '-', '.', '　'][..])
                        .to_string();
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        cleaned
    }

    fn unique_path(path: &Path) -> PathBuf {
        if !path.exists() {
            return path.to_path_buf();
        }

        let parent = path.parent().unwrap_or(Path::new("."));
        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let ext = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_string());
        for index in 1.. {
            let name = match ext.as_ref() {
                Some(ext) => format!("{stem} ({index}).{ext}"),
                None => format!("{stem} ({index})"),
            };
            let candidate = parent.join(name);
            if !candidate.exists() {
                return candidate;
            }
        }
        unreachable!()
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
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
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
        let total_bytes = (0..archive.len())
            .filter_map(|index| archive.by_index_raw(index).ok().map(|entry| entry.size()))
            .sum::<u64>();
        let total_files = archive.len();
        let mut bytes_done = 0u64;
        let mut files_done = 0usize;

        for i in 0..archive.len() {
            control.wait_if_paused()?;
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
                let mut buffer = [0u8; 128 * 1024];
                loop {
                    control.wait_if_paused()?;
                    let read = entry.read(&mut buffer)?;
                    if read == 0 {
                        break;
                    }
                    out_file.write_all(&buffer[..read])?;
                    bytes_done += read as u64;
                    Self::emit_progress(
                        progress,
                        "正在解压",
                        Some(&out_path),
                        bytes_done,
                        total_bytes,
                        files_done,
                        total_files,
                    );
                }
            }
            files_done += 1;
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

    fn extract_tar(
        path: &Path,
        target: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<Vec<String>> {
        let file = Self::progress_file_reader(path, control, progress, "正在解压")?;
        let mut archive = tar::Archive::new(file);
        archive.unpack(target)?;
        Self::collect_files(target)
    }

    fn extract_targz(
        path: &Path,
        target: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<Vec<String>> {
        let file = Self::progress_file_reader(path, control, progress, "正在解压")?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);
        archive.unpack(target)?;
        Self::collect_files(target)
    }

    fn extract_tarbz2(
        path: &Path,
        target: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<Vec<String>> {
        let file = Self::progress_file_reader(path, control, progress, "正在解压")?;
        let bz2 = bzip2::read::BzDecoder::new(file);
        let mut archive = tar::Archive::new(bz2);
        archive.unpack(target)?;
        Self::collect_files(target)
    }

    fn extract_tarxz(
        path: &Path,
        target: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<Vec<String>> {
        let file = Self::progress_file_reader(path, control, progress, "正在解压")?;
        let xz = xz2::read::XzDecoder::new(file);
        let mut archive = tar::Archive::new(xz);
        archive.unpack(target)?;
        Self::collect_files(target)
    }

    fn extract_gz(
        path: &Path,
        target: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<Vec<String>> {
        let file = Self::progress_file_reader(path, control, progress, "正在解压")?;
        let mut gz = flate2::read::GzDecoder::new(file);

        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let out_path = target.join(stem);

        let mut out_file = fs::File::create(&out_path)?;
        Self::copy_reader_to_file(&mut gz, &mut out_file, control)?;

        Ok(vec![out_path.to_string_lossy().to_string()])
    }

    fn extract_bz2(
        path: &Path,
        target: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<Vec<String>> {
        let file = Self::progress_file_reader(path, control, progress, "正在解压")?;
        let mut bz2 = bzip2::read::BzDecoder::new(file);

        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let out_path = target.join(stem);

        let mut out_file = fs::File::create(&out_path)?;
        Self::copy_reader_to_file(&mut bz2, &mut out_file, control)?;

        Ok(vec![out_path.to_string_lossy().to_string()])
    }

    fn extract_xz(
        path: &Path,
        target: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<Vec<String>> {
        let file = Self::progress_file_reader(path, control, progress, "正在解压")?;
        let mut xz = xz2::read::XzDecoder::new(file);

        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let out_path = target.join(stem);

        let mut out_file = fs::File::create(&out_path)?;
        Self::copy_reader_to_file(&mut xz, &mut out_file, control)?;

        Ok(vec![out_path.to_string_lossy().to_string()])
    }

    fn extract_zstd(
        path: &Path,
        target: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<Vec<String>> {
        let file = Self::progress_file_reader(path, control, progress, "正在解压")?;
        let mut zst = zstd::Decoder::new(file)?;

        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let out_path = target.join(stem);

        let mut out_file = fs::File::create(&out_path)?;
        Self::copy_reader_to_file(&mut zst, &mut out_file, control)?;

        Ok(vec![out_path.to_string_lossy().to_string()])
    }

    fn extract_lz4(
        path: &Path,
        target: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<Vec<String>> {
        let file = Self::progress_file_reader(path, control, progress, "正在解压")?;
        let mut lz4 = lz4_flex::frame::FrameDecoder::new(file);

        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let out_path = target.join(stem);

        let mut out_file = fs::File::create(&out_path)?;
        Self::copy_reader_to_file(&mut lz4, &mut out_file, control)?;

        Ok(vec![out_path.to_string_lossy().to_string()])
    }

    fn extract_7z(
        path: &Path,
        target: &Path,
        _opts: &ExtractOptions,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<Vec<String>> {
        control.wait_if_paused()?;
        let total = fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or_default();
        Self::emit_progress(progress, "正在解压 7z", Some(path), 0, total, 0, 1);
        sevenz_rust::decompress_file(path, target)
            .with_context(|| format!("Failed to extract 7z: {:?}", path))?;
        Self::emit_progress(progress, "正在解压 7z", Some(path), total, total, 1, 1);
        Self::collect_files(target)
    }

    fn progress_file_reader<'a>(
        path: &'a Path,
        control: &'a OperationControl,
        progress: Option<ProgressCallback<'a>>,
        phase: &'static str,
    ) -> Result<ProgressReader<fs::File, impl Fn(u64) + 'a>> {
        let file = fs::File::open(path)?;
        let total = file.metadata()?.len();
        let done = Arc::new(AtomicU64::new(0));
        let done_for_reader = done.clone();
        let path_for_reader = path.to_string_lossy().to_string();
        Ok(ProgressReader::new(file, control.clone(), move |read| {
            let bytes_done = done_for_reader.fetch_add(read, Ordering::SeqCst) + read;
            if let Some(progress) = progress {
                progress(ProgressUpdate {
                    phase: phase.into(),
                    current_path: Some(path_for_reader.clone()),
                    bytes_done,
                    total_bytes: total,
                    files_done: 0,
                    total_files: 1,
                });
            }
        }))
    }

    fn copy_reader_to_file<R: Read>(
        reader: &mut R,
        writer: &mut fs::File,
        control: &OperationControl,
    ) -> Result<()> {
        let mut buffer = [0u8; 128 * 1024];
        loop {
            control.wait_if_paused()?;
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            writer.write_all(&buffer[..read])?;
        }
        Ok(())
    }

    fn emit_progress(
        progress: Option<ProgressCallback<'_>>,
        phase: &str,
        current_path: Option<&Path>,
        bytes_done: u64,
        total_bytes: u64,
        files_done: usize,
        total_files: usize,
    ) {
        if let Some(progress) = progress {
            progress(ProgressUpdate {
                phase: phase.into(),
                current_path: current_path.map(|path| path.to_string_lossy().to_string()),
                bytes_done,
                total_bytes,
                files_done,
                total_files,
            });
        }
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

    #[test]
    fn extracts_obfuscated_split_tar_from_any_part() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let output = temp.path().join("bundle.tar");
        let target = temp.path().join("target");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("large.bin"), vec![7u8; 2 * 1024 * 1024]).unwrap();

        crate::compress::CompressEngine::compress(
            &[source.as_path()],
            &output,
            &crate::compress::CompressOptions {
                format: crate::compress::CompressFormat::Tar,
                volume_size_mb: Some(1),
                obfuscate_suffix: Some("中文混淆".into()),
                ..crate::compress::CompressOptions::default()
            },
        )
        .unwrap();

        let second_part = temp.path().join("bundle.tar.002.中文混淆");
        let opts = ExtractOptions {
            target_dir: Some(target.to_string_lossy().to_string()),
            create_subfolder: false,
            ..ExtractOptions::default()
        };
        ExtractEngine::extract(&second_part, &opts).unwrap();

        assert_eq!(
            fs::read(target.join("source/large.bin")).unwrap(),
            vec![7u8; 2 * 1024 * 1024]
        );
    }

    #[test]
    fn applies_default_target_flatten_and_prefix_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let archive_path = temp.path().join("payload.zip");
        let default_target = temp.path().join("default-output");
        let file = fs::File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file(
                "outer/广告前缀_file.txt",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"clean me").unwrap();
        writer.finish().unwrap();

        let opts = ExtractOptions {
            default_target_dir: Some(default_target.to_string_lossy().to_string()),
            rename_prefixes: vec!["广告前缀_".into()],
            flatten_single_root: true,
            ..ExtractOptions::default()
        };
        let result = ExtractEngine::extract(&archive_path, &opts).unwrap();

        let expected = default_target.join("payload/file.txt");
        assert_eq!(
            result.target_dir,
            default_target.join("payload").to_string_lossy().to_string()
        );
        assert_eq!(fs::read(expected).unwrap(), b"clean me");
    }
}
