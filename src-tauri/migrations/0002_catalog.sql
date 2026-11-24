-- المرحلة 1: الكتالوج (تصنيفات، ماركات، وحدات، منتجات، متغيرات، باركود).
-- الأموال: أعداد صحيحة بالقروش. الكميات: أعداد صحيحة بالألف (3 خانات عشرية).
-- معامل التحويل بين الوحدات: عدد صحيح بالمليون (6 خانات عشرية).

CREATE TABLE categories (
    id TEXT PRIMARY KEY NOT NULL,
    name_ar TEXT NOT NULL,
    name_en TEXT NOT NULL DEFAULT '',
    name_key TEXT NOT NULL UNIQUE,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE brands (
    id TEXT PRIMARY KEY NOT NULL,
    name_ar TEXT NOT NULL,
    name_en TEXT NOT NULL DEFAULT '',
    name_key TEXT NOT NULL UNIQUE,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE units (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL UNIQUE,
    name_ar TEXT NOT NULL,
    name_en TEXT NOT NULL DEFAULT '',
    allows_decimal INTEGER NOT NULL DEFAULT 0 CHECK (allows_decimal IN (0, 1)),
    is_system INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO units (id, code, name_ar, name_en, allows_decimal, is_system) VALUES
    ('b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0001', 'piece',  'قطعة',   'Piece',  0, 1),
    ('b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0002', 'kg',     'كيلو',   'Kilogram', 1, 1),
    ('b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0003', 'g',      'جرام',   'Gram',   1, 1),
    ('b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0004', 'liter',  'لتر',    'Liter',  1, 1),
    ('b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0005', 'ml',     'مللي',   'Milliliter', 1, 1),
    ('b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0006', 'carton', 'كرتونة', 'Carton', 0, 1),
    ('b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0007', 'pack',   'عبوة',   'Pack',   0, 1),
    ('b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0008', 'dozen',  'دستة',   'Dozen',  0, 1),
    ('b4d2e2e3-7c1f-4a7b-8e45-2e7b9e1a0009', 'box',    'علبة',   'Box',    0, 1);

CREATE TABLE products (
    id TEXT PRIMARY KEY NOT NULL,
    sku TEXT,
    name_ar TEXT NOT NULL,
    name_en TEXT NOT NULL DEFAULT '',
    search_text TEXT NOT NULL DEFAULT '',
    category_id TEXT REFERENCES categories(id),
    brand_id TEXT REFERENCES brands(id),
    base_unit_id TEXT NOT NULL REFERENCES units(id),
    purchase_price INTEGER NOT NULL DEFAULT 0 CHECK (purchase_price >= 0),
    sale_price INTEGER NOT NULL DEFAULT 0 CHECK (sale_price >= 0),
    min_stock_qty INTEGER NOT NULL DEFAULT 0 CHECK (min_stock_qty >= 0),
    is_weighted INTEGER NOT NULL DEFAULT 0 CHECK (is_weighted IN (0, 1)),
    track_expiry INTEGER NOT NULL DEFAULT 0 CHECK (track_expiry IN (0, 1)),
    image_path TEXT,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    archived_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE UNIQUE INDEX uq_products_sku ON products(sku) WHERE sku IS NOT NULL;
CREATE INDEX idx_products_category ON products(category_id);
CREATE INDEX idx_products_brand ON products(brand_id);
CREATE INDEX idx_products_status ON products(archived_at, is_active);

CREATE TABLE product_variants (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id),
    name_ar TEXT NOT NULL,
    name_en TEXT NOT NULL DEFAULT '',
    sku TEXT,
    purchase_price INTEGER NOT NULL DEFAULT 0 CHECK (purchase_price >= 0),
    sale_price INTEGER NOT NULL DEFAULT 0 CHECK (sale_price >= 0),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE UNIQUE INDEX uq_variants_sku ON product_variants(sku) WHERE sku IS NOT NULL;
CREATE INDEX idx_variants_product ON product_variants(product_id);

-- الباركود فريد على مستوى النظام كله (منتج أو متغير).
CREATE TABLE product_barcodes (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    barcode TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_barcodes_product ON product_barcodes(product_id);

-- وحدات إضافية للمنتج غير الوحدة الأساسية. factor_micro = كم وحدة أساسية في هذه الوحدة × 1,000,000.
CREATE TABLE product_units (
    id TEXT PRIMARY KEY NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id),
    unit_id TEXT NOT NULL REFERENCES units(id),
    factor_micro INTEGER NOT NULL CHECK (factor_micro > 0),
    is_purchase_unit INTEGER NOT NULL DEFAULT 0 CHECK (is_purchase_unit IN (0, 1)),
    is_sale_unit INTEGER NOT NULL DEFAULT 0 CHECK (is_sale_unit IN (0, 1)),
    UNIQUE (product_id, unit_id)
);
