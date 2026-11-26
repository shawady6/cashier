use sqlx::SqlitePool;

use super::csv::export_csv;
use super::*;
use crate::app_core::money::{format_milli, format_minor};
use crate::database::test_support::memory_pool;
use crate::modules::finance::customers::{self, CustomerInput};
use crate::modules::finance::expenses::{self, ExpenseCategoryInput, ExpenseInput};
use crate::modules::purchasing::{purchases, returns, suppliers, PurchaseInput, PurchaseItemInput, PurchaseReturnInput, PurchaseReturnItemInput, SupplierInput, SupplierPaymentInput};
use crate::modules::sales::{self, CartInput, CartItemInput, DiscountInput, PaymentInput, ReturnInput, ReturnItemInput};
use crate::modules::settings;
use crate::modules::test_helpers::*;

const WH: &str = "c5e3a1b0-2d4f-4b6a-9c11-7a1e5d300001";

fn today() -> Range {
    Range::default()
}

fn cart(pid: &str, qty: &str, pay: &str, discount: Option<&str>, cust: Option<&str>) -> CartInput {
    CartInput {
        items: vec![CartItemInput { product_id: pid.into(), variant_id: None, unit_id: None, qty: qty.into(), discount: None }],
        invoice_discount: discount.map(|d| DiscountInput { kind: "amount".into(), value: d.into() }),
        payments: if pay.is_empty() { vec![] } else { vec![PaymentInput { method: "cash".into(), amount: pay.into(), reference: String::new() }] },
        client_request_id: None,
        note: String::new(),
        customer_id: cust.map(String::from),
    }
}

async fn sale_item_id(pool: &SqlitePool, sale_id: &str) -> String {
    sqlx::query_scalar("SELECT id FROM sale_items WHERE sale_id = ?1 ORDER BY line_no LIMIT 1").bind(sale_id).fetch_one(pool).await.unwrap()
}

async fn refund(pool: &SqlitePool, sale_id: &str, qty: &str) {
    let item = sale_item_id(pool, sale_id).await;
    sales::create_return(pool, &ReturnInput { sale_id: sale_id.into(), items: vec![ReturnItemInput { sale_item_id: item, qty: qty.into() }], reason: String::new(), refund_method: "cash".into() })
        .await
        .unwrap();
}

#[test]
fn money_and_qty_formatting() {
    assert_eq!(format_minor(12345), "123.45");
    assert_eq!(format_minor(5), "0.05");
    assert_eq!(format_minor(-5), "-0.05");
    assert_eq!(format_minor(0), "0.00");
    assert_eq!(format_milli(12_000), "12");
    assert_eq!(format_milli(2_500), "2.5");
    assert_eq!(format_milli(-1_250), "-1.25");
}

#[tokio::test]
async fn profit_uses_actual_cost_with_discount_and_returns() {
    let pool = memory_pool().await;
    let p = product(&pool, "سكر", "10", "15").await;
    stock(&pool, &p, "100", "10").await;
    let first = sales::checkout(&pool, &cart(&p, "10", "150", None, None)).await.unwrap();
    sales::checkout(&pool, &cart(&p, "5", "70", Some("5"), None)).await.unwrap();
    refund(&pool, &first.sale_id, "2").await;

    let r = sales_report(&pool, &today()).await.unwrap();
    assert_eq!((r.invoices, r.gross_minor, r.discounts_minor, r.tax_minor), (2, 22_500, 500, 0));
    assert_eq!((r.total_minor, r.returns_minor, r.net_sales_minor), (22_000, 3_000, 19_000));
    assert_eq!((r.revenue_minor, r.cogs_minor, r.profit_minor), (19_000, 13_000, 6_000)); // تكلفة 150 - 20 مرتجع
    assert_eq!(r.margin_bp, 3_158);
    assert_eq!(r.by_day.len(), 1);
    assert_eq!(r.by_day[0].profit_minor, 6_000);
    let cash = r.by_method.iter().find(|m| m.method == "cash").unwrap();
    assert_eq!((cash.received_minor, cash.refunded_minor), (22_000, 3_000));
}

#[tokio::test]
async fn tax_is_excluded_from_revenue_and_return_tax_is_proportional() {
    let pool = memory_pool().await;
    settings::set(&pool, "tax_rate_bp", "1400").await.unwrap();
    let p = product(&pool, "سكر", "10", "15").await;
    stock(&pool, &p, "100", "10").await;
    let s = sales::checkout(&pool, &cart(&p, "10", "150", None, None)).await.unwrap();
    let r = sales_report(&pool, &today()).await.unwrap();
    assert_eq!((r.total_minor, r.tax_minor), (15_000, 1_842)); // 15000 × 14 ÷ 114
    assert_eq!((r.revenue_minor, r.cogs_minor, r.profit_minor), (13_158, 10_000, 3_158));

    refund(&pool, &s.sale_id, "2").await;
    let r = sales_report(&pool, &today()).await.unwrap();
    // مرتجع 30.00 فيه ضريبة 3.68 (بنسبة البند الأصلي) = إيراد 26.32، وتكلفته 20.00
    assert_eq!((r.revenue_minor, r.cogs_minor, r.profit_minor), (10_526, 8_000, 2_526));
}

