//! أدوات مشتركة للاختبارات فقط.
use sqlx::SqlitePool;

use crate::modules::inventory::{adjust, AdjustInput};
use crate::modules::products::{self, ProductInput, ProductUnitInput};

pub const PIECE: &str = "b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0001";
pub const KG: &str = "b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0002";
pub const CARTON: &str = "b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0006";

pub fn base_input(name: &str, purchase: &str, sale: &str) -> ProductInput {
    ProductInput {
        id: None, sku: None, name_ar: name.into(), name_en: String::new(), category_id: None, brand_id: None,
        base_unit_id: PIECE.into(), purchase_price: purchase.into(), sale_price: sale.into(),
        min_stock_qty: "0".into(), is_weighted: false, track_expiry: false, is_active: true,
        barcodes: vec![], variants: vec![], units: vec![],
    }
}

pub async fn product(pool: &SqlitePool, name: &str, purchase: &str, sale: &str) -> String {
    products::save(pool, &base_input(name, purchase, sale)).await.unwrap()
}

/// منتج بالقطعة مع كرتونة (12 قطعة) كوحدة بيع.
pub async fn product_with_carton(pool: &SqlitePool, name: &str, purchase: &str, sale: &str) -> String {
    let mut i = base_input(name, purchase, sale);
    i.units = vec![ProductUnitInput { unit_id: CARTON.into(), factor: "12".into(), is_purchase_unit: true, is_sale_unit: true, ..Default::default() }];
    products::save(pool, &i).await.unwrap()
}

pub async fn product_kg(pool: &SqlitePool, name: &str, purchase: &str, sale: &str) -> String {
    let mut i = base_input(name, purchase, sale);
    i.base_unit_id = KG.into();
    products::save(pool, &i).await.unwrap()
}

pub async fn stock(pool: &SqlitePool, product_id: &str, qty: &str, unit_cost: &str) {
    adjust(
        pool,
        &AdjustInput {
            product_id: product_id.into(), variant_id: None, direction: "increase".into(), qty: qty.into(),
            unit_cost: Some(unit_cost.into()), note: String::new(), ..Default::default()
        },
    )
    .await
    .unwrap();
}
