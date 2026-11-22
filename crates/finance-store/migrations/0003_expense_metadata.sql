CREATE TABLE IF NOT EXISTS exchange_rates (
    day        TEXT NOT NULL PRIMARY KEY,
    rate_cents INTEGER NOT NULL CHECK (rate_cents > 0),
    source     TEXT NOT NULL DEFAULT 'manual',
    created_at TEXT NOT NULL
);

ALTER TABLE expenses ADD COLUMN evidence_path TEXT;
ALTER TABLE expenses ADD COLUMN rate_cents INTEGER;
ALTER TABLE expenses ADD COLUMN rate_day TEXT;
