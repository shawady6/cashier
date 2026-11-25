//! مرتجعات الشراء: تخرج الكمية من مخزن الفاتورة بمتوسط التكلفة، ويُخصم قيمتها (بالتناسب) من حساب المورد.
//! آخر مرتجع لبند يأخذ الباقي بالضبط فلا يحدث فرق تقريب.

use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::money::div_round;
use crate::app_core::quantity::parse_qty;
use crate::modules::inventory::{allow_negative, issue, MovementRef};
use crate::modules::sales::checkout::next_number;

use super::model::PurchaseReturnInput;

fn bad<T>(m: impl Into<String>) -> Result<T, AppError> {
    Err(AppError::Validation(m.into()))
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    product_id: String,
    variant_id: Option<String>,
    name_ar: String,
    qty_milli: i64,
    base_qty_milli: i64,
    net_minor: i64,
    allows_decimal: bool,
    ret_qty: i64,
    ret_base: i64,
    ret_credit: i64,
}

fn share(total: i64, part: i64, whole: i64) -> i64 {
    div_round(i128::from(total) * i128::from(part), i128::from(whole)) as i64
}

pub async fn create(pool: &SqlitePool, input: &PurchaseReturnInput) -> Result<(String, i64), AppError> {
    let mut tx = pool.begin().await?;
    match create_in_tx(&mut tx, input).await {
        Ok(r) => {
            tx.commit().await?;
            Ok(r)
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

async fn create_in_tx(conn: &mut SqliteConnection, input: &PurchaseReturnInput) -> Result<(String, i64), AppError> {
    if input.items.is_empty() {
        return bad("اختر بندًا واحدًا على الأقل");
    }
    let head: Option<(String, String, String)> =
        sqlx::query_as("SELECT status, supplier_id, warehouse_id FROM purchases WHERE id = ?1")
            .bind(&input.purchase_id)
            .fetch_optional(&mut *conn)
            .await?;
    let Some((status, supplier_id, wh)) = head else { return bad("فاتورة الشراء غير موجودة") };
    if status != "received" {
        return bad("المرتجع يتم على الفواتير المستلمة فقط");
    }
    let allow = allow_negative(conn).await?;
    let id = Uuid::new_v4().to_string();
    let mut seen = std::collections::HashSet::new();
    let mut plan = Vec::new();
    let mut total = 0i64;

    for it in &input.items {
        if !seen.insert(it.purchase_item_id.clone()) {
            return bad("بند مكرر في المرتجع");
        }
        let r = sqlx::query_as::<_, Row>(
            "SELECT pi.id, pi.product_id, pi.variant_id, pi.name_ar, pi.qty_milli, pi.base_qty_milli, pi.net_minor, u.allows_decimal, \
                    COALESCE((SELECT SUM(qty_milli) FROM purchase_return_items WHERE purchase_item_id = pi.id), 0) AS ret_qty, \
                    COALESCE((SELECT SUM(base_qty_milli) FROM purchase_return_items WHERE purchase_item_id = pi.id), 0) AS ret_base, \
                    COALESCE((SELECT SUM(credit_minor) FROM purchase_return_items WHERE purchase_item_id = pi.id), 0) AS ret_credit \
             FROM purchase_items pi JOIN units u ON u.id = pi.unit_id WHERE pi.id = ?1 AND pi.purchase_id = ?2",
        )
        .bind(&it.purchase_item_id)
        .bind(&input.purchase_id)
        .fetch_optional(&mut *conn)
        .await?;
        let Some(r) = r else { return bad("بند لا يخص هذه الفاتورة") };
        let qty = parse_qty(&it.qty, r.allows_decimal)?;
        let remaining = r.qty_milli - r.ret_qty;
        if qty <= 0 {
            return bad("كمية المرتجع يجب أن تكون أكبر من صفر");
        }
        if qty > remaining {
            return bad(format!("كمية مرتجع «{}» أكبر من المتاح للإرجاع", r.name_ar));
        }
        let last = qty == remaining;
        let base = if last { r.base_qty_milli - r.ret_base } else { share(r.base_qty_milli, qty, r.qty_milli).max(1) };
        let credit = if last { r.net_minor - r.ret_credit } else { share(r.net_minor, qty, r.qty_milli) };
        total += credit;
        plan.push((r, qty, base, credit));
    }

    let no = next_number(conn, "purchase_return", "PRT-").await?;
    sqlx::query("INSERT INTO purchase_returns (id, return_no, purchase_id, supplier_id, warehouse_id, total_minor, note) VALUES (?1,?2,?3,?4,?5,?6,?7)")
        .bind(&id)
        .bind(&no)
        .bind(&input.purchase_id)
        .bind(&supplier_id)
        .bind(&wh)
        .bind(total)
        .bind(input.note.trim())
        .execute(&mut *conn)
        .await?;
    for (r, qty, base, credit) in plan {
        sqlx::query("INSERT INTO purchase_return_items (id, return_id, purchase_item_id, qty_milli, base_qty_milli, credit_minor) VALUES (?1,?2,?3,?4,?5,?6)")
            .bind(Uuid::new_v4().to_string())
            .bind(&id)
            .bind(&r.id)
            .bind(qty)
            .bind(base)
            .bind(credit)
            .execute(&mut *conn)
            .await?;
        let fallback = div_round(i128::from(r.net_minor) * 1000, i128::from(r.base_qty_milli)) as i64;
        let m = MovementRef { movement_type: "purchase_return", ref_type: "purchase_return", ref_id: &id, note: "" };
        issue(conn, &wh, &r.product_id, r.variant_id.as_deref(), base, fallback, allow, &m, &r.name_ar).await?;
    }
    tracing::info!("تم تسجيل مرتجع الشراء {no}");
    Ok((no, total))
}
