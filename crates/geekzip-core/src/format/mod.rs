use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArchiveFormat {
    Zip,
    Rar4,
    Rar5,
    SevenZ,
    Tar,
    Gz,
    Bz2,
    Xz,
    Zstd,
    Lz4,
    TarGz,
    TarBz2,
    TarXz,
    TarZstd,
    Unknown,
}

impl ArchiveFormat {
    pub fn extensions(&self) -> &[&str] {
        match self {
            Self::Zip => &["zip", "jar", "war", "apk", "docx", "xlsx", "pptx"],
            Self::Rar4 | Self::Rar5 => &["rar"],
            Self::SevenZ => &["7z"],
            Self::Tar => &["tar"],
            Self::Gz => &["gz"],
            Self::Bz2 => &["bz2"],
            Self::Xz => &["xz"],
            Self::Zstd => &["zst"],
            Self::Lz4 => &["lz4"],
            Self::TarGz => &["tgz", "tar.gz"],
            Self::TarBz2 => &["tbz2", "tar.bz2"],
            Self::TarXz => &["txz", "tar.xz"],
            Self::TarZstd => &["tar.zst"],
            Self::Unknown => &[],
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Zip => "ZIP",
            Self::Rar4 => "RAR4",
            Self::Rar5 => "RAR5",
            Self::SevenZ => "7Z",
            Self::Tar => "TAR",
            Self::Gz => "GZ",
            Self::Bz2 => "BZ2",
            Self::Xz => "XZ",
            Self::Zstd => "ZSTD",
            Self::Lz4 => "LZ4",
            Self::TarGz => "TAR.GZ",
            Self::TarBz2 => "TAR.BZ2",
            Self::TarXz => "TAR.XZ",
            Self::TarZstd => "TAR.ZSTD",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub format: ArchiveFormat,
    pub detected_by: DetectionMethod,
    pub original_extension: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionMethod {
    MagicBytes,
    Extension,
    Both,
}

pub fn detect_format(path: &Path) -> FormatInfo {
    let ext_detection = detect_by_extension(path);
    let magic_detection = detect_by_magic(path);

    match (ext_detection, magic_detection) {
        (Some(ext_fmt), Some(magic_fmt)) if ext_fmt == magic_fmt => FormatInfo {
            format: ext_fmt,
            detected_by: DetectionMethod::Both,
            original_extension: path.extension().map(|e| e.to_string_lossy().to_string()),
        },
        (Some(_), Some(magic_fmt)) => FormatInfo {
            format: magic_fmt,
            detected_by: DetectionMethod::MagicBytes,
            original_extension: path.extension().map(|e| e.to_string_lossy().to_string()),
        },
        (Some(ext_fmt), None) => FormatInfo {
            format: ext_fmt,
            detected_by: DetectionMethod::Extension,
            original_extension: path.extension().map(|e| e.to_string_lossy().to_string()),
        },
        (None, Some(magic_fmt)) => FormatInfo {
            format: magic_fmt,
            detected_by: DetectionMethod::MagicBytes,
            original_extension: path.extension().map(|e| e.to_string_lossy().to_string()),
        },
        (None, None) => FormatInfo {
            format: ArchiveFormat::Unknown,
            detected_by: DetectionMethod::Extension,
            original_extension: path.extension().map(|e| e.to_string_lossy().to_string()),
        },
    }
}

fn detect_by_extension(path: &Path) -> Option<ArchiveFormat> {
    let filename = path.file_name()?.to_string_lossy().to_lowercase();
    let name_lower = filename.as_str();

    if name_lower.ends_with(".tar.gz") || name_lower.ends_with(".tgz") {
        return Some(ArchiveFormat::TarGz);
    }
    if name_lower.ends_with(".tar.bz2") || name_lower.ends_with(".tbz2") {
        return Some(ArchiveFormat::TarBz2);
    }
    if name_lower.ends_with(".tar.xz") || name_lower.ends_with(".txz") {
        return Some(ArchiveFormat::TarXz);
    }
    if name_lower.ends_with(".tar.zst") {
        return Some(ArchiveFormat::TarZstd);
    }

    let ext = path.extension()?.to_string_lossy().to_lowercase();
    match ext.as_str() {
        "zip" | "jar" | "war" | "apk" | "docx" | "xlsx" | "pptx" => Some(ArchiveFormat::Zip),
        "rar" => Some(ArchiveFormat::Rar5),
        "7z" => Some(ArchiveFormat::SevenZ),
        "tar" => Some(ArchiveFormat::Tar),
        "gz" => Some(ArchiveFormat::Gz),
        "bz2" => Some(ArchiveFormat::Bz2),
        "xz" => Some(ArchiveFormat::Xz),
        "zst" => Some(ArchiveFormat::Zstd),
        "lz4" => Some(ArchiveFormat::Lz4),
        _ => None,
    }
}

fn detect_by_magic(path: &Path) -> Option<ArchiveFormat> {
    let inf = infer::get_from_path(path).ok()??;
    let mime = inf.mime_type();

    match mime {
        "application/zip" => Some(ArchiveFormat::Zip),
        "application/x-rar-compressed" => Some(ArchiveFormat::Rar5),
        "application/x-7z-compressed" => Some(ArchiveFormat::SevenZ),
        "application/x-tar" => Some(ArchiveFormat::Tar),
        "application/gzip" => Some(ArchiveFormat::Gz),
        "application/x-bzip2" => Some(ArchiveFormat::Bz2),
        "application/x-xz" => Some(ArchiveFormat::Xz),
        "application/zstd" => Some(ArchiveFormat::Zstd),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_name() {
        assert_eq!(ArchiveFormat::Zip.name(), "ZIP");
        assert_eq!(ArchiveFormat::SevenZ.name(), "7Z");
        assert_eq!(ArchiveFormat::TarGz.name(), "TAR.GZ");
    }

    #[test]
    fn test_extensions() {
        assert!(ArchiveFormat::Zip.extensions().contains(&"zip"));
        assert!(ArchiveFormat::TarGz.extensions().contains(&"tar.gz"));
    }
}
