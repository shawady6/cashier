use std::path::PathBuf;

use sqlx::SqlitePool;
use tracing_appender::non_blocking::WorkerGuard;

pub struct AppState {
    pub db: SqlitePool,
    pub data_dir: PathBuf,
    #[allow(dead_code)]
    pub log_guard: WorkerGuard,
}
