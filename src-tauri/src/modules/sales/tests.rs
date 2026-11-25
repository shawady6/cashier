use sqlx::SqlitePool;

use super::*;
use crate::database::test_support::memory_pool;
use crate::modules::inventory::{adjust, AdjustInput};
use crate::modules::settings;
use crate::modules::test_helpers::*;

fn item(pid: &str, qty: &str) -> CartItemInput {
    CartItemInput { product_id: pid.into(), variant_id: None, unit_id: None, qty: qty.into(), discount: None }
}

fn pay(method: &str, amount: &str) -> PaymentInput {
    PaymentInput { method: method.into(), amount: amount.into(), reference: String::new() }
}

fn cart(items: Vec<CartItemInput>, payments: Vec<PaymentInput>) -> CartInput {
    CartInput { items, invoice_discount: None, payments, client_request_id: None, note: String::new(), customer_id: None }
}

async fn scalar(pool: &SqlitePool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

async fn on_hand(pool: &SqlitePool, pid: &str) -> (i64, i64) {
    sqlx::query_as("SELECT qty_milli, value_minor FROM stock_balances WHERE product_id = ?1")
        .bind(pid)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn successful_sale_deducts_stock_and_records_everything() {
    let pool = memory_pool().await;
    let p = product(&pool, "سكر", "10", "15").await;
    stock(&pool, &p, "10", "10").await;

    let r = checkout(&pool, &cart(vec![item(&p, "3")], vec![pay("cash", "50")])).await.unwrap();
    assert_eq!(r.total, 4500);
    assert_eq!(r.change, 500);
    assert_eq!(r.invoice_no, "INV-000001");

    assert_eq!(on_hand(&pool, &p).await, (7000, 7000)); // 7 × 10 جنيه
    assert_eq!(scalar(&pool, "SELECT cost_minor FROM sales").await, 3000);
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM sale_items").await, 1);
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM stock_movements WHERE movement_type='sale'").await, 1);
    // النقدية الصافية = المدفوع - الفكة
    assert_eq!(scalar(&pool, "SELECT amount_minor FROM cash_transactions WHERE kind='sale'").await, 4500);
    // الربح على التكلفة الفعلية: 4500 - 3000
    assert_eq!(scalar(&pool, "SELECT total_minor - cost_minor FROM sales").await, 1500);
}

#[tokio::test]
async fn profit_uses_weighted_average_not_latest_purchase_price() {
    let pool = memory_pool().await;
    let p = product(&pool, "زيت", "99", "30").await; // سعر شراء حالي مضلل
    stock(&pool, &p, "10", "10").await;
    stock(&pool, &p, "10", "20").await; // متوسط 15
    checkout(&pool, &cart(vec![item(&p, "4")], vec![pay("cash", "200")])).await.unwrap();
    assert_eq!(scalar(&pool, "SELECT cost_minor FROM sales").await, 6000); // 4 × 15
}

#[tokio::test]
async fn insufficient_stock_rolls_back_everything() {
    let pool = memory_pool().await;
    let a = product(&pool, "أ", "10", "15").await;
    let b = product(&pool, "ب", "10", "15").await;
    stock(&pool, &a, "10", "10").await;
    stock(&pool, &b, "1", "10").await;

    let err = checkout(&pool, &cart(vec![item(&a, "2"), item(&b, "5")], vec![pay("cash", "500")])).await.unwrap_err();
    assert!(err.to_string().contains("ب"), "{err}");
    // لا بقايا: لا فاتورة ولا حركة بيع ولا خصم من المنتج الأول، والرقم التسلسلي لم يُستهلك
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM sales").await, 0);
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM stock_movements WHERE movement_type='sale'").await, 0);
    assert_eq!(on_hand(&pool, &a).await.0, 10_000);
    assert_eq!(scalar(&pool, "SELECT next_value FROM sequences WHERE name='invoice'").await, 1);
}

#[tokio::test]
async fn same_product_in_two_lines_is_checked_against_total_stock() {
    let pool = memory_pool().await;
    let p = product(&pool, "أ", "10", "15").await;
    stock(&pool, &p, "5", "10").await;
    let c = cart(vec![item(&p, "3"), item(&p, "3")], vec![pay("cash", "500")]);
    assert!(checkout(&pool, &c).await.is_err());
    assert_eq!(on_hand(&pool, &p).await.0, 5000);
}

#[tokio::test]
async fn negative_stock_sale_works_only_when_enabled() {
    let pool = memory_pool().await;
    let p = product(&pool, "أ", "10", "15").await;
    let c = cart(vec![item(&p, "2")], vec![pay("cash", "30")]);
    assert!(checkout(&pool, &c).await.is_err());
    settings::set(&pool, "allow_negative_stock", "1").await.unwrap();
    checkout(&pool, &c).await.unwrap();
    assert_eq!(on_hand(&pool, &p).await.0, -2000);
    assert_eq!(scalar(&pool, "SELECT cost_minor FROM sales").await, 2000); // سعر الشراء الاحتياطي
}

#[tokio::test]
async fn split_payment_cash_and_card() {
    let pool = memory_pool().await;
    let p = product(&pool, "أ", "10", "50").await;
    stock(&pool, &p, "10", "10").await;
    checkout(&pool, &cart(vec![item(&p, "2")], vec![pay("card", "40"), pay("cash", "60")])).await.unwrap();
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM payments").await, 2);
    assert_eq!(scalar(&pool, "SELECT change_minor FROM sales").await, 0);
    assert_eq!(scalar(&pool, "SELECT amount_minor FROM cash_transactions WHERE method='card'").await, 4000);
    assert_eq!(scalar(&pool, "SELECT amount_minor FROM cash_transactions WHERE method='cash'").await, 6000);
}

#[tokio::test]
async fn underpayment_and_card_overpayment_are_rejected() {
    let pool = memory_pool().await;
    let p = product(&pool, "أ", "10", "50").await;
    stock(&pool, &p, "10", "10").await;
    assert!(checkout(&pool, &cart(vec![item(&p, "2")], vec![pay("cash", "50")])).await.is_err()); // دفع جزئي
    assert!(checkout(&pool, &cart(vec![item(&p, "1")], vec![pay("card", "60")])).await.is_err()); // كارت أكبر
    assert!(checkout(&pool, &cart(vec![item(&p, "1")], vec![pay("bitcoin", "50")])).await.is_err());
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM sales").await, 0);
    assert_eq!(on_hand(&pool, &p).await.0, 10_000);
}

#[tokio::test]
async fn item_and_invoice_discounts_sum_exactly() {
    let pool = memory_pool().await;
    let a = product(&pool, "أ", "1", "33.33").await;
    let b = product(&pool, "ب", "1", "10").await;
    stock(&pool, &a, "10", "1").await;
    stock(&pool, &b, "10", "1").await;

    let mut c = cart(vec![item(&a, "3"), item(&b, "1")], vec![]);
    c.items[0].discount = Some(DiscountInput { kind: "percent".into(), value: "10".into() });
    c.invoice_discount = Some(DiscountInput { kind: "amount".into(), value: "5.55".into() });
    let q = price_cart(&pool, &c).await.unwrap();
    // 3 × 33.33 = 99.99 ، خصم 10% = 10.00 → 89.99 ، + 10.00 = 99.99 ، - 5.55 = 94.44
    assert_eq!(q.subtotal, 10_999);
    assert_eq!(q.item_discount, 1000);
    assert_eq!(q.invoice_discount, 555);
    assert_eq!(q.total, 9444);
    assert_eq!(q.lines.iter().map(|l| l.invoice_discount).sum::<i64>(), 555);
    assert_eq!(q.lines.iter().map(|l| l.total).sum::<i64>(), q.total);

    // الخصم أكبر من الفاتورة مرفوض
    c.invoice_discount = Some(DiscountInput { kind: "amount".into(), value: "500".into() });
    assert!(price_cart(&pool, &c).await.is_err());
}

#[tokio::test]
async fn inclusive_tax_is_computed_from_settings() {
    let pool = memory_pool().await;
    settings::set(&pool, "tax_rate_bp", "1400").await.unwrap();
    let p = product(&pool, "أ", "1", "114").await;
    let q = price_cart(&pool, &cart(vec![item(&p, "1")], vec![])).await.unwrap();
    assert_eq!(q.total, 11_400);
    assert_eq!(q.tax, 1400); // الضريبة داخل السعر وليست إضافة عليه
}

#[tokio::test]
async fn duplicate_request_id_does_not_create_second_invoice_or_deduct_twice() {
    let pool = memory_pool().await;
    let p = product(&pool, "أ", "10", "15").await;
    stock(&pool, &p, "10", "10").await;
    let mut c = cart(vec![item(&p, "2")], vec![pay("cash", "30")]);
    c.client_request_id = Some("req-1".into());
    let first = checkout(&pool, &c).await.unwrap();
    let second = checkout(&pool, &c).await.unwrap();
    assert!(!first.duplicate && second.duplicate);
    assert_eq!(first.sale_id, second.sale_id);
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM sales").await, 1);
    assert_eq!(on_hand(&pool, &p).await.0, 8000);
}

#[tokio::test]
async fn carton_sale_deducts_twelve_base_pieces() {
    let pool = memory_pool().await;
    let p = product_with_carton(&pool, "مياه", "5", "10").await;
    stock(&pool, &p, "100", "5").await;
    let mut it = item(&p, "2");
    it.unit_id = Some(CARTON.into());
    let r = checkout(&pool, &cart(vec![it], vec![pay("cash", "300")])).await.unwrap();
    assert_eq!(r.total, 24_000); // 2 كرتونة × (12 × 10 جنيه)
    assert_eq!(on_hand(&pool, &p).await.0, 76_000); // 100 - 24 قطعة
}

#[tokio::test]
async fn weighted_decimal_quantity_sale() {
    let pool = memory_pool().await;
    let p = product_kg(&pool, "جبنة", "60", "80").await;
    stock(&pool, &p, "10", "60").await;
    let r = checkout(&pool, &cart(vec![item(&p, "0.5")], vec![pay("cash", "40")])).await.unwrap();
    assert_eq!(r.total, 4000);
    assert_eq!(on_hand(&pool, &p).await, (9500, 57_000));
    // القطعة لا تقبل كسورًا
    let piece = product(&pool, "قطعة", "1", "2").await;
    assert!(price_cart(&pool, &cart(vec![item(&piece, "0.5")], vec![])).await.is_err());
}

#[tokio::test]
async fn archived_inactive_or_unknown_products_cannot_be_sold() {
    let pool = memory_pool().await;
    let p = product(&pool, "أ", "1", "2").await;
    stock(&pool, &p, "10", "1").await;
    crate::modules::products::set_archived(&pool, &p, true).await.unwrap();
    assert!(price_cart(&pool, &cart(vec![item(&p, "1")], vec![])).await.is_err());
    assert!(price_cart(&pool, &cart(vec![item("nope", "1")], vec![])).await.is_err());
    assert!(checkout(&pool, &cart(vec![], vec![])).await.is_err());
}

#[tokio::test]
async fn price_comes_from_database_after_price_update() {
    let pool = memory_pool().await;
    let p = product(&pool, "أ", "1", "10").await;
    let mut edit = base_input("أ", "1", "12");
    edit.id = Some(p.clone());
    crate::modules::products::save(&pool, &edit).await.unwrap();
    let q = price_cart(&pool, &cart(vec![item(&p, "1")], vec![])).await.unwrap();
    assert_eq!(q.total, 1200);
}

#[tokio::test]
async fn invoices_are_immutable_and_keep_historical_snapshot() {
    let pool = memory_pool().await;
    let p = product(&pool, "اسم قديم", "1", "10").await;
    stock(&pool, &p, "5", "1").await;
    let r = checkout(&pool, &cart(vec![item(&p, "1")], vec![pay("cash", "10")])).await.unwrap();

    // تعديل المنتج لاحقًا لا يغيّر الفاتورة
    let mut edit = base_input("اسم جديد", "1", "99");
    edit.id = Some(p.clone());
    crate::modules::products::save(&pool, &edit).await.unwrap();
    let d = history::get(&pool, &r.sale_id).await.unwrap();
    assert_eq!(d.items[0].name_ar, "اسم قديم");
    assert_eq!(d.items[0].unit_price_minor, 1000);

    // التعديل والحذف مرفوضان من قاعدة البيانات نفسها
    for sql in [
        "UPDATE sales SET total_minor = 1",
        "DELETE FROM sales",
        "UPDATE sale_items SET total_minor = 1",
        "DELETE FROM sale_items",
        "UPDATE payments SET amount_minor = 1",
        "DELETE FROM payments",
    ] {
        assert!(sqlx::query(sql).execute(&pool).await.is_err(), "{sql} should be blocked");
    }
}

#[tokio::test]
async fn partial_then_full_return_restores_stock_at_original_cost_and_sums_exactly() {
    let pool = memory_pool().await;
    let p = product(&pool, "أ", "7", "10").await;
    stock(&pool, &p, "10", "7").await;
    let mut c = cart(vec![item(&p, "3")], vec![pay("cash", "100")]);
    c.invoice_discount = Some(DiscountInput { kind: "amount".into(), value: "1".into() });
    let sale = checkout(&pool, &c).await.unwrap();
    assert_eq!(sale.total, 2900);
    let d = history::get(&pool, &sale.sale_id).await.unwrap();
    let line = d.items[0].id.clone();
    assert_eq!(on_hand(&pool, &p).await, (7000, 4900));

    let ret = |qty: &str| ReturnInput {
        sale_id: sale.sale_id.clone(),
        items: vec![ReturnItemInput { sale_item_id: line.clone(), qty: qty.into() }],
        reason: "عيب".into(),
        refund_method: "cash".into(),
    };
    let r1 = create_return(&pool, &ret("1")).await.unwrap();
    let r2 = create_return(&pool, &ret("2")).await.unwrap();
    assert_eq!(r1.refund + r2.refund, 2900); // مجموع المرتجعات = قيمة البند بالضبط
    assert_eq!(on_hand(&pool, &p).await, (10_000, 7000)); // رجع بالتكلفة الأصلية
    assert_eq!(scalar(&pool, "SELECT SUM(amount_minor) FROM cash_transactions WHERE kind='sale_return'").await, -2900);
    assert_eq!(r1.return_no, "RET-000001");

    // لا يمكن إرجاع أكثر من المباع
    assert!(create_return(&pool, &ret("1")).await.is_err());
    // الفاتورة الأصلية لم تتغير
    assert_eq!(history::get(&pool, &sale.sale_id).await.unwrap().total_minor, 2900);
    assert_eq!(history::get(&pool, &sale.sale_id).await.unwrap().returns.len(), 2);
}

#[tokio::test]
async fn over_return_and_foreign_items_are_rejected_without_side_effects() {
    let pool = memory_pool().await;
    let p = product(&pool, "أ", "7", "10").await;
    stock(&pool, &p, "10", "7").await;
    let s1 = checkout(&pool, &cart(vec![item(&p, "2")], vec![pay("cash", "20")])).await.unwrap();
    let s2 = checkout(&pool, &cart(vec![item(&p, "1")], vec![pay("cash", "10")])).await.unwrap();
    let item_of_s2 = history::get(&pool, &s2.sale_id).await.unwrap().items[0].id.clone();
    let item_of_s1 = history::get(&pool, &s1.sale_id).await.unwrap().items[0].id.clone();

    let foreign = ReturnInput {
        sale_id: s1.sale_id.clone(),
        items: vec![ReturnItemInput { sale_item_id: item_of_s2, qty: "1".into() }],
        reason: String::new(),
        refund_method: "cash".into(),
    };
    assert!(create_return(&pool, &foreign).await.is_err());

    let too_many = ReturnInput {
        sale_id: s1.sale_id.clone(),
        items: vec![ReturnItemInput { sale_item_id: item_of_s1, qty: "3".into() }],
        reason: String::new(),
        refund_method: "cash".into(),
    };
    assert!(create_return(&pool, &too_many).await.is_err());
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM sales_returns").await, 0);
    assert_eq!(scalar(&pool, "SELECT next_value FROM sequences WHERE name='return'").await, 1);
    assert_eq!(on_hand(&pool, &p).await.0, 7000);
}

#[tokio::test]
async fn returning_carton_sale_restores_base_pieces() {
    let pool = memory_pool().await;
    let p = product_with_carton(&pool, "مياه", "5", "10").await;
    stock(&pool, &p, "100", "5").await;
    let mut it = item(&p, "2");
    it.unit_id = Some(CARTON.into());
    let sale = checkout(&pool, &cart(vec![it], vec![pay("cash", "300")])).await.unwrap();
    let line = history::get(&pool, &sale.sale_id).await.unwrap().items[0].id.clone();
    create_return(
        &pool,
        &ReturnInput {
            sale_id: sale.sale_id,
            items: vec![ReturnItemInput { sale_item_id: line, qty: "1".into() }],
            reason: String::new(),
            refund_method: "cash".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(on_hand(&pool, &p).await.0, 88_000); // 76 + 12
}

#[tokio::test]
async fn invoice_history_filters() {
    let pool = memory_pool().await;
    let p = product(&pool, "أ", "1", "10").await;
    stock(&pool, &p, "50", "1").await;
    for _ in 0..3 {
        checkout(&pool, &cart(vec![item(&p, "1")], vec![pay("cash", "10")])).await.unwrap();
    }
    let all = history::list(&pool, &SaleFilter::default()).await.unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].invoice_no, "INV-000003"); // الأحدث أولًا
    let one = history::list(&pool, &SaleFilter { query: Some("000002".into()), ..Default::default() }).await.unwrap();
    assert_eq!(one.len(), 1);
    let today = chrono_free_today(&pool).await;
    let ranged = history::list(&pool, &SaleFilter { date_from: Some(today.clone()), date_to: Some(today), ..Default::default() }).await.unwrap();
    assert_eq!(ranged.len(), 3);
    let none = history::list(&pool, &SaleFilter { date_from: Some("2001-01-01".into()), date_to: Some("2001-01-02".into()), ..Default::default() }).await.unwrap();
    assert!(none.is_empty());
}

async fn chrono_free_today(pool: &SqlitePool) -> String {
    sqlx::query_scalar("SELECT date('now','localtime')").fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn suspend_and_resume_roundtrip_once() {
    let pool = memory_pool().await;
    let id = suspended::suspend(&pool, "عميل 1", r#"{"items":[{"productId":"x","qty":"2"}]}"#).await.unwrap();
    assert_eq!(suspended::list(&pool).await.unwrap().len(), 1);
    let row = suspended::resume(&pool, &id).await.unwrap();
    assert!(row.payload.contains("productId"));
    assert!(suspended::resume(&pool, &id).await.is_err()); // لا تُسترجع مرتين
    assert!(suspended::suspend(&pool, "x", "not json").await.is_err());
}

#[tokio::test]
async fn stock_adjustment_helper_roundtrip_via_inventory_module() {
    let pool = memory_pool().await;
    let p = product(&pool, "أ", "10", "15").await;
    stock(&pool, &p, "3", "10").await;
    adjust(
        &pool,
        &AdjustInput { product_id: p.clone(), variant_id: None, direction: "decrease".into(), qty: "1".into(), unit_cost: None, note: "تالف".into() },
    )
    .await
    .unwrap();
    assert_eq!(on_hand(&pool, &p).await, (2000, 2000));
}
