//! قراءة المنتجات والبحث والوحدات.

use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::text::{ascii_digits, normalize};

use super::model::*;
use super::validate::err;

fn like_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

pub async fn list(pool: &SqlitePool, f: &ProductFilter) -> Result<Vec<ProductListItem>, AppError> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT p.id, p.sku, p.name_ar, p.name_en, c.name_ar AS category_name, b.name_ar AS brand_name, \
                u.name_ar AS base_unit_name, p.purchase_price, p.sale_price, p.min_stock_qty, p.is_active, \
                (p.archived_at IS NOT NULL) AS archived, \
                (SELECT COUNT(*) FROM product_variants v WHERE v.product_id = p.id AND v.is_active = 1) AS variant_count, \
                (SELECT barcode FROM product_barcodes pb WHERE pb.product_id = p.id AND pb.variant_id IS NULL \
                   ORDER BY pb.created_at, pb.barcode LIMIT 1) AS first_barcode, \
                COALESCE((SELECT SUM(sb.qty_milli) FROM stock_balances sb WHERE sb.product_id = p.id), 0) AS stock_qty \
         FROM products p \
         JOIN units u ON u.id = p.base_unit_id \
         LEFT JOIN categories c ON c.id = p.category_id \
         LEFT JOIN brands b ON b.id = p.brand_id \
         WHERE 1 = 1",
    );
    if !f.include_archived {
        qb.push(" AND p.archived_at IS NULL");
    }
    if let Some(c) = &f.category_id {
        qb.push(" AND p.category_id = ").push_bind(c.clone());
    }
    if let Some(b) = &f.brand_id {
        qb.push(" AND p.brand_id = ").push_bind(b.clone());
    }
    if let Some(q) = &f.query {
        // كل كلمة يجب أن توجد في الاسم/SKU (بعد التطبيع) أو في باركود المنتج
        for token in normalize(q).split(' ').filter(|t| !t.is_empty()) {
            let pattern = format!("%{}%", like_escape(token));
            qb.push(" AND (p.search_text LIKE ")
                .push_bind(pattern.clone())
                .push(" ESCAPE '\\' OR EXISTS (SELECT 1 FROM product_barcodes pb WHERE pb.product_id = p.id AND pb.barcode LIKE ")
                .push_bind(pattern)
                .push(" ESCAPE '\\'))");
        }
    }
    qb.push(" ORDER BY p.name_ar LIMIT ")
        .push_bind(f.limit.unwrap_or(200).clamp(1, 500))
        .push(" OFFSET ")
        .push_bind(f.offset.unwrap_or(0).max(0));
    Ok(qb.build_query_as::<ProductListItem>().fetch_all(pool).await?)
}

