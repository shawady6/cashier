use super::*;
use crate::database::test_support::memory_pool;
use crate::modules::settings;
use sqlx::SqlitePool;

const PIECE: &str = "b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0001";
const KG: &str = "b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0002";
const CARTON: &str = "b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0006";

fn input(name: &str) -> ProductInput {
    ProductInput {
        id: None,
        sku: None,
        name_ar: name.into(),
        name_en: String::new(),
        category_id: None,
        brand_id: None,
        base_unit_id: PIECE.into(),
        purchase_price: "10".into(),
        sale_price: "12.50".into(),
        min_stock_qty: "0".into(),
        is_weighted: false,
        track_expiry: false,
        is_active: true,
        barcodes: vec![],
        variants: vec![],
        units: vec![],
        margin_percent: None,
        buy_in_base: true,
        sell_in_base: true,
        default_buy_big: false,
        default_sell_big: false,
    }
}

fn variant(name: &str) -> VariantInput {
    VariantInput {
        id: None,
        name_ar: name.into(),
        name_en: String::new(),
        sku: None,
        purchase_price: "100".into(),
        sale_price: "150".into(),
        is_active: true,
        barcodes: vec![],
    }
}

async fn count(pool: &SqlitePool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}")).fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn creates_product_with_money_in_minor_units() {
    let pool = memory_pool().await;
    let mut i = input("لبن جهينة");
    i.barcodes = vec!["6221".into()];
    let id = save(&pool, &i).await.unwrap();
    let d = get(&pool, &id).await.unwrap();
    assert_eq!(d.sale_price, 1250);
    assert_eq!(d.purchase_price, 1000);
    assert_eq!(d.barcodes, vec!["6221"]);
}

#[tokio::test]
async fn duplicate_barcode_is_rejected_and_nothing_is_saved() {
    let pool = memory_pool().await;
    let mut a = input("منتج أ");
    a.barcodes = vec!["111".into()];
    save(&pool, &a).await.unwrap();

    let mut b = input("منتج ب");
    b.barcodes = vec!["222".into(), "111".into()];
    let e = save(&pool, &b).await.unwrap_err().to_string();
    assert!(e.contains("111"), "{e}");
    // الـ transaction ما تركت أي بقايا (المنتج ب ولا الباركود 222)
    assert_eq!(count(&pool, "products").await, 1);
    assert_eq!(count(&pool, "product_barcodes").await, 1);
}

#[tokio::test]
async fn duplicate_barcode_inside_same_product_is_rejected() {
    let pool = memory_pool().await;
    let mut a = input("منتج");
    a.barcodes = vec!["555".into(), "٥٥٥".into()]; // نفس الرقم بالأرقام العربية
    assert!(save(&pool, &a).await.is_err());
}

#[tokio::test]
async fn duplicate_sku_is_rejected() {
    let pool = memory_pool().await;
    let mut a = input("أ");
    a.sku = Some("SKU-1".into());
    save(&pool, &a).await.unwrap();
    let mut b = input("ب");
    b.sku = Some("SKU-1".into());
    assert!(save(&pool, &b).await.is_err());
}

#[tokio::test]
async fn editing_keeps_own_barcode_and_updates_prices() {
    let pool = memory_pool().await;
    let mut a = input("سكر");
    a.barcodes = vec!["900".into()];
    let id = save(&pool, &a).await.unwrap();

    let mut edit = input("سكر");
    edit.id = Some(id.clone());
    edit.barcodes = vec!["900".into(), "901".into()];
    edit.sale_price = "15".into();
    save(&pool, &edit).await.unwrap();

    let d = get(&pool, &id).await.unwrap();
    assert_eq!(d.sale_price, 1500);
    assert_eq!(d.barcodes.len(), 2);
    assert_eq!(count(&pool, "products").await, 1);
}

#[tokio::test]
async fn invalid_price_is_rejected() {
    let pool = memory_pool().await;
    let mut a = input("x");
    a.sale_price = "12.345".into();
    assert!(save(&pool, &a).await.is_err());
    a.sale_price = "-5".into();
    assert!(save(&pool, &a).await.is_err());
    assert_eq!(count(&pool, "products").await, 0);
}

#[tokio::test]
async fn decimal_quantities_follow_unit_rules() {
    let pool = memory_pool().await;
    let mut a = input("جبنة");
    a.base_unit_id = KG.into();
    a.min_stock_qty = "2.5".into();
    let id = save(&pool, &a).await.unwrap();
    assert_eq!(get(&pool, &id).await.unwrap().min_stock_qty, 2500);

    let mut b = input("قطع");
    b.min_stock_qty = "2.5".into(); // القطعة لا تقبل كسورًا
    assert!(save(&pool, &b).await.is_err());
}

