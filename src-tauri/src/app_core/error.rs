use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("خطأ في الملفات: {0}")]
    Io(#[from] std::io::Error),
    #[error("خطأ في قاعدة البيانات: {0}")]
    Database(#[from] sqlx::Error),
    #[error("فشل تحديث بنية قاعدة البيانات: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("{0}")]
    Validation(String),
    #[error("خطأ داخلي: {0}")]
    Internal(String),
}

/// الأخطاء ترجع للواجهة كنص عربي.
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
