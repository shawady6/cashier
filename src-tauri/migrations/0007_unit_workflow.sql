-- خيارات الشراء والبيع بالوحدات لكل منتج، وسعر شراء الوحدة الكبيرة، ومصدر سعر القطعة.
-- كل الأعمدة الجديدة لها قيم افتراضية تحافظ على سلوك المنتجات الحالية كما هو.
ALTER TABLE products ADD COLUMN buy_in_base INTEGER NOT NULL DEFAULT 1 CHECK (buy_in_base IN (0, 1));
ALTER TABLE products ADD COLUMN sell_in_base INTEGER NOT NULL DEFAULT 1 CHECK (sell_in_base IN (0, 1));
ALTER TABLE products ADD COLUMN default_buy_big INTEGER NOT NULL DEFAULT 0 CHECK (default_buy_big IN (0, 1));
ALTER TABLE products ADD COLUMN default_sell_big INTEGER NOT NULL DEFAULT 0 CHECK (default_sell_big IN (0, 1));
-- manual: سعر القطعة مُدخل يدويًا. derived: محسوب من سعر بيع الوحدة الكبيرة (مقرّب لأعلى) ويُعاد حسابه عند تغيّره.
ALTER TABLE products ADD COLUMN price_source TEXT NOT NULL DEFAULT 'manual' CHECK (price_source IN ('manual', 'derived'));
ALTER TABLE product_units ADD COLUMN purchase_price_minor INTEGER CHECK (purchase_price_minor IS NULL OR purchase_price_minor >= 0);
