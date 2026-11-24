//! الكميات أعداد صحيحة بالألف (3 خانات عشرية): 1.5 كجم = 1500.
//! معاملات التحويل بين الوحدات أعداد صحيحة بالمليون: 1/12 ≈ 83333.
//! التقريب: نصف لأعلى (بعيدًا عن الصفر) عند التحويل، موثّق ومختبر.

use crate::app_core::error::AppError;
use crate::app_core::text::ascii_digits;

pub const QTY_SCALE: i64 = 1_000;
pub const FACTOR_SCALE: i64 = 1_000_000;
const MAX_QTY_MILLI: i64 = 1_000_000_000_000;

fn parse_scaled(input: &str, decimals: usize, what: &str) -> Result<i64, AppError> {
    let bad = || AppError::Validation(format!("{what} غير صالحة: «{}»", input.trim()));
    let s = ascii_digits(input.trim()).replace('٫', ".").replace(',', ".");
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i, f),
        None => (s.as_str(), ""),
    };
    let digits_only = |t: &str| t.chars().all(|c| c.is_ascii_digit());
    if s.is_empty()
        || (int_part.is_empty() && frac_part.is_empty())
        || !digits_only(int_part)
        || !digits_only(frac_part)
        || frac_part.len() > decimals
        || int_part.len() > 9
    {
        return Err(bad());
    }
    let whole: i64 = if int_part.is_empty() { 0 } else { int_part.parse().map_err(|_| bad())? };
    let mut frac_str = frac_part.to_string();
    while frac_str.len() < decimals {
        frac_str.push('0');
    }
    let frac: i64 = if frac_str.is_empty() { 0 } else { frac_str.parse().map_err(|_| bad())? };
    Ok(whole * 10_i64.pow(decimals as u32) + frac)
}

/// كمية بالألف. عند `allows_decimal = false` يجب أن تكون عددًا صحيحًا.
pub fn parse_qty(input: &str, allows_decimal: bool) -> Result<i64, AppError> {
    let v = parse_scaled(input, 3, "الكمية")?;
    if v > MAX_QTY_MILLI {
        return Err(AppError::Validation("الكمية أكبر من الحد المسموح".into()));
    }
    if !allows_decimal && v % QTY_SCALE != 0 {
        return Err(AppError::Validation("هذه الوحدة لا تقبل كسورًا".into()));
    }
    Ok(v)
}

/// معامل تحويل (يجب أن يكون أكبر من صفر) بالمليون.
pub fn parse_factor(input: &str) -> Result<i64, AppError> {
    let v = parse_scaled(input, 6, "معامل التحويل")?;
    if v <= 0 {
        return Err(AppError::Validation("معامل التحويل يجب أن يكون أكبر من صفر".into()));
    }
    Ok(v)
}

/// يحوّل كمية بوحدة ما (بالألف) إلى الوحدة الأساسية (بالألف).
/// مثال: 2 كرتونة (2000) × معامل 12 (12_000_000) = 24 قطعة (24000).
pub fn to_base_qty(qty_milli: i64, factor_micro: i64) -> Result<i64, AppError> {
    let product = i128::from(qty_milli) * i128::from(factor_micro);
    let scale = i128::from(FACTOR_SCALE);
    let rounded = if product >= 0 { (product + scale / 2) / scale } else { (product - scale / 2) / scale };
    i64::try_from(rounded).map_err(|_| AppError::Validation("ناتج التحويل كبير جدًا".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decimal_quantities() {
        assert_eq!(parse_qty("0.5", true).unwrap(), 500);
        assert_eq!(parse_qty("1.25", true).unwrap(), 1250);
        assert_eq!(parse_qty("٥", true).unwrap(), 5000);
    }

    #[test]
    fn whole_only_units_reject_fractions() {
        assert!(parse_qty("0.5", false).is_err());
        assert_eq!(parse_qty("3", false).unwrap(), 3000);
    }

    #[test]
    fn rejects_bad_quantities() {
        for v in ["", "-1", "1.2345", "x"] {
            assert!(parse_qty(v, true).is_err(), "should reject {v}");
        }
    }

    #[test]
    fn carton_of_twelve_converts_exactly() {
        let factor = parse_factor("12").unwrap();
        assert_eq!(to_base_qty(1000, factor).unwrap(), 12_000); // 1 كرتونة = 12 قطعة
        assert_eq!(to_base_qty(2500, factor).unwrap(), 30_000); // 2.5 كرتونة = 30 قطعة
    }

    #[test]
    fn piece_to_carton_base_rounds_to_nearest_milli() {
        let factor = parse_factor("0.083333").unwrap(); // قطعة = 1/12 كرتونة
        assert_eq!(to_base_qty(1000, factor).unwrap(), 83); // 0.083 كرتونة
        assert_eq!(to_base_qty(12_000, factor).unwrap(), 1000); // 12 قطعة ≈ كرتونة
    }

    #[test]
    fn gram_to_kg_is_exact() {
        let factor = parse_factor("0.001").unwrap();
        assert_eq!(to_base_qty(250_000, factor).unwrap(), 250); // 250 جم = 0.250 كجم
    }

    #[test]
    fn factor_must_be_positive() {
        assert!(parse_factor("0").is_err());
    }
}
