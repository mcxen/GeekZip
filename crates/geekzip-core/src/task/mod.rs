use serde::{Deserialize, Serialize};

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