//! تحقق وتحليل مدخلات المنتج (بدون قاعدة بيانات). كل الأرقام تُحلَّل هنا في Rust.

use std::collections::HashSet;

use crate::app_core::error::AppError;
use crate::app_core::money::parse_minor;
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
    pub barcode: Option<String>,
}

pub struct Parsed<'a> {
    pub src: &'a ProductInput,
    pub name_ar: String,
    pub name_en: String,
    pub sku: Option<String>,
    pub purchase_price: i64,
    pub sale_price: i64,
    pub barcodes: Vec<String>,
    pub variants: Vec<ParsedVariant<'a>>,
    pub units: Vec<ParsedUnit<'a>>,
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
        units.push(ParsedUnit { src: u, factor_micro, sale_price, barcode });
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

    Ok(Parsed {
        src: input,
        name_ar,
        name_en,
        sku,
        purchase_price: parse_minor(&input.purchase_price)?,
        sale_price: parse_minor(&input.sale_price)?,
        barcodes,
        variants,
        units,
    })
}
