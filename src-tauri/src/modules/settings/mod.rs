//! الإعدادات وأنواع النشاط. كل التحقق هنا في Rust وليس في الواجهة.

use std::collections::HashMap;

use serde::Serialize;
use sqlx::SqlitePool;

use crate::app_core::error::AppError;

pub const ALLOWED_KEYS: &[&str] = &[
    "store_name",
    "store_address",
    "store_phone",
    "currency",
    "language",
    "theme",
    "business_profile_code",
    "allow_negative_stock",
    "setup_completed",
    "tax_rate_bp",
    "receipt_footer",
    "receipt_paper_width",
    "pos_shortcuts",
    "require_cash_session",
];

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BusinessProfile {
    pub id: String,
    pub code: String,
    pub name_ar: String,
    pub name_en: String,
    pub track_expiry: bool,
    pub track_batches: bool,
    pub allow_weighted_products: bool,
    pub allow_variants: bool,
}

pub async fn get_all(pool: &SqlitePool) -> Result<HashMap<String, String>, AppError> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT setting_key, setting_value FROM settings")
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().collect())
}

pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> Result<(), AppError> {
    if !ALLOWED_KEYS.contains(&key) {
        return Err(AppError::Validation(format!("إعداد غير معروف: {key}")));
    }
    validate(pool, key, value).await?;

    sqlx::query(
        "INSERT INTO settings (setting_key, setting_value) VALUES (?1, ?2) \
         ON CONFLICT(setting_key) DO UPDATE SET \
           setting_value = excluded.setting_value, \
           updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    tracing::info!("تم تحديث الإعداد {key}");
    Ok(())
}

pub async fn list_business_profiles(pool: &SqlitePool) -> Result<Vec<BusinessProfile>, AppError> {
    let rows = sqlx::query_as::<_, BusinessProfile>(
        "SELECT id, code, name_ar, name_en, track_expiry, track_batches, \
                allow_weighted_products, allow_variants \
         FROM business_profiles ORDER BY name_ar",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

async fn validate(pool: &SqlitePool, key: &str, value: &str) -> Result<(), AppError> {
    let bad = |msg: &str| AppError::Validation(msg.to_string());
    match key {
        "store_name" | "store_address" | "store_phone" => {
            let max = match key {
                "store_name" => 120,
                "store_address" => 250,
                _ => 30,
            };
            if value.chars().count() > max {
                return Err(bad("القيمة أطول من الحد المسموح"));
            }
        }
        "currency" => {
            if value != "EGP" {
                return Err(bad("العملة المدعومة حاليًا هي EGP فقط"));
            }
        }
        "language" => {
            if !["ar", "en"].contains(&value) {
                return Err(bad("لغة غير مدعومة"));
            }
        }
        "theme" => {
            if !["light", "dark"].contains(&value) {
                return Err(bad("مظهر غير مدعوم"));
            }
        }
        "allow_negative_stock" | "setup_completed" => {
            if !["0", "1"].contains(&value) {
                return Err(bad("القيمة يجب أن تكون 0 أو 1"));
            }
        }
        "tax_rate_bp" => {
            let ok = value.parse::<i64>().map(|v| (0..=10_000).contains(&v)).unwrap_or(false);
            if !ok {
                return Err(bad("نسبة الضريبة يجب أن تكون بين 0 و 10000 (نقطة أساسية)"));
            }
        }
        "receipt_footer" => {
            if value.chars().count() > 200 {
                return Err(bad("نص أسفل الإيصال طويل"));
            }
        }
        "receipt_paper_width" => {
            if !["58", "80"].contains(&value) {
                return Err(bad("عرض الورق 58 أو 80 مم"));
            }
        }
        "pos_shortcuts" => validate_shortcuts(value)?,
        "business_profile_code" => {
            let found: Option<i64> =
                sqlx::query_scalar("SELECT 1 FROM business_profiles WHERE code = ?1")
                    .bind(value)
                    .fetch_optional(pool)
                    .await?;
            if found.is_none() {
                return Err(bad("نوع النشاط غير موجود"));
            }
        }
        _ => {}
    }
    Ok(())
}

pub const SHORTCUT_ACTIONS: &[&str] = &["search", "pay", "suspend", "resume", "remove"];

/// اختصارات الكاشير: JSON لقاموس action -> مفتاح. لا يُسمح بتكرار نفس المفتاح.
fn validate_shortcuts(value: &str) -> Result<(), AppError> {
    let bad = |m: &str| AppError::Validation(m.to_string());
    let map: HashMap<String, String> =
        serde_json::from_str(value).map_err(|_| bad("صيغة الاختصارات غير صالحة"))?;
    let mut seen = std::collections::HashSet::new();
    for (action, key) in &map {
        if !SHORTCUT_ACTIONS.contains(&action.as_str()) {
            return Err(bad("إجراء اختصار غير معروف"));
        }
        let valid_key = key == "Delete" || (key.len() >= 2 && key.starts_with('F') && key[1..].parse::<u8>().map(|n| (1..=12).contains(&n)).unwrap_or(false));
        if !valid_key {
            return Err(bad("المفاتيح المسموحة: F1 إلى F12 أو Delete"));
        }
        if key == "F5" || key == "F11" || key == "F12" {
            return Err(bad("هذا المفتاح محجوز للنظام"));
        }
        if !seen.insert(key.clone()) {
            return Err(bad("لا يمكن استخدام نفس المفتاح لأكثر من إجراء"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_support::memory_pool;

    #[tokio::test]
    async fn defaults_are_seeded() {
        let pool = memory_pool().await;
        let s = get_all(&pool).await.unwrap();
        assert_eq!(s.get("currency").map(String::as_str), Some("EGP"));
        assert_eq!(s.get("allow_negative_stock").map(String::as_str), Some("0"));
        assert_eq!(s.len(), ALLOWED_KEYS.len());
    }

    #[tokio::test]
    async fn shortcuts_are_validated() {
        let pool = memory_pool().await;
        assert!(set(&pool, "pos_shortcuts", r#"{"pay":"F3","search":"F3"}"#).await.is_err());
        assert!(set(&pool, "pos_shortcuts", r#"{"pay":"x"}"#).await.is_err());
        assert!(set(&pool, "pos_shortcuts", r#"{"hack":"F3"}"#).await.is_err());
        set(&pool, "pos_shortcuts", r#"{"pay":"F3","search":"F1"}"#).await.unwrap();
    }

    #[tokio::test]
    async fn tax_rate_and_paper_width_are_validated() {
        let pool = memory_pool().await;
        assert!(set(&pool, "tax_rate_bp", "-1").await.is_err());
        assert!(set(&pool, "tax_rate_bp", "abc").await.is_err());
        set(&pool, "tax_rate_bp", "1400").await.unwrap();
        assert!(set(&pool, "receipt_paper_width", "70").await.is_err());
        set(&pool, "receipt_paper_width", "58").await.unwrap();
    }

    #[tokio::test]
    async fn unknown_key_is_rejected() {
        let pool = memory_pool().await;
        let err = set(&pool, "hacker_key", "1").await.unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[tokio::test]
    async fn invalid_theme_is_rejected_and_valid_is_saved() {
        let pool = memory_pool().await;
        assert!(set(&pool, "theme", "purple").await.is_err());
        set(&pool, "theme", "dark").await.unwrap();
        let s = get_all(&pool).await.unwrap();
        assert_eq!(s.get("theme").map(String::as_str), Some("dark"));
    }

    #[tokio::test]
    async fn business_profile_must_exist() {
        let pool = memory_pool().await;
        assert!(set(&pool, "business_profile_code", "nope").await.is_err());
        set(&pool, "business_profile_code", "grocery").await.unwrap();
    }

    #[tokio::test]
    async fn negative_stock_is_disabled_by_default_and_toggles() {
        let pool = memory_pool().await;
        set(&pool, "allow_negative_stock", "1").await.unwrap();
        assert!(set(&pool, "allow_negative_stock", "yes").await.is_err());
    }
}
