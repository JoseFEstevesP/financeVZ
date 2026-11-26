ALTER TABLE expenses ADD COLUMN original_amount_cents INTEGER;
ALTER TABLE expenses ADD COLUMN original_currency TEXT;
ALTER TABLE incomes ADD COLUMN original_amount_cents INTEGER;
ALTER TABLE incomes ADD COLUMN original_currency TEXT;
