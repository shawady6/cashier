use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use sqlx::SqlitePool;

use super::{in_range, margin_bp, Range};
use crate::app_core::error::AppError;

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayRow {
    pub day: String,
    pub invoices: i64,
    pub total_minor: i64,
    pub tax_minor: i64,
    pub returns_minor: i64,
    /// الإيراد الصافي بعد المرتجعات وبدون ضريبة
    pub revenue_minor: i64,
    pub cogs_minor: i64,
    pub profit_minor: i64,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodRow {
    pub method: String,
    pub received_minor: i64,
    pub refunded_minor: i64,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesReport {
    pub invoices: i64,
    pub gross_minor: i64,
    pub discounts_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub returns_minor: i64,
    pub net_sales_minor: i64,
    pub revenue_minor: i64,
    pub cogs_minor: i64,
    pub profit_minor: i64,
    pub margin_bp: i64,
    pub by_day: Vec<DayRow>,
    pub by_method: Vec<MethodRow>,
}

/// مرتجعات بند: القيمة، وقيمتها بدون الضريبة (بنسبة ضريبة البند الأصلي)، وتكلفتها.
const RETURN_EX_TAX: &str =
    "ri.refund_minor - CASE WHEN si.total_minor > 0 THEN (ri.refund_minor * si.tax_minor + si.total_minor / 2) / si.total_minor ELSE 0 END";

pub async fn sales_report(pool: &SqlitePool, range: &Range) -> Result<SalesReport, AppError> {
    let (from, to) = range.bounds()?;
    let sold: Vec<(String, i64, i64, i64, i64, i64, i64)> = sqlx::query_as(&format!(
        "SELECT date(s.created_at, 'localtime') AS d, COUNT(*), COALESCE(SUM(s.subtotal_minor), 0), \
                COALESCE(SUM(s.item_discount_minor + s.invoice_discount_minor), 0), COALESCE(SUM(s.tax_minor), 0), \
                COALESCE(SUM(s.total_minor), 0), COALESCE(SUM(s.cost_minor), 0) \
         FROM sales s WHERE {} GROUP BY d",
        in_range("s.created_at")
    ))
    .bind(&from)
    .bind(&to)
    .fetch_all(pool)
    .await?;
    let returned: Vec<(String, i64, i64, i64)> = sqlx::query_as(&format!(
        "SELECT date(r.created_at, 'localtime') AS d, COALESCE(SUM(ri.refund_minor), 0), COALESCE(SUM({RETURN_EX_TAX}), 0), \
                COALESCE(SUM(ri.cost_minor), 0) \
         FROM sales_return_items ri JOIN sales_returns r ON r.id = ri.return_id JOIN sale_items si ON si.id = ri.sale_item_id \
         WHERE {} GROUP BY d",
        in_range("r.created_at")
    ))
    .bind(&from)
    .bind(&to)
    .fetch_all(pool)
    .await?;

    let mut days: BTreeMap<String, DayRow> = BTreeMap::new();
    let mut rep = SalesReport::default();
    let mut ex_tax_total: HashMap<String, i64> = HashMap::new();
    for (d, n, gross, disc, tax, total, cost) in sold {
        let r = days.entry(d.clone()).or_insert_with(|| DayRow { day: d.clone(), ..Default::default() });
        r.invoices += n;
        r.total_minor += total;
        r.tax_minor += tax;
        r.revenue_minor += total - tax;
        r.cogs_minor += cost;
        rep.invoices += n;
        rep.gross_minor += gross;
        rep.discounts_minor += disc;
        rep.tax_minor += tax;
        rep.total_minor += total;
        ex_tax_total.insert(d, total - tax);
    }
    for (d, refund, refund_ex_tax, cost) in returned {
        let r = days.entry(d.clone()).or_insert_with(|| DayRow { day: d.clone(), ..Default::default() });
        r.returns_minor += refund;
        r.revenue_minor -= refund_ex_tax;
        r.cogs_minor -= cost;
        rep.returns_minor += refund;
    }
    for r in days.values_mut() {
        r.profit_minor = r.revenue_minor - r.cogs_minor;
        rep.revenue_minor += r.revenue_minor;
        rep.cogs_minor += r.cogs_minor;
    }
    rep.net_sales_minor = rep.total_minor - rep.returns_minor;
    rep.profit_minor = rep.revenue_minor - rep.cogs_minor;
    rep.margin_bp = margin_bp(rep.profit_minor, rep.revenue_minor);
    rep.by_day = days.into_values().collect();

    let methods: Vec<(String, i64, i64)> = sqlx::query_as(&format!(
        "SELECT method, COALESCE(SUM(CASE WHEN kind = 'sale' THEN amount_minor ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN kind = 'sale_return' THEN -amount_minor ELSE 0 END), 0) \
         FROM cash_transactions WHERE kind IN ('sale', 'sale_return') AND {} GROUP BY method ORDER BY method",
        in_range("created_at")
    ))
    .bind(&from)
    .bind(&to)
    .fetch_all(pool)
    .await?;
    rep.by_method = methods.into_iter().map(|(method, received_minor, refunded_minor)| MethodRow { method, received_minor, refunded_minor }).collect();
    Ok(rep)
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductProfitRow {
    pub product_id: String,
    pub variant_id: Option<String>,
    pub name_ar: String,
    pub variant_name: Option<String>,
    /// الكمية الصافية بالوحدة الأساسية (milli) بعد المرتجعات
    pub qty_base_milli: i64,
    pub revenue_minor: i64,
    pub cost_minor: i64,
    pub profit_minor: i64,
    pub margin_bp: i64,
}

/// ربح كل صنف على تكلفته الفعلية وقت البيع. sort: profit | revenue | qty
pub async fn product_profit(pool: &SqlitePool, range: &Range, sort: &str, limit: i64) -> Result<Vec<ProductProfitRow>, AppError> {
    let (from, to) = range.bounds()?;
    let sold: Vec<(String, Option<String>, String, Option<String>, i64, i64, i64)> = sqlx::query_as(&format!(
        "SELECT si.product_id, si.variant_id, MAX(si.name_ar), MAX(si.variant_name), SUM(si.base_qty_milli), \
                SUM(si.total_minor - si.tax_minor), SUM(si.cost_minor) \
         FROM sale_items si JOIN sales s ON s.id = si.sale_id WHERE {} GROUP BY si.product_id, si.variant_id",
        in_range("s.created_at")
    ))
    .bind(&from)
    .bind(&to)
    .fetch_all(pool)
    .await?;
    let returned: Vec<(String, Option<String>, String, Option<String>, i64, i64, i64)> = sqlx::query_as(&format!(
        "SELECT si.product_id, si.variant_id, MAX(si.name_ar), MAX(si.variant_name), SUM(ri.base_qty_milli), \
                SUM({RETURN_EX_TAX}), SUM(ri.cost_minor) \
         FROM sales_return_items ri JOIN sales_returns r ON r.id = ri.return_id JOIN sale_items si ON si.id = ri.sale_item_id \
         WHERE {} GROUP BY si.product_id, si.variant_id",
        in_range("r.created_at")
    ))
    .bind(&from)
    .bind(&to)
    .fetch_all(pool)
    .await?;

    let mut map: HashMap<(String, Option<String>), ProductProfitRow> = HashMap::new();
    for (pid, vid, name, vname, qty, rev, cost) in sold {
        let e = map.entry((pid.clone(), vid.clone())).or_insert_with(|| ProductProfitRow {
            product_id: pid, variant_id: vid, name_ar: name, variant_name: vname, ..Default::default()
        });
        e.qty_base_milli += qty;
        e.revenue_minor += rev;
        e.cost_minor += cost;
    }
    for (pid, vid, name, vname, qty, rev, cost) in returned {
        let e = map.entry((pid.clone(), vid.clone())).or_insert_with(|| ProductProfitRow {
            product_id: pid, variant_id: vid, name_ar: name, variant_name: vname, ..Default::default()
        });
        e.qty_base_milli -= qty;
        e.revenue_minor -= rev;
        e.cost_minor -= cost;
    }
    let mut rows: Vec<ProductProfitRow> = map.into_values().collect();
    for r in &mut rows {
        r.profit_minor = r.revenue_minor - r.cost_minor;
        r.margin_bp = margin_bp(r.profit_minor, r.revenue_minor);
    }
    match sort {
        "revenue" => rows.sort_by(|a, b| b.revenue_minor.cmp(&a.revenue_minor).then(a.name_ar.cmp(&b.name_ar))),
        "qty" => rows.sort_by(|a, b| b.qty_base_milli.cmp(&a.qty_base_milli).then(a.name_ar.cmp(&b.name_ar))),
        _ => rows.sort_by(|a, b| b.profit_minor.cmp(&a.profit_minor).then(a.name_ar.cmp(&b.name_ar))),
    }
    rows.truncate(limit.clamp(1, 1000) as usize);
    Ok(rows)
}
