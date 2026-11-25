use sqlx::SqlitePool;

use super::*;
use crate::database::test_support::memory_pool;
use crate::modules::inventory::reports;
use crate::modules::sales::{self, CartInput, CartItemInput, DiscountInput};
use crate::modules::test_helpers::*;

const WH: &str = "c5e3a1b0-2d4f-4b6a-9c11-7a1e5d300001";

async fn supplier(pool: &SqlitePool, name: &str) -> String {
    suppliers::save(pool, &SupplierInput { id: None, name_ar: name.into(), phone: String::new(), address: String::new(), notes: String::new(), is_active: true })
        .await
        .unwrap()
}

fn line(pid: &str, qty: &str, cost: &str) -> PurchaseItemInput {
    PurchaseItemInput { product_id: pid.into(), variant_id: None, unit_id: None, qty: qty.into(), unit_cost: cost.into(), factor: None, sale_price: None }
}

fn input(sup: &str, items: Vec<PurchaseItemInput>) -> PurchaseInput {
    PurchaseInput { id: None, supplier_id: sup.into(), warehouse_id: WH.into(), supplier_invoice_no: "A-1".into(), note: String::new(), discount: None, items }
}

async fn on_hand(pool: &SqlitePool, pid: &str) -> (i64, i64) {
    sqlx::query_as("SELECT COALESCE(SUM(qty_milli),0), COALESCE(SUM(value_minor),0) FROM stock_balances WHERE product_id = ?1")
        .bind(pid)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn count(pool: &SqlitePool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn stock_increases_only_when_purchase_is_received() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    let p = product(&pool, "سكر", "1", "15").await;
    let id = purchases::save_draft(&pool, &input(&s, vec![line(&p, "10", "10")])).await.unwrap();
    assert_eq!(on_hand(&pool, &p).await, (0, 0)); // المسودة لا تغيّر المخزون
    purchases::receive_purchase(&pool, &id).await.unwrap();
    assert_eq!(on_hand(&pool, &p).await, (10_000, 10_000));
    // لا يمكن استلامها مرتين
    assert!(purchases::receive_purchase(&pool, &id).await.is_err());
    assert_eq!(on_hand(&pool, &p).await.0, 10_000);
    // سعر الشراء المرجعي تحدّث
    assert_eq!(crate::modules::products::get(&pool, &p).await.unwrap().purchase_price, 1000);
}

#[tokio::test]
async fn receiving_twice_gives_weighted_average_used_by_sales_profit() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    let p = product(&pool, "زيت", "1", "30").await;
    for cost in ["10", "20"] {
        let id = purchases::save_draft(&pool, &input(&s, vec![line(&p, "10", cost)])).await.unwrap();
        purchases::receive_purchase(&pool, &id).await.unwrap();
    }
    let cart = CartInput {
        items: vec![CartItemInput { product_id: p.clone(), variant_id: None, unit_id: None, qty: "4".into(), discount: None }],
        invoice_discount: None,
        payments: vec![sales::PaymentInput { method: "cash".into(), amount: "200".into(), reference: String::new() }],
        client_request_id: None,
        note: String::new(),
        customer_id: None,
    };
    sales::checkout(&pool, &cart).await.unwrap();
    assert_eq!(count(&pool, "SELECT cost_minor FROM sales").await, 6000); // 4 × متوسط 15
}

#[tokio::test]
async fn invoice_discount_reduces_stock_value_exactly() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    let a = product(&pool, "أ", "1", "5").await;
    let b = product(&pool, "ب", "1", "5").await;
    let mut i = input(&s, vec![line(&a, "3", "10"), line(&b, "1", "10.01")]);
    i.discount = Some(DiscountInput { kind: "amount".into(), value: "5".into() });
    let id = purchases::save_draft(&pool, &i).await.unwrap();
    let d = purchases::get(&pool, &id).await.unwrap();
    assert_eq!(d.subtotal_minor, 4001);
    assert_eq!(d.header.total_minor, 3501);
    assert_eq!(d.items.iter().map(|x| x.net_minor).sum::<i64>(), 3501); // التوزيع = الإجمالي بالضبط
    purchases::receive_purchase(&pool, &id).await.unwrap();
    let total_value = on_hand(&pool, &a).await.1 + on_hand(&pool, &b).await.1;
    assert_eq!(total_value, 3501);
}

