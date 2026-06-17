use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Analyzing,
    PasswordRequired,
    Extracting,
    Verifying,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub source: String,
    pub status: TaskStatus,
    pub progress: f64,
    pub speed_bytes_per_sec: u64,
    pub eta_seconds: u64,
    pub error_message: Option<String>,
    pub password_used: Option<String>,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct OperationControl {
    cancelled: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

impl Default for OperationControl {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationControl {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.paused.store(false, Ordering::SeqCst);
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn wait_if_paused(&self) -> io::Result<()> {
        while self.is_paused() {
            if self.is_cancelled() {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "operation cancelled",
                ));
            }
            std::thread::sleep(Duration::from_millis(80));
        }
        if self.is_cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "operation cancelled",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub phase: String,
    pub current_path: Option<String>,
    pub bytes_done: u64,
    pub total_bytes: u64,
    pub files_done: usize,
    pub total_files: usize,
}

impl ProgressUpdate {
    pub fn percent(&self) -> f64 {
        if self.total_bytes == 0 {
            return if self.total_files > 0 {
                (self.files_done as f64 / self.total_files as f64) * 100.0
            } else {
                0.0
            };
        }
        (self.bytes_done as f64 / self.total_bytes as f64 * 100.0).clamp(0.0, 100.0)
    }
}

pub type ProgressCallback<'a> = &'a dyn Fn(ProgressUpdate);

pub struct ProgressReader<R, F> {
    inner: R,
    control: OperationControl,
    on_read: F,
}

impl<R, F> ProgressReader<R, F> {
    pub fn new(inner: R, control: OperationControl, on_read: F) -> Self {
        Self {
            inner,
            control,
            on_read,
        }
    }
}

impl<R: Read, F: Fn(u64)> Read for ProgressReader<R, F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.control.wait_if_paused()?;
        let read = self.inner.read(buf)?;
        if read > 0 {
            (self.on_read)(read as u64);
        }
        Ok(read)
    }
}

impl Task {
    pub fn new(source: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.to_string(),
            status: TaskStatus::Queued,
            progress: 0.0,
            speed_bytes_per_sec: 0,
            eta_seconds: 0,
            error_message: None,
            password_used: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            started_at: None,
            completed_at: None,
        }
    }
}
