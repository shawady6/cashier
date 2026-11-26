//! سيناريوهات "بتشتري بإيه وبتبيع بإيه": سعر القطعة يُحسب تلقائيًا ويُقرَّب لأعلى، وخيارات الوحدات.

use sqlx::SqlitePool;

use super::*;
use crate::database::test_support::memory_pool;
use crate::modules::products::validate::{ceil_div, piece_price_from_big};
use crate::modules::products::{self, ProductInput, ProductUnitInput};
use crate::modules::sales::{self, CartInput, CartItemInput, PaymentInput};
use crate::modules::test_helpers::*;

const WH: &str = "c5e3a1b0-2d4f-4b6a-9c11-7a1e5d300001";

async fn supplier(pool: &SqlitePool) -> String {
    suppliers::save(pool, &SupplierInput { id: None, name_ar: "مورد".into(), phone: String::new(), address: String::new(), notes: String::new(), is_active: true })
        .await
        .unwrap()
}

/// منتج بوحدة كبيرة (12 قطعة): سعر الشراء والبيع للكرتونة فقط، ولا يوجد سعر قطعة مُدخل.
fn carton_product(name: &str, buy: &str, sell: &str) -> ProductInput {
    let mut i = base_input(name, "", "");
    i.units = vec![ProductUnitInput {
        unit_id: CARTON.into(), factor: "12".into(), is_purchase_unit: true, is_sale_unit: true,
        sale_price: if sell.is_empty() { None } else { Some(sell.into()) },
        purchase_price: if buy.is_empty() { None } else { Some(buy.into()) },
        ..Default::default()
    }];
    i
}

fn line(pid: &str, unit: Option<&str>, qty: &str, cost: &str, price: Option<&str>) -> PurchaseItemInput {
    PurchaseItemInput {
        product_id: pid.into(), variant_id: None, unit_id: unit.map(Into::into), qty: qty.into(), unit_cost: cost.into(),
        factor: None, sale_price: price.map(Into::into),
    }
}

async fn receive(pool: &SqlitePool, sup: &str, items: Vec<PurchaseItemInput>) {
    let input = PurchaseInput { id: None, supplier_id: sup.into(), warehouse_id: WH.into(), supplier_invoice_no: String::new(), note: String::new(), discount: None, items };
    let id = purchases::save_draft(pool, &input).await.unwrap();
    purchases::receive_purchase(pool, &id).await.unwrap();
}

