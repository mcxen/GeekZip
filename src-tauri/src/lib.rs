mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::analyze_file,
            commands::extract_archive,
            commands::extract_smart,
            commands::compress_files,
            commands::get_settings,
            commands::save_settings,
            commands::reset_settings,
            commands::stats::get_system_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GeekZip");
}