//! تحقق وتحليل مدخلات المنتج (بدون قاعدة بيانات). كل الأرقام تُحلَّل هنا في Rust.

use std::collections::HashSet;

use crate::app_core::error::AppError;
use crate::app_core::money::{div_round, parse_minor, parse_percent_bp};
use crate::app_core::quantity::parse_factor;
use crate::app_core::text::ascii_digits;

use super::model::{ProductInput, ProductUnitInput, VariantInput};

pub fn err<T>(msg: impl Into<String>) -> Result<T, AppError> {
    Err(AppError::Validation(msg.into()))
}

pub fn clean_sku(sku: &Option<String>) -> Result<Option<String>, AppError> {
    let Some(raw) = sku else { return Ok(None) };
    let s = ascii_digits(raw.trim());
    if s.is_empty() {
        return Ok(None);
    }
    if s.chars().count() > 50 || s.chars().any(char::is_whitespace) {
        return err("رمز الصنف (SKU) يجب ألا يحتوي مسافات ولا يزيد عن 50 حرفًا");
    }
    Ok(Some(s))
}

pub fn clean_barcode(raw: &str) -> Result<String, AppError> {
    let s = ascii_digits(raw.trim());
    if s.is_empty() || s.chars().count() > 64 || s.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return err(format!("باركود غير صالح: «{}»", raw.trim()));
    }
    Ok(s)
}

fn clean_name(name: &str, what: &str) -> Result<String, AppError> {
    let n = name.trim().to_string();
    if n.is_empty() {
        return err(format!("{what} مطلوب"));
    }
    if n.chars().count() > 150 {
        return err(format!("{what} أطول من الحد المسموح"));
    }
    Ok(n)
}

pub struct ParsedVariant<'a> {
    pub src: &'a VariantInput,
    pub name_ar: String,
    pub name_en: String,
    pub sku: Option<String>,
    pub purchase_price: i64,
    pub sale_price: i64,
    pub barcodes: Vec<String>,
}

pub struct ParsedUnit<'a> {
    pub src: &'a ProductUnitInput,
    pub factor_micro: i64,
    pub sale_price: Option<i64>,
    pub purchase_price: Option<i64>,
    pub barcode: Option<String>,
}

pub struct Parsed<'a> {
    pub src: &'a ProductInput,
    pub name_ar: String,
    pub name_en: String,
    pub sku: Option<String>,
    pub purchase_price: i64,
    pub sale_price: i64,
    /// manual أو derived (محسوب من سعر الوحدة الكبيرة ويُعاد حسابه عند تغيّره)
    pub price_source: &'static str,
    pub barcodes: Vec<String>,
    pub variants: Vec<ParsedVariant<'a>>,
    pub units: Vec<ParsedUnit<'a>>,
}

/// قسمة صحيحة بتقريب لأعلى (للأرقام الموجبة). كل سعر قطعة محسوب يمر من هنا.
pub fn ceil_div(num: i128, den: i128) -> i128 {
    assert!(den > 0 && num >= 0);
    (num + den - 1) / den
}

/// سعر القطعة المحسوب من سعر الوحدة الكبيرة = السعر ÷ عدد القطع، مقرّبًا لأعلى قرش.
pub fn piece_price_from_big(big_price_minor: i64, factor_micro: i64) -> i64 {
    ceil_div(i128::from(big_price_minor) * 1_000_000, i128::from(factor_micro)) as i64
}

/// تكلفة القطعة من سعر شراء الوحدة الكبيرة (للعرض كمرجع فقط؛ المخزون يحمل القيمة الكاملة).
pub fn piece_cost_from_big(big_cost_minor: i64, factor_micro: i64) -> i64 {
    div_round(i128::from(big_cost_minor) * 1_000_000, i128::from(factor_micro)) as i64
}

pub struct Resolved {
    pub piece_price: i64,
    pub big_sale_price: Option<i64>,
    pub source: &'static str,
}

/// ترتيب مصادر الأسعار: سعر القطعة اليدوي ← سعر بيع الوحدة الكبيرة (÷ العدد) ← هامش الربح على التكلفة.
/// الهامش يُستخدم فقط لملء سعر ناقص، ويُقرَّب دائمًا لأعلى.
pub fn resolve_prices(
    manual_piece: Option<i64>,
    big: Option<(i64, Option<i64>, Option<i64>)>, // (factor_micro, sale, purchase)
    margin_bp: Option<i64>,
    piece_cost: i64,
) -> Option<Resolved> {
    let big_sale = match big {
        Some((_, Some(s), _)) => Some(s),
        Some((_, None, Some(cost))) => {
            margin_bp.map(|m| ceil_div(i128::from(cost) * i128::from(10_000 + m), 10_000) as i64)
        }
        _ => None,
    };
    if let Some(m) = manual_piece {
        return Some(Resolved { piece_price: m, big_sale_price: big_sale, source: "manual" });
    }
    if let (Some(s), Some((f, _, _))) = (big_sale, big) {
        return Some(Resolved { piece_price: piece_price_from_big(s, f), big_sale_price: big_sale, source: "derived" });
    }
    if let (Some(m), true) = (margin_bp, piece_cost > 0) {
        let p = ceil_div(i128::from(piece_cost) * i128::from(10_000 + m), 10_000) as i64;
        return Some(Resolved { piece_price: p, big_sale_price: None, source: "manual" });
    }
    None
}

