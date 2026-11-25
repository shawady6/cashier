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
use crate::app_core::quantity::parse_qty;

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjustInput {
    pub product_id: String,
    pub variant_id: Option<String>,
    /// "increase" أو "decrease"
    pub direction: String,
    /// بالوحدة الأساسية للمنتج
    pub qty: String,
    /// تكلفة الوحدة الأساسية (للزيادة فقط). الافتراضي سعر الشراء الحالي.
    pub unit_cost: Option<String>,
    #[serde(default)]
    pub note: String,
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
    let qty = parse_qty(&input.qty, allows_decimal)?;
    if qty == 0 {
        return Err(bad("الكمية يجب أن تكون أكبر من صفر"));
    }
    let wh = default_warehouse(conn).await?;
    let variant = input.variant_id.as_deref();
    let r = MovementRef { movement_type: "adjustment", ref_type: "adjustment", ref_id: "", note: input.note.trim() };
    match input.direction.as_str() {
        "increase" => {
            let unit_cost = match &input.unit_cost {
                Some(c) if !c.trim().is_empty() => parse_minor(c)?,
                _ => default_cost,
            };
            let value = div_round(i128::from(unit_cost) * i128::from(qty), 1000) as i64;
            receive(conn, &wh, &input.product_id, variant, qty, value, &r).await?;
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
            unit_cost: cost.map(Into::into), note: String::new(),
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
