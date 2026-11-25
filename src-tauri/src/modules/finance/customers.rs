//! العملاء: بيانات، حد ائتماني، رصيد محسوب (آجل − دفعات − مرتجعات مخصومة)، سداد على الحساب، وكشف حساب.
//! الرصيد لا يُخزَّن بل يُحسب من السجلات غير القابلة للتعديل.

use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::money::parse_minor;
use crate::app_core::text::normalize;

use super::record_tx;

fn bad<T>(m: impl Into<String>) -> Result<T, AppError> {
    Err(AppError::Validation(m.into()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerInput {
    pub id: Option<String>,
    pub name_ar: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub notes: String,
    /// بالجنيه. فارغ أو 0 = بلا حد
    #[serde(default)]
    pub credit_limit: String,
    #[serde(default = "yes")]
    pub is_active: bool,
}
fn yes() -> bool {
    true
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub id: String,
    pub name_ar: String,
    pub phone: String,
    pub address: String,
    pub notes: String,
    pub credit_limit_minor: i64,
    pub is_active: bool,
    /// المستحق على العميل لنا
    pub balance_minor: i64,
}

const BALANCE_SQL: &str = "(COALESCE((SELECT SUM(total_minor - paid_minor + change_minor) FROM sales WHERE customer_id = c.id), 0) \
     - COALESCE((SELECT SUM(amount_minor) FROM customer_payments WHERE customer_id = c.id), 0) \
     - COALESCE((SELECT SUM(r.credit_minor) FROM sales_returns r JOIN sales s ON s.id = r.sale_id WHERE s.customer_id = c.id), 0))";

pub async fn balance(conn: &mut SqliteConnection, customer_id: &str) -> Result<i64, AppError> {
    let sql = format!("SELECT {BALANCE_SQL} FROM customers c WHERE c.id = ?1");
    let b: Option<i64> = sqlx::query_scalar(&sql).bind(customer_id).fetch_optional(&mut *conn).await?;
    b.ok_or_else(|| AppError::Validation("العميل غير موجود".into()))
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Customer>, AppError> {
    let sql = format!(
        "SELECT c.id, c.name_ar, c.phone, c.address, c.notes, c.credit_limit_minor, c.is_active, {BALANCE_SQL} AS balance_minor \
         FROM customers c ORDER BY c.is_active DESC, c.name_ar"
    );
    Ok(sqlx::query_as::<_, Customer>(&sql).fetch_all(pool).await?)
}

pub async fn save(pool: &SqlitePool, input: &CustomerInput) -> Result<String, AppError> {
    let name = input.name_ar.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return bad("اسم العميل مطلوب ولا يزيد عن 120 حرفًا");
    }
    if input.phone.chars().count() > 30 || input.address.chars().count() > 250 || input.notes.chars().count() > 500 {
        return bad("أحد الحقول أطول من الحد المسموح");
    }
    let limit = if input.credit_limit.trim().is_empty() { 0 } else { parse_minor(&input.credit_limit)? };
    let key = normalize(name);
    let dup: Option<String> = sqlx::query_scalar("SELECT id FROM customers WHERE name_key = ?1 AND (?2 IS NULL OR id <> ?2)")
        .bind(&key)
        .bind(&input.id)
        .fetch_optional(pool)
        .await?;
    if dup.is_some() {
        return bad("يوجد عميل بنفس الاسم");
    }
    if let Some(id) = &input.id {
        let r = sqlx::query(
            "UPDATE customers SET name_ar=?2, name_key=?3, phone=?4, address=?5, notes=?6, credit_limit_minor=?7, is_active=?8, \
             updated_at=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id=?1",
        )
        .bind(id)
        .bind(name)
        .bind(&key)
        .bind(input.phone.trim())
        .bind(input.address.trim())
        .bind(input.notes.trim())
        .bind(limit)
        .bind(input.is_active)
        .execute(pool)
        .await?;
        if r.rows_affected() == 0 {
            return bad("العميل غير موجود");
        }
        Ok(id.clone())
    } else {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO customers (id, name_ar, name_key, phone, address, notes, credit_limit_minor, is_active) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        )
        .bind(&id)
        .bind(name)
        .bind(&key)
        .bind(input.phone.trim())
        .bind(input.address.trim())
        .bind(input.notes.trim())
        .bind(limit)
        .bind(input.is_active)
        .execute(pool)
        .await?;
        Ok(id)
    }
}

/// يتحقق من صلاحية البيع الآجل لعميل بمبلغ معيّن (داخل نفس transaction البيع).
pub async fn check_credit(conn: &mut SqliteConnection, customer_id: &str, amount_minor: i64) -> Result<(), AppError> {
    let row: Option<(bool, i64, String)> =
        sqlx::query_as("SELECT is_active, credit_limit_minor, name_ar FROM customers WHERE id = ?1")
            .bind(customer_id)
            .fetch_optional(&mut *conn)
            .await?;
    let Some((active, limit, name)) = row else { return bad("العميل غير موجود") };
    if !active {
        return bad(format!("العميل «{name}» موقوف"));
    }
    if limit > 0 {
        let current = balance(conn, customer_id).await?;
        if current + amount_minor > limit {
            return bad(format!("البيع الآجل يتجاوز الحد الائتماني للعميل «{name}»"));
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerPaymentInput {
    pub customer_id: String,
    /// "cash" أو "card"
    pub method: String,
    pub amount: String,
    #[serde(default)]
    pub note: String,
}

pub async fn take_payment(pool: &SqlitePool, input: &CustomerPaymentInput) -> Result<String, AppError> {
    let amount = parse_minor(&input.amount)?;
    if amount <= 0 {
        return bad("المبلغ يجب أن يكون أكبر من صفر");
    }
    if !["cash", "card"].contains(&input.method.as_str()) {
        return bad("طريقة الدفع غير مدعومة");
    }
    let mut tx = pool.begin().await?;
    let due = balance(&mut tx, &input.customer_id).await?;
    if amount > due {
        return bad("المبلغ أكبر من المستحق على العميل");
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO customer_payments (id, customer_id, method, amount_minor, note) VALUES (?1,?2,?3,?4,?5)")
        .bind(&id)
        .bind(&input.customer_id)
        .bind(&input.method)
        .bind(amount)
        .bind(input.note.trim())
        .execute(&mut *tx)
        .await?;
    record_tx(&mut tx, "customer_payment", &input.method, amount, "customer_payment", &id).await?;
    tx.commit().await?;
    Ok(id)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementRow {
    pub date: String,
    /// sale | payment | return
    pub kind: String,
    pub ref_no: String,
    pub debit_minor: i64,
    pub credit_minor: i64,
    pub balance_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Statement {
    pub customer: Customer,
    pub rows: Vec<StatementRow>,
}

pub async fn statement(pool: &SqlitePool, customer_id: &str) -> Result<Statement, AppError> {
    let customer = list(pool).await?.into_iter().find(|c| c.id == customer_id);
    let Some(customer) = customer else { return bad("العميل غير موجود") };
    // (التاريخ، ترتيب داخلي، النوع، المرجع، مدين، دائن)
    let mut raw: Vec<(String, i64, String, String, i64, i64)> = Vec::new();
    let sales: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT created_at, invoice_no, total_minor - paid_minor + change_minor FROM sales WHERE customer_id = ?1",
    )
    .bind(customer_id)
    .fetch_all(pool)
    .await?;
    for (d, n, due) in sales {
        raw.push((d, 0, "sale".into(), n, due, 0));
    }
    let pays: Vec<(String, i64)> = sqlx::query_as("SELECT created_at, amount_minor FROM customer_payments WHERE customer_id = ?1")
        .bind(customer_id)
        .fetch_all(pool)
        .await?;
    for (d, a) in pays {
        raw.push((d, 1, "payment".into(), String::new(), 0, a));
    }
    let rets: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT r.created_at, r.return_no, r.credit_minor FROM sales_returns r JOIN sales s ON s.id = r.sale_id \
         WHERE s.customer_id = ?1 AND r.credit_minor > 0",
    )
    .bind(customer_id)
    .fetch_all(pool)
    .await?;
    for (d, n, c) in rets {
        raw.push((d, 2, "return".into(), n, 0, c));
    }
    raw.sort_by(|a, b| (&a.0, a.1, &a.3).cmp(&(&b.0, b.1, &b.3)));
    let mut running = 0i64;
    let rows = raw
        .into_iter()
        .filter(|r| r.4 != 0 || r.5 != 0 || r.2 == "sale")
        .map(|(date, _, kind, ref_no, debit, credit)| {
            running += debit - credit;
            StatementRow { date, kind, ref_no, debit_minor: debit, credit_minor: credit, balance_minor: running }
        })
        .collect();
    Ok(Statement { customer, rows })
}
