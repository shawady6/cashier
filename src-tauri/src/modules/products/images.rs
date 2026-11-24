//! صور المنتجات. الواجهة ترسل base64 فقط، و Rust يتحقق من النوع والحجم ويكتب في مجلد بيانات التطبيق.
//! لا يوجد أي مسار ملف يأتي من الواجهة، فلا تُعرَّض عمليات ملفات حساسة.

use std::path::Path;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::app_core::error::AppError;

const MAX_IMAGE_BYTES: usize = 1_000_000;
const DIR: &str = "product-images";

fn detect_ext(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some(("png", "image/png"))
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("jpg", "image/jpeg"))
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(("webp", "image/webp"))
    } else {
        None
    }
}

fn safe_file_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        && !name.contains("..")
}

async fn current_image(pool: &SqlitePool, product_id: &str) -> Result<Option<String>, AppError> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT image_path FROM products WHERE id = ?1")
            .bind(product_id)
            .fetch_optional(pool)
            .await?;
    match row {
        None => Err(AppError::Validation("المنتج غير موجود".into())),
        Some((path,)) => Ok(path),
    }
}

pub async fn set_image(
    pool: &SqlitePool,
    data_dir: &Path,
    product_id: &str,
    base64_data: &str,
) -> Result<(), AppError> {
    let payload = base64_data.split_once(',').map(|(_, b)| b).unwrap_or(base64_data);
    let bytes = STANDARD
        .decode(payload.trim())
        .map_err(|_| AppError::Validation("ملف الصورة غير صالح".into()))?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(AppError::Validation("حجم الصورة يجب ألا يتجاوز 1 ميجابايت".into()));
    }
    let (ext, _) = detect_ext(&bytes)
        .ok_or_else(|| AppError::Validation("الصيغ المدعومة: PNG أو JPG أو WEBP".into()))?;

    let old = current_image(pool, product_id).await?;
    let dir = data_dir.join(DIR);
    std::fs::create_dir_all(&dir)?;
    let file_name = format!("{}.{}", Uuid::new_v4(), ext);
    std::fs::write(dir.join(&file_name), &bytes)?;

    let res = sqlx::query(
        "UPDATE products SET image_path = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?2",
    )
    .bind(&file_name)
    .bind(product_id)
    .execute(pool)
    .await;
    if let Err(e) = res {
        let _ = std::fs::remove_file(dir.join(&file_name));
        return Err(e.into());
    }
    if let Some(old) = old.filter(|o| safe_file_name(o)) {
        let _ = std::fs::remove_file(dir.join(old));
    }
    Ok(())
}

pub async fn remove_image(pool: &SqlitePool, data_dir: &Path, product_id: &str) -> Result<(), AppError> {
    let old = current_image(pool, product_id).await?;
    sqlx::query(
        "UPDATE products SET image_path = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?1",
    )
    .bind(product_id)
    .execute(pool)
    .await?;
    if let Some(old) = old.filter(|o| safe_file_name(o)) {
        let _ = std::fs::remove_file(data_dir.join(DIR).join(old));
    }
    Ok(())
}

/// يرجع الصورة كـ data URL أو None إن لم توجد.
pub async fn get_image(
    pool: &SqlitePool,
    data_dir: &Path,
    product_id: &str,
) -> Result<Option<String>, AppError> {
    let Some(name) = current_image(pool, product_id).await? else {
        return Ok(None);
    };
    if !safe_file_name(&name) {
        return Err(AppError::Internal("اسم ملف الصورة غير آمن".into()));
    }
    let bytes = match std::fs::read(data_dir.join(DIR).join(&name)) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let mime = detect_ext(&bytes).map(|(_, m)| m).unwrap_or("application/octet-stream");
    Ok(Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes))))
}
