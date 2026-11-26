use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::*;
use crate::modules::settings;

struct Dir(PathBuf);
impl Dir {
    fn new() -> Dir {
        let p = std::env::temp_dir().join(format!("pos-backup-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        Dir(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn open(dir: &Path) -> SqlitePool {
    database::init(&dir.join(DB_FILE)).await.unwrap()
}

async fn store_name(pool: &SqlitePool) -> String {
    settings::get_all(pool).await.unwrap().get("store_name").cloned().unwrap_or_default()
}

#[tokio::test]
async fn manual_backup_is_valid_and_listed() {
    let d = Dir::new();
    let pool = open(d.path()).await;
    settings::set(&pool, "store_name", "متجري").await.unwrap();
    let r = create_backup(&pool, d.path(), "manual", "").await.unwrap();
    assert!(r.backup.file_name.starts_with("manual-"));
    assert!(r.backup.size_bytes > 0);
    assert!(!r.external_copied && r.external_error.is_none());
    let check = check_backup(d.path(), "", Location::Local, &r.backup.file_name).await.unwrap();
    assert!(check.ok, "{}", check.message);
    assert_eq!(check.migrations, database::migration_count());
    let list = list_backups(d.path(), "");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].kind, "manual");
}

#[tokio::test]
async fn backup_taken_while_wal_has_unflushed_data_contains_it() {
    let d = Dir::new();
    let pool = open(d.path()).await;
    settings::set(&pool, "store_name", "في الـWAL").await.unwrap();
    let r = create_backup(&pool, d.path(), "manual", "").await.unwrap();
    let copy = backups_dir(d.path()).join(&r.backup.file_name);
    let p2 = open_readonly(&copy).await.unwrap();
    assert_eq!(store_name(&p2).await, "في الـWAL");
}

#[tokio::test]
async fn external_copy_and_external_failure() {
    let d = Dir::new();
    let ext = Dir::new();
    let pool = open(d.path()).await;
    let r = create_backup(&pool, d.path(), "manual", ext.path().to_str().unwrap()).await.unwrap();
    assert!(r.external_copied);
    assert!(ext.path().join(&r.backup.file_name).is_file());
    assert!(list_backups(d.path(), ext.path().to_str().unwrap()).iter().any(|b| b.location == Location::External));

    let missing = ext.path().join("not-here");
    let r2 = create_backup(&pool, d.path(), "manual", missing.to_str().unwrap()).await.unwrap();
    assert!(!r2.external_copied);
    assert!(r2.external_error.is_some());
    assert!(backups_dir(d.path()).join(&r2.backup.file_name).is_file(), "النسخة المحلية تبقى");

    let rel = create_backup(&pool, d.path(), "manual", "relative/dir").await.unwrap();
    assert!(rel.external_error.is_some());
}

#[tokio::test]
async fn corrupt_and_foreign_files_are_rejected_without_side_effects() {
    let d = Dir::new();
    let pool = open(d.path()).await;
    let dir = backups_dir(d.path());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("manual-20200101-000000.db"), b"this is not a database at all").unwrap();
    let c = check_backup(d.path(), "", Location::Local, "manual-20200101-000000.db").await.unwrap();
    assert!(!c.ok);

    // قاعدة SQLite سليمة لكنها ليست لهذا البرنامج
    let foreign = dir.join("manual-20200102-000000.db");
    let fp = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(&foreign).create_if_missing(true))
        .await
        .unwrap();
    sqlx::query("CREATE TABLE x (a INTEGER)").execute(&fp).await.unwrap();
    fp.close().await;
    let c = check_backup(d.path(), "", Location::Local, "manual-20200102-000000.db").await.unwrap();
    assert!(!c.ok && c.message.contains("ليس قاعدة بيانات هذا البرنامج"));

    for name in ["manual-20200101-000000.db", "manual-20200102-000000.db"] {
        let e = stage_restore(&pool, d.path(), "", Location::Local, name).await;
        assert!(e.is_err());
    }
    assert!(!has_pending_restore(d.path()));
    assert!(!list_backups(d.path(), "").iter().any(|b| b.kind == "pre-restore"), "لا نسخة أمان عند رفض الملف");
}

#[tokio::test]
async fn backup_from_newer_schema_is_rejected() {
    let d = Dir::new();
    let pool = open(d.path()).await;
    let r = create_backup(&pool, d.path(), "manual", "").await.unwrap();
    let path = backups_dir(d.path()).join(&r.backup.file_name);
    let rw = SqlitePoolOptions::new().max_connections(1).connect_with(SqliteConnectOptions::new().filename(&path)).await.unwrap();
    sqlx::query("INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (9999, 'future', 1, x'00', 0)")
        .execute(&rw)
        .await
        .unwrap();
    rw.close().await;
    let c = check_backup(d.path(), "", Location::Local, &r.backup.file_name).await.unwrap();
    assert!(!c.ok && c.message.contains("أحدث"));
    assert!(stage_restore(&pool, d.path(), "", Location::Local, &r.backup.file_name).await.is_err());
}