#[tokio::test]
async fn carton_purchase_adds_twelve_base_pieces() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    let p = product_with_carton(&pool, "مياه", "1", "10").await;
    let mut it = line(&p, "2", "60");
    it.unit_id = Some(CARTON.into());
    let id = purchases::save_draft(&pool, &input(&s, vec![it])).await.unwrap();
    purchases::receive_purchase(&pool, &id).await.unwrap();
    assert_eq!(on_hand(&pool, &p).await, (24_000, 12_000)); // 24 قطعة بقيمة 120 جنيه
    assert_eq!(crate::modules::products::get(&pool, &p).await.unwrap().purchase_price, 500); // 5 جنيه للقطعة
}

#[tokio::test]
async fn draft_can_be_edited_but_received_cannot() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    let p = product(&pool, "سكر", "1", "15").await;
    let id = purchases::save_draft(&pool, &input(&s, vec![line(&p, "1", "10")])).await.unwrap();
    let mut edit = input(&s, vec![line(&p, "5", "11")]);
    edit.id = Some(id.clone());
    purchases::save_draft(&pool, &edit).await.unwrap();
    assert_eq!(purchases::get(&pool, &id).await.unwrap().header.total_minor, 5500);
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM purchase_items").await, 1);

    purchases::receive_purchase(&pool, &id).await.unwrap();
    assert!(purchases::save_draft(&pool, &edit).await.is_err());
    // وقاعدة البيانات نفسها ترفض التعديل والحذف
    for sql in ["UPDATE purchases SET total_minor = 1", "DELETE FROM purchases", "UPDATE purchase_items SET qty_milli = 1", "DELETE FROM purchase_items"] {
        assert!(sqlx::query(sql).execute(&pool).await.is_err(), "{sql}");
    }
}

#[tokio::test]
async fn cancelled_draft_never_touches_stock_and_cannot_be_received() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    let p = product(&pool, "سكر", "1", "15").await;
    let id = purchases::save_draft(&pool, &input(&s, vec![line(&p, "5", "10")])).await.unwrap();
    purchases::cancel_draft(&pool, &id).await.unwrap();
    assert!(purchases::receive_purchase(&pool, &id).await.is_err());
    assert_eq!(on_hand(&pool, &p).await, (0, 0));
    assert!(purchases::cancel_draft(&pool, &id).await.is_err());
}

#[tokio::test]
async fn invalid_purchases_are_rejected_without_side_effects() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    let p = product(&pool, "سكر", "1", "15").await;
    assert!(purchases::save_draft(&pool, &input(&s, vec![])).await.is_err());
    assert!(purchases::save_draft(&pool, &input(&s, vec![line(&p, "0", "10")])).await.is_err());
    assert!(purchases::save_draft(&pool, &input(&s, vec![line(&p, "1", "-5")])).await.is_err());
    assert!(purchases::save_draft(&pool, &input(&s, vec![line(&p, "1.5", "5")])).await.is_err()); // قطعة لا تقبل كسورًا
    assert!(purchases::save_draft(&pool, &input("nope", vec![line(&p, "1", "10")])).await.is_err());
    let mut bad_unit = line(&p, "1", "10");
    bad_unit.unit_id = Some(CARTON.into()); // الكرتونة غير معرّفة لهذا المنتج
    assert!(purchases::save_draft(&pool, &input(&s, vec![bad_unit])).await.is_err());
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM purchases").await, 0);
    assert_eq!(count(&pool, "SELECT next_value FROM sequences WHERE name='purchase'").await, 1);
}

