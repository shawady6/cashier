-- المرحلة 3: المخازن والنقل والجرد + الموردون والمشتريات والمرتجعات والمدفوعات.
-- ملاحظة: SQLite لا يعدّل قيود CHECK، فنعيد بناء الجدولين مع نسخ كل بياناتهما كما هي.

-- ---------- إعادة بناء stock_movements بأنواع حركات جديدة ----------
CREATE TABLE stock_movements_new (
    id TEXT PRIMARY KEY NOT NULL,
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    movement_type TEXT NOT NULL CHECK (movement_type IN
        ('adjustment', 'sale', 'sale_return', 'purchase', 'purchase_return', 'transfer_out', 'transfer_in', 'count')),
    qty_delta_milli INTEGER NOT NULL,
    value_delta_minor INTEGER NOT NULL,
    ref_type TEXT,
    ref_id TEXT,
    note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
INSERT INTO stock_movements_new
    (id, warehouse_id, product_id, variant_id, movement_type, qty_delta_milli, value_delta_minor, ref_type, ref_id, note, created_at)
SELECT id, warehouse_id, product_id, variant_id, movement_type, qty_delta_milli, value_delta_minor, ref_type, ref_id, note, created_at
FROM stock_movements;
DROP TABLE stock_movements;
ALTER TABLE stock_movements_new RENAME TO stock_movements;
CREATE INDEX idx_movements_product ON stock_movements(product_id, created_at);
CREATE INDEX idx_movements_ref ON stock_movements(ref_type, ref_id);
CREATE INDEX idx_movements_warehouse ON stock_movements(warehouse_id, created_at);

-- ---------- إعادة بناء cash_transactions لتشمل مدفوعات الموردين ----------
CREATE TABLE cash_transactions_new (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('sale', 'sale_return', 'supplier_payment')),
    method TEXT NOT NULL CHECK (method IN ('cash', 'card')),
    amount_minor INTEGER NOT NULL,
    ref_type TEXT NOT NULL,
    ref_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
INSERT INTO cash_transactions_new (id, kind, method, amount_minor, ref_type, ref_id, created_at)
SELECT id, kind, method, amount_minor, ref_type, ref_id, created_at FROM cash_transactions;
DROP TABLE cash_transactions;
ALTER TABLE cash_transactions_new RENAME TO cash_transactions;
CREATE INDEX idx_cash_tx_created ON cash_transactions(created_at);

INSERT INTO sequences (name, next_value) VALUES ('purchase', 1), ('purchase_return', 1), ('transfer', 1), ('count', 1);

-- ---------- الموردون ----------
CREATE TABLE suppliers (
    id TEXT PRIMARY KEY NOT NULL,
    name_ar TEXT NOT NULL,
    name_key TEXT NOT NULL UNIQUE,
    phone TEXT NOT NULL DEFAULT '',
    address TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- ---------- فواتير الشراء: المخزون يزيد فقط عند الاستلام (status = received) ----------
CREATE TABLE purchases (
    id TEXT PRIMARY KEY NOT NULL,
    purchase_no TEXT NOT NULL UNIQUE,
    supplier_id TEXT NOT NULL REFERENCES suppliers(id),
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'received', 'cancelled')),
    supplier_invoice_no TEXT NOT NULL DEFAULT '',
    note TEXT NOT NULL DEFAULT '',
    subtotal_minor INTEGER NOT NULL DEFAULT 0,
    discount_minor INTEGER NOT NULL DEFAULT 0,
    total_minor INTEGER NOT NULL DEFAULT 0,
    received_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_purchases_supplier ON purchases(supplier_id, created_at);

-- net_minor = قيمة البند بعد توزيع خصم الفاتورة (هي قيمة دخول المخزون).
CREATE TABLE purchase_items (
    id TEXT PRIMARY KEY NOT NULL,
    purchase_id TEXT NOT NULL REFERENCES purchases(id),
    line_no INTEGER NOT NULL,
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    name_ar TEXT NOT NULL,
    variant_name TEXT,
    unit_id TEXT NOT NULL REFERENCES units(id),
    unit_name TEXT NOT NULL,
    unit_factor_micro INTEGER NOT NULL CHECK (unit_factor_micro > 0),
    qty_milli INTEGER NOT NULL CHECK (qty_milli > 0),
    base_qty_milli INTEGER NOT NULL CHECK (base_qty_milli > 0),
    unit_cost_minor INTEGER NOT NULL CHECK (unit_cost_minor >= 0),
    line_total_minor INTEGER NOT NULL,
    net_minor INTEGER NOT NULL
);
CREATE INDEX idx_purchase_items_purchase ON purchase_items(purchase_id);

CREATE TABLE purchase_payments (
    id TEXT PRIMARY KEY NOT NULL,
    purchase_id TEXT NOT NULL REFERENCES purchases(id),
    supplier_id TEXT NOT NULL REFERENCES suppliers(id),
    method TEXT NOT NULL CHECK (method IN ('cash', 'card')),
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_purchase_payments_purchase ON purchase_payments(purchase_id);
CREATE INDEX idx_purchase_payments_supplier ON purchase_payments(supplier_id);

CREATE TABLE purchase_returns (
    id TEXT PRIMARY KEY NOT NULL,
    return_no TEXT NOT NULL UNIQUE,
    purchase_id TEXT NOT NULL REFERENCES purchases(id),
    supplier_id TEXT NOT NULL REFERENCES suppliers(id),
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    total_minor INTEGER NOT NULL CHECK (total_minor >= 0),
    note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_purchase_returns_purchase ON purchase_returns(purchase_id);

CREATE TABLE purchase_return_items (
    id TEXT PRIMARY KEY NOT NULL,
    return_id TEXT NOT NULL REFERENCES purchase_returns(id),
    purchase_item_id TEXT NOT NULL REFERENCES purchase_items(id),
    qty_milli INTEGER NOT NULL CHECK (qty_milli > 0),
    base_qty_milli INTEGER NOT NULL CHECK (base_qty_milli > 0),
    credit_minor INTEGER NOT NULL
);
CREATE INDEX idx_purchase_return_items_item ON purchase_return_items(purchase_item_id);

-- ---------- النقل بين المخازن (فوري) ----------
CREATE TABLE stock_transfers (
    id TEXT PRIMARY KEY NOT NULL,
    transfer_no TEXT NOT NULL UNIQUE,
    from_warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    to_warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    CHECK (from_warehouse_id <> to_warehouse_id)
);
CREATE TABLE stock_transfer_items (
    id TEXT PRIMARY KEY NOT NULL,
    transfer_id TEXT NOT NULL REFERENCES stock_transfers(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    qty_milli INTEGER NOT NULL CHECK (qty_milli > 0),
    value_minor INTEGER NOT NULL
);
CREATE INDEX idx_transfer_items_transfer ON stock_transfer_items(transfer_id);

-- ---------- الجرد ----------
-- system_qty_milli = لقطة الرصيد وقت بدء الجرد. الفرق يُطبَّق على الرصيد الحالي عند الاعتماد
-- فلا تضيع المبيعات التي حدثت أثناء الجرد.
CREATE TABLE stock_counts (
    id TEXT PRIMARY KEY NOT NULL,
    count_no TEXT NOT NULL UNIQUE,
    warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'finalized', 'cancelled')),
    note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    finalized_at TEXT
);
CREATE TABLE stock_count_items (
    id TEXT PRIMARY KEY NOT NULL,
    count_id TEXT NOT NULL REFERENCES stock_counts(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    variant_id TEXT REFERENCES product_variants(id),
    name_ar TEXT NOT NULL,
    variant_name TEXT,
    system_qty_milli INTEGER NOT NULL,
    counted_qty_milli INTEGER
);
CREATE INDEX idx_count_items_count ON stock_count_items(count_id);

-- ---------- حماية السجلات المعتمدة ----------
CREATE TRIGGER purchases_frozen_update BEFORE UPDATE ON purchases WHEN OLD.status <> 'draft'
    BEGIN SELECT RAISE(ABORT, 'فاتورة الشراء المستلمة أو الملغاة لا تُعدَّل'); END;
CREATE TRIGGER purchases_frozen_delete BEFORE DELETE ON purchases WHEN OLD.status <> 'draft'
    BEGIN SELECT RAISE(ABORT, 'فاتورة الشراء المستلمة أو الملغاة لا تُحذف'); END;
CREATE TRIGGER purchase_items_frozen_insert BEFORE INSERT ON purchase_items
    WHEN (SELECT status FROM purchases WHERE id = NEW.purchase_id) <> 'draft'
    BEGIN SELECT RAISE(ABORT, 'لا يمكن تعديل بنود فاتورة شراء معتمدة'); END;
CREATE TRIGGER purchase_items_frozen_update BEFORE UPDATE ON purchase_items
    WHEN (SELECT status FROM purchases WHERE id = OLD.purchase_id) <> 'draft'
    BEGIN SELECT RAISE(ABORT, 'لا يمكن تعديل بنود فاتورة شراء معتمدة'); END;
CREATE TRIGGER purchase_items_frozen_delete BEFORE DELETE ON purchase_items
    WHEN (SELECT status FROM purchases WHERE id = OLD.purchase_id) <> 'draft'
    BEGIN SELECT RAISE(ABORT, 'لا يمكن حذف بنود فاتورة شراء معتمدة'); END;
CREATE TRIGGER purchase_payments_no_update BEFORE UPDATE ON purchase_payments BEGIN SELECT RAISE(ABORT, 'دفعة المورد لا تُعدَّل'); END;
CREATE TRIGGER purchase_payments_no_delete BEFORE DELETE ON purchase_payments BEGIN SELECT RAISE(ABORT, 'دفعة المورد لا تُحذف'); END;
CREATE TRIGGER purchase_returns_no_update BEFORE UPDATE ON purchase_returns BEGIN SELECT RAISE(ABORT, 'مرتجع الشراء لا يُعدَّل'); END;
CREATE TRIGGER purchase_returns_no_delete BEFORE DELETE ON purchase_returns BEGIN SELECT RAISE(ABORT, 'مرتجع الشراء لا يُحذف'); END;
CREATE TRIGGER transfers_no_update BEFORE UPDATE ON stock_transfers BEGIN SELECT RAISE(ABORT, 'التحويل المخزني لا يُعدَّل'); END;
CREATE TRIGGER transfers_no_delete BEFORE DELETE ON stock_transfers BEGIN SELECT RAISE(ABORT, 'التحويل المخزني لا يُحذف'); END;
CREATE TRIGGER count_items_frozen_update BEFORE UPDATE ON stock_count_items
    WHEN (SELECT status FROM stock_counts WHERE id = OLD.count_id) <> 'open'
    BEGIN SELECT RAISE(ABORT, 'الجرد المعتمد لا يُعدَّل'); END;
CREATE TRIGGER counts_frozen_update BEFORE UPDATE ON stock_counts WHEN OLD.status <> 'open'
    BEGIN SELECT RAISE(ABORT, 'الجرد المعتمد لا يُعدَّل'); END;
