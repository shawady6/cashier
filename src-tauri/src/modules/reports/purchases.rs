use std::collections::HashMap;

use serde::Serialize;
use sqlx::SqlitePool;

use super::{in_range, Range};
use crate::app_core::error::AppError;

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierRow {
    pub supplier_id: String,
    pub name_ar: String,
    pub invoices: i64,
    pub total_minor: i64,
    pub returned_minor: i64,
    pub paid_minor: i64,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchasedProductRow {
    pub product_id: String,
    pub variant_id: Option<String>,
    pub name_ar: String,
    pub variant_name: Option<String>,
    pub qty_base_milli: i64,
    pub value_minor: i64,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchasesReport {
    pub invoices: i64,
    pub total_minor: i64,
    pub returned_minor: i64,
    pub paid_minor: i64,
    pub by_supplier: Vec<SupplierRow>,
    pub by_product: Vec<PurchasedProductRow>,
}

/// المشتريات المستلمة فقط (المسودات والملغاة لا تدخل).
pub async fn purchases_report(pool: &SqlitePool, range: &Range) -> Result<PurchasesReport, AppError> {
    let (from, to) = range.bounds()?;
    let mut suppliers: HashMap<String, SupplierRow> = HashMap::new();

    let received: Vec<(String, String, i64, i64)> = sqlx::query_as(&format!(
        "SELECT s.id, s.name_ar, COUNT(p.id), COALESCE(SUM(p.total_minor), 0) FROM purchases p JOIN suppliers s ON s.id = p.supplier_id \
         WHERE p.status = 'received' AND {} GROUP BY s.id",
        in_range("p.received_at")
    ))
    .bind(&from)
    .bind(&to)
    .fetch_all(pool)
    .await?;
    for (id, name, n, total) in received {
        let e = suppliers.entry(id.clone()).or_insert_with(|| SupplierRow { supplier_id: id, name_ar: name, ..Default::default() });
        e.invoices += n;
        e.total_minor += total;
    }
    let returns: Vec<(String, String, i64)> = sqlx::query_as(&format!(
        "SELECT s.id, s.name_ar, COALESCE(SUM(r.total_minor), 0) FROM purchase_returns r JOIN suppliers s ON s.id = r.supplier_id \
         WHERE {} GROUP BY s.id",
        in_range("r.created_at")
    ))
    .bind(&from)
    .bind(&to)
    .fetch_all(pool)
    .await?;
    for (id, name, v) in returns {
        suppliers.entry(id.clone()).or_insert_with(|| SupplierRow { supplier_id: id, name_ar: name, ..Default::default() }).returned_minor += v;
    }
    let payments: Vec<(String, String, i64)> = sqlx::query_as(&format!(
        "SELECT s.id, s.name_ar, COALESCE(SUM(pp.amount_minor), 0) FROM purchase_payments pp JOIN suppliers s ON s.id = pp.supplier_id \
         WHERE {} GROUP BY s.id",
        in_range("pp.created_at")
    ))
    .bind(&from)
    .bind(&to)
    .fetch_all(pool)
    .await?;
    for (id, name, v) in payments {
        suppliers.entry(id.clone()).or_insert_with(|| SupplierRow { supplier_id: id, name_ar: name, ..Default::default() }).paid_minor += v;
    }
    let mut by_supplier: Vec<SupplierRow> = suppliers.into_values().collect();
    by_supplier.sort_by(|a, b| b.total_minor.cmp(&a.total_minor).then(a.name_ar.cmp(&b.name_ar)));

    let by_product: Vec<(String, Option<String>, String, Option<String>, i64, i64)> = sqlx::query_as(&format!(
        "SELECT pi.product_id, pi.variant_id, MAX(pi.name_ar), MAX(pi.variant_name), SUM(pi.base_qty_milli), SUM(pi.net_minor) \
         FROM purchase_items pi JOIN purchases p ON p.id = pi.purchase_id WHERE p.status = 'received' AND {} \
         GROUP BY pi.product_id, pi.variant_id ORDER BY SUM(pi.net_minor) DESC, MAX(pi.name_ar) LIMIT 200",
        in_range("p.received_at")
    ))
    .bind(&from)
    .bind(&to)
    .fetch_all(pool)
    .await?;

    let mut rep = PurchasesReport::default();
    for s in &by_supplier {
        rep.invoices += s.invoices;
        rep.total_minor += s.total_minor;
        rep.returned_minor += s.returned_minor;
        rep.paid_minor += s.paid_minor;
    }
    rep.by_supplier = by_supplier;
    rep.by_product = by_product
        .into_iter()
        .map(|(product_id, variant_id, name_ar, variant_name, qty_base_milli, value_minor)| PurchasedProductRow {
            product_id, variant_id, name_ar, variant_name, qty_base_milli, value_minor,
        })
        .collect();
    Ok(rep)
}