pub fn parse(input: &ProductInput) -> Result<Parsed<'_>, AppError> {
    let name_ar = clean_name(&input.name_ar, "اسم المنتج بالعربية")?;
    let name_en = input.name_en.trim().to_string();
    if name_en.chars().count() > 150 {
        return err("الاسم الإنجليزي أطول من الحد المسموح");
    }

    let mut seen_barcodes = HashSet::new();
    let mut all_barcodes = |list: &[String]| -> Result<Vec<String>, AppError> {
        let mut out = Vec::new();
        for raw in list {
            let b = clean_barcode(raw)?;
            if !seen_barcodes.insert(b.clone()) {
                return err(format!("الباركود {b} مكرر داخل نفس المنتج"));
            }
            out.push(b);
        }
        Ok(out)
    };

    let barcodes = all_barcodes(&input.barcodes)?;

    let mut variants = Vec::new();
    let mut seen_variant_names = HashSet::new();
    for v in &input.variants {
        let vname = clean_name(&v.name_ar, "اسم المتغير")?;
        if !seen_variant_names.insert(crate::app_core::text::normalize(&vname)) {
            return err(format!("اسم المتغير «{vname}» مكرر"));
        }
        variants.push(ParsedVariant {
            src: v,
            name_en: v.name_en.trim().to_string(),
            sku: clean_sku(&v.sku)?,
            purchase_price: parse_minor(&v.purchase_price)?,
            sale_price: parse_minor(&v.sale_price)?,
            barcodes: all_barcodes(&v.barcodes)?,
            name_ar: vname,
        });
    }

    let mut seen_units = HashSet::new();
    let mut units = Vec::new();
    for u in &input.units {
        if u.unit_id == input.base_unit_id {
            return err("لا يمكن إضافة الوحدة الأساسية كوحدة إضافية");
        }
        if !seen_units.insert(u.unit_id.clone()) {
            return err("وحدة مكررة في قائمة الوحدات");
        }
        let factor_micro = parse_factor(&u.factor)?;
        if factor_micro < 2 * 1_000_000 || factor_micro % 1_000_000 != 0 {
            return err("عدد القطع في الوحدة الكبيرة يجب أن يكون رقمًا صحيحًا 2 أو أكثر");
        }
        let sale_price = match u.sale_price.as_deref().map(str::trim) {
            Some(p) if !p.is_empty() => Some(parse_minor(p)?),
            _ => None,
        };
        let barcode = match u.barcode.as_deref().map(str::trim) {
            Some(b) if !b.is_empty() => {
                let b = clean_barcode(b)?;
                if !seen_barcodes.insert(b.clone()) {
                    return err(format!("الباركود {b} مكرر داخل نفس المنتج"));
                }
                Some(b)
            }
            _ => None,
        };
        let purchase_price = match u.purchase_price.as_deref().map(str::trim) {
            Some(p) if !p.is_empty() => Some(parse_minor(p)?),
            _ => None,
        };
        units.push(ParsedUnit { src: u, factor_micro, sale_price, purchase_price, barcode });
    }
    if units.len() > 1 {
        return err("يسمح بوحدة كبيرة واحدة فقط لكل منتج");
    }
    if units.iter().any(|u| u.barcode.is_some()) && !variants.is_empty() {
        return err("باركود الوحدة الكبيرة غير متاح للمنتجات ذات المتغيرات");
    }

    // SKU مكرر داخل نفس الطلب (منتج + متغيراته)
    let mut skus = HashSet::new();
    let sku = clean_sku(&input.sku)?;
    for s in sku.iter().chain(variants.iter().filter_map(|v| v.sku.as_ref())) {
        if !skus.insert(s.clone()) {
            return err(format!("رمز الصنف {s} مكرر داخل نفس المنتج"));
        }
    }

    // خيارات الشراء والبيع بالوحدات
    let big = units.first();
    let buy_big = big.map_or(false, |u| u.src.is_purchase_unit);
    let sell_big = big.map_or(false, |u| u.src.is_sale_unit);
    if !input.buy_in_base && !buy_big {
        return err("اختر وحدة شراء واحدة على الأقل");
    }
    if !input.sell_in_base && !sell_big {
        return err("اختر وحدة بيع واحدة على الأقل");
    }
    if input.default_buy_big && !buy_big {
        return err("الوحدة الكبيرة غير مفعّلة للشراء");
    }
    if input.default_sell_big && !sell_big {
        return err("الوحدة الكبيرة غير مفعّلة للبيع");
    }
    if !input.buy_in_base && !input.default_buy_big {
        return err("وحدة الشراء الافتراضية غير متاحة للشراء");
    }
    if !input.sell_in_base && !input.default_sell_big {
        return err("وحدة البيع الافتراضية غير متاحة للبيع");
    }

    // الأسعار: سعر القطعة يُحسب تلقائيًا ولا يُطلب إدخاله
    let manual_piece = match input.sale_price.trim() {
        "" => None,
        v => Some(parse_minor(v)?),
    };
    let margin_bp = match input.margin_percent.as_deref().map(str::trim) {
        Some(m) if !m.is_empty() => Some(parse_percent_bp(m)?),
        _ => None,
    };
    let big_cost = big.and_then(|u| u.purchase_price);
    let typed_cost = match input.purchase_price.trim() {
        "" => None,
        v => Some(parse_minor(v)?),
    };
    let purchase_price = match (typed_cost, big.zip(big_cost)) {
        (Some(c), _) => c,
        (None, Some((u, c))) => piece_cost_from_big(c, u.factor_micro),
        (None, None) => 0,
    };
    let resolved = resolve_prices(
        manual_piece,
        big.map(|u| (u.factor_micro, u.sale_price, u.purchase_price)),
        margin_bp,
        purchase_price,
    );
    let (sale_price, source) = match &resolved {
        Some(r) => (r.piece_price, r.source),
        // منتج بمتغيرات: لكل متغير سعره، فسعر المنتج نفسه اختياري
        None if !variants.is_empty() => (0, "manual"),
        None => return err("حدد سعر البيع: سعر الوحدة الكبيرة أو سعر القطعة أو هامش الربح"),
    };
    if let (Some(r), Some(u)) = (&resolved, units.first_mut()) {
        if u.sale_price.is_none() {
            u.sale_price = r.big_sale_price;
        }
    }

    Ok(Parsed {
        src: input,
        name_ar,
        name_en,
        sku,
        purchase_price,
        sale_price,
        price_source: source,
        barcodes,
        variants,
        units,
    })
}