#[tokio::test]
async fn extra_units_are_saved_with_exact_factor() {
    let pool = memory_pool().await;
    let mut a = input("مياه");
    a.units = vec![ProductUnitInput { unit_id: CARTON.into(), factor: "12".into(), is_purchase_unit: true, is_sale_unit: false, ..Default::default() }];
    let id = save(&pool, &a).await.unwrap();
    let d = get(&pool, &id).await.unwrap();
    assert_eq!(d.units.len(), 1);
    assert_eq!(d.units[0].factor_micro, 12_000_000);

    let mut bad = input("مياه 2");
    bad.units = vec![ProductUnitInput { unit_id: PIECE.into(), factor: "1".into(), is_purchase_unit: false, is_sale_unit: false, ..Default::default() }];
    assert!(save(&pool, &bad).await.is_err()); // الوحدة الأساسية لا تُضاف كوحدة إضافية
}

#[tokio::test]
async fn weighted_and_variants_follow_business_profile() {
    let pool = memory_pool().await;
    // النشاط الافتراضي (تجزئة عامة): متغيرات مسموحة، موزون غير مسموح
    let mut w = input("جبنة رومي");
    w.base_unit_id = KG.into();
    w.is_weighted = true;
    assert!(save(&pool, &w).await.is_err());

    settings::set(&pool, "business_profile_code", "grocery").await.unwrap();
    save(&pool, &w).await.unwrap();

    // البقالة لا تدعم المتغيرات
    let mut v = input("زيت");
    v.variants = vec![variant("1 لتر")];
    assert!(save(&pool, &v).await.is_err());

    settings::set(&pool, "business_profile_code", "perfumes").await.unwrap();
    save(&pool, &v).await.unwrap();
}

#[tokio::test]
async fn weighted_requires_decimal_base_unit() {
    let pool = memory_pool().await;
    settings::set(&pool, "business_profile_code", "grocery").await.unwrap();
    let mut w = input("موزون خطأ");
    w.is_weighted = true; // وحدة القطعة لا تقبل كسورًا
    assert!(save(&pool, &w).await.is_err());
}

#[tokio::test]
async fn variants_have_own_prices_and_barcodes_and_removed_ones_are_deactivated() {
    let pool = memory_pool().await;
    settings::set(&pool, "business_profile_code", "perfumes").await.unwrap();

    let mut p = input("عطر");
    let mut v1 = variant("50 مل");
    v1.barcodes = vec!["v50".into()];
    let mut v2 = variant("100 مل");
    v2.barcodes = vec!["v100".into()];
    v2.sale_price = "250".into();
    p.variants = vec![v1, v2];
    let id = save(&pool, &p).await.unwrap();

    let m = find_by_barcode(&pool, "v100").await.unwrap().unwrap();
    assert_eq!(m.sale_price, 25000);
    assert_eq!(m.variant_name_ar.as_deref(), Some("100 مل"));

    // نعدّل المنتج ونبقي متغيرًا واحدًا فقط
    let d = get(&pool, &id).await.unwrap();
    let keep = d.variants.iter().find(|v| v.name_ar == "50 مل").unwrap();
    let mut edit = input("عطر");
    edit.id = Some(id.clone());
    let mut kv = variant("50 مل");
    kv.id = Some(keep.id.clone());
    kv.barcodes = vec!["v50".into()];
    edit.variants = vec![kv];
    save(&pool, &edit).await.unwrap();

    let d = get(&pool, &id).await.unwrap();
    assert_eq!(d.variants.len(), 2); // لم يُمسح شيء
    assert!(d.variants.iter().any(|v| v.name_ar == "100 مل" && !v.is_active));
}

#[tokio::test]
async fn existing_variants_survive_profile_change() {
    let pool = memory_pool().await;
    settings::set(&pool, "business_profile_code", "perfumes").await.unwrap();
    let mut p = input("عطر");
    p.variants = vec![variant("50 مل")];
    let id = save(&pool, &p).await.unwrap();

    settings::set(&pool, "business_profile_code", "grocery").await.unwrap();
    let d = get(&pool, &id).await.unwrap();
    assert_eq!(d.variants.len(), 1); // البيانات لم تُفقد

    let mut edit = input("عطر");
    edit.id = Some(id);
    let mut kv = variant("50 مل");
    kv.id = Some(d.variants[0].id.clone());
    edit.variants = vec![kv];
    save(&pool, &edit).await.unwrap(); // التعديل على متغير موجود مسموح
}

