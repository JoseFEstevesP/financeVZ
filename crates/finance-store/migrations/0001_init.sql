CREATE TABLE IF NOT EXISTS categories (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL UNIQUE,
    icon       TEXT,
    color      TEXT NOT NULL DEFAULT '#888888',
    is_active  INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS expenses (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    currency     TEXT NOT NULL,
    category_id  INTEGER NOT NULL REFERENCES categories(id),
    note         TEXT,
    spent_at      TEXT NOT NULL,
    local_date   TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_expenses_local_date ON expenses(local_date);
CREATE INDEX IF NOT EXISTS idx_expenses_category_date ON expenses(category_id, local_date);
