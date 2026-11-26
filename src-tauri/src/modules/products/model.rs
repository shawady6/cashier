//! نماذج الإدخال والإخراج للمنتجات. الأموال بالقروش والكميات بالألف في كل المخرجات.

use serde::{Deserialize, Serialize};

// ---------- الإدخال (الأرقام تصل نصوصًا وتُحلَّل في Rust) ----------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInput {
    pub id: Option<String>,
    pub sku: Option<String>,
    pub name_ar: String,
    #[serde(default)]
    pub name_en: String,
    pub category_id: Option<String>,
    pub brand_id: Option<String>,
    pub base_unit_id: String,
    /// سعر شراء القطعة (اختياري): لو فارغ يُحسب من سعر شراء الوحدة الكبيرة.
    #[serde(default)]
    pub purchase_price: String,
    /// سعر بيع القطعة يدويًا (اختياري): لو فارغ يُحسب من سعر الوحدة الكبيرة أو الهامش.
    #[serde(default)]
    pub sale_price: String,
    /// هامش الربح % على التكلفة، يُستخدم لحساب سعر البيع الناقص فقط.
    #[serde(default)]
    pub margin_percent: Option<String>,
    #[serde(default = "yes")]
    pub buy_in_base: bool,
    #[serde(default = "yes")]
    pub sell_in_base: bool,
    #[serde(default)]
    pub default_buy_big: bool,
    #[serde(default)]
    pub default_sell_big: bool,
    #[serde(default = "zero")]
    pub min_stock_qty: String,
    #[serde(default)]
    pub is_weighted: bool,
    #[serde(default)]
    pub track_expiry: bool,
    #[serde(default = "yes")]
    pub is_active: bool,
    #[serde(default)]
    pub barcodes: Vec<String>,
    #[serde(default)]
    pub variants: Vec<VariantInput>,
    #[serde(default)]
    pub units: Vec<ProductUnitInput>,
}

fn zero() -> String {
    "0".into()
}
fn yes() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantInput {
    pub id: Option<String>,
    pub name_ar: String,
    #[serde(default)]
    pub name_en: String,
    pub sku: Option<String>,
    pub purchase_price: String,
    pub sale_price: String,
    #[serde(default = "yes")]
    pub is_active: bool,
    #[serde(default)]
    pub barcodes: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductUnitInput {
    pub unit_id: String,
    /// كم وحدة أساسية تساوي وحدة واحدة من هذه الوحدة (مثال: كرتونة = 12).
    pub factor: String,
    #[serde(default)]
    pub is_purchase_unit: bool,
    #[serde(default)]
    pub is_sale_unit: bool,
    /// سعر بيع الوحدة الكبيرة بالجنيه. فارغ = سعر القطعة × العدد.
    #[serde(default)]
    pub sale_price: Option<String>,
    /// باركود الوحدة الكبيرة (اختياري).
    #[serde(default)]
    pub barcode: Option<String>,
    /// سعر شراء (جملة) الوحدة الكبيرة بالجنيه.
    #[serde(default)]
    pub purchase_price: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductFilter {
    pub query: Option<String>,
    pub category_id: Option<String>,
    pub brand_id: Option<String>,
    #[serde(default)]
    pub include_archived: bool,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ---------- الإخراج ----------

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProductListItem {
    pub id: String,
    pub sku: Option<String>,
    pub name_ar: String,
    pub name_en: String,
    pub category_name: Option<String>,
    pub brand_name: Option<String>,
    pub base_unit_name: String,
    pub purchase_price: i64,
    pub sale_price: i64,
    pub min_stock_qty: i64,
    pub is_active: bool,
    pub archived: bool,
    pub variant_count: i64,
    pub first_barcode: Option<String>,
    /// الرصيد الحالي بالألف (بالوحدة الأساسية) في كل المخازن
    pub stock_qty: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductDetail {
    pub id: String,
    pub sku: Option<String>,
    pub name_ar: String,
    pub name_en: String,
    pub category_id: Option<String>,
    pub brand_id: Option<String>,
    pub base_unit_id: String,
    pub purchase_price: i64,
    pub sale_price: i64,
    pub min_stock_qty: i64,
    pub is_weighted: bool,
    pub track_expiry: bool,
    pub buy_in_base: bool,
    pub sell_in_base: bool,
    pub default_buy_big: bool,
    pub default_sell_big: bool,
    pub price_source: String,
    pub is_active: bool,
    pub archived: bool,
    pub has_image: bool,
    pub barcodes: Vec<String>,
    pub variants: Vec<VariantDetail>,
    pub units: Vec<ProductUnitDetail>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantDetail {
    pub id: String,
    pub name_ar: String,
    pub name_en: String,
    pub sku: Option<String>,
    pub purchase_price: i64,
    pub sale_price: i64,
    pub is_active: bool,
    pub barcodes: Vec<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProductUnitDetail {
    pub unit_id: String,
    pub factor_micro: i64,
    pub is_purchase_unit: bool,
    pub is_sale_unit: bool,
    pub sale_price_minor: Option<i64>,
    pub purchase_price_minor: Option<i64>,
    pub barcode: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub id: String,
    pub code: String,
    pub name_ar: String,
    pub name_en: String,
    pub allows_decimal: bool,
    pub is_system: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitInput {
    pub name_ar: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub allows_decimal: bool,
}

/// نتيجة البحث بالباركود (يستخدمها الكاشير لاحقًا).
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BarcodeMatch {
    pub product_id: String,
    pub variant_id: Option<String>,
    pub name_ar: String,
    pub variant_name_ar: Option<String>,
    pub sale_price: i64,
    pub is_active: bool,
    pub archived: bool,
    /// لو الباركود يخص وحدة كبيرة
    pub unit_id: Option<String>,
}

/// عنصر قابل للبيع في الكاشير: منتج عادي أو متغير منتج.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sellable {
    pub product_id: String,
    pub variant_id: Option<String>,
    pub name_ar: String,
    pub variant_name: Option<String>,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub sale_price: i64,
    pub stock_qty: i64,
    pub base_unit_id: String,
    pub base_unit_name: String,
    pub allows_decimal: bool,
    pub units: Vec<SellUnit>,
    /// الوحدة التي طابقها باركود الوحدة الكبيرة (يضيفها الكاشير بهذه الوحدة)
    pub matched_unit_id: Option<String>,
    /// الوحدة التي يضيف بها الكاشير هذا الصنف عند الاختيار (حسب إعداد المنتج)
    pub default_unit_id: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SellUnit {
    pub unit_id: String,
    pub name_ar: String,
    pub factor_micro: i64,
    pub allows_decimal: bool,
    /// سعر بيع هذه الوحدة (NULL = سعر القطعة × المعامل)
    pub sale_price_minor: Option<i64>,
}