#[tokio::test]
async fn product_profit_ranks_and_nets_returns() {
    let pool = memory_pool().await;
    let a = product(&pool, "أ", "10", "15").await;
    let b = product(&pool, "ب", "5", "20").await;
    stock(&pool, &a, "100", "10").await;
    stock(&pool, &b, "100", "5").await;
    let sa = sales::checkout(&pool, &cart(&a, "10", "150", None, None)).await.unwrap();
    sales::checkout(&pool, &cart(&b, "4", "80", None, None)).await.unwrap();
    refund(&pool, &sa.sale_id, "5").await;

    let rows = product_profit(&pool, &today(), "profit", 10).await.unwrap();
    assert_eq!(rows.len(), 2);
    // ب: إيراد 80 تكلفة 20 ربح 60. أ: (150-75) - (100-50) = 25
    assert_eq!((rows[0].name_ar.as_str(), rows[0].revenue_minor, rows[0].cost_minor, rows[0].profit_minor), ("ب", 8_000, 2_000, 6_000));
    assert_eq!((rows[1].name_ar.as_str(), rows[1].qty_base_milli, rows[1].revenue_minor, rows[1].profit_minor), ("أ", 5_000, 7_500, 2_500));
    assert_eq!(rows[0].margin_bp, 7_500);
    let by_qty = product_profit(&pool, &today(), "qty", 1).await.unwrap();
    assert_eq!(by_qty.len(), 1);
    assert_eq!(by_qty[0].name_ar, "أ");
}

async fn supplier(pool: &SqlitePool, name: &str) -> String {
    suppliers::save(pool, &SupplierInput { id: None, name_ar: name.into(), phone: "010".into(), address: String::new(), notes: String::new(), is_active: true }).await.unwrap()
}

async fn buy(pool: &SqlitePool, sup: &str, pid: &str, qty: &str, cost: &str) -> String {
    let input = PurchaseInput {
        id: None, supplier_id: sup.into(), warehouse_id: WH.into(), supplier_invoice_no: String::new(), note: String::new(), discount: None,
        items: vec![PurchaseItemInput { product_id: pid.into(), variant_id: None, unit_id: None, qty: qty.into(), unit_cost: cost.into(), factor: None, sale_price: None }],
    };
    let id = purchases::save_draft(pool, &input).await.unwrap();
    purchases::receive_purchase(pool, &id).await.unwrap();
    id
}

#[tokio::test]
async fn purchases_report_counts_received_only_with_returns_and_payments() {
    let pool = memory_pool().await;
    let p = product(&pool, "سكر", "10", "15").await;
    let s1 = supplier(&pool, "مورد 1").await;
    let s2 = supplier(&pool, "مورد 2").await;
    let inv = buy(&pool, &s1, &p, "10", "10").await; // 100
    buy(&pool, &s2, &p, "5", "12").await; // 60
    // مسودة لا تدخل التقرير
    let draft = PurchaseInput {
        id: None, supplier_id: s1.clone(), warehouse_id: WH.into(), supplier_invoice_no: String::new(), note: String::new(), discount: None,
        items: vec![PurchaseItemInput { product_id: p.clone(), variant_id: None, unit_id: None, qty: "100".into(), unit_cost: "10".into(), factor: None, sale_price: None }],
    };
    purchases::save_draft(&pool, &draft).await.unwrap();
    purchases::add_payment(&pool, &SupplierPaymentInput { purchase_id: inv.clone(), method: "cash".into(), amount: "40".into(), note: String::new() }).await.unwrap();
    let detail = purchases::get(&pool, &inv).await.unwrap();
    returns::create(&pool, &PurchaseReturnInput { purchase_id: inv, items: vec![PurchaseReturnItemInput { purchase_item_id: detail.items[0].id.clone(), qty: "2".into() }], note: String::new() }).await.unwrap();

    let r = purchases_report(&pool, &today()).await.unwrap();
    assert_eq!((r.invoices, r.total_minor, r.returned_minor, r.paid_minor), (2, 16_000, 2_000, 4_000));
    assert_eq!(r.by_supplier[0].name_ar, "مورد 1");
    assert_eq!((r.by_supplier[0].total_minor, r.by_supplier[0].returned_minor, r.by_supplier[0].paid_minor), (10_000, 2_000, 4_000));
    assert_eq!(r.by_product.len(), 1);
    assert_eq!((r.by_product[0].qty_base_milli, r.by_product[0].value_minor), (15_000, 16_000));
}

