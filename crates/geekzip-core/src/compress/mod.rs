use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crate::task::{OperationControl, ProgressCallback, ProgressReader, ProgressUpdate};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressFormat {
    Zip,
    TarGz,
    TarBz2,
    TarXz,
    Tar,
    SevenZ,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressOptions {
    pub format: CompressFormat,
    pub level: u32,
    pub password: Option<String>,
    pub create_subfolder: bool,
    #[serde(default)]
    pub volume_size_mb: Option<u64>,
    #[serde(default)]
    pub obfuscate_suffix: Option<String>,
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            format: CompressFormat::Zip,
            level: 6,
            password: None,
            create_subfolder: true,
            volume_size_mb: None,
            obfuscate_suffix: None,
        }
    }
}

pub struct CompressEngine;

impl CompressEngine {
    pub fn compress(paths: &[&Path], output: &Path, opts: &CompressOptions) -> Result<()> {
        Self::compress_with_progress(paths, output, opts, OperationControl::new(), None)
    }

    pub fn compress_with_progress(
        paths: &[&Path],
        output: &Path,
        opts: &CompressOptions,
        control: OperationControl,
        progress: Option<ProgressCallback<'_>>,
    ) -> Result<()> {
        let plan = Self::collect_input_files(paths)?;
        let total_bytes = plan.iter().map(|(_, size)| *size).sum::<u64>();
        Self::emit_progress(progress, "准备压缩", None, 0, total_bytes, 0, plan.len());
        match opts.format {
            CompressFormat::Zip => {
                Self::compress_zip(paths, output, opts, &control, progress, total_bytes)
            }
            CompressFormat::TarGz => Self::compress_targz(
                paths,
                output,
                opts,
                &control,
                progress,
                total_bytes,
                plan.len(),
            ),
            CompressFormat::TarBz2 => Self::compress_tarbz2(
                paths,
                output,
                opts,
                &control,
                progress,
                total_bytes,
                plan.len(),
            ),
            CompressFormat::TarXz => Self::compress_tarxz(
                paths,
                output,
                opts,
                &control,
                progress,
                total_bytes,
                plan.len(),
            ),
            CompressFormat::Tar => {
                Self::compress_tar(paths, output, &control, progress, total_bytes, plan.len())
            }
            CompressFormat::SevenZ => anyhow::bail!("7z compression not yet implemented"),
        }?;
        control.wait_if_paused()?;
        Self::split_volume_if_needed(output, opts)?;
        Self::emit_progress(
            progress,
            "压缩完成",
            None,
            total_bytes,
            total_bytes,
            plan.len(),
            plan.len(),
        );
        Ok(())
    }

    fn compress_zip(
        paths: &[&Path],
        output: &Path,
        opts: &CompressOptions,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
        total_bytes: u64,
    ) -> Result<()> {
        let file = fs::File::create(output)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(opts.level.clamp(1, 9) as i64));
        let done = Arc::new(AtomicU64::new(0));
        let files_done = Arc::new(AtomicU64::new(0));
        let total_files = Self::collect_input_files(paths)?.len();

        for path in paths {
            let name = Self::archive_name(path)?;
            if path.is_file() {
                zip.start_file(Self::zip_name(&name), options)?;
                Self::copy_file_to_writer(
                    path,
                    &mut zip,
                    control,
                    progress,
                    &done,
                    &files_done,
                    total_bytes,
                    total_files,
                    "正在压缩",
                )?;
            } else if path.is_dir() {
                zip.add_directory(Self::zip_dir_name(&name), options)?;
                Self::add_dir_to_zip(
                    &mut zip,
                    path,
                    &name,
                    &options,
                    control,
                    progress,
                    &done,
                    &files_done,
                    total_bytes,
                    total_files,
                )?;
            }
        }

