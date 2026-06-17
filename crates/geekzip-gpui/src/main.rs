use geekzip_core::{
    ArchiveFormat, CompressEngine, CompressFormat, CompressOptions, ContextMenuScope,
    ExtractEngine, ExtractOptions, OperationControl, ProgressUpdate, RecursiveExtractor,
    WindowsContextMenuOptions, default_cli_path, format::detect_format,
};
use gpui::prelude::FluentBuilder;
use gpui::{
    App, AppContext, Application, Bounds, Context, Div, Entity, Hsla, InteractiveElement,
    IntoElement, ParentElement, PathPromptOptions, Render, StatefulInteractiveElement, Styled,
    Task, Window, WindowBounds, WindowOptions, canvas, div, point, px, size,
};
use gpui_component::{
    Root, Theme, ThemeMode,
    input::{Input, InputState},
    scroll::ScrollableElement,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use sysinfo::{Pid, ProcessesToUpdate, System};

const BG: Hsla = Hsla {
    h: 0.43,
    s: 0.33,
    l: 0.022,
    a: 1.0,
};
const PANEL: Hsla = Hsla {
    h: 0.43,
    s: 0.34,
    l: 0.047,
    a: 1.0,
};
const BORDER: Hsla = Hsla {
    h: 0.43,
    s: 0.44,
    l: 0.155,
    a: 1.0,
};
const GREEN: Hsla = Hsla {
    h: 0.431,
    s: 1.0,
    l: 0.50,
    a: 1.0,
};
const MUTED: Hsla = Hsla {
    h: 0.43,
    s: 0.14,
    l: 0.49,
    a: 1.0,
};
const TEXT: Hsla = Hsla {
    h: 0.43,
    s: 0.31,
    l: 0.89,
    a: 1.0,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    Extract,
    Compress,
    AutoExtract,
    Recursive,
    Batch,
    Passwords,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppMode {
    Normal,
    Pro,
    Terminal,
}

impl Page {
    fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "工作台",
            Self::Extract => "解压缩",
            Self::Compress => "压缩文件",
            Self::AutoExtract => "自动解压",
            Self::Recursive => "递归解压",
            Self::Batch => "批量解压",
            Self::Passwords => "密码本",
            Self::Settings => "设置",
        }
    }

    fn detail(self) -> &'static str {
        match self {
            Self::Dashboard => "任务与快速操作",
            Self::Extract => "单个压缩包",
            Self::Compress => "文件与文件夹",
            Self::AutoExtract => "监控下载目录",
            Self::Recursive => "多层压缩包",
            Self::Batch => "处理整个文件夹",
            Self::Passwords => "自动尝试密码",
            Self::Settings => "系统集成",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Self::Dashboard => "◇",
            Self::Extract => "⇩",
            Self::Compress => "⇧",
            Self::AutoExtract => "◫",
            Self::Recursive => "↻",
            Self::Batch => "▦",
            Self::Passwords => "⌘",
            Self::Settings => "⚙",
        }
    }
}

#[derive(Clone, Default)]
struct ResourceStats {
    system_cpu: u8,
    process_cpu: u8,
    gpu: Option<u8>,
    memory_used_percent: u8,
    memory_used_mb: u64,
    process_memory_mb: u64,
    threads: usize,
}

#[derive(Clone)]
struct ResourceHistory {
    system_cpu: Vec<u8>,
    process_cpu: Vec<u8>,
    gpu: Vec<u8>,
    memory: Vec<u8>,
    process_memory: Vec<u8>,
    threads: Vec<u8>,
}

#[derive(Clone, Default)]
struct ArchiveDetail {
    rows: Vec<(String, String)>,
    entries: Vec<ArchiveEntryDetail>,
    status: String,
}

#[derive(Clone)]
struct ArchiveEntryDetail {
    name: String,
    kind: &'static str,
    size: u64,
    compressed_size: u64,
    encrypted: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ManagedResultKind {
    Extract,
    Compress,
}

impl ManagedResultKind {
    fn label(self) -> &'static str {
        match self {
            Self::Extract => "解压结果",
            Self::Compress => "压缩结果",
        }
    }
}

#[derive(Clone)]
struct ManagedResult {
    kind: ManagedResultKind,
    path: PathBuf,
}

impl Default for ResourceHistory {
    fn default() -> Self {
        Self {
            system_cpu: vec![0; 36],
            process_cpu: vec![0; 36],
            gpu: vec![0; 36],
            memory: vec![0; 36],
            process_memory: vec![0; 36],
            threads: vec![0; 36],
        }
    }
}

impl ResourceHistory {
    const MAX_POINTS: usize = 36;

    fn push_value(values: &mut Vec<u8>, value: u8) {
        values.push(value.min(100));
        if values.len() > Self::MAX_POINTS {
            values.drain(0..values.len() - Self::MAX_POINTS);
        }
    }

    fn push(&mut self, stats: &ResourceStats) {
        Self::push_value(&mut self.system_cpu, stats.system_cpu);
        Self::push_value(&mut self.process_cpu, stats.process_cpu);
        Self::push_value(&mut self.gpu, stats.gpu.unwrap_or_default());
        Self::push_value(&mut self.memory, stats.memory_used_percent);
        Self::push_value(
            &mut self.process_memory,
            ((stats.process_memory_mb as f32 / 2048.0) * 100.0).round() as u8,
        );
        Self::push_value(
            &mut self.threads,
            ((stats.threads as f32 / 32.0) * 100.0).round() as u8,
        );
    }
}

struct GeekZipApp {
    mode: AppMode,
    page: Page,
    archive_path: Option<PathBuf>,
    extract_archives: Vec<PathBuf>,
    archive_detail_path: Option<PathBuf>,
    archive_detail: ArchiveDetail,
    target_dir: Option<PathBuf>,
    default_extract_dir: Option<PathBuf>,
    extract_prefix_input: Entity<InputState>,
    flatten_single_root: bool,
    windows_context_menu: WindowsContextMenuOptions,
    windows_context_menu_machine: bool,
    compress_sources: Vec<PathBuf>,
    compress_format: usize,
    compress_level: u32,
    compress_volume_size_mb: Option<u64>,
    compress_suffix_input: Entity<InputState>,
    batch_dir: Option<PathBuf>,
    watch_dir: Option<PathBuf>,
    password_input: Entity<InputState>,
    passwords: Vec<String>,
    managed_results: Vec<ManagedResult>,
    selected_result: Option<usize>,
    result_rename_input: Entity<InputState>,
    busy: bool,
    operation: String,
    result: String,
    operation_progress: f32,
    operation_bytes_done: u64,
    operation_total_bytes: u64,
    operation_files_done: usize,
    operation_total_files: usize,
    operation_speed_bps: u64,
    operation_eta_seconds: u64,
    operation_current_path: Option<String>,
    operation_started_at: Option<Instant>,
    operation_control: Option<OperationControl>,
    operation_paused: bool,
    operation_log: Vec<String>,
    watch_task: Option<Task<()>>,
    resource_stats: ResourceStats,
    resource_history: ResourceHistory,
    resource_system: Arc<Mutex<System>>,
    resource_task: Option<Task<()>>,
    led_pulse: bool,
    capsule_mode: bool,
    context_action: Option<String>,
}

#[derive(Clone, Default)]
struct ContextLaunch {
    extract_path: Option<PathBuf>,
    action: Option<String>,
    output: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
struct AppSettings {
    #[serde(default)]
    default_extract_dir: Option<String>,
    #[serde(default)]
    extract_prefixes: String,
    #[serde(default = "default_flatten_single_root")]
    flatten_single_root: bool,
    #[serde(default)]
    windows_context_menu: WindowsContextMenuOptions,
    #[serde(default)]
    windows_context_menu_machine: bool,
}

fn default_flatten_single_root() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_extract_dir: None,
            extract_prefixes: String::new(),
            flatten_single_root: true,
            windows_context_menu: WindowsContextMenuOptions::default(),
            windows_context_menu_machine: false,
        }
    }
}

impl GeekZipApp {
    fn new(window: &mut Window, cx: &mut Context<Self>, launch: ContextLaunch) -> Self {
        let settings = Self::load_settings();
        let extract_prefixes = settings.extract_prefixes.clone();
        Self {
            mode: AppMode::Pro,
            page: if launch.extract_path.is_some() {
                Page::Extract
            } else {
                Page::Dashboard
            },
            archive_path: launch.extract_path.clone(),
            extract_archives: launch.extract_path.iter().cloned().collect(),
            archive_detail_path: launch.extract_path.clone(),
            archive_detail: launch
                .extract_path
                .as_ref()
                .map(|path| Self::analyze_archive_detail(path))
                .unwrap_or_default(),
            target_dir: launch.output.clone(),
            default_extract_dir: settings.default_extract_dir.map(PathBuf::from),
            extract_prefix_input: cx.new(|cx| {
                let mut input =
                    InputState::new(window, cx).placeholder("例如：广告前缀_, [站点名]");
                input.set_value(extract_prefixes, window, cx);
                input
            }),
            flatten_single_root: settings.flatten_single_root,
            windows_context_menu: settings.windows_context_menu,
            windows_context_menu_machine: settings.windows_context_menu_machine,
            compress_sources: Vec::new(),
            compress_format: 0,
            compress_level: 6,
            compress_volume_size_mb: None,
            compress_suffix_input: cx
                .new(|cx| InputState::new(window, cx).placeholder("例如：中文混淆")),
            batch_dir: None,
            watch_dir: None,
            password_input: cx
                .new(|cx| InputState::new(window, cx).placeholder("输入一个解压密码")),
            passwords: Self::load_passwords(),
            managed_results: Vec::new(),
            selected_result: None,
            result_rename_input: cx.new(|cx| InputState::new(window, cx).placeholder("输入新名称")),
            busy: false,
            operation: "等待任务".into(),
            result: "选择文件后即可开始".into(),
            operation_progress: 0.0,
            operation_bytes_done: 0,
            operation_total_bytes: 0,
            operation_files_done: 0,
            operation_total_files: 0,
            operation_speed_bps: 0,
            operation_eta_seconds: 0,
            operation_current_path: None,
            operation_started_at: None,
            operation_control: None,
            operation_paused: false,
            operation_log: vec!["[READY] Rust 核心已连接".into()],
            watch_task: None,
            resource_stats: ResourceStats::default(),
            resource_history: ResourceHistory::default(),
            resource_system: Arc::new(Mutex::new(System::new_all())),
            resource_task: None,
            led_pulse: false,
            capsule_mode: launch.extract_path.is_some(),
            context_action: launch.action,
        }
    }

