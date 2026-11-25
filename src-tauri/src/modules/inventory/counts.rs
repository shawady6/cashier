//! الجرد: لقطة للرصيد وقت البدء، إدخال الكميات الفعلية، ثم اعتماد يُنتج حركات «count».
//! الفرق (الفعلي - اللقطة) يُطبَّق على الرصيد الحالي، فالمبيعات أثناء الجرد لا تضيع.

use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::money::div_round;
use crate::app_core::quantity::parse_qty;
use crate::modules::sales::checkout::next_number;

use super::{balance, issue, receive, warehouses::require_active, MovementRef};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountInput {
    pub warehouse_id: String,
    pub category_id: Option<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CountRow {
    pub id: String,
    pub count_no: String,
    pub warehouse_name: String,
    pub status: String,
    pub note: String,
    pub created_at: String,
    pub finalized_at: Option<String>,
    pub item_count: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct CountItem {
    pub id: String,
    pub name_ar: String,
    pub variant_name: Option<String>,
    pub base_unit_name: String,
    pub system_qty_milli: i64,
    pub counted_qty_milli: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CountDetail {
    pub header: CountRow,
    pub items: Vec<CountItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountLineInput {
    pub item_id: String,
    /// فارغ = لم يُجرَد بعد (يُتجاهل عند الاعتماد)
    pub counted_qty: Option<String>,
}

pub async fn create(pool: &SqlitePool, input: &CountInput) -> Result<String, AppError> {
    let mut tx = pool.begin().await?;
    let r = create_in_tx(&mut tx, input).await;
    match r {
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

async fn create_in_tx(conn: &mut SqliteConnection, input: &CountInput) -> Result<String, AppError> {
    require_active(conn, &input.warehouse_id).await?;
    let id = Uuid::new_v4().to_string();
    let no = next_number(conn, "count", "CNT-").await?;
    sqlx::query("INSERT INTO stock_counts (id, count_no, warehouse_id, note) VALUES (?1,?2,?3,?4)")
        .bind(&id)
        .bind(&no)
        .bind(&input.warehouse_id)
        .bind(input.note.trim())
        .execute(&mut *conn)
        .await?;

    // كل المنتجات النشطة غير المؤرشفة (المتغيرات النشطة تُجرد كل متغير على حدة)
    let rows: Vec<(String, Option<String>, String, Option<String>, i64)> = sqlx::query_as(
        "SELECT p.id, NULL, p.name_ar, NULL, \
                COALESCE((SELECT qty_milli FROM stock_balances sb WHERE sb.warehouse_id = ?1 AND sb.product_id = p.id AND sb.variant_id IS NULL), 0) \
         FROM products p WHERE p.archived_at IS NULL AND p.is_active = 1 \
           AND (?2 IS NULL OR p.category_id = ?2) \
           AND NOT EXISTS (SELECT 1 FROM product_variants v WHERE v.product_id = p.id AND v.is_active = 1) \
         UNION ALL \
         SELECT p.id, v.id, p.name_ar, v.name_ar, \
                COALESCE((SELECT qty_milli FROM stock_balances sb WHERE sb.warehouse_id = ?1 AND sb.variant_id = v.id), 0) \
         FROM product_variants v JOIN products p ON p.id = v.product_id \
         WHERE p.archived_at IS NULL AND p.is_active = 1 AND v.is_active = 1 AND (?2 IS NULL OR p.category_id = ?2) \
         ORDER BY 3, 4",
    )
    .bind(&input.warehouse_id)
    .bind(&input.category_id)
    .fetch_all(&mut *conn)
    .await?;
    if rows.is_empty() {
        return Err(AppError::Validation("لا توجد منتجات للجرد".into()));
    }
    for (pid, vid, name, vname, qty) in rows {
        sqlx::query(
            "INSERT INTO stock_count_items (id, count_id, product_id, variant_id, name_ar, variant_name, system_qty_milli) \
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&id)
        .bind(pid)
        .bind(vid)
        .bind(name)
        .bind(vname)
        .bind(qty)
        .execute(&mut *conn)
        .await?;
    }
    Ok(id)
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<CountRow>, AppError> {
    Ok(sqlx::query_as::<_, CountRow>(
        "SELECT c.id, c.count_no, w.name_ar AS warehouse_name, c.status, c.note, c.created_at, c.finalized_at, \
                (SELECT COUNT(*) FROM stock_count_items WHERE count_id = c.id) AS item_count \
         FROM stock_counts c JOIN warehouses w ON w.id = c.warehouse_id ORDER BY c.created_at DESC, c.count_no DESC LIMIT 200",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<CountDetail, AppError> {
    let header = sqlx::query_as::<_, CountRow>(
        "SELECT c.id, c.count_no, w.name_ar AS warehouse_name, c.status, c.note, c.created_at, c.finalized_at, \
                (SELECT COUNT(*) FROM stock_count_items WHERE count_id = c.id) AS item_count \
         FROM stock_counts c JOIN warehouses w ON w.id = c.warehouse_id WHERE c.id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Validation("الجرد غير موجود".into()))?;
    let items = sqlx::query_as::<_, CountItem>(
        "SELECT i.id, i.name_ar, i.variant_name, u.name_ar AS base_unit_name, i.system_qty_milli, i.counted_qty_milli \
         FROM stock_count_items i JOIN products p ON p.id = i.product_id JOIN units u ON u.id = p.base_unit_id \
         WHERE i.count_id = ?1 ORDER BY i.name_ar, i.variant_name",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    Ok(CountDetail { header, items })
}

async fn require_open(conn: &mut SqliteConnection, id: &str) -> Result<String, AppError> {
    let row: Option<(String, String)> = sqlx::query_as("SELECT status, warehouse_id FROM stock_counts WHERE id = ?1")
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
    match row {
        None => Err(AppError::Validation("الجرد غير موجود".into())),
        Some((s, _)) if s != "open" => Err(AppError::Validation("هذا الجرد مغلق ولا يمكن تعديله".into())),
        Some((_, wh)) => Ok(wh),
    }
}

pub async fn save_lines(pool: &SqlitePool, count_id: &str, lines: &[CountLineInput]) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    require_open(&mut tx, count_id).await?;
    for l in lines {
        let item: Option<(bool,)> = sqlx::query_as(
            "SELECT u.allows_decimal FROM stock_count_items i JOIN products p ON p.id = i.product_id \
             JOIN units u ON u.id = p.base_unit_id WHERE i.id = ?1 AND i.count_id = ?2",
        )
        .bind(&l.item_id)
        .bind(count_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((allows_decimal,)) = item else {
            return Err(AppError::Validation("بند لا يخص هذا الجرد".into()));
        };
        let counted = match l.counted_qty.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(q) => Some(parse_qty(q, allows_decimal)?),
        };
        sqlx::query("UPDATE stock_count_items SET counted_qty_milli = ?2 WHERE id = ?1")
            .bind(&l.item_id)
            .bind(counted)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn cancel(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    require_open(&mut tx, id).await?;
    sqlx::query("UPDATE stock_counts SET status = 'cancelled' WHERE id = ?1").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// يرجع عدد البنود التي سُوِّيت (الفرق ≠ 0).
pub async fn finalize(pool: &SqlitePool, id: &str) -> Result<i64, AppError> {
    let mut tx = pool.begin().await?;
    let r = finalize_in_tx(&mut tx, id).await;
    match r {
        Ok(n) => {
            tx.commit().await?;
            tracing::info!("تم اعتماد الجرد {id} وتسوية {n} بند");
            Ok(n)
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

async fn finalize_in_tx(conn: &mut SqliteConnection, id: &str) -> Result<i64, AppError> {
    let wh = require_open(conn, id).await?;
    let items: Vec<(String, Option<String>, String, i64, Option<i64>, i64)> = sqlx::query_as(
        "SELECT i.product_id, i.variant_id, i.name_ar, i.system_qty_milli, i.counted_qty_milli, \
                COALESCE(v.purchase_price, p.purchase_price) \
         FROM stock_count_items i JOIN products p ON p.id = i.product_id \
         LEFT JOIN product_variants v ON v.id = i.variant_id WHERE i.count_id = ?1",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;

    let mut adjusted = 0;
    for (pid, vid, name, system, counted, fallback) in items {
        let Some(counted) = counted else { continue };
        let delta = counted - system;
        if delta == 0 {
            continue;
        }
        let r = MovementRef { movement_type: "count", ref_type: "count", ref_id: id, note: "" };
        if delta > 0 {
            let b = balance(conn, &wh, &pid, vid.as_deref()).await?;
            let value = if b.qty > 0 {
                div_round(i128::from(b.value) * i128::from(delta), i128::from(b.qty)) as i64
            } else {
                div_round(i128::from(fallback) * i128::from(delta), 1000) as i64
            };
            receive(conn, &wh, &pid, vid.as_deref(), delta, value, &r).await?;
        } else {
            // الجرد حقيقة فعلية: يُسمح بالسالب هنا حتى لا يفشل الاعتماد بسبب مبيعات أثناء الجرد
            issue(conn, &wh, &pid, vid.as_deref(), -delta, fallback, true, &r, &name).await?;
        }
        adjusted += 1;
    }
    sqlx::query("UPDATE stock_counts SET status = 'finalized', finalized_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(adjusted)
}
