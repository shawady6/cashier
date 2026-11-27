//! النسخ الاحتياطي والاستعادة. النسخ بطريقة SQLite الآمنة (VACUUM INTO) وليس نسخ ملف شغّال.
//! الاستعادة على مرحلتين: تجهيز ملف مفحوص بجوار القاعدة، ثم استبداله عند بدء التشغيل التالي
//! قبل فتح القاعدة. أي فشل في أي خطوة يترك القاعدة الحالية كما هي.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::app_core::error::AppError;
use crate::database;

pub const DB_FILE: &str = "pos.db";
pub const BACKUPS_DIR: &str = "backups";
const PENDING_FILE: &str = "pending-restore.db";
const DEFAULT_KEEP: usize = 14;
const KEEP_SAFETY_BACKUPS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Local,
    External,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub file_name: String,
    pub kind: String,
    pub location: Location,
    pub size_bytes: u64,
    pub modified_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCheck {
    pub ok: bool,
    pub message: String,
    pub migrations: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub backup: BackupInfo,
    pub external_copied: bool,
    pub external_error: Option<String>,
}

const KINDS: &[&str] = &["manual", "auto", "pre-migration", "pre-restore"];

fn kind_of(name: &str) -> Option<&'static str> {
    KINDS.iter().copied().find(|k| name.starts_with(&format!("{k}-")))
}

/// اسم ملف نسخة صالح: لا مسارات ولا رموز غريبة، يبدأ بنوع معروف وينتهي بـ .db
fn valid_name(name: &str) -> bool {
    kind_of(name).is_some()
        && name.ends_with(".db")
        && name.len() <= 80
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !name.contains("..")
}

pub fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(BACKUPS_DIR)
}

fn external_dir(raw: &str) -> Result<Option<PathBuf>, AppError> {
    let t = raw.trim();
    if t.is_empty() {
        return Ok(None);
    }
    let p = PathBuf::from(t);
    if !p.is_absolute() {
        return Err(AppError::Validation("مسار المجلد الخارجي يجب أن يكون كاملًا (مثل E:\\Backups)".into()));
    }
    if !p.is_dir() {
        return Err(AppError::Validation("المجلد الخارجي غير موجود أو غير متصل".into()));
    }
    Ok(Some(p))
}

fn resolve(data_dir: &Path, ext: &str, location: Location, name: &str) -> Result<PathBuf, AppError> {
    if !valid_name(name) {
        return Err(AppError::Validation("اسم الملف غير صالح".into()));
    }
    let dir = match location {
        Location::Local => backups_dir(data_dir),
        Location::External => external_dir(ext)?.ok_or_else(|| AppError::Validation("لم يتم تحديد مجلد خارجي".into()))?,
    };
    let path = dir.join(name);
    if !path.is_file() {
        return Err(AppError::Validation("ملف النسخة غير موجود".into()));
    }
    Ok(path)
}

async fn open_readonly(path: &Path) -> Result<SqlitePool, AppError> {
    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(path).read_only(true))
        .await?)
}

/// فحص ملف قاعدة بيانات: سليم، تابع للبرنامج، وإصدار بنيته ليس أحدث من البرنامج.
pub async fn verify_file(path: &Path) -> BackupCheck {
    let fail = |m: &str| BackupCheck { ok: false, message: m.to_string(), migrations: 0 };
    let Ok(pool) = open_readonly(path).await else {
        return fail("تعذر فتح الملف كقاعدة بيانات");
    };
    let out = verify_pool(&pool).await;
    pool.close().await;
    out.unwrap_or_else(|_| fail("الملف تالف أو ليس قاعدة بيانات"))
}

async fn verify_pool(pool: &SqlitePool) -> Result<BackupCheck, sqlx::Error> {
    let fail = |m: &str| BackupCheck { ok: false, message: m.to_string(), migrations: 0 };
    let verdict: String = sqlx::query_scalar("PRAGMA integrity_check").fetch_one(pool).await?;
    if verdict != "ok" {
        return Ok(fail("فحص السلامة فشل: الملف تالف"));
    }
    let has: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('_sqlx_migrations','settings','products','sales')")
        .fetch_one(pool)
        .await?;
    if has != 4 {
        return Ok(fail("الملف ليس قاعدة بيانات هذا البرنامج"));
    }
    let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1")
        .fetch_one(pool)
        .await?;
    let ours = database::migration_count();
    if applied > ours {
        return Ok(BackupCheck { ok: false, message: "النسخة من إصدار أحدث من هذا البرنامج".into(), migrations: applied });
    }
    if applied == 0 {
        return Ok(fail("النسخة فارغة"));
    }
    Ok(BackupCheck { ok: true, message: "سليمة".into(), migrations: applied })
}

fn info_of(path: &Path, location: Location) -> Option<BackupInfo> {
    let name = path.file_name()?.to_string_lossy().to_string();
    if !valid_name(&name) {
        return None;
    }
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    let modified_at = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Some(BackupInfo { kind: kind_of(&name)?.to_string(), file_name: name, location, size_bytes: meta.len(), modified_at })
}

