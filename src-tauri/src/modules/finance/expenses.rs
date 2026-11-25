//! المصروفات: تصنيفات، تسجيل مصروف (يخصم من الخزينة)، وإلغاؤه بقيد عكسي.

use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::money::parse_minor;
use crate::app_core::text::normalize;

use super::record_tx;

fn bad<T>(m: impl Into<String>) -> Result<T, AppError> {
    Err(AppError::Validation(m.into()))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategory {
    pub id: String,
    pub name_ar: String,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategoryInput {
    pub id: Option<String>,
    pub name_ar: String,
    #[serde(default = "yes")]
    pub is_active: bool,
}
fn yes() -> bool {
    true
}

pub async fn list_categories(pool: &SqlitePool) -> Result<Vec<ExpenseCategory>, AppError> {
    Ok(sqlx::query_as("SELECT id, name_ar, is_active FROM expense_categories ORDER BY is_active DESC, name_ar")
        .fetch_all(pool)
        .await?)
}

pub async fn save_category(pool: &SqlitePool, input: &ExpenseCategoryInput) -> Result<String, AppError> {
    let name = input.name_ar.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return bad("اسم التصنيف مطلوب ولا يزيد عن 80 حرفًا");
    }
    let key = normalize(name);
    let dup: Option<String> = sqlx::query_scalar("SELECT id FROM expense_categories WHERE name_key = ?1 AND (?2 IS NULL OR id <> ?2)")
        .bind(&key)
        .bind(&input.id)
        .fetch_optional(pool)
        .await?;
    if dup.is_some() {
        return bad("يوجد تصنيف بنفس الاسم");
    }
    if let Some(id) = &input.id {
        let r = sqlx::query("UPDATE expense_categories SET name_ar=?2, name_key=?3, is_active=?4 WHERE id=?1")
            .bind(id)
            .bind(name)
            .bind(&key)
            .bind(input.is_active)
            .execute(pool)
            .await?;
        if r.rows_affected() == 0 {
            return bad("التصنيف غير موجود");
        }
        Ok(id.clone())
    } else {
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO expense_categories (id, name_ar, name_key, is_active) VALUES (?1,?2,?3,?4)")
            .bind(&id)
            .bind(name)
            .bind(&key)
            .bind(input.is_active)
            .execute(pool)
            .await?;
        Ok(id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseInput {
    pub category_id: String,
    pub amount: String,
    /// "cash" أو "card"
    pub method: String,
    #[serde(default)]
    pub description: String,
}

pub async fn create(pool: &SqlitePool, input: &ExpenseInput) -> Result<String, AppError> {
    let amount = parse_minor(&input.amount)?;
    if amount <= 0 {
        return bad("المبلغ يجب أن يكون أكبر من صفر");
    }
    if !["cash", "card"].contains(&input.method.as_str()) {
        return bad("طريقة الدفع غير مدعومة");
    }
    if input.description.chars().count() > 250 {
        return bad("الوصف أطول من 250 حرفًا");
    }
    let mut tx = pool.begin().await?;
    let active: Option<bool> = sqlx::query_scalar("SELECT is_active FROM expense_categories WHERE id = ?1")
        .bind(&input.category_id)
        .fetch_optional(&mut *tx)
        .await?;
    match active {
        None => return bad("التصنيف غير موجود"),
        Some(false) => return bad("التصنيف موقوف"),
        _ => {}
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO expenses (id, category_id, amount_minor, method, description) VALUES (?1,?2,?3,?4,?5)")
        .bind(&id)
        .bind(&input.category_id)
        .bind(amount)
        .bind(&input.method)
        .bind(input.description.trim())
        .execute(&mut *tx)
        .await?;
    record_tx(&mut tx, "expense", &input.method, -amount, "expense", &id).await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn void(pool: &SqlitePool, id: &str, reason: &str) -> Result<(), AppError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return bad("سبب الإلغاء مطلوب");
    }
    let mut tx = pool.begin().await?;
    let row: Option<(i64, String, Option<String>)> =
        sqlx::query_as("SELECT amount_minor, method, voided_at FROM expenses WHERE id = ?1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((amount, method, voided)) = row else { return bad("المصروف غير موجود") };
    if voided.is_some() {
        return bad("المصروف ملغي بالفعل");
    }
    sqlx::query("UPDATE expenses SET voided_at = strftime('%Y-%m-%dT%H:%M:%SZ','now'), void_reason = ?2 WHERE id = ?1")
        .bind(id)
        .bind(reason)
        .execute(&mut *tx)
        .await?;
    record_tx(&mut tx, "expense_void", &method, amount, "expense", id).await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseFilter {
    pub category_id: Option<String>,
    /// YYYY-MM-DD بتوقيت الجهاز
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseRow {
    pub id: String,
    pub category_name: String,
    pub amount_minor: i64,
    pub method: String,
    pub description: String,
    pub created_at: String,
    pub voided_at: Option<String>,
    pub void_reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseReport {
    pub rows: Vec<ExpenseRow>,
    /// مجموع المصروفات غير الملغاة
    pub total_minor: i64,
}

fn valid_date(s: &str) -> bool {
    s.len() == 10 && s.chars().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() })
}

pub async fn list(pool: &SqlitePool, f: &ExpenseFilter) -> Result<ExpenseReport, AppError> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT e.id, c.name_ar AS category_name, e.amount_minor, e.method, e.description, e.created_at, e.voided_at, e.void_reason \
         FROM expenses e JOIN expense_categories c ON c.id = e.category_id WHERE 1 = 1",
    );
    if let Some(c) = f.category_id.as_ref().filter(|c| !c.is_empty()) {
        qb.push(" AND e.category_id = ").push_bind(c.clone());
    }
    if let Some(d) = f.date_from.as_ref().filter(|d| valid_date(d)) {
        qb.push(" AND date(e.created_at, 'localtime') >= ").push_bind(d.clone());
    }
    if let Some(d) = f.date_to.as_ref().filter(|d| valid_date(d)) {
        qb.push(" AND date(e.created_at, 'localtime') <= ").push_bind(d.clone());
    }
    qb.push(" ORDER BY e.created_at DESC, e.rowid DESC LIMIT 500");
    let rows = qb.build_query_as::<ExpenseRow>().fetch_all(pool).await?;
    let total_minor = rows.iter().filter(|r| r.voided_at.is_none()).map(|r| r.amount_minor).sum();
    Ok(ExpenseReport { rows, total_minor })
}
