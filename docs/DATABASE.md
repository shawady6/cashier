# قاعدة البيانات

SQLite في `%APPDATA%\com.shawady.pos\pos.db`، وليست في مجلد التثبيت.

## الإعدادات
- `journal_mode = WAL`، `synchronous = FULL`، `foreign_keys = ON`، `busy_timeout = 5s`.
- التحديثات عبر `src-tauri/migrations/` (SQLx migrate). لا تعيد ضبط القاعدة ولا تحذف بيانات.

## المعايير
- المعرفات UUID نصية. التواريخ ISO-8601 بتوقيت UTC.
- الأموال أعداد صحيحة بالقروش. الكميات أعداد صحيحة بالألف (3 خانات عشرية: 1.5 كجم = 1500). معاملات تحويل الوحدات أعداد صحيحة بالمليون. التقريب عند التحويل: نصف لأعلى.

## الجداول الحالية (المرحلة 0)
- `business_profiles`: أنواع النشاط (بقالة، منظفات، عطور، تجزئة عامة) وأعلام الميزات.
- `settings`: إعدادات (مفتاح/قيمة) مع قائمة مفاتيح مسموحة يتحقق منها Rust.

## جداول المرحلة 1 (`0002_catalog.sql`)
- `categories`, `brands`: اسم عربي/إنجليزي، `name_key` فريد (اسم مُطبَّع بدون تشكيل وبتوحيد الألف والياء والتاء المربوطة)، `is_active`.
- `units`: وحدات أساسية جاهزة (قطعة، كيلو، جرام، لتر، مللي، كرتونة، عبوة، دستة، علبة) + وحدات مضافة. `allows_decimal` يحدد قبول الكسور.
- `products`: SKU فريد (جزئي)، أسعار بالقروش، `min_stock_qty` بالألف، `is_weighted`، `track_expiry`، `image_path` (اسم ملف فقط)، `is_active`، `archived_at` (أرشفة بدون حذف)، `search_text` مُطبَّع للبحث.
- `product_variants`: أسعار مستقلة وSKU فريد. المتغير المحذوف من الشاشة يُعطَّل ولا يُمسح.
- `product_barcodes`: الباركود فريد على مستوى النظام كله (UNIQUE) سواء لمنتج أو متغير.
- `product_units`: وحدات إضافية لكل منتج بمعامل تحويل للوحدة الأساسية (`factor_micro`).

الصور تُحفظ في `<مجلد البيانات>\product-images\` ويستلمها Rust كـ base64 بعد التحقق من النوع (PNG/JPG/WEBP) والحجم (≤ 1MB).

## جداول المرحلة 2 (`0003_sales.sql`)
- `sequences`: عدّادات ترقيم الفواتير (`INV-000001`) والمرتجعات (`RET-000001`) داخل نفس الـ transaction.
- `warehouses` (مخزن افتراضي)، `stock_balances` (الكمية بالألف + القيمة الإجمالية بالقروش، فمتوسط التكلفة = القيمة ÷ الكمية بدون float)، `stock_movements` (كل حركة: بيع، مرتجع، تسوية).
- `sales`, `sale_items`, `payments`: الفاتورة وبنودها (نسخة تاريخية من الاسم والسعر والتكلفة وقت البيع) والمدفوعات. `idempotency_key` فريد لمنع تكرار الفاتورة.
- `sales_returns`, `sales_return_items`: مرتجعات كسجلات عكسية. آخر مرتجع لبند يأخذ الباقي بالضبط فلا يحدث فرق تقريب.
- `cash_transactions`: حركة نقدية/كارت (بيع موجب، مرتجع سالب). ربط جلسات الخزينة في المرحلة 4.
- `suspended_sales`: السلات المعلّقة (JSON تُعاد تسعيرتها عند الاسترجاع).
- Triggers تمنع UPDATE وDELETE على `sales` و`sale_items` و`payments` و`sales_returns`.
- إعدادات جديدة: `tax_rate_bp`, `receipt_footer`, `receipt_paper_width`, `pos_shortcuts`.

## قواعد الحساب
- التقريب: نصف لأعلى بعيدًا عن الصفر على القرش (`div_round`).
- الضريبة شاملة في السعر: ضريبة السطر = المبلغ × النسبة ÷ (10000 + النسبة).
- خصم الفاتورة يُوزَّع على السطور بنسبة صافيها بطريقة «أكبر باقي» فمجموع التوزيع = الخصم بالضبط.
- سعر الوحدة الإضافية = سعر الأساسية × معامل التحويل.
- التكلفة المباعة = متوسط التكلفة المرجّح وقت البيع. عند السماح بالمخزون السالب يُكلَّف الجزء غير المتاح بسعر الشراء الحالي.

## المرحلة 3 (migration 0004)
- `suppliers`, `purchases` (draft/received/cancelled), `purchase_items`, `purchase_payments`, `purchase_returns`, `purchase_return_items`: المخزون يزيد عند الاستلام فقط؛ بعده تُمنع التعديلات بـ triggers.
- `stock_transfers` + بنودها، `stock_counts` + بنودها (لقطة رصيد النظام والمعدود؛ بعد الاعتماد لا تعديل).
- أُعيد بناء `stock_movements` (أنواع: adjustment, sale, sale_return, purchase, purchase_return, transfer_out, transfer_in, count) و`cash_transactions` (نوع جديد supplier_payment) مع الحفاظ على البيانات.
- أرصدة المورد = المشتريات المستلمة − المرتجعات − الدفعات (محسوبة ولا تُخزَّن).

## المرحلة 4 (migration 0005)
- `customers` (حد ائتماني، 0 = بلا حد)، `customer_payments` (لا تُعدَّل/تُحذف). رصيد العميل = مجموع (إجمالي − مدفوع + باقي) لفواتيره − السداد − `sales_returns.credit_minor`؛ محسوب ولا يُخزَّن.
- `sales.customer_id` و`sales_returns.credit_minor` (أعمدة جديدة عبر ALTER بدون المساس بالفواتير القديمة).
- `cash_sessions` (وردية مفتوحة واحدة بفهرس فريد جزئي، المغلقة لا تُعدَّل)، `cash_entries`، و`cash_transactions` أُعيد بناؤها بعمود `session_id` وأنواع expense/expense_void/customer_payment/cash_in/cash_out وحماية من التعديل والحذف.
- `expense_categories` (مع تصنيفات افتراضية) و`expenses` (لا حذف ولا تعديل، الإلغاء فقط).
- إعداد `require_cash_session`.

باقي الجداول (التقارير، العملاء، الخزينة…) تُضاف في المراحل القادمة.
