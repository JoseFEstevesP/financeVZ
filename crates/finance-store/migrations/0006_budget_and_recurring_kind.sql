CREATE TABLE IF NOT EXISTS budgets (
    category_id INTEGER PRIMARY KEY REFERENCES categories(id),
    limit_cents INTEGER NOT NULL CHECK (limit_cents > 0)
);

ALTER TABLE recurring_payments ADD COLUMN kind TEXT NOT NULL DEFAULT 'payment';
