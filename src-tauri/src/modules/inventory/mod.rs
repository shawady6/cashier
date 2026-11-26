//! المخزون الأساسي (المرحلة 2): أرصدة وحركات بتكلفة المتوسط المرجّح.
//! القيمة الإجمالية للرصيد تُخزَّن كعدد صحيح (قروش) فلا يوجد float ولا فروق تقريب تراكمية:
//! متوسط التكلفة = value / qty، وعند بيع كل الرصيد تُخصم القيمة كاملة.
//! المرحلة 3 تضيف المخازن المتعددة والنقل والجرد فوق نفس الجداول.

pub mod counts;
pub mod reports;
pub mod transfers;
pub mod warehouses;

#[cfg(test)]
mod stock_tests;

use serde::Deserialize;
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::money::{div_round, parse_minor};
use crate::app_core::quantity::{parse_qty, to_base_qty, FACTOR_SCALE};

pub async fn default_warehouse(conn: &mut SqliteConnection) -> Result<String, AppError> {
    let id: Option<String> =
        sqlx::query_scalar("SELECT id FROM warehouses WHERE is_default = 1 AND is_active = 1 LIMIT 1")
            .fetch_optional(&mut *conn)
            .await?;
    id.ok_or_else(|| AppError::Internal("لا يوجد مخزن افتراضي".into()))
}

pub async fn allow_negative(conn: &mut SqliteConnection) -> Result<bool, AppError> {
    let v: Option<String> =
        sqlx::query_scalar("SELECT setting_value FROM settings WHERE setting_key = 'allow_negative_stock'")
            .fetch_optional(&mut *conn)
            .await?;
    Ok(v.as_deref() == Some("1"))
}

pub struct ProductInfo {
    pub name: String,
    pub variant_name: Option<String>,
    pub allows_decimal: bool,
    pub base_unit_name: String,
    /// سعر الشراء الحالي للوحدة الأساسية (تكلفة احتياطية)
    pub fallback_cost: i64,
}

/// بيانات منتج/متغير للتحقق. ترفض منتجًا غير موجود أو متغيرًا لا يخصه.
pub async fn product_info(
    conn: &mut SqliteConnection,
    product: &str,
    variant: Option<&str>,
) -> Result<ProductInfo, AppError> {
    let row: Option<(String, bool, String, i64)> = sqlx::query_as(
        "SELECT p.name_ar, u.allows_decimal, u.name_ar, p.purchase_price FROM products p \
         JOIN units u ON u.id = p.base_unit_id WHERE p.id = ?1",
    )
    .bind(product)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((name, allows_decimal, base_unit_name, mut fallback_cost)) = row else {
        return Err(AppError::Validation("منتج غير موجود".into()));
    };
    let mut variant_name = None;
    if let Some(v) = variant {
        let vr: Option<(String, i64)> =
            sqlx::query_as("SELECT name_ar, purchase_price FROM product_variants WHERE id = ?1 AND product_id = ?2")
                .bind(v)
                .bind(product)
                .fetch_optional(&mut *conn)
                .await?;
        let Some((vn, vc)) = vr else {
            return Err(AppError::Validation("المتغير لا يخص هذا المنتج".into()));
        };
        variant_name = Some(vn);
        fallback_cost = vc;
    }
    Ok(ProductInfo { name, variant_name, allows_decimal, base_unit_name, fallback_cost })
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Balance {
    pub qty: i64,
    pub value: i64,
}

pub async fn balance(
    conn: &mut SqliteConnection,
    warehouse: &str,
    product: &str,
    variant: Option<&str>,
) -> Result<Balance, AppError> {
    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT qty_milli, value_minor FROM stock_balances \
         WHERE warehouse_id = ?1 AND product_id = ?2 AND variant_id IS ?3",
    )
    .bind(warehouse)
    .bind(product)
    .bind(variant)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.map(|(qty, value)| Balance { qty, value }).unwrap_or_default())
}

