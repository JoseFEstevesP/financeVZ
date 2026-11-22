CREATE TABLE IF NOT EXISTS income_categories (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL UNIQUE,
    icon       TEXT,
    color      TEXT NOT NULL DEFAULT '#888888',
    is_active  INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS incomes (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    currency     TEXT NOT NULL,
    category_id  INTEGER NOT NULL REFERENCES income_categories(id),
    note         TEXT,
    received_at  TEXT NOT NULL,
    local_date   TEXT NOT NULL,
    rate_cents   INTEGER,
    rate_day     TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_incomes_local_date ON incomes(local_date);
CREATE INDEX IF NOT EXISTS idx_incomes_category_date ON incomes(category_id, local_date);
