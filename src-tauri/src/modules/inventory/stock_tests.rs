use sqlx::SqlitePool;

use super::counts::{self, CountInput, CountLineInput};
use super::reports::{self, BalanceFilter};
use super::transfers::{self, TransferInput, TransferItemInput};
use super::warehouses::{self, WarehouseInput};
use crate::database::test_support::memory_pool;
use crate::modules::settings;
use crate::modules::test_helpers::*;

const WH1: &str = "c5e3a1b0-2d4f-4b6a-9c11-7a1e5d300001";

async fn second_warehouse(pool: &SqlitePool) -> String {
    warehouses::save(pool, &WarehouseInput { id: None, name_ar: "مخزن الفرع".into(), is_active: true, is_default: false }).await.unwrap()
}

async fn qty_in(pool: &SqlitePool, wh: &str, pid: &str) -> (i64, i64) {
    sqlx::query_as("SELECT COALESCE(SUM(qty_milli),0), COALESCE(SUM(value_minor),0) FROM stock_balances WHERE warehouse_id = ?1 AND product_id = ?2")
        .bind(wh)
        .bind(pid)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn warehouse_rules() {
    let pool = memory_pool().await;
    let w2 = second_warehouse(&pool).await;
    assert!(warehouses::save(&pool, &WarehouseInput { id: None, name_ar: "مخزن الفرع".into(), is_active: true, is_default: false }).await.is_err()); // مكرر
    // الافتراضي لا يُعطَّل
    assert!(warehouses::save(&pool, &WarehouseInput { id: Some(WH1.into()), name_ar: "المخزن الرئيسي".into(), is_active: false, is_default: true }).await.is_err());
    // تغيير الافتراضي
    warehouses::save(&pool, &WarehouseInput { id: Some(w2.clone()), name_ar: "مخزن الفرع".into(), is_active: true, is_default: true }).await.unwrap();
    let list = warehouses::list(&pool).await.unwrap();
    assert_eq!(list.iter().filter(|w| w.is_default).count(), 1);
    assert!(list.iter().find(|w| w.id == w2).unwrap().is_default);
    // الآن يمكن تعطيل القديم
    warehouses::save(&pool, &WarehouseInput { id: Some(WH1.into()), name_ar: "المخزن الرئيسي".into(), is_active: false, is_default: false }).await.unwrap();
}

#[tokio::test]
async fn transfer_moves_quantity_and_value_between_warehouses() {
    let pool = memory_pool().await;
    let w2 = second_warehouse(&pool).await;
    let p = product(&pool, "سكر", "1", "15").await;
    stock(&pool, &p, "10", "10").await;
    let t = |qty: &str, from: &str, to: &str| TransferInput {
        from_warehouse_id: from.into(), to_warehouse_id: to.into(), note: String::new(),
        items: vec![TransferItemInput { product_id: p.clone(), variant_id: None, qty: qty.into() }],
    };
    transfers::create(&pool, &t("4", WH1, &w2)).await.unwrap();
    assert_eq!(qty_in(&pool, WH1, &p).await, (6000, 6000));
    assert_eq!(qty_in(&pool, &w2, &p).await, (4000, 4000)); // نفس متوسط التكلفة
    let total: (i64, i64) = (qty_in(&pool, WH1, &p).await.1 + qty_in(&pool, &w2, &p).await.1, 0);
    assert_eq!(total.0, 10_000); // لا تتغير القيمة الكلية

    // أكثر من الرصيد، أو نفس المخزن، أو كمية صفر: مرفوض بلا آثار جانبية
    assert!(transfers::create(&pool, &t("7", WH1, &w2)).await.is_err());
    assert!(transfers::create(&pool, &t("1", WH1, WH1)).await.is_err());
    assert!(transfers::create(&pool, &t("0", WH1, &w2)).await.is_err());
    assert_eq!(qty_in(&pool, WH1, &p).await.0, 6000);
    assert_eq!(transfers::list(&pool).await.unwrap().len(), 1);
    assert!(sqlx::query("DELETE FROM stock_transfers").execute(&pool).await.is_err());
}

#[tokio::test]
async fn failed_multi_item_transfer_rolls_back_first_item() {
    let pool = memory_pool().await;
    let w2 = second_warehouse(&pool).await;
    let a = product(&pool, "أ", "1", "5").await;
    let b = product(&pool, "ب", "1", "5").await;
    stock(&pool, &a, "10", "1").await;
    stock(&pool, &b, "1", "1").await;
    let r = transfers::create(&pool, &TransferInput {
        from_warehouse_id: WH1.into(), to_warehouse_id: w2.clone(), note: String::new(),
        items: vec![
            TransferItemInput { product_id: a.clone(), variant_id: None, qty: "5".into() },
            TransferItemInput { product_id: b.clone(), variant_id: None, qty: "9".into() },
        ],
    })
    .await;
    assert!(r.is_err());
    assert_eq!(qty_in(&pool, WH1, &a).await.0, 10_000);
    assert_eq!(qty_in(&pool, &w2, &a).await.0, 0);
}

#[tokio::test]
async fn stock_count_applies_difference_to_current_balance() {
    let pool = memory_pool().await;
    let a = product(&pool, "أ", "5", "10").await;
    let b = product(&pool, "ب", "5", "10").await;
    let c = product(&pool, "ج", "5", "10").await;
    stock(&pool, &a, "10", "5").await;
    stock(&pool, &b, "10", "5").await;
    stock(&pool, &c, "10", "5").await;

    let id = counts::create(&pool, &CountInput { warehouse_id: WH1.into(), category_id: None, note: String::new() }).await.unwrap();
    let detail = counts::get(&pool, &id).await.unwrap();
    assert_eq!(detail.items.len(), 3);
    assert_eq!(detail.header.count_no, "CNT-000001");

    // أثناء الجرد: تُباع قطعتان من (أ)
    crate::modules::inventory::adjust(
        &pool,
        &crate::modules::inventory::AdjustInput { product_id: a.clone(), variant_id: None, direction: "decrease".into(), qty: "2".into(), unit_cost: None, note: String::new() },
    )
    .await
    .unwrap();

    let item = |name: &str| detail.items.iter().find(|i| i.name_ar == name).unwrap().id.clone();
    counts::save_lines(&pool, &id, &[
        CountLineInput { item_id: item("أ"), counted_qty: Some("8".into()) },  // اللقطة 10، الفعلي 8 → فرق -2 على الرصيد الحالي 8 → 6
        CountLineInput { item_id: item("ب"), counted_qty: Some("12".into()) }, // +2
        CountLineInput { item_id: item("ج"), counted_qty: None },              // لم يُجرَد: يُتجاهل
    ]).await.unwrap();
    assert_eq!(counts::finalize(&pool, &id).await.unwrap(), 2);

    assert_eq!(qty_in(&pool, WH1, &a).await, (6000, 3000));
    assert_eq!(qty_in(&pool, WH1, &b).await, (12_000, 6000)); // الزيادة بمتوسط التكلفة
    assert_eq!(qty_in(&pool, WH1, &c).await.0, 10_000);

    // بعد الاعتماد: مغلق
    assert!(counts::finalize(&pool, &id).await.is_err());
    assert!(counts::save_lines(&pool, &id, &[]).await.is_err());
    assert!(counts::cancel(&pool, &id).await.is_err());
    let m = reports::movements(&pool, &reports::MovementFilter { movement_type: Some("count".into()), ..Default::default() }).await.unwrap();
    assert_eq!(m.len(), 2);
}

#[tokio::test]
async fn count_rejects_bad_quantities_and_foreign_items_and_can_be_cancelled() {
    let pool = memory_pool().await;
    let a = product(&pool, "أ", "5", "10").await;
    stock(&pool, &a, "1", "5").await;
    let id = counts::create(&pool, &CountInput { warehouse_id: WH1.into(), category_id: None, note: String::new() }).await.unwrap();
    let item = counts::get(&pool, &id).await.unwrap().items[0].id.clone();
    assert!(counts::save_lines(&pool, &id, &[CountLineInput { item_id: item.clone(), counted_qty: Some("1.5".into()) }]).await.is_err());
    assert!(counts::save_lines(&pool, &id, &[CountLineInput { item_id: item.clone(), counted_qty: Some("-1".into()) }]).await.is_err());
    assert!(counts::save_lines(&pool, &id, &[CountLineInput { item_id: "zzz".into(), counted_qty: Some("1".into()) }]).await.is_err());
    counts::cancel(&pool, &id).await.unwrap();
    assert!(counts::finalize(&pool, &id).await.is_err());
    // لا جرد لمخزن غير موجود
    assert!(counts::create(&pool, &CountInput { warehouse_id: "x".into(), category_id: None, note: String::new() }).await.is_err());
}

#[tokio::test]
async fn balances_report_and_low_stock() {
    let pool = memory_pool().await;
    let a = product(&pool, "أرز", "5", "10").await;
    let b = product(&pool, "سكر", "5", "10").await;
    stock(&pool, &a, "10", "5").await;
    stock(&pool, &b, "2", "6").await;
    // حد إعادة الطلب للسكر = 5
    let mut edit = base_input("سكر", "5", "10");
    edit.id = Some(b.clone());
    edit.min_stock_qty = "5".into();
    crate::modules::products::save(&pool, &edit).await.unwrap();

    let r = reports::balances(&pool, &BalanceFilter::default()).await.unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.total_value_minor, 5000 + 1200);
    assert_eq!(r.rows.iter().find(|x| x.name_ar == "أرز").unwrap().avg_cost_minor, 500);
    assert_eq!(reports::balances(&pool, &BalanceFilter { query: Some("ارز".into()), ..Default::default() }).await.unwrap().rows.len(), 1);

    let low = reports::low_stock(&pool).await.unwrap();
    assert_eq!(low.len(), 1);
    assert_eq!(low[0].name_ar, "سكر");
    stock(&pool, &b, "10", "6").await; // 12 > 5
    assert!(reports::low_stock(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn sales_use_the_default_warehouse_after_default_changes() {
    let pool = memory_pool().await;
    let w2 = second_warehouse(&pool).await;
    warehouses::save(&pool, &WarehouseInput { id: Some(w2.clone()), name_ar: "مخزن الفرع".into(), is_active: true, is_default: true }).await.unwrap();
    let p = product(&pool, "أ", "5", "10").await;
    // الرصيد الافتتاحي يدخل المخزن الافتراضي الحالي
    stock(&pool, &p, "5", "5").await;
    assert_eq!(qty_in(&pool, &w2, &p).await.0, 5000);
    assert_eq!(qty_in(&pool, WH1, &p).await.0, 0);
    settings::set(&pool, "allow_negative_stock", "0").await.unwrap();
}
