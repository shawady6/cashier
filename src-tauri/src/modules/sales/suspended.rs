//! الفواتير المعلّقة: حالة سلة الواجهة تُحفظ كما هي (JSON). لا أرقام مالية موثوقة فيها،
//! فعند الاسترجاع تُسعَّر السلة من جديد في Rust.

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::app_core::error::AppError;

use super::model::SuspendedSale;

const MAX_PAYLOAD: usize = 200_000;
const MAX_SUSPENDED: i64 = 100;

pub async fn suspend(pool: &SqlitePool, label: &str, payload: &str) -> Result<String, AppError> {
    if payload.len() > MAX_PAYLOAD || serde_json::from_str::<serde_json::Value>(payload).is_err() {
        return Err(AppError::Validation("بيانات السلة غير صالحة".into()));
    }
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM suspended_sales").fetch_one(pool).await?;
    if n >= MAX_SUSPENDED {
        return Err(AppError::Validation("عدد الفواتير المعلّقة وصل للحد الأقصى".into()));
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO suspended_sales (id, label, payload) VALUES (?1, ?2, ?3)")
        .bind(&id)
        .bind(label.trim().chars().take(80).collect::<String>())
        .bind(payload)
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<SuspendedSale>, AppError> {
    Ok(sqlx::query_as::<_, SuspendedSale>(
        "SELECT id, label, payload, created_at FROM suspended_sales ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?)
}

/// استرجاع = قراءة ثم حذف في transaction واحدة (فلا تُسترجع الفاتورة مرتين).
pub async fn resume(pool: &SqlitePool, id: &str) -> Result<SuspendedSale, AppError> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query_as::<_, SuspendedSale>("SELECT id, label, payload, created_at FROM suspended_sales WHERE id = ?1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    let Some(row) = row else {
        return Err(AppError::Validation("الفاتورة المعلّقة غير موجودة".into()));
    };
    sqlx::query("DELETE FROM suspended_sales WHERE id = ?1").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn discard(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM suspended_sales WHERE id = ?1").bind(id).execute(pool).await?;
    Ok(())
}
