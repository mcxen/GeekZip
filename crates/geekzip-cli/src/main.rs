use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "geekzip", version, about = "GeekZip - Smart Archive Tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Extract {
        #[arg(help = "Archive file to extract")]
        input: PathBuf,
        #[arg(short, long, help = "Target directory")]
        output: Option<PathBuf>,
        #[arg(long, help = "Default extract directory when --output is not set")]
        default_output: Option<PathBuf>,
        #[arg(short, long, help = "Password")]
        password: Option<String>,
        #[arg(long, help = "Delete archive after extraction")]
        delete: bool,
        #[arg(long, help = "Create subfolder named after archive")]
        subfolder: bool,
        #[arg(short, long, help = "Recursive extraction")]
        recursive: bool,
        #[arg(long, default_value = "10", help = "Max recursion depth")]
        max_depth: u32,
        #[arg(long, help = "Remove this prefix from extracted file and folder names")]
        strip_prefix: Vec<String>,
        #[arg(
            long,
            help = "Flatten a redundant single top-level folder after extraction"
        )]
        flatten_single_root: bool,
    },
    Compress {
        #[arg(help = "Files/directories to compress")]
        input: Vec<PathBuf>,
        #[arg(help = "Output archive path")]
        output: PathBuf,
        #[arg(
            short,
            long,
            default_value = "zip",
            help = "Format: zip, tar.gz, tar.bz2, tar.xz, tar"
        )]
        format: String,
        #[arg(short, long, help = "Password for encryption")]
        password: Option<String>,
        #[arg(short, long, default_value = "6", help = "Compression level: 1-9")]
        level: u32,
        #[arg(long, help = "Split archive into volumes of this size in MB")]
        volume_size_mb: Option<u64>,
        #[arg(long, help = "Append suffix to volume parts, e.g. 中文混淆")]
        obfuscate_suffix: Option<String>,
    },
    Info {
        #[arg(help = "Archive file to analyze")]
        input: PathBuf,
    },
    InstallContextMenu {
        #[arg(long, value_enum, default_value = "user")]
        scope: MenuScope,
        #[arg(long, help = "Path to geekzip.exe")]
        cli_path: Option<PathBuf>,
        #[arg(long, help = "Path to GeekZip.exe")]
        app_path: Option<PathBuf>,
        #[arg(long)]
        no_smart_extract: bool,
        #[arg(long)]
        no_extract_here: bool,
        #[arg(long)]
        no_extract_to_folder: bool,
        #[arg(long)]
        no_extract_delete: bool,
        #[arg(long)]
        no_open_app: bool,
    },
    UninstallContextMenu {
        #[arg(long, value_enum, default_value = "user")]
        scope: MenuScope,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum MenuScope {
    User,
    Machine,
}

impl From<MenuScope> for geekzip_core::ContextMenuScope {
    fn from(value: MenuScope) -> Self {
        match value {
            MenuScope::User => Self::User,
            MenuScope::Machine => Self::Machine,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract {
            input,
            output,
            default_output,
            password,
            delete,
            subfolder,
            recursive,
            max_depth,
            strip_prefix,
            flatten_single_root,
        } => {
            let opts = geekzip_core::ExtractOptions {
                target_dir: output.map(|p| p.to_string_lossy().to_string()),
                default_target_dir: default_output.map(|p| p.to_string_lossy().to_string()),
                create_subfolder: subfolder,
                overwrite: geekzip_core::OverwritePolicy::Rename,
                password,
                password_candidates: Vec::new(),
                delete_after: delete,
                open_after: false,
                verify: false,
                rename_prefixes: strip_prefix,
                flatten_single_root,
            };

            if recursive {
                let extractor = geekzip_core::RecursiveExtractor::new(max_depth);
                let result = extractor.extract_recursive(&input, &opts)?;
                println!("Recursive extraction complete:");
                println!("  Layers: {}", result.total_layers);
                println!("  Files:  {}", result.total_files);
                for r in &result.results {
                    println!("  {} -> {}", r.source, r.target_dir);
                }
            } else {
                let result = geekzip_core::ExtractEngine::extract(&input, &opts)?;
                println!("Extraction complete:");
                println!("  Source: {}", result.source);
                println!("  Target: {}", result.target_dir);
                println!("  Format: {}", result.format);
                println!("  Files:  {}", result.files.len());
                println!("  Time:   {}ms", result.elapsed_ms);
            }
        }
        Commands::Compress {
            input,
            output,
            format,
            password,
            level,
            volume_size_mb,
            obfuscate_suffix,
        } => {
            let fmt = match format.as_str() {
                "zip" => geekzip_core::CompressFormat::Zip,
                "tar.gz" | "tgz" => geekzip_core::CompressFormat::TarGz,
                "tar.bz2" | "tbz2" => geekzip_core::CompressFormat::TarBz2,
                "tar.xz" | "txz" => geekzip_core::CompressFormat::TarXz,
                "tar" => geekzip_core::CompressFormat::Tar,
                _ => anyhow::bail!("Unknown format: {}", format),
            };
            let opts = geekzip_core::CompressOptions {
                format: fmt,
                level,
                password,
                create_subfolder: false,
                volume_size_mb,
                obfuscate_suffix,
            };
            let path_refs: Vec<&Path> = input.iter().map(|p| p.as_path()).collect();
            geekzip_core::CompressEngine::compress(&path_refs, &output, &opts)?;
            println!("Compression complete: {:?}", output);
        }
        Commands::Info { input } => {
            let info = geekzip_core::format::detect_format(&input);
            println!("File:       {}", input.display());
            println!("Format:     {}", info.format.name());
            println!("Detected:   {:?}", info.detected_by);
            println!("Extension:  {:?}", info.original_extension);
            if let Ok(meta) = std::fs::metadata(&input) {
                println!("Size:       {} bytes", meta.len());
            }
        }
        Commands::InstallContextMenu {
            scope,
            cli_path,
            app_path,
            no_smart_extract,
            no_extract_here,
            no_extract_to_folder,
            no_extract_delete,
            no_open_app,
        } => {
            let cli_path = cli_path.unwrap_or_else(geekzip_core::default_cli_path);
            let options = geekzip_core::WindowsContextMenuOptions {
                smart_extract: !no_smart_extract,
                extract_here: !no_extract_here,
                extract_to_folder: !no_extract_to_folder,
                extract_delete: !no_extract_delete,
                open_app: !no_open_app,
            };
            geekzip_core::install_windows_context_menu(
                &cli_path,
                app_path.as_deref(),
                &options,
                scope.into(),
            )?;
            println!("Windows context menu installed");
        }
        Commands::UninstallContextMenu { scope } => {
            geekzip_core::uninstall_windows_context_menu(scope.into())?;
            println!("Windows context menu removed");
        }
    }

    Ok(())
}
