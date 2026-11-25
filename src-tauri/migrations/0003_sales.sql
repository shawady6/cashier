-- المرحلة 2: الكاشير. مخزون أساسي (مخزن + أرصدة + حركات) يلزم لخصم المبيعات،
-- ثم الفواتير والمدفوعات والمرتجعات والفواتير المعلّقة.
-- كل المبالغ بالقروش، الكميات بالألف، النسب بالنقاط الأساسية (basis points: 14% = 1400).

CREATE TABLE sequences (
    name TEXT PRIMARY KEY NOT NULL,
    next_value INTEGER NOT NULL CHECK (next_value >= 1)
);
INSERT INTO sequences (name, next_value) VALUES ('invoice', 1), ('return', 1);

CREATE TABLE warehouses (
    id TEXT PRIMARY KEY NOT NULL,
    name_ar TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
INSERT INTO warehouses (id, name_ar, is_default) VALUES ('c5e3a1b0-2d4f-4b6a-9c11-7a1e5d300001', 'المخزن الرئيسي', 1);

-- value_minor = القيمة الإجمالية للرصيد بالقروش. متوسط التكلفة = value / qty (بدون كسور عائمة).
CREATE TABLE stock_balances (
    id TEXT PRIMARY KEY NOT NULL,
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    qty_milli INTEGER NOT NULL DEFAULT 0,
    value_minor INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE UNIQUE INDEX uq_stock_product ON stock_balances(warehouse_id, product_id) WHERE variant_id IS NULL;
CREATE UNIQUE INDEX uq_stock_variant ON stock_balances(warehouse_id, product_id, variant_id) WHERE variant_id IS NOT NULL;

CREATE TABLE stock_movements (
    id TEXT PRIMARY KEY NOT NULL,
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    movement_type TEXT NOT NULL CHECK (movement_type IN ('adjustment', 'sale', 'sale_return')),
    qty_delta_milli INTEGER NOT NULL,
    value_delta_minor INTEGER NOT NULL,
    ref_type TEXT,
    ref_id TEXT,
    note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_movements_product ON stock_movements(product_id, created_at);
CREATE INDEX idx_movements_ref ON stock_movements(ref_type, ref_id);

-- فاتورة مكتملة لا تُعدَّل ولا تُحذف أبدًا (تُفرض بـ triggers). التصحيح بالمرتجعات فقط.
CREATE TABLE sales (
    id TEXT PRIMARY KEY NOT NULL,
    invoice_no TEXT NOT NULL UNIQUE,
    idempotency_key TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'completed' CHECK (status IN ('completed')),
    payment_status TEXT NOT NULL DEFAULT 'paid' CHECK (payment_status IN ('paid', 'partial', 'unpaid')),
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    subtotal_minor INTEGER NOT NULL,
    item_discount_minor INTEGER NOT NULL,
    invoice_discount_minor INTEGER NOT NULL,
    tax_minor INTEGER NOT NULL,
    total_minor INTEGER NOT NULL CHECK (total_minor >= 0),
    paid_minor INTEGER NOT NULL,
    change_minor INTEGER NOT NULL,
    cost_minor INTEGER NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_sales_created ON sales(created_at);

-- نسخة تاريخية من اسم المنتج والسعر وقت البيع (لا تتأثر بتعديل المنتج لاحقًا).
CREATE TABLE sale_items (
    id TEXT PRIMARY KEY NOT NULL,
    sale_id TEXT NOT NULL REFERENCES sales(id),
    line_no INTEGER NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    name_ar TEXT NOT NULL,
    variant_name TEXT,
    sku TEXT,
    unit_id TEXT NOT NULL REFERENCES units(id),
    unit_name TEXT NOT NULL,
    unit_factor_micro INTEGER NOT NULL CHECK (unit_factor_micro > 0),
    qty_milli INTEGER NOT NULL CHECK (qty_milli > 0),
    base_qty_milli INTEGER NOT NULL CHECK (base_qty_milli > 0),
    unit_price_minor INTEGER NOT NULL CHECK (unit_price_minor >= 0),
    gross_minor INTEGER NOT NULL,
    discount_minor INTEGER NOT NULL,
    invoice_discount_minor INTEGER NOT NULL,
    tax_rate_bp INTEGER NOT NULL,
    tax_minor INTEGER NOT NULL,
    total_minor INTEGER NOT NULL,
    cost_minor INTEGER NOT NULL
);
CREATE INDEX idx_sale_items_sale ON sale_items(sale_id);

CREATE TABLE payments (
    id TEXT PRIMARY KEY NOT NULL,
    sale_id TEXT NOT NULL REFERENCES sales(id),
    method TEXT NOT NULL CHECK (method IN ('cash', 'card')),
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    reference TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_payments_sale ON payments(sale_id);

CREATE TABLE sales_returns (
    id TEXT PRIMARY KEY NOT NULL,
    return_no TEXT NOT NULL UNIQUE,
    sale_id TEXT NOT NULL REFERENCES sales(id),
    refund_minor INTEGER NOT NULL CHECK (refund_minor >= 0),
    refund_method TEXT NOT NULL CHECK (refund_method IN ('cash', 'card')),
    reason TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_returns_sale ON sales_returns(sale_id);

CREATE TABLE sales_return_items (
    id TEXT PRIMARY KEY NOT NULL,
    return_id TEXT NOT NULL REFERENCES sales_returns(id),
    sale_item_id TEXT NOT NULL REFERENCES sale_items(id),
    qty_milli INTEGER NOT NULL CHECK (qty_milli > 0),
    base_qty_milli INTEGER NOT NULL CHECK (base_qty_milli > 0),
    refund_minor INTEGER NOT NULL,
    cost_minor INTEGER NOT NULL
);
CREATE INDEX idx_return_items_sale_item ON sales_return_items(sale_item_id);

-- سجل حركة النقدية/الكارت. جلسات الخزينة (session_id) تُضاف في المرحلة 4.
CREATE TABLE cash_transactions (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('sale', 'sale_return')),
    method TEXT NOT NULL CHECK (method IN ('cash', 'card')),
    amount_minor INTEGER NOT NULL,
    ref_type TEXT NOT NULL,
    ref_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_cash_tx_created ON cash_transactions(created_at);

CREATE TABLE suspended_sales (
    id TEXT PRIMARY KEY NOT NULL,
    label TEXT NOT NULL DEFAULT '',
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- حماية الفواتير والمرتجعات من التعديل والحذف على مستوى قاعدة البيانات.
CREATE TRIGGER sales_no_update BEFORE UPDATE ON sales BEGIN SELECT RAISE(ABORT, 'الفاتورة المكتملة لا تُعدَّل'); END;
CREATE TRIGGER sales_no_delete BEFORE DELETE ON sales BEGIN SELECT RAISE(ABORT, 'الفاتورة المكتملة لا تُحذف'); END;
CREATE TRIGGER sale_items_no_update BEFORE UPDATE ON sale_items BEGIN SELECT RAISE(ABORT, 'بنود الفاتورة لا تُعدَّل'); END;
CREATE TRIGGER sale_items_no_delete BEFORE DELETE ON sale_items BEGIN SELECT RAISE(ABORT, 'بنود الفاتورة لا تُحذف'); END;
CREATE TRIGGER payments_no_update BEFORE UPDATE ON payments BEGIN SELECT RAISE(ABORT, 'المدفوعات لا تُعدَّل'); END;
CREATE TRIGGER payments_no_delete BEFORE DELETE ON payments BEGIN SELECT RAISE(ABORT, 'المدفوعات لا تُحذف'); END;
CREATE TRIGGER returns_no_update BEFORE UPDATE ON sales_returns BEGIN SELECT RAISE(ABORT, 'المرتجع لا يُعدَّل'); END;
CREATE TRIGGER returns_no_delete BEFORE DELETE ON sales_returns BEGIN SELECT RAISE(ABORT, 'المرتجع لا يُحذف'); END;

INSERT INTO settings (setting_key, setting_value) VALUES
    ('tax_rate_bp', '0'),
    ('receipt_footer', 'شكرًا لزيارتكم'),
    ('receipt_paper_width', '80'),
    ('pos_shortcuts', '{"search":"F2","pay":"F4","suspend":"F8","resume":"F9","remove":"Delete"}');
