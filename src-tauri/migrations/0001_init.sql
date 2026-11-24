-- المرحلة 0: الأساس. أنواع النشاط + الإعدادات.
-- كل المعرفات UUID نصية، والتواريخ بصيغة ISO-8601 UTC.

CREATE TABLE business_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL UNIQUE,
    name_ar TEXT NOT NULL,
    name_en TEXT NOT NULL,
    track_expiry INTEGER NOT NULL DEFAULT 0 CHECK (track_expiry IN (0, 1)),
    track_batches INTEGER NOT NULL DEFAULT 0 CHECK (track_batches IN (0, 1)),
    allow_weighted_products INTEGER NOT NULL DEFAULT 0 CHECK (allow_weighted_products IN (0, 1)),
    allow_variants INTEGER NOT NULL DEFAULT 0 CHECK (allow_variants IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE settings (
    setting_key TEXT PRIMARY KEY NOT NULL,
    setting_value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

INSERT INTO business_profiles
    (id, code, name_ar, name_en, track_expiry, track_batches, allow_weighted_products, allow_variants)
VALUES
    ('a3c1f1d2-6b0e-4f6a-9d34-1d6a8d0f0001', 'grocery', 'بقالة / سوبر ماركت', 'Grocery / Supermarket', 1, 1, 1, 0),
    ('a3c1f1d2-6b0e-4f6a-9d34-1d6a8d0f0002', 'detergents', 'منظفات', 'Detergents', 0, 0, 0, 1),
    ('a3c1f1d2-6b0e-4f6a-9d34-1d6a8d0f0003', 'perfumes', 'عطور', 'Perfumes', 0, 0, 0, 1),
    ('a3c1f1d2-6b0e-4f6a-9d34-1d6a8d0f0004', 'general_retail', 'تجزئة عامة', 'General Retail', 0, 0, 0, 1);

INSERT INTO settings (setting_key, setting_value) VALUES
    ('store_name', ''),
    ('store_address', ''),
    ('store_phone', ''),
    ('currency', 'EGP'),
    ('language', 'ar'),
    ('theme', 'light'),
    ('business_profile_code', 'general_retail'),
    ('allow_negative_stock', '0'),
    ('setup_completed', '0');
