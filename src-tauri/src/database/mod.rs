//! طبقة قاعدة البيانات: SQLite بوضع WAL ومفاتيح أجنبية مفعّلة وتحديثات بنية (migrations).

use std::path::Path;
use std::time::Duration;

use sqlx::migrate::Migrator;
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

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// عدد نسخ ما قبل التحديث المحفوظة؛ الأقدم تُحذف.
const KEEP_PRE_MIGRATION_BACKUPS: usize = 5;

pub fn migration_count() -> i64 {
    MIGRATOR.iter().count() as i64
}

pub async fn init(path: &Path) -> Result<SqlitePool, AppError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options(path))
        .await?;
    backup_before_migrate(&pool, path).await?;
    migrate(&pool).await?;
    Ok(pool)
}

/// قبل تطبيق أي تحديث بنية على قاعدة فيها بيانات، تُحفظ نسخة كاملة سليمة بجوارها
/// (VACUUM INTO وليس نسخ ملف شغّال). فشل النسخة يوقف التحديث ولا تُمس البيانات.
pub async fn backup_before_migrate(pool: &SqlitePool, db_path: &Path) -> Result<Option<std::path::PathBuf>, AppError> {
    let has_table: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'")
        .fetch_one(pool)
        .await?;
    if has_table == 0 {
        return Ok(None); // قاعدة جديدة: لا بيانات تُحمى
    }
    let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1")
        .fetch_one(pool)
        .await?;
    let total = MIGRATOR.iter().count() as i64;
    if applied >= total {
        return Ok(None); // محدّثة بالفعل
    }
    let dir = db_path
        .parent()
        .ok_or_else(|| AppError::Internal("مسار قاعدة البيانات غير صالح".into()))?
        .join("backups");
    std::fs::create_dir_all(&dir)?;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let target = dir.join(format!("pre-migration-{secs:012}-v{applied}.db"));
    if target.exists() {
        std::fs::remove_file(&target)?;
    }
    sqlx::query("VACUUM INTO ?1")
        .bind(target.to_string_lossy().to_string())
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("تعذر حفظ نسخة احتياطية قبل التحديث، لم يتم تغيير أي بيانات: {e}")))?;

    // تحقق أن النسخة سليمة قبل السماح بالتحديث.
    let check = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(&target).read_only(true))
        .await?;
    let verdict: String = sqlx::query_scalar("PRAGMA integrity_check").fetch_one(&check).await?;
    check.close().await;
    if verdict != "ok" {
        let _ = std::fs::remove_file(&target);
        return Err(AppError::Internal("النسخة الاحتياطية قبل التحديث غير سليمة، تم إيقاف التحديث".into()));
    }
    prune_old_backups(&dir);
    Ok(Some(target))
}

fn prune_old_backups(dir: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<_> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("pre-migration-"))
        .map(|e| e.path())
        .collect();
    files.sort();
    while files.len() > KEEP_PRE_MIGRATION_BACKUPS {
        let _ = std::fs::remove_file(files.remove(0));
    }
}

/// تطبيق تحديثات البنية. لا تحذف بيانات ولا تعيد ضبط القاعدة.
pub async fn migrate(pool: &SqlitePool) -> Result<(), AppError> {
    MIGRATOR.run(pool).await?;
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
        assert_eq!(n, 7);
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

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("pos-test-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// قاعدة جديدة: لا نسخة احتياطية. ثم قاعدة قديمة فيها بيانات: نسخة سليمة بنفس البيانات قبل التحديث.
    #[tokio::test]
    async fn pre_migration_backup_only_when_pending_and_is_valid() {
        use sqlx::sqlite::SqlitePoolOptions;
        let dir = temp_dir("bk");
        let db = dir.join("pos.db");

        // أول تشغيل: بدون نسخة
        let pool = super::init(&db).await.unwrap();
        assert!(!dir.join("backups").exists());
        pool.close().await;

        // تشغيل ثانٍ والقاعدة محدّثة: بدون نسخة
        let pool = super::init(&db).await.unwrap();
        assert!(!dir.join("backups").exists());
        // نحاكي قاعدة قديمة: نحذف سجل آخر migration ونضيف بيانات
        sqlx::query("INSERT INTO products (id, name_ar, base_unit_id, sale_price) VALUES ('p1','قديم','b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0001',1500)").execute(&pool).await.unwrap();
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = (SELECT MAX(version) FROM _sqlx_migrations)").execute(&pool).await.unwrap();
        let made = super::backup_before_migrate(&pool, &db).await.unwrap().expect("نسخة مطلوبة");
        pool.close().await;

        let bk = SqlitePoolOptions::new().max_connections(1)
            .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&made).read_only(true)).await.unwrap();
        let price: i64 = sqlx::query_scalar("SELECT sale_price FROM products WHERE id='p1'").fetch_one(&bk).await.unwrap();
        assert_eq!(price, 1500);
        bk.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn old_backups_are_pruned_to_five() {
        let dir = temp_dir("prune");
        for i in 0..8 {
            std::fs::write(dir.join(format!("pre-migration-{i:012}-v1.db")), b"x").unwrap();
        }
        super::prune_old_backups(&dir);
        let left = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(left, 5);
        assert!(!dir.join("pre-migration-000000000000-v1.db").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
