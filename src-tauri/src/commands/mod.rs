mod settings;

pub mod stats;

use geekzip_core::{
    ExtractEngine, ExtractOptions, OverwritePolicy,
    RecursiveExtractor, VolumeDetector,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzeResult {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub format: String,
    pub detected_by: String,
    pub original_extension: Option<String>,
    pub is_volume: bool,
    pub volume_parts: Option<Vec<String>>,
    pub is_encrypted: bool,
}

#[tauri::command]
pub async fn analyze_file(path: String) -> Result<AnalyzeResult, String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("File not found: {}", path));
    }

    let info = geekzip_core::format::detect_format(&p);
    let metadata = std::fs::metadata(&p).map_err(|e| e.to_string())?;

    let vol_info = VolumeDetector::detect(&p);

    Ok(AnalyzeResult {
        path: path.clone(),
        name: p.file_name().unwrap_or_default().to_string_lossy().to_string(),
        size: metadata.len(),
        format: info.format.name().to_string(),
        detected_by: format!("{:?}", info.detected_by),
        original_extension: info.original_extension,
        is_volume: vol_info.is_some(),
        volume_parts: vol_info.map(|v| v.parts),
        is_encrypted: false,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractResultData {
    pub source: String,
    pub target_dir: String,
    pub format: String,
    pub files: Vec<String>,
    pub file_count: usize,
    pub elapsed_ms: u64,
    pub password_used: Option<String>,
}

#[tauri::command]
pub async fn extract_archive(
    path: String,
    target_dir: Option<String>,
    create_subfolder: Option<bool>,
    password: Option<String>,
    delete_after: Option<bool>,
) -> Result<ExtractResultData, String> {
    let opts = ExtractOptions {
        target_dir,
        create_subfolder: create_subfolder.unwrap_or(true),
        overwrite: OverwritePolicy::Rename,
        password,
        delete_after: delete_after.unwrap_or(false),
        open_after: false,
        verify: false,
    };

    let result = ExtractEngine::extract(&PathBuf::from(&path), &opts)
        .map_err(|e| e.to_string())?;

    let file_count = result.files.len();
    Ok(ExtractResultData {
        source: result.source,
        target_dir: result.target_dir,
        format: result.format,
        files: result.files,
        file_count,
        elapsed_ms: result.elapsed_ms,
        password_used: result.password_used,
    })
}

#[tauri::command]
pub async fn extract_smart(
    path: String,
    target_dir: Option<String>,
    recursive: Option<bool>,
    max_depth: Option<u32>,
    password: Option<String>,
    delete_after: Option<bool>,
) -> Result<serde_json::Value, String> {
    let opts = ExtractOptions {
        target_dir: target_dir.clone(),
        create_subfolder: true,
        overwrite: OverwritePolicy::Rename,
        password,
        delete_after: delete_after.unwrap_or(false),
        open_after: false,
        verify: false,
    };

    if recursive.unwrap_or(false) {
        let depth = max_depth.unwrap_or(10);
        let extractor = RecursiveExtractor::new(depth);
        let result = extractor
            .extract_recursive(&PathBuf::from(&path), &opts)
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "type": "recursive",
            "total_layers": result.total_layers,
            "total_files": result.total_files,
            "results": result.results.iter().map(|r| serde_json::json!({
                "source": r.source,
                "target_dir": r.target_dir,
                "format": r.format,
                "file_count": r.files.len(),
                "elapsed_ms": r.elapsed_ms,
            })).collect::<Vec<_>>(),
        }))
    } else {
        let result = ExtractEngine::extract(&PathBuf::from(&path), &opts)
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "type": "single",
            "source": result.source,
            "target_dir": result.target_dir,
            "format": result.format,
            "file_count": result.files.len(),
            "elapsed_ms": result.elapsed_ms,
        }))
    }
}

#[tauri::command]
pub async fn compress_files(
    paths: Vec<String>,
    output: String,
    format: String,
) -> Result<String, String> {
    let fmt = match format.as_str() {
        "zip" => geekzip_core::CompressFormat::Zip,
        "tar.gz" | "tgz" => geekzip_core::CompressFormat::TarGz,
        "tar.bz2" | "tbz2" => geekzip_core::CompressFormat::TarBz2,
        "tar.xz" | "txz" => geekzip_core::CompressFormat::TarXz,
        "tar" => geekzip_core::CompressFormat::Tar,
        _ => return Err(format!("Unknown format: {}", format)),
    };

    let opts = geekzip_core::CompressOptions {
        format: fmt,
        level: 6,
        password: None,
        create_subfolder: false,
    };

    let path_refs: Vec<&std::path::Path> = paths.iter().map(std::path::Path::new).collect();

    geekzip_core::CompressEngine::compress(&path_refs, &PathBuf::from(&output), &opts)
        .map_err(|e| e.to_string())?;

    Ok(output)
}

#[tauri::command]
pub async fn get_settings() -> Result<settings::AppSettings, String> {
    Ok(settings::AppSettings::load())
}

#[tauri::command]
pub async fn save_settings(settings: settings::AppSettings) -> Result<(), String> {
    settings.save()
}

#[tauri::command]
pub async fn reset_settings() -> Result<settings::AppSettings, String> {
    let default = settings::AppSettings::default();
    default.save()?;
    Ok(default)
}