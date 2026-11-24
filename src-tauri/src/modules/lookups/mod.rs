//! التصنيفات والماركات: جدولان متطابقان في البنية، يشتركان في نفس المنطق.
//! اسم الجدول يأتي من `Kind` الثابت وليس من مدخلات المستخدم.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::text::normalize;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Category,
    Brand,
}

impl Kind {
    fn table(self) -> &'static str {
        match self {
            Kind::Category => "categories",
            Kind::Brand => "brands",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Kind::Category => "التصنيف",
            Kind::Brand => "الماركة",
        }
    }
    fn product_column(self) -> &'static str {
        match self {
            Kind::Category => "category_id",
            Kind::Brand => "brand_id",
        }
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LookupItem {
    pub id: String,
    pub name_ar: String,
    pub name_en: String,
    pub is_active: bool,
    pub product_count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupInput {
    pub id: Option<String>,
    pub name_ar: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

pub async fn list(pool: &SqlitePool, kind: Kind) -> Result<Vec<LookupItem>, AppError> {
    let sql = format!(
        "SELECT t.id, t.name_ar, t.name_en, t.is_active, \
                (SELECT COUNT(*) FROM products p WHERE p.{col} = t.id AND p.archived_at IS NULL) AS product_count \
         FROM {table} t ORDER BY t.is_active DESC, t.name_ar",
        col = kind.product_column(),
        table = kind.table()
    );
    Ok(sqlx::query_as::<_, LookupItem>(&sql).fetch_all(pool).await?)
}

pub async fn save(pool: &SqlitePool, kind: Kind, input: LookupInput) -> Result<String, AppError> {
    let name_ar = input.name_ar.trim().to_string();
    let name_en = input.name_en.trim().to_string();
    if name_ar.is_empty() {
        return Err(AppError::Validation(format!("اسم {} مطلوب", kind.label())));
    }
    if name_ar.chars().count() > 100 || name_en.chars().count() > 100 {
        return Err(AppError::Validation("الاسم أطول من الحد المسموح".into()));
    }
    let key = normalize(&name_ar);

    let dup: Option<String> = sqlx::query_scalar(&format!(
        "SELECT id FROM {} WHERE name_key = ?1 AND (?2 IS NULL OR id <> ?2)",
        kind.table()
    ))
    .bind(&key)
    .bind(input.id.as_deref())
    .fetch_optional(pool)
    .await?;
    if dup.is_some() {
        return Err(AppError::Validation(format!("يوجد {} بنفس الاسم بالفعل", kind.label())));
    }

    match input.id {
        Some(id) => {
            let r = sqlx::query(&format!(
                "UPDATE {} SET name_ar = ?1, name_en = ?2, name_key = ?3, is_active = ?4, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?5",
                kind.table()
            ))
            .bind(&name_ar)
            .bind(&name_en)
            .bind(&key)
            .bind(input.is_active)
            .bind(&id)
            .execute(pool)
            .await?;
            if r.rows_affected() == 0 {
                return Err(AppError::Validation(format!("{} غير موجود", kind.label())));
            }
            Ok(id)
        }
        None => {
            let id = Uuid::new_v4().to_string();
            sqlx::query(&format!(
                "INSERT INTO {} (id, name_ar, name_en, name_key, is_active) VALUES (?1, ?2, ?3, ?4, ?5)",
                kind.table()
            ))
            .bind(&id)
            .bind(&name_ar)
            .bind(&name_en)
            .bind(&key)
            .bind(input.is_active)
            .execute(pool)
            .await?;
            tracing::info!("تم إنشاء {}: {}", kind.label(), name_ar);
            Ok(id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_support::memory_pool;

    fn input(name: &str) -> LookupInput {
        LookupInput { id: None, name_ar: name.into(), name_en: String::new(), is_active: true }
    }

    #[tokio::test]
    async fn creates_and_lists() {
        let pool = memory_pool().await;
        save(&pool, Kind::Category, input("ألبان")).await.unwrap();
        let items = list(&pool, Kind::Category).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name_ar, "ألبان");
    }

    #[tokio::test]
    async fn duplicate_names_are_rejected_after_normalization() {
        let pool = memory_pool().await;
        save(&pool, Kind::Brand, input("أريال")).await.unwrap();
        assert!(save(&pool, Kind::Brand, input("اريال")).await.is_err());
        // نفس الاسم مسموح في جدول مختلف
        save(&pool, Kind::Category, input("أريال")).await.unwrap();
    }

    #[tokio::test]
    async fn rename_keeps_uniqueness_and_can_deactivate() {
        let pool = memory_pool().await;
        let id = save(&pool, Kind::Category, input("عطور")).await.unwrap();
        save(&pool, Kind::Category, input("منظفات")).await.unwrap();
        let mut edit = input("منظفات");
        edit.id = Some(id.clone());
        assert!(save(&pool, Kind::Category, edit).await.is_err());
        let mut off = input("عطور");
        off.id = Some(id);
        off.is_active = false;
        save(&pool, Kind::Category, off).await.unwrap();
        let items = list(&pool, Kind::Category).await.unwrap();
        assert!(items.iter().any(|i| i.name_ar == "عطور" && !i.is_active));
    }

    #[tokio::test]
    async fn empty_name_is_rejected() {
        let pool = memory_pool().await;
        assert!(save(&pool, Kind::Category, input("   ")).await.is_err());
    }
}
