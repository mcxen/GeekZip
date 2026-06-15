use geekzip_core::{
    ArchiveFormat, CompressEngine, CompressFormat, CompressOptions, ExtractEngine, ExtractOptions,
    RecursiveExtractor, format::detect_format,
};
use gpui::prelude::FluentBuilder;
use gpui::{
    App, AppContext, Application, Bounds, Context, Div, Entity, Hsla, InteractiveElement,
    IntoElement, ParentElement, PathPromptOptions, Render, StatefulInteractiveElement, Styled,
    Task, Window, WindowBounds, WindowOptions, canvas, div, hsla, point, px, size,
};
use gpui_component::{
    Root, Theme, ThemeMode,
    input::{Input, InputState},
    scroll::ScrollableElement,
};
use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use sysinfo::{Pid, ProcessesToUpdate, System};

const BG: Hsla = Hsla {
    h: 0.55,
    s: 0.28,
    l: 0.035,
    a: 1.0,
};
const PANEL: Hsla = Hsla {
    h: 0.48,
    s: 0.20,
    l: 0.055,
    a: 1.0,
};
const BORDER: Hsla = Hsla {
    h: 0.46,
    s: 0.16,
    l: 0.18,
    a: 1.0,
};
const GREEN: Hsla = Hsla {
    h: 0.40,
    s: 0.92,
    l: 0.51,
    a: 1.0,
};
const MUTED: Hsla = Hsla {
    h: 0.48,
    s: 0.08,
    l: 0.58,
    a: 1.0,
};
const TEXT: Hsla = Hsla {
    h: 0.46,
    s: 0.08,
    l: 0.86,
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
        }
    }
}

#[derive(Clone, Default)]
struct ResourceStats {
    system_cpu: u8,
    process_cpu: u8,
    gpu: Option<u8>,
    memory_used_mb: u64,
    process_memory_mb: u64,
    threads: usize,
}

struct GeekZipApp {
    mode: AppMode,
    page: Page,
    archive_path: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    compress_sources: Vec<PathBuf>,
    compress_format: usize,
    compress_level: u32,
    batch_dir: Option<PathBuf>,
    watch_dir: Option<PathBuf>,
    password_input: Entity<InputState>,
    passwords: Vec<String>,
    busy: bool,
    operation: String,
    result: String,
    operation_log: Vec<String>,
    watch_task: Option<Task<()>>,
    resource_stats: ResourceStats,
    resource_system: Arc<Mutex<System>>,
    resource_task: Option<Task<()>>,
}

