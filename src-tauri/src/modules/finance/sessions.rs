//! ورديات الخزينة: فتح بعهدة افتتاحية، حركات يدوية، وإغلاق بجرد فعلي وحساب الفرق.
//! المتوقع في الدرج = الافتتاحي + مجموع حركات النقد الموقّعة في الوردية.

use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::money::parse_minor;

use super::{open_session_id, record_tx};

fn bad<T>(m: impl Into<String>) -> Result<T, AppError> {
    Err(AppError::Validation(m.into()))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub status: String,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub opening_minor: i64,
    pub cash_sales_minor: i64,
    pub card_sales_minor: i64,
    /// موجب = مبلغ مردود (يُعرض كقيمة موجبة)
    pub cash_returns_minor: i64,
    pub cash_expenses_minor: i64,
    pub supplier_payments_minor: i64,
    pub customer_payments_minor: i64,
    pub cash_in_minor: i64,
    pub cash_out_minor: i64,
    pub card_total_minor: i64,
    pub expected_cash_minor: i64,
    pub counted_cash_minor: Option<i64>,
    pub difference_minor: Option<i64>,
    pub note: String,
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: String,
    status: String,
    opening_minor: i64,
    opened_at: String,
    closed_at: Option<String>,
    counted_cash_minor: Option<i64>,
    difference_minor: Option<i64>,
    note: String,
}

pub(super) async fn summary(conn: &mut SqliteConnection, id: &str) -> Result<SessionSummary, AppError> {
    let s = sqlx::query_as::<_, SessionRow>(
        "SELECT id, status, opening_minor, opened_at, closed_at, counted_cash_minor, difference_minor, note \
         FROM cash_sessions WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some(s) = s else { return bad("الوردية غير موجودة") };
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT kind, method, COALESCE(SUM(amount_minor), 0) FROM cash_transactions WHERE session_id = ?1 GROUP BY kind, method",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    let sum = |kind: &str, method: &str| -> i64 {
        rows.iter().filter(|(k, m, _)| k == kind && m == method).map(|r| r.2).sum()
    };
    let cash_total: i64 = rows.iter().filter(|(_, m, _)| m == "cash").map(|r| r.2).sum();
    let card_total: i64 = rows.iter().filter(|(_, m, _)| m == "card").map(|r| r.2).sum();
    Ok(SessionSummary {
        id: s.id,
        status: s.status,
        opened_at: s.opened_at,
        closed_at: s.closed_at,
        opening_minor: s.opening_minor,
        cash_sales_minor: sum("sale", "cash"),
        card_sales_minor: sum("sale", "card"),
        cash_returns_minor: -sum("sale_return", "cash"),
        cash_expenses_minor: -(sum("expense", "cash") + sum("expense_void", "cash")),
        supplier_payments_minor: -sum("supplier_payment", "cash"),
        customer_payments_minor: sum("customer_payment", "cash"),
        cash_in_minor: sum("cash_in", "cash"),
        cash_out_minor: -sum("cash_out", "cash"),
        card_total_minor: card_total,
        expected_cash_minor: s.opening_minor + cash_total,
        counted_cash_minor: s.counted_cash_minor,
        difference_minor: s.difference_minor,
        note: s.note,
    })
}

pub async fn open(pool: &SqlitePool, opening: &str, note: &str) -> Result<String, AppError> {
    let opening = parse_minor(opening)?;
    let mut tx = pool.begin().await?;
    if open_session_id(&mut tx).await?.is_some() {
        return bad("توجد وردية مفتوحة بالفعل. أغلقها أولًا");
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO cash_sessions (id, opening_minor, note) VALUES (?1,?2,?3)")
        .bind(&id)
        .bind(opening)
        .bind(note.trim())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    tracing::info!("تم فتح وردية بعهدة {opening} قرش");
    Ok(id)
}

pub async fn current(pool: &SqlitePool) -> Result<Option<SessionSummary>, AppError> {
    let mut conn = pool.acquire().await?;
    match open_session_id(&mut conn).await? {
        Some(id) => Ok(Some(summary(&mut conn, &id).await?)),
        None => Ok(None),
    }
}

pub async fn close(pool: &SqlitePool, id: &str, counted: &str, note: &str) -> Result<SessionSummary, AppError> {
    let counted = parse_minor(counted)?;
    let mut tx = pool.begin().await?;
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM cash_sessions WHERE id = ?1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    match status.as_deref() {
        None => return bad("الوردية غير موجودة"),
        Some("closed") => return bad("الوردية مغلقة بالفعل"),
        _ => {}
    }
    let expected = summary(&mut tx, id).await?.expected_cash_minor;
    sqlx::query(
        "UPDATE cash_sessions SET status='closed', closed_at=strftime('%Y-%m-%dT%H:%M:%SZ','now'), \
         expected_cash_minor=?2, counted_cash_minor=?3, difference_minor=?4, note=CASE WHEN ?5 = '' THEN note ELSE ?5 END WHERE id=?1",
    )
    .bind(id)
    .bind(expected)
    .bind(counted)
    .bind(counted - expected)
    .bind(note.trim())
    .execute(&mut *tx)
    .await?;
    let s = summary(&mut tx, id).await?;
    tx.commit().await?;
    tracing::info!("تم إغلاق الوردية، الفرق {} قرش", counted - expected);
    Ok(s)
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<SessionSummary>, AppError> {
    let mut conn = pool.acquire().await?;
    let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM cash_sessions ORDER BY opened_at DESC, rowid DESC LIMIT 100")
        .fetch_all(&mut *conn)
        .await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(summary(&mut conn, &id).await?);
    }
    Ok(out)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashEntryInput {
    /// "in" أو "out"
    pub direction: String,
    pub amount: String,
    pub reason: String,
}

/// إدخال/إخراج نقدية يدوي من الدرج. يلزم وردية مفتوحة وسبب.
pub async fn add_entry(pool: &SqlitePool, input: &CashEntryInput) -> Result<String, AppError> {
    let amount = parse_minor(&input.amount)?;
    if amount <= 0 {
        return bad("المبلغ يجب أن يكون أكبر من صفر");
    }
    if !["in", "out"].contains(&input.direction.as_str()) {
        return bad("اتجاه الحركة غير صحيح");
    }
    let reason = input.reason.trim();
    if reason.is_empty() || reason.chars().count() > 250 {
        return bad("السبب مطلوب ولا يزيد عن 250 حرفًا");
    }
    let mut tx = pool.begin().await?;
    let Some(sid) = open_session_id(&mut tx).await? else { return bad("لا توجد وردية مفتوحة") };
    if input.direction == "out" {
        let s = summary(&mut tx, &sid).await?;
        if amount > s.expected_cash_minor {
            return bad("المبلغ أكبر من النقدية المتوقعة في الدرج");
        }
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO cash_entries (id, direction, amount_minor, reason) VALUES (?1,?2,?3,?4)")
        .bind(&id)
        .bind(&input.direction)
        .bind(amount)
        .bind(reason)
        .execute(&mut *tx)
        .await?;
    let (kind, signed) = if input.direction == "in" { ("cash_in", amount) } else { ("cash_out", -amount) };
    record_tx(&mut tx, kind, "cash", signed, "cash_entry", &id).await?;
    tx.commit().await?;
    Ok(id)
}