#[tokio::test]
async fn supplier_balance_tracks_purchases_payments_and_returns() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    let p = product(&pool, "سكر", "1", "15").await;
    let id = purchases::save_draft(&pool, &input(&s, vec![line(&p, "10", "10")])).await.unwrap(); // 100
    let bal = |pool: SqlitePool, s: String| async move {
        suppliers::list(&pool).await.unwrap().into_iter().find(|x| x.id == s).unwrap().balance_minor
    };
    assert_eq!(bal(pool.clone(), s.clone()).await, 0); // المسودة لا تدخل الرصيد
    purchases::receive_purchase(&pool, &id).await.unwrap();
    assert_eq!(bal(pool.clone(), s.clone()).await, 10_000);

    let pay = |amount: &str| SupplierPaymentInput { purchase_id: id.clone(), method: "cash".into(), amount: amount.into(), note: String::new() };
    purchases::add_payment(&pool, &pay("30")).await.unwrap();
    assert_eq!(bal(pool.clone(), s.clone()).await, 7000);
    assert_eq!(count(&pool, "SELECT SUM(amount_minor) FROM cash_transactions WHERE kind='supplier_payment'").await, -3000);

    // مرتجع 2 قطعة = 20 جنيه
    let item = purchases::get(&pool, &id).await.unwrap().items[0].id.clone();
    let (no, credit) = returns::create(&pool, &PurchaseReturnInput {
        purchase_id: id.clone(),
        items: vec![PurchaseReturnItemInput { purchase_item_id: item, qty: "2".into() }],
        note: String::new(),
    })
    .await
    .unwrap();
    assert_eq!((no.as_str(), credit), ("PRT-000001", 2000));
    assert_eq!(bal(pool.clone(), s.clone()).await, 5000);
    assert_eq!(on_hand(&pool, &p).await, (8000, 8000));

    // لا يتجاوز المتبقي (100 - 20 - 30 = 50)
    assert!(purchases::add_payment(&pool, &pay("50.01")).await.is_err());
    purchases::add_payment(&pool, &pay("50")).await.unwrap();
    assert_eq!(bal(pool.clone(), s.clone()).await, 0);
    assert_eq!(purchases::get(&pool, &id).await.unwrap().outstanding_minor, 0);
    assert!(sqlx::query("DELETE FROM purchase_payments").execute(&pool).await.is_err());
}

#[tokio::test]
async fn payment_on_draft_or_bad_method_is_rejected() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    let p = product(&pool, "سكر", "1", "15").await;
    let id = purchases::save_draft(&pool, &input(&s, vec![line(&p, "1", "10")])).await.unwrap();
    let pay = |m: &str| SupplierPaymentInput { purchase_id: id.clone(), method: m.into(), amount: "5".into(), note: String::new() };
    assert!(purchases::add_payment(&pool, &pay("cash")).await.is_err());
    purchases::receive_purchase(&pool, &id).await.unwrap();
    assert!(purchases::add_payment(&pool, &pay("crypto")).await.is_err());
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM purchase_payments").await, 0);
}

#[tokio::test]
async fn partial_then_full_purchase_return_sums_exactly_and_over_return_fails() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    let p = product(&pool, "سكر", "1", "15").await;
    let mut i = input(&s, vec![line(&p, "3", "3.33")]);
    i.discount = Some(DiscountInput { kind: "amount".into(), value: "0.5".into() });
    let id = purchases::save_draft(&pool, &i).await.unwrap(); // 9.99 - 0.50 = 9.49
    purchases::receive_purchase(&pool, &id).await.unwrap();
    let item = purchases::get(&pool, &id).await.unwrap().items[0].id.clone();
    let ret = |q: &str| PurchaseReturnInput { purchase_id: id.clone(), items: vec![PurchaseReturnItemInput { purchase_item_id: item.clone(), qty: q.into() }], note: String::new() };
    let (_, c1) = returns::create(&pool, &ret("1")).await.unwrap();
    let (_, c2) = returns::create(&pool, &ret("2")).await.unwrap();
    assert_eq!(c1 + c2, 949); // مجموع المرتجعات = قيمة البند بعد الخصم
    assert_eq!(on_hand(&pool, &p).await, (0, 0)); // المخزون رجع صفر بدون فروق
    assert!(returns::create(&pool, &ret("1")).await.is_err());
}

