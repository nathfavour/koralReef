use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Instant;
use crate::config::AppMode;

pub struct AppState {
    pub total_reclaimed_lamports: u64,
    pub total_accounts_closed: u64,
    pub start_time: Instant,
    pub force_run: bool,
    pub mode: AppMode,
    pub last_scan_time: Option<Instant>,
    pub last_reclaim_summary: Option<String>,
}

impl AppState {
    pub fn new(mode: AppMode) -> Self {
        Self {
            total_reclaimed_lamports: 0,
            total_accounts_closed: 0,
            start_time: Instant::now(),
            force_run: false,
            mode,
            last_scan_time: None,
            last_reclaim_summary: None,
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;