async fn row(pool: &SqlitePool, sql: &str) -> (i64, i64) {
    sqlx::query_as(sql).fetch_one(pool).await.unwrap()
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

#[test]
fn derived_piece_price_always_rounds_up() {
    assert_eq!(piece_price_from_big(10_000, 12_000_000), 834); // 8.3333 ← 8.34
    assert_eq!(piece_price_from_big(12_000, 12_000_000), 1_000); // قسمة تامة بدون زيادة
    assert_eq!(piece_price_from_big(10_001, 12_000_000), 834);
    assert_eq!(piece_price_from_big(1, 2_000_000), 1);
    assert_eq!(ceil_div(7, 2), 4);
}

#[tokio::test]
async fn carton_prices_give_piece_price_and_cost_without_typing_them() {
    let pool = memory_pool().await;
    let id = products::save(&pool, &carton_product("مياه", "90", "100")).await.unwrap();
    let d = products::get(&pool, &id).await.unwrap();
    assert_eq!(d.sale_price, 834); // 100 ÷ 12 مقرّبًا لأعلى
    assert_eq!(d.purchase_price, 750); // 90 ÷ 12
    assert_eq!(d.price_source, "derived");
    assert_eq!(d.units[0].sale_price_minor, Some(10_000));
    assert_eq!(d.units[0].purchase_price_minor, Some(9_000));
}

#[tokio::test]
async fn manual_piece_price_wins_and_margin_fills_missing_prices() {
    let pool = memory_pool().await;
    let mut m = carton_product("يدوي", "90", "100");
    m.sale_price = "8.50".into();
    let d = products::get(&pool, &products::save(&pool, &m).await.unwrap()).await.unwrap();
    assert_eq!((d.sale_price, d.price_source.as_str()), (850, "manual"));
    assert_eq!(d.units[0].sale_price_minor, Some(10_000));

    // هامش 15% على شراء الكرتونة 90 ← بيع الكرتونة 103.50 ← القطعة 8.63
    let mut g = carton_product("هامش", "90", "");
    g.margin_percent = Some("15".into());
    let d = products::get(&pool, &products::save(&pool, &g).await.unwrap()).await.unwrap();
    assert_eq!(d.units[0].sale_price_minor, Some(10_350));
    assert_eq!(d.sale_price, 863);

    // منتج بالقطعة فقط: تكلفة 7.50 وهامش 20% ← 9.00
    let mut p = base_input("قطعة", "7.50", "");
    p.margin_percent = Some("20".into());
    let d = products::get(&pool, &products::save(&pool, &p).await.unwrap()).await.unwrap();
    assert_eq!(d.sale_price, 900);
}

#[tokio::test]
async fn invalid_unit_options_are_rejected() {
    let pool = memory_pool().await;
    let no_price = base_input("بلا سعر", "", "");
    assert!(products::save(&pool, &no_price).await.is_err());

    let mut none_buy = carton_product("x", "90", "100");
    none_buy.buy_in_base = false;
    none_buy.units[0].is_purchase_unit = false;
    assert!(products::save(&pool, &none_buy).await.is_err());

    let mut bad_default = carton_product("y", "90", "100");
    bad_default.units[0].is_sale_unit = false;
    bad_default.default_sell_big = true;
    assert!(products::save(&pool, &bad_default).await.is_err());

    let mut base_off_no_default = carton_product("z", "90", "100");
    base_off_no_default.buy_in_base = false;
    assert!(products::save(&pool, &base_off_no_default).await.is_err()); // الافتراضي لازم يكون الكبيرة

    let mut factor_one = carton_product("w", "90", "100");
    factor_one.units[0].factor = "1".into();
    assert!(products::save(&pool, &factor_one).await.is_err());
}

#[tokio::test]
async fn fifteen_cartons_at_90_sell_at_100_then_price_change_updates_everything_together() {
    let pool = memory_pool().await;
    let s = supplier(&pool).await;
    let id = products::save(&pool, &carton_product("مياه", "90", "100")).await.unwrap();

    receive(&pool, &s, vec![line(&id, Some(CARTON), "15", "90", None)]).await;
    assert_eq!(row(&pool, "SELECT SUM(qty_milli), SUM(value_minor) FROM stock_balances").await, (180_000, 135_000));

    // سعر الشراء ارتفع لـ 95 والبيع لـ 105: كل الأسعار المحسوبة تتحدث في نفس الاستلام
    receive(&pool, &s, vec![line(&id, Some(CARTON), "5", "95", Some("105"))]).await;
    let d = products::get(&pool, &id).await.unwrap();
    assert_eq!(d.units[0].sale_price_minor, Some(10_500));
    assert_eq!(d.units[0].purchase_price_minor, Some(9_500));
    assert_eq!(d.sale_price, 875); // 105 ÷ 12 = 8.75 تمامًا
    assert_eq!(d.price_source, "derived");
    assert_eq!(row(&pool, "SELECT SUM(qty_milli), SUM(value_minor) FROM stock_balances").await, (240_000, 135_000 + 47_500));

    // بيع قطعة وبيع كرتونة بالأسعار الجديدة
    sales::checkout(&pool, &sell(&id, None, "1")).await.unwrap();
    sales::checkout(&pool, &sell(&id, Some(CARTON), "1")).await.unwrap();
    let prices: Vec<i64> = sqlx::query_scalar("SELECT unit_price_minor FROM sale_items ORDER BY rowid").fetch_all(&pool).await.unwrap();
    assert_eq!(prices, vec![875, 10_500]);
}

#[tokio::test]
async fn manual_piece_price_is_not_touched_by_receiving() {
    let pool = memory_pool().await;
    let s = supplier(&pool).await;
    let mut m = carton_product("يدوي", "90", "100");
    m.sale_price = "8.50".into();
    let id = products::save(&pool, &m).await.unwrap();
    receive(&pool, &s, vec![line(&id, Some(CARTON), "1", "95", Some("120"))]).await;
    let d = products::get(&pool, &id).await.unwrap();
    assert_eq!((d.sale_price, d.units[0].sale_price_minor), (850, Some(12_000)));
}

#[tokio::test]
async fn buying_and_selling_options_are_enforced() {
    let pool = memory_pool().await;
    let s = supplier(&pool).await;
    // يُشترى بالكرتونة فقط ويُباع بالكرتونة فقط
    let mut only_big = carton_product("كرتونة فقط", "90", "100");
    only_big.buy_in_base = false;
    only_big.sell_in_base = false;
    only_big.default_buy_big = true;
    only_big.default_sell_big = true;
    let id = products::save(&pool, &only_big).await.unwrap();

    let piece_buy = PurchaseInput { id: None, supplier_id: s.clone(), warehouse_id: WH.into(), supplier_invoice_no: String::new(), note: String::new(), discount: None, items: vec![line(&id, None, "5", "8", None)] };
    assert!(purchases::save_draft(&pool, &piece_buy).await.is_err());

    receive(&pool, &s, vec![line(&id, Some(CARTON), "2", "90", None)]).await;
    assert!(sales::checkout(&pool, &sell(&id, None, "1")).await.is_err());
    sales::checkout(&pool, &sell(&id, Some(CARTON), "1")).await.unwrap();

    let hit = &products::search_sellables(&pool, "كرتونة فقط", 5).await.unwrap()[0];
    assert_eq!(hit.units.len(), 1);
    assert_eq!(hit.default_unit_id, CARTON);
}

#[tokio::test]
async fn default_sale_unit_is_reported_for_the_pos() {
    let pool = memory_pool().await;
    let mut both = carton_product("الاتنين", "90", "100");
    both.default_sell_big = true;
    products::save(&pool, &both).await.unwrap();
    let hit = &products::search_sellables(&pool, "الاتنين", 5).await.unwrap()[0];
    assert_eq!(hit.units.len(), 2);
    assert_eq!(hit.default_unit_id, CARTON);

    let plain = base_input("عادي", "5", "8");
    products::save(&pool, &plain).await.unwrap();
    let hit = &products::search_sellables(&pool, "عادي", 5).await.unwrap()[0];
    assert_eq!(hit.default_unit_id, PIECE);
}

#[tokio::test]
async fn opening_stock_batch_is_all_or_nothing() {
    use crate::modules::inventory::{adjust_many, AdjustInput};
    let pool = memory_pool().await;
    let a = products::save(&pool, &carton_product("أ", "90", "100")).await.unwrap();
    let b = products::save(&pool, &carton_product("ب", "80", "120")).await.unwrap();
    let ok = |pid: &str, qty: &str, cost: &str| AdjustInput {
        product_id: pid.into(), direction: "increase".into(), qty: qty.into(), unit_id: Some(CARTON.into()),
        unit_cost: Some(cost.into()), ..Default::default()
    };
    // سطر ثانٍ خاطئ (كمية صفر): لا يُحفظ أي شيء
    assert!(adjust_many(&pool, &[ok(&a, "12", "90"), ok(&b, "0", "80")]).await.is_err());
    assert_eq!(row(&pool, "SELECT COUNT(*), 0 FROM stock_balances").await.0, 0);

    adjust_many(&pool, &[ok(&a, "12", "90"), ok(&b, "13", "80")]).await.unwrap();
    // 12 كرتونة بـ 90 = 144 قطعة، 13 كرتونة بـ 80 = 156 قطعة
    assert_eq!(row(&pool, "SELECT SUM(qty_milli), SUM(value_minor) FROM stock_balances").await, (300_000, 108_000 + 104_000));
}