#[derive(sqlx::FromRow)]
struct ProductRow {
    id: String,
    sku: Option<String>,
    name_ar: String,
    name_en: String,
    category_id: Option<String>,
    brand_id: Option<String>,
    base_unit_id: String,
    purchase_price: i64,
    sale_price: i64,
    min_stock_qty: i64,
    is_weighted: bool,
    track_expiry: bool,
    is_active: bool,
    archived: bool,
    has_image: bool,
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<ProductDetail, AppError> {
    let row = sqlx::query_as::<_, ProductRow>(
        "SELECT id, sku, name_ar, name_en, category_id, brand_id, base_unit_id, purchase_price, sale_price, \
                min_stock_qty, is_weighted, track_expiry, is_active, (archived_at IS NOT NULL) AS archived, \
                (image_path IS NOT NULL) AS has_image FROM products WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let Some(r) = row else { return err("المنتج غير موجود") };

    let barcodes: Vec<(Option<String>, String)> = sqlx::query_as(
        "SELECT variant_id, barcode FROM product_barcodes WHERE product_id = ?1 ORDER BY created_at, barcode",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let variant_rows: Vec<(String, String, String, Option<String>, i64, i64, bool)> = sqlx::query_as(
        "SELECT id, name_ar, name_en, sku, purchase_price, sale_price, is_active \
         FROM product_variants WHERE product_id = ?1 ORDER BY is_active DESC, name_ar",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let variants = variant_rows
        .into_iter()
        .map(|(vid, name_ar, name_en, sku, purchase_price, sale_price, is_active)| VariantDetail {
            barcodes: barcodes
                .iter()
                .filter(|(v, _)| v.as_deref() == Some(vid.as_str()))
                .map(|(_, b)| b.clone())
                .collect(),
            id: vid,
            name_ar,
            name_en,
            sku,
            purchase_price,
            sale_price,
            is_active,
        })
        .collect();

    let units = sqlx::query_as::<_, ProductUnitDetail>(
        "SELECT unit_id, factor_micro, is_purchase_unit, is_sale_unit FROM product_units \
         WHERE product_id = ?1 ORDER BY factor_micro DESC",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(ProductDetail {
        id: r.id,
        sku: r.sku,
        name_ar: r.name_ar,
        name_en: r.name_en,
        category_id: r.category_id,
        brand_id: r.brand_id,
        base_unit_id: r.base_unit_id,
        purchase_price: r.purchase_price,
        sale_price: r.sale_price,
        min_stock_qty: r.min_stock_qty,
        is_weighted: r.is_weighted,
        track_expiry: r.track_expiry,
        is_active: r.is_active,
        archived: r.archived,
        has_image: r.has_image,
        barcodes: barcodes.iter().filter(|(v, _)| v.is_none()).map(|(_, b)| b.clone()).collect(),
        variants,
        units,
    })
}

/// بحث دقيق بالباركود (للماسح الضوئي في الكاشير لاحقًا).
pub async fn find_by_barcode(pool: &SqlitePool, barcode: &str) -> Result<Option<BarcodeMatch>, AppError> {
    let code = ascii_digits(barcode.trim());
    Ok(sqlx::query_as::<_, BarcodeMatch>(
        "SELECT p.id AS product_id, v.id AS variant_id, p.name_ar, v.name_ar AS variant_name_ar, \
                COALESCE(v.sale_price, p.sale_price) AS sale_price, \
                (p.is_active = 1 AND COALESCE(v.is_active, 1) = 1) AS is_active, \
                (p.archived_at IS NOT NULL) AS archived \
         FROM product_barcodes pb \
         JOIN products p ON p.id = pb.product_id \
         LEFT JOIN product_variants v ON v.id = pb.variant_id \
         WHERE pb.barcode = ?1",
    )
    .bind(code)
    .fetch_optional(pool)
    .await?)
}

pub async fn list_units(pool: &SqlitePool) -> Result<Vec<Unit>, AppError> {
    Ok(sqlx::query_as::<_, Unit>(
        "SELECT id, code, name_ar, name_en, allows_decimal, is_system FROM units ORDER BY is_system DESC, name_ar",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn create_unit(pool: &SqlitePool, input: &UnitInput) -> Result<String, AppError> {
    let name = input.name_ar.trim();
    if name.is_empty() || name.chars().count() > 50 {
        return err("اسم الوحدة مطلوب ولا يزيد عن 50 حرفًا");
    }
    let key = normalize(name);
    for u in list_units(pool).await? {
        if normalize(&u.name_ar) == key {
            return err("توجد وحدة بنفس الاسم");
        }
    }
    let id = Uuid::new_v4().to_string();
    let code = format!("custom-{}", &id[..8]);
    sqlx::query("INSERT INTO units (id, code, name_ar, name_en, allows_decimal, is_system) VALUES (?1,?2,?3,?4,?5,0)")
        .bind(&id)
        .bind(code)
        .bind(name)
        .bind(input.name_en.trim())
        .bind(input.allows_decimal)
        .execute(pool)
        .await?;
    Ok(id)
}

#[derive(sqlx::FromRow)]
struct SellableRow {
    product_id: String,
    variant_id: Option<String>,
    name_ar: String,
    variant_name: Option<String>,
    sku: Option<String>,
    barcode: Option<String>,
    sale_price: i64,
    stock_qty: i64,
    base_unit_id: String,
    base_unit_name: String,
    allows_decimal: bool,
}

/// بحث الكاشير: منتجات نشطة وغير مؤرشفة، والمنتج ذو المتغيرات يظهر كمتغيرات.
/// يطابق الاسم (مُطبَّعًا) أو SKU أو الباركود.
pub async fn search_sellables(pool: &SqlitePool, query: &str, limit: i64) -> Result<Vec<Sellable>, AppError> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT * FROM ( \
           SELECT p.id AS product_id, NULL AS variant_id, p.name_ar, NULL AS variant_name, p.sku, \
                  (SELECT barcode FROM product_barcodes pb WHERE pb.product_id = p.id AND pb.variant_id IS NULL ORDER BY pb.created_at LIMIT 1) AS barcode, \
                  p.sale_price, \
                  COALESCE((SELECT SUM(sb.qty_milli) FROM stock_balances sb WHERE sb.product_id = p.id AND sb.variant_id IS NULL), 0) AS stock_qty, \
                  p.base_unit_id, u.name_ar AS base_unit_name, u.allows_decimal, p.search_text AS search_text \
           FROM products p JOIN units u ON u.id = p.base_unit_id \
           WHERE p.archived_at IS NULL AND p.is_active = 1 \
             AND NOT EXISTS (SELECT 1 FROM product_variants v WHERE v.product_id = p.id AND v.is_active = 1) \
           UNION ALL \
           SELECT p.id, v.id, p.name_ar, v.name_ar, COALESCE(v.sku, p.sku), \
                  (SELECT barcode FROM product_barcodes pb WHERE pb.variant_id = v.id ORDER BY pb.created_at LIMIT 1), \
                  v.sale_price, \
                  COALESCE((SELECT SUM(sb.qty_milli) FROM stock_balances sb WHERE sb.variant_id = v.id), 0), \
                  p.base_unit_id, u.name_ar, u.allows_decimal, p.search_text \
           FROM product_variants v JOIN products p ON p.id = v.product_id JOIN units u ON u.id = p.base_unit_id \
           WHERE p.archived_at IS NULL AND p.is_active = 1 AND v.is_active = 1 \
         ) s WHERE 1 = 1",
    );
    for token in normalize(query).split(' ').filter(|t| !t.is_empty()) {
        let pattern = format!("%{}%", like_escape(token));
        qb.push(" AND (s.search_text LIKE ")
            .push_bind(pattern.clone())
            .push(" ESCAPE '\\' OR EXISTS (SELECT 1 FROM product_barcodes pb WHERE pb.product_id = s.product_id \
                    AND (s.variant_id IS NULL OR pb.variant_id = s.variant_id) AND pb.barcode LIKE ")
            .push_bind(pattern)
            .push(" ESCAPE '\\'))");
    }
    qb.push(" ORDER BY s.name_ar, s.variant_name LIMIT ").push_bind(limit.clamp(1, 100));
    let rows = qb.build_query_as::<SellableRow>().fetch_all(pool).await?;

    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let mut units: Vec<SellUnit> = vec![SellUnit {
            unit_id: r.base_unit_id.clone(),
            name_ar: r.base_unit_name.clone(),
            factor_micro: 1_000_000,
            allows_decimal: r.allows_decimal,
        }];
        units.extend(
            sqlx::query_as::<_, SellUnit>(
                "SELECT pu.unit_id, u.name_ar, pu.factor_micro, u.allows_decimal FROM product_units pu \
                 JOIN units u ON u.id = pu.unit_id WHERE pu.product_id = ?1 AND pu.is_sale_unit = 1 ORDER BY pu.factor_micro",
            )
            .bind(&r.product_id)
            .fetch_all(pool)
            .await?,
        );
        out.push(Sellable {
            product_id: r.product_id, variant_id: r.variant_id, name_ar: r.name_ar, variant_name: r.variant_name,
            sku: r.sku, barcode: r.barcode, sale_price: r.sale_price, stock_qty: r.stock_qty,
            base_unit_id: r.base_unit_id, base_unit_name: r.base_unit_name, allows_decimal: r.allows_decimal, units,
        });
    }
    Ok(out)
}
