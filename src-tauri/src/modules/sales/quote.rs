//! تسعير السلة: يقرأ الأسعار والوحدات من قاعدة البيانات (لا يثق بأي سعر من الواجهة)
//! ويحسب الخصومات والضريبة والإجمالي وتسوية الدفع. لا يكتب شيئًا في القاعدة.

use sqlx::SqliteConnection;

use crate::app_core::error::AppError;
use crate::app_core::money::parse_minor;
use crate::app_core::quantity::{parse_qty, to_base_qty, FACTOR_SCALE};

use super::calc::{allocate, discount_amount, line_gross, parse_discount, settle, tax_inclusive, unit_price_for, Discount};
use super::model::{CartInput, CartQuote, QuoteLine};

const MAX_LINES: usize = 200;

fn bad<T>(m: impl Into<String>) -> Result<T, AppError> {
    Err(AppError::Validation(m.into()))
}

#[derive(sqlx::FromRow)]
struct ProductRow {
    name_ar: String,
    sku: Option<String>,
    sale_price: i64,
    purchase_price: i64,
    is_active: bool,
    archived: bool,
    base_unit_id: String,
    base_unit_name: String,
    base_allows_decimal: bool,
}

pub async fn tax_rate_bp(conn: &mut SqliteConnection) -> Result<i64, AppError> {
    let v: Option<String> = sqlx::query_scalar("SELECT setting_value FROM settings WHERE setting_key = 'tax_rate_bp'")
        .fetch_optional(&mut *conn)
        .await?;
    Ok(v.and_then(|s| s.parse().ok()).unwrap_or(0))
}

