use anyhow::Result;
use eframe::egui;
use geekzip_core::{
    compress::{CompressEngine, CompressFormat, CompressOptions},
    extract::{ExtractEngine, ExtractOptions, ExtractResult, OverwritePolicy},
    format::ArchiveFormat,
    task::{OperationControl, ProgressUpdate},
};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;
use tracing_subscriber::EnvFilter;

// ============================================================
// Theme
// ============================================================

#[derive(Clone, PartialEq, serde::Deserialize, serde::Serialize)]
enum Theme {
    Tactical,
    Normal,
}

const TACTICAL_GREEN: egui::Color32 = egui::Color32::from_rgb(0x00, 0xff, 0x95);
const TACTICAL_BG: egui::Color32 = egui::Color32::from_rgb(0x0a, 0x0a, 0x0a);
const TACTICAL_DIM: egui::Color32 = egui::Color32::from_rgb(0x00, 0x60, 0x40);
const BORDER: egui::Color32 = egui::Color32::from_rgb(0x00, 0x80, 0x55);

// ============================================================
// App State
// ============================================================

#[derive(Clone, PartialEq)]
enum Tab {
    Extract,
    Compress,
    Batch,
    Settings,
}

struct OperationState {
    running: bool,
    progress: f32,
    phase: String,
    started: Option<Instant>,
    cancel: Arc<AtomicBool>,
    message: String,
}

impl Default for OperationState {
    fn default() -> Self {
        Self {
            running: false,
            progress: 0.0,
            phase: String::new(),
            started: None,
            cancel: Arc::new(AtomicBool::new(false)),
            message: String::new(),
        }
    }
}

struct GeekZipEgui {
    // Tabs
    active_tab: Tab,

    // Theme
    theme: Theme,

    // Extract state
    extract_archives: Vec<PathBuf>,
    extract_output: Option<PathBuf>,
    extract_overwrite: OverwritePolicy,
    extract_flat: bool,
    extract_detail: String,

    // Compress state
    compress_sources: Vec<PathBuf>,
    compress_output: Option<PathBuf>,
    compress_format: CompressFormat,

    // Batch state
    batch_dir: Option<PathBuf>,
    batch_recursive: bool,

    // Operation
    op: OperationState,
    results: Vec<String>,

    // Settings
    passwords: Vec<String>,
    new_password: String,
}

impl Default for GeekZipEgui {
    fn default() -> Self {
        Self {
            active_tab: Tab::Extract,
            theme: Theme::Tactical,
            extract_archives: Vec::new(),
            extract_output: None,
            extract_overwrite: OverwritePolicy::Ask,
            extract_flat: true,
            extract_detail: String::new(),
            compress_sources: Vec::new(),
            compress_output: None,
            compress_format: CompressFormat::Zip,
            batch_dir: None,
            batch_recursive: true,
            op: OperationState::default(),
            results: Vec::new(),
            passwords: Vec::new(),
            new_password: String::new(),
        }
    }
}

// ============================================================
// Tactical-style custom painting
// ============================================================

fn paint_tactical_frame(ui: &mut egui::Ui, rect: egui::Rect) {
    if ui.ctx().style().visuals.window_fill == TACTICAL_BG {
        // Corner brackets
        let p = egui::Pos2::new;
        let s = 6.0; // bracket size
        let c = BORDER;
        let painter = ui.painter();

        // Top-left
        painter.line_segment([p(rect.left(), rect.top() + s), p(rect.left(), rect.top())], (1.0, c));
        painter.line_segment([p(rect.left(), rect.top()), p(rect.left() + s, rect.top())], (1.0, c));
        // Top-right
        painter.line_segment([p(rect.right() - s, rect.top()), p(rect.right(), rect.top())], (1.0, c));
        painter.line_segment([p(rect.right(), rect.top()), p(rect.right(), rect.top() + s)], (1.0, c));
        // Bottom-left
        painter.line_segment([p(rect.left(), rect.bottom() - s), p(rect.left(), rect.bottom())], (1.0, c));
        painter.line_segment([p(rect.left(), rect.bottom()), p(rect.left() + s, rect.bottom())], (1.0, c));
        // Bottom-right
        painter.line_segment([p(rect.right() - s, rect.top() + rect.height()), p(rect.right(), rect.top() + rect.height())], (1.0, c));
        painter.line_segment([p(rect.right(), rect.bottom() - s), p(rect.right(), rect.bottom())], (1.0, c));
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.2} {}", size, UNITS[unit_idx])
}

