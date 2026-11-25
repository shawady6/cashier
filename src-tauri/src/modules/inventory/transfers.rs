//! نقل مخزون بين مخزنين: خروج من المصدر بمتوسط تكلفته ودخول للهدف بنفس القيمة (فوري).

use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::quantity::parse_qty;
use crate::modules::sales::checkout::next_number;

use super::{allow_negative, issue, product_info, receive, warehouses::require_active, MovementRef};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferItemInput {
    pub product_id: String,
    pub variant_id: Option<String>,
    /// بالوحدة الأساسية
    pub qty: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferInput {
    pub from_warehouse_id: String,
    pub to_warehouse_id: String,
    #[serde(default)]
    pub note: String,
    pub items: Vec<TransferItemInput>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TransferRow {
    pub id: String,
    pub transfer_no: String,
    pub from_name: String,
    pub to_name: String,
    pub note: String,
    pub created_at: String,
    pub item_count: i64,
    pub value_minor: i64,
}

pub async fn create(pool: &SqlitePool, input: &TransferInput) -> Result<String, AppError> {
    let mut tx = pool.begin().await?;
    match create_in_tx(&mut tx, input).await {
        Ok(id) => {
            tx.commit().await?;
            Ok(id)
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

async fn create_in_tx(conn: &mut SqliteConnection, input: &TransferInput) -> Result<String, AppError> {
    let bad = |m: &str| AppError::Validation(m.to_string());
    if input.items.is_empty() {
        return Err(bad("أضف بندًا واحدًا على الأقل"));
    }
    if input.from_warehouse_id == input.to_warehouse_id {
        return Err(bad("المخزن المصدر والهدف يجب أن يختلفا"));
    }
    require_active(conn, &input.from_warehouse_id).await?;
    require_active(conn, &input.to_warehouse_id).await?;
    let allow = allow_negative(conn).await?;

    let id = Uuid::new_v4().to_string();
    let no = next_number(conn, "transfer", "TRF-").await?;
    sqlx::query("INSERT INTO stock_transfers (id, transfer_no, from_warehouse_id, to_warehouse_id, note) VALUES (?1,?2,?3,?4,?5)")
        .bind(&id)
        .bind(&no)
        .bind(&input.from_warehouse_id)
        .bind(&input.to_warehouse_id)
        .bind(input.note.trim())
        .execute(&mut *conn)
        .await?;

    for it in &input.items {
        let info = product_info(conn, &it.product_id, it.variant_id.as_deref()).await?;
        let qty = parse_qty(&it.qty, info.allows_decimal)?;
        if qty <= 0 {
            return Err(bad("كمية النقل يجب أن تكون أكبر من صفر"));
        }
        let out = MovementRef { movement_type: "transfer_out", ref_type: "transfer", ref_id: &id, note: "" };
        let cost = issue(
            conn, &input.from_warehouse_id, &it.product_id, it.variant_id.as_deref(), qty,
            info.fallback_cost, allow, &out, &info.name,
        )
        .await?;
        let inn = MovementRef { movement_type: "transfer_in", ref_type: "transfer", ref_id: &id, note: "" };
        receive(conn, &input.to_warehouse_id, &it.product_id, it.variant_id.as_deref(), qty, cost, &inn).await?;
        sqlx::query(
            "INSERT INTO stock_transfer_items (id, transfer_id, product_id, variant_id, qty_milli, value_minor) VALUES (?1,?2,?3,?4,?5,?6)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&id)
        .bind(&it.product_id)
        .bind(&it.variant_id)
        .bind(qty)
        .bind(cost)
        .execute(&mut *conn)
        .await?;
    }
    tracing::info!("تم التحويل المخزني {no}");
    Ok(id)
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<TransferRow>, AppError> {
    Ok(sqlx::query_as::<_, TransferRow>(
        "SELECT t.id, t.transfer_no, wf.name_ar AS from_name, wt.name_ar AS to_name, t.note, t.created_at, \
                (SELECT COUNT(*) FROM stock_transfer_items WHERE transfer_id = t.id) AS item_count, \
                COALESCE((SELECT SUM(value_minor) FROM stock_transfer_items WHERE transfer_id = t.id), 0) AS value_minor \
         FROM stock_transfers t JOIN warehouses wf ON wf.id = t.from_warehouse_id JOIN warehouses wt ON wt.id = t.to_warehouse_id \
         ORDER BY t.created_at DESC, t.transfer_no DESC LIMIT 200",
    )
    .fetch_all(pool)
    .await?)
}