pub async fn quote(conn: &mut SqliteConnection, input: &CartInput) -> Result<CartQuote, AppError> {
    if input.items.len() > MAX_LINES {
        return bad("عدد بنود الفاتورة كبير جدًا");
    }
    let rate = tax_rate_bp(conn).await?;
    let mut lines: Vec<QuoteLine> = Vec::new();
    let mut nets: Vec<i64> = Vec::new();

    for item in &input.items {
        let p = sqlx::query_as::<_, ProductRow>(
            "SELECT p.name_ar, p.sku, p.sale_price, p.purchase_price, p.is_active, (p.archived_at IS NOT NULL) AS archived, \
                    p.base_unit_id, u.name_ar AS base_unit_name, u.allows_decimal AS base_allows_decimal \
             FROM products p JOIN units u ON u.id = p.base_unit_id WHERE p.id = ?1",
        )
        .bind(&item.product_id)
        .fetch_optional(&mut *conn)
        .await?;
        let Some(p) = p else { return bad("منتج غير موجود في السلة") };
        if p.archived || !p.is_active {
            return bad(format!("المنتج «{}» غير متاح للبيع", p.name_ar));
        }

        // المتغير: المنتج ذو المتغيرات النشطة يُباع بمتغير فقط
        let (variant_name, sku, base_price, base_cost) = match &item.variant_id {
            Some(vid) => {
                let v: Option<(String, Option<String>, i64, i64, bool)> = sqlx::query_as(
                    "SELECT name_ar, sku, sale_price, purchase_price, is_active FROM product_variants \
                     WHERE id = ?1 AND product_id = ?2",
                )
                .bind(vid)
                .bind(&item.product_id)
                .fetch_optional(&mut *conn)
                .await?;
                let Some((vname, vsku, sale, cost, active)) = v else { return bad("متغير غير صالح") };
                if !active {
                    return bad(format!("المتغير «{vname}» غير متاح للبيع"));
                }
                (Some(vname), vsku.or(p.sku.clone()), sale, cost)
            }
            None => {
                let n: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM product_variants WHERE product_id = ?1 AND is_active = 1",
                )
                .bind(&item.product_id)
                .fetch_one(&mut *conn)
                .await?;
                if n > 0 {
                    return bad(format!("اختر متغيرًا للمنتج «{}»", p.name_ar));
                }
                (None, p.sku.clone(), p.sale_price, p.purchase_price)
            }
        };

        // وحدة البيع
        let (unit_id, unit_name, factor, allows_decimal) = match &item.unit_id {
            Some(uid) if *uid != p.base_unit_id => {
                let u: Option<(i64, String, bool)> = sqlx::query_as(
                    "SELECT pu.factor_micro, u.name_ar, u.allows_decimal FROM product_units pu \
                     JOIN units u ON u.id = pu.unit_id \
                     WHERE pu.product_id = ?1 AND pu.unit_id = ?2 AND pu.is_sale_unit = 1",
                )
                .bind(&item.product_id)
                .bind(uid)
                .fetch_optional(&mut *conn)
                .await?;
                let Some((f, n, d)) = u else { return bad("وحدة البيع غير مسموحة لهذا المنتج") };
                (uid.clone(), n, f, d)
            }
            _ => (p.base_unit_id.clone(), p.base_unit_name.clone(), FACTOR_SCALE, p.base_allows_decimal),
        };

        let qty = parse_qty(&item.qty, allows_decimal)?;
        if qty <= 0 {
            return bad(format!("كمية «{}» يجب أن تكون أكبر من صفر", p.name_ar));
        }
        let base_qty = to_base_qty(qty, factor)?;
        if base_qty <= 0 {
            return bad("الكمية صغيرة جدًا بعد تحويل الوحدة");
        }
        let unit_price = unit_price_for(base_price, factor);
        let gross = line_gross(unit_price, qty);
        let discount = discount_amount(gross, parse_discount(&item.discount)?)?;
        nets.push(gross - discount);
        lines.push(QuoteLine {
            product_id: item.product_id.clone(),
            variant_id: item.variant_id.clone(),
            name_ar: p.name_ar,
            variant_name,
            sku,
            unit_id,
            unit_name,
            unit_factor_micro: factor,
            qty_milli: qty,
            base_qty_milli: base_qty,
            unit_price,
            gross,
            discount,
            invoice_discount: 0,
            tax_rate_bp: rate,
            tax: 0,
            total: 0,
            fallback_unit_cost: base_cost,
        });
    }

    let net_sum: i64 = nets.iter().sum();
    let inv_disc_kind = parse_discount(&input.invoice_discount)?;
    let invoice_discount = if net_sum == 0 && inv_disc_kind != Discount::None && !matches!(inv_disc_kind, Discount::Amount(0)) {
        if matches!(inv_disc_kind, Discount::PercentBp(_)) { 0 } else { return bad("لا يوجد مبلغ لتطبيق الخصم عليه") }
    } else {
        discount_amount(net_sum, inv_disc_kind)?
    };
    let alloc = allocate(invoice_discount, &nets);
    for (i, l) in lines.iter_mut().enumerate() {
        l.invoice_discount = alloc[i];
        l.total = nets[i] - alloc[i];
        l.tax = tax_inclusive(l.total, rate);
    }

    let total: i64 = lines.iter().map(|l| l.total).sum();
    let (mut cash, mut card) = (0i64, 0i64);
    for pay in &input.payments {
        let amount = parse_minor(&pay.amount)?;
        if amount == 0 {
            continue;
        }
        match pay.method.as_str() {
            "cash" => cash += amount,
            "card" => card += amount,
            _ => return bad("طريقة دفع غير مدعومة"),
        }
    }
    let s = settle(total, cash, card)?;

    Ok(CartQuote {
        subtotal: lines.iter().map(|l| l.gross).sum(),
        item_discount: lines.iter().map(|l| l.discount).sum(),
        invoice_discount,
        tax: lines.iter().map(|l| l.tax).sum(),
        total,
        paid: s.paid,
        change: s.change,
        remaining: s.remaining,
        cash_net: s.cash_net,
        card: s.card,
        lines,
    })
}