// ============================================================
// eframe App
// ============================================================

impl eframe::App for GeekZipEgui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);

        if self.op.running {
            // Request repaint during operations for smooth progress
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_tabs(ui);

            if self.op.running {
                self.render_progress(ui);
            }

            match self.active_tab {
                Tab::Extract => self.render_extract(ui),
                Tab::Compress => self.render_compress(ui),
                Tab::Batch => self.render_batch(ui),
                Tab::Settings => self.render_settings(ui),
            }

            if !self.results.is_empty() {
                self.render_results(ui);
            }
        });
    }
}

impl GeekZipEgui {
    fn apply_theme(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();

        match self.theme {
            Theme::Tactical => {
                style.visuals.window_fill = TACTICAL_BG;
                style.visuals.panel_fill = TACTICAL_BG;
                style.visuals.faint_bg_color = TACTICAL_BG;
                style.visuals.extreme_bg_color = egui::Color32::from_rgb(0x05, 0x05, 0x05);
                style.visuals.code_bg_color = egui::Color32::from_rgb(0x00, 0x20, 0x15);
                style.visuals.warn_fg_color = egui::Color32::from_rgb(0xff, 0xaa, 0x00);
                style.visuals.error_fg_color = egui::Color32::from_rgb(0xff, 0x33, 0x33);
                style.visuals.hyperlink_color = TACTICAL_GREEN;
                style.visuals.selection.bg_fill = egui::Color32::from_rgb(0x00, 0x40, 0x20);
                style.visuals.selection.stroke.color = TACTICAL_GREEN;
                style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(0x0a, 0x0a, 0x0a);
                style.visuals.widgets.noninteractive.fg_stroke.color = BORDER;
                style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(0x00, 0x20, 0x10);
                style.visuals.widgets.inactive.fg_stroke.color = TACTICAL_DIM;
                style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(0x00, 0x30, 0x20);
                style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0x00, 0x50, 0x30);
                style.visuals.override_text_color = Some(TACTICAL_GREEN);

                style.spacing.item_spacing = egui::vec2(8.0, 4.0);
                style.spacing.button_padding = egui::vec2(12.0, 6.0);
            }
            Theme::Normal => {
                *style = egui::Style::default();
            }
        }