    fn gpu_usage() -> Option<u8> {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("ioreg")
                .args(["-r", "-d", "1", "-w", "0", "-c", "IOAccelerator"])
                .output()
                .ok()?;
            let text = String::from_utf8_lossy(&output.stdout);
            let marker = "\"Device Utilization %\"=";
            let tail = text.split_once(marker)?.1;
            return tail
                .split(|character: char| !character.is_ascii_digit())
                .find(|value| !value.is_empty())
                .and_then(|value| value.parse::<u8>().ok());
        }

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    r#"$sample = Get-Counter '\GPU Engine(*)\Utilization Percentage' -ErrorAction SilentlyContinue; if ($sample) { $sum = ($sample.CounterSamples | Measure-Object -Property CookedValue -Sum).Sum; [Math]::Round([Math]::Min($sum, 100)) }"#,
                ])
                .output()
                .ok()?;
            let text = String::from_utf8_lossy(&output.stdout);
            return text
                .split(|character: char| !character.is_ascii_digit())
                .find(|value| !value.is_empty())
                .and_then(|value| value.parse::<u8>().ok());
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        None
    }

    fn sample_resources(system: &Arc<Mutex<System>>) -> ResourceStats {
        let mut system = match system.lock() {
            Ok(system) => system,
            Err(_) => return ResourceStats::default(),
        };
        system.refresh_cpu_usage();
        system.refresh_memory();
        let pid = Pid::from_u32(std::process::id());
        system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
        let process = system.process(pid);
        let total_memory = system.total_memory().max(1);
        ResourceStats {
            system_cpu: system.global_cpu_usage().round().clamp(0.0, 100.0) as u8,
            process_cpu: process
                .map(|process| process.cpu_usage().round().clamp(0.0, 100.0) as u8)
                .unwrap_or_default(),
            gpu: Self::gpu_usage(),
            memory_used_percent: ((system.used_memory() as f32 / total_memory as f32) * 100.0)
                .round()
                .clamp(0.0, 100.0) as u8,
            memory_used_mb: system.used_memory() / 1024 / 1024,
            process_memory_mb: process
                .map(|process| process.memory() / 1024 / 1024)
                .unwrap_or_default(),
            threads: std::thread::available_parallelism()
                .map(|threads| threads.get())
                .unwrap_or(1),
        }
    }

    fn start_resource_monitor(&mut self, cx: &mut Context<Self>) {
        let system = self.resource_system.clone();
        self.resource_task = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let sampler = system.clone();
                let stats = cx
                    .background_executor()
                    .spawn(async move { Self::sample_resources(&sampler) })
                    .await;
                _ = cx.update(|cx| {
                    _ = this.update(cx, |this, cx| {
                        this.resource_stats = stats;
                        this.resource_history.push(&this.resource_stats);
                        this.led_pulse = !this.led_pulse;
                        cx.notify();
                    });
                });
            }
        }));
    }

    fn password_file() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".geekzip/passwords.json"))
    }

    fn settings_file() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".geekzip/settings.json"))
    }

    fn load_settings() -> AppSettings {
        Self::settings_file()
            .and_then(|path| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save_settings(&self, cx: &Context<Self>) {
        let Some(path) = Self::settings_file() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let settings = AppSettings {
            default_extract_dir: self
                .default_extract_dir
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            extract_prefixes: self.extract_prefix_input.read(cx).value().to_string(),
            flatten_single_root: self.flatten_single_root,
            windows_context_menu: self.windows_context_menu.clone(),
            windows_context_menu_machine: self.windows_context_menu_machine,
        };
        if let Ok(bytes) = serde_json::to_vec_pretty(&settings) {
            let _ = fs::write(path, bytes);
        }
    }

    fn context_menu_scope(&self) -> ContextMenuScope {
        if self.windows_context_menu_machine {
            ContextMenuScope::Machine
        } else {
            ContextMenuScope::User
        }
    }

    fn install_windows_context_menu_from_settings(&self) -> anyhow::Result<()> {
        let cli_path = default_cli_path();
        let app_path = std::env::current_exe().ok();
        if self.context_menu_scope() == ContextMenuScope::Machine {
            return Self::run_elevated_context_menu_command(
                "install-context-menu",
                &self.windows_context_menu,
                app_path.as_deref(),
            );
        }
        geekzip_core::install_windows_context_menu(
            &cli_path,
            app_path.as_deref(),
            &self.windows_context_menu,
            ContextMenuScope::User,
        )
    }

    fn uninstall_windows_context_menu_from_settings(&self) -> anyhow::Result<()> {
        if self.context_menu_scope() == ContextMenuScope::Machine {
            return Self::run_elevated_context_menu_command(
                "uninstall-context-menu",
                &self.windows_context_menu,
                None,
            );
        }
        geekzip_core::uninstall_windows_context_menu(ContextMenuScope::User)
    }

    fn run_elevated_context_menu_command(
        command: &str,
        options: &WindowsContextMenuOptions,
        app_path: Option<&std::path::Path>,
    ) -> anyhow::Result<()> {
        #[cfg(target_os = "windows")]
        {
            let cli_path = default_cli_path();
            let mut args = vec![command.to_string(), "--scope".into(), "machine".into()];
            if let Some(app_path) = app_path {
                args.push("--app-path".into());
                args.push(app_path.to_string_lossy().to_string());
            }
            if command == "install-context-menu" {
                if !options.smart_extract {
                    args.push("--no-smart-extract".into());
                }
                if !options.extract_here {
                    args.push("--no-extract-here".into());
                }
                if !options.extract_to_folder {
                    args.push("--no-extract-to-folder".into());
                }
                if !options.extract_delete {
                    args.push("--no-extract-delete".into());
                }
                if !options.open_app {
                    args.push("--no-open-app".into());
                }
            }
            let ps_args = args
                .iter()
                .map(|arg| format!("'{}'", arg.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(",");
            let script = format!(
                "Start-Process -FilePath '{}' -ArgumentList @({}) -Verb RunAs -Wait",
                cli_path.to_string_lossy().replace('\'', "''"),
                ps_args
            );
            let status = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .status()?;
            if status.success() {
                Ok(())
            } else {
                anyhow::bail!("administrator install command failed")
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (command, options, app_path);
            anyhow::bail!("Windows context menu is only available on Windows")
        }
    }

    fn load_passwords() -> Vec<String> {
        Self::password_file()
            .and_then(|path| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn select_archive_path(&mut self, path: PathBuf) {
        self.archive_detail = Self::analyze_archive_detail(&path);
        self.archive_detail_path = Some(path.clone());
        self.archive_path = Some(path);
    }

    fn add_extract_archives(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if !self.extract_archives.contains(&path) {
                self.extract_archives.push(path.clone());
            }
            self.select_archive_path(path);
        }
    }

    fn record_managed_result(&mut self, kind: ManagedResultKind, path: impl Into<PathBuf>) {
        let path = path.into();
        self.managed_results
            .retain(|item| item.path != path || item.kind != kind);
        self.managed_results.insert(0, ManagedResult { kind, path });
        self.managed_results.truncate(24);
        self.selected_result = Some(0);
    }

    fn rename_selected_result(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.selected_result else {
            self.result = "请先选择一个结果".into();
            return;
        };
        let Some(item) = self.managed_results.get(index).cloned() else {
            self.result = "结果不存在".into();
            return;
        };
        let new_name = self.result_rename_input.read(cx).value().trim().to_string();
        if new_name.is_empty() {
            self.result = "请输入新名称".into();
            return;
        }
        if new_name.contains(std::path::MAIN_SEPARATOR) {
            self.result = "名称不能包含路径分隔符".into();
            return;
        }
        let Some(parent) = item.path.parent() else {
            self.result = "无法确定父目录".into();
            return;
        };
        let destination = parent.join(new_name);
        if destination.exists() {
            self.result = "目标名称已存在".into();
            return;
        }
        match fs::rename(&item.path, &destination) {
            Ok(_) => {
                if let Some(slot) = self.managed_results.get_mut(index) {
                    slot.path = destination.clone();
                }
                self.result = format!("已重命名为 {}", destination.display());
                self.operation_log.push(format!(
                    "[RENAME] {} -> {}",
                    item.path.display(),
                    destination.display()
                ));
                self.result_rename_input
                    .update(cx, |input, cx| input.set_value("", window, cx));
            }
            Err(error) => {
                self.result = format!("重命名失败：{error}");
                self.operation_log
                    .push(format!("[ERROR] rename: {error:#}"));
            }
        }
    }

    fn open_selected_result(&mut self) {
        let Some(index) = self.selected_result else {
            self.result = "请先选择一个结果".into();
            return;
        };
        let Some(item) = self.managed_results.get(index) else {
            self.result = "结果不存在".into();
            return;
        };
        if !item.path.exists() {
            self.result = "路径已不存在".into();
            return;
        }
        let status = {
            #[cfg(target_os = "macos")]
            {
                Command::new("open").arg(&item.path).status()
            }
            #[cfg(target_os = "windows")]
            {
                Command::new("explorer").arg(&item.path).status()
            }
            #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
            {
                Command::new("xdg-open").arg(&item.path).status()
            }
        };
        match status {
            Ok(status) if status.success() => {
                self.result = format!("已打开 {}", item.path.display());
            }
            Ok(_) => {
                self.result = "打开失败：系统命令返回错误".into();
            }
            Err(error) => {
                self.result = format!("打开失败：{error}");
            }
        }
    }

    fn format_bytes(bytes: u64) -> String {
        const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
        let mut value = bytes as f64;
        let mut unit = 0usize;
        while value >= 1024.0 && unit < UNITS.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }
        if unit == 0 {
            format!("{bytes} {}", UNITS[unit])
        } else {
            format!("{value:.1} {}", UNITS[unit])
        }
    }

    fn format_system_time(time: SystemTime) -> String {
        let Ok(duration) = time.duration_since(UNIX_EPOCH) else {
            return "早于 UNIX_EPOCH".into();
        };
        let total_seconds = duration.as_secs() as i64;
        let days = total_seconds.div_euclid(86_400);
        let seconds_of_day = total_seconds.rem_euclid(86_400);
        let (year, month, day) = Self::civil_from_days(days);
        let hour = seconds_of_day / 3_600;
        let minute = seconds_of_day % 3_600 / 60;
        let second = seconds_of_day % 60;
        format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} UTC")
    }

    fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
        let z = days_since_epoch + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let mut year = (yoe + era * 400) as i32;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = (mp + if mp < 10 { 3 } else { -9 }) as u32;
        year += if month <= 2 { 1 } else { 0 };
        (year, month, day)
    }

    fn detection_label(method: &geekzip_core::format::DetectionMethod) -> &'static str {
        match method {
            geekzip_core::format::DetectionMethod::MagicBytes => "Magic Bytes",
            geekzip_core::format::DetectionMethod::Extension => "扩展名",
            geekzip_core::format::DetectionMethod::Both => "Magic Bytes + 扩展名",
        }
    }

    fn ratio_label(archive_size: u64, unpacked_size: u64) -> String {
        if unpacked_size == 0 {
            return "--".into();
        }
        let ratio = archive_size as f64 / unpacked_size as f64 * 100.0;
        let saved = (100.0 - ratio).max(0.0);
        format!("{ratio:.1}%（节省 {saved:.1}%）")
    }

    fn analyze_archive_detail(path: &Path) -> ArchiveDetail {
        let mut rows = Vec::new();
        let metadata = fs::metadata(path).ok();
        let format_info = detect_format(path);
        let archive_size = metadata
            .as_ref()
            .map(|value| value.len())
            .unwrap_or_default();

        rows.push(("路径".into(), path.to_string_lossy().to_string()));
        rows.push(("格式".into(), format_info.format.name().to_string()));
        rows.push((
            "识别方式".into(),
            Self::detection_label(&format_info.detected_by).into(),
        ));
        rows.push((
            "扩展名".into(),
            format_info
                .original_extension
                .clone()
                .unwrap_or_else(|| "无".into()),
        ));
        rows.push(("压缩包大小".into(), Self::format_bytes(archive_size)));
        if let Some(metadata) = metadata.as_ref() {
            if let Ok(modified) = metadata.modified() {
                rows.push(("修改时间".into(), Self::format_system_time(modified)));
            }
            if let Ok(created) = metadata.created() {
                rows.push(("创建时间".into(), Self::format_system_time(created)));
            }
        }

        let archive_result = match format_info.format {
            ArchiveFormat::Zip => Self::zip_archive_detail(path),
            ArchiveFormat::Tar => Self::tar_archive_detail(path, None),
            ArchiveFormat::TarGz => Self::tar_archive_detail(path, Some("gz")),
            ArchiveFormat::TarBz2 => Self::tar_archive_detail(path, Some("bz2")),
            ArchiveFormat::TarXz => Self::tar_archive_detail(path, Some("xz")),
            _ => Ok((Vec::new(), Vec::new(), "已读取基础文件信息".to_string())),
        };

        match archive_result {
            Ok((mut archive_rows, entries, status)) => {
                rows.append(&mut archive_rows);
                ArchiveDetail {
                    rows,
                    entries,
                    status,
                }
            }
            Err(error) => {
                rows.push(("读取状态".into(), error.to_string()));
                ArchiveDetail {
                    rows,
                    entries: Vec::new(),
                    status: "基础信息可用，内部目录读取失败".into(),
                }
            }
        }
    }

    fn zip_archive_detail(
        path: &Path,
    ) -> anyhow::Result<(Vec<(String, String)>, Vec<ArchiveEntryDetail>, String)> {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut total_unpacked = 0u64;
        let mut total_compressed = 0u64;
        let mut file_count = 0usize;
        let mut dir_count = 0usize;
        let mut encrypted_count = 0usize;
        let mut methods = HashSet::new();
        let mut entries = Vec::new();

        for index in 0..archive.len() {
            let entry = archive.by_index_raw(index)?;
            total_unpacked = total_unpacked.saturating_add(entry.size());
            total_compressed = total_compressed.saturating_add(entry.compressed_size());
            if entry.is_dir() {
                dir_count += 1;
            } else {
                file_count += 1;
            }
            if entry.encrypted() {
                encrypted_count += 1;
            }
            methods.insert(format!("{:?}", entry.compression()));
            entries.push(ArchiveEntryDetail {
                name: entry.name().to_string(),
                kind: if entry.is_dir() { "DIR" } else { "FILE" },
                size: entry.size(),
                compressed_size: entry.compressed_size(),
                encrypted: entry.encrypted(),
            });
        }

        let mut rows = vec![
            ("条目总数".into(), archive.len().to_string()),
            (
                "文件 / 文件夹".into(),
                format!("{file_count} / {dir_count}"),
            ),
            ("原始大小".into(), Self::format_bytes(total_unpacked)),
            ("压缩后大小".into(), Self::format_bytes(total_compressed)),
            (
                "压缩率".into(),
                Self::ratio_label(total_compressed, total_unpacked),
            ),
            ("加密条目".into(), encrypted_count.to_string()),
        ];
        let mut methods = methods.into_iter().collect::<Vec<_>>();
        methods.sort();
        rows.push(("压缩方法".into(), methods.join(", ")));
        Ok((rows, entries, "已读取 ZIP 中央目录详情".into()))
    }

    fn tar_archive_detail(
        path: &Path,
        compression: Option<&str>,
    ) -> anyhow::Result<(Vec<(String, String)>, Vec<ArchiveEntryDetail>, String)> {
        let file = fs::File::open(path)?;
        let reader: Box<dyn Read> = match compression {
            Some("gz") => Box::new(flate2::read::GzDecoder::new(file)),
            Some("bz2") => Box::new(bzip2::read::BzDecoder::new(file)),
            Some("xz") => Box::new(xz2::read::XzDecoder::new(file)),
            _ => Box::new(file),
        };
        let mut archive = tar::Archive::new(reader);
        let mut total_unpacked = 0u64;
        let mut file_count = 0usize;
        let mut dir_count = 0usize;

        for entry in archive.entries()? {
            let entry = entry?;
            let header = entry.header();
            total_unpacked = total_unpacked.saturating_add(header.size().unwrap_or_default());
            let entry_type = header.entry_type();
            if entry_type.is_dir() {
                dir_count += 1;
            } else if entry_type.is_file() {
                file_count += 1;
            }
        }

        let archive_size = fs::metadata(path)
            .map(|value| value.len())
            .unwrap_or_default();
        let rows = vec![
            ("条目总数".into(), (file_count + dir_count).to_string()),
            (
                "文件 / 文件夹".into(),
                format!("{file_count} / {dir_count}"),
            ),
            ("原始大小".into(), Self::format_bytes(total_unpacked)),
            ("压缩后大小".into(), Self::format_bytes(archive_size)),
            (
                "压缩率".into(),
                Self::ratio_label(archive_size, total_unpacked),
            ),
        ];
        Ok((rows, Vec::new(), "已读取 TAR 目录详情".into()))
    }

    fn format_duration(seconds: u64) -> String {
        if seconds == 0 {
            return "--".into();
        }
        let minutes = seconds / 60;
        let seconds = seconds % 60;
        if minutes == 0 {
            format!("{seconds}s")
        } else {
            format!("{minutes}m {seconds}s")
        }
    }

    fn reset_operation_progress(&mut self, control: OperationControl) {
        self.operation_progress = 0.0;
        self.operation_bytes_done = 0;
        self.operation_total_bytes = 0;
        self.operation_files_done = 0;
        self.operation_total_files = 0;
        self.operation_speed_bps = 0;
        self.operation_eta_seconds = 0;
        self.operation_current_path = None;
        self.operation_started_at = Some(Instant::now());
        self.operation_control = Some(control);
        self.operation_paused = false;
    }

    fn clear_operation_control(&mut self) {
        self.operation_control = None;
        self.operation_paused = false;
        self.operation_started_at = None;
    }

    fn apply_progress_update(&mut self, update: ProgressUpdate) {
        self.operation = update.phase.clone();
        self.operation_progress = update.percent() as f32;
        self.operation_bytes_done = update.bytes_done;
        self.operation_total_bytes = update.total_bytes;
        self.operation_files_done = update.files_done;
        self.operation_total_files = update.total_files;
        self.operation_current_path = update.current_path;
        if let Some(started) = self.operation_started_at {
            let elapsed = started.elapsed().as_secs_f64().max(0.1);
            self.operation_speed_bps = (self.operation_bytes_done as f64 / elapsed) as u64;
            if self.operation_speed_bps > 0
                && self.operation_total_bytes > self.operation_bytes_done
            {
                self.operation_eta_seconds = (self.operation_total_bytes
                    - self.operation_bytes_done)
                    / self.operation_speed_bps;
            }
        }
    }

    fn start_progress_pump(
        &self,
        shared_progress: Arc<Mutex<Option<ProgressUpdate>>>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                let update = shared_progress
                    .lock()
                    .ok()
                    .and_then(|mut value| value.take());
                let mut keep_running = false;
                _ = cx.update(|cx| {
                    _ = this.update(cx, |this, cx| {
                        if let Some(update) = update {
                            this.apply_progress_update(update);
                        }
                        keep_running = this.busy;
                        cx.notify();
                    });
                });
                if !keep_running {
                    break;
                }
            }
        })
        .detach();
    }

    fn save_passwords(&self) {
        let Some(path) = Self::password_file() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.passwords) {
            let _ = fs::write(path, bytes);
        }
    }

    fn panel() -> Div {
        div()
            .bg(PANEL)
            .border_1()
            .border_color(BORDER)
            .rounded(px(2.))
            .relative()
            .overflow_hidden()
            .child(Self::scanlines())
    }

    fn label(text: impl Into<gpui::SharedString>) -> Div {
        div()
            .font_family("JetBrains Mono")
            .text_xs()
            .text_color(GREEN)
            .child(text.into())
    }

    fn scanlines() -> impl IntoElement {
        canvas(
            |bounds, _, _| bounds,
            |_, bounds, window, _| {
                let rows = (bounds.size.height / px(4.)).ceil() as i32;
                for row in 0..rows {
                    window.paint_quad(gpui::fill(
                        Bounds {
                            origin: bounds.origin + point(px(0.), px(row as f32 * 4.)),
                            size: size(bounds.size.width, px(1.)),
                        },
                        GREEN.opacity(0.018),
                    ));
                }

                let length = px(18.);
                let weight = px(1.5);
                let color = GREEN.opacity(0.72);
                let corners = [
                    bounds.origin,
                    point(bounds.right() - length, bounds.origin.y),
                    point(bounds.origin.x, bounds.bottom() - length),
                    point(bounds.right() - length, bounds.bottom() - length),
                ];
                for (index, origin) in corners.into_iter().enumerate() {
                    let horizontal_y = if index > 1 {
                        origin.y + length - weight
                    } else {
                        origin.y
                    };
                    let vertical_x = if index % 2 == 1 {
                        origin.x + length - weight
                    } else {
                        origin.x
                    };
                    window.paint_quad(gpui::fill(
                        Bounds {
                            origin: point(origin.x, horizontal_y),
                            size: size(length, weight),
                        },
                        color,
                    ));
                    window.paint_quad(gpui::fill(
                        Bounds {
                            origin: point(vertical_x, origin.y),
                            size: size(weight, length),
                        },
                        color,
                    ));
                }
            },
        )
        .absolute()
        .size_full()
    }

    fn dot_grid() -> impl IntoElement {
        canvas(
            |bounds, _, _| bounds,
            |_, bounds, window, _| {
                let spacing = 14.0;
                let rows = (bounds.size.height / px(spacing)).ceil() as i32;
                let cols = (bounds.size.width / px(spacing)).ceil() as i32;
                for row in 0..rows {
                    for col in 0..cols {
                        let origin = bounds.origin
                            + point(px(col as f32 * spacing), px(row as f32 * spacing));
                        window.paint_quad(gpui::fill(
                            Bounds {
                                origin,
                                size: size(px(1.5), px(1.5)),
                            },
                            GREEN.opacity(0.17),
                        ));
                    }
                }
            },
        )
        .absolute()
        .size_full()
    }

    fn dot_matrix_title() -> impl IntoElement {
        const GLYPHS: &[(&str, [&str; 7])] = &[
            (
                "D",
                ["1110", "1001", "1001", "1001", "1001", "1001", "1110"],
            ),
            (
                "R",
                ["1110", "1001", "1001", "1110", "1010", "1001", "1001"],
            ),
            (
                "O",
                ["0110", "1001", "1001", "1001", "1001", "1001", "0110"],
            ),
            (
                "P",
                ["1110", "1001", "1001", "1110", "1000", "1000", "1000"],
            ),
            (
                "A",
                ["0110", "1001", "1001", "1111", "1001", "1001", "1001"],
            ),
            (
                "C",
                ["0111", "1000", "1000", "1000", "1000", "1000", "0111"],
            ),
            (
                "H",
                ["1001", "1001", "1001", "1111", "1001", "1001", "1001"],
            ),
            ("I", ["111", "010", "010", "010", "010", "010", "111"]),
            (
                "V",
                ["1001", "1001", "1001", "1001", "1001", "0110", "0110"],
            ),
            (
                "E",
                ["1111", "1000", "1000", "1110", "1000", "1000", "1111"],
            ),
            (
                "S",
                ["0111", "1000", "1000", "0110", "0001", "0001", "1110"],
            ),
        ];
        let text = "DROP ARCHIVES HERE";
        canvas(
            move |bounds, _, _| bounds,
            move |_, bounds, window, _| {
                let dot = 3.0;
                let gap = 2.0;
                let glyph_gap = 5.0;
                let mut width = 0.0;
                for character in text.chars() {
                    width += if character == ' ' {
                        9.0
                    } else {
                        let columns = GLYPHS
                            .iter()
                            .find(|(key, _)| key.starts_with(character))
                            .map(|(_, rows)| rows[0].len())
                            .unwrap_or(4) as f32;
                        columns * (dot + gap) + glyph_gap
                    };
                }
                let mut x = bounds.origin.x + (bounds.size.width - px(width)) / 2.0;
                let y = bounds.origin.y + px(4.0);
                for character in text.chars() {
                    if character == ' ' {
                        x += px(9.0);
                        continue;
                    }
                    let Some((_, rows)) = GLYPHS.iter().find(|(key, _)| key.starts_with(character))
                    else {
                        continue;
                    };
                    for (row, pattern) in rows.iter().enumerate() {
                        for (column, pixel) in pattern.chars().enumerate() {
                            if pixel == '1' {
                                window.paint_quad(gpui::fill(
                                    Bounds {
                                        origin: point(
                                            x + px(column as f32 * (dot + gap)),
                                            y + px(row as f32 * (dot + gap)),
                                        ),
                                        size: size(px(dot), px(dot)),
                                    },
                                    if (column + row) % 5 == 0 { TEXT } else { GREEN },
                                ));
                            }
                        }
                    }
                    x += px(rows[0].len() as f32 * (dot + gap) + glyph_gap);
                }
            },
        )
        .w(px(520.))
        .h(px(46.))
    }

    fn segmented_progress(value: f32) -> Div {
        let segments = 42usize;
        let active = ((value.clamp(0.0, 100.0) / 100.0) * segments as f32).round() as usize;
        div()
            .h(px(9.))
            .min_w_0()
            .flex_1()
            .flex()
            .gap(px(3.))
            .children((0..segments).map(move |index| {
                div()
                    .h_full()
                    .flex_1()
                    .rounded(px(1.))
                    .bg(if index < active {
                        GREEN
                    } else {
                        BORDER.opacity(0.7)
                    })
            }))
    }

    fn dynamic_sparkline(values: Vec<u8>) -> impl IntoElement {
        div().w(px(72.)).h(px(22.)).relative().child(
            canvas(
                move |bounds, _, _| bounds,
                move |_, bounds, window, _| {
                    let count = values.len().max(1);
                    let usable_w = 68.0;
                    let usable_h = 16.0;
                    let step = if count > 1 {
                        usable_w / (count - 1) as f32
                    } else {
                        usable_w
                    };
                    let base_y = 19.0;
                    let mut previous: Option<(f32, f32)> = None;

                    for (index, value) in values.iter().enumerate() {
                        let x = 2.0 + index as f32 * step;
                        let y = base_y - (*value as f32 / 100.0) * usable_h;

                        if let Some((prev_x, prev_y)) = previous {
                            for point_index in 1..=4 {
                                let t = point_index as f32 / 4.0;
                                let line_x = prev_x + (x - prev_x) * t;
                                let line_y = prev_y + (y - prev_y) * t;
                                window.paint_quad(gpui::fill(
                                    Bounds {
                                        origin: bounds.origin + point(px(line_x), px(line_y)),
                                        size: size(px(1.3), px(1.3)),
                                    },
                                    GREEN.opacity(0.58),
                                ));
                            }
                        }

                        window.paint_quad(gpui::fill(
                            Bounds {
                                origin: bounds.origin + point(px(x), px(y)),
                                size: size(px(2.4), px(2.4)),
                            },
                            GREEN.opacity(0.92),
                        ));
                        previous = Some((x, y));
                    }
                },
            )
            .absolute()
            .size_full(),
        )
    }

    fn action_button(id: &'static str, text: &'static str) -> gpui::Stateful<Div> {
        div()
            .id(id)
            .cursor_pointer()
            .h(px(42.))
            .px_5()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(2.))
            .border_1()
            .border_color(GREEN.opacity(0.7))
            .bg(GREEN.opacity(0.055))
            .text_sm()
            .text_color(GREEN)
            .hover(|style| style.bg(GREEN.opacity(0.14)).border_color(GREEN))
            .child(text)
    }

    fn option_button(
        id: impl Into<gpui::ElementId>,
        text: impl Into<gpui::SharedString>,
        active: bool,
    ) -> gpui::Stateful<Div> {
        let text = text.into();
        div()
            .id(id)
            .cursor_pointer()
            .h(px(38.))
            .px_4()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(2.))
            .border_1()
            .border_color(if active { GREEN } else { BORDER })
            .bg(if active { GREEN.opacity(0.09) } else { BG })
            .text_xs()
            .text_color(if active { GREEN } else { MUTED })
            .child(text)
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let pages = [
            Page::Dashboard,
            Page::Extract,
            Page::Compress,
            Page::AutoExtract,
            Page::Recursive,
            Page::Batch,
            Page::Passwords,
            Page::Settings,
        ];

        div()
            .w(px(222.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(BG)
            .border_r_1()
            .border_color(BORDER)
            .child(
                div()
                    .flex_1()
                    .p_3()
                    .child(
                        div()
                            .px_3()
                            .pt_2()
                            .pb_3()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(Self::label("TACTICAL MODULES"))
                            .child(div().text_xs().text_color(MUTED).child("07 ONLINE")),
                    )
                    .children(pages.into_iter().enumerate().map(|(index, page)| {
                        let active = self.page == page;
                        div()
                            .id(("nav", index))
                            .cursor_pointer()
                            .h(px(56.))
                            .mt_1()
                            .px_3()
                            .flex()
                            .items_center()
                            .gap_3()
                            .rounded(px(2.))
                            .border_1()
                            .border_color(if active { GREEN.opacity(0.45) } else { BG })
                            .bg(if active { GREEN.opacity(0.08) } else { BG })
                            .text_color(if active { GREEN } else { TEXT })
                            .child(
                                div()
                                    .w(px(34.))
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .text_color(if active { GREEN } else { MUTED })
                                    .child(div().text_xs().child(format!("M·{:02}", index + 1)))
                                    .child(div().text_base().child(page.glyph())),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(div().text_sm().child(page.label()))
                                    .child(div().text_xs().text_color(MUTED).child(page.detail())),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.page = page;
                                cx.notify();
                            }))
                    })),
            )
            .child(
                div()
                    .h(px(52.))
                    .px_4()
                    .flex()
                    .items_center()
                    .border_t_1()
                    .border_color(BORDER)
                    .text_sm()
                    .text_color(MUTED)
                    .child("SYS·CONFIG  /  设置"),
            )
    }

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h(px(68.))
            .flex_none()
            .px_6()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(BORDER)
            .bg(BG)
            .child(
                div()
                    .w(px(310.))
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().text_color(GREEN.opacity(0.24)).child("●"))
                    .child(div().text_color(GREEN.opacity(0.5)).child("●"))
                    .child(div().text_color(GREEN).child("●"))
                    .child(div().h(px(30.)).w(px(1.)).mx_2().bg(BORDER))
                    .child(
                        div()
                            .font_family("Major Mono Display")
                            .text_xl()
                            .text_color(TEXT)
                            .child("GEEKZIP"),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded(px(2.))
                            .border_1()
                            .border_color(GREEN.opacity(0.65))
                            .text_xs()
                            .text_color(GREEN)
                            .child("TACTICAL"),
                    ),
            )
            .child(
                div().flex_1().flex().justify_center().child(
                    div()
                        .w(px(382.))
                        .h(px(44.))
                        .flex()
                        .rounded(px(2.))
                        .border_1()
                        .border_color(BORDER)
                        .overflow_hidden()
                        .children(
                            [
                                ("标准", AppMode::Normal),
                                ("专业", AppMode::Pro),
                                ("终端", AppMode::Terminal),
                            ]
                            .into_iter()
                            .enumerate()
                            .map(|(index, (label, mode))| {
                                let active = self.mode == mode;
                                div()
                                    .id(("mode", index))
                                    .cursor_pointer()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .border_r_1()
                                    .border_color(BORDER)
                                    .bg(if active { PANEL } else { BG })
                                    .text_sm()
                                    .text_color(if active { GREEN } else { MUTED })
                                    .child(format!("[ {} ]", label))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.mode = mode;
                                        if mode == AppMode::Normal
                                            && !matches!(this.page, Page::Extract | Page::Compress)
                                        {
                                            this.page = Page::Extract;
                                        }
                                        cx.notify();
                                    }))
                            }),
                        ),
                ),
            )
            .child(
                div()
                    .w(px(310.))
                    .flex()
                    .justify_end()
                    .gap_5()
                    .text_sm()
                    .text_color(TEXT)
                    .child("NET/LOCAL")
                    .child("SYS")
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_color(GREEN)
                            .child(
                                div()
                                    .size(px(if self.led_pulse { 9. } else { 6. }))
                                    .rounded_full()
                                    .bg(GREEN.opacity(if self.led_pulse { 1.0 } else { 0.45 })),
                            )
                            .child("LINK"),
                    ),
            )
    }

    fn drop_zone(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Self::panel()
            .h(px(295.))
            .m_4()
            .relative()
            .overflow_hidden()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .child(Self::dot_grid())
            .child(
                div()
                    .font_family("JetBrains Mono")
                    .text_xs()
                    .text_color(GREEN.opacity(0.7))
                    .child("TARGET // ARCHIVE INPUT ZONE"),
            )
            .child(div().mt_4().text_2xl().text_color(GREEN).child("▣"))
            .child(div().mt_4().child(Self::dot_matrix_title()))
            .child(
                div()
                    .mt_2()
                    .text_sm()
                    .text_color(MUTED)
                    .child("或点击选择本地文件"),
            )
            .child(
                div()
                    .mt_7()
                    .flex()
                    .gap_4()
                    .child(
                        div()
                            .id("dashboard-pick-file")
                            .cursor_pointer()
                            .w(px(214.))
                            .h(px(42.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.))
                            .border_1()
                            .border_color(GREEN.opacity(0.65))
                            .text_sm()
                            .text_color(GREEN)
                            .child("⇧  选择文件     ⌘ O")
                            .on_click(cx.listener(|_this, _, window, cx| {
                                let prompt = cx.prompt_for_paths(PathPromptOptions {
                                    files: true,
                                    directories: false,
                                    multiple: true,
                                    prompt: Some("选择一个或多个压缩文件".into()),
                                });
                                cx.spawn_in(window, async move |this, window| {
                                    let paths = prompt.await.ok()?.ok()??;
                                    this.update_in(window, |this, _, cx| {
                                        let count = paths.len();
                                        this.add_extract_archives(paths.clone());
                                        this.operation = "已选择压缩文件".into();
                                        this.result = format!("已加入 {count} 个压缩文件");
                                        for path in paths {
                                            this.operation_log
                                                .push(format!("[SELECT] {}", path.display()));
                                        }
                                        cx.notify();
                                    })
                                    .ok()?;
                                    Some(())
                                })
                                .detach();
                            })),
                    )
                    .child(
                        div()
                            .id("dashboard-pick-folder")
                            .cursor_pointer()
                            .w(px(214.))
                            .h(px(42.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.))
                            .border_1()
                            .border_color(BORDER)
                            .text_sm()
                            .text_color(TEXT)
                            .child("▭  选择文件夹   ⇧⌘ O")
                            .on_click(cx.listener(|_this, _, window, cx| {
                                let prompt = cx.prompt_for_paths(PathPromptOptions {
                                    files: false,
                                    directories: true,
                                    multiple: false,
                                    prompt: Some("选择批量解压目录".into()),
                                });
                                cx.spawn_in(window, async move |this, window| {
                                    let path = prompt.await.ok()?.ok()??.into_iter().next()?;
                                    this.update_in(window, |this, _, cx| {
                                        this.batch_dir = Some(path);
                                        this.page = Page::Batch;
                                        cx.notify();
                                    })
                                    .ok()?;
                                    Some(())
                                })
                                .detach();
                            })),
                    ),
            )
    }

    fn dashboard(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .min_w_0()
            .flex()
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .h_full()
                    .id("dashboard-scroll")
                    .overflow_y_scrollbar()
                    .border_r_1()
                    .border_color(BORDER)
                    .child(self.drop_zone(cx))
                    .child(
                        div()
                            .px_4()
                            .py_5()
                            .border_t_1()
                            .border_color(BORDER)
                            .child(
                                Self::label(if self.busy {
                                    "当前任务（1）"
                                } else {
                                    "当前任务（0）"
                                })
                                .mb_3(),
                            )
                            .child(
                                Self::panel()
                                    .h(px(96.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_sm()
                                    .text_color(if self.busy { GREEN } else { MUTED })
                                    .child(if self.busy {
                                        format!("{} · {}", self.operation, self.result)
                                    } else {
                                        "暂无运行中的任务".into()
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_5()
                            .border_t_1()
                            .border_color(BORDER)
                            .child(Self::label("最近归档").mb_3())
                            .child(self.managed_results_panel(cx)),
                    ),
            )
            .child(self.inspector())
    }

    fn managed_results_panel(&self, cx: &mut Context<Self>) -> Div {
        let selected = self
            .selected_result
            .and_then(|index| self.managed_results.get(index));
        Self::panel()
            .min_h(px(210.))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(if self.managed_results.is_empty() {
                div()
                    .h(px(86.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(MUTED)
                    .child("暂无可管理结果")
            } else {
                div().flex().flex_col().gap_2().children(
                    self.managed_results
                        .iter()
                        .enumerate()
                        .take(8)
                        .map(|(index, item)| {
                            let active = self.selected_result == Some(index);
                            let exists = item.path.exists();
                            div()
                                .id(("managed-result", index))
                                .cursor_pointer()
                                .min_h(px(48.))
                                .px_3()
                                .py_2()
                                .rounded(px(3.))
                                .border_1()
                                .border_color(if active { GREEN } else { BORDER })
                                .bg(if active { GREEN.opacity(0.07) } else { BG })
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .w(px(74.))
                                        .flex_none()
                                        .text_xs()
                                        .text_color(if active { GREEN } else { MUTED })
                                        .child(item.kind.label()),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .text_xs()
                                        .text_color(TEXT)
                                        .overflow_hidden()
                                        .child(item.path.to_string_lossy().to_string()),
                                )
                                .child(
                                    div()
                                        .w(px(48.))
                                        .flex_none()
                                        .text_xs()
                                        .text_color(if exists { GREEN } else { MUTED })
                                        .child(if exists { "存在" } else { "缺失" }),
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.selected_result = Some(index);
                                    cx.notify();
                                }))
                        }),
                )
            })
            .child(div().h(px(1.)).bg(BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .w(px(76.))
                            .text_xs()
                            .text_color(MUTED)
                            .child("当前选择"),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_xs()
                            .text_color(if selected.is_some() { TEXT } else { MUTED })
                            .overflow_hidden()
                            .child(
                                selected
                                    .map(|item| item.path.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "未选择".into()),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Self::panel()
                            .h(px(42.))
                            .min_w_0()
                            .flex_1()
                            .px_3()
                            .flex()
                            .items_center()
                            .child(
                                Input::new(&self.result_rename_input)
                                    .w_full()
                                    .text_color(TEXT)
                                    .text_sm(),
                            ),
                    )
                    .child(
                        Self::option_button("open-result", "打开", false).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.open_selected_result();
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        Self::option_button("rename-result", "重命名", false).on_click(
                            cx.listener(|this, _, window, cx| {
                                this.rename_selected_result(window, cx);
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        Self::option_button("remove-result", "移除", false).on_click(cx.listener(
                            |this, _, _, cx| {
                                if let Some(index) = this.selected_result {
                                    if index < this.managed_results.len() {
                                        let item = this.managed_results.remove(index);
                                        this.result =
                                            format!("已从列表移除 {}", item.path.display());
                                    }
                                    this.selected_result = None;
                                    cx.notify();
                                }
                            },
                        )),
                    ),
            )
    }

    fn inspector(&self) -> impl IntoElement {
        let selected = self.archive_path.as_ref();
        let rows = if selected.is_some() {
            self.archive_detail.rows.clone()
        } else {
            Vec::new()
        };

        div()
            .w(px(330.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(BG)
            .child(
                div()
                    .p_5()
                    .border_b_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .pb_4()
                            .border_b_1()
                            .border_color(BORDER)
                            .text_base()
                            .text_color(GREEN)
                            .child(
                                selected
                                    .and_then(|path| path.file_name())
                                    .map(|name| name.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "尚未选择文件".into()),
                            ),
                    )
                    .child(
                        div()
                            .py_4()
                            .border_b_1()
                            .border_color(BORDER)
                            .flex()
                            .flex_col()
                            .gap_3()
                            .children(rows.into_iter().map(|(key, value)| {
                                div()
                                    .flex()
                                    .items_start()
                                    .gap_2()
                                    .text_xs()
                                    .child(
                                        div().w(px(86.)).flex_none().text_color(MUTED).child(key),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .overflow_hidden()
                                            .text_color(TEXT)
                                            .child(value),
                                    )
                            })),
                    )
                    .child(
                        div()
                            .mt_4()
                            .h(px(44.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.))
                            .border_1()
                            .border_color(BORDER)
                            .text_sm()
                            .text_color(GREEN)
                            .child(if selected.is_some() {
                                self.archive_detail.status.clone()
                            } else {
                                "等待选择文件"
                            }),
                    ),
            )
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .p_5()
                    .id("log-scroll")
                    .overflow_y_scrollbar()
                    .children(
                        (!self.archive_detail.entries.is_empty()).then(|| self.zip_browser_panel()),
                    )
                    .child(Self::label("日志").mb_4())
                    .children(
                        self.operation_log
                            .iter()
                            .cloned()
                            .map(|line| div().mb_2().text_xs().text_color(MUTED).child(line)),
                    )
                    .child(if self.operation_log.is_empty() {
                        div().text_xs().text_color(MUTED).child("暂无日志")
                    } else {
                        div().text_color(GREEN).child("...")
                    }),
            )
    }

    fn zip_browser_panel(&self) -> Div {
        let total = self.archive_detail.entries.len();
        Self::panel()
            .mb_5()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(Self::label("ZIP 浏览"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(MUTED)
                            .child(format!("{} 项", total)),
                    ),
            )
            .children(
                self.archive_detail
                    .entries
                    .iter()
                    .take(200)
                    .enumerate()
                    .map(|(index, entry)| {
                        div()
                            .min_h(px(38.))
                            .px_2()
                            .py_2()
                            .rounded(px(2.))
                            .border_1()
                            .border_color(BORDER.opacity(0.72))
                            .bg(if index % 2 == 0 { BG } else { PANEL })
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .child(
                                div()
                                    .w(px(40.))
                                    .flex_none()
                                    .text_color(if entry.kind == "DIR" { GREEN } else { MUTED })
                                    .child(entry.kind),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .overflow_hidden()
                                    .text_color(TEXT)
                                    .child(entry.name.clone()),
                            )
                            .child(
                                div()
                                    .w(px(70.))
                                    .flex_none()
                                    .text_color(MUTED)
                                    .child(Self::format_bytes(entry.size)),
                            )
                            .child(
                                div()
                                    .w(px(70.))
                                    .flex_none()
                                    .text_color(MUTED)
                                    .child(Self::format_bytes(entry.compressed_size)),
                            )
                            .child(
                                div()
                                    .w(px(24.))
                                    .flex_none()
                                    .text_color(if entry.encrypted { GREEN } else { MUTED })
                                    .child(if entry.encrypted { "锁" } else { "" }),
                            )
                    }),
            )
            .children((total > 200).then(|| {
                div()
                    .pt_2()
                    .text_xs()
                    .text_color(MUTED)
                    .child(format!("已显示前 200 项，剩余 {} 项", total - 200))
            }))
    }

    fn path_row(label: &'static str, path: Option<&PathBuf>) -> Div {
        Self::panel()
            .h(px(66.))
            .px_4()
            .flex()
            .items_center()
            .gap_4()
            .child(div().w(px(92.)).text_xs().text_color(MUTED).child(label))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_sm()
                    .text_color(if path.is_some() { TEXT } else { MUTED })
                    .overflow_hidden()
                    .child(
                        path.map(|value| value.to_string_lossy().to_string())
                            .unwrap_or_else(|| "尚未选择".into()),
                    ),
            )
    }

    fn extract_queue_panel(&self, cx: &mut Context<Self>) -> Div {
        let summary = match self.extract_archives.as_slice() {
            [] => "尚未选择".into(),
            [path] => path.to_string_lossy().to_string(),
            paths => format!("已加入 {} 个压缩文件，将同时解压", paths.len()),
        };
        Self::panel()
            .min_h(px(92.))
            .px_4()
            .py_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .w(px(92.))
                            .text_xs()
                            .text_color(MUTED)
                            .child("压缩文件"),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_sm()
                            .text_color(if self.extract_archives.is_empty() {
                                MUTED
                            } else {
                                TEXT
                            })
                            .overflow_hidden()
                            .child(summary),
                    )
                    .child(
                        Self::option_button("clear-extract-queue", "清空", false).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.extract_archives.clear();
                                this.archive_path = None;
                                this.archive_detail = ArchiveDetail::default();
                                this.archive_detail_path = None;
                                this.result = "解压队列已清空".into();
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .children(
                self.extract_archives
                    .iter()
                    .take(6)
                    .enumerate()
                    .map(|(index, path)| {
                        div()
                            .ml(px(106.))
                            .min_w_0()
                            .text_xs()
                            .text_color(if self.archive_path.as_ref() == Some(path) {
                                GREEN
                            } else {
                                MUTED
                            })
                            .overflow_hidden()
                            .child(format!("{:02}  {}", index + 1, path.display()))
                    }),
            )
            .children((self.extract_archives.len() > 6).then(|| {
                div()
                    .ml(px(106.))
                    .text_xs()
                    .text_color(MUTED)
                    .child(format!("... 还有 {} 个", self.extract_archives.len() - 6))
            }))
    }

    fn extract_rename_prefixes(&self, cx: &Context<Self>) -> Vec<String> {
        self.extract_prefix_input
            .read(cx)
            .value()
            .split([',', '，', '\n'])
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect()
    }

    fn operation_panel(&self, cx: &mut Context<Self>) -> Div {
        let progress = if self.result.starts_with("完成") {
            100.0
        } else {
            self.operation_progress
        };
        let current_file = self
            .operation_current_path
            .as_ref()
            .and_then(|path| {
                PathBuf::from(path)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "等待文件流".into());
        let bytes_line = if self.operation_total_bytes > 0 {
            format!(
                "{} / {}",
                Self::format_bytes(self.operation_bytes_done),
                Self::format_bytes(self.operation_total_bytes)
            )
        } else {
            "正在分析任务大小".into()
        };
        let file_line = if self.operation_total_files > 0 {
            format!(
                "{}/{} 个文件",
                self.operation_files_done, self.operation_total_files
            )
        } else {
            "文件清单待生成".into()
        };
        let pause_label = if self.operation_paused {
            "继续"
        } else {
            "暂停"
        };
        Self::panel()
            .w(px(350.))
            .h_full()
            .p_5()
            .flex()
            .flex_col()
            .gap_4()
            .child(Self::label("任务状态"))
            .child(
                div()
                    .text_lg()
                    .text_color(if self.busy { GREEN } else { TEXT })
                    .child(self.operation.clone()),
            )
            .child(Self::segmented_progress(progress))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_xs()
                    .text_color(MUTED)
                    .child(format!("{progress:.1}%"))
                    .child(file_line),
            )
            .child(div().text_sm().text_color(MUTED).child(self.result.clone()))
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(Self::resource_chip("已处理", bytes_line))
                    .child(Self::resource_chip(
                        "速度",
                        if self.operation_speed_bps > 0 {
                            format!("{}/s", Self::format_bytes(self.operation_speed_bps))
                        } else {
                            "--".into()
                        },
                    ))
                    .child(Self::resource_chip(
                        "ETA",
                        Self::format_duration(self.operation_eta_seconds),
                    ))
                    .child(Self::resource_chip("当前文件", current_file)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Self::option_button("pause-operation", pause_label, self.operation_paused)
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(control) = this.operation_control.as_ref() {
                                    if this.operation_paused {
                                        control.resume();
                                        this.operation_paused = false;
                                        this.operation_log.push("[TASK] resumed".into());
                                    } else {
                                        control.pause();
                                        this.operation_paused = true;
                                        this.operation_log.push("[TASK] paused".into());
                                    }
                                    cx.notify();
                                }
                            })),
                    )
                    .child(
                        Self::option_button("cancel-operation", "取消", false).on_click(
                            cx.listener(|this, _, _, cx| {
                                if let Some(control) = this.operation_control.as_ref() {
                                    control.cancel();
                                    this.operation = "正在取消".into();
                                    this.result = "已请求取消当前任务".into();
                                    this.operation_log.push("[TASK] cancel requested".into());
                                    cx.notify();
                                }
                            }),
                        ),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(Self::resource_chip(
                        "GeekZip CPU",
                        format!("{}%", self.resource_stats.process_cpu),
                    ))
                    .child(Self::resource_chip(
                        "系统 CPU",
                        format!("{}%", self.resource_stats.system_cpu),
                    ))
                    .child(Self::resource_chip(
                        "GPU",
                        self.resource_stats
                            .gpu
                            .map(|usage| format!("{usage}%"))
                            .unwrap_or_else(|| "不可用".into()),
                    ))
                    .child(Self::resource_chip(
                        "进程内存",
                        format!("{} MB", self.resource_stats.process_memory_mb),
                    )),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(if self.busy { GREEN } else { MUTED })
                    .child(if self.busy {
                        "执行引擎：CPU + 磁盘 I/O（GPU 仅监控）"
                    } else {
                        "执行引擎待命"
                    }),
            )
            .child(div().h(px(1.)).bg(BORDER))
            .child(Self::label("运行日志"))
            .child(
                div()
                    .id("operation-log")
                    .min_h_0()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .children(
                        self.operation_log
                            .iter()
                            .cloned()
                            .map(|line| div().mb_2().text_xs().text_color(MUTED).child(line)),
                    ),
            )
    }

    fn resource_chip(label: &'static str, value: String) -> Div {
        div()
            .h(px(48.))
            .px_3()
            .flex()
            .flex_col()
            .justify_center()
            .gap_1()
            .rounded(px(4.))
            .border_1()
            .border_color(BORDER)
            .bg(BG)
            .child(div().text_xs().text_color(MUTED).child(label))
            .child(div().text_sm().text_color(GREEN).child(value))
    }

    fn extract_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_5()
            .flex()
            .gap_4()
            .child(
                Self::panel()
                    .id("extract-scroll")
                    .overflow_y_scrollbar()
                    .min_w_0()
                    .flex_1()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(Self::label("EXTRACT / 解压缩"))
                    .child(
                        div()
                            .text_2xl()
                            .text_color(TEXT)
                            .child("选择压缩包并交给 Rust 核心"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(MUTED)
                            .child("自动读取密码本并按顺序尝试密码，支持 Magic Bytes 格式识别。"),
                    )
                    .child(self.extract_queue_panel(cx))
                    .child(Self::path_row(
                        "默认目录",
                        self.default_extract_dir.as_ref(),
                    ))
                    .child(Self::path_row("目标目录", self.target_dir.as_ref()))
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(
                                Self::action_button("pick-archive", "选择压缩文件").on_click(
                                    cx.listener(|_this, _, window, cx| {
                                        let prompt = cx.prompt_for_paths(PathPromptOptions {
                                            files: true,
                                            directories: false,
                                            multiple: true,
                                            prompt: Some("选择一个或多个要解压的压缩文件".into()),
                                        });
                                        cx.spawn_in(window, async move |this, window| {
                                            let paths = prompt.await.ok()?.ok()??;
                                            this.update_in(window, |this, _, cx| {
                                                let count = paths.len();
                                                this.add_extract_archives(paths.clone());
                                                this.result = format!("已加入 {count} 个压缩文件");
                                                for path in paths {
                                                    this.operation_log.push(format!(
                                                        "[SELECT] {}",
                                                        path.display()
                                                    ));
                                                }
                                                cx.notify();
                                            })
                                            .ok()?;
                                            Some(())
                                        })
                                        .detach();
                                    }),
                                ),
                            )
                            .child(
                                Self::action_button("pick-default-target", "默认目录").on_click(
                                    cx.listener(|_this, _, window, cx| {
                                        let prompt = cx.prompt_for_paths(PathPromptOptions {
                                            files: false,
                                            directories: true,
                                            multiple: false,
                                            prompt: Some("选择默认解压目录".into()),
                                        });
                                        cx.spawn_in(window, async move |this, window| {
                                            let path =
                                                prompt.await.ok()?.ok()??.into_iter().next()?;
                                            this.update_in(window, |this, _, cx| {
                                                this.default_extract_dir = Some(path);
                                                this.save_settings(cx);
                                                cx.notify();
                                            })
                                            .ok()?;
                                            Some(())
                                        })
                                        .detach();
                                    }),
                                ),
                            )
                            .child(Self::action_button("pick-target", "选择目标目录").on_click(
                                cx.listener(|_this, _, window, cx| {
                                    let prompt = cx.prompt_for_paths(PathPromptOptions {
                                        files: false,
                                        directories: true,
                                        multiple: false,
                                        prompt: Some("选择解压目录".into()),
                                    });
                                    cx.spawn_in(window, async move |this, window| {
                                        let path = prompt.await.ok()?.ok()??.into_iter().next()?;
                                        this.update_in(window, |this, _, cx| {
                                            this.target_dir = Some(path);
                                            cx.notify();
                                        })
                                        .ok()?;
                                        Some(())
                                    })
                                    .detach();
                                }),
                            )),
                    )
                    .child(Self::label("解压后重命名"))
                    .child(
                        Self::panel().h(px(52.)).px_4().flex().items_center().child(
                            Input::new(&self.extract_prefix_input)
                                .w_full()
                                .text_color(TEXT)
                                .text_sm(),
                        ),
                    )
                    .child(Self::label("目录整理"))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Self::option_button(
                                    ("flatten-root", 0),
                                    "只保留一层",
                                    self.flatten_single_root,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.flatten_single_root = true;
                                        this.save_settings(cx);
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                Self::option_button(
                                    ("flatten-root", 1),
                                    "保留原结构",
                                    !self.flatten_single_root,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.flatten_single_root = false;
                                        this.save_settings(cx);
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(
                        Self::action_button("save-extract-settings", "保存解压设置").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.save_settings(cx);
                                this.result = "解压设置已保存".into();
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        Self::action_button(
                            "run-extract",
                            if self.busy {
                                "处理中..."
                            } else {
                                "开始智能解压"
                            },
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.start_extract(cx))),
                    ),
            )
            .child(self.operation_panel(cx))
    }

    fn start_extract(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let paths = if self.extract_archives.is_empty() {
            self.archive_path.iter().cloned().collect::<Vec<_>>()
        } else {
            self.extract_archives.clone()
        };
        if paths.is_empty() {
            self.result = "请先选择压缩文件".into();
            cx.notify();
            return;
        };
        self.save_settings(cx);
        let target = self.target_dir.clone();
        let default_target = self.default_extract_dir.clone();
        let delete_after = self.context_action.as_deref() == Some("delete");
        let rename_prefixes = self.extract_rename_prefixes(cx);
        let flatten_single_root = self.flatten_single_root;
        let passwords = self.passwords.clone();
        let control = OperationControl::new();
        let worker_control = control.clone();
        let shared_progress: Arc<Mutex<Option<ProgressUpdate>>> = Arc::new(Mutex::new(None));
        let worker_progress = shared_progress.clone();
        self.busy = true;
        self.operation = if paths.len() > 1 {
            "正在同时解压".into()
        } else {
            "正在解压".into()
        };
        self.result = if paths.len() > 1 {
            format!("并行启动 {} 个解压任务", paths.len())
        } else {
            paths[0]
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        };
        self.reset_operation_progress(control);
        self.operation_total_files = paths.len();
        self.operation_log
            .push(format!("[RUN] 启动 {} 个 ExtractEngine", paths.len()));
        self.start_progress_pump(shared_progress, cx);
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let total_inputs = paths.len();
                    let initial_totals = paths
                        .iter()
                        .map(|path| {
                            fs::metadata(path)
                                .ok()
                                .map(|metadata| metadata.len())
                                .unwrap_or_default()
                        })
                        .collect::<Vec<_>>();
                    let aggregate_bytes = Arc::new(Mutex::new(vec![0u64; total_inputs]));
                    let aggregate_totals = Arc::new(Mutex::new(initial_totals));
                    let aggregate_done = Arc::new(Mutex::new(vec![false; total_inputs]));
                    let mut handles = Vec::new();

                    for (index, path) in paths.into_iter().enumerate() {
                        let target_dir = target.as_ref().map(|value| {
                            if total_inputs > 1 {
                                let stem = path
                                    .file_stem()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                value.join(stem).to_string_lossy().to_string()
                            } else {
                                value.to_string_lossy().to_string()
                            }
                        });
                        let options = ExtractOptions {
                            target_dir,
                            default_target_dir: default_target
                                .as_ref()
                                .map(|value| value.to_string_lossy().to_string()),
                            password_candidates: passwords.clone(),
                            delete_after,
                            rename_prefixes: rename_prefixes.clone(),
                            flatten_single_root,
                            ..Default::default()
                        };
                        let task_control = worker_control.clone();
                        let task_progress = worker_progress.clone();
                        let task_bytes = aggregate_bytes.clone();
                        let task_totals = aggregate_totals.clone();
                        let task_done = aggregate_done.clone();
                        handles.push(std::thread::spawn(move || {
                            let progress = |update: ProgressUpdate| {
                                let bytes_done = if let Ok(mut bytes) = task_bytes.lock() {
                                    bytes[index] = update.bytes_done;
                                    bytes.iter().sum()
                                } else {
                                    update.bytes_done
                                };
                                let total_bytes = if let Ok(mut totals) = task_totals.lock() {
                                    if update.total_bytes > 0 {
                                        totals[index] = update.total_bytes;
                                    }
                                    totals.iter().sum()
                                } else {
                                    update.total_bytes
                                };
                                let files_done = task_done
                                    .lock()
                                    .map(|done| done.iter().filter(|value| **value).count())
                                    .unwrap_or_default();
                                if let Ok(mut slot) = task_progress.lock() {
                                    *slot = Some(ProgressUpdate {
                                        phase: if total_inputs > 1 {
                                            "正在同时解压".into()
                                        } else {
                                            update.phase.clone()
                                        },
                                        current_path: update.current_path.clone(),
                                        bytes_done,
                                        total_bytes,
                                        files_done,
                                        total_files: total_inputs,
                                    });
                                }
                            };
                            let result = ExtractEngine::extract_with_progress(
                                &path,
                                &options,
                                task_control,
                                Some(&progress),
                            );
                            if result.is_ok() {
                                if let Ok(mut done) = task_done.lock() {
                                    done[index] = true;
                                }
                            }
                            (path, result)
                        }));
                    }

                    let mut successes = Vec::new();
                    let mut errors = Vec::new();
                    let mut passwords_used = Vec::new();
                    for handle in handles {
                        match handle.join() {
                            Ok((path, Ok(result))) => {
                                if let Some(password) = result.password_used.clone() {
                                    passwords_used.push(password);
                                }
                                successes.push((path, result));
                            }
                            Ok((path, Err(error))) => {
                                errors.push(format!("{}: {error:#}", path.display()));
                            }
                            Err(_) => errors.push("解压线程异常退出".into()),
                        }
                    }
                    (successes, errors, passwords_used)
                })
                .await;
            _ = cx.update(|cx| {
                _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    this.clear_operation_control();
                    let (successes, errors, passwords_used) = result;
                    for password in passwords_used {
                        if !this.passwords.contains(&password) {
                            this.passwords.push(password.clone());
                            this.save_passwords();
                        }
                        this.operation_log
                            .push(format!("[PASSWORD] 已记住成功密码 {}", password));
                    }
                    if errors.is_empty() {
                        this.operation = if successes.len() > 1 {
                            "全部解压完成".into()
                        } else {
                            "解压完成".into()
                        };
                        this.operation_progress = 100.0;
                        this.result = format!("完成 · 成功 {} 个", successes.len());
                        for (_, result) in &successes {
                            this.record_managed_result(
                                ManagedResultKind::Extract,
                                PathBuf::from(&result.target_dir),
                            );
                        }
                        for (_, result) in successes.iter().take(8) {
                            this.operation_log
                                .push(format!("[OK] 输出到 {}", result.target_dir));
                        }
                    } else if successes.is_empty() {
                        let cancelled = errors.iter().any(|error| error.contains("cancelled"));
                        this.operation = if cancelled {
                            "解压已取消".into()
                        } else {
                            "解压失败".into()
                        };
                        this.result = format!("失败 · {} 个错误", errors.len());
                        this.operation_log.extend(
                            errors
                                .into_iter()
                                .take(8)
                                .map(|error| format!("[ERROR] {error}")),
                        );
                    } else {
                        this.operation = "部分解压完成".into();
                        this.operation_progress = 100.0;
                        this.result =
                            format!("完成 {} 个 · 失败 {} 个", successes.len(), errors.len());
                        for (_, result) in &successes {
                            this.record_managed_result(
                                ManagedResultKind::Extract,
                                PathBuf::from(&result.target_dir),
                            );
                        }
                        for (_, result) in successes.iter().take(6) {
                            this.operation_log
                                .push(format!("[OK] 输出到 {}", result.target_dir));
                        }
                        this.operation_log.extend(
                            errors
                                .into_iter()
                                .take(6)
                                .map(|error| format!("[ERROR] {error}")),
                        );
                    }
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn compress_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let formats = ["ZIP", "TAR.GZ", "TAR.BZ2", "TAR.XZ", "TAR"];
        let source_summary = match self.compress_sources.as_slice() {
            [] => "尚未选择".into(),
            [path] => path.to_string_lossy().to_string(),
            paths => format!("已选择 {} 个文件 / 文件夹", paths.len()),
        };
        let show_compress_level = self.compress_format != 4;
        let volume_options = [
            ("不分卷", None),
            ("10 MB", Some(10)),
            ("50 MB", Some(50)),
            ("100 MB", Some(100)),
            ("500 MB", Some(500)),
        ];
        div()
            .size_full()
            .p_5()
            .flex()
            .gap_4()
            .child(
                Self::panel()
                    .id("compress-scroll")
                    .overflow_y_scrollbar()
                    .min_w_0()
                    .flex_1()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(Self::label("COMPRESS / 压缩"))
                    .child(div().text_2xl().text_color(TEXT).child("压缩文件与文件夹"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(MUTED)
                            .child("输出文件保存在第一个来源的同级目录。"),
                    )
                    .child(Self::label("压缩格式"))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .children(formats.into_iter().enumerate().map(|(index, label)| {
                                Self::option_button(
                                    ("compress-format", index),
                                    label,
                                    self.compress_format == index,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.compress_format = index;
                                        cx.notify();
                                    },
                                ))
                            })),
                    )
                    .children(show_compress_level.then(|| Self::label("压缩等级")))
                    .children(show_compress_level.then(|| {
                        div().flex().gap_2().children((1u32..=9).map(|level| {
                            Self::option_button(
                                ("compress-level", level as usize),
                                level.to_string(),
                                self.compress_level == level,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.compress_level = level;
                                    cx.notify();
                                },
                            ))
                        }))
                    }))
                    .child(Self::label("分卷压缩"))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .children(volume_options.into_iter().enumerate().map(
                                |(index, (label, value))| {
                                    Self::option_button(
                                        ("compress-volume", index),
                                        label,
                                        self.compress_volume_size_mb == value,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.compress_volume_size_mb = value;
                                            cx.notify();
                                        },
                                    ))
                                },
                            )),
                    )
                    .child(Self::label("分卷混淆后缀"))
                    .child(
                        Self::panel().h(px(52.)).px_4().flex().items_center().child(
                            Input::new(&self.compress_suffix_input)
                                .w_full()
                                .text_color(TEXT)
                                .text_sm(),
                        ),
                    )
                    .child(
                        Self::panel()
                            .h(px(76.))
                            .px_4()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(div().w(px(92.)).text_xs().text_color(MUTED).child("来源"))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_sm()
                                    .text_color(TEXT)
                                    .child(source_summary),
                            ),
                    )
                    .child(
                        Self::action_button("pick-compress", "选择文件 / 文件夹").on_click(
                            cx.listener(|_this, _, window, cx| {
                                let prompt = cx.prompt_for_paths(PathPromptOptions {
                                    files: true,
                                    directories: true,
                                    multiple: true,
                                    prompt: Some("选择要压缩的内容".into()),
                                });
                                cx.spawn_in(window, async move |this, window| {
                                    let paths = prompt.await.ok()?.ok()??;
                                    this.update_in(window, |this, _, cx| {
                                        this.compress_sources = paths;
                                        cx.notify();
                                    })
                                    .ok()?;
                                    Some(())
                                })
                                .detach();
                            }),
                        ),
                    )
                    .child(
                        Self::action_button(
                            "run-compress",
                            if self.busy {
                                "处理中..."
                            } else {
                                "开始压缩"
                            },
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.start_compress(cx))),
                    ),
            )
            .child(self.operation_panel(cx))
    }

    fn start_compress(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(first) = self.compress_sources.first().cloned() else {
            self.result = "请先选择文件或文件夹".into();
            cx.notify();
            return;
        };
        let paths = self.compress_sources.clone();
        let format_index = self.compress_format;
        let level = self.compress_level;
        let volume_size_mb = self.compress_volume_size_mb;
        let obfuscate_suffix = self
            .compress_suffix_input
            .read(cx)
            .value()
            .trim()
            .to_string();
        let (format, extension, format_label) = match format_index {
            1 => (CompressFormat::TarGz, "tar.gz", "TAR.GZ"),
            2 => (CompressFormat::TarBz2, "tar.bz2", "TAR.BZ2"),
            3 => (CompressFormat::TarXz, "tar.xz", "TAR.XZ"),
            4 => (CompressFormat::Tar, "tar", "TAR"),
            _ => (CompressFormat::Zip, "zip", "ZIP"),
        };
        let output = first
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!("GeekZip-output.{extension}"));
        let control = OperationControl::new();
        let worker_control = control.clone();
        let shared_progress: Arc<Mutex<Option<ProgressUpdate>>> = Arc::new(Mutex::new(None));
        let worker_progress = shared_progress.clone();
        self.busy = true;
        self.operation = "正在压缩".into();
        self.result = output.to_string_lossy().to_string();
        self.reset_operation_progress(control);
        self.operation_log
            .push(format!("[RUN] 输出 {}", output.display()));
        if let Some(size) = volume_size_mb {
            self.operation_log.push(format!("[VOLUME] 每卷 {size} MB"));
        }
        if !obfuscate_suffix.is_empty() {
            self.operation_log
                .push(format!("[MASK] 后缀 {}", obfuscate_suffix));
        }
        self.start_progress_pump(shared_progress, cx);
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let refs: Vec<_> = paths.iter().map(PathBuf::as_path).collect();
                    let options = CompressOptions {
                        format,
                        level,
                        volume_size_mb,
                        obfuscate_suffix: (!obfuscate_suffix.is_empty())
                            .then_some(obfuscate_suffix),
                        ..Default::default()
                    };
                    let progress = |update: ProgressUpdate| {
                        if let Ok(mut slot) = worker_progress.lock() {
                            *slot = Some(update);
                        }
                    };
                    CompressEngine::compress_with_progress(
                        &refs,
                        &output,
                        &options,
                        worker_control,
                        Some(&progress),
                    )
                    .map(|_| output)
                })
                .await;
            _ = cx.update(|cx| {
                _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    this.clear_operation_control();
                    match result {
                        Ok(output) => {
                            this.operation = "压缩完成".into();
                            this.operation_progress = 100.0;
                            if output.exists() {
                                this.record_managed_result(
                                    ManagedResultKind::Compress,
                                    output.clone(),
                                );
                            } else if let Some(parent) = output.parent() {
                                this.record_managed_result(
                                    ManagedResultKind::Compress,
                                    parent.to_path_buf(),
                                );
                            }
                            this.result = if volume_size_mb.is_some() {
                                format!("完成 · 分卷输出 {}", output.display())
                            } else {
                                format!("完成 · {}", output.display())
                            };
                            this.operation_log
                                .push(format!("[OK] {format_label} 已生成"));
                        }
                        Err(error) => {
                            if error.to_string().contains("cancelled") {
                                this.operation = "压缩已取消".into();
                            } else {
                                this.operation = "压缩失败".into();
                            }
                            this.result = error.to_string();
                            this.operation_log.push(format!("[ERROR] {error:#}"));
                        }
                    }
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn recursive_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_5()
            .flex()
            .gap_4()
            .child(
                Self::panel()
                    .min_w_0()
                    .flex_1()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(Self::label("RECURSIVE / 递归解压"))
                    .child(
                        div()
                            .text_2xl()
                            .text_color(TEXT)
                            .child("连续解开嵌套压缩包"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(MUTED)
                            .child("最大深度 10 层；每一层都会自动使用密码本。"),
                    )
                    .child(Self::path_row("入口文件", self.archive_path.as_ref()))
                    .child(
                        Self::action_button("pick-recursive", "选择入口压缩包").on_click(
                            cx.listener(|_this, _, window, cx| {
                                let prompt = cx.prompt_for_paths(PathPromptOptions {
                                    files: true,
                                    directories: false,
                                    multiple: false,
                                    prompt: Some("选择递归解压入口".into()),
                                });
                                cx.spawn_in(window, async move |this, window| {
                                    let path = prompt.await.ok()?.ok()??.into_iter().next()?;
                                    this.update_in(window, |this, _, cx| {
                                        this.select_archive_path(path);
                                        cx.notify();
                                    })
                                    .ok()?;
                                    Some(())
                                })
                                .detach();
                            }),
                        ),
                    )
                    .child(
                        Self::action_button(
                            "run-recursive",
                            if self.busy {
                                "处理中..."
                            } else {
                                "开始递归解压"
                            },
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.start_recursive(cx))),
                    ),
            )
            .child(self.operation_panel(cx))
    }

    fn start_recursive(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(path) = self.archive_path.clone() else {
            self.result = "请先选择入口压缩包".into();
            cx.notify();
            return;
        };
        self.save_settings(cx);
        let passwords = self.passwords.clone();
        let default_target = self.default_extract_dir.clone();
        let rename_prefixes = self.extract_rename_prefixes(cx);
        let flatten_single_root = self.flatten_single_root;
        self.busy = true;
        self.operation = "正在递归解压".into();
        self.result = "扫描嵌套压缩层...".into();
        self.operation_log
            .push("[RUN] RecursiveExtractor depth=10".into());
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let options = ExtractOptions {
                        default_target_dir: default_target
                            .map(|value| value.to_string_lossy().to_string()),
                        password_candidates: passwords,
                        rename_prefixes,
                        flatten_single_root,
                        ..Default::default()
                    };
                    RecursiveExtractor::new(10).extract_recursive(&path, &options)
                })
                .await;
            _ = cx.update(|cx| {
                _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    match result {
                        Ok(result) => {
                            this.operation = "递归解压完成".into();
                            this.result = format!(
                                "完成 · {} 层 · {} 个文件",
                                result.total_layers, result.total_files
                            );
                            this.operation_log
                                .push(format!("[OK] 已处理 {} 个压缩层", result.results.len()));
                        }
                        Err(error) => {
                            this.operation = "递归解压失败".into();
                            this.result = error.to_string();
                            this.operation_log.push(format!("[ERROR] {error:#}"));
                        }
                    }
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn batch_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_5()
            .flex()
            .gap_4()
            .child(
                Self::panel()
                    .min_w_0()
                    .flex_1()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(Self::label("BATCH / 批量解压"))
                    .child(div().text_2xl().text_color(TEXT).child("扫描整个文件夹"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(MUTED)
                            .child("递归查找可识别压缩包，并分别解压到同名目录。"),
                    )
                    .child(Self::path_row("扫描目录", self.batch_dir.as_ref()))
                    .child(
                        Self::action_button("pick-batch", "选择扫描目录").on_click(cx.listener(
                            |_this, _, window, cx| {
                                let prompt = cx.prompt_for_paths(PathPromptOptions {
                                    files: false,
                                    directories: true,
                                    multiple: false,
                                    prompt: Some("选择批量解压目录".into()),
                                });
                                cx.spawn_in(window, async move |this, window| {
                                    let path = prompt.await.ok()?.ok()??.into_iter().next()?;
                                    this.update_in(window, |this, _, cx| {
                                        this.batch_dir = Some(path);
                                        cx.notify();
                                    })
                                    .ok()?;
                                    Some(())
                                })
                                .detach();
                            },
                        )),
                    )
                    .child(
                        Self::action_button(
                            "run-batch",
                            if self.busy {
                                "处理中..."
                            } else {
                                "开始批量解压"
                            },
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.start_batch(cx))),
                    ),
            )
            .child(self.operation_panel(cx))
    }

    fn archive_paths_in(dir: &PathBuf) -> Vec<PathBuf> {
        walkdir::WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.into_path())
            .filter(|path| detect_format(path).format != ArchiveFormat::Unknown)
            .collect()
    }

    fn start_batch(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(dir) = self.batch_dir.clone() else {
            self.result = "请先选择扫描目录".into();
            cx.notify();
            return;
        };
        self.save_settings(cx);
        let passwords = self.passwords.clone();
        let default_target = self.default_extract_dir.clone();
        let rename_prefixes = self.extract_rename_prefixes(cx);
        let flatten_single_root = self.flatten_single_root;
        self.busy = true;
        self.operation = "正在批量解压".into();
        self.result = "扫描压缩文件...".into();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let paths = Self::archive_paths_in(&dir);
                    let found = paths.len();
                    let mut completed = 0usize;
                    let mut errors = Vec::new();
                    for path in paths {
                        let options = ExtractOptions {
                            default_target_dir: default_target
                                .as_ref()
                                .map(|value| value.to_string_lossy().to_string()),
                            password_candidates: passwords.clone(),
                            rename_prefixes: rename_prefixes.clone(),
                            flatten_single_root,
                            ..Default::default()
                        };
                        match ExtractEngine::extract(&path, &options) {
                            Ok(_) => completed += 1,
                            Err(error) => errors.push(format!("{}: {error}", path.display())),
                        }
                    }
                    (found, completed, errors)
                })
                .await;
            _ = cx.update(|cx| {
                _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    let (found, completed, errors) = result;
                    this.operation = "批量解压完成".into();
                    this.result = format!("完成 · 找到 {found} 个 · 成功 {completed} 个");
                    this.operation_log
                        .push(format!("[OK] {completed}/{found} archives"));
                    this.operation_log.extend(
                        errors
                            .into_iter()
                            .take(8)
                            .map(|error| format!("[ERROR] {error}")),
                    );
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn auto_extract_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let watching = self.watch_task.is_some();
        div()
            .size_full()
            .p_5()
            .flex()
            .gap_4()
            .child(
                Self::panel()
                    .min_w_0()
                    .flex_1()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(Self::label("AUTO / 自动解压"))
                    .child(div().text_2xl().text_color(TEXT).child("监控下载目录"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(MUTED)
                            .child("每 2 秒扫描新压缩包；发现后自动使用密码本解压。"),
                    )
                    .child(Self::path_row("监控目录", self.watch_dir.as_ref()))
                    .child(Self::path_row(
                        "默认输出",
                        self.default_extract_dir.as_ref(),
                    ))
                    .child(
                        Self::action_button("pick-watch", "选择监控目录").on_click(cx.listener(
                            |_this, _, window, cx| {
                                let prompt = cx.prompt_for_paths(PathPromptOptions {
                                    files: false,
                                    directories: true,
                                    multiple: false,
                                    prompt: Some("选择自动解压目录".into()),
                                });
                                cx.spawn_in(window, async move |this, window| {
                                    let path = prompt.await.ok()?.ok()??.into_iter().next()?;
                                    this.update_in(window, |this, _, cx| {
                                        this.watch_dir = Some(path);
                                        cx.notify();
                                    })
                                    .ok()?;
                                    Some(())
                                })
                                .detach();
                            },
                        )),
                    )
                    .child(
                        Self::action_button(
                            "toggle-watch",
                            if watching {
                                "停止自动解压"
                            } else {
                                "启动自动解压"
                            },
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_watcher(cx))),
                    ),
            )
            .child(self.operation_panel(cx))
    }

    fn toggle_watcher(&mut self, cx: &mut Context<Self>) {
        if self.watch_task.take().is_some() {
            self.operation = "自动解压已停止".into();
            self.result = "监控任务已关闭".into();
            self.operation_log.push("[WATCH] stopped".into());
            cx.notify();
            return;
        }
        let Some(dir) = self.watch_dir.clone() else {
            self.result = "请先选择监控目录".into();
            cx.notify();
            return;
        };
        self.save_settings(cx);
        let passwords = self.passwords.clone();
        let default_target = self.default_extract_dir.clone();
        let rename_prefixes = self.extract_rename_prefixes(cx);
        let flatten_single_root = self.flatten_single_root;
        self.operation = "自动解压运行中".into();
        self.result = format!("正在监控 {}", dir.display());
        self.operation_log.push("[WATCH] started".into());

        self.watch_task = Some(cx.spawn(async move |this, cx| {
            let mut seen = HashSet::new();
            loop {
                cx.background_executor().timer(Duration::from_secs(2)).await;
                let scan_dir = dir.clone();
                let known = seen.clone();
                let paths = cx
                    .background_executor()
                    .spawn(async move {
                        Self::archive_paths_in(&scan_dir)
                            .into_iter()
                            .filter(|path| !known.contains(path))
                            .collect::<Vec<_>>()
                    })
                    .await;
                for path in paths {
                    seen.insert(path.clone());
                    let options = ExtractOptions {
                        default_target_dir: default_target
                            .as_ref()
                            .map(|value| value.to_string_lossy().to_string()),
                        password_candidates: passwords.clone(),
                        rename_prefixes: rename_prefixes.clone(),
                        flatten_single_root,
                        ..Default::default()
                    };
                    let result = cx
                        .background_executor()
                        .spawn({
                            let path = path.clone();
                            async move { ExtractEngine::extract(&path, &options) }
                        })
                        .await;
                    _ = cx.update(|cx| {
                        _ = this.update(cx, |this, cx| {
                            match result {
                                Ok(result) => this.operation_log.push(format!(
                                    "[AUTO OK] {} -> {}",
                                    path.display(),
                                    result.target_dir
                                )),
                                Err(error) => this
                                    .operation_log
                                    .push(format!("[AUTO ERROR] {}: {error}", path.display())),
                            }
                            cx.notify();
                        });
                    });
                }
            }
        }));
        cx.notify();
    }

    fn passwords_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_5()
            .flex()
            .gap_4()
            .child(
                Self::panel()
                    .min_w_0()
                    .flex_1()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(Self::label("PASSWORD BOOK / 密码本"))
                    .child(div().text_2xl().text_color(TEXT).child("自动解压密码候选"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(MUTED)
                            .child("解压加密 ZIP 时会自动依次尝试。成功密码也会自动写回密码本。"),
                    )
                    .child(
                        Input::new(&self.password_input)
                            .mask_toggle()
                            .cleanable(true),
                    )
                    .child(
                        Self::action_button("add-password", "添加到密码本").on_click(cx.listener(
                            |this, _, window, cx| {
                                let value = this.password_input.read(cx).value().trim().to_string();
                                if value.is_empty() || this.passwords.contains(&value) {
                                    return;
                                }
                                this.passwords.push(value);
                                this.save_passwords();
                                this.password_input
                                    .update(cx, |input, cx| input.set_value("", window, cx));
                                this.result = "密码已保存".into();
                                cx.notify();
                            },
                        )),
                    )
                    .child(div().h(px(1.)).bg(BORDER))
                    .children(self.passwords.iter().enumerate().map(|(index, password)| {
                        Self::panel()
                            .h(px(54.))
                            .px_4()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(
                                div()
                                    .w(px(32.))
                                    .text_xs()
                                    .text_color(GREEN)
                                    .child(format!("{:02}", index + 1)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .text_color(TEXT)
                                    .child(password.clone()),
                            )
                            .child(div().text_xs().text_color(MUTED).child("AUTO TRY"))
                    })),
            )
            .child(self.operation_panel(cx))
    }

    fn settings_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_5()
            .flex()
            .gap_4()
            .child(
                Self::panel()
                    .min_w_0()
                    .flex_1()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(Self::label("SETTINGS / 设置"))
                    .child(div().text_2xl().text_color(TEXT).child("Windows 右键菜单"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(MUTED)
                            .child("安装到当前用户无需管理员；安装到所有用户会触发 UAC。"),
                    )
                    .child(Self::label("安装范围"))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Self::option_button(
                                    ("context-scope", 0),
                                    "当前用户",
                                    !self.windows_context_menu_machine,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.windows_context_menu_machine = false;
                                        this.save_settings(cx);
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                Self::option_button(
                                    ("context-scope", 1),
                                    "所有用户",
                                    self.windows_context_menu_machine,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.windows_context_menu_machine = true;
                                        this.save_settings(cx);
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(Self::label("菜单内容"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(self.context_option(cx, 0, "智能解压", |opts| {
                                        &mut opts.smart_extract
                                    }))
                                    .child(self.context_option(
                                        cx,
                                        1,
                                        "解压到当前目录",
                                        |opts| &mut opts.extract_here,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(self.context_option(
                                        cx,
                                        2,
                                        "解压到文件名文件夹",
                                        |opts| &mut opts.extract_to_folder,
                                    ))
                                    .child(self.context_option(
                                        cx,
                                        3,
                                        "解压并删除源文件",
                                        |opts| &mut opts.extract_delete,
                                    )),
                            )
                            .child(self.context_option(cx, 4, "打开 GeekZip", |opts| {
                                &mut opts.open_app
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(
                                Self::action_button("install-context-menu", "安装右键菜单")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.save_settings(cx);
                                        match this.install_windows_context_menu_from_settings() {
                                            Ok(_) => {
                                                this.operation = "右键菜单已安装".into();
                                                this.result = if this.windows_context_menu_machine {
                                                    "已写入所有用户菜单".into()
                                                } else {
                                                    "已写入当前用户菜单".into()
                                                };
                                                this.operation_log.push(
                                                    "[OK] Windows context menu installed".into(),
                                                );
                                            }
                                            Err(error) => {
                                                this.operation = "右键菜单安装失败".into();
                                                this.result = error.to_string();
                                                this.operation_log
                                                    .push(format!("[ERROR] {error:#}"));
                                            }
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Self::action_button("uninstall-context-menu", "卸载右键菜单")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        match this.uninstall_windows_context_menu_from_settings() {
                                            Ok(_) => {
                                                this.operation = "右键菜单已卸载".into();
                                                this.result = "GeekZip 菜单已移除".into();
                                                this.operation_log.push(
                                                    "[OK] Windows context menu removed".into(),
                                                );
                                            }
                                            Err(error) => {
                                                this.operation = "右键菜单卸载失败".into();
                                                this.result = error.to_string();
                                                this.operation_log
                                                    .push(format!("[ERROR] {error:#}"));
                                            }
                                        }
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(self.operation_panel(cx))
    }

    fn context_option(
        &self,
        cx: &mut Context<Self>,
        index: usize,
        label: &'static str,
        field: fn(&mut WindowsContextMenuOptions) -> &mut bool,
    ) -> gpui::Stateful<Div> {
        let mut options = self.windows_context_menu.clone();
        let active = *field(&mut options);
        Self::option_button(("context-menu-item", index), label, active).on_click(cx.listener(
            move |this, _, _, cx| {
                let value = field(&mut this.windows_context_menu);
                *value = !*value;
                this.save_settings(cx);
                cx.notify();
            },
        ))
    }

    fn normal_workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(BG)
            .child(
                div()
                    .h(px(58.))
                    .flex_none()
                    .px_5()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .border_b_1()
                    .border_color(BORDER)
                    .child(
                        Self::option_button("normal-extract", "解压缩", self.page == Page::Extract)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.page = Page::Extract;
                                cx.notify();
                            })),
                    )
                    .child(
                        Self::option_button(
                            "normal-compress",
                            "压缩文件",
                            self.page == Page::Compress,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.page = Page::Compress;
                            cx.notify();
                        })),
                    ),
            )
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .child(if self.page == Page::Compress {
                        self.compress_page(cx).into_any_element()
                    } else {
                        self.extract_page(cx).into_any_element()
                    }),
            )
    }

    fn terminal_workspace(&self) -> impl IntoElement {
        div().size_full().p_6().child(
            Self::panel()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(MUTED)
                .child("终端模式尚未启用"),
        )
    }

    fn status_bar(&self) -> impl IntoElement {
        let metrics = vec![
            (
                "系统 CPU",
                format!("{}%", self.resource_stats.system_cpu),
                self.resource_history.system_cpu.clone(),
            ),
            (
                "GeekZip",
                format!("{}%", self.resource_stats.process_cpu),
                self.resource_history.process_cpu.clone(),
            ),
            (
                "GPU",
                self.resource_stats
                    .gpu
                    .map(|usage| format!("{usage}%"))
                    .unwrap_or_else(|| "N/A".into()),
                self.resource_history.gpu.clone(),
            ),
            (
                "内存",
                format!("{} MB", self.resource_stats.memory_used_mb),
                self.resource_history.memory.clone(),
            ),
            (
                "进程内存",
                format!("{} MB", self.resource_stats.process_memory_mb),
                self.resource_history.process_memory.clone(),
            ),
            (
                "线程",
                self.resource_stats.threads.to_string(),
                self.resource_history.threads.clone(),
            ),
        ];
        div()
            .h(px(55.))
            .flex_none()
            .flex()
            .border_t_1()
            .border_color(BORDER)
            .bg(BG)
            .children(
                metrics
                    .into_iter()
                    .enumerate()
                    .map(|(_index, (label, value, history))| {
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap_2()
                            .border_r_1()
                            .border_color(BORDER)
                            .text_xs()
                            .child(div().text_color(MUTED).child(label))
                            .child(div().text_color(GREEN).child(value))
                            .child(Self::dynamic_sparkline(history))
                    }),
            )
            .child(
                div()
                    .w(px(250.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .text_color(GREEN)
                    .gap_2()
                    .child(
                        div()
                            .size(px(if self.led_pulse { 9. } else { 6. }))
                            .rounded_full()
                            .bg(GREEN.opacity(if self.led_pulse { 1.0 } else { 0.45 })),
                    )
                    .child("ALL SYSTEMS NOMINAL"),
            )
    }

    fn capsule_window(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .bg(BG)
            .font_family("JetBrains Mono")
            .child(
                div()
                    .size_full()
                    .rounded(px(32.))
                    .border_1()
                    .border_color(GREEN.opacity(0.85))
                    .bg(PANEL)
                    .relative()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .gap_4()
                    .px_5()
                    .child(Self::dot_grid())
                    .child(
                        div()
                            .size(px(if self.led_pulse { 13. } else { 10. }))
                            .rounded_full()
                            .bg(GREEN.opacity(if self.led_pulse { 1.0 } else { 0.62 })),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_sm()
                                    .text_color(TEXT)
                                    .child(self.operation.clone())
                                    .child(format!("{:.0}%", self.operation_progress)),
                            )
                            .child(Self::segmented_progress(self.operation_progress))
                            .child(div().text_xs().text_color(MUTED).child(
                                if self.operation_total_bytes > 0 {
                                    format!(
                                        "{} · {}/s",
                                        Self::format_bytes(self.operation_bytes_done),
                                        Self::format_bytes(self.operation_speed_bps)
                                    )
                                } else {
                                    self.result.clone()
                                },
                            )),
                    )
                    .child(
                        Self::option_button("capsule-cancel", "取消", false).on_click(cx.listener(
                            |this, _, _, cx| {
                                if let Some(control) = this.operation_control.as_ref() {
                                    control.cancel();
                                    this.operation = "正在取消".into();
                                    this.result = "已请求取消".into();
                                    cx.notify();
                                }
                            },
                        )),
                    ),
            )
    }
}

impl Render for GeekZipApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.capsule_mode {
            return self.capsule_window(cx).into_any_element();
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(BG)
            .font_family("JetBrains Mono")
            .child(self.header(cx))
            .child(match self.mode {
                AppMode::Normal => self.normal_workspace(cx).into_any_element(),
                AppMode::Terminal => self.terminal_workspace().into_any_element(),
                AppMode::Pro => div()
                    .min_h_0()
                    .flex_1()
                    .flex()
                    .child(self.sidebar(cx))
                    .child(div().min_w_0().flex_1().child(match self.page {
                        Page::Dashboard => self.dashboard(cx).into_any_element(),
                        Page::Extract => self.extract_page(cx).into_any_element(),
                        Page::Compress => self.compress_page(cx).into_any_element(),
                        Page::AutoExtract => self.auto_extract_page(cx).into_any_element(),
                        Page::Recursive => self.recursive_page(cx).into_any_element(),
                        Page::Batch => self.batch_page(cx).into_any_element(),
                        Page::Passwords => self.passwords_page(cx).into_any_element(),
                        Page::Settings => self.settings_page(cx).into_any_element(),
                    }))
                    .into_any_element(),
            })
            .when(self.mode == AppMode::Pro, |view| {
                view.child(self.status_bar())
            })
            .into_any_element()
    }
}

fn parse_context_launch() -> ContextLaunch {
    let mut launch = ContextLaunch::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--context-extract" => {
                launch.extract_path = args.next().map(PathBuf::from);
            }
            "--context-action" => {
                launch.action = args.next();
            }
            "--context-output" => {
                launch.output = args.next().map(PathBuf::from);
            }
            _ => {}
        }
    }
    launch
}

fn main() {
    let launch = parse_context_launch();
    Application::new().run(move |cx: &mut App| {
        let _ = cx.text_system().add_fonts(vec![
            Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono.ttf")),
            Cow::Borrowed(include_bytes!(
                "../assets/fonts/MajorMonoDisplay-Regular.ttf"
            )),
        ]);
        gpui_component::init(cx);
        let launch_for_window = launch.clone();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(
                if launch.extract_path.is_some() {
                    size(px(520.), px(150.))
                } else {
                    size(px(1536.), px(1024.))
                },
                cx,
            )),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(options, |window, cx| {
                window.activate_window();
                window.set_window_title("GeekZip");
                Theme::change(ThemeMode::Dark, Some(window), cx);
                let app = cx.new(|cx| GeekZipApp::new(window, cx, launch_for_window.clone()));
                app.update(cx, |app, cx| app.start_resource_monitor(cx));
                if app.read(cx).capsule_mode {
                    app.update(cx, |app, cx| app.start_extract(cx));
                }
                cx.new(|cx| Root::new(app, window, cx))
            })?;
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
