use sqlx::SqlitePool;
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::text::normalize;

use super::model::{Supplier, SupplierInput};

pub async fn list(pool: &SqlitePool) -> Result<Vec<Supplier>, AppError> {
    Ok(sqlx::query_as::<_, Supplier>(
        "SELECT s.id, s.name_ar, s.phone, s.address, s.notes, s.is_active, \
                (COALESCE((SELECT SUM(total_minor) FROM purchases WHERE supplier_id = s.id AND status = 'received'), 0) \
                 - COALESCE((SELECT SUM(total_minor) FROM purchase_returns WHERE supplier_id = s.id), 0) \
                 - COALESCE((SELECT SUM(amount_minor) FROM purchase_payments WHERE supplier_id = s.id), 0)) AS balance_minor \
         FROM suppliers s ORDER BY s.is_active DESC, s.name_ar",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn save(pool: &SqlitePool, input: &SupplierInput) -> Result<String, AppError> {
    let bad = |m: &str| AppError::Validation(m.to_string());
    let name = input.name_ar.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(bad("اسم المورد مطلوب ولا يزيد عن 120 حرفًا"));
    }
    if input.phone.chars().count() > 30 || input.address.chars().count() > 250 || input.notes.chars().count() > 500 {
        return Err(bad("أحد الحقول أطول من الحد المسموح"));
    }
    let key = normalize(name);
    let dup: Option<String> = sqlx::query_scalar("SELECT id FROM suppliers WHERE name_key = ?1 AND (?2 IS NULL OR id <> ?2)")
        .bind(&key)
        .bind(&input.id)
        .fetch_optional(pool)
        .await?;
    if dup.is_some() {
        return Err(bad("يوجد مورد بنفس الاسم"));
    }
    match &input.id {
        Some(id) => {
            let r = sqlx::query(
                "UPDATE suppliers SET name_ar=?2, name_key=?3, phone=?4, address=?5, notes=?6, is_active=?7, \
                 updated_at=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id=?1",
            )
            .bind(id)
            .bind(name)
            .bind(&key)
            .bind(input.phone.trim())
            .bind(input.address.trim())
            .bind(input.notes.trim())
            .bind(input.is_active)
            .execute(pool)
            .await?;
            if r.rows_affected() == 0 {
                return Err(bad("المورد غير موجود"));
            }
            Ok(id.clone())
        }
        None => {
            let id = Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO suppliers (id, name_ar, name_key, phone, address, notes, is_active) VALUES (?1,?2,?3,?4,?5,?6,?7)")
                .bind(&id)
                .bind(name)
                .bind(&key)
                .bind(input.phone.trim())
                .bind(input.address.trim())
                .bind(input.notes.trim())
                .bind(input.is_active)
                .execute(pool)
                .await?;
            Ok(id)
        }
    }
}