impl GeekZipApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            mode: AppMode::Pro,
            page: Page::Dashboard,
            archive_path: None,
            target_dir: None,
            compress_sources: Vec::new(),
            compress_format: 0,
            compress_level: 6,
            batch_dir: None,
            watch_dir: None,
            password_input: cx
                .new(|cx| InputState::new(window, cx).placeholder("输入一个解压密码")),
            passwords: Self::load_passwords(),
            busy: false,
            operation: "等待任务".into(),
            result: "选择文件后即可开始".into(),
            operation_log: vec!["[READY] Rust 核心已连接".into()],
            watch_task: None,
            resource_stats: ResourceStats::default(),
            resource_system: Arc::new(Mutex::new(System::new_all())),
            resource_task: None,
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
        #[cfg(not(target_os = "macos"))]
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
        ResourceStats {
            system_cpu: system.global_cpu_usage().round().clamp(0.0, 100.0) as u8,
            process_cpu: process
                .map(|process| process.cpu_usage().round().clamp(0.0, 100.0) as u8)
                .unwrap_or_default(),
            gpu: Self::gpu_usage(),
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

    fn load_passwords() -> Vec<String> {
        Self::password_file()
            .and_then(|path| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
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
            .rounded(px(7.))
    }

    fn label(text: impl Into<gpui::SharedString>) -> Div {
        div().text_xs().text_color(GREEN).child(text.into())
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

    fn dot_sparkline(seed: u8) -> Div {
        let values = [2u8, 3, 2, 4, 3, 7, 4, 3, 2, 3, 2, 2];
        div()
            .w(px(54.))
            .h(px(18.))
            .flex()
            .items_end()
            .gap(px(2.))
            .children(values.into_iter().enumerate().map(move |(index, value)| {
                let height = 3 + ((value + seed + index as u8) % 8);
                div()
                    .w(px(2.5))
                    .h(px(height as f32 * 1.6))
                    .rounded(px(1.))
                    .bg(GREEN.opacity(0.38 + height as f32 * 0.055))
            }))
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
            .rounded(px(5.))
            .border_1()
            .border_color(GREEN.opacity(0.7))
            .bg(GREEN.opacity(0.08))
            .text_sm()
            .text_color(GREEN)
            .hover(|style| style.bg(GREEN.opacity(0.16)))
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
            .rounded(px(4.))
            .border_1()
            .border_color(if active { GREEN } else { BORDER })
            .bg(if active { GREEN.opacity(0.1) } else { BG })
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
        ];

        div()
            .w(px(236.))
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
                    .child(Self::label("核心功能").px_3().py_2())
                    .children(pages.into_iter().enumerate().map(|(index, page)| {
                        let active = self.page == page;
                        div()
                            .id(("nav", index))
                            .cursor_pointer()
                            .h(px(58.))
                            .mt_1()
                            .px_3()
                            .flex()
                            .items_center()
                            .gap_3()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(if active { GREEN.opacity(0.45) } else { BG })
                            .bg(if active { GREEN.opacity(0.08) } else { BG })
                            .text_color(if active { GREEN } else { TEXT })
                            .child(div().w(px(22.)).text_lg().child(page.glyph()))
                            .child(
                                div()
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
                    .child("⚙  设置"),
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
                    .child(div().text_color(hsla(0.0, 0.85, 0.6, 1.0)).child("●"))
                    .child(div().text_color(hsla(0.12, 0.9, 0.6, 1.0)).child("●"))
                    .child(div().text_color(GREEN).child("●"))
                    .child(div().h(px(30.)).w(px(1.)).mx_2().bg(BORDER))
                    .child(div().text_xl().text_color(TEXT).child("GEEKZIP"))
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded(px(3.))
                            .border_1()
                            .border_color(GREEN.opacity(0.65))
                            .text_xs()
                            .text_color(GREEN)
                            .child("PRO"),
                    ),
            )
            .child(
                div().flex_1().flex().justify_center().child(
                    div()
                        .w(px(382.))
                        .h(px(44.))
                        .flex()
                        .rounded(px(7.))
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
                                    .child(label)
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
                    .text_lg()
                    .text_color(TEXT)
                    .child("⌁")
                    .child("⚙")
                    .child("◎"),
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
            .child(div().text_2xl().text_color(GREEN).child("▣"))
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
                                    multiple: false,
                                    prompt: Some("选择压缩文件".into()),
                                });
                                cx.spawn_in(window, async move |this, window| {
                                    let path = prompt.await.ok()?.ok()??.into_iter().next()?;
                                    this.update_in(window, |this, _, cx| {
                                        this.archive_path = Some(path.clone());
                                        this.operation = "已选择压缩文件".into();
                                        this.result = path.to_string_lossy().to_string();
                                        this.operation_log
                                            .push(format!("[SELECT] {}", path.display()));
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
                            .child(
                                Self::panel()
                                    .h(px(110.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_sm()
                                    .text_color(MUTED)
                                    .child("暂无历史记录"),
                            ),
                    ),
            )
            .child(self.inspector())
    }

    fn inspector(&self) -> impl IntoElement {
        let selected = self.archive_path.as_ref();
        let metadata = selected.and_then(|path| fs::metadata(path).ok());
        let format = selected
            .map(|path| detect_format(path).format.name().to_string())
            .unwrap_or_else(|| "未选择".into());
        let rows = selected
            .map(|path| {
                vec![
                    ("路径", path.to_string_lossy().to_string()),
                    (
                        "大小",
                        metadata
                            .as_ref()
                            .map(|value| format!("{} bytes", value.len()))
                            .unwrap_or_else(|| "不可读取".into()),
                    ),
                    ("类型", format),
                ]
            })
            .unwrap_or_default();

        div()
            .w(px(350.))
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
                                    .text_xs()
                                    .child(div().w(px(104.)).text_color(MUTED).child(key))
                                    .child(div().flex_1().text_color(TEXT).child(value))
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
                                "✓ 已读取真实文件信息"
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
                    .child(
                        path.map(|value| value.to_string_lossy().to_string())
                            .unwrap_or_else(|| "尚未选择".into()),
                    ),
            )
    }

    fn operation_panel(&self) -> Div {
        let progress = if self.busy {
            0.62
        } else if self.result.starts_with("完成") {
            1.0
        } else {
            0.0
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
            .child(Self::segmented_progress(progress * 100.0))
            .child(div().text_sm().text_color(MUTED).child(self.result.clone()))
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
                    .child(Self::path_row("压缩文件", self.archive_path.as_ref()))
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
                                            multiple: false,
                                            prompt: Some("选择要解压的压缩文件".into()),
                                        });
                                        cx.spawn_in(window, async move |this, window| {
                                            let path =
                                                prompt.await.ok()?.ok()??.into_iter().next()?;
                                            this.update_in(window, |this, _, cx| {
                                                this.archive_path = Some(path.clone());
                                                this.operation_log
                                                    .push(format!("[SELECT] {}", path.display()));
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
            .child(self.operation_panel())
    }

    fn start_extract(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(path) = self.archive_path.clone() else {
            self.result = "请先选择压缩文件".into();
            cx.notify();
            return;
        };
        let target = self.target_dir.clone();
        let passwords = self.passwords.clone();
        self.busy = true;
        self.operation = "正在解压".into();
        self.result = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        self.operation_log.push("[RUN] 启动 ExtractEngine".into());
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let options = ExtractOptions {
                        target_dir: target.map(|value| value.to_string_lossy().to_string()),
                        password_candidates: passwords,
                        ..Default::default()
                    };
                    ExtractEngine::extract(&path, &options)
                })
                .await;
            _ = cx.update(|cx| {
                _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    match result {
                        Ok(result) => {
                            this.operation = "解压完成".into();
                            this.result =
                                format!("完成 · {} · {} ms", result.format, result.elapsed_ms);
                            this.operation_log
                                .push(format!("[OK] 输出到 {}", result.target_dir));
                            if let Some(password) = result.password_used {
                                if !this.passwords.contains(&password) {
                                    this.passwords.push(password.clone());
                                    this.save_passwords();
                                }
                                this.operation_log
                                    .push(format!("[PASSWORD] 已记住成功密码 {}", password));
                            }
                        }
                        Err(error) => {
                            this.operation = "解压失败".into();
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

    fn compress_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let formats = ["ZIP", "TAR.GZ", "TAR.BZ2", "TAR.XZ", "TAR"];
        let source_summary = match self.compress_sources.as_slice() {
            [] => "尚未选择".into(),
            [path] => path.to_string_lossy().to_string(),
            paths => format!("已选择 {} 个文件 / 文件夹", paths.len()),
        };
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
                    .child(Self::label("压缩等级"))
                    .child(div().flex().gap_2().children((1u32..=9).map(|level| {
                        Self::option_button(
                            ("compress-level", level as usize),
                            level.to_string(),
                            self.compress_level == level,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.compress_level = level;
                            cx.notify();
                        }))
                    })))
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
            .child(self.operation_panel())
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
        let (format, extension) = match format_index {
            1 => (CompressFormat::TarGz, "tar.gz"),
            2 => (CompressFormat::TarBz2, "tar.bz2"),
            3 => (CompressFormat::TarXz, "tar.xz"),
            4 => (CompressFormat::Tar, "tar"),
            _ => (CompressFormat::Zip, "zip"),
        };
        let output = first
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!("GeekZip-output.{extension}"));
        self.busy = true;
        self.operation = "正在压缩".into();
        self.result = output.to_string_lossy().to_string();
        self.operation_log
            .push(format!("[RUN] 输出 {}", output.display()));
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let refs: Vec<_> = paths.iter().map(PathBuf::as_path).collect();
                    let options = CompressOptions {
                        format,
                        level,
                        ..Default::default()
                    };
                    CompressEngine::compress(&refs, &output, &options).map(|_| output)
                })
                .await;
            _ = cx.update(|cx| {
                _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    match result {
                        Ok(output) => {
                            this.operation = "压缩完成".into();
                            this.result = format!("完成 · {}", output.display());
                            this.operation_log.push("[OK] ZIP 已生成".into());
                        }
                        Err(error) => {
                            this.operation = "压缩失败".into();
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
                                        this.archive_path = Some(path);
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
            .child(self.operation_panel())
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
        let passwords = self.passwords.clone();
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
                        password_candidates: passwords,
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
            .child(self.operation_panel())
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
        let passwords = self.passwords.clone();
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
                            password_candidates: passwords.clone(),
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
            .child(self.operation_panel())
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
        let passwords = self.passwords.clone();
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
                        password_candidates: passwords.clone(),
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
            .child(self.operation_panel())
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
            ("系统 CPU", format!("{}%", self.resource_stats.system_cpu)),
            ("GeekZip", format!("{}%", self.resource_stats.process_cpu)),
            (
                "GPU",
                self.resource_stats
                    .gpu
                    .map(|usage| format!("{usage}%"))
                    .unwrap_or_else(|| "N/A".into()),
            ),
            ("内存", format!("{} MB", self.resource_stats.memory_used_mb)),
            (
                "进程内存",
                format!("{} MB", self.resource_stats.process_memory_mb),
            ),
            ("线程", self.resource_stats.threads.to_string()),
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
                    .map(|(index, (label, value))| {
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
                            .child(Self::dot_sparkline(index as u8 * 2))
                    }),
            )
            .child(
                div()
                    .w(px(265.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .text_color(GREEN)
                    .child("●  系统运行正常"),
            )
    }
}

impl Render for GeekZipApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(BG)
            .font_family("Maple Mono NF CN")
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
                    }))
                    .into_any_element(),
            })
            .when(self.mode == AppMode::Pro, |view| {
                view.child(self.status_bar())
            })
    }
}

fn main() {
    Application::new().run(move |cx: &mut App| {
        gpui_component::init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1536.), px(1024.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(options, |window, cx| {
                window.activate_window();
                window.set_window_title("GeekZip");
                Theme::change(ThemeMode::Dark, Some(window), cx);
                let app = cx.new(|cx| GeekZipApp::new(window, cx));
                app.update(cx, |app, cx| app.start_resource_monitor(cx));
                cx.new(|cx| Root::new(app, window, cx))
            })?;
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
