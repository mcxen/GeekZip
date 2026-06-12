use clap::{Parser, Subcommand};
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
    },
    Compress {
        #[arg(help = "Files/directories to compress")]
        input: Vec<PathBuf>,
        #[arg(help = "Output archive path")]
        output: PathBuf,
        #[arg(short, long, default_value = "zip", help = "Format: zip, tar.gz, tar.bz2, tar.xz, tar")]
        format: String,
        #[arg(short, long, help = "Password for encryption")]
        password: Option<String>,
    },
    Info {
        #[arg(help = "Archive file to analyze")]
        input: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract {
            input, output, password, delete, subfolder, recursive, max_depth,
        } => {
            let opts = geekzip_core::ExtractOptions {
                target_dir: output.map(|p| p.to_string_lossy().to_string()),
                create_subfolder: subfolder,
                overwrite: geekzip_core::OverwritePolicy::Rename,
                password,
                delete_after: delete,
                open_after: false,
                verify: false,
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
        Commands::Compress { input, output, format, password } => {
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
                level: 6,
                password,
                create_subfolder: false,
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
    }

    Ok(())
}