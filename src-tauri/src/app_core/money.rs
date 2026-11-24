//! الأموال أعداد صحيحة بالقروش (1 جنيه = 100 قرش). لا يوجد float في أي حساب.

use crate::app_core::error::AppError;
use crate::app_core::text::ascii_digits;

pub const MAX_MINOR: i64 = 999_999_999_99; // 999,999,999.99 جنيه

/// يحوّل نص مثل "12.5" أو "١٢٫٥٠" إلى قروش (1250). يرفض السالب وأكثر من خانتين عشريتين.
pub fn parse_minor(input: &str) -> Result<i64, AppError> {
    let bad = || AppError::Validation(format!("مبلغ غير صالح: «{}»", input.trim()));
    let s = ascii_digits(input.trim()).replace('٫', ".").replace(',', ".");
    if s.is_empty() {
        return Err(bad());
    }
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i, f),
        None => (s.as_str(), ""),
    };
    let digits_only = |t: &str| t.chars().all(|c| c.is_ascii_digit());
    if int_part.is_empty() && frac_part.is_empty()
        || !digits_only(int_part)
        || !digits_only(frac_part)
        || frac_part.len() > 2
        || int_part.len() > 12
    {
        return Err(bad());
    }
    let whole: i64 = if int_part.is_empty() { 0 } else { int_part.parse().map_err(|_| bad())? };
    let frac: i64 = match frac_part.len() {
        0 => 0,
        1 => frac_part.parse::<i64>().map_err(|_| bad())? * 10,
        _ => frac_part.parse::<i64>().map_err(|_| bad())?,
    };
    let total = whole.checked_mul(100).and_then(|v| v.checked_add(frac)).ok_or_else(bad)?;
    if total > MAX_MINOR {
        return Err(AppError::Validation("المبلغ أكبر من الحد المسموح".into()));
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_formats() {
        assert_eq!(parse_minor("12").unwrap(), 1200);
        assert_eq!(parse_minor("12.5").unwrap(), 1250);
        assert_eq!(parse_minor("12.05").unwrap(), 1205);
        assert_eq!(parse_minor("١٢٫٥٠").unwrap(), 1250);
        assert_eq!(parse_minor("0.99").unwrap(), 99);
        assert_eq!(parse_minor(".5").unwrap(), 50);
    }

    #[test]
    fn rejects_invalid_values() {
        for v in ["", "-1", "1.234", "abc", "1.2.3", ".", "1e3", "9999999999999"] {
            assert!(parse_minor(v).is_err(), "should reject {v}");
        }
    }

    #[test]
    fn has_no_floating_point_drift() {
        // 0.1 + 0.2 كأعداد صحيحة بالقروش = 30 بالضبط
        assert_eq!(parse_minor("0.1").unwrap() + parse_minor("0.2").unwrap(), parse_minor("0.3").unwrap());
    }
}
