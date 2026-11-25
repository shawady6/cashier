//! تقارير المخزون: الأرصدة، الحركات، النواقص، قيمة المخزون.

use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::app_core::error::AppError;
use crate::app_core::money::div_round;
use crate::app_core::text::normalize;

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BalanceRow {
    pub warehouse_id: String,
    pub warehouse_name: String,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub name_ar: String,
    pub variant_name: Option<String>,
    pub sku: Option<String>,
    pub base_unit_name: String,
    pub qty_milli: i64,
    pub value_minor: i64,
    /// متوسط تكلفة الوحدة الأساسية (للعرض فقط)
    #[sqlx(default)]
    pub avg_cost_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalancesReport {
    pub rows: Vec<BalanceRow>,
    pub total_value_minor: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceFilter {
    pub warehouse_id: Option<String>,
    pub query: Option<String>,
}

pub async fn balances(pool: &SqlitePool, f: &BalanceFilter) -> Result<BalancesReport, AppError> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT sb.warehouse_id, w.name_ar AS warehouse_name, p.id AS product_id, sb.variant_id, p.name_ar, \
                v.name_ar AS variant_name, COALESCE(v.sku, p.sku) AS sku, u.name_ar AS base_unit_name, \
                sb.qty_milli, sb.value_minor, 0 AS avg_cost_minor \
         FROM stock_balances sb JOIN warehouses w ON w.id = sb.warehouse_id JOIN products p ON p.id = sb.product_id \
         JOIN units u ON u.id = p.base_unit_id LEFT JOIN product_variants v ON v.id = sb.variant_id \
         WHERE (sb.qty_milli <> 0 OR sb.value_minor <> 0)",
    );
    if let Some(w) = &f.warehouse_id {
        qb.push(" AND sb.warehouse_id = ").push_bind(w.clone());
    }
    if let Some(q) = &f.query {
        for t in normalize(q).split(' ').filter(|t| !t.is_empty()) {
            qb.push(" AND p.search_text LIKE ").push_bind(format!("%{}%", t.replace('%', "\\%").replace('_', "\\_"))).push(" ESCAPE '\\'");
        }
    }
    qb.push(" ORDER BY p.name_ar, v.name_ar, w.name_ar LIMIT 2000");
    let mut rows = qb.build_query_as::<BalanceRow>().fetch_all(pool).await?;
    for r in &mut rows {
        r.avg_cost_minor = if r.qty_milli > 0 {
            div_round(i128::from(r.value_minor) * 1000, i128::from(r.qty_milli)) as i64
        } else {
            0
        };
    }
    let total = rows.iter().map(|r| r.value_minor).sum();
    Ok(BalancesReport { rows, total_value_minor: total })
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovementFilter {
    pub product_id: Option<String>,
    pub warehouse_id: Option<String>,
    pub movement_type: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MovementRow {
    pub id: String,
    pub created_at: String,
    pub warehouse_name: String,
    pub name_ar: String,
    pub variant_name: Option<String>,
    pub movement_type: String,
    pub qty_delta_milli: i64,
    pub value_delta_minor: i64,
    pub note: String,
}

pub async fn movements(pool: &SqlitePool, f: &MovementFilter) -> Result<Vec<MovementRow>, AppError> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT m.id, m.created_at, w.name_ar AS warehouse_name, p.name_ar, v.name_ar AS variant_name, \
                m.movement_type, m.qty_delta_milli, m.value_delta_minor, m.note \
         FROM stock_movements m JOIN warehouses w ON w.id = m.warehouse_id JOIN products p ON p.id = m.product_id \
         LEFT JOIN product_variants v ON v.id = m.variant_id WHERE 1 = 1",
    );
    if let Some(v) = &f.product_id {
        qb.push(" AND m.product_id = ").push_bind(v.clone());
    }
    if let Some(v) = &f.warehouse_id {
        qb.push(" AND m.warehouse_id = ").push_bind(v.clone());
    }
    if let Some(v) = &f.movement_type {
        qb.push(" AND m.movement_type = ").push_bind(v.clone());
    }
    qb.push(" ORDER BY m.created_at DESC, m.rowid DESC LIMIT ").push_bind(f.limit.unwrap_or(300).clamp(1, 1000));
    Ok(qb.build_query_as::<MovementRow>().fetch_all(pool).await?)
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LowStockRow {
    pub product_id: String,
    pub name_ar: String,
    pub base_unit_name: String,
    pub stock_qty_milli: i64,
    pub min_stock_qty_milli: i64,
}

/// المنتجات التي لها حد إعادة طلب ورصيدها (كل المخازن والمتغيرات) أقل منه أو يساويه.
pub async fn low_stock(pool: &SqlitePool) -> Result<Vec<LowStockRow>, AppError> {
    Ok(sqlx::query_as::<_, LowStockRow>(
        "SELECT * FROM ( \
           SELECT p.id AS product_id, p.name_ar, u.name_ar AS base_unit_name, \
                  COALESCE((SELECT SUM(qty_milli) FROM stock_balances sb WHERE sb.product_id = p.id), 0) AS stock_qty_milli, \
                  p.min_stock_qty AS min_stock_qty_milli \
           FROM products p JOIN units u ON u.id = p.base_unit_id \
           WHERE p.archived_at IS NULL AND p.is_active = 1 AND p.min_stock_qty > 0 \
         ) WHERE stock_qty_milli <= min_stock_qty_milli ORDER BY (stock_qty_milli - min_stock_qty_milli), name_ar",
    )
    .fetch_all(pool)
    .await?)
}