#[tokio::test]
async fn purchase_return_blocked_when_stock_already_sold() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    let p = product(&pool, "سكر", "1", "15").await;
    let id = purchases::save_draft(&pool, &input(&s, vec![line(&p, "5", "10")])).await.unwrap();
    purchases::receive_purchase(&pool, &id).await.unwrap();
    stock_sale(&pool, &p, "4").await;
    let item = purchases::get(&pool, &id).await.unwrap().items[0].id.clone();
    let r = returns::create(&pool, &PurchaseReturnInput {
        purchase_id: id.clone(),
        items: vec![PurchaseReturnItemInput { purchase_item_id: item, qty: "3".into() }],
        note: String::new(),
    })
    .await;
    assert!(r.is_err()); // المتاح 1 فقط
    assert_eq!(count(&pool, "SELECT COUNT(*) FROM purchase_returns").await, 0);
    assert_eq!(on_hand(&pool, &p).await.0, 1000);
}

async fn stock_sale(pool: &SqlitePool, pid: &str, qty: &str) {
    let cart = CartInput {
        items: vec![CartItemInput { product_id: pid.into(), variant_id: None, unit_id: None, qty: qty.into(), discount: None }],
        invoice_discount: None,
        payments: vec![sales::PaymentInput { method: "cash".into(), amount: "1000".into(), reference: String::new() }],
        client_request_id: None,
        note: String::new(),
        customer_id: None,
    };
    sales::checkout(pool, &cart).await.unwrap();
}

#[tokio::test]
async fn supplier_names_are_unique_and_inactive_supplier_cannot_receive_new_drafts() {
    let pool = memory_pool().await;
    let s = supplier(&pool, "مورد").await;
    assert!(suppliers::save(&pool, &SupplierInput { id: None, name_ar: "مُورد".into(), phone: String::new(), address: String::new(), notes: String::new(), is_active: true }).await.is_err());
    suppliers::save(&pool, &SupplierInput { id: Some(s.clone()), name_ar: "مورد".into(), phone: "010".into(), address: String::new(), notes: String::new(), is_active: false }).await.unwrap();
    let p = product(&pool, "سكر", "1", "15").await;
    assert!(purchases::save_draft(&pool, &input(&s, vec![line(&p, "1", "10")])).await.is_err());
}

#[tokio::test]
async fn purchase_list_filters_and_movements_report() {
    let pool = memory_pool().await;
    let s1 = supplier(&pool, "أ").await;
    let s2 = supplier(&pool, "ب").await;
    let p = product(&pool, "سكر", "1", "15").await;
    let a = purchases::save_draft(&pool, &input(&s1, vec![line(&p, "1", "10")])).await.unwrap();
    purchases::save_draft(&pool, &input(&s2, vec![line(&p, "1", "10")])).await.unwrap();
    purchases::receive_purchase(&pool, &a).await.unwrap();
    assert_eq!(purchases::list(&pool, &PurchaseFilter::default()).await.unwrap().len(), 2);
    assert_eq!(purchases::list(&pool, &PurchaseFilter { supplier_id: Some(s1), ..Default::default() }).await.unwrap().len(), 1);
    assert_eq!(purchases::list(&pool, &PurchaseFilter { status: Some("draft".into()), ..Default::default() }).await.unwrap().len(), 1);
    assert_eq!(purchases::list(&pool, &PurchaseFilter { query: Some("PUR-000002".into()), ..Default::default() }).await.unwrap().len(), 1);
    let mv = reports::movements(&pool, &reports::MovementFilter { movement_type: Some("purchase".into()), ..Default::default() }).await.unwrap();
    assert_eq!(mv.len(), 1);
    assert_eq!(mv[0].qty_delta_milli, 1000);
}
