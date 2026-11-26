//! فواتير الشراء: مسودة قابلة للتعديل ← استلام (يزيد المخزون بالمتوسط المرجّح) ← مدفوعات ومرتجعات.
//! المخزون لا يتغير إلا عند الاستلام. بعد الاستلام لا تُعدَّل الفاتورة (triggers) والتصحيح بمرتجع.

use sqlx::{QueryBuilder, Sqlite, SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::app_core::error::AppError;
use crate::app_core::money::{div_round, parse_minor};
use crate::app_core::quantity::{parse_qty, to_base_qty};
use crate::modules::inventory::{apply_unit_sale_price, record_unit_purchase_price, product_info, receive, resolve_unit, warehouses::require_active, MovementRef};
use crate::modules::sales::calc::{allocate, discount_amount, parse_discount};
use crate::modules::sales::checkout::next_number;

use super::model::*;

fn bad<T>(m: impl Into<String>) -> Result<T, AppError> {
    Err(AppError::Validation(m.into()))
}

const MAX_LINES: usize = 300;

struct Line {
    product_id: String,
    variant_id: Option<String>,
    name_ar: String,
    variant_name: Option<String>,
    unit_id: String,
    unit_name: String,
    factor: i64,
    qty: i64,
    base_qty: i64,
    unit_cost: i64,
    line_total: i64,
    new_sale_price: Option<i64>,
}

pub async fn save_draft(pool: &SqlitePool, input: &PurchaseInput) -> Result<String, AppError> {
    let mut tx = pool.begin().await?;
    match save_in_tx(&mut tx, input).await {
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

async fn save_in_tx(conn: &mut SqliteConnection, input: &PurchaseInput) -> Result<String, AppError> {
    if input.items.is_empty() {
        return bad("أضف بندًا واحدًا على الأقل");
    }
    if input.items.len() > MAX_LINES {
        return bad("عدد البنود كبير جدًا");
    }
    if input.supplier_invoice_no.chars().count() > 60 || input.note.chars().count() > 500 {
        return bad("أحد الحقول أطول من الحد المسموح");
    }
    let sup: Option<(bool,)> = sqlx::query_as("SELECT is_active FROM suppliers WHERE id = ?1")
        .bind(&input.supplier_id)
        .fetch_optional(&mut *conn)
        .await?;
    match sup {
        None => return bad("المورد غير موجود"),
        Some((false,)) => return bad("المورد غير نشط"),
        _ => {}
    }
    require_active(conn, &input.warehouse_id).await?;

    let mut lines = Vec::new();
    for it in &input.items {
        let info = product_info(conn, &it.product_id, it.variant_id.as_deref()).await?;
        let ru = resolve_unit(conn, &it.product_id, it.unit_id.as_deref(), it.factor.as_deref(), true).await?;
        let (unit_id, unit_name, factor, allows_decimal) = (ru.unit_id, ru.unit_name, ru.factor_micro, ru.allows_decimal);
        let qty = parse_qty(&it.qty, allows_decimal)?;
        if qty <= 0 {
            return bad(format!("كمية «{}» يجب أن تكون أكبر من صفر", info.name));
        }
        let base_qty = to_base_qty(qty, factor)?;
        if base_qty <= 0 {
            return bad("الكمية صغيرة جدًا بعد تحويل الوحدة");
        }
        let unit_cost = parse_minor(&it.unit_cost)?;
        // إجمالي السطر = تكلفة الوحدة × الكمية. هو أصل حساب التكلفة، فلا يحدث تقريب لكل قطعة.
        let line_total = div_round(i128::from(unit_cost) * i128::from(qty), 1000) as i64;
        let new_sale_price = match it.sale_price.as_deref().map(str::trim) {
            Some(p) if !p.is_empty() => Some(parse_minor(p)?),
            _ => None,
        };
        lines.push(Line {
            product_id: it.product_id.clone(), variant_id: it.variant_id.clone(), name_ar: info.name,
            variant_name: info.variant_name, unit_id, unit_name, factor, qty, base_qty, unit_cost, line_total, new_sale_price,
        });
    }

    let subtotal: i64 = lines.iter().map(|l| l.line_total).sum();
    let discount = discount_amount(subtotal, parse_discount(&input.discount)?)?;
    let nets = allocate(discount, &lines.iter().map(|l| l.line_total).collect::<Vec<_>>());
    let total = subtotal - discount;

    let id = match &input.id {
        Some(id) => {
            let st: Option<String> = sqlx::query_scalar("SELECT status FROM purchases WHERE id = ?1")
                .bind(id)
                .fetch_optional(&mut *conn)
                .await?;
            match st.as_deref() {
                None => return bad("فاتورة الشراء غير موجودة"),
                Some("draft") => {}
                Some(_) => return bad("لا يمكن تعديل فاتورة شراء مستلمة أو ملغاة. استخدم المرتجع"),
            }
            sqlx::query(
                "UPDATE purchases SET supplier_id=?2, warehouse_id=?3, supplier_invoice_no=?4, note=?5, subtotal_minor=?6, \
                 discount_minor=?7, total_minor=?8, updated_at=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id=?1",
            )
            .bind(id)
            .bind(&input.supplier_id)
            .bind(&input.warehouse_id)
            .bind(input.supplier_invoice_no.trim())
            .bind(input.note.trim())
            .bind(subtotal)
            .bind(discount)
            .bind(total)
            .execute(&mut *conn)
            .await?;
            sqlx::query("DELETE FROM purchase_items WHERE purchase_id = ?1").bind(id).execute(&mut *conn).await?;
            id.clone()
        }
        None => {
            let id = Uuid::new_v4().to_string();
            let no = next_number(conn, "purchase", "PUR-").await?;
            sqlx::query(
                "INSERT INTO purchases (id, purchase_no, supplier_id, warehouse_id, supplier_invoice_no, note, subtotal_minor, discount_minor, total_minor) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            )
            .bind(&id)
            .bind(&no)
            .bind(&input.supplier_id)
            .bind(&input.warehouse_id)
            .bind(input.supplier_invoice_no.trim())
            .bind(input.note.trim())
            .bind(subtotal)
            .bind(discount)
            .bind(total)
            .execute(&mut *conn)
            .await?;
            id
        }
    };

    for (i, (l, alloc)) in lines.iter().zip(&nets).enumerate() {
        sqlx::query(
            "INSERT INTO purchase_items (id, purchase_id, line_no, product_id, variant_id, name_ar, variant_name, unit_id, unit_name, \
             unit_factor_micro, qty_milli, base_qty_milli, unit_cost_minor, line_total_minor, net_minor, new_sale_price_minor) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&id)
        .bind(i as i64 + 1)
        .bind(&l.product_id)
        .bind(&l.variant_id)
        .bind(&l.name_ar)
        .bind(&l.variant_name)
        .bind(&l.unit_id)
        .bind(&l.unit_name)
        .bind(l.factor)
        .bind(l.qty)
        .bind(l.base_qty)
        .bind(l.unit_cost)
        .bind(l.line_total)
        .bind(l.line_total - alloc)
        .bind(l.new_sale_price)
        .execute(&mut *conn)
        .await?;
    }
    Ok(id)
}

/// الاستلام: يزيد المخزون بقيمة كل بند (بعد الخصم) ويحدّث سعر الشراء المرجعي للمنتج.
pub async fn receive_purchase(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    match receive_in_tx(&mut tx, id).await {
        Ok(()) => {
            tx.commit().await?;
            tracing::info!("تم استلام فاتورة الشراء {id}");
            Ok(())
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

async fn receive_in_tx(conn: &mut SqliteConnection, id: &str) -> Result<(), AppError> {
    let head: Option<(String, String)> = sqlx::query_as("SELECT status, warehouse_id FROM purchases WHERE id = ?1")
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
    let Some((status, wh)) = head else { return bad("فاتورة الشراء غير موجودة") };
    if status != "draft" {
        return bad("هذه الفاتورة تم استلامها أو إلغاؤها بالفعل");
    }
    require_active(conn, &wh).await?;
    let items: Vec<(String, Option<String>, i64, i64, String, Option<i64>)> = sqlx::query_as(
        "SELECT product_id, variant_id, base_qty_milli, net_minor, unit_id, new_sale_price_minor \
         FROM purchase_items WHERE purchase_id = ?1 ORDER BY line_no",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    if items.is_empty() {
        return bad("الفاتورة بلا بنود");
    }
    for (pid, vid, base_qty, net, unit_id, new_price) in items {
        let m = MovementRef { movement_type: "purchase", ref_type: "purchase", ref_id: id, note: "" };
        receive(conn, &wh, &pid, vid.as_deref(), base_qty, net, &m).await?;
        // سعر الشراء المرجعي للقطعة = قيمة السطر ÷ القطع (للعرض فقط؛ المخزون يحمل القيمة الكاملة).
        let unit_cost = div_round(i128::from(net) * 1000, i128::from(base_qty)) as i64;
        record_unit_purchase_price(conn, &pid, &unit_id, net, base_qty).await?;
        if let Some(p) = new_price {
            apply_unit_sale_price(conn, &pid, vid.as_deref(), &unit_id, p).await?;
        }
        match &vid {
            Some(v) => {
                sqlx::query("UPDATE product_variants SET purchase_price = ?2 WHERE id = ?1").bind(v).bind(unit_cost).execute(&mut *conn).await?;
            }
            None => {
                sqlx::query("UPDATE products SET purchase_price = ?2 WHERE id = ?1").bind(&pid).bind(unit_cost).execute(&mut *conn).await?;
            }
        }
    }
    sqlx::query("UPDATE purchases SET status='received', received_at=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn cancel_draft(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let r = sqlx::query("UPDATE purchases SET status = 'cancelled' WHERE id = ?1 AND status = 'draft'")
        .bind(id)
        .execute(pool)
        .await?;
    if r.rows_affected() == 0 {
        return bad("يمكن إلغاء المسودات فقط");
    }
    Ok(())
}

pub async fn list(pool: &SqlitePool, f: &PurchaseFilter) -> Result<Vec<PurchaseRow>, AppError> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT p.id, p.purchase_no, s.name_ar AS supplier_name, p.supplier_invoice_no, p.status, p.total_minor, \
                COALESCE((SELECT SUM(amount_minor) FROM purchase_payments WHERE purchase_id = p.id), 0) AS paid_minor, \
                COALESCE((SELECT SUM(total_minor) FROM purchase_returns WHERE purchase_id = p.id), 0) AS returned_minor, \
                p.created_at, p.received_at \
         FROM purchases p JOIN suppliers s ON s.id = p.supplier_id WHERE 1 = 1",
    );
    if let Some(v) = &f.supplier_id {
        qb.push(" AND p.supplier_id = ").push_bind(v.clone());
    }
    if let Some(v) = f.status.as_ref().filter(|v| ["draft", "received", "cancelled"].contains(&v.as_str())) {
        qb.push(" AND p.status = ").push_bind(v.clone());
    }
    if let Some(q) = f.query.as_ref().map(|q| q.trim()).filter(|q| !q.is_empty()) {
        let pat = format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
        qb.push(" AND (p.purchase_no LIKE ").push_bind(pat.clone()).push(" ESCAPE '\\' OR p.supplier_invoice_no LIKE ")
            .push_bind(pat).push(" ESCAPE '\\')");
    }
    qb.push(" ORDER BY p.created_at DESC, p.purchase_no DESC LIMIT 300");
    Ok(qb.build_query_as::<PurchaseRow>().fetch_all(pool).await?)
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<PurchaseDetail, AppError> {
    let header = sqlx::query_as::<_, PurchaseRow>(
        "SELECT p.id, p.purchase_no, s.name_ar AS supplier_name, p.supplier_invoice_no, p.status, p.total_minor, \
                COALESCE((SELECT SUM(amount_minor) FROM purchase_payments WHERE purchase_id = p.id), 0) AS paid_minor, \
                COALESCE((SELECT SUM(total_minor) FROM purchase_returns WHERE purchase_id = p.id), 0) AS returned_minor, \
                p.created_at, p.received_at \
         FROM purchases p JOIN suppliers s ON s.id = p.supplier_id WHERE p.id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Validation("فاتورة الشراء غير موجودة".into()))?;
    let extra: (String, String, String, String, i64, i64) = sqlx::query_as(
        "SELECT p.supplier_id, p.warehouse_id, w.name_ar, p.note, p.subtotal_minor, p.discount_minor \
         FROM purchases p JOIN warehouses w ON w.id = p.warehouse_id WHERE p.id = ?1",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    let items = sqlx::query_as::<_, PurchaseItemDetail>(
        "SELECT pi.id, pi.line_no, pi.product_id, pi.variant_id, pi.name_ar, pi.variant_name, pi.unit_id, pi.unit_name, \
                pi.unit_factor_micro, pi.qty_milli, pi.base_qty_milli, pi.unit_cost_minor, pi.line_total_minor, pi.net_minor, pi.new_sale_price_minor, \
                COALESCE((SELECT SUM(qty_milli) FROM purchase_return_items WHERE purchase_item_id = pi.id), 0) AS returned_qty_milli \
         FROM purchase_items pi WHERE pi.purchase_id = ?1 ORDER BY pi.line_no",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let payments = sqlx::query_as::<_, PurchasePayment>(
        "SELECT id, method, amount_minor, note, created_at FROM purchase_payments WHERE purchase_id = ?1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let returns = sqlx::query_as::<_, PurchaseReturnRow>(
        "SELECT id, return_no, total_minor, note, created_at FROM purchase_returns WHERE purchase_id = ?1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let outstanding = if header.status == "received" {
        header.total_minor - header.returned_minor - header.paid_minor
    } else {
        0
    };
    Ok(PurchaseDetail {
        supplier_id: extra.0, warehouse_id: extra.1, warehouse_name: extra.2, note: extra.3,
        subtotal_minor: extra.4, discount_minor: extra.5, outstanding_minor: outstanding,
        header, items, payments, returns,
    })
}

/// دفعة لمورد على فاتورة مستلمة. لا تتجاوز المتبقي. النقد/الكارت يُسجَّل في حركة النقدية (سالب).
pub async fn add_payment(pool: &SqlitePool, input: &SupplierPaymentInput) -> Result<String, AppError> {
    if !["cash", "card"].contains(&input.method.as_str()) {
        return bad("طريقة دفع غير مدعومة");
    }
    let amount = parse_minor(&input.amount)?;
    if amount <= 0 {
        return bad("مبلغ الدفعة يجب أن يكون أكبر من صفر");
    }
    let mut tx = pool.begin().await?;
    let row: Option<(String, String, i64)> =
        sqlx::query_as("SELECT status, supplier_id, total_minor FROM purchases WHERE id = ?1")
            .bind(&input.purchase_id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((status, supplier_id, total)) = row else { return bad("فاتورة الشراء غير موجودة") };
    if status != "received" {
        return bad("الدفع يتم على الفواتير المستلمة فقط");
    }
    let paid: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(amount_minor),0) FROM purchase_payments WHERE purchase_id = ?1")
        .bind(&input.purchase_id)
        .fetch_one(&mut *tx)
        .await?;
    let returned: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(total_minor),0) FROM purchase_returns WHERE purchase_id = ?1")
        .bind(&input.purchase_id)
        .fetch_one(&mut *tx)
        .await?;
    if amount > total - returned - paid {
        return bad("المبلغ أكبر من المتبقي على الفاتورة");
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO purchase_payments (id, purchase_id, supplier_id, method, amount_minor, note) VALUES (?1,?2,?3,?4,?5,?6)")
        .bind(&id)
        .bind(&input.purchase_id)
        .bind(&supplier_id)
        .bind(&input.method)
        .bind(amount)
        .bind(input.note.trim())
        .execute(&mut *tx)
        .await?;
    crate::modules::finance::record_tx(&mut tx, "supplier_payment", &input.method, -amount, "purchase_payment", &id).await?;
    tx.commit().await?;
    Ok(id)
}
