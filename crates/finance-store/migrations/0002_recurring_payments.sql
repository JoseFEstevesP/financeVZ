CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS recurring_payments (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    name             TEXT NOT NULL,
    amount_cents     INTEGER NOT NULL CHECK (amount_cents > 0),
    currency         TEXT NOT NULL,
    category_id      INTEGER NOT NULL REFERENCES categories(id),
    due_day          INTEGER NOT NULL CHECK (due_day BETWEEN 1 AND 31),
    note             TEXT,
    last_paid_period TEXT,
    is_active        INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_recurring_payments_active ON recurring_payments(is_active);
