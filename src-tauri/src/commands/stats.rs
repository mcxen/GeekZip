use serde::Serialize;
use std::sync::Mutex;
use std::time::Instant;
use sysinfo::System;

const HISTORY_LEN: usize = 60;

struct StatsState {
    cpu_history: Vec<u8>,
    mem_history: Vec<u8>,
    sys: System,
    last_refresh: Instant,
}

static STATS: once_cell::sync::Lazy<Mutex<StatsState>> = once_cell::sync::Lazy::new(|| {
    Mutex::new(StatsState {
        cpu_history: Vec::with_capacity(HISTORY_LEN),
        mem_history: Vec::with_capacity(HISTORY_LEN),
        sys: System::new_all(),
        last_refresh: Instant::now(),
    })
});

#[tauri::command]
pub async fn get_system_stats() -> Result<SystemStats, String> {
    let mut state = STATS.lock().map_err(|e| e.to_string())?;

    if state.last_refresh.elapsed() > std::time::Duration::from_millis(800) {
        state.sys.refresh_all();
        state.last_refresh = Instant::now();
    }

    let cpu_usage = state.sys.global_cpu_usage();
    let total_mem = state.sys.total_memory();
    let used_mem = state.sys.used_memory();
    let mem_percent = if total_mem > 0 {
        (used_mem as f64 / total_mem as f64 * 100.0) as u8
    } else {
        0
    };

    state.cpu_history.push(cpu_usage as u8);
    state.mem_history.push(mem_percent);
    if state.cpu_history.len() > HISTORY_LEN {
        state.cpu_history.remove(0);
    }
    if state.mem_history.len() > HISTORY_LEN {
        state.mem_history.remove(0);
    }

    let thread_count = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);

    Ok(SystemStats {
        cpu_percent: cpu_usage as u8,
        mem_total_mb: total_mem / 1024 / 1024,
        mem_used_mb: used_mem / 1024 / 1024,
        mem_percent,
        threads: thread_count,
        cpu_history: state.cpu_history.clone(),
        mem_history: state.mem_history.clone(),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStats {
    pub cpu_percent: u8,
    pub mem_total_mb: u64,
    pub mem_used_mb: u64,
    pub mem_percent: u8,
    pub threads: u32,
    pub cpu_history: Vec<u8>,
    pub mem_history: Vec<u8>,
}