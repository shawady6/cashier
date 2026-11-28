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

## Migration 0006 — الوحدات الكبيرة (كرتونة/دستة/علبة)
- `product_units.sale_price_minor` (اختياري): سعر بيع الوحدة الكبيرة. يُحدَّث عند استلام فاتورة شراء أو تسوية.
- `product_barcodes.unit_id` (اختياري): باركود خاص بالوحدة الكبيرة؛ الباركود العادي `unit_id IS NULL`.
- `purchase_items.new_sale_price_minor` (اختياري): سعر البيع المُدخل في السطر، للتدقيق.
- المخزون دائمًا بالوحدة الأساسية؛ `purchase_items.unit_factor_micro` و`base_qty_milli` تحفظان لقطة التحويل.
- ملاحظة: المواصفات ذكرت 0005 لكنها كانت محجوزة للمالية، فالرقم 0006.

## Migration 0007 — خيارات الشراء والبيع بالوحدات
- `products.buy_in_base` / `sell_in_base`: هل يُشترى/يُباع بالقطعة (الافتراضي 1). و`default_buy_big` / `default_sell_big`: الوحدة الكبيرة هي الافتراضية.
- `products.price_source`: `manual` (سعر القطعة مُدخل) أو `derived` (محسوب من سعر بيع الوحدة الكبيرة ÷ العدد، مقرّبًا لأعلى، ويُعاد حسابه عند تغيّر سعر الوحدة بالاستلام).
- `product_units.purchase_price_minor`: آخر سعر شراء (جملة) للوحدة الكبيرة، يُحدَّث عند الاستلام.
- كل الأعمدة الجديدة لها قيم افتراضية تحافظ على سلوك المنتجات القديمة.
- قبل تطبيق أي migration على قاعدة فيها بيانات تُحفظ نسخة كاملة في `backups/pre-migration-*.db` (VACUUM INTO + integrity_check)، وتبقى آخر 5 نسخ. فشل النسخة يوقف التحديث.

## النسخ الاحتياطي (6.1)
- لا migration جديد. مفاتيح إعدادات اختيارية: `backup_auto_enabled` (افتراضي مفعّل)، `backup_keep_count` (3–365، افتراضي 14)، `backup_external_dir`.
- الملفات في `backups/`: `manual-*`, `auto-*` (يُحتفظ بآخر N)، `pre-migration-*` (آخر 5)، `pre-restore-*` (آخر 5). النسخ اليدوية لا تُحذف تلقائيًا.
- الاستعادة: `pending-restore.db` بجوار `pos.db` يُفحص ويُستبدل عند بدء التشغيل التالي قبل فتح القاعدة، وتُحذف ملفات `-wal/-shm` القديمة.
- يُرفض أي ملف تالف، أو ليس لهذا البرنامج، أو من إصدار بنية أحدث.

## إعدادات الأجهزة والتحديث (بدون migration)
- `printer_mode` (browser|thermal)، `printer_name`، `printer_cut`، `print_auto`، `print_copies` (1–3)، `receipt_paper_width` (58|80).
- `drawer_mode` (off|cash|always)، `drawer_pin` (0|1).
- `scanner_min_length` (1–30)، `scanner_max_gap_ms` (10–500)، `scanner_global`.
- `update_channel` (test|stable). نسخ ما قبل التحديث: `backups/pre-update-*` (آخر 5).
- `backup_external_dir` فارغ = مجلد `Documents\POS System\Backups` الافتراضي.

