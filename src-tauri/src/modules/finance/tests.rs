use sqlx::SqlitePool;

use super::customers::{self, CustomerInput, CustomerPaymentInput};
use super::expenses::{self, ExpenseInput};
use super::sessions::{self, CashEntryInput};
use crate::database::test_support::memory_pool;
use crate::modules::sales::{self, CartInput, CartItemInput, PaymentInput, ReturnInput, ReturnItemInput};
use crate::modules::test_helpers::*;

async fn scalar(pool: &SqlitePool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

async fn customer(pool: &SqlitePool, name: &str, limit: &str) -> String {
    customers::save(
        pool,
        &CustomerInput { id: None, name_ar: name.into(), phone: String::new(), address: String::new(), notes: String::new(), credit_limit: limit.into(), is_active: true },
    )
    .await
    .unwrap()
}

fn cart(pid: &str, qty: &str, pay: &str, cust: Option<&str>) -> CartInput {
    CartInput {
        items: vec![CartItemInput { product_id: pid.into(), variant_id: None, unit_id: None, qty: qty.into(), discount: None }],
        invoice_discount: None,
        payments: if pay.is_empty() { vec![] } else { vec![PaymentInput { method: "cash".into(), amount: pay.into(), reference: String::new() }] },
        client_request_id: None,
        note: String::new(),
        customer_id: cust.map(String::from),
    }
}

async fn setup() -> (SqlitePool, String) {
    let pool = memory_pool().await;
    let p = product(&pool, "سكر", "10", "15").await;
    stock(&pool, &p, "100", "10").await;
    (pool, p)
}

#[tokio::test]
async fn session_expected_cash_and_difference() {
    let (pool, p) = setup().await;
    let sid = sessions::open(&pool, "100", "").await.unwrap();
    assert!(sessions::open(&pool, "50", "").await.is_err()); // وردية واحدة فقط
    sales::checkout(&pool, &cart(&p, "2", "30", None)).await.unwrap(); // +30
    expenses::create(&pool, &ExpenseInput { category_id: "e1a00000-0000-4000-8000-000000000001".into(), amount: "10".into(), method: "cash".into(), description: String::new() }).await.unwrap(); // -10
    sessions::add_entry(&pool, &CashEntryInput { direction: "out".into(), amount: "5".into(), reason: "فكة".into() }).await.unwrap();
    let cur = sessions::current(&pool).await.unwrap().unwrap();
    assert_eq!(cur.expected_cash_minor, 10_000 + 3_000 - 1_000 - 500);
    let closed = sessions::close(&pool, &sid, "112", "").await.unwrap();
    assert_eq!(closed.difference_minor, Some(11_200 - 11_500));
    assert!(sessions::close(&pool, &sid, "112", "").await.is_err());
    assert!(sqlx::query("UPDATE cash_sessions SET note='x'").execute(&pool).await.is_err());
}

#[tokio::test]
async fn require_session_setting_blocks_cash_ops() {
    let (pool, p) = setup().await;
    sqlx::query("UPDATE settings SET setting_value='1' WHERE setting_key='require_cash_session'").execute(&pool).await.unwrap();
    assert!(sales::checkout(&pool, &cart(&p, "1", "20", None)).await.is_err());
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM sales").await, 0); // تراجع كامل
    sessions::open(&pool, "0", "").await.unwrap();
    sales::checkout(&pool, &cart(&p, "1", "20", None)).await.unwrap();
}

#[tokio::test]
async fn expense_void_reverses_cash_and_is_immutable() {
    let (pool, _) = setup().await;
    sessions::open(&pool, "100", "").await.unwrap();
    let id = expenses::create(&pool, &ExpenseInput { category_id: "e1a00000-0000-4000-8000-000000000002".into(), amount: "40".into(), method: "cash".into(), description: "فاتورة".into() }).await.unwrap();
    assert_eq!(sessions::current(&pool).await.unwrap().unwrap().expected_cash_minor, 6_000);
    assert!(sqlx::query("UPDATE expenses SET amount_minor = 1").execute(&pool).await.is_err());
    assert!(sqlx::query("DELETE FROM expenses").execute(&pool).await.is_err());
    assert!(expenses::void(&pool, &id, "").await.is_err());
    expenses::void(&pool, &id, "خطأ إدخال").await.unwrap();
    assert!(expenses::void(&pool, &id, "تاني").await.is_err());
    assert_eq!(sessions::current(&pool).await.unwrap().unwrap().expected_cash_minor, 10_000);
    let r = expenses::list(&pool, &Default::default()).await.unwrap();
    assert_eq!(r.total_minor, 0);
    assert_eq!(r.rows.len(), 1);
}

#[tokio::test]
async fn credit_sale_requires_customer_and_respects_limit() {
    let (pool, p) = setup().await;
    assert!(sales::checkout(&pool, &cart(&p, "2", "10", None)).await.is_err()); // آجل بدون عميل
    let c = customer(&pool, "أحمد", "50").await;
    // 2 × 15 = 30، مدفوع 10 → آجل 20
    let r = sales::checkout(&pool, &cart(&p, "2", "10", Some(&c))).await.unwrap();
    assert_eq!(r.total, 3000);
    assert_eq!(scalar(&pool, "SELECT paid_minor FROM sales").await, 1000);
    let d = sales::history::get(&pool, &r.sale_id).await.unwrap();
    assert_eq!(d.payment_status, "partial");
    assert_eq!(d.due_minor, 2000);
    assert_eq!(customers::list(&pool).await.unwrap()[0].balance_minor, 2000);
    // الحد 50: آجل 20 + 40 = 60 يتجاوز
    assert!(sales::checkout(&pool, &cart(&p, "3", "5", Some(&c))).await.is_err());
    // المخزون لم يتأثر بالمحاولة الفاشلة
    let qty: i64 = sqlx::query_scalar("SELECT qty_milli FROM stock_balances").fetch_one(&pool).await.unwrap();
    assert_eq!(qty, 98_000);
    // بيع كامل الآجل
    sales::checkout(&pool, &cart(&p, "1", "", Some(&c))).await.unwrap();
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM sales WHERE payment_status='unpaid'").await, 1);
    assert_eq!(customers::list(&pool).await.unwrap()[0].balance_minor, 3500);
}

#[tokio::test]
async fn customer_payment_reduces_balance_and_cannot_overpay() {
    let (pool, p) = setup().await;
    let c = customer(&pool, "منى", "").await;
    sales::checkout(&pool, &cart(&p, "2", "", Some(&c))).await.unwrap(); // 30 آجل
    let pay = |a: &str| CustomerPaymentInput { customer_id: c.clone(), method: "cash".into(), amount: a.into(), note: String::new() };
    assert!(customers::take_payment(&pool, &pay("31")).await.is_err());
    customers::take_payment(&pool, &pay("12.50")).await.unwrap();
    assert_eq!(customers::list(&pool).await.unwrap()[0].balance_minor, 1750);
    assert_eq!(scalar(&pool, "SELECT amount_minor FROM cash_transactions WHERE kind='customer_payment'").await, 1250);
    let st = customers::statement(&pool, &c).await.unwrap();
    assert_eq!(st.rows.len(), 2);
    assert_eq!(st.rows.last().unwrap().balance_minor, 1750);
    assert!(sqlx::query("DELETE FROM customer_payments").execute(&pool).await.is_err());
}

#[tokio::test]
async fn return_on_credit_sale_reduces_debt_first() {
    let (pool, p) = setup().await;
    let c = customer(&pool, "كريم", "").await;
    // 4 × 15 = 60، مدفوع 20 → دين 40
    let r = sales::checkout(&pool, &cart(&p, "4", "20", Some(&c))).await.unwrap();
    let item: String = sqlx::query_scalar("SELECT id FROM sale_items").fetch_one(&pool).await.unwrap();
    // إرجاع 3 = 45: 40 تُخصم من الدين و5 تُردّ نقدًا
    let ret = sales::create_return(&pool, &ReturnInput { sale_id: r.sale_id.clone(), items: vec![ReturnItemInput { sale_item_id: item, qty: "3".into() }], reason: String::new(), refund_method: "cash".into() }).await.unwrap();
    assert_eq!(ret.refund, 4500);
    assert_eq!(scalar(&pool, "SELECT credit_minor FROM sales_returns").await, 4000);
    assert_eq!(scalar(&pool, "SELECT amount_minor FROM cash_transactions WHERE kind='sale_return'").await, -500);
    assert_eq!(customers::list(&pool).await.unwrap()[0].balance_minor, 0);
}

#[tokio::test]
async fn cash_in_requires_open_session_and_reason() {
    let (pool, _) = setup().await;
    let e = |r: &str| CashEntryInput { direction: "in".into(), amount: "10".into(), reason: r.into() };
    assert!(sessions::add_entry(&pool, &e("x")).await.is_err());
    sessions::open(&pool, "0", "").await.unwrap();
    assert!(sessions::add_entry(&pool, &e("  ")).await.is_err());
    sessions::add_entry(&pool, &e("إيداع")).await.unwrap();
    assert!(sessions::add_entry(&pool, &CashEntryInput { direction: "out".into(), amount: "11".into(), reason: "x".into() }).await.is_err());
}