fn list_dir(dir: &Path, location: Location) -> Vec<BackupInfo> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    rd.filter_map(|e| e.ok()).filter_map(|e| info_of(&e.path(), location)).collect()
}

/// نسخ المجلد المحلي ثم الخارجي (إن وُجد ومتصل)، الأحدث أولًا.
pub fn list_backups(data_dir: &Path, ext: &str) -> Vec<BackupInfo> {
    let mut all = list_dir(&backups_dir(data_dir), Location::Local);
    if let Ok(Some(dir)) = external_dir(ext) {
        all.extend(list_dir(&dir, Location::External));
    }
    all.sort_by(|a, b| b.modified_at.cmp(&a.modified_at).then(b.file_name.cmp(&a.file_name)));
    all
}

async fn stamp(pool: &SqlitePool) -> Result<(String, String), AppError> {
    Ok(sqlx::query_as("SELECT strftime('%Y%m%d-%H%M%S','now','localtime'), strftime('%Y%m%d','now','localtime')")
        .fetch_one(pool)
        .await?)
}

/// ينشئ نسخة داخل مجلد البيانات، يفحصها، ثم ينسخها للمجلد الخارجي إن حُدد.
/// فشل النسخ الخارجي لا يلغي النسخة المحلية السليمة، لكنه يُرجَع للمستخدم.
pub async fn create_backup(pool: &SqlitePool, data_dir: &Path, kind: &str, ext: &str) -> Result<BackupResult, AppError> {
    if !["manual", "auto", "pre-restore"].contains(&kind) {
        return Err(AppError::Validation("نوع نسخة غير صالح".into()));
    }
    let dir = backups_dir(data_dir);
    std::fs::create_dir_all(&dir)?;
    let (ts, _) = stamp(pool).await?;
    let mut name = format!("{kind}-{ts}.db");
    let mut n = 1;
    while dir.join(&name).exists() {
        n += 1;
        name = format!("{kind}-{ts}-{n}.db");
    }
    let target = dir.join(&name);
    if let Err(e) = sqlx::query("VACUUM INTO ?1").bind(target.to_string_lossy().to_string()).execute(pool).await {
        let _ = std::fs::remove_file(&target);
        return Err(AppError::Internal(format!("تعذر إنشاء النسخة الاحتياطية: {e}")));
    }
    let check = verify_file(&target).await;
    if !check.ok {
        let _ = std::fs::remove_file(&target);
        return Err(AppError::Internal(format!("النسخة المنشأة لم تجتز الفحص: {}", check.message)));
    }
    let backup = info_of(&target, Location::Local).ok_or_else(|| AppError::Internal("تعذر قراءة النسخة".into()))?;

    let (mut external_copied, mut external_error) = (false, None);
    match external_dir(ext) {
        Ok(None) => {}
        Ok(Some(edir)) => match copy_verified(&target, &edir.join(&name)).await {
            Ok(()) => external_copied = true,
            Err(e) => external_error = Some(e.to_string()),
        },
        Err(e) => external_error = Some(e.to_string()),
    }
    Ok(BackupResult { backup, external_copied, external_error })
}

