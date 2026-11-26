//! التقارير: مبيعات، أرباح على التكلفة الفعلية، مشتريات، أرباح وخسائر، ديون، مخزون، وتصدير CSV.
//! كل الأرقام تُحسب في Rust بأعداد صحيحة (قروش). الأرباح بعد خصم الضريبة وعلى تكلفة المخزون المباع فعليًا،
//! والمرتجعات تُخصم في يوم حدوثها (قيمتها وتكلفتها).

pub mod csv;
mod debts;
mod pnl;
mod purchases;
mod sales;

#[cfg(test)]
mod tests;

use serde::Deserialize;

use crate::app_core::error::AppError;

pub use debts::{debts_summary, inventory_summary, DebtsSummary, InventorySummary};
pub use pnl::{profit_loss, ProfitLoss};
pub use purchases::{purchases_report, PurchasesReport};
pub use sales::{product_profit, sales_report, ProductProfitRow, SalesReport};

/// نطاق تواريخ محلية YYYY-MM-DD (شاملة الطرفين). الفارغ = بلا حد.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

fn valid_date(s: &str) -> bool {
    s.len() == 10 && s.chars().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() })
}

impl Range {
    pub fn bounds(&self) -> Result<(Option<String>, Option<String>), AppError> {
        let clean = |v: &Option<String>| -> Result<Option<String>, AppError> {
            match v.as_deref().map(str::trim) {
                None | Some("") => Ok(None),
                Some(d) if valid_date(d) => Ok(Some(d.to_string())),
                Some(_) => Err(AppError::Validation("تاريخ غير صالح".into())),
            }
        };
        let (from, to) = (clean(&self.date_from)?, clean(&self.date_to)?);
        if let (Some(f), Some(t)) = (&from, &to) {
            if f > t {
                return Err(AppError::Validation("تاريخ البداية بعد تاريخ النهاية".into()));
            }
        }
        Ok((from, to))
    }
}

/// شرط التاريخ لعمود زمني، بمعاملين ?1 (من) و?2 (إلى).
pub(crate) fn in_range(col: &str) -> String {
    format!("(?1 IS NULL OR date({col}, 'localtime') >= ?1) AND (?2 IS NULL OR date({col}, 'localtime') <= ?2)")
}

/// هامش الربح بالنقطة الأساسية (10000 = 100%) = الربح ÷ الإيراد.
pub(crate) fn margin_bp(profit: i64, revenue: i64) -> i64 {
    if revenue <= 0 {
        0
    } else {
        crate::app_core::money::div_round(i128::from(profit) * 10_000, i128::from(revenue)) as i64
    }
}
