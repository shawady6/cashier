//! إدارة المخازن. لا يوجد حذف: المخزن يُعطَّل فقط. دائمًا مخزن افتراضي واحد نشط (تُباع منه الفواتير).

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::text::normalize;

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Warehouse {
    pub id: String,
    pub name_ar: String,
    pub is_default: bool,
    pub is_active: bool,
    pub stock_value_minor: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarehouseInput {
    pub id: Option<String>,
    pub name_ar: String,
    #[serde(default = "yes")]
    pub is_active: bool,
    #[serde(default)]
    pub is_default: bool,
}

fn yes() -> bool {
    true
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Warehouse>, AppError> {
    Ok(sqlx::query_as::<_, Warehouse>(
        "SELECT w.id, w.name_ar, w.is_default, w.is_active, \
                COALESCE((SELECT SUM(value_minor) FROM stock_balances WHERE warehouse_id = w.id), 0) AS stock_value_minor \
         FROM warehouses w ORDER BY w.is_default DESC, w.is_active DESC, w.name_ar",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn save(pool: &SqlitePool, input: &WarehouseInput) -> Result<String, AppError> {
    let bad = |m: &str| AppError::Validation(m.to_string());
    let name = input.name_ar.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(bad("اسم المخزن مطلوب ولا يزيد عن 80 حرفًا"));
    }
    let key = normalize(name);
    let mut tx = pool.begin().await?;

    let all: Vec<(String, String)> = sqlx::query_as("SELECT id, name_ar FROM warehouses").fetch_all(&mut *tx).await?;
    if all.iter().any(|(id, n)| normalize(n) == key && Some(id) != input.id.as_ref()) {
        return Err(bad("يوجد مخزن بنفس الاسم"));
    }

    let id = match &input.id {
        Some(id) => {
            let cur: Option<(bool,)> = sqlx::query_as("SELECT is_default FROM warehouses WHERE id = ?1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
            let Some((was_default,)) = cur else { return Err(bad("المخزن غير موجود")) };
            if was_default && (!input.is_active || !input.is_default) {
                return Err(bad("المخزن الافتراضي لا يمكن تعطيله. اجعل مخزنًا آخر افتراضيًا أولًا"));
            }
            sqlx::query("UPDATE warehouses SET name_ar = ?2, is_active = ?3 WHERE id = ?1")
                .bind(id)
                .bind(name)
                .bind(input.is_active)
                .execute(&mut *tx)
                .await?;
            id.clone()
        }
        None => {
            let id = Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO warehouses (id, name_ar, is_default, is_active) VALUES (?1, ?2, 0, ?3)")
                .bind(&id)
                .bind(name)
                .bind(input.is_active)
                .execute(&mut *tx)
                .await?;
            id
        }
    };

    if input.is_default {
        if !input.is_active {
            return Err(bad("المخزن الافتراضي يجب أن يكون نشطًا"));
        }
        sqlx::query("UPDATE warehouses SET is_default = CASE WHEN id = ?1 THEN 1 ELSE 0 END").bind(&id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(id)
}

/// يتحقق أن المخزن موجود ونشط.
pub async fn require_active(conn: &mut sqlx::SqliteConnection, id: &str) -> Result<(), AppError> {
    let ok: Option<i64> = sqlx::query_scalar("SELECT 1 FROM warehouses WHERE id = ?1 AND is_active = 1")
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
    ok.map(|_| ()).ok_or_else(|| AppError::Validation("المخزن غير موجود أو غير نشط".into()))
}