        ctx.set_style(style);
    }

    fn render_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (Tab::Extract, "🗜 Extract"),
                (Tab::Compress, "📦 Compress"),
                (Tab::Batch, "🔁 Batch"),
                (Tab::Settings, "⚙ Settings"),
            ];

            for (tab, label) in &tabs {
                let is_active = self.active_tab == *tab;
                let btn = egui::Button::new(egui::RichText::new(*label).size(14.0));
                if ui
                    .add(if is_active {
                        btn.fill(match self.theme {
                            Theme::Tactical => egui::Color32::from_rgb(0x00, 0x50, 0x30),
                            Theme::Normal => egui::Color32::LIGHT_BLUE,
                        })
                    } else {
                        btn
                    })
                    .clicked()
                {
                    self.active_tab = tab.clone();
                }
            }
        });
        ui.separator();
    }

    fn render_progress(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        paint_tactical_frame(ui, rect);

        egui::Frame::group(ui.style())
            .fill(match self.theme {
                Theme::Tactical => egui::Color32::from_rgb(0x00, 0x10, 0x08),
                Theme::Normal => egui::Color32::from_gray(240),
            })
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(&self.op.phase)
                        .color(TACTICAL_GREEN)
                        .size(13.0),
                );

                let prog = egui::ProgressBar::new(self.op.progress as f32)
                    .desired_width(ui.available_width() - 20.0);
                ui.add(prog);

                ui.horizontal(|ui| {
                    if let Some(start) = self.op.started {
                        let elapsed = start.elapsed().as_secs();
                        ui.label(
                            egui::RichText::new(format!("⏱ {}s", elapsed))
                                .color(TACTICAL_DIM)
                                .size(11.0),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(
                                egui::RichText::new("⏹ Cancel")
                                    .color(egui::Color32::RED)
                                    .size(12.0),
                            )
                            .clicked()
                        {
                            self.op.cancel.store(true, Ordering::SeqCst);
                        }
                    });
                });
            });
        ui.add_space(8.0);
    }

    fn render_results(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                egui::Frame::group(ui.style())
                    .fill(match self.theme {
                        Theme::Tactical => egui::Color32::from_rgb(0x00, 0x10, 0x08),
                        Theme::Normal => egui::Color32::from_gray(245),
                    })
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Results")
                                .color(TACTICAL_GREEN)
                                .strong()
                                .size(13.0),
                        );
                        ui.separator();
                        for r in &self.results {
                            let color = if r.contains("error") || r.contains("failed") {
                                egui::Color32::RED
                            } else {
                                TACTICAL_GREEN
                            };
                            ui.label(egui::RichText::new(r).color(color).size(11.0));
                        }
                    });
            });
    }

    // ============================================================
    // Extract Tab
    // ============================================================

    fn render_extract(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        paint_tactical_frame(ui, rect);

        egui::Grid::new("extract_grid")
            .striped(true)
            .min_col_width(100.0)
            .show(ui, |ui| {
                // Archive
                ui.label(egui::RichText::new("Archive").color(TACTICAL_GREEN));
                ui.horizontal(|ui| {
                    let count = self.extract_archives.len();
                    let label = if count == 0 {
                        "No archive selected".to_string()
                    } else if count == 1 {
                        self.extract_archives[0]
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default()
                    } else {
                        format!("{} archives selected", count)
                    };
                    ui.label(egui::RichText::new(&label).size(12.0));
                    if ui
                        .button(egui::RichText::new("📂 Browse").size(12.0))
                        .clicked()
                    {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("Archives", &["zip", "tar", "gz", "bz2", "xz", "7z", "rar"])
                            .pick_files()
                        {
                            self.extract_archives = files;
                            self.update_extract_detail();
                        }
                    }
                });
                ui.end_row();

                // Output
                ui.label(egui::RichText::new("Output").color(TACTICAL_GREEN));
                ui.horizontal(|ui| {
                    let label = self
                        .extract_output
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Same as archive".to_string());
                    ui.label(egui::RichText::new(&label).size(12.0));
                    if ui
                        .button(egui::RichText::new("📁 Choose").size(12.0))
                        .clicked()
                    {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            self.extract_output = Some(dir);
                        }
                    }
                    if self.extract_output.is_some() && ui.button("Clear").clicked() {
                        self.extract_output = None;
                    }
                });
                ui.end_row();

                // Options
                ui.label(egui::RichText::new("Overwrite").color(TACTICAL_GREEN));
                egui::ComboBox::from_id_salt("overwrite")
                    .selected_text(format!("{:?}", self.extract_overwrite))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.extract_overwrite, OverwritePolicy::Ask, "Ask");
                        ui.selectable_value(&mut self.extract_overwrite, OverwritePolicy::Skip, "Skip");
                        ui.selectable_value(
                            &mut self.extract_overwrite,
                            OverwritePolicy::Overwrite,
                            "Overwrite",
                        );
                    });
                ui.end_row();

                ui.label(egui::RichText::new("Flat").color(TACTICAL_GREEN));
                ui.checkbox(&mut self.extract_flat, "Flatten to single directory");
                ui.end_row();
            });

        ui.add_space(8.0);

        // Archive detail
        if !self.extract_detail.is_empty() {
            egui::Frame::group(ui.style())
                .fill(match self.theme {
                    Theme::Tactical => egui::Color32::from_rgb(0x00, 0x0a, 0x05),
                    Theme::Normal => egui::Color32::from_gray(245),
                })
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(&self.extract_detail)
                            .color(TACTICAL_GREEN)
                            .size(11.0),
                    );
                });
            ui.add_space(8.0);
        }

        // Extract button
        if !self.extract_archives.is_empty() && !self.op.running {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("▶ EXTRACT")
                            .color(egui::Color32::BLACK)
                            .size(16.0)
                            .strong(),
                    )
                    .fill(TACTICAL_GREEN)
                    .min_size(egui::vec2(ui.available_width(), 40.0)),
                )
                .clicked()
            {
                self.run_extract();
            }
        }
    }

    fn update_extract_detail(&mut self) {
        if let Some(path) = self.extract_archives.first() {
            match GeekZipEgui::analyze_archive(path) {
                Ok(detail) => self.extract_detail = detail,
                Err(e) => self.extract_detail = format!("Error: {}", e),
            }
        }
    }

    fn analyze_archive(path: &std::path::Path) -> Result<String> {
        let format = ArchiveFormat::detect(path).ok_or_else(|| anyhow::anyhow!("Unknown format"))?;
        let meta = std::fs::metadata(path)?;
        let size = format_bytes(meta.len());
        Ok(format!(
            "Format: {:?}  |  Size: {}  |  Modified: {}",
            format,
            size,
            chrono::DateTime::<chrono::Local>::from(meta.modified()?)
                .format("%Y-%m-%d %H:%M")
        ))
    }

    fn run_extract(&mut self) {
        let archives = self.extract_archives.clone();
        let output = self.extract_output.clone();
        let overwrite = self.extract_overwrite;
        let flat = self.extract_flat;
        let cancel = self.op.cancel.clone();

        self.op.running = true;
        self.op.started = Some(Instant::now());
        self.op.cancel.store(false, Ordering::SeqCst);

        let phase = Arc::new(Mutex::new(String::new()));
        let progress = Arc::new(Mutex::new(0.0_f32));
        let results = Arc::new(Mutex::new(Vec::new()));

        let phase_clone = phase.clone();
        let progress_clone = progress.clone();
        let results_clone = results.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let mut handles = Vec::new();
                for archive in &archives {
                    if cancel.load(Ordering::SeqCst) {
                        break;
                    }

                    let out_dir = output.clone().unwrap_or_else(|| {
                        let parent = archive.parent().unwrap_or(std::path::Path::new("."));
                        let stem = archive
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "extracted".to_string());
                        parent.join(&stem)
                    });

                    *phase.lock().unwrap() = format!(
                        "Extracting: {}",
                        archive.file_name().unwrap().to_string_lossy()
                    );

                    let cb: geekzip_core::task::ProgressCallback = Box::new(move |update: ProgressUpdate| {
                        if update.total > 0 {
                            *progress.lock().unwrap() =
                                update.current as f32 / update.total as f32;
                        }
                        if !update.message.is_empty() {
                            *phase.lock().unwrap() = update.message;
                        }
                    });

                    let options = ExtractOptions {
                        overwrite,
                        flatten_single_root: flat,
                        flatten_nested_single_root: true,
                        remove_archive: false,
                        ..Default::default()
                    };

                    match ExtractEngine::extract(archive, &out_dir, options, Some(cb)).await {
                        Ok(result) => {
                            let msg = format!(
                                "✅ {} → {} files extracted",
                                archive.file_name().unwrap().to_string_lossy(),
                                result.files_extracted
                            );
                            results_clone.lock().unwrap().push(msg);
                        }
                        Err(e) => {
                            let msg = format!(
                                "❌ {} failed: {}",
                                archive.file_name().unwrap().to_string_lossy(),
                                e
                            );
                            results_clone.lock().unwrap().push(msg);
                        }
                    }
                }
            });

            *phase.lock().unwrap() = "Done".to_string();
            *progress.lock().unwrap() = 1.0;
        });

        // Poll the thread via a custom update loop
        // We use a simple polling approach
        self.op.message = "Extraction in progress...".to_string();
        let _ = (phase, progress, results);
    }

    // ============================================================
    // Compress Tab
    // ============================================================

    fn render_compress(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        paint_tactical_frame(ui, rect);

        egui::Grid::new("compress_grid")
            .striped(true)
            .min_col_width(100.0)
            .show(ui, |ui| {
                // Sources
                ui.label(egui::RichText::new("Sources").color(TACTICAL_GREEN));
                ui.horizontal(|ui| {
                    let count = self.compress_sources.len();
                    let label = if count == 0 {
                        "No files selected".to_string()
                    } else if count == 1 {
                        self.compress_sources[0]
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default()
                    } else {
                        format!("{} files selected", count)
                    };
                    ui.label(egui::RichText::new(&label).size(12.0));
                    if ui
                        .button(egui::RichText::new("📂 Add Files").size(12.0))
                        .clicked()
                    {
                        if let Some(files) = rfd::FileDialog::new().pick_files() {
                            self.compress_sources = files;
                        }
                    }
                    if ui
                        .button(egui::RichText::new("📁 Add Folder").size(12.0))
                        .clicked()
                    {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            self.compress_sources.push(dir);
                        }
                    }
                });
                ui.end_row();

                // Format
                ui.label(egui::RichText::new("Format").color(TACTICAL_GREEN));
                egui::ComboBox::from_id_salt("compress_format")
                    .selected_text(format!("{:?}", self.compress_format))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.compress_format, CompressFormat::Zip, "Zip");
                        ui.selectable_value(&mut self.compress_format, CompressFormat::TarGz, "Tar.gz");
                        ui.selectable_value(&mut self.compress_format, CompressFormat::TarBz2, "Tar.bz2");
                        ui.selectable_value(&mut self.compress_format, CompressFormat::TarXz, "Tar.xz");
                        ui.selectable_value(&mut self.compress_format, CompressFormat::SevenZ, "7z");
                    });
                ui.end_row();

                // Output
                ui.label(egui::RichText::new("Output").color(TACTICAL_GREEN));
                ui.horizontal(|ui| {
                    let label = self
                        .compress_output
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Auto".to_string());
                    ui.label(egui::RichText::new(&label).size(12.0));
                    if ui
                        .button(egui::RichText::new("📁 Save As").size(12.0))
                        .clicked()
                    {
                        let ext = match self.compress_format {
                            CompressFormat::Zip => "zip",
                            CompressFormat::TarGz => "tar.gz",
                            CompressFormat::TarBz2 => "tar.bz2",
                            CompressFormat::TarXz => "tar.xz",
                            CompressFormat::SevenZ => "7z",
                        };
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&format!("output.{}", ext))
                            .save_file()
                        {
                            self.compress_output = Some(path);
                        }
                    }
                });
                ui.end_row();
            });

        ui.add_space(8.0);

        if !self.compress_sources.is_empty() && !self.op.running {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("▶ COMPRESS")
                            .color(egui::Color32::BLACK)
                            .size(16.0)
                            .strong(),
                    )
                    .fill(TACTICAL_GREEN)
                    .min_size(egui::vec2(ui.available_width(), 40.0)),
                )
                .clicked()
            {
                // Run compress
                let sources = self.compress_sources.clone();
                let output = self.compress_output.clone();
                let format = self.compress_format;
                let cancel = self.op.cancel.clone();

                self.op.running = true;
                self.op.started = Some(Instant::now());
                self.op.cancel.store(false, Ordering::SeqCst);
                self.op.phase = "Compressing...".to_string();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    rt.block_on(async {
                        let out_path = output.unwrap_or_else(|| {
                            let first = sources.first().unwrap();
                            let parent = first.parent().unwrap_or(std::path::Path::new("."));
                            let base = first
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| "archive".to_string());
                            let ext = match format {
                                CompressFormat::Zip => "zip",
                                CompressFormat::TarGz => "tar.gz",
                                CompressFormat::TarBz2 => "tar.bz2",
                                CompressFormat::TarXz => "tar.xz",
                                CompressFormat::SevenZ => "7z",
                            };
                            parent.join(format!("{}.{}", base, ext))
                        });

                        let options = CompressOptions {
                            format,
                            password: None,
                            compression_level: None,
                        };

                        match CompressEngine::compress(&sources, &out_path, options).await {
                            Ok(()) => {
                                tracing::info!("Compressed to: {}", out_path.display());
                            }
                            Err(e) => {
                                tracing::error!("Compress failed: {}", e);
                            }
                        }
                    });
                });
            }
        }
    }

    // ============================================================
    // Batch Tab
    // ============================================================

    fn render_batch(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        paint_tactical_frame(ui, rect);

        egui::Grid::new("batch_grid")
            .striped(true)
            .min_col_width(100.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Directory").color(TACTICAL_GREEN));
                ui.horizontal(|ui| {
                    let label = self
                        .batch_dir
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Select folder with archives".to_string());
                    ui.label(egui::RichText::new(&label).size(12.0));
                    if ui
                        .button(egui::RichText::new("📁 Browse").size(12.0))
                        .clicked()
                    {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            self.batch_dir = Some(dir);
                        }
                    }
                });
                ui.end_row();

                ui.label(egui::RichText::new("Recursive").color(TACTICAL_GREEN));
                ui.checkbox(&mut self.batch_recursive, "Search subdirectories");
                ui.end_row();
            });

        ui.add_space(8.0);

        if self.batch_dir.is_some() && !self.op.running {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("▶ BATCH EXTRACT")
                            .color(egui::Color32::BLACK)
                            .size(16.0)
                            .strong(),
                    )
                    .fill(TACTICAL_GREEN)
                    .min_size(egui::vec2(ui.available_width(), 40.0)),
                )
                .clicked()
            {
                // Run batch extraction
                let dir = self.batch_dir.clone().unwrap();
                let recursive = self.batch_recursive;

                self.op.running = true;
                self.op.started = Some(Instant::now());
                self.op.cancel.store(false, Ordering::SeqCst);
                self.op.phase = "Scanning for archives...".to_string();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    rt.block_on(async {
                        let walk = if recursive {
                            walkdir::WalkDir::new(&dir)
                        } else {
                            walkdir::WalkDir::new(&dir).max_depth(1)
                        };

                        let archives: Vec<PathBuf> = walk
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .filter(|e| e.path().is_file())
                            .filter(|e| {
                                ArchiveFormat::detect(e.path()).is_some()
                            })
                            .map(|e| e.path().to_path_buf())
                            .collect();

                        for archive in &archives {
                            let out_dir = dir.join(
                                archive
                                    .file_stem()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "extracted".to_string()),
                            );

                            let options = ExtractOptions {
                                overwrite: OverwritePolicy::Skip,
                                flatten_single_root: true,
                                flatten_nested_single_root: true,
                                remove_archive: false,
                                ..Default::default()
                            };

                            let _ = ExtractEngine::extract(archive, &out_dir, options, None).await;
                        }
                    });
                });
            }
        }
    }

    // ============================================================
    // Settings Tab
    // ============================================================

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        paint_tactical_frame(ui, rect);

        // Theme
        ui.label(
            egui::RichText::new("Appearance")
                .color(TACTICAL_GREEN)
                .size(14.0)
                .strong(),
        );
        ui.horizontal(|ui| {
            ui.label("Theme:");
            egui::ComboBox::from_id_salt("theme")
                .selected_text(match self.theme {
                    Theme::Tactical => "Tactical Terminal",
                    Theme::Normal => "Normal",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.theme, Theme::Tactical, "Tactical Terminal");
                    ui.selectable_value(&mut self.theme, Theme::Normal, "Normal");
                });
        });

        ui.add_space(12.0);

        // Passwords
        ui.label(
            egui::RichText::new("Passwords")
                .color(TACTICAL_GREEN)
                .size(14.0)
                .strong(),
        );

        ui.horizontal(|ui| {
            ui.label("Add password:");
            ui.text_edit_singleline(&mut self.new_password);
            if ui
                .button(egui::RichText::new("+ Add").size(12.0))
                .clicked()
                && !self.new_password.is_empty()
            {
                self.passwords.push(self.new_password.clone());
                self.new_password.clear();
            }
        });

        if !self.passwords.is_empty() {
            let mut remove_idx = None;
            for (i, pw) in self.passwords.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("🔑")
                            .color(TACTICAL_GREEN)
                            .size(12.0),
                    );
                    ui.label(egui::RichText::new(pw).size(12.0));
                    if ui
                        .button(egui::RichText::new("✕").color(egui::Color32::RED).size(12.0))
                        .clicked()
                    {
                        remove_idx = Some(i);
                    }
                });
            }
            if let Some(idx) = remove_idx {
                self.passwords.remove(idx);
            }
        }

        ui.add_space(12.0);

        // About
        ui.label(
            egui::RichText::new("About")
                .color(TACTICAL_GREEN)
                .size(14.0)
                .strong(),
        );
        ui.label(
            egui::RichText::new("GeekZip v0.4.0 — Cross-platform archive tool")
                .size(11.0),
        );
        ui.label(
            egui::RichText::new("Built with egui + geekzip-core")
                .color(TACTICAL_DIM)
                .size(10.0),
        );
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(820.0, 640.0))
            .with_min_inner_size(egui::vec2(600.0, 480.0))
            .with_title("GeekZip"),
        ..Default::default()
    };

    eframe::run_native(
        "GeekZip",
        native_options,
        Box::new(|_cc| Ok(Box::<GeekZipEgui>::default())),
    )?;

    Ok(())
}
