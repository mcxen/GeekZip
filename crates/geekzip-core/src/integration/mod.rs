use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsContextMenuOptions {
    pub smart_extract: bool,
    pub extract_here: bool,
    pub extract_to_folder: bool,
    pub extract_delete: bool,
    pub open_app: bool,
}

impl Default for WindowsContextMenuOptions {
    fn default() -> Self {
        Self {
            smart_extract: true,
            extract_here: true,
            extract_to_folder: true,
            extract_delete: true,
            open_app: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextMenuScope {
    User,
    Machine,
}

impl ContextMenuScope {
    pub fn registry_root(self) -> &'static str {
        match self {
            Self::User => r"HKCU\Software\Classes\*\shell\GeekZip",
            Self::Machine => r"HKLM\Software\Classes\*\shell\GeekZip",
        }
    }
}

pub fn default_cli_path() -> PathBuf {
    let current = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("geekzip.exe"));
    if current
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.eq_ignore_ascii_case("geekzip"))
    {
        return current;
    }
    current
        .parent()
        .map(|parent| parent.join("geekzip.exe"))
        .unwrap_or(current)
}

#[cfg(target_os = "windows")]
pub fn install_windows_context_menu(
    cli_path: &Path,
    app_path: Option<&Path>,
    opts: &WindowsContextMenuOptions,
    scope: ContextMenuScope,
) -> Result<()> {
    uninstall_windows_context_menu(scope)?;

    let root = scope.registry_root();
    reg_add(root, Some("MUIVerb"), "GeekZip")?;
    reg_add(
        root,
        Some("Icon"),
        &path_string(app_path.unwrap_or(cli_path)),
    )?;
    reg_add(root, Some("SubCommands"), "")?;

    if opts.smart_extract {
        add_menu_item(
            scope,
            "smart",
            "智能解压",
            &context_extract_command(cli_path, app_path, "smart", None),
        )?;
    }
    if opts.extract_here {
        add_menu_item(
            scope,
            "here",
            "解压到当前目录",
            &context_extract_command(cli_path, app_path, "here", Some(r#""%~dpI""#)),
        )?;
    }
    if opts.extract_to_folder {
        add_menu_item(
            scope,
            "folder",
            "解压到文件名文件夹",
            &context_extract_command(cli_path, app_path, "folder", None),
        )?;
    }
    if opts.extract_delete {
        add_menu_item(
            scope,
            "delete",
            "解压并删除源文件",
            &context_extract_command(cli_path, app_path, "delete", None),
        )?;
    }
    if opts.open_app {
        let app = app_path.unwrap_or(cli_path);
        add_menu_item(
            scope,
            "settings",
            "打开 GeekZip",
            &format!("\"{}\"", path_string(app)),
        )?;
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn install_windows_context_menu(
    _cli_path: &Path,
    _app_path: Option<&Path>,
    _opts: &WindowsContextMenuOptions,
    _scope: ContextMenuScope,
) -> Result<()> {
    bail!("Windows context menu installation is only available on Windows")
}

#[cfg(target_os = "windows")]
pub fn uninstall_windows_context_menu(scope: ContextMenuScope) -> Result<()> {
    let status = std::process::Command::new("reg")
        .args(["delete", scope.registry_root(), "/f"])
        .status()
        .context("failed to run reg delete")?;
    if !status.success() {
        return Ok(());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn uninstall_windows_context_menu(_scope: ContextMenuScope) -> Result<()> {
    bail!("Windows context menu uninstallation is only available on Windows")
}

#[cfg(target_os = "windows")]
fn add_menu_item(scope: ContextMenuScope, key: &str, label: &str, command: &str) -> Result<()> {
    let item = format!(r"{}\shell\{key}", scope.registry_root());
    let command_key = format!(r"{item}\command");
    reg_add(&item, Some("MUIVerb"), label)?;
    reg_add(&command_key, None, command)
}

#[cfg(target_os = "windows")]
fn reg_add(key: &str, value: Option<&str>, data: &str) -> Result<()> {
    let mut command = std::process::Command::new("reg");
    command.args(["add", key]);
    if let Some(value) = value {
        command.args(["/v", value]);
    } else {
        command.arg("/ve");
    }
    let status = command
        .args(["/d", data, "/f"])
        .status()
        .with_context(|| format!("failed to run reg add for {key}"))?;
    if !status.success() {
        bail!("reg add failed for {key}");
    }
    Ok(())
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(target_os = "windows")]
fn context_extract_command(
    cli_path: &Path,
    app_path: Option<&Path>,
    action: &str,
    output: Option<&str>,
) -> String {
    if let Some(app_path) = app_path {
        let mut command = format!(
            "\"{}\" --context-extract \"%1\" --context-action {action}",
            path_string(app_path)
        );
        if let Some(output) = output {
            return format!("cmd /c for %I in (\"%1\") do {command} --context-output {output}");
        }
        return command;
    }

    match action {
        "here" => format!(
            "cmd /c for %I in (\"%1\") do \"{}\" extract \"%1\" --output \"%~dpI\" --flatten-single-root",
            path_string(cli_path)
        ),
        "folder" => format!(
            "\"{}\" extract \"%1\" --subfolder --flatten-single-root",
            path_string(cli_path)
        ),
        "delete" => format!(
            "\"{}\" extract \"%1\" --delete --flatten-single-root",
            path_string(cli_path)
        ),
        _ => format!(
            "\"{}\" extract \"%1\" --recursive --flatten-single-root",
            path_string(cli_path)
        ),
    }
}