#[tokio::test]
async fn restore_replaces_database_and_keeps_safety_backup() {
    let d = Dir::new();
    let pool = open(d.path()).await;
    settings::set(&pool, "store_name", "قبل").await.unwrap();
    let b = create_backup(&pool, d.path(), "manual", "").await.unwrap();
    settings::set(&pool, "store_name", "بعد").await.unwrap();

    let safety = stage_restore(&pool, d.path(), "", Location::Local, &b.backup.file_name).await.unwrap();
    assert!(safety.file_name.starts_with("pre-restore-"));
    assert!(has_pending_restore(d.path()));
    assert_eq!(store_name(&pool).await, "بعد", "القاعدة الحالية لم تُمس قبل إعادة التشغيل");
    pool.close().await;

    assert!(apply_pending_restore(d.path()).await.unwrap());
    assert!(!has_pending_restore(d.path()));
    assert!(!d.path().join("pos.db-wal").exists() && !d.path().join("pos.db-shm").exists());
    let after = open(d.path()).await;
    assert_eq!(store_name(&after).await, "قبل");

    // نسخة الأمان تحتوي الحالة السابقة للاستعادة
    let sp = open_readonly(&backups_dir(d.path()).join(&safety.file_name)).await.unwrap();
    assert_eq!(store_name(&sp).await, "بعد");
}

#[tokio::test]
async fn failed_apply_leaves_current_database_untouched() {
    let d = Dir::new();
    let pool = open(d.path()).await;
    settings::set(&pool, "store_name", "الحالية").await.unwrap();
    pool.close().await;
    std::fs::write(d.path().join(PENDING_FILE), b"garbage garbage garbage").unwrap();
    let e = apply_pending_restore(d.path()).await;
    assert!(e.is_err());
    assert!(!has_pending_restore(d.path()), "الملف المعلّق التالف يُحذف");
    let again = open(d.path()).await;
    assert_eq!(store_name(&again).await, "الحالية");

    // لا يوجد ملف معلّق: لا شيء يحدث
    assert!(!apply_pending_restore(d.path()).await.unwrap());
}

#[tokio::test]
async fn restore_from_external_location_works() {
    let d = Dir::new();
    let ext = Dir::new();
    let e = ext.path().to_str().unwrap();
    let pool = open(d.path()).await;
    settings::set(&pool, "store_name", "الفلاشة").await.unwrap();
    let b = create_backup(&pool, d.path(), "manual", e).await.unwrap();
    delete_backup(d.path(), e, Location::Local, &b.backup.file_name).unwrap();
    settings::set(&pool, "store_name", "تغيير").await.unwrap();
    stage_restore(&pool, d.path(), e, Location::External, &b.backup.file_name).await.unwrap();
    pool.close().await;
    assert!(apply_pending_restore(d.path()).await.unwrap());
    assert_eq!(store_name(&open(d.path()).await).await, "الفلاشة");
}

#[tokio::test]
async fn unsafe_names_are_refused() {
    let d = Dir::new();
    let _pool = open(d.path()).await;
    for name in ["../pos.db", "manual-../x.db", "pos.db", "manual-1/../../pos.db", "manual-1\\x.db", "other-1.db", "manual-1.txt"] {
        assert!(delete_backup(d.path(), "", Location::Local, name).is_err(), "{name}");
        assert!(check_backup(d.path(), "", Location::Local, name).await.is_err(), "{name}");
    }
    assert!(d.path().join(DB_FILE).exists());
}

#[tokio::test]
async fn auto_backup_runs_once_per_day_and_prunes_only_auto() {
    let d = Dir::new();
    let pool = open(d.path()).await;
    let dir = backups_dir(d.path());
    let mut s: HashMap<String, String> = HashMap::new();
    s.insert("backup_keep_count".into(), "3".into());

    assert!(run_auto_if_due(&pool, d.path(), &s).await.is_some());
    assert!(run_auto_if_due(&pool, d.path(), &s).await.is_none(), "مرة واحدة في اليوم");

    // نسخ قديمة مختلقة: 5 تلقائية + 2 يدوية
    for i in 1..=5 {
        std::fs::copy(dir.join(list_backups(d.path(), "")[0].file_name.clone()), dir.join(format!("auto-2020010{i}-000000.db"))).unwrap();
    }
    for i in 1..=2 {
        std::fs::copy(dir.join(list_backups(d.path(), "")[0].file_name.clone()), dir.join(format!("manual-2020010{i}-000000.db"))).unwrap();
    }
    prune(&dir, "auto", keep_count(s.get("backup_keep_count")));
    let list = list_backups(d.path(), "");
    assert_eq!(list.iter().filter(|b| b.kind == "auto").count(), 3);
    assert_eq!(list.iter().filter(|b| b.kind == "manual").count(), 2);
    assert!(list.iter().any(|b| b.kind == "auto" && !b.file_name.contains("2020")), "الأحدث يبقى");

    s.insert("backup_auto_enabled".into(), "0".into());
    let d2 = Dir::new();
    let p2 = open(d2.path()).await;
    assert!(run_auto_if_due(&p2, d2.path(), &s).await.is_none());
}

#[test]
fn keep_count_defaults_and_clamps() {
    assert_eq!(keep_count(None), 14);
    assert_eq!(keep_count(Some(&"30".to_string())), 30);
    assert_eq!(keep_count(Some(&"1".to_string())), 14);
    assert_eq!(keep_count(Some(&"abc".to_string())), 14);
}

#[tokio::test]
async fn settings_validation_for_backup_keys() {
    let d = Dir::new();
    let pool = open(d.path()).await;
    assert!(settings::set(&pool, "backup_keep_count", "2").await.is_err());
    assert!(settings::set(&pool, "backup_keep_count", "20").await.is_ok());
    assert!(settings::set(&pool, "backup_auto_enabled", "x").await.is_err());
    assert!(settings::set(&pool, "backup_auto_enabled", "0").await.is_ok());
    assert!(settings::set(&pool, "backup_external_dir", "E:\\Backups").await.is_ok());
}
