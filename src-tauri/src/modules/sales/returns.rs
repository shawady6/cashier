//! مرتجعات المبيعات: سجلات عكسية جديدة، والفاتورة الأصلية لا تُمَس.
//! المبلغ والتكلفة والكمية الأساسية تُحسب بالتناسب، وآخر مرتجع لبند يأخذ الباقي بالضبط
//! فمجموع المرتجعات لا يزيد ولا ينقص عن قيمة البند الأصلية.

use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::money::div_round;
use crate::app_core::quantity::parse_qty;
use crate::modules::finance;
use crate::modules::inventory::{self, MovementRef};

use super::checkout::next_number;
use super::model::{ReturnInput, ReturnResult};

fn bad<T>(m: impl Into<String>) -> Result<T, AppError> {
    Err(AppError::Validation(m.into()))
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: String,
    product_id: String,
    variant_id: Option<String>,
    name_ar: String,
    qty_milli: i64,
    base_qty_milli: i64,
    total_minor: i64,
    cost_minor: i64,
    allows_decimal: bool,
    ret_qty: i64,
    ret_base: i64,
    ret_refund: i64,
    ret_cost: i64,
}

fn share(total: i64, part: i64, whole: i64) -> i64 {
    div_round(i128::from(total) * i128::from(part), i128::from(whole)) as i64
}

pub async fn create_return(pool: &SqlitePool, input: &ReturnInput) -> Result<ReturnResult, AppError> {
    let mut tx = pool.begin().await?;
    match create_in_tx(&mut tx, input).await {
        Ok(r) => {
            tx.commit().await?;
            tracing::info!("تم تسجيل المرتجع {} بمبلغ {} قرش", r.return_no, r.refund);
            Ok(r)
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

async fn create_in_tx(conn: &mut SqliteConnection, input: &ReturnInput) -> Result<ReturnResult, AppError> {
    if input.items.is_empty() {
        return bad("اختر بندًا واحدًا على الأقل للإرجاع");
    }
    if !["cash", "card"].contains(&input.refund_method.as_str()) {
        return bad("طريقة الاسترداد غير مدعومة");
    }
    let warehouse: Option<String> = sqlx::query_scalar("SELECT warehouse_id FROM sales WHERE id = ?1")
        .bind(&input.sale_id)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(warehouse) = warehouse else { return bad("الفاتورة غير موجودة") };

    let return_id = Uuid::new_v4().to_string();
    let mut planned = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut refund_total = 0i64;

    for it in &input.items {
        if !seen.insert(it.sale_item_id.clone()) {
            return bad("بند مكرر في المرتجع");
        }
        let row = sqlx::query_as::<_, ItemRow>(
            "SELECT si.id, si.product_id, si.variant_id, si.name_ar, si.qty_milli, si.base_qty_milli, si.total_minor, \
                    si.cost_minor, u.allows_decimal, \
                    COALESCE((SELECT SUM(qty_milli) FROM sales_return_items WHERE sale_item_id = si.id), 0) AS ret_qty, \
                    COALESCE((SELECT SUM(base_qty_milli) FROM sales_return_items WHERE sale_item_id = si.id), 0) AS ret_base, \
                    COALESCE((SELECT SUM(refund_minor) FROM sales_return_items WHERE sale_item_id = si.id), 0) AS ret_refund, \
                    COALESCE((SELECT SUM(cost_minor) FROM sales_return_items WHERE sale_item_id = si.id), 0) AS ret_cost \
             FROM sale_items si JOIN units u ON u.id = si.unit_id \
             WHERE si.id = ?1 AND si.sale_id = ?2",
        )
        .bind(&it.sale_item_id)
        .bind(&input.sale_id)
        .fetch_optional(&mut *conn)
        .await?;
        let Some(r) = row else { return bad("بند لا يخص هذه الفاتورة") };

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
        let refund = if last { r.total_minor - r.ret_refund } else { share(r.total_minor, qty, r.qty_milli) };
        let cost = if last { r.cost_minor - r.ret_cost } else { share(r.cost_minor, qty, r.qty_milli) };
        refund_total += refund;
        planned.push((r, qty, base, refund, cost));
    }

    // العميل الآجل: يُخصم المرتجع من مديونيته أولًا، والباقي يُردّ بالطريقة المختارة.
    let customer: Option<String> = sqlx::query_scalar("SELECT customer_id FROM sales WHERE id = ?1")
        .bind(&input.sale_id)
        .fetch_one(&mut *conn)
        .await?;
    let credit = match &customer {
        Some(c) => refund_total.min(finance::customers::balance(conn, c).await?.max(0)),
        None => 0,
    };
    let cash_refund = refund_total - credit;

    let return_no = next_number(conn, "return", "RET-").await?;
    sqlx::query("INSERT INTO sales_returns (id, return_no, sale_id, refund_minor, refund_method, reason, credit_minor) VALUES (?1,?2,?3,?4,?5,?6,?7)")
        .bind(&return_id)
        .bind(&return_no)
        .bind(&input.sale_id)
        .bind(refund_total)
        .bind(&input.refund_method)
        .bind(input.reason.trim())
        .bind(credit)
        .execute(&mut *conn)
        .await?;

    for (r, qty, base, refund, cost) in planned {
        sqlx::query(
            "INSERT INTO sales_return_items (id, return_id, sale_item_id, qty_milli, base_qty_milli, refund_minor, cost_minor) \
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&return_id)
        .bind(&r.id)
        .bind(qty)
        .bind(base)
        .bind(refund)
        .bind(cost)
        .execute(&mut *conn)
        .await?;
        // المخزون يرجع بتكلفة البيع الأصلية (لا بسعر اليوم)
        let movement = MovementRef { movement_type: "sale_return", ref_type: "sales_return", ref_id: &return_id, note: "" };
        inventory::receive(conn, &warehouse, &r.product_id, r.variant_id.as_deref(), base, cost, &movement).await?;
    }

    if cash_refund > 0 {
        finance::record_tx(conn, "sale_return", &input.refund_method, -cash_refund, "sales_return", &return_id).await?;
    }

    Ok(ReturnResult { return_id, return_no, refund: refund_total })
}
