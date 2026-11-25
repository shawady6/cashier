//! كتابة المنتجات: كل حفظ داخل transaction واحدة، وأي فشل يرجّع كل شيء.

use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::quantity::parse_qty;
use crate::app_core::text::normalize;

use super::model::ProductInput;
use super::validate::{err, parse, Parsed};

type Tx<'a> = Transaction<'a, Sqlite>;

struct ProfileFlags {
    allow_weighted: bool,
    allow_variants: bool,
}

async fn profile_flags(pool: &SqlitePool) -> Result<ProfileFlags, AppError> {
    let row: Option<(bool, bool)> = sqlx::query_as(
        "SELECT bp.allow_weighted_products, bp.allow_variants FROM business_profiles bp \
         JOIN settings s ON s.setting_key = 'business_profile_code' AND s.setting_value = bp.code",
    )
    .fetch_optional(pool)
    .await?;
    let (allow_weighted, allow_variants) = row.unwrap_or((false, false));
    Ok(ProfileFlags { allow_weighted, allow_variants })
}

async fn exists(tx: &mut Tx<'_>, table: &'static str, id: &str) -> Result<bool, AppError> {
    // اسم الجدول ثابت من الكود وليس من مدخلات المستخدم
    let sql = format!("SELECT 1 FROM {table} WHERE id = ?1");
    let r: Option<i64> = sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut **tx).await?;
    Ok(r.is_some())
}

/// SKU فريد على مستوى المنتجات والمتغيرات. تعارضات نفس المنتج تُكتشف في التحليل (parse).
async fn sku_taken(tx: &mut Tx<'_>, sku: &str, own_product: &str) -> Result<bool, AppError> {
    let r: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM products WHERE sku = ?1 AND id <> ?2 \
         UNION ALL SELECT 1 FROM product_variants WHERE sku = ?1 AND product_id <> ?2 LIMIT 1",
    )
    .bind(sku)
    .bind(own_product)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(r.is_some())
}

pub async fn save(pool: &SqlitePool, input: &ProductInput) -> Result<String, AppError> {
    let p = parse(input)?;
    let flags = profile_flags(pool).await?;

    let mut tx = pool.begin().await?;
    let result = save_in_tx(&mut tx, &p, &flags).await;
    match result {
        Ok(id) => {
            tx.commit().await?;
            tracing::info!("تم حفظ المنتج {} ({})", p.name_ar, id);
            Ok(id)
        }
        Err(e) => {
            tx.rollback().await?;
            Err(map_unique(e))
        }
    }
}

/// خط الدفاع الأخير: لو فشل قيد التفرد في قاعدة البيانات نعرض رسالة عربية مفهومة.
fn map_unique(e: AppError) -> AppError {
    if let AppError::Database(sqlx::Error::Database(d)) = &e {
        if d.is_unique_violation() {
            return AppError::Validation("قيمة مكررة (باركود أو رمز صنف مستخدم بالفعل)".into());
        }
    }
    e
}

