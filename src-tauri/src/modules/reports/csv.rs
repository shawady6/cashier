//! تصدير التقارير CSV (UTF-8) بأرقام عشرية. الفاصلة "," وأقواس الاقتباس للحقول التي تحتاجها.

use sqlx::SqlitePool;

use super::{debts_summary, product_profit, profit_loss, purchases_report, sales_report, Range};
use crate::app_core::error::AppError;
use crate::app_core::money::{format_milli, format_minor};

fn esc(v: &str) -> String {
    // منع تنفيذ صيغ Excel من نص مُدخل: أي حقل يبدأ برمز صيغة يُسبق بفاصلة عليا
    let v = if v.starts_with(['=', '+', '-', '@']) && v.parse::<f64>().is_err() { format!("'{v}") } else { v.to_string() };
    if v.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v
    }
}

fn render(header: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut out = String::new();
    out.push_str(&header.iter().map(|h| esc(h)).collect::<Vec<_>>().join(","));
    for r in rows {
        out.push_str("\r\n");
        out.push_str(&r.iter().map(|c| esc(c)).collect::<Vec<_>>().join(","));
    }
    out.push_str("\r\n");
    out
}

pub async fn export_csv(pool: &SqlitePool, kind: &str, range: &Range) -> Result<String, AppError> {
    let m = format_minor;
    Ok(match kind {
        "sales_daily" => {
            let r = sales_report(pool, range).await?;
            render(
                &["اليوم", "عدد الفواتير", "الإجمالي", "الضريبة", "المرتجعات", "الإيراد الصافي", "تكلفة المباع", "الربح"],
                r.by_day
                    .iter()
                    .map(|d| vec![d.day.clone(), d.invoices.to_string(), m(d.total_minor), m(d.tax_minor), m(d.returns_minor), m(d.revenue_minor), m(d.cogs_minor), m(d.profit_minor)])
                    .collect(),
            )
        }
        "product_profit" => render(
            &["الصنف", "المتغير", "الكمية (وحدة أساسية)", "الإيراد", "التكلفة", "الربح", "الهامش %"],
            product_profit(pool, range, "profit", 1000)
                .await?
                .iter()
                .map(|p| vec![p.name_ar.clone(), p.variant_name.clone().unwrap_or_default(), format_milli(p.qty_base_milli), m(p.revenue_minor), m(p.cost_minor), m(p.profit_minor), m(p.margin_bp)])
                .collect(),
        ),
        "purchases_suppliers" => render(
            &["المورد", "عدد الفواتير", "إجمالي المشتريات", "المرتجعات", "المدفوع"],
            purchases_report(pool, range)
                .await?
                .by_supplier
                .iter()
                .map(|s| vec![s.name_ar.clone(), s.invoices.to_string(), m(s.total_minor), m(s.returned_minor), m(s.paid_minor)])
                .collect(),
        ),
        "purchases_products" => render(
            &["الصنف", "المتغير", "الكمية (وحدة أساسية)", "القيمة"],
            purchases_report(pool, range)
                .await?
                .by_product
                .iter()
                .map(|p| vec![p.name_ar.clone(), p.variant_name.clone().unwrap_or_default(), format_milli(p.qty_base_milli), m(p.value_minor)])
                .collect(),
        ),
        "profit_loss" => {
            let p = profit_loss(pool, range).await?;
            let mut rows = vec![
                vec!["صافي المبيعات (بدون ضريبة)".into(), m(p.revenue_minor)],
                vec!["تكلفة المباع".into(), m(p.cogs_minor)],
                vec!["مجمل الربح".into(), m(p.gross_profit_minor)],
            ];
            rows.extend(p.expenses_by_category.iter().map(|e| vec![format!("مصروفات: {}", e.category), m(e.amount_minor)]));
            rows.push(vec!["إجمالي المصروفات".into(), m(p.expenses_minor)]);
            rows.push(vec!["صافي الربح".into(), m(p.net_profit_minor)]);
            render(&["البند", "القيمة"], rows)
        }
        "debts" => {
            let d = debts_summary(pool).await?;
            let mut rows: Vec<Vec<String>> = d.customers.iter().map(|c| vec!["عميل".into(), c.name_ar.clone(), c.phone.clone(), m(c.amount_minor)]).collect();
            rows.extend(d.suppliers.iter().map(|s| vec!["مورد".into(), s.name_ar.clone(), s.phone.clone(), m(s.amount_minor)]));
            render(&["النوع", "الاسم", "الهاتف", "المبلغ المستحق"], rows)
        }
        _ => return Err(AppError::Validation("نوع التقرير غير معروف".into())),
    })
}
