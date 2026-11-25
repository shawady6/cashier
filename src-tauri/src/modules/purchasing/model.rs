use serde::{Deserialize, Serialize};

use crate::modules::sales::DiscountInput;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierInput {
    pub id: Option<String>,
    pub name_ar: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "yes")]
    pub is_active: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Supplier {
    pub id: String,
    pub name_ar: String,
    pub phone: String,
    pub address: String,
    pub notes: String,
    pub is_active: bool,
    /// ما علينا للمورد (موجب) = المشتريات المستلمة - المرتجعات - الدفعات
    pub balance_minor: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseItemInput {
    pub product_id: String,
    pub variant_id: Option<String>,
    /// وحدة الشراء. الفارغ = الوحدة الأساسية.
    pub unit_id: Option<String>,
    pub qty: String,
    /// تكلفة وحدة الشراء بالجنيه
    pub unit_cost: String,
    /// عدد القطع في وحدة الشراء لهذا البند فقط (مثلًا 14 بدل 24). فارغ = الافتراضي للمنتج.
    #[serde(default)]
    pub factor: Option<String>,
    /// سعر بيع وحدة الشراء. يُطبَّق عند الاستلام فقط.
    #[serde(default)]
    pub sale_price: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseInput {
    pub id: Option<String>,
    pub supplier_id: String,
    pub warehouse_id: String,
    #[serde(default)]
    pub supplier_invoice_no: String,
    #[serde(default)]
    pub note: String,
    pub discount: Option<DiscountInput>,
    pub items: Vec<PurchaseItemInput>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseFilter {
    pub supplier_id: Option<String>,
    pub status: Option<String>,
    pub query: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseRow {
    pub id: String,
    pub purchase_no: String,
    pub supplier_name: String,
    pub supplier_invoice_no: String,
    pub status: String,
    pub total_minor: i64,
    pub paid_minor: i64,
    pub returned_minor: i64,
    pub created_at: String,
    pub received_at: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseItemDetail {
    pub id: String,
    pub line_no: i64,
    pub product_id: String,
    pub variant_id: Option<String>,
    pub name_ar: String,
    pub variant_name: Option<String>,
    pub unit_id: String,
    pub unit_name: String,
    pub unit_factor_micro: i64,
    pub qty_milli: i64,
    pub base_qty_milli: i64,
    pub unit_cost_minor: i64,
    pub line_total_minor: i64,
    pub net_minor: i64,
    pub new_sale_price_minor: Option<i64>,
    pub returned_qty_milli: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PurchasePayment {
    pub id: String,
    pub method: String,
    pub amount_minor: i64,
    pub note: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseReturnRow {
    pub id: String,
    pub return_no: String,
    pub total_minor: i64,
    pub note: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseDetail {
    pub header: PurchaseRow,
    pub supplier_id: String,
    pub warehouse_id: String,
    pub warehouse_name: String,
    pub note: String,
    pub subtotal_minor: i64,
    pub discount_minor: i64,
    pub outstanding_minor: i64,
    pub items: Vec<PurchaseItemDetail>,
    pub payments: Vec<PurchasePayment>,
    pub returns: Vec<PurchaseReturnRow>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierPaymentInput {
    pub purchase_id: String,
    /// "cash" أو "card"
    pub method: String,
    pub amount: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseReturnItemInput {
    pub purchase_item_id: String,
    pub qty: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseReturnInput {
    pub purchase_id: String,
    pub items: Vec<PurchaseReturnItemInput>,
    #[serde(default)]
    pub note: String,
}