        zip.finish()?;
        Ok(())
    }

    fn add_dir_to_zip(
        zip: &mut zip::ZipWriter<fs::File>,
        dir: &Path,
        archive_dir: &Path,
        options: &zip::write::SimpleFileOptions,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
        done: &Arc<AtomicU64>,
        files_done: &Arc<AtomicU64>,
        total_bytes: u64,
        total_files: usize,
    ) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = archive_dir.join(entry.file_name());

            if path.is_dir() {
                zip.add_directory(Self::zip_dir_name(&name), *options)?;
                Self::add_dir_to_zip(
                    zip,
                    &path,
                    &name,
                    options,
                    control,
                    progress,
                    done,
                    files_done,
                    total_bytes,
                    total_files,
                )?;
            } else {
                zip.start_file(Self::zip_name(&name), *options)?;
                Self::copy_file_to_writer(
                    &path,
                    zip,
                    control,
                    progress,
                    done,
                    files_done,
                    total_bytes,
                    total_files,
                    "正在压缩",
                )?;
            }
        }
        Ok(())
    }

    fn compress_targz(
        paths: &[&Path],
        output: &Path,
        opts: &CompressOptions,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
        total_bytes: u64,
        total_files: usize,
    ) -> Result<()> {
        let file = fs::File::create(output)?;
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::new(Self::level(opts)));
        let mut archive = tar::Builder::new(gz);
        Self::append_paths_to_tar(
            &mut archive,
            paths,
            control,
            progress,
            total_bytes,
            total_files,
        )?;
        archive.finish()?;
        Ok(())
    }

    fn compress_tarbz2(
        paths: &[&Path],
        output: &Path,
        opts: &CompressOptions,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
        total_bytes: u64,
        total_files: usize,
    ) -> Result<()> {
        let file = fs::File::create(output)?;
        let bz2 = bzip2::write::BzEncoder::new(file, bzip2::Compression::new(Self::level(opts)));
        let mut archive = tar::Builder::new(bz2);
        Self::append_paths_to_tar(
            &mut archive,
            paths,
            control,
            progress,
            total_bytes,
            total_files,
        )?;
        archive.finish()?;
        Ok(())
    }

    fn compress_tarxz(
        paths: &[&Path],
        output: &Path,
        opts: &CompressOptions,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
        total_bytes: u64,
        total_files: usize,
    ) -> Result<()> {
        let file = fs::File::create(output)?;
        let xz = xz2::write::XzEncoder::new(file, Self::level(opts));
        let mut archive = tar::Builder::new(xz);
        Self::append_paths_to_tar(
            &mut archive,
            paths,
            control,
            progress,
            total_bytes,
            total_files,
        )?;
        archive.finish()?;
        Ok(())
    }

    fn compress_tar(
        paths: &[&Path],
        output: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
        total_bytes: u64,
        total_files: usize,
    ) -> Result<()> {
        let file = fs::File::create(output)?;
        let mut archive = tar::Builder::new(file);
        Self::append_paths_to_tar(
            &mut archive,
            paths,
            control,
            progress,
            total_bytes,
            total_files,
        )?;
        archive.finish()?;
        Ok(())
    }

    fn append_paths_to_tar<W: Write>(
        archive: &mut tar::Builder<W>,
        paths: &[&Path],
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
        total_bytes: u64,
        total_files: usize,
    ) -> Result<()> {
        let done = Arc::new(AtomicU64::new(0));
        let files_done = Arc::new(AtomicU64::new(0));
        for path in paths {
            let name = Self::archive_name(path)?;
            if path.is_dir() {
                archive.append_dir(&name, path)?;
                Self::append_dir_to_tar(
                    archive,
                    path,
                    &name,
                    control,
                    progress,
                    &done,
                    &files_done,
                    total_bytes,
                    total_files,
                )?;
            } else {
                Self::append_file_to_tar(
                    archive,
                    path,
                    &name,
                    control,
                    progress,
                    &done,
                    &files_done,
                    total_bytes,
                    total_files,
                )?;
            }
        }
        Ok(())
    }

    fn append_dir_to_tar<W: Write>(
        archive: &mut tar::Builder<W>,
        dir: &Path,
        archive_dir: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
        done: &Arc<AtomicU64>,
        files_done: &Arc<AtomicU64>,
        total_bytes: u64,
        total_files: usize,
    ) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = archive_dir.join(entry.file_name());
            if path.is_dir() {
                archive.append_dir(&name, &path)?;
                Self::append_dir_to_tar(
                    archive,
                    &path,
                    &name,
                    control,
                    progress,
                    done,
                    files_done,
                    total_bytes,
                    total_files,
                )?;
            } else {
                Self::append_file_to_tar(
                    archive,
                    &path,
                    &name,
                    control,
                    progress,
                    done,
                    files_done,
                    total_bytes,
                    total_files,
                )?;
            }
        }
        Ok(())
    }

    fn append_file_to_tar<W: Write>(
        archive: &mut tar::Builder<W>,
        path: &Path,
        archive_name: &Path,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
        done: &Arc<AtomicU64>,
        files_done: &Arc<AtomicU64>,
        total_bytes: u64,
        total_files: usize,
    ) -> Result<()> {
        let file = fs::File::open(path)?;
        let size = file.metadata()?.len();
        let done_for_reader = done.clone();
        let path_for_reader = path.to_string_lossy().to_string();
        let files_current = files_done.load(Ordering::SeqCst) as usize;
        let mut reader = ProgressReader::new(file, control.clone(), move |read| {
            let bytes_done = done_for_reader.fetch_add(read, Ordering::SeqCst) + read;
            if let Some(callback) = progress {
                callback(ProgressUpdate {
                    phase: "正在压缩".into(),
                    current_path: Some(path_for_reader.clone()),
                    bytes_done,
                    total_bytes,
                    files_done: files_current,
                    total_files,
                });
            }
        });
        let mut header = tar::Header::new_gnu();
        header.set_metadata(&fs::metadata(path)?);
        header.set_size(size);
        header.set_cksum();
        archive.append_data(&mut header, archive_name, &mut reader)?;
        files_done.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn copy_file_to_writer<W: Write>(
        path: &Path,
        writer: &mut W,
        control: &OperationControl,
        progress: Option<ProgressCallback<'_>>,
        done: &Arc<AtomicU64>,
        files_done: &Arc<AtomicU64>,
        total_bytes: u64,
        total_files: usize,
        phase: &str,
    ) -> Result<()> {
        let mut file = fs::File::open(path)?;
        let mut buffer = [0u8; 128 * 1024];
        loop {
            control.wait_if_paused()?;
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            writer.write_all(&buffer[..read])?;
            let bytes_done = done.fetch_add(read as u64, Ordering::SeqCst) + read as u64;
            Self::emit_progress(
                progress,
                phase,
                Some(path),
                bytes_done,
                total_bytes,
                files_done.load(Ordering::SeqCst) as usize,
                total_files,
            );
        }
        files_done.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn collect_input_files(paths: &[&Path]) -> Result<Vec<(PathBuf, u64)>> {
        let mut files = Vec::new();
        for path in paths {
            if path.is_file() {
                files.push((path.to_path_buf(), fs::metadata(path)?.len()));
            } else if path.is_dir() {
                for entry in walkdir::WalkDir::new(path)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(std::result::Result::ok)
                    .filter(|entry| entry.file_type().is_file())
                {
                    files.push((entry.path().to_path_buf(), entry.metadata()?.len()));
                }
            }
        }
        Ok(files)
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

    fn archive_name(path: &Path) -> Result<PathBuf> {
        path.file_name()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("Cannot determine archive name for {}", path.display()))
    }

    fn zip_name(path: &Path) -> String {
        path.components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }

    fn zip_dir_name(path: &Path) -> String {
        let mut name = Self::zip_name(path);
        if !name.ends_with('/') {
            name.push('/');
        }
        name
    }

    fn level(opts: &CompressOptions) -> u32 {
        opts.level.clamp(1, 9)
    }

    fn split_volume_if_needed(output: &Path, opts: &CompressOptions) -> Result<()> {
        let Some(size_mb) = opts.volume_size_mb else {
            return Ok(());
        };
        let volume_size = size_mb.saturating_mul(1024 * 1024);
        if volume_size == 0 {
            return Ok(());
        }

        let total_size = fs::metadata(output)?.len();
        if total_size <= volume_size {
            return Ok(());
        }

        let suffix = opts.obfuscate_suffix.as_deref().unwrap_or("").trim();
        let mut input = fs::File::open(output)?;
        let mut index = 1u32;
        loop {
            let part_path = Self::volume_part_path(output, index, suffix);
            let mut part = fs::File::create(part_path)?;
            let written = std::io::copy(
                &mut std::io::Read::by_ref(&mut input).take(volume_size),
                &mut part,
            )?;
            if written == 0 {
                break;
            }
            index += 1;
        }
        fs::remove_file(output)?;
        Ok(())
    }

    fn volume_part_path(output: &Path, index: u32, suffix: &str) -> PathBuf {
        let part_name = match output.file_name().and_then(|name| name.to_str()) {
            Some(name) if suffix.is_empty() => format!("{name}.{index:03}"),
            Some(name) => format!("{name}.{index:03}.{suffix}"),
            None if suffix.is_empty() => format!("{index:03}"),
            None => format!("{index:03}.{suffix}"),
        };
        output.with_file_name(part_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_keeps_selected_directory_as_archive_root() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("file.txt"), "hello").unwrap();
        let output = temp.path().join("out.zip");
        let opts = CompressOptions {
            format: CompressFormat::Zip,
            ..Default::default()
        };

        CompressEngine::compress(&[source.as_path()], &output, &opts).unwrap();

        let file = fs::File::open(output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name("source/").is_ok());
        assert!(archive.by_name("source/file.txt").is_ok());
        assert!(archive.by_name("file.txt").is_err());
    }

    #[test]
    fn targz_uses_relative_archive_names() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("file.txt"), "hello").unwrap();
        let output = temp.path().join("out.tar.gz");
        let opts = CompressOptions {
            format: CompressFormat::TarGz,
            level: 9,
            ..Default::default()
        };

        CompressEngine::compress(&[source.as_path()], &output, &opts).unwrap();

        let file = fs::File::open(output).unwrap();
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        let names = archive
            .entries()
            .unwrap()
            .map(|entry| entry.unwrap().path().unwrap().to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert!(names.iter().any(|name| name == "source/file.txt"));
        assert!(names.iter().all(|name| !Path::new(name).is_absolute()));
    }
}
