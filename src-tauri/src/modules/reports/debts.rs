use serde::Serialize;
use sqlx::SqlitePool;

use crate::app_core::error::AppError;
use crate::modules::{finance::customers, inventory, purchasing::suppliers};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebtRow {
    pub id: String,
    pub name_ar: String,
    pub phone: String,
    pub amount_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebtsSummary {
    /// مستحق لنا على العملاء
    pub receivables_minor: i64,
    /// مستحق علينا للموردين
    pub payables_minor: i64,
    pub customers: Vec<DebtRow>,
    pub suppliers: Vec<DebtRow>,
}

/// أرصدة اللحظة الحالية (غير مرتبطة بنطاق تاريخ).
pub async fn debts_summary(pool: &SqlitePool) -> Result<DebtsSummary, AppError> {
    let mut cs: Vec<DebtRow> = customers::list(pool)
        .await?
        .into_iter()
        .filter(|c| c.balance_minor > 0)
        .map(|c| DebtRow { id: c.id, name_ar: c.name_ar, phone: c.phone, amount_minor: c.balance_minor })
        .collect();
    cs.sort_by(|a, b| b.amount_minor.cmp(&a.amount_minor).then(a.name_ar.cmp(&b.name_ar)));
    let mut ss: Vec<DebtRow> = suppliers::list(pool)
        .await?
        .into_iter()
        .filter(|s| s.balance_minor > 0)
        .map(|s| DebtRow { id: s.id, name_ar: s.name_ar, phone: s.phone, amount_minor: s.balance_minor })
        .collect();
    ss.sort_by(|a, b| b.amount_minor.cmp(&a.amount_minor).then(a.name_ar.cmp(&b.name_ar)));
    Ok(DebtsSummary {
        receivables_minor: cs.iter().map(|c| c.amount_minor).sum(),
        payables_minor: ss.iter().map(|s| s.amount_minor).sum(),
        customers: cs,
        suppliers: ss,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarehouseValue {
    pub name_ar: String,
    pub value_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventorySummary {
    pub total_value_minor: i64,
    pub items_in_stock: i64,
    pub low_stock_count: i64,
    pub warehouses: Vec<WarehouseValue>,
}

pub async fn inventory_summary(pool: &SqlitePool) -> Result<InventorySummary, AppError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT w.name_ar, COALESCE(SUM(sb.value_minor), 0) FROM warehouses w LEFT JOIN stock_balances sb ON sb.warehouse_id = w.id \
         WHERE w.is_active = 1 GROUP BY w.id ORDER BY w.name_ar",
    )
    .fetch_all(pool)
    .await?;
    let items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM (SELECT product_id, variant_id FROM stock_balances GROUP BY product_id, variant_id HAVING SUM(qty_milli) > 0)")
        .fetch_one(pool)
        .await?;
    let low = inventory::reports::low_stock(pool).await?.len() as i64;
    let warehouses: Vec<WarehouseValue> = rows.into_iter().map(|(name_ar, value_minor)| WarehouseValue { name_ar, value_minor }).collect();
    Ok(InventorySummary {
        total_value_minor: warehouses.iter().map(|w| w.value_minor).sum(),
        items_in_stock: items,
        low_stock_count: low,
        warehouses,
    })
}