async fn write_balance(
    conn: &mut SqliteConnection,
    warehouse: &str,
    product: &str,
    variant: Option<&str>,
    b: Balance,
) -> Result<(), AppError> {
    let updated = sqlx::query(
        "UPDATE stock_balances SET qty_milli = ?4, value_minor = ?5, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
         WHERE warehouse_id = ?1 AND product_id = ?2 AND variant_id IS ?3",
    )
    .bind(warehouse)
    .bind(product)
    .bind(variant)
    .bind(b.qty)
    .bind(b.value)
    .execute(&mut *conn)
    .await?;
    if updated.rows_affected() == 0 {
        sqlx::query(
            "INSERT INTO stock_balances (id, warehouse_id, product_id, variant_id, qty_milli, value_minor) \
             VALUES (?1,?2,?3,?4,?5,?6)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(warehouse)
        .bind(product)
        .bind(variant)
        .bind(b.qty)
        .bind(b.value)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub struct MovementRef<'a> {
    pub movement_type: &'a str,
    pub ref_type: &'a str,
    pub ref_id: &'a str,
    pub note: &'a str,
}

async fn record_movement(
    conn: &mut SqliteConnection,
    warehouse: &str,
    product: &str,
    variant: Option<&str>,
    qty_delta: i64,
    value_delta: i64,
    r: &MovementRef<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO stock_movements (id, warehouse_id, product_id, variant_id, movement_type, qty_delta_milli, \
         value_delta_minor, ref_type, ref_id, note) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(warehouse)
    .bind(product)
    .bind(variant)
    .bind(r.movement_type)
    .bind(qty_delta)
    .bind(value_delta)
    .bind(r.ref_type)
    .bind(r.ref_id)
    .bind(r.note)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// إدخال كمية بقيمة إجمالية (يرفع المتوسط المرجّح تلقائيًا).
pub async fn receive(
    conn: &mut SqliteConnection,
    warehouse: &str,
    product: &str,
    variant: Option<&str>,
    qty: i64,
    value: i64,
    r: &MovementRef<'_>,
) -> Result<(), AppError> {
    let b = balance(conn, warehouse, product, variant).await?;
    write_balance(conn, warehouse, product, variant, Balance { qty: b.qty + qty, value: b.value + value }).await?;
    record_movement(conn, warehouse, product, variant, qty, value, r).await
}

/// إخراج كمية بمتوسط التكلفة ويرجع التكلفة. يرفض لو الرصيد غير كافٍ والسالب غير مسموح.
/// عند السماح بالسالب: الجزء غير المتاح يُكلَّف بـ `fallback_unit_cost` (سعر الشراء لكل وحدة أساسية).
pub async fn issue(
    conn: &mut SqliteConnection,
    warehouse: &str,
    product: &str,
    variant: Option<&str>,
    qty: i64,
    fallback_unit_cost: i64,
    allow_neg: bool,
    r: &MovementRef<'_>,
    product_name: &str,
) -> Result<i64, AppError> {
    let b = balance(conn, warehouse, product, variant).await?;
    let available = b.qty.max(0);
    let (cost, value_removed) = if qty <= available {
        let c = if qty == b.qty { b.value } else { div_round(i128::from(b.value) * i128::from(qty), i128::from(b.qty)) as i64 };
        (c, c)
    } else {
        if !allow_neg {
            return Err(AppError::Validation(format!(
                "الكمية المطلوبة من «{product_name}» أكبر من الرصيد المتاح"
            )));
        }
        let from_stock = if available > 0 { b.value } else { 0 };
        let rest = div_round(i128::from(fallback_unit_cost) * i128::from(qty - available), 1000) as i64;
        (from_stock + rest, from_stock)
    };
    write_balance(conn, warehouse, product, variant, Balance { qty: b.qty - qty, value: b.value - value_removed }).await?;
    record_movement(conn, warehouse, product, variant, -qty, -value_removed, r).await?;
    Ok(cost)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjustInput {
    pub product_id: String,
    pub variant_id: Option<String>,
    /// "increase" أو "decrease"
    pub direction: String,
    /// بوحدة الإدخال المختارة (الأساسية لو `unit_id` فارغ)
    pub qty: String,
    /// وحدة الإدخال. الفارغ = الوحدة الأساسية.
    #[serde(default)]
    pub unit_id: Option<String>,
    /// عدد القطع في الوحدة لهذه العملية فقط (يغيّر الافتراضي للمنتج مؤقتًا). رقم صحيح 2 أو أكثر.
    #[serde(default)]
    pub factor: Option<String>,
    /// تكلفة وحدة الإدخال (للزيادة فقط). الافتراضي سعر الشراء الحالي للقطعة.
    pub unit_cost: Option<String>,
    /// سعر بيع وحدة الإدخال (للزيادة فقط). يحدّث سعر البيع الحالي لهذه الوحدة.
    #[serde(default)]
    pub sale_price: Option<String>,
    #[serde(default)]
    pub note: String,
}

/// الوحدة المختارة لعملية إدخال: (معرّف الوحدة، المعامل بالمليون، تسمح بالكسور؟)
pub struct ResolvedUnit {
    pub unit_id: String,
    pub unit_name: String,
    pub factor_micro: i64,
    pub allows_decimal: bool,
    pub is_base: bool,
}

/// تحديد وحدة الإدخال والمعامل الفعلي. `purchase_only` تشترط أن تكون الوحدة وحدة شراء.
/// المعامل المُمرَّر يغيّر القيمة لهذه العملية فقط ولا يمس إعدادات المنتج.
pub async fn resolve_unit(
    conn: &mut SqliteConnection,
    product_id: &str,
    unit_id: Option<&str>,
    factor_override: Option<&str>,
    purchase_only: bool,
) -> Result<ResolvedUnit, AppError> {
    let bad = |m: &str| AppError::Validation(m.to_string());
    let base: (String, String, bool) = sqlx::query_as(
        "SELECT p.base_unit_id, u.name_ar, u.allows_decimal FROM products p JOIN units u ON u.id = p.base_unit_id WHERE p.id = ?1",
    )
    .bind(product_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| bad("المنتج غير موجود"))?;
    match unit_id {
        Some(uid) if !uid.is_empty() && uid != base.0 => {
            let q = if purchase_only {
                "SELECT pu.factor_micro, u.name_ar, u.allows_decimal FROM product_units pu JOIN units u ON u.id = pu.unit_id \
                 WHERE pu.product_id = ?1 AND pu.unit_id = ?2 AND pu.is_purchase_unit = 1"
            } else {
                "SELECT pu.factor_micro, u.name_ar, u.allows_decimal FROM product_units pu JOIN units u ON u.id = pu.unit_id \
                 WHERE pu.product_id = ?1 AND pu.unit_id = ?2"
            };
            let row: Option<(i64, String, bool)> =
                sqlx::query_as(q).bind(product_id).bind(uid).fetch_optional(&mut *conn).await?;
            let Some((default_factor, name, dec)) = row else { return Err(bad("الوحدة غير مسموحة لهذا المنتج")) };
            let factor_micro = match factor_override.map(str::trim) {
                Some(f) if !f.is_empty() => {
                    let v = crate::app_core::quantity::parse_factor(f)?;
                    if v < 2 * FACTOR_SCALE || v % FACTOR_SCALE != 0 {
                        return Err(bad("عدد القطع في الوحدة الكبيرة يجب أن يكون رقمًا صحيحًا 2 أو أكثر"));
                    }
                    v
                }
                _ => default_factor,
            };
            Ok(ResolvedUnit { unit_id: uid.to_string(), unit_name: name, factor_micro, allows_decimal: dec, is_base: false })
        }
        _ => {
            if purchase_only {
                let buy_base: bool = sqlx::query_scalar("SELECT buy_in_base FROM products WHERE id = ?1")
                    .bind(product_id)
                    .fetch_one(&mut *conn)
                    .await?;
                if !buy_base {
                    return Err(bad("هذا المنتج لا يُشترى بالقطعة، اختر الوحدة الكبيرة"));
                }
            }
            Ok(ResolvedUnit { unit_id: base.0, unit_name: base.1, factor_micro: FACTOR_SCALE, allows_decimal: base.2, is_base: true })
        }
    }
}

/// تحديث سعر بيع وحدة (يُستدعى عند استلام الشراء أو إدخال رصيد): القطعة ← سعر المنتج/المتغير،
/// الوحدة الكبيرة ← `product_units.sale_price_minor`. بند متغير بوحدة كبيرة لا يحدّث شيئًا.
/// يرجع true لو تم التحديث فعلًا.
pub async fn apply_unit_sale_price(
    conn: &mut SqliteConnection,
    product_id: &str,
    variant_id: Option<&str>,
    unit_id: &str,
    price_minor: i64,
) -> Result<bool, AppError> {
    let base_unit: String = sqlx::query_scalar("SELECT base_unit_id FROM products WHERE id = ?1")
        .bind(product_id)
        .fetch_one(&mut *conn)
        .await?;
    if unit_id == base_unit {
        match variant_id {
            Some(v) => {
                sqlx::query("UPDATE product_variants SET sale_price = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?1")
                    .bind(v)
                    .bind(price_minor)
                    .execute(&mut *conn)
                    .await?;
            }
            None => {
                sqlx::query("UPDATE products SET sale_price = ?2, price_source = 'manual', updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?1")
                    .bind(product_id)
                    .bind(price_minor)
                    .execute(&mut *conn)
                    .await?;
            }
        }
        Ok(true)
    } else if variant_id.is_none() {
        let r = sqlx::query("UPDATE product_units SET sale_price_minor = ?3 WHERE product_id = ?1 AND unit_id = ?2")
            .bind(product_id)
            .bind(unit_id)
            .bind(price_minor)
            .execute(&mut *conn)
            .await?;
        // سعر القطعة المحسوب يتبع سعر الوحدة الكبيرة (مقرّبًا لأعلى)؛ اليدوي لا يُمس.
        let factor: Option<i64> = sqlx::query_scalar("SELECT factor_micro FROM product_units WHERE product_id = ?1 AND unit_id = ?2")
            .bind(product_id)
            .bind(unit_id)
            .fetch_optional(&mut *conn)
            .await?;
        if let Some(f) = factor {
            let piece = crate::modules::products::validate::piece_price_from_big(price_minor, f);
            sqlx::query("UPDATE products SET sale_price = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?1 AND price_source = 'derived'")
                .bind(product_id)
                .bind(piece)
                .execute(&mut *conn)
                .await?;
        }
        Ok(r.rows_affected() > 0)
    } else {
        Ok(false)
    }
}

/// آخر سعر شراء للوحدة الكبيرة (يُملأ تلقائيًا في فاتورة الشراء القادمة).
pub async fn record_unit_purchase_price(
    conn: &mut SqliteConnection,
    product_id: &str,
    unit_id: &str,
    line_net_minor: i64,
    base_qty_milli: i64,
) -> Result<(), AppError> {
    let factor: Option<i64> = sqlx::query_scalar("SELECT factor_micro FROM product_units WHERE product_id = ?1 AND unit_id = ?2")
        .bind(product_id)
        .bind(unit_id)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(f) = factor else { return Ok(()) };
    // سعر الوحدة = قيمة السطر ÷ عدد الوحدات الكبيرة المشتراة
    let units_qty_milli = i128::from(base_qty_milli) * 1_000_000 / i128::from(f).max(1);
    if units_qty_milli <= 0 {
        return Ok(());
    }
    let price = div_round(i128::from(line_net_minor) * 1000, units_qty_milli) as i64;
    sqlx::query("UPDATE product_units SET purchase_price_minor = ?3 WHERE product_id = ?1 AND unit_id = ?2")
        .bind(product_id)
        .bind(unit_id)
        .bind(price)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn adjust(pool: &SqlitePool, input: &AdjustInput) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let result = adjust_in_tx(&mut tx, input).await;
    match result {
        Ok(()) => {
            tx.commit().await?;
            Ok(())
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

async fn adjust_in_tx(conn: &mut SqliteConnection, input: &AdjustInput) -> Result<(), AppError> {
    let bad = |m: &str| AppError::Validation(m.to_string());
    let row: Option<(String, bool, i64)> = sqlx::query_as(
        "SELECT p.name_ar, u.allows_decimal, \
                COALESCE((SELECT purchase_price FROM product_variants WHERE id = ?2 AND product_id = p.id), p.purchase_price) \
         FROM products p JOIN units u ON u.id = p.base_unit_id WHERE p.id = ?1",
    )
    .bind(&input.product_id)
    .bind(&input.variant_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((name, allows_decimal, default_cost)) = row else { return Err(bad("المنتج غير موجود")) };
    if let Some(v) = &input.variant_id {
        let ok: Option<i64> = sqlx::query_scalar("SELECT 1 FROM product_variants WHERE id = ?1 AND product_id = ?2")
            .bind(v)
            .bind(&input.product_id)
            .fetch_optional(&mut *conn)
            .await?;
        if ok.is_none() {
            return Err(bad("المتغير لا يخص هذا المنتج"));
        }
    }
    let _ = allows_decimal;
    let unit = resolve_unit(conn, &input.product_id, input.unit_id.as_deref(), input.factor.as_deref(), false).await?;
    let entered = parse_qty(&input.qty, unit.allows_decimal)?;
    if entered == 0 {
        return Err(bad("الكمية يجب أن تكون أكبر من صفر"));
    }
    let qty = to_base_qty(entered, unit.factor_micro)?;
    if qty <= 0 {
        return Err(bad("الكمية صغيرة جدًا بعد تحويل الوحدة"));
    }
    let wh = default_warehouse(conn).await?;
    let variant = input.variant_id.as_deref();
    let r = MovementRef { movement_type: "adjustment", ref_type: "adjustment", ref_id: "", note: input.note.trim() };
    match input.direction.as_str() {
        "increase" => {
            // القيمة الكاملة من إجمالي السطر (تكلفة الوحدة المدخلة × الكمية المدخلة) بدون تقريب لكل قطعة.
            let value = match &input.unit_cost {
                Some(c) if !c.trim().is_empty() => div_round(i128::from(parse_minor(c)?) * i128::from(entered), 1000) as i64,
                _ => div_round(i128::from(default_cost) * i128::from(qty), 1000) as i64,
            };
            let new_price = match input.sale_price.as_deref().map(str::trim) {
                Some(p) if !p.is_empty() => Some(parse_minor(p)?),
                _ => None,
            };
            receive(conn, &wh, &input.product_id, variant, qty, value, &r).await?;
            if let Some(p) = new_price {
                apply_unit_sale_price(conn, &input.product_id, variant, &unit.unit_id, p).await?;
            }
        }
        "decrease" => {
            let allow = allow_negative(conn).await?;
            issue(conn, &wh, &input.product_id, variant, qty, default_cost, allow, &r, &name).await?;
        }
        _ => return Err(bad("نوع التسوية غير صالح")),
    }
    tracing::info!("تسوية مخزون للمنتج {name}: {} {}", input.direction, input.qty);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_support::memory_pool;
    use crate::modules::test_helpers::product;

    async fn bal(pool: &SqlitePool, id: &str) -> Balance {
        let mut c = pool.acquire().await.unwrap();
        let wh = default_warehouse(&mut c).await.unwrap();
        balance(&mut c, &wh, id, None).await.unwrap()
    }

    fn adj(id: &str, dir: &str, qty: &str, cost: Option<&str>) -> AdjustInput {
        AdjustInput {
            product_id: id.into(), variant_id: None, direction: dir.into(), qty: qty.into(),
            unit_cost: cost.map(Into::into), note: String::new(), ..Default::default()
        }
    }

    #[tokio::test]
    async fn weighted_average_cost_after_two_receipts() {
        let pool = memory_pool().await;
        let id = product(&pool, "سكر", "10", "15").await;
        adjust(&pool, &adj(&id, "increase", "10", Some("10"))).await.unwrap(); // 10 × 10 = 100
        adjust(&pool, &adj(&id, "increase", "10", Some("20"))).await.unwrap(); // 10 × 20 = 200
        let b = bal(&pool, &id).await;
        assert_eq!((b.qty, b.value), (20_000, 30_000)); // متوسط 15 جنيه
        adjust(&pool, &adj(&id, "decrease", "5", None)).await.unwrap(); // 5 × 15
        let b = bal(&pool, &id).await;
        assert_eq!((b.qty, b.value), (15_000, 22_500));
    }

    #[tokio::test]
    async fn decrease_below_zero_is_rejected_by_default_and_changes_nothing() {
        let pool = memory_pool().await;
        let id = product(&pool, "سكر", "10", "15").await;
        adjust(&pool, &adj(&id, "increase", "2", None)).await.unwrap();
        assert!(adjust(&pool, &adj(&id, "decrease", "3", None)).await.is_err());
        assert_eq!(bal(&pool, &id).await.qty, 2000);
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM stock_movements").fetch_one(&pool).await.unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn negative_stock_allowed_when_enabled() {
        let pool = memory_pool().await;
        crate::modules::settings::set(&pool, "allow_negative_stock", "1").await.unwrap();
        let id = product(&pool, "سكر", "10", "15").await;
        adjust(&pool, &adj(&id, "decrease", "2", None)).await.unwrap();
        assert_eq!(bal(&pool, &id).await.qty, -2000);
    }

    #[tokio::test]
    async fn invalid_adjustments_are_rejected() {
        let pool = memory_pool().await;
        let id = product(&pool, "سكر", "10", "15").await;
        assert!(adjust(&pool, &adj(&id, "increase", "0", None)).await.is_err());
        assert!(adjust(&pool, &adj(&id, "increase", "1.5", None)).await.is_err()); // القطعة لا تقبل كسورًا
        assert!(adjust(&pool, &adj(&id, "sideways", "1", None)).await.is_err());
        assert!(adjust(&pool, &adj("nope", "increase", "1", None)).await.is_err());
    }
}
