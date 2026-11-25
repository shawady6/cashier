//! إتمام البيع: transaction واحدة تشمل التحقق والفاتورة والبنود والمدفوعات وخصم المخزون
//! وحركات المخزون وحركة النقدية. أي فشل يرجّع كل شيء.

use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::modules::inventory::{self, MovementRef};

use super::model::{CartInput, CartQuote, CheckoutResult};
use super::quote::quote;

pub async fn next_number(conn: &mut SqliteConnection, seq: &str, prefix: &str) -> Result<String, AppError> {
    let n: i64 = sqlx::query_scalar("SELECT next_value FROM sequences WHERE name = ?1")
        .bind(seq)
        .fetch_one(&mut *conn)
        .await?;
    sqlx::query("UPDATE sequences SET next_value = next_value + 1 WHERE name = ?1")
        .bind(seq)
        .execute(&mut *conn)
        .await?;
    Ok(format!("{prefix}{n:06}"))
}

pub async fn price_cart(pool: &SqlitePool, input: &CartInput) -> Result<CartQuote, AppError> {
    let mut conn = pool.acquire().await?;
    quote(&mut conn, input).await
}

pub async fn checkout(pool: &SqlitePool, input: &CartInput) -> Result<CheckoutResult, AppError> {
    if input.items.is_empty() {
        return Err(AppError::Validation("السلة فارغة".into()));
    }
    let key = input.client_request_id.clone().filter(|k| !k.trim().is_empty()).unwrap_or_else(|| Uuid::new_v4().to_string());

    let mut tx = pool.begin().await?;
    match checkout_in_tx(&mut tx, input, &key).await {
        Ok(r) => {
            tx.commit().await?;
            if !r.duplicate {
                tracing::info!("تم إتمام الفاتورة {} بإجمالي {} قرش", r.invoice_no, r.total);
            }
            Ok(r)
        }
        Err(e) => {
            tx.rollback().await?;
            // سباق نادر: طلب مكرر دخل بين الفحص والإدخال. نرجّع الفاتورة الأصلية بدل خطأ.
            if let AppError::Database(sqlx::Error::Database(d)) = &e {
                if d.is_unique_violation() {
                    if let Some(r) = existing(pool, &key).await? {
                        return Ok(r);
                    }
                }
            }
            Err(e)
        }
    }
}

async fn existing(pool: &SqlitePool, key: &str) -> Result<Option<CheckoutResult>, AppError> {
    let row: Option<(String, String, i64, i64)> =
        sqlx::query_as("SELECT id, invoice_no, total_minor, change_minor FROM sales WHERE idempotency_key = ?1")
            .bind(key)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(sale_id, invoice_no, total, change)| CheckoutResult { sale_id, invoice_no, total, change, duplicate: true }))
}

