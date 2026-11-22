use std::path::Path;

use chrono::{Datelike, NaiveDate, Utc};
use finance_core::{
    build_income_report, build_report, category::validate_name, Category, CategoryId, Expense,
    ExpenseId, Income, IncomeId, NewExpense, NewIncome, NewRecurringPayment, RecurringPayment,
    RecurringPaymentId, Report,
};
use rusqlite::{params, Connection};
use rusqlite_migration::{M, Migrations};

use crate::{error::StoreError, mapping};

const MIGRATIONS: &[M<'_>] = &[
    M::up(include_str!("../migrations/0001_init.sql")),
    M::up(include_str!("../migrations/0002_recurring_payments.sql")),
    M::up(include_str!("../migrations/0003_expense_metadata.sql")),
    M::up(include_str!("../migrations/0004_income.sql")),
];

pub struct SqliteRepository {
    conn: Connection,
}

impl SqliteRepository {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut conn: Connection) -> Result<Self, StoreError> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Migrations::from_slice(MIGRATIONS).to_latest(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn add_category(
        &self,
        name: &str,
        icon: Option<&str>,
        color: &str,
    ) -> Result<CategoryId, StoreError> {
        validate_name(name)?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO categories (name, icon, color, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![name.trim(), icon, color, now],
        )?;
        Ok(CategoryId::new(self.conn.last_insert_rowid()))
    }

    pub fn categories(&self) -> Result<Vec<Category>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, icon, color, is_active, sort_order
             FROM categories ORDER BY sort_order, name",
        )?;
        let rows = stmt.query_map([], mapping::category_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
    }

    pub fn insert_expense(&self, new: &NewExpense) -> Result<ExpenseId, StoreError> {
        insert_expense_on(&self.conn, new)
    }

    pub fn expenses_between(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<Expense>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, amount_cents, currency, category_id, note, spent_at, local_date,
                    evidence_path, rate_cents, rate_day
             FROM expenses WHERE local_date BETWEEN ?1 AND ?2
             ORDER BY local_date DESC, id DESC",
        )?;
        let rows = stmt.query_map(params![from.to_string(), to.to_string()], mapping::expense_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
    }

    pub fn delete_expense(&self, id: ExpenseId) -> Result<bool, StoreError> {
        let deleted = self
            .conn
            .execute("DELETE FROM expenses WHERE id = ?1", params![id.get()])?;
        Ok(deleted > 0)
    }

    pub fn add_income_category(
        &self,
        name: &str,
        icon: Option<&str>,
        color: &str,
    ) -> Result<CategoryId, StoreError> {
        validate_name(name)?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO income_categories (name, icon, color, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![name.trim(), icon, color, now],
        )?;
        Ok(CategoryId::new(self.conn.last_insert_rowid()))
    }

    pub fn income_categories(&self) -> Result<Vec<Category>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, icon, color, is_active, sort_order
             FROM income_categories ORDER BY sort_order, name",
        )?;
        let rows = stmt.query_map([], mapping::category_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
    }

    pub fn insert_income(&self, new: &NewIncome) -> Result<IncomeId, StoreError> {
        new.validate()?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO incomes
             (amount_cents, currency, category_id, note, received_at, local_date,
              rate_cents, rate_day, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                new.amount.cents(),
                new.amount.currency().as_str(),
                new.category_id.get(),
                new.note,
                new.received_at.to_rfc3339(),
                new.local_date.to_string(),
                new.rate_cents,
                new.rate_day,
                now,
                now,
            ],
        )?;
        Ok(IncomeId::new(self.conn.last_insert_rowid()))
    }

    pub fn incomes_between(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<Income>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, amount_cents, currency, category_id, note, received_at, local_date,
                    rate_cents, rate_day
             FROM incomes WHERE local_date BETWEEN ?1 AND ?2
             ORDER BY local_date DESC, id DESC",
        )?;
        let rows = stmt.query_map(params![from.to_string(), to.to_string()], mapping::income_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
    }

    pub fn delete_income(&self, id: IncomeId) -> Result<bool, StoreError> {
        let deleted = self
            .conn
            .execute("DELETE FROM incomes WHERE id = ?1", params![id.get()])?;
        Ok(deleted > 0)
    }

    pub fn income_report_between(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Report, StoreError> {
        let incomes = self.incomes_between(from, to)?;
        let categories = self.income_categories()?;
        Ok(build_income_report(&incomes, &categories)?)
    }

    pub fn report_between(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Report, StoreError> {
        let expenses = self.expenses_between(from, to)?;
        let categories = self.categories()?;
        Ok(build_report(&expenses, &categories)?)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let value = stmt.query_row(params![key], |row| row.get::<_, String>(0));
        match value {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn set_rate_for_day(
        &self,
        day: NaiveDate,
        rate_cents: i64,
        source: &str,
    ) -> Result<(), StoreError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO exchange_rates (day, rate_cents, source, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(day) DO UPDATE SET rate_cents = excluded.rate_cents,
                                           source = excluded.source",
            params![day.to_string(), rate_cents, source, now],
        )?;
        Ok(())
    }

    /// Última tasa conocida en o antes de `day`.
    pub fn rate_on(&self, day: NaiveDate) -> Result<Option<(NaiveDate, i64)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT day, rate_cents FROM exchange_rates WHERE day <= ?1 ORDER BY day DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![day.to_string()], |row| {
            let day: String = row.get(0)?;
            let rate: i64 = row.get(1)?;
            Ok((day, rate))
        })?;
        match rows.next() {
            Some(value) => {
                let (day, rate) = value?;
                Ok(NaiveDate::parse_from_str(&day, "%Y-%m-%d")
                    .ok()
                    .map(|day| (day, rate)))
            }
            None => Ok(None),
        }
    }

    pub fn all_rates(&self) -> Result<Vec<(NaiveDate, i64)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT day, rate_cents FROM exchange_rates ORDER BY day")?;
        let rows = stmt.query_map([], |row| {
            let day: String = row.get(0)?;
            let rate: i64 = row.get(1)?;
            Ok((day, rate))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (day, rate) = row?;
            if let Ok(day) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") {
                out.push((day, rate));
            }
        }
        Ok(out)
    }

    pub fn recurring_payments(&self) -> Result<Vec<RecurringPayment>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, amount_cents, currency, category_id, due_day,
                    note, last_paid_period, is_active
             FROM recurring_payments WHERE is_active = 1 ORDER BY due_day, name",
        )?;
        let rows = stmt.query_map([], mapping::recurring_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
    }

    pub fn add_recurring_payment(
        &self,
        new: &NewRecurringPayment,
    ) -> Result<RecurringPaymentId, StoreError> {
        new.validate()?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO recurring_payments
             (name, amount_cents, currency, category_id, due_day, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                new.name.trim(),
                new.amount.cents(),
                new.amount.currency().as_str(),
                new.category_id.get(),
                new.due_day,
                new.note,
                now,
                now,
            ],
        )?;
        Ok(RecurringPaymentId::new(self.conn.last_insert_rowid()))
    }

    pub fn delete_recurring_payment(&self, id: RecurringPaymentId) -> Result<bool, StoreError> {
        let deleted = self
            .conn
            .execute("DELETE FROM recurring_payments WHERE id = ?1", params![id.get()])?;
        Ok(deleted > 0)
    }

    pub fn pay_recurring_payment(
        &self,
        id: RecurringPaymentId,
        expense: &NewExpense,
    ) -> Result<(), StoreError> {
        expense.validate()?;
        let tx = self.conn.unchecked_transaction()?;
        insert_expense_on(&tx, expense)?;
        let period = format!("{:04}-{:02}", expense.local_date.year(), expense.local_date.month());
        let now = Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE recurring_payments
             SET last_paid_period = ?2, updated_at = ?3 WHERE id = ?1",
            params![id.get(), period, now],
        )?;
        tx.commit()?;
        Ok(())
    }
}

fn insert_expense_on(conn: &Connection, new: &NewExpense) -> Result<ExpenseId, StoreError> {
    new.validate()?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO expenses
         (amount_cents, currency, category_id, note, spent_at, local_date,
          evidence_path, rate_cents, rate_day, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            new.amount.cents(),
            new.amount.currency().as_str(),
            new.category_id.get(),
            new.note,
            new.spent_at.to_rfc3339(),
            new.local_date.to_string(),
            new.evidence_path,
            new.rate_cents,
            new.rate_day,
            now,
            now,
        ],
    )?;
    Ok(ExpenseId::new(conn.last_insert_rowid()))
}
