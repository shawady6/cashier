//! قراءة الفواتير: القائمة والتفاصيل (تستخدم أيضًا للإيصال وإعادة الطباعة).

use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::app_core::error::AppError;

use super::model::*;

fn like_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn valid_date(s: &str) -> bool {
    s.len() == 10
        && s.chars().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() })
}

pub async fn list(pool: &SqlitePool, f: &SaleFilter) -> Result<Vec<SaleListItem>, AppError> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT s.id, s.invoice_no, s.created_at, s.total_minor, s.payment_status, \
                (SELECT COUNT(*) FROM sale_items WHERE sale_id = s.id) AS item_count, \
                COALESCE((SELECT SUM(refund_minor) FROM sales_returns WHERE sale_id = s.id), 0) AS returned_minor \
         FROM sales s WHERE 1 = 1",
    );
    if let Some(q) = f.query.as_ref().map(|q| q.trim()).filter(|q| !q.is_empty()) {
        qb.push(" AND s.invoice_no LIKE ").push_bind(format!("%{}%", like_escape(&q.to_uppercase()))).push(" ESCAPE '\\'");
    }
    if let Some(d) = f.date_from.as_ref().filter(|d| valid_date(d)) {
        qb.push(" AND date(s.created_at, 'localtime') >= ").push_bind(d.clone());
    }
    if let Some(d) = f.date_to.as_ref().filter(|d| valid_date(d)) {
        qb.push(" AND date(s.created_at, 'localtime') <= ").push_bind(d.clone());
    }
    qb.push(" ORDER BY s.created_at DESC, s.invoice_no DESC LIMIT ").push_bind(f.limit.unwrap_or(200).clamp(1, 500));
    Ok(qb.build_query_as::<SaleListItem>().fetch_all(pool).await?)
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<SaleDetail, AppError> {
    let head: Option<(String, String, String, String, i64, i64, i64, i64, i64, i64, i64, String)> = sqlx::query_as(
        "SELECT id, invoice_no, created_at, payment_status, subtotal_minor, item_discount_minor, invoice_discount_minor, \
                tax_minor, total_minor, paid_minor, change_minor, note FROM sales WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let Some(h) = head else { return Err(AppError::Validation("الفاتورة غير موجودة".into())) };

    let items = sqlx::query_as::<_, SaleItemDetail>(
        "SELECT si.id, si.line_no, si.name_ar, si.variant_name, si.unit_name, si.qty_milli, si.unit_price_minor, \
                si.gross_minor, si.discount_minor, si.invoice_discount_minor, si.tax_minor, si.total_minor, \
                COALESCE((SELECT SUM(qty_milli) FROM sales_return_items WHERE sale_item_id = si.id), 0) AS returned_qty_milli \
         FROM sale_items si WHERE si.sale_id = ?1 ORDER BY si.line_no",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let payments = sqlx::query_as::<_, PaymentDetail>(
        "SELECT method, amount_minor, reference FROM payments WHERE sale_id = ?1 ORDER BY created_at, method",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let returns = sqlx::query_as::<_, ReturnSummary>(
        "SELECT id, return_no, refund_minor, refund_method, reason, created_at FROM sales_returns \
         WHERE sale_id = ?1 ORDER BY created_at, return_no",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(SaleDetail {
        id: h.0, invoice_no: h.1, created_at: h.2, payment_status: h.3, subtotal_minor: h.4,
        item_discount_minor: h.5, invoice_discount_minor: h.6, tax_minor: h.7, total_minor: h.8,
        paid_minor: h.9, change_minor: h.10, note: h.11, items, payments, returns,
    })
}
