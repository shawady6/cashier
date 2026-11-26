-- وحدة كبيرة بسيطة (كرتونة/دستة/علبة/عبوة): سعر بيع للوحدة، باركود للوحدة، وسعر بيع على بند الشراء.
-- كلها أعمدة اختيارية (NULL) فلا تتغير أي بيانات أو فواتير قديمة. المخزون يبقى دائمًا بالوحدة الأساسية.

-- سعر بيع الوحدة الكبيرة (بالقروش). NULL = سعر القطعة × عدد القطع.
ALTER TABLE product_units ADD COLUMN sale_price_minor INTEGER
    CHECK (sale_price_minor IS NULL OR sale_price_minor >= 0);

-- باركود خاص بالوحدة الكبيرة. NULL = باركود القطعة/المتغير كما كان.
ALTER TABLE product_barcodes ADD COLUMN unit_id TEXT REFERENCES units(id);

-- سعر البيع المُدخل في بند الشراء (للمراجعة). NULL = لم يُحدَّد سعر جديد.
ALTER TABLE purchase_items ADD COLUMN new_sale_price_minor INTEGER
    CHECK (new_sale_price_minor IS NULL OR new_sale_price_minor >= 0);
