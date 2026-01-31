use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Instant;

pub struct AppState {
    pub total_reclaimed_lamports: u64,
    pub total_accounts_closed: u64,
    pub start_time: Instant,
    pub force_run: bool,
    pub last_scan_time: Option<Instant>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            total_reclaimed_lamports: 0,
            total_accounts_closed: 0,
            start_time: Instant::now(),
            force_run: false,
            last_scan_time: None,
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;
