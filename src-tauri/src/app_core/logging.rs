use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;

use super::error::AppError;

/// تهيئة السجلات المنظمة في ملف يومي داخل مجلد البيانات.
/// يجب الاحتفاظ بالـ guard طوال عمر التطبيق وإلا تضيع آخر السجلات.
pub fn init(dir: &Path) -> Result<WorkerGuard, AppError> {
    std::fs::create_dir_all(dir)?;
    let appender = tracing_appender::rolling::daily(dir, "pos.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);

    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(guard)
}
