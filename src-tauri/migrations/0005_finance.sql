-- المرحلة 4: المالية. ورديات الخزينة، المصروفات، العملاء والبيع الآجل.
-- كل المبالغ بالقروش.

-- ---------- العملاء ----------
CREATE TABLE customers (
    id TEXT PRIMARY KEY NOT NULL,
    name_ar TEXT NOT NULL,
    name_key TEXT NOT NULL UNIQUE,
    phone TEXT NOT NULL DEFAULT '',
    address TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    -- 0 = بلا حد ائتماني
    credit_limit_minor INTEGER NOT NULL DEFAULT 0 CHECK (credit_limit_minor >= 0),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- دفعات العملاء لسداد الآجل (على الحساب). لا تُعدَّل ولا تُحذف.
CREATE TABLE customer_payments (
    id TEXT PRIMARY KEY NOT NULL,
    customer_id TEXT NOT NULL REFERENCES customers(id),
    method TEXT NOT NULL CHECK (method IN ('cash', 'card')),
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
CREATE INDEX idx_customer_payments_customer ON customer_payments(customer_id, created_at);

-- الفاتورة تُربط بعميل (للآجل). ALTER لا يطلق triggers الحماية ولا يغيّر الفواتير القديمة.
ALTER TABLE sales ADD COLUMN customer_id TEXT REFERENCES customers(id);
CREATE INDEX idx_sales_customer ON sales(customer_id);
-- الجزء من قيمة المرتجع الذي خُصم من مديونية العميل بدل ردّه نقدًا.
ALTER TABLE sales_returns ADD COLUMN credit_minor INTEGER NOT NULL DEFAULT 0;

-- ---------- ورديات الخزينة ----------
CREATE TABLE cash_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'closed')),
    opening_minor INTEGER NOT NULL CHECK (opening_minor >= 0),
    opened_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    closed_at TEXT,
    expected_cash_minor INTEGER,
    counted_cash_minor INTEGER,
    difference_minor INTEGER,
    note TEXT NOT NULL DEFAULT ''
);
-- وردية مفتوحة واحدة فقط في أي وقت.
CREATE UNIQUE INDEX uq_one_open_session ON cash_sessions(status) WHERE status = 'open';

-- إدخال/إخراج نقدية يدوي من الدرج (سبب إلزامي). لا يُعدَّل ولا يُحذف.
CREATE TABLE cash_entries (
    id TEXT PRIMARY KEY NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('in', 'out')),
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- ---------- المصروفات ----------
CREATE TABLE expense_categories (
    id TEXT PRIMARY KEY NOT NULL,
    name_ar TEXT NOT NULL,
    name_key TEXT NOT NULL UNIQUE,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1))
);
INSERT INTO expense_categories (id, name_ar, name_key) VALUES
    ('e1a00000-0000-4000-8000-000000000001', 'إيجار', 'ايجار'),
    ('e1a00000-0000-4000-8000-000000000002', 'كهرباء ومياه', 'كهرباء ومياه'),
    ('e1a00000-0000-4000-8000-000000000003', 'رواتب وأجور', 'رواتب واجور'),
    ('e1a00000-0000-4000-8000-000000000004', 'مواصلات ونقل', 'مواصلات ونقل'),
    ('e1a00000-0000-4000-8000-000000000005', 'صيانة', 'صيانه'),
    ('e1a00000-0000-4000-8000-000000000006', 'أخرى', 'اخري');

-- المصروف لا يُحذف ولا يُعدَّل؛ الإلغاء يملأ بيانات الإلغاء مرة واحدة ويُسجَّل قيد عكسي في الخزينة.
CREATE TABLE expenses (
    id TEXT PRIMARY KEY NOT NULL,
    category_id TEXT NOT NULL REFERENCES expense_categories(id),
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    method TEXT NOT NULL CHECK (method IN ('cash', 'card')),
    description TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    voided_at TEXT,
    void_reason TEXT
);
CREATE INDEX idx_expenses_created ON expenses(created_at);

-- ---------- إعادة بناء cash_transactions: ربط بالوردية وأنواع جديدة ----------
CREATE TABLE cash_transactions_new (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('sale', 'sale_return', 'supplier_payment', 'expense', 'expense_void',
                                       'customer_payment', 'cash_in', 'cash_out')),
    method TEXT NOT NULL CHECK (method IN ('cash', 'card')),
    amount_minor INTEGER NOT NULL,
    ref_type TEXT NOT NULL,
    ref_id TEXT NOT NULL,
    session_id TEXT REFERENCES cash_sessions(id),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
INSERT INTO cash_transactions_new (id, kind, method, amount_minor, ref_type, ref_id, created_at)
SELECT id, kind, method, amount_minor, ref_type, ref_id, created_at FROM cash_transactions;
DROP TABLE cash_transactions;
ALTER TABLE cash_transactions_new RENAME TO cash_transactions;
CREATE INDEX idx_cash_tx_created ON cash_transactions(created_at);
CREATE INDEX idx_cash_tx_session ON cash_transactions(session_id);

-- ---------- الحماية ----------
CREATE TRIGGER customer_payments_no_update BEFORE UPDATE ON customer_payments BEGIN SELECT RAISE(ABORT, 'دفعة العميل لا تُعدَّل'); END;
CREATE TRIGGER customer_payments_no_delete BEFORE DELETE ON customer_payments BEGIN SELECT RAISE(ABORT, 'دفعة العميل لا تُحذف'); END;
CREATE TRIGGER cash_entries_no_update BEFORE UPDATE ON cash_entries BEGIN SELECT RAISE(ABORT, 'قيد الخزينة لا يُعدَّل'); END;
CREATE TRIGGER cash_entries_no_delete BEFORE DELETE ON cash_entries BEGIN SELECT RAISE(ABORT, 'قيد الخزينة لا يُحذف'); END;
CREATE TRIGGER cash_sessions_no_delete BEFORE DELETE ON cash_sessions BEGIN SELECT RAISE(ABORT, 'الوردية لا تُحذف'); END;
CREATE TRIGGER cash_sessions_closed_frozen BEFORE UPDATE ON cash_sessions WHEN OLD.status = 'closed'
BEGIN SELECT RAISE(ABORT, 'الوردية المغلقة لا تُعدَّل'); END;
CREATE TRIGGER cash_tx_no_update BEFORE UPDATE ON cash_transactions BEGIN SELECT RAISE(ABORT, 'حركة الخزينة لا تُعدَّل'); END;
CREATE TRIGGER cash_tx_no_delete BEFORE DELETE ON cash_transactions BEGIN SELECT RAISE(ABORT, 'حركة الخزينة لا تُحذف'); END;
CREATE TRIGGER expenses_no_delete BEFORE DELETE ON expenses BEGIN SELECT RAISE(ABORT, 'المصروف لا يُحذف'); END;
CREATE TRIGGER expenses_frozen_update BEFORE UPDATE ON expenses
WHEN OLD.voided_at IS NOT NULL OR NEW.amount_minor <> OLD.amount_minor OR NEW.method <> OLD.method
     OR NEW.category_id <> OLD.category_id OR NEW.description <> OLD.description OR NEW.created_at <> OLD.created_at
BEGIN SELECT RAISE(ABORT, 'المصروف لا يُعدَّل. يمكن إلغاؤه فقط'); END;

INSERT INTO settings (setting_key, setting_value) VALUES ('require_cash_session', '0');