#[tokio::test]
async fn profit_loss_subtracts_active_expenses_only() {
    let pool = memory_pool().await;
    let p = product(&pool, "سكر", "10", "15").await;
    stock(&pool, &p, "100", "10").await;
    sales::checkout(&pool, &cart(&p, "10", "150", None, None)).await.unwrap();
    let cat = expenses::save_category(&pool, &ExpenseCategoryInput { id: None, name_ar: "تسويق".into(), is_active: true }).await.unwrap();
    expenses::create(&pool, &ExpenseInput { category_id: cat.clone(), amount: "20".into(), method: "cash".into(), description: String::new() }).await.unwrap();
    let voided = expenses::create(&pool, &ExpenseInput { category_id: cat, amount: "7".into(), method: "cash".into(), description: String::new() }).await.unwrap();
    expenses::void(&pool, &voided, "خطأ").await.unwrap();

    let r = profit_loss(&pool, &today()).await.unwrap();
    assert_eq!((r.revenue_minor, r.cogs_minor, r.gross_profit_minor), (15_000, 10_000, 5_000));
    assert_eq!((r.expenses_minor, r.net_profit_minor), (2_000, 3_000));
    assert_eq!(r.expenses_by_category.len(), 1);
    assert_eq!(r.gross_margin_bp, 3_333);
}

#[tokio::test]
async fn debts_and_inventory_summaries() {
    let pool = memory_pool().await;
    let p = product(&pool, "سكر", "10", "15").await;
    stock(&pool, &p, "100", "10").await;
    let c = customers::save(&pool, &CustomerInput { id: None, name_ar: "أحمد".into(), phone: String::new(), address: String::new(), notes: String::new(), credit_limit: "1000".into(), is_active: true }).await.unwrap();
    sales::checkout(&pool, &cart(&p, "10", "50", None, Some(&c))).await.unwrap(); // 150 مدفوع 50 ← دين 100
    let s = supplier(&pool, "مورد").await;
    buy(&pool, &s, &p, "10", "10").await; // 100 على المورد

    let d = debts_summary(&pool).await.unwrap();
    assert_eq!((d.receivables_minor, d.payables_minor), (10_000, 10_000));
    assert_eq!(d.customers[0].name_ar, "أحمد");

    let i = inventory_summary(&pool).await.unwrap();
    assert_eq!(i.items_in_stock, 1);
    assert_eq!(i.total_value_minor, 100_000); // 100 قطعة بمتوسط 10 جنيه
}

#[tokio::test]
async fn ranges_are_validated_and_filter_results() {
    let pool = memory_pool().await;
    let p = product(&pool, "سكر", "10", "15").await;
    stock(&pool, &p, "100", "10").await;
    sales::checkout(&pool, &cart(&p, "1", "15", None, None)).await.unwrap();
    let old = Range { date_from: Some("2000-01-01".into()), date_to: Some("2000-01-31".into()) };
    assert_eq!(sales_report(&pool, &old).await.unwrap().invoices, 0);
    let wide = Range { date_from: Some("2000-01-01".into()), date_to: Some("2999-12-31".into()) };
    assert_eq!(sales_report(&pool, &wide).await.unwrap().invoices, 1);
    assert!(sales_report(&pool, &Range { date_from: Some("2026-13-x".into()), date_to: None }).await.is_err());
    assert!(sales_report(&pool, &Range { date_from: Some("2026-05-02".into()), date_to: Some("2026-05-01".into()) }).await.is_err());
    assert!(sales_report(&pool, &Range { date_from: Some("'; DROP TABLE sales;--".into()), date_to: None }).await.is_err());
}

#[tokio::test]
async fn csv_export_has_decimal_money_escaping_and_unknown_kinds_fail() {
    let pool = memory_pool().await;
    let p = product(&pool, "=سكر, \"أبيض\"", "10", "15").await;
    stock(&pool, &p, "100", "10").await;
    sales::checkout(&pool, &cart(&p, "10", "150", None, None)).await.unwrap();
    let daily = export_csv(&pool, "sales_daily", &today()).await.unwrap();
    let lines: Vec<&str> = daily.trim_end().split("\r\n").collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("اليوم,"));
    assert!(lines[1].ends_with(",1,150.00,0.00,0.00,150.00,100.00,50.00"));
    let prof = export_csv(&pool, "product_profit", &today()).await.unwrap();
    assert!(prof.contains("\"'=سكر, \"\"أبيض\"\"\",,10,150.00,100.00,50.00,33.33"), "{prof}");
    assert!(export_csv(&pool, "nope", &today()).await.is_err());
}
