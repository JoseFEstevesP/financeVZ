ALTER TABLE recurring_payments ADD COLUMN payment_method TEXT NOT NULL DEFAULT 'other';
ALTER TABLE recurring_payments ADD COLUMN due_date TEXT;
