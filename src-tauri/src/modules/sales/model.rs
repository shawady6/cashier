use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscountInput {
    /// "amount" أو "percent"
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CartItemInput {
    pub product_id: String,
    pub variant_id: Option<String>,
    /// وحدة البيع. الفارغ = الوحدة الأساسية.
    pub unit_id: Option<String>,
    pub qty: String,
    pub discount: Option<DiscountInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInput {
    /// "cash" أو "card"
    pub method: String,
    pub amount: String,
    #[serde(default)]
    pub reference: String,
}

/// السعر لا يأتي من الواجهة أبدًا: يُقرأ من قاعدة البيانات في Rust.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CartInput {
    pub items: Vec<CartItemInput>,
    pub invoice_discount: Option<DiscountInput>,
    #[serde(default)]
    pub payments: Vec<PaymentInput>,
    /// يمنع تكرار الفاتورة عند الضغط المزدوج أو إعادة المحاولة.
    pub client_request_id: Option<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteLine {
    pub product_id: String,
    pub variant_id: Option<String>,
    pub name_ar: String,
    pub variant_name: Option<String>,
    pub sku: Option<String>,
    pub unit_id: String,
    pub unit_name: String,
    pub unit_factor_micro: i64,
    pub qty_milli: i64,
    pub base_qty_milli: i64,
    pub unit_price: i64,
    pub gross: i64,
    pub discount: i64,
    pub invoice_discount: i64,
    pub tax_rate_bp: i64,
    pub tax: i64,
    pub total: i64,
    /// سعر الشراء الحالي للوحدة الأساسية (للتكلفة الاحتياطية فقط).
    #[serde(skip)]
    pub fallback_unit_cost: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartQuote {
    pub lines: Vec<QuoteLine>,
    pub subtotal: i64,
    pub item_discount: i64,
    pub invoice_discount: i64,
    pub tax: i64,
    pub total: i64,
    pub paid: i64,
    pub change: i64,
    pub remaining: i64,
    pub cash_net: i64,
    pub card: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutResult {
    pub sale_id: String,
    pub invoice_no: String,
    pub total: i64,
    pub change: i64,
    /// true لو الطلب مكرر ورجعنا الفاتورة الأصلية دون إنشاء جديدة.
    pub duplicate: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnItemInput {
    pub sale_item_id: String,
    pub qty: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnInput {
    pub sale_id: String,
    pub items: Vec<ReturnItemInput>,
    #[serde(default)]
    pub reason: String,
    /// "cash" أو "card"
    pub refund_method: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnResult {
    pub return_id: String,
    pub return_no: String,
    pub refund: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleFilter {
    pub query: Option<String>,
    /// YYYY-MM-DD بتوقيت الجهاز
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SaleListItem {
    pub id: String,
    pub invoice_no: String,
    pub created_at: String,
    pub total_minor: i64,
    pub payment_status: String,
    pub item_count: i64,
    pub returned_minor: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SaleItemDetail {
    pub id: String,
    pub line_no: i64,
    pub name_ar: String,
    pub variant_name: Option<String>,
    pub unit_name: String,
    pub qty_milli: i64,
    pub unit_price_minor: i64,
    pub gross_minor: i64,
    pub discount_minor: i64,
    pub invoice_discount_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub returned_qty_milli: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDetail {
    pub method: String,
    pub amount_minor: i64,
    pub reference: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ReturnSummary {
    pub id: String,
    pub return_no: String,
    pub refund_minor: i64,
    pub refund_method: String,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleDetail {
    pub id: String,
    pub invoice_no: String,
    pub created_at: String,
    pub payment_status: String,
    pub subtotal_minor: i64,
    pub item_discount_minor: i64,
    pub invoice_discount_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub paid_minor: i64,
    pub change_minor: i64,
    pub note: String,
    pub items: Vec<SaleItemDetail>,
    pub payments: Vec<PaymentDetail>,
    pub returns: Vec<ReturnSummary>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SuspendedSale {
    pub id: String,
    pub label: String,
    pub payload: String,
    pub created_at: String,
}
