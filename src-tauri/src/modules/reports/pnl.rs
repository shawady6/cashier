use serde::Serialize;
use sqlx::SqlitePool;

use super::{in_range, margin_bp, sales::sales_report, Range};
use crate::app_core::error::AppError;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseRow {
    pub category: String,
    pub amount_minor: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfitLoss {
    pub revenue_minor: i64,
    pub cogs_minor: i64,
    pub gross_profit_minor: i64,
    pub gross_margin_bp: i64,
    pub expenses_minor: i64,
    pub net_profit_minor: i64,
    pub expenses_by_category: Vec<ExpenseRow>,
}

/// الأرباح والخسائر: صافي المبيعات (بدون ضريبة وبعد المرتجعات) - تكلفة المباع = مجمل الربح، ثم - المصروفات = الصافي.
/// المصروفات الملغاة لا تدخل.
pub async fn profit_loss(pool: &SqlitePool, range: &Range) -> Result<ProfitLoss, AppError> {
    let (from, to) = range.bounds()?;
    let s = sales_report(pool, range).await?;
    let rows: Vec<(String, i64)> = sqlx::query_as(&format!(
        "SELECT c.name_ar, COALESCE(SUM(e.amount_minor), 0) FROM expenses e JOIN expense_categories c ON c.id = e.category_id \
         WHERE e.voided_at IS NULL AND {} GROUP BY c.id ORDER BY SUM(e.amount_minor) DESC, c.name_ar",
        in_range("e.created_at")
    ))
    .bind(&from)
    .bind(&to)
    .fetch_all(pool)
    .await?;
    let expenses_by_category: Vec<ExpenseRow> = rows.into_iter().map(|(category, amount_minor)| ExpenseRow { category, amount_minor }).collect();
    let expenses_minor: i64 = expenses_by_category.iter().map(|e| e.amount_minor).sum();
    let gross = s.revenue_minor - s.cogs_minor;
    Ok(ProfitLoss {
        revenue_minor: s.revenue_minor,
        cogs_minor: s.cogs_minor,
        gross_profit_minor: gross,
        gross_margin_bp: margin_bp(gross, s.revenue_minor),
        expenses_minor,
        net_profit_minor: gross - expenses_minor,
        expenses_by_category,
    })
}
