//! حسابات الفاتورة الصافية (بدون قاعدة بيانات): خصومات، ضريبة، توزيع، تسوية الدفع.
//!
//! سياسة التقريب: نصف لأعلى بعيدًا عن الصفر (`div_round`) على أصغر وحدة (قرش).
//! الضريبة **شاملة** في سعر البيع: ضريبة السطر = المبلغ × النسبة / (10000 + النسبة).
//! خصم الفاتورة يُوزَّع على السطور بنسبة صافي كل سطر بطريقة «أكبر باقي»، فمجموع التوزيع = الخصم بالضبط.

use crate::app_core::error::AppError;
use crate::app_core::money::{div_round, parse_minor, parse_percent_bp};

use super::model::DiscountInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discount {
    None,
    Amount(i64),
    PercentBp(i64),
}

pub fn parse_discount(d: &Option<DiscountInput>) -> Result<Discount, AppError> {
    let Some(d) = d else { return Ok(Discount::None) };
    if d.value.trim().is_empty() {
        return Ok(Discount::None);
    }
    match d.kind.as_str() {
        "amount" => Ok(Discount::Amount(parse_minor(&d.value)?)),
        "percent" => Ok(Discount::PercentBp(parse_percent_bp(&d.value)?)),
        _ => Err(AppError::Validation("نوع الخصم غير صالح".into())),
    }
}

pub fn discount_amount(base: i64, d: Discount) -> Result<i64, AppError> {
    let v = match d {
        Discount::None => 0,
        Discount::Amount(a) => a,
        Discount::PercentBp(bp) => div_round(i128::from(base) * i128::from(bp), 10_000) as i64,
    };
    if v > base {
        return Err(AppError::Validation("الخصم أكبر من قيمة البند".into()));
    }
    Ok(v)
}

/// سعر × كمية (بالألف) → مبلغ بالقروش.
pub fn line_gross(unit_price: i64, qty_milli: i64) -> i64 {
    div_round(i128::from(unit_price) * i128::from(qty_milli), 1000) as i64
}

/// سعر الوحدة الإضافية = سعر الأساسية × المعامل.
pub fn unit_price_for(base_price: i64, factor_micro: i64) -> i64 {
    div_round(i128::from(base_price) * i128::from(factor_micro), 1_000_000) as i64
}

pub fn tax_inclusive(amount: i64, rate_bp: i64) -> i64 {
    if rate_bp == 0 {
        return 0;
    }
    div_round(i128::from(amount) * i128::from(rate_bp), 10_000 + i128::from(rate_bp)) as i64
}

/// توزيع `total` على الأوزان بطريقة أكبر باقي. مجموع الناتج = `total` دائمًا.
pub fn allocate(total: i64, weights: &[i64]) -> Vec<i64> {
    let sum: i128 = weights.iter().map(|w| i128::from(*w)).sum();
    if total == 0 || sum == 0 {
        return vec![0; weights.len()];
    }
    let mut shares: Vec<(usize, i64, i128)> = weights
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let num = i128::from(total) * i128::from(*w);
            (i, (num / sum) as i64, num % sum)
        })
        .collect();
    let allocated: i64 = shares.iter().map(|s| s.1).sum();
    let mut left = total - allocated;
    let mut order: Vec<usize> = (0..shares.len()).collect();
    order.sort_by(|a, b| shares[*b].2.cmp(&shares[*a].2).then(a.cmp(b)));
    for i in order {
        if left == 0 {
            break;
        }
        if weights[i] > 0 {
            shares[i].1 += 1;
            left -= 1;
        }
    }
    shares.into_iter().map(|s| s.1).collect()
}

#[derive(Debug, PartialEq, Eq)]
pub struct Settlement {
    pub paid: i64,
    pub change: i64,
    pub remaining: i64,
    pub cash_net: i64,
    pub card: i64,
}

/// تسوية الدفع: الكارت لا يزيد عن المطلوب، والباقي كاش والفكة تُرد من الكاش.
pub fn settle(total: i64, cash: i64, card: i64) -> Result<Settlement, AppError> {
    if card > total {
        return Err(AppError::Validation("مبلغ الكارت أكبر من إجمالي الفاتورة".into()));
    }
    let paid = cash + card;
    let change = (paid - total).max(0);
    Ok(Settlement { paid, change, remaining: (total - paid).max(0), cash_net: cash - change, card })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_gross_handles_decimal_quantities() {
        assert_eq!(line_gross(1250, 1000), 1250);
        assert_eq!(line_gross(8000, 500), 4000); // نصف كيلو بسعر 80
        assert_eq!(line_gross(1000, 333), 333);
    }

    #[test]
    fn percent_and_amount_discounts() {
        assert_eq!(discount_amount(10_000, Discount::PercentBp(1000)).unwrap(), 1000);
        assert_eq!(discount_amount(999, Discount::PercentBp(1000)).unwrap(), 100); // 99.9 → 100
        assert_eq!(discount_amount(500, Discount::Amount(500)).unwrap(), 500);
        assert!(discount_amount(500, Discount::Amount(501)).is_err());
    }

    #[test]
    fn inclusive_tax_14_percent() {
        assert_eq!(tax_inclusive(11_400, 1400), 1400);
        assert_eq!(tax_inclusive(10_000, 0), 0);
    }

    #[test]
    fn allocation_always_sums_to_total() {
        for (total, weights) in [(100, vec![1, 1, 1]), (7, vec![3, 3, 3, 1]), (1, vec![5, 5]), (999, vec![1234, 5678, 91])] {
            let a = allocate(total, &weights);
            assert_eq!(a.iter().sum::<i64>(), total, "{total} {weights:?}");
        }
        assert_eq!(allocate(100, &[0, 0]), vec![0, 0]);
        assert_eq!(allocate(100, &[1, 3]), vec![25, 75]);
    }

    #[test]
    fn settlement_with_split_payment_and_change() {
        let s = settle(10_000, 6_000, 4_000).unwrap();
        assert_eq!((s.paid, s.change, s.remaining, s.cash_net), (10_000, 0, 0, 6_000));
        let s = settle(10_000, 12_000, 0).unwrap();
        assert_eq!((s.change, s.cash_net), (2_000, 10_000));
        let s = settle(10_000, 3_000, 0).unwrap();
        assert_eq!(s.remaining, 7_000);
        assert!(settle(10_000, 0, 10_001).is_err());
    }

    #[test]
    fn unit_price_scales_by_factor() {
        assert_eq!(unit_price_for(1250, 12_000_000), 15_000);
    }
}