/// نسخ عبر ملف مؤقت ثم إعادة تسمية، مع فحص الناتج؛ لا يظهر ملف ناقص باسم نسخة.
async fn copy_verified(src: &Path, dest: &Path) -> Result<(), AppError> {
    let tmp = dest.with_extension("db.part");
    let _ = std::fs::remove_file(&tmp);
    let res = async {
        std::fs::copy(src, &tmp)?;
        let c = verify_file(&tmp).await;
        if !c.ok {
            return Err(AppError::Internal(format!("النسخة المنسوخة لم تجتز الفحص: {}", c.message)));
        }
        std::fs::rename(&tmp, dest)?;
        Ok(())
    }
    .await;
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

/// حذف أقدم النسخ التلقائية بحيث يبقى `keep` فقط. النسخ اليدوية لا تُحذف تلقائيًا أبدًا.
pub fn prune(dir: &Path, kind: &str, keep: usize) {
    let mut files: Vec<_> = list_dir(dir, Location::Local).into_iter().filter(|b| b.kind == kind).collect();
    files.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    while files.len() > keep {
        let _ = std::fs::remove_file(dir.join(files.remove(0).file_name));
    }
}

pub fn keep_count(raw: Option<&String>) -> usize {
    raw.and_then(|v| v.parse::<usize>().ok()).filter(|n| (3..=365).contains(n)).unwrap_or(DEFAULT_KEEP)
}

/// نسخة تلقائية مرة كل يوم (أول فتح للبرنامج في اليوم). الفشل يُسجَّل ولا يمنع التشغيل.
pub async fn run_auto_if_due(pool: &SqlitePool, data_dir: &Path, settings: &std::collections::HashMap<String, String>) -> Option<BackupResult> {
    if settings.get("backup_auto_enabled").map(|v| v.as_str()) == Some("0") {
        return None;
    }
    let today = match stamp(pool).await {
        Ok((_, d)) => d,
        Err(e) => {
            tracing::warn!("تعذر قراءة التاريخ للنسخ التلقائي: {e}");
            return None;
        }
    };
    let dir = backups_dir(data_dir);
    let done = list_dir(&dir, Location::Local).iter().any(|b| b.kind == "auto" && b.file_name.starts_with(&format!("auto-{today}")));
    if done {
        return None;
    }
    let ext = settings.get("backup_external_dir").map(|s| s.as_str()).unwrap_or("");
    match create_backup(pool, data_dir, "auto", ext).await {
        Ok(r) => {
            prune(&dir, "auto", keep_count(settings.get("backup_keep_count")));
            prune(&dir, "pre-restore", KEEP_SAFETY_BACKUPS);
            if let Some(e) = &r.external_error {
                tracing::warn!("النسخة التلقائية: تعذر النسخ للمجلد الخارجي: {e}");
            }
            tracing::info!("تمت النسخة الاحتياطية التلقائية {}", r.backup.file_name);
            Some(r)
        }
        Err(e) => {
            tracing::error!("فشلت النسخة الاحتياطية التلقائية: {e}");
            None
        }
    }
}

pub async fn check_backup(data_dir: &Path, ext: &str, location: Location, name: &str) -> Result<BackupCheck, AppError> {
    let path = resolve(data_dir, ext, location, name)?;
    Ok(verify_file(&path).await)
}

pub fn delete_backup(data_dir: &Path, ext: &str, location: Location, name: &str) -> Result<(), AppError> {
    let path = resolve(data_dir, ext, location, name)?;
    std::fs::remove_file(path)?;
    Ok(())
}

/// المرحلة الأولى من الاستعادة: فحص النسخة، نسخة أمان من القاعدة الحالية، ثم تجهيز ملف الاستبدال.
/// لا يُمس `pos.db` هنا. الاستبدال الفعلي يحدث في `apply_pending_restore` عند التشغيل التالي.
pub async fn stage_restore(pool: &SqlitePool, data_dir: &Path, ext: &str, location: Location, name: &str) -> Result<BackupInfo, AppError> {
    let src = resolve(data_dir, ext, location, name)?;
    let check = verify_file(&src).await;
    if !check.ok {
        return Err(AppError::Validation(format!("لا يمكن الاستعادة من هذه النسخة: {}", check.message)));
    }
    let safety = create_backup(pool, data_dir, "pre-restore", "").await?;
    prune(&backups_dir(data_dir), "pre-restore", KEEP_SAFETY_BACKUPS);
    let pending = data_dir.join(PENDING_FILE);
    copy_verified(&src, &pending).await?;
    Ok(safety.backup)
}

pub fn has_pending_restore(data_dir: &Path) -> bool {
    data_dir.join(PENDING_FILE).exists()
}

pub fn cancel_pending_restore(data_dir: &Path) {
    let _ = std::fs::remove_file(data_dir.join(PENDING_FILE));
}

/// يُستدعى عند بدء التشغيل قبل فتح القاعدة. يعيد true إذا تم الاستبدال.
/// إن فشل الفحص أو أي خطوة، تبقى القاعدة الحالية كما هي ويُحذف ملف الاستعادة المعلّق.
pub async fn apply_pending_restore(data_dir: &Path) -> Result<bool, AppError> {
    let pending = data_dir.join(PENDING_FILE);
    if !pending.exists() {
        return Ok(false);
    }
    let check = verify_file(&pending).await;
    if !check.ok {
        let _ = std::fs::remove_file(&pending);
        return Err(AppError::Validation(format!("تم إلغاء الاستعادة، الملف المجهّز غير سليم: {}", check.message)));
    }
    let db = data_dir.join(DB_FILE);
    let old = data_dir.join("pos.db.replaced");
    let _ = std::fs::remove_file(&old);
    if db.exists() {
        if let Err(e) = std::fs::rename(&db, &old) {
            let _ = std::fs::remove_file(&pending);
            return Err(AppError::Internal(format!("تم إلغاء الاستعادة، تعذر الوصول للقاعدة الحالية: {e}")));
        }
    }
    if let Err(e) = std::fs::rename(&pending, &db) {
        if old.exists() {
            let _ = std::fs::rename(&old, &db);
        }
        let _ = std::fs::remove_file(&pending);
        return Err(AppError::Internal(format!("تم إلغاء الاستعادة وإرجاع القاعدة السابقة: {e}")));
    }
    // ملفات WAL/SHM تخص القاعدة القديمة ويجب ألا تُطبَّق على المستعادة.
    for ext in ["pos.db-wal", "pos.db-shm"] {
        let _ = std::fs::remove_file(data_dir.join(ext));
    }
    let _ = std::fs::remove_file(&old);
    Ok(true)
}

#[cfg(test)]
mod tests;