async fn checkout_in_tx(conn: &mut SqliteConnection, input: &CartInput, key: &str) -> Result<CheckoutResult, AppError> {
    // منع التكرار: نفس المفتاح يرجّع نفس الفاتورة.
    let dup: Option<(String, String, i64, i64)> =
        sqlx::query_as("SELECT id, invoice_no, total_minor, change_minor FROM sales WHERE idempotency_key = ?1")
            .bind(key)
            .fetch_optional(&mut *conn)
            .await?;
    if let Some((sale_id, invoice_no, total, change)) = dup {
        return Ok(CheckoutResult { sale_id, invoice_no, total, change, duplicate: true });
    }

    // 1-3: تحقق السلة والأسعار والخصومات (الأسعار من القاعدة)
    let q = quote(conn, input).await?;
    if q.remaining > 0 {
        return Err(AppError::Validation(format!(
            "المدفوع أقل من إجمالي الفاتورة (المتبقي {} قرش). البيع الآجل يتاح مع العملاء في مرحلة لاحقة",
            q.remaining
        )));
    }

    let sale_id = Uuid::new_v4().to_string();
    let warehouse = inventory::default_warehouse(conn).await?;
    let allow_neg = inventory::allow_negative(conn).await?;

    // 7-8: خصم المخزون وحركاته، والحصول على التكلفة الفعلية لكل سطر
    let mut costs = Vec::with_capacity(q.lines.len());
    for l in &q.lines {
        let movement = MovementRef { movement_type: "sale", ref_type: "sale", ref_id: &sale_id, note: "" };
        let cost = inventory::issue(
            conn, &warehouse, &l.product_id, l.variant_id.as_deref(), l.base_qty_milli,
            l.fallback_unit_cost, allow_neg, &movement, &l.name_ar,
        )
        .await?;
        costs.push(cost);
    }
    let cost_total: i64 = costs.iter().sum();

    // 4: الفاتورة
    let invoice_no = next_number(conn, "invoice", "INV-").await?;
    sqlx::query(
        "INSERT INTO sales (id, invoice_no, idempotency_key, warehouse_id, subtotal_minor, item_discount_minor, \
         invoice_discount_minor, tax_minor, total_minor, paid_minor, change_minor, cost_minor, note) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
    )
    .bind(&sale_id)
    .bind(&invoice_no)
    .bind(key)
    .bind(&warehouse)
    .bind(q.subtotal)
    .bind(q.item_discount)
    .bind(q.invoice_discount)
    .bind(q.tax)
    .bind(q.total)
    .bind(q.paid)
    .bind(q.change)
    .bind(cost_total)
    .bind(input.note.trim())
    .execute(&mut *conn)
    .await?;

    // 5: البنود (نسخة تاريخية من الأسماء والأسعار)
    for (i, (l, cost)) in q.lines.iter().zip(&costs).enumerate() {
        sqlx::query(
            "INSERT INTO sale_items (id, sale_id, line_no, product_id, variant_id, name_ar, variant_name, sku, unit_id, \
             unit_name, unit_factor_micro, qty_milli, base_qty_milli, unit_price_minor, gross_minor, discount_minor, \
             invoice_discount_minor, tax_rate_bp, tax_minor, total_minor, cost_minor) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&sale_id)
        .bind(i as i64 + 1)
        .bind(&l.product_id)
        .bind(&l.variant_id)
        .bind(&l.name_ar)
        .bind(&l.variant_name)
        .bind(&l.sku)
        .bind(&l.unit_id)
        .bind(&l.unit_name)
        .bind(l.unit_factor_micro)
        .bind(l.qty_milli)
        .bind(l.base_qty_milli)
        .bind(l.unit_price)
        .bind(l.gross)
        .bind(l.discount)
        .bind(l.invoice_discount)
        .bind(l.tax_rate_bp)
        .bind(l.tax)
        .bind(l.total)
        .bind(cost)
        .execute(&mut *conn)
        .await?;
    }

    // 6 + 9: المدفوعات وحركة النقدية (الكاش صافي بعد الفكة)
    for p in &input.payments {
        let amount = crate::app_core::money::parse_minor(&p.amount)?;
        if amount == 0 {
            continue;
        }
        sqlx::query("INSERT INTO payments (id, sale_id, method, amount_minor, reference) VALUES (?1,?2,?3,?4,?5)")
            .bind(Uuid::new_v4().to_string())
            .bind(&sale_id)
            .bind(&p.method)
            .bind(amount)
            .bind(p.reference.trim())
            .execute(&mut *conn)
            .await?;
    }
    for (method, amount) in [("cash", q.cash_net), ("card", q.card)] {
        if amount > 0 {
            sqlx::query(
                "INSERT INTO cash_transactions (id, kind, method, amount_minor, ref_type, ref_id) \
                 VALUES (?1,'sale',?2,?3,'sale',?4)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(method)
            .bind(amount)
            .bind(&sale_id)
            .execute(&mut *conn)
            .await?;
        }
    }

    Ok(CheckoutResult { sale_id, invoice_no, total: q.total, change: q.change, duplicate: false })
}
