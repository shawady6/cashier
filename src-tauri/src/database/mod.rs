//! طبقة قاعدة البيانات: SQLite بوضع WAL ومفاتيح أجنبية مفعّلة وتحديثات بنية (migrations).

use std::path::Path;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

use crate::app_core::error::AppError;

pub fn connect_options(path: &Path) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        // FULL: أقصى أمان للبيانات المالية عند انقطاع الكهرباء.
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(5))
}

pub async fn init(path: &Path) -> Result<SqlitePool, AppError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options(path))
        .await?;
    migrate(&pool).await?;
    Ok(pool)
}

/// تطبيق تحديثات البنية. لا تحذف بيانات ولا تعيد ضبط القاعدة.
pub async fn migrate(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

#[cfg(test)]
pub mod test_support {
    use std::str::FromStr;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use sqlx::SqlitePool;

    /// قاعدة بيانات في الذاكرة بنفس الإعدادات المهمة (مفاتيح أجنبية) لاختبارات سريعة.
    pub async fn memory_pool() -> SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .expect("invalid memory url")
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await
            .expect("connect");
        super::migrate(&pool).await.expect("migrate");
        pool
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::memory_pool;

    #[tokio::test]
    async fn foreign_keys_are_enabled() {
        let pool = memory_pool().await;
        let on: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(on, 1);
    }

    #[tokio::test]
    async fn migrations_are_idempotent() {
        let pool = memory_pool().await;
        super::migrate(&pool).await.unwrap();
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn four_business_profiles_are_seeded() {
        let pool = memory_pool().await;
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM business_profiles")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 4);
    }
}
