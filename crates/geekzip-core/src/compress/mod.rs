use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

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
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            format: CompressFormat::Zip,
            level: 6,
            password: None,
            create_subfolder: true,
        }
    }
}

pub struct CompressEngine;

impl CompressEngine {
    pub fn compress(paths: &[&Path], output: &Path, opts: &CompressOptions) -> Result<()> {
        match opts.format {
            CompressFormat::Zip => Self::compress_zip(paths, output, opts),
            CompressFormat::TarGz => Self::compress_targz(paths, output),
            CompressFormat::TarBz2 => Self::compress_tarbz2(paths, output),
            CompressFormat::TarXz => Self::compress_tarxz(paths, output),
            CompressFormat::Tar => Self::compress_tar(paths, output),
            CompressFormat::SevenZ => anyhow::bail!("7z compression not yet implemented"),
        }
    }

    fn compress_zip(paths: &[&Path], output: &Path, _opts: &CompressOptions) -> Result<()> {
        let file = fs::File::create(output)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for path in paths {
            if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                zip.start_file(name.as_ref(), options)?;
                let mut f = fs::File::open(path)?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)?;
                zip.write_all(&buf)?;
            } else if path.is_dir() {
                Self::add_dir_to_zip(&mut zip, path, path, &options)?;
            }
        }

        zip.finish()?;
        Ok(())
    }

    fn add_dir_to_zip(
        zip: &mut zip::ZipWriter<fs::File>,
        base: &Path,
        dir: &Path,
        options: &zip::write::SimpleFileOptions,
    ) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(base)?;
            let name = relative.to_string_lossy();

            if path.is_dir() {
                zip.add_directory(name.as_ref(), *options)?;
                Self::add_dir_to_zip(zip, base, &path, options)?;
            } else {
                zip.start_file(name.as_ref(), *options)?;
                let mut f = fs::File::open(&path)?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)?;
                zip.write_all(&buf)?;
            }
        }
        Ok(())
    }

    fn compress_targz(paths: &[&Path], output: &Path) -> Result<()> {
        let file = fs::File::create(output)?;
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut archive = tar::Builder::new(gz);
        for path in paths {
            if path.is_dir() {
                archive.append_dir_all(path, path)?;
            } else {
                let mut f = fs::File::open(path)?;
                archive.append_file(path, &mut f)?;
            }
        }
        archive.finish()?;
        Ok(())
    }

    fn compress_tarbz2(paths: &[&Path], output: &Path) -> Result<()> {
        let file = fs::File::create(output)?;
        let bz2 = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
        let mut archive = tar::Builder::new(bz2);
        for path in paths {
            if path.is_dir() {
                archive.append_dir_all(path, path)?;
            } else {
                let mut f = fs::File::open(path)?;
                archive.append_file(path, &mut f)?;
            }
        }
        archive.finish()?;
        Ok(())
    }

    fn compress_tarxz(paths: &[&Path], output: &Path) -> Result<()> {
        let file = fs::File::create(output)?;
        let xz = xz2::write::XzEncoder::new(file, 6);
        let mut archive = tar::Builder::new(xz);
        for path in paths {
            if path.is_dir() {
                archive.append_dir_all(path, path)?;
            } else {
                let mut f = fs::File::open(path)?;
                archive.append_file(path, &mut f)?;
            }
        }
        archive.finish()?;
        Ok(())
    }

    fn compress_tar(paths: &[&Path], output: &Path) -> Result<()> {
        let file = fs::File::create(output)?;
        let mut archive = tar::Builder::new(file);
        for path in paths {
            if path.is_dir() {
                archive.append_dir_all(path, path)?;
            } else {
                let mut f = fs::File::open(path)?;
                archive.append_file(path, &mut f)?;
            }
        }
        archive.finish()?;
        Ok(())
    }
}