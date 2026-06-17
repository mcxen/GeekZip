pub mod compress;
pub mod extract;
pub mod filename;
pub mod format;
pub mod integration;
pub mod password;
pub mod recursive;
pub mod safety;
pub mod task;
pub mod volume;

pub use compress::{CompressEngine, CompressFormat, CompressOptions};
pub use extract::{ExtractEngine, ExtractOptions, ExtractResult, OverwritePolicy};
pub use filename::FilenameCleaner;
pub use format::ArchiveFormat;
pub use integration::{
    default_cli_path, install_windows_context_menu, uninstall_windows_context_menu,
    ContextMenuScope, WindowsContextMenuOptions,
};
pub use password::PasswordEngine;
pub use recursive::{RecursiveExtractor, RecursiveResult};
pub use safety::SafetyGuard;
pub use task::{
    OperationControl, ProgressCallback, ProgressReader, ProgressUpdate, Task, TaskStatus,
};
pub use volume::VolumeDetector;
