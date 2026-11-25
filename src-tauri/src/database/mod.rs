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
        assert_eq!(n, 6);
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

    /// الـ migration 0006 يُطبَّق على قاعدة فيها بيانات فعلية ولا يمس شيئًا منها.
    #[tokio::test]
    async fn migration_0006_applies_on_existing_data() {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(SqliteConnectOptions::from_str("sqlite::memory:").unwrap().foreign_keys(true))
            .await
            .unwrap();
        for sql in [
            include_str!("../../migrations/0001_init.sql"),
            include_str!("../../migrations/0002_catalog.sql"),
            include_str!("../../migrations/0003_sales.sql"),
            include_str!("../../migrations/0004_inventory_purchasing.sql"),
            include_str!("../../migrations/0005_finance.sql"),
        ] {
            sqlx::raw_sql(sql).execute(&pool).await.unwrap();
        }
        let carton = "b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0006";
        let piece = "b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0001";
        sqlx::query("INSERT INTO products (id, name_ar, base_unit_id, sale_price) VALUES ('p1', 'قديم', ?1, 1500)").bind(piece).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO product_units (id, product_id, unit_id, factor_micro, is_purchase_unit, is_sale_unit) VALUES ('u1','p1',?1,12000000,1,1)").bind(carton).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO product_barcodes (id, product_id, barcode) VALUES ('b1','p1','123')").execute(&pool).await.unwrap();

        sqlx::raw_sql(include_str!("../../migrations/0006_big_units.sql")).execute(&pool).await.unwrap();

        let (price, factor): (Option<i64>, i64) = sqlx::query_as("SELECT sale_price_minor, factor_micro FROM product_units WHERE id='u1'").fetch_one(&pool).await.unwrap();
        assert_eq!((price, factor), (None, 12_000_000));
        let unit: Option<String> = sqlx::query_scalar("SELECT unit_id FROM product_barcodes WHERE id='b1'").fetch_one(&pool).await.unwrap();
        assert!(unit.is_none());
        let sale: i64 = sqlx::query_scalar("SELECT sale_price FROM products WHERE id='p1'").fetch_one(&pool).await.unwrap();
        assert_eq!(sale, 1500);
        // العمود الجديد في بنود الشراء موجود
        sqlx::query("SELECT new_sale_price_minor FROM purchase_items").fetch_all(&pool).await.unwrap();
    }
}
