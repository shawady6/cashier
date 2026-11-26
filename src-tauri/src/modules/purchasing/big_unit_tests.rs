//! اختبارات الوحدة الكبيرة (كرتونة/دستة...): المخزون بالقطعة دائمًا والتكلفة من إجمالي السطر.

use sqlx::SqlitePool;

use super::*;
use crate::database::test_support::memory_pool;
use crate::modules::inventory::{adjust, AdjustInput};
use crate::modules::products::{self, ProductUnitInput};
use crate::modules::sales::{self, CartInput, CartItemInput, PaymentInput, ReturnInput, ReturnItemInput};
use crate::modules::test_helpers::*;

const WH: &str = "c5e3a1b0-2d4f-4b6a-9c11-7a1e5d300001";

async fn supplier(pool: &SqlitePool) -> String {
    suppliers::save(pool, &SupplierInput { id: None, name_ar: "مورد".into(), phone: String::new(), address: String::new(), notes: String::new(), is_active: true })
        .await
        .unwrap()
}

fn carton_line(pid: &str, qty: &str, cost: &str, factor: Option<&str>, price: Option<&str>) -> PurchaseItemInput {
    PurchaseItemInput {
        product_id: pid.into(), variant_id: None, unit_id: Some(CARTON.into()), qty: qty.into(), unit_cost: cost.into(),
        factor: factor.map(Into::into), sale_price: price.map(Into::into),
    }
}

fn draft(sup: &str, items: Vec<PurchaseItemInput>) -> PurchaseInput {
    PurchaseInput { id: None, supplier_id: sup.into(), warehouse_id: WH.into(), supplier_invoice_no: String::new(), note: String::new(), discount: None, items }
}

async fn receive_new(pool: &SqlitePool, sup: &str, items: Vec<PurchaseItemInput>) -> String {
    let id = purchases::save_draft(pool, &draft(sup, items)).await.unwrap();
    purchases::receive_purchase(pool, &id).await.unwrap();
    id
}