#[tokio::test]
async fn search_works_in_arabic_english_sku_and_barcode() {
    let pool = memory_pool().await;
    let mut a = input("أرز مصري");
    a.name_en = "Egyptian Rice".into();
    a.sku = Some("RICE-01".into());
    a.barcodes = vec!["6221234".into()];
    save(&pool, &a).await.unwrap();
    save(&pool, &input("سكر")).await.unwrap();

    let q = |s: &str| ProductFilter { query: Some(s.into()), ..Default::default() };
    assert_eq!(list(&pool, &q("ارز")).await.unwrap().len(), 1); // أ = ا
    assert_eq!(list(&pool, &q("rice")).await.unwrap().len(), 1);
    assert_eq!(list(&pool, &q("rice-01")).await.unwrap().len(), 1);
    assert_eq!(list(&pool, &q("6221234")).await.unwrap().len(), 1);
    assert_eq!(list(&pool, &q("٦٢٢١٢٣٤")).await.unwrap().len(), 1);
    assert_eq!(list(&pool, &q("ارز مصري")).await.unwrap().len(), 1);
    assert_eq!(list(&pool, &q("100%")).await.unwrap().len(), 0); // علامة % لا تعمل كـ wildcard
    assert_eq!(list(&pool, &ProductFilter::default()).await.unwrap().len(), 2);
}

#[tokio::test]
async fn archive_hides_product_and_can_restore() {
    let pool = memory_pool().await;
    let id = save(&pool, &input("منتج")).await.unwrap();
    set_archived(&pool, &id, true).await.unwrap();
    assert_eq!(list(&pool, &ProductFilter::default()).await.unwrap().len(), 0);
    let all = ProductFilter { include_archived: true, ..Default::default() };
    assert_eq!(list(&pool, &all).await.unwrap().len(), 1);
    set_archived(&pool, &id, false).await.unwrap();
    assert_eq!(list(&pool, &ProductFilter::default()).await.unwrap().len(), 1);
    assert_eq!(count(&pool, "products").await, 1); // لا حذف فعلي
    assert!(set_archived(&pool, "missing", true).await.is_err());
}

#[tokio::test]
async fn images_are_validated_and_stored_in_data_dir() {
    let pool = memory_pool().await;
    let id = save(&pool, &input("منتج")).await.unwrap();
    let dir = std::env::temp_dir().join(format!("pos-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    // PNG صغيرة صالحة (1x1)
    let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGNgYGD4DwABBAEAX+XDSwAAAABJRU5ErkJggg==";
    images::set_image(&pool, &dir, &id, png).await.unwrap();
    let url = images::get_image(&pool, &dir, &id).await.unwrap().unwrap();
    assert!(url.starts_with("data:image/png;base64,"));
    assert!(get(&pool, &id).await.unwrap().has_image);

    // ملف ليس صورة
    let txt = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"hello world not image");
    assert!(images::set_image(&pool, &dir, &id, &txt).await.is_err());

    images::remove_image(&pool, &dir, &id).await.unwrap();
    assert!(images::get_image(&pool, &dir, &id).await.unwrap().is_none());
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn custom_unit_creation_rejects_duplicates() {
    let pool = memory_pool().await;
    let u = UnitInput { name_ar: "رباط".into(), name_en: String::new(), allows_decimal: false };
    create_unit(&pool, &u).await.unwrap();
    assert!(create_unit(&pool, &u).await.is_err());
    assert!(list_units(&pool).await.unwrap().len() >= 10);
}

#[tokio::test]
async fn sellable_search_hides_archived_and_expands_variants() {
    let pool = memory_pool().await;
    settings::set(&pool, "business_profile_code", "perfumes").await.unwrap();
    let mut p = input("عطر");
    let mut v = variant("50 مل");
    v.barcodes = vec!["v50".into()];
    p.variants = vec![v, variant("100 مل")];
    save(&pool, &p).await.unwrap();
    let plain = save(&pool, &input("صابون")).await.unwrap();

    let all = search_sellables(&pool, "", 50).await.unwrap();
    assert_eq!(all.len(), 3); // صابون + متغيران، والمنتج ذو المتغيرات لا يظهر وحده
    let by_barcode = search_sellables(&pool, "v50", 50).await.unwrap();
    assert_eq!(by_barcode.len(), 1);
    assert_eq!(by_barcode[0].variant_name.as_deref(), Some("50 مل"));
    assert_eq!(search_sellables(&pool, "صابون", 50).await.unwrap()[0].units.len(), 1);

    set_archived(&pool, &plain, true).await.unwrap();
    assert_eq!(search_sellables(&pool, "", 50).await.unwrap().len(), 2);
}