async fn save_in_tx(tx: &mut Tx<'_>, p: &Parsed<'_>, flags: &ProfileFlags) -> Result<String, AppError> {
    let src = p.src;

    // الوحدة الأساسية
    let base: Option<(bool,)> = sqlx::query_as("SELECT allows_decimal FROM units WHERE id = ?1")
        .bind(&src.base_unit_id)
        .fetch_optional(&mut **tx)
        .await?;
    let Some((allows_decimal,)) = base else { return err("الوحدة الأساسية غير موجودة") };
    let min_stock = parse_qty(&src.min_stock_qty, allows_decimal)?;

    if src.is_weighted && !allows_decimal {
        return err("المنتج الموزون يحتاج وحدة أساسية تقبل الكسور (مثل الكيلو)");
    }
    if let Some(c) = &src.category_id {
        if !exists(tx, "categories", c).await? {
            return err("التصنيف غير موجود");
        }
    }
    if let Some(b) = &src.brand_id {
        if !exists(tx, "brands", b).await? {
            return err("الماركة غير موجودة");
        }
    }
    for u in &p.units {
        if !exists(tx, "units", &u.src.unit_id).await? {
            return err("وحدة غير موجودة في قائمة وحدات المنتج");
        }
    }

    // المنتج الحالي (إن وُجد) لتطبيق قواعد نوع النشاط على التغييرات الجديدة فقط
    let (product_id, was_weighted, existing_variants) = match &src.id {
        Some(id) => {
            let row: Option<(bool,)> = sqlx::query_as("SELECT is_weighted FROM products WHERE id = ?1")
                .bind(id)
                .fetch_optional(&mut **tx)
                .await?;
            let Some((w,)) = row else { return err("المنتج غير موجود") };
            let vs: Vec<String> = sqlx::query_scalar("SELECT id FROM product_variants WHERE product_id = ?1")
                .bind(id)
                .fetch_all(&mut **tx)
                .await?;
            (id.clone(), w, vs)
        }
        None => (Uuid::new_v4().to_string(), false, vec![]),
    };
    let is_new = src.id.is_none();

    if src.is_weighted && !was_weighted && !flags.allow_weighted {
        return err("نوع النشاط الحالي لا يدعم المنتجات الموزونة");
    }
    if p.variants.iter().any(|v| v.src.id.is_none()) && !flags.allow_variants {
        return err("نوع النشاط الحالي لا يدعم متغيرات المنتج");
    }
    for v in &p.variants {
        if let Some(vid) = &v.src.id {
            if !existing_variants.contains(vid) {
                return err("متغير لا يخص هذا المنتج");
            }
        }
    }

    // تفرد SKU
    if let Some(s) = &p.sku {
        if sku_taken(tx, s, &product_id).await? {
            return err(format!("رمز الصنف {s} مستخدم بالفعل"));
        }
    }
    for v in &p.variants {
        if let Some(s) = &v.sku {
            if sku_taken(tx, s, &product_id).await? {
                return err(format!("رمز الصنف {s} مستخدم بالفعل"));
            }
        }
    }

    // تفرد الباركود (باستثناء باركودات هذا المنتج نفسه لأنها ستُستبدل)
    let all_barcodes = p
        .barcodes
        .iter()
        .chain(p.variants.iter().flat_map(|v| v.barcodes.iter()))
        .chain(p.units.iter().filter_map(|u| u.barcode.as_ref()));
    for b in all_barcodes {
        let owner: Option<String> = sqlx::query_scalar(
            "SELECT p.name_ar FROM product_barcodes pb JOIN products p ON p.id = pb.product_id \
             WHERE pb.barcode = ?1 AND pb.product_id <> ?2",
        )
        .bind(b)
        .bind(&product_id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(name) = owner {
            return err(format!("الباركود {b} مستخدم بالفعل للمنتج «{name}»"));
        }
    }

    // المنتج
    let search_text = normalize(&format!(
        "{} {} {} {}",
        p.name_ar,
        p.name_en,
        p.sku.clone().unwrap_or_default(),
        p.variants
            .iter()
            .map(|v| format!("{} {} {}", v.name_ar, v.name_en, v.sku.clone().unwrap_or_default()))
            .collect::<Vec<_>>()
            .join(" ")
    ));

    if is_new {
        sqlx::query(
            "INSERT INTO products (id, sku, name_ar, name_en, search_text, category_id, brand_id, base_unit_id, \
             purchase_price, sale_price, min_stock_qty, is_weighted, track_expiry, is_active) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        )
        .bind(&product_id)
        .bind(&p.sku)
        .bind(&p.name_ar)
        .bind(&p.name_en)
        .bind(&search_text)
        .bind(&src.category_id)
        .bind(&src.brand_id)
        .bind(&src.base_unit_id)
        .bind(p.purchase_price)
        .bind(p.sale_price)
        .bind(min_stock)
        .bind(src.is_weighted)
        .bind(src.track_expiry)
        .bind(src.is_active)
        .execute(&mut **tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE products SET sku=?2, name_ar=?3, name_en=?4, search_text=?5, category_id=?6, brand_id=?7, \
             base_unit_id=?8, purchase_price=?9, sale_price=?10, min_stock_qty=?11, is_weighted=?12, \
             track_expiry=?13, is_active=?14, updated_at=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id=?1",
        )
        .bind(&product_id)
        .bind(&p.sku)
        .bind(&p.name_ar)
        .bind(&p.name_en)
        .bind(&search_text)
        .bind(&src.category_id)
        .bind(&src.brand_id)
        .bind(&src.base_unit_id)
        .bind(p.purchase_price)
        .bind(p.sale_price)
        .bind(min_stock)
        .bind(src.is_weighted)
        .bind(src.track_expiry)
        .bind(src.is_active)
        .execute(&mut **tx)
        .await?;
    }

    // المتغيرات: تحديث/إضافة، والمحذوفة من القائمة تُعطَّل ولا تُمسح
    let mut kept = Vec::new();
    let mut variant_ids = Vec::new();
    for v in &p.variants {
        let vid = match &v.src.id {
            Some(id) => {
                sqlx::query(
                    "UPDATE product_variants SET name_ar=?2, name_en=?3, sku=?4, purchase_price=?5, sale_price=?6, \
                     is_active=?7, updated_at=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id=?1",
                )
                .bind(id)
                .bind(&v.name_ar)
                .bind(&v.name_en)
                .bind(&v.sku)
                .bind(v.purchase_price)
                .bind(v.sale_price)
                .bind(v.src.is_active)
                .execute(&mut **tx)
                .await?;
                id.clone()
            }
            None => {
                let id = Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO product_variants (id, product_id, name_ar, name_en, sku, purchase_price, sale_price, is_active) \
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                )
                .bind(&id)
                .bind(&product_id)
                .bind(&v.name_ar)
                .bind(&v.name_en)
                .bind(&v.sku)
                .bind(v.purchase_price)
                .bind(v.sale_price)
                .bind(v.src.is_active)
                .execute(&mut **tx)
                .await?;
                id
            }
        };
        kept.push(vid.clone());
        variant_ids.push(vid);
    }
    for old in existing_variants.iter().filter(|o| !kept.contains(o)) {
        sqlx::query(
            "UPDATE product_variants SET is_active = 0, updated_at=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?1",
        )
        .bind(old)
        .execute(&mut **tx)
        .await?;
    }

    // الباركود: استبدال كامل لباركودات هذا المنتج
    sqlx::query("DELETE FROM product_barcodes WHERE product_id = ?1")
        .bind(&product_id)
        .execute(&mut **tx)
        .await?;
    for b in &p.barcodes {
        insert_barcode(tx, &product_id, None, None, b).await?;
    }
    for (v, vid) in p.variants.iter().zip(&variant_ids) {
        for b in &v.barcodes {
            insert_barcode(tx, &product_id, Some(vid), None, b).await?;
        }
    }

    // الوحدات الإضافية: استبدال كامل
    sqlx::query("DELETE FROM product_units WHERE product_id = ?1")
        .bind(&product_id)
        .execute(&mut **tx)
        .await?;
    for u in &p.units {
        sqlx::query(
            "INSERT INTO product_units (id, product_id, unit_id, factor_micro, is_purchase_unit, is_sale_unit, sale_price_minor) \
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&product_id)
        .bind(&u.src.unit_id)
        .bind(u.factor_micro)
        .bind(u.src.is_purchase_unit)
        .bind(u.src.is_sale_unit)
        .bind(u.sale_price)
        .execute(&mut **tx)
        .await?;
        if let Some(b) = &u.barcode {
            insert_barcode(tx, &product_id, None, Some(&u.src.unit_id), b).await?;
        }
    }

    Ok(product_id)
}

async fn insert_barcode(
    tx: &mut Tx<'_>,
    product_id: &str,
    variant_id: Option<&str>,
    unit_id: Option<&str>,
    barcode: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO product_barcodes (id, product_id, variant_id, unit_id, barcode) VALUES (?1,?2,?3,?4,?5)")
        .bind(Uuid::new_v4().to_string())
        .bind(product_id)
        .bind(variant_id)
        .bind(unit_id)
        .bind(barcode)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// الأرشفة لا تحذف شيئًا: المنتج يختفي من القوائم والبحث ويمكن استرجاعه.
pub async fn set_archived(pool: &SqlitePool, id: &str, archived: bool) -> Result<(), AppError> {
    let r = sqlx::query(
        "UPDATE products SET archived_at = CASE WHEN ?2 = 1 THEN strftime('%Y-%m-%dT%H:%M:%SZ','now') ELSE NULL END, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?1",
    )
    .bind(id)
    .bind(archived)
    .execute(pool)
    .await?;
    if r.rows_affected() == 0 {
        return err("المنتج غير موجود");
    }
    tracing::info!("{} المنتج {id}", if archived { "تمت أرشفة" } else { "تم استرجاع" });
    Ok(())
}