async fn stock_of(pool: &SqlitePool) -> (i64, i64) {
    sqlx::query_as("SELECT COALESCE(SUM(qty_milli),0), COALESCE(SUM(value_minor),0) FROM stock_balances")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn num(pool: &SqlitePool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

fn sell(pid: &str, unit: Option<&str>, qty: &str) -> CartInput {
    CartInput {
        items: vec![CartItemInput { product_id: pid.into(), variant_id: None, unit_id: unit.map(Into::into), qty: qty.into(), discount: None }],
        invoice_discount: None,
        payments: vec![PaymentInput { method: "cash".into(), amount: "5000".into(), reference: String::new() }],
        client_request_id: None,
        note: String::new(),
        customer_id: None,
    }
}

/// 3 كراتين × 12 + 2 كرتونة × 14 = 64 قطعة، بتكلفة 10 جنيه للقطعة.
async fn stocked() -> (SqlitePool, String, String) {
    let pool = memory_pool().await;
    let p = product_with_carton(&pool, "مياه", "10", "15").await;
    let s = supplier(&pool).await;
    receive_new(&pool, &s, vec![carton_line(&p, "3", "120", None, None), carton_line(&p, "2", "140", Some("14"), None)]).await;
    (pool, p, s)
}

#[tokio::test]
async fn cartons_with_different_counts_give_exact_pieces_value_and_average_cost() {
    let (pool, p, _) = stocked().await;
    assert_eq!(stock_of(&pool).await, (64_000, 64_000 * 1)); // 64 قطعة، قيمة 640 جنيه = 64000 قرش
    let (qty, value): (i64, i64) = (stock_of(&pool).await.0, stock_of(&pool).await.1);
    assert_eq!(value * 1000 / qty, 1000); // متوسط 10 جنيه للقطعة
    // المنتج نفسه: المعامل الافتراضي لم يتغير بسبب البند ذي الـ14
    let f: i64 = num(&pool, &format!("SELECT factor_micro FROM product_units WHERE product_id = '{p}'")).await;
    assert_eq!(f, 12_000_000);
    // البنود تحمل المعامل الفعلي وكمية القطع
    let rows: Vec<(i64, i64)> = sqlx::query_as("SELECT unit_factor_micro, base_qty_milli FROM purchase_items ORDER BY line_no").fetch_all(&pool).await.unwrap();
    assert_eq!(rows, vec![(12_000_000, 36_000), (14_000_000, 28_000)]);
}

#[tokio::test]
async fn cost_comes_from_line_total_without_per_piece_drift() {
    let pool = memory_pool().await;
    let p = product_with_carton(&pool, "عصير", "1", "2").await;
    let s = supplier(&pool).await;
    // 100 جنيه لكرتونة 24 قطعة = 4.1666.. للقطعة؛ القيمة الكاملة يجب أن تبقى 100 جنيه بالضبط
    receive_new(&pool, &s, vec![carton_line(&p, "1", "100", Some("24"), None)]).await;
    assert_eq!(stock_of(&pool).await, (24_000, 10_000));
}

#[tokio::test]
async fn selling_pieces_then_a_carton_reduces_stock_and_cost_correctly() {
    let (pool, p, _) = stocked().await;
    sales::checkout(&pool, &sell(&p, None, "10")).await.unwrap();
    assert_eq!(stock_of(&pool).await, (54_000, 54_000));
    sales::checkout(&pool, &sell(&p, Some(CARTON), "1")).await.unwrap();
    assert_eq!(stock_of(&pool).await, (42_000, 42_000));
    let costs: Vec<i64> = sqlx::query_scalar("SELECT cost_minor FROM sale_items ORDER BY rowid").fetch_all(&pool).await.unwrap();
    assert_eq!(costs, vec![10_000, 12_000]); // 10 قطع و12 قطعة بمتوسط 10 جنيه
    let price: i64 = num(&pool, "SELECT unit_price_minor FROM sale_items ORDER BY rowid DESC LIMIT 1").await;
    assert_eq!(price, 18_000); // لا سعر مخصص: 15 × 12
}

#[tokio::test]
async fn rejects_small_or_fractional_counts_and_overselling() {
    let pool = memory_pool().await;
    for bad in ["1", "0", "12.5", "-3"] {
        let mut i = base_input("س", "10", "15");
        i.units = vec![ProductUnitInput { unit_id: CARTON.into(), factor: bad.into(), is_purchase_unit: true, is_sale_unit: true, ..Default::default() }];
        assert!(products::save(&pool, &i).await.is_err(), "factor {bad}");
    }
    // الوحدة الأساسية لا تكون وحدة كبيرة، ولا أكثر من وحدة كبيرة واحدة
    let mut i = base_input("ص", "10", "15");
    i.units = vec![ProductUnitInput { unit_id: PIECE.into(), factor: "12".into(), ..Default::default() }];
    assert!(products::save(&pool, &i).await.is_err());
    let mut i = base_input("ع", "10", "15");
    i.units = vec![
        ProductUnitInput { unit_id: CARTON.into(), factor: "12".into(), ..Default::default() },
        ProductUnitInput { unit_id: "b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0008".into(), factor: "12".into(), ..Default::default() },
    ];
    assert!(products::save(&pool, &i).await.is_err());

    let (pool, p, _) = stocked().await;
    assert!(sales::checkout(&pool, &sell(&p, Some(CARTON), "6")).await.is_err()); // 72 > 64
    assert_eq!(stock_of(&pool).await, (64_000, 64_000));
    assert_eq!(num(&pool, "SELECT COUNT(*) FROM sales").await, 0);
    // السماح بالسالب يغيّر السلوك
    crate::modules::settings::set(&pool, "allow_negative_stock", "1").await.unwrap();
    sales::checkout(&pool, &sell(&p, Some(CARTON), "6")).await.unwrap();
    assert_eq!(stock_of(&pool).await.0, -8_000);
}

#[tokio::test]
async fn purchase_return_and_sale_return_restore_exact_base_quantities() {
    let (pool, p, _) = stocked().await;
    // مرتجع كرتونة من البند ذي الـ14: ينقص 14 قطعة بالضبط
    let pid: String = sqlx::query_scalar("SELECT purchase_id FROM purchase_items LIMIT 1").fetch_one(&pool).await.unwrap();
    let item14: String = sqlx::query_scalar("SELECT id FROM purchase_items WHERE unit_factor_micro = 14000000").fetch_one(&pool).await.unwrap();
    returns::create(&pool, &PurchaseReturnInput { purchase_id: pid, items: vec![PurchaseReturnItemInput { purchase_item_id: item14, qty: "1".into() }], note: String::new() })
        .await
        .unwrap();
    assert_eq!(stock_of(&pool).await, (50_000, 50_000));

    // بيع كرتونة (12) ثم ارجاعها: يرجع 12 قطعة بالضبط
    let r = sales::checkout(&pool, &sell(&p, Some(CARTON), "1")).await.unwrap();
    assert_eq!(stock_of(&pool).await.0, 38_000);
    let item: String = sqlx::query_scalar("SELECT id FROM sale_items").fetch_one(&pool).await.unwrap();
    sales::create_return(&pool, &ReturnInput { sale_id: r.sale_id, items: vec![ReturnItemInput { sale_item_id: item, qty: "1".into() }], reason: String::new(), refund_method: "cash".into() })
        .await
        .unwrap();
    assert_eq!(stock_of(&pool).await.0, 50_000);
}

#[tokio::test]
async fn receiving_updates_unit_sale_prices_but_not_old_invoices() {
    let (pool, p, s) = stocked().await;
    sales::checkout(&pool, &sell(&p, Some(CARTON), "1")).await.unwrap(); // بسعر 180
    let old: i64 = num(&pool, "SELECT unit_price_minor FROM sale_items").await;
    assert_eq!(old, 18_000);

    // مسودة بسعر كرتونة 200 وسعر قطعة 16: لا يتغير شيء قبل الاستلام
    let mut piece = line_piece(&p, "5", "10");
    piece.sale_price = Some("16".into());
    let id = purchases::save_draft(&pool, &draft(&s, vec![carton_line(&p, "1", "130", None, Some("200")), piece])).await.unwrap();
    assert_eq!(num(&pool, "SELECT COALESCE(sale_price_minor, 0) FROM product_units").await, 0);
    assert_eq!(num(&pool, "SELECT sale_price FROM products").await, 1500);
    assert_eq!(num(&pool, "SELECT status = 'draft' FROM purchases WHERE id = (SELECT id FROM purchases ORDER BY rowid DESC LIMIT 1)").await, 1);

    purchases::receive_purchase(&pool, &id).await.unwrap();
    assert_eq!(num(&pool, "SELECT sale_price_minor FROM product_units").await, 20_000);
    assert_eq!(num(&pool, "SELECT sale_price FROM products").await, 1600);
    assert_eq!(num(&pool, "SELECT unit_price_minor FROM sale_items").await, 18_000); // الفاتورة القديمة كما هي
    assert_eq!(num(&pool, "SELECT new_sale_price_minor FROM purchase_items WHERE unit_id = 'b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0006' ORDER BY rowid DESC LIMIT 1").await, 20_000);

    // فاتورة جديدة تستخدم السعر الجديد للكرتونة
    sales::checkout(&pool, &sell(&p, Some(CARTON), "1")).await.unwrap();
    assert_eq!(num(&pool, "SELECT unit_price_minor FROM sale_items ORDER BY rowid DESC LIMIT 1").await, 20_000);
}

fn line_piece(pid: &str, qty: &str, cost: &str) -> PurchaseItemInput {
    PurchaseItemInput { product_id: pid.into(), variant_id: None, unit_id: None, qty: qty.into(), unit_cost: cost.into(), factor: None, sale_price: None }
}

#[tokio::test]
async fn failed_receive_rolls_back_prices_and_stock() {
    let pool = memory_pool().await;
    let a = product_with_carton(&pool, "أ", "10", "15").await;
    let b = product_with_carton(&pool, "ب", "10", "15").await;
    let s = supplier(&pool).await;
    let id = purchases::save_draft(&pool, &draft(&s, vec![carton_line(&a, "1", "100", None, Some("250")), carton_line(&b, "1", "100", None, Some("250"))]))
        .await
        .unwrap();
    // نفشل عند البند الثاني بعد نجاح الأول
    sqlx::query(&format!("CREATE TRIGGER fail_b BEFORE INSERT ON stock_movements WHEN NEW.product_id = '{b}' BEGIN SELECT RAISE(ABORT, 'فشل مقصود'); END;"))
        .execute(&pool)
        .await
        .unwrap();
    assert!(purchases::receive_purchase(&pool, &id).await.is_err());
    assert_eq!(num(&pool, "SELECT COUNT(*) FROM product_units WHERE sale_price_minor IS NOT NULL").await, 0);
    assert_eq!(stock_of(&pool).await, (0, 0));
    assert_eq!(num(&pool, "SELECT status = 'draft' FROM purchases").await, 1);
}

#[tokio::test]
async fn saving_a_purchase_is_always_a_draft_until_explicit_receive() {
    let pool = memory_pool().await;
    let p = product_with_carton(&pool, "ماء", "10", "15").await;
    let s = supplier(&pool).await;
    let id = purchases::save_draft(&pool, &draft(&s, vec![carton_line(&p, "2", "120", None, Some("200"))])).await.unwrap();
    // تعديل المسودة أكثر من مرة لا يغيّر مخزونًا ولا أسعارًا
    let mut again = draft(&s, vec![carton_line(&p, "3", "125", Some("10"), Some("210"))]);
    again.id = Some(id.clone());
    purchases::save_draft(&pool, &again).await.unwrap();
    assert_eq!(purchases::get(&pool, &id).await.unwrap().header.status, "draft");
    assert_eq!(stock_of(&pool).await, (0, 0));
    assert_eq!(num(&pool, "SELECT sale_price FROM products").await, 1500);
    assert_eq!(num(&pool, "SELECT COUNT(*) FROM product_units WHERE sale_price_minor IS NOT NULL").await, 0);
    // بعد الاستلام لا يمكن حفظها من جديد
    purchases::receive_purchase(&pool, &id).await.unwrap();
    assert!(purchases::save_draft(&pool, &again).await.is_err());
    assert_eq!(stock_of(&pool).await, (30_000, 37_500)); // 3 كراتين × 10 = 30 قطعة، 3 × 125 جنيه
}

#[tokio::test]
async fn opening_stock_adjustment_supports_big_unit_cost_and_sale_price() {
    let pool = memory_pool().await;
    let p = product_with_carton(&pool, "زيت", "10", "15").await;
    adjust(
        &pool,
        &AdjustInput {
            product_id: p.clone(), direction: "increase".into(), qty: "2".into(), unit_id: Some(CARTON.into()), factor: Some("14".into()),
            unit_cost: Some("140".into()), sale_price: Some("200".into()), ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(stock_of(&pool).await, (28_000, 28_000));
    assert_eq!(num(&pool, "SELECT sale_price_minor FROM product_units").await, 20_000);
    // نقص كرتونة بالمعامل الافتراضي 12
    adjust(&pool, &AdjustInput { product_id: p.clone(), direction: "decrease".into(), qty: "1".into(), unit_id: Some(CARTON.into()), ..Default::default() })
        .await
        .unwrap();
    assert_eq!(stock_of(&pool).await.0, 16_000);
    // معامل غير صالح
    assert!(adjust(&pool, &AdjustInput { product_id: p, direction: "increase".into(), qty: "1".into(), unit_id: Some(CARTON.into()), factor: Some("1".into()), ..Default::default() }).await.is_err());
}

#[tokio::test]
async fn big_unit_barcode_is_unique_and_matches_the_unit() {
    let pool = memory_pool().await;
    let mut i = base_input("حليب", "10", "15");
    i.units = vec![ProductUnitInput {
        unit_id: CARTON.into(), factor: "24".into(), is_purchase_unit: true, is_sale_unit: true,
        sale_price: Some("330".into()), barcode: Some("6220001".into()),
    }];
    let id = products::save(&pool, &i).await.unwrap();
    let hits = products::search_sellables(&pool, "6220001", 10).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].matched_unit_id.as_deref(), Some(CARTON));
    assert!(hits[0].units.iter().any(|u| u.unit_id == CARTON && u.sale_price_minor == Some(33_000)));
    // التفاصيل تعيد باركود الوحدة ولا تخلطه بباركود القطعة
    let d = products::get(&pool, &id).await.unwrap();
    assert!(d.barcodes.is_empty());
    assert_eq!(d.units[0].barcode.as_deref(), Some("6220001"));
    assert_eq!(d.units[0].sale_price_minor, Some(33_000));
    // الحفظ مرة ثانية يحافظ عليه، ومنتج آخر لا يستطيع استخدامه
    products::save(&pool, &{ let mut j = base_input("حليب", "10", "15"); j.id = Some(id.clone()); j.units = vec![ProductUnitInput { unit_id: CARTON.into(), factor: "24".into(), is_purchase_unit: true, is_sale_unit: true, sale_price: Some("330".into()), barcode: Some("6220001".into()) }]; j }).await.unwrap();
    let mut other = base_input("لبن", "10", "15");
    other.barcodes = vec!["6220001".into()];
    assert!(products::save(&pool, &other).await.is_err());
    // البيع بباركود الكرتونة بسعرها المخصص
    sales::checkout(&pool, &sell_unit(&id)).await.unwrap_err(); // لا مخزون
    crate::modules::settings::set(&pool, "allow_negative_stock", "1").await.unwrap();
    sales::checkout(&pool, &sell_unit(&id)).await.unwrap();
    assert_eq!(num(&pool, "SELECT unit_price_minor FROM sale_items").await, 33_000);
    assert_eq!(stock_of(&pool).await.0, -24_000);
}

fn sell_unit(pid: &str) -> CartInput {
    let mut c = sell(pid, Some(CARTON), "1");
    c.payments[0].amount = "10000".into();
    c
}
