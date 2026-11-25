//! المالية: ورديات الخزينة، المصروفات، العملاء والبيع الآجل.
//! كل حركة نقدية/كارت تمر عبر `record_tx` لتُربط بالوردية المفتوحة.

use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::app_core::error::AppError;

pub mod customers;
pub mod expenses;
pub mod sessions;

#[cfg(test)]
mod tests;

pub async fn open_session_id(conn: &mut SqliteConnection) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar("SELECT id FROM cash_sessions WHERE status = 'open'").fetch_optional(&mut *conn).await?)
}

pub async fn require_session_setting(conn: &mut SqliteConnection) -> Result<bool, AppError> {
    let v: Option<String> = sqlx::query_scalar("SELECT setting_value FROM settings WHERE setting_key = 'require_cash_session'")
        .fetch_optional(&mut *conn)
        .await?;
    Ok(v.as_deref() == Some("1"))
}

/// تسجيل حركة نقدية/كارت (الموجب داخل، السالب خارج) على الوردية المفتوحة.
/// لو الإعداد «الوردية إلزامية» مفعّل ولا توجد وردية مفتوحة تُرفض العملية.
pub async fn record_tx(
    conn: &mut SqliteConnection,
    kind: &str,
    method: &str,
    amount_minor: i64,
    ref_type: &str,
    ref_id: &str,
) -> Result<(), AppError> {
    let session = open_session_id(conn).await?;
    if session.is_none() && require_session_setting(conn).await? {
        return Err(AppError::Validation("افتح وردية خزينة أولًا قبل تسجيل أي حركة مالية".into()));
    }
    sqlx::query(
        "INSERT INTO cash_transactions (id, kind, method, amount_minor, ref_type, ref_id, session_id) \
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(kind)
    .bind(method)
    .bind(amount_minor)
    .bind(ref_type)
    .bind(ref_id)
    .bind(session)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
