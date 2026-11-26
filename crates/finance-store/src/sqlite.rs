use std::path::Path;

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use finance_core::{
    ActivityEntry, ActivityKind, Backup, BackupActivity, BackupBudget, BackupCategory,
    BackupExpense, BackupIncome, BackupPayment, BackupRate, BackupSetting, Category, CategoryId,
    Expense, ExpenseId, Income, IncomeId, NewExpense, NewIncome, NewRecurringPayment,
    RecurringPayment, RecurringPaymentId, Report, build_income_report, build_report,
    category::validate_name,
};
use rusqlite::{Connection, params};
use rusqlite_migration::{M, Migrations};

use crate::{error::StoreError, mapping};

const MIGRATIONS: &[M<'_>] = &[
    M::up(include_str!("../migrations/0001_init.sql")),
    M::up(include_str!("../migrations/0002_recurring_payments.sql")),
    M::up(include_str!("../migrations/0003_expense_metadata.sql")),
    M::up(include_str!("../migrations/0004_income.sql")),
    M::up(include_str!("../migrations/0005_payment_details.sql")),
    M::up(include_str!(
        "../migrations/0006_budget_and_recurring_kind.sql"
    )),
    M::up(include_str!("../migrations/0007_movement_method.sql")),
    M::up(include_str!("../migrations/0008_activity_log.sql")),
    M::up(include_str!("../migrations/0009_original_amount.sql")),
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

    pub fn ensure_category(
        &self,
        name: &str,
        icon: Option<&str>,
        color: &str,
    ) -> Result<(), StoreError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR IGNORE INTO categories (name, icon, color, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![name.trim(), icon, color, now],
        )?;
        Ok(())
    }

    pub fn categories(&self) -> Result<Vec<Category>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, icon, color, is_active, sort_order
             FROM categories ORDER BY sort_order, name",
        )?;
        let rows = stmt.query_map([], mapping::category_from_row)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
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
                    evidence_path, rate_cents, rate_day, method, original_amount_cents, original_currency
             FROM expenses WHERE local_date BETWEEN ?1 AND ?2
             ORDER BY local_date DESC, id DESC",
        )?;
        let rows = stmt.query_map(
            params![from.to_string(), to.to_string()],
            mapping::expense_from_row,
        )?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
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
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn insert_income(&self, new: &NewIncome) -> Result<IncomeId, StoreError> {
        new.validate()?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO incomes
             (amount_cents, currency, category_id, note, received_at, local_date,
              rate_cents, rate_day, method, created_at, updated_at,
              original_amount_cents, original_currency)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                new.amount.cents(),
                new.amount.currency().as_str(),
                new.category_id.get(),
                new.note,
                new.received_at.to_rfc3339(),
                new.local_date.to_string(),
                new.rate_cents,
                new.rate_day,
                new.method.as_str(),
                now,
                now,
                new.original_amount_cents,
                new.original_currency,
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
                    rate_cents, rate_day, method, original_amount_cents, original_currency
             FROM incomes WHERE local_date BETWEEN ?1 AND ?2
             ORDER BY local_date DESC, id DESC",
        )?;
        let rows = stmt.query_map(
            params![from.to_string(), to.to_string()],
            mapping::income_from_row,
        )?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn delete_income(&self, id: IncomeId) -> Result<bool, StoreError> {
        let deleted = self
            .conn
            .execute("DELETE FROM incomes WHERE id = ?1", params![id.get()])?;
        Ok(deleted > 0)
    }

    pub fn update_expense(&self, expense: &Expense) -> Result<bool, StoreError> {
        let now = Utc::now().to_rfc3339();
        let updated = self.conn.execute(
            "UPDATE expenses SET amount_cents = ?2, currency = ?3, category_id = ?4,
                    note = ?5, spent_at = ?6, local_date = ?7, evidence_path = ?8,
                    rate_cents = ?9, rate_day = ?10, method = ?11, updated_at = ?12,
                    original_amount_cents = ?13, original_currency = ?14
             WHERE id = ?1",
            params![
                expense.id.get(),
                expense.amount.cents(),
                expense.amount.currency().as_str(),
                expense.category_id.get(),
                expense.note,
                expense.spent_at.to_rfc3339(),
                expense.local_date.to_string(),
                expense.evidence_path,
                expense.rate_cents,
                expense.rate_day,
                expense.method.as_str(),
                now,
                expense.original_amount_cents,
                expense.original_currency,
            ],
        )?;
        Ok(updated > 0)
    }

    pub fn update_income(&self, income: &Income) -> Result<bool, StoreError> {
        let now = Utc::now().to_rfc3339();
        let updated = self.conn.execute(
            "UPDATE incomes SET amount_cents = ?2, currency = ?3, category_id = ?4,
                    note = ?5, received_at = ?6, local_date = ?7,
                    rate_cents = ?8, rate_day = ?9, method = ?10, updated_at = ?11,
                    original_amount_cents = ?12, original_currency = ?13
             WHERE id = ?1",
            params![
                income.id.get(),
                income.amount.cents(),
                income.amount.currency().as_str(),
                income.category_id.get(),
                income.note,
                income.received_at.to_rfc3339(),
                income.local_date.to_string(),
                income.rate_cents,
                income.rate_day,
                income.method.as_str(),
                now,
                income.original_amount_cents,
                income.original_currency,
            ],
        )?;
        Ok(updated > 0)
    }

    pub fn budgets(&self) -> Result<Vec<(CategoryId, i64)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT category_id, limit_cents FROM budgets ORDER BY category_id")?;
        let rows = stmt.query_map([], |row| Ok((CategoryId::new(row.get(0)?), row.get(1)?)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn set_budget(&self, category_id: CategoryId, limit_cents: i64) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO budgets (category_id, limit_cents) VALUES (?1, ?2)
             ON CONFLICT(category_id) DO UPDATE SET limit_cents = excluded.limit_cents",
            params![category_id.get(), limit_cents],
        )?;
        Ok(())
    }

    pub fn remove_budget(&self, category_id: CategoryId) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM budgets WHERE category_id = ?1",
            params![category_id.get()],
        )?;
        Ok(())
    }

    /// Historial completo con origen, para distinguir tasa automática de manual.
    pub fn rate_details(&self) -> Result<Vec<(NaiveDate, i64, String, String)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT day, rate_cents, source, created_at FROM exchange_rates ORDER BY day DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let day: String = row.get(0)?;
            let day = NaiveDate::parse_from_str(&day, "%Y-%m-%d").map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok((day, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn insert_activity(
        &self,
        kind: ActivityKind,
        entity_id: i64,
        summary: &str,
        amount_cents: i64,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO activity_log (kind, entity_id, summary, amount_cents, occurred_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                kind.as_str(),
                entity_id,
                summary,
                amount_cents,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn activities(&self, limit: i64) -> Result<Vec<ActivityEntry>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, entity_id, summary, amount_cents, occurred_at
             FROM activity_log ORDER BY occurred_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            let kind: String = row.get(1)?;
            let occurred_at: String = row.get(5)?;
            let occurred_at = DateTime::parse_from_rfc3339(&occurred_at)
                .map(|moment| moment.with_timezone(&Utc))
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok(ActivityEntry {
                id: row.get(0)?,
                kind: ActivityKind::parse(&kind),
                entity_id: row.get(2)?,
                summary: row.get(3)?,
                amount_cents: row.get(4)?,
                occurred_at,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn export_backup(&self) -> Result<Backup, StoreError> {
        let mut backup = Backup::new(Utc::now().to_rfc3339());

        let mut stmt = self.conn.prepare(
            "SELECT id, name, icon, color, is_active, sort_order FROM categories ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupCategory {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                color: row.get(3)?,
                is_active: row.get::<_, i64>(4)? == 1,
                sort_order: row.get(5)?,
            })
        })?;
        backup.categories = rows.collect::<Result<Vec<_>, _>>()?;

        let mut stmt = self.conn.prepare(
            "SELECT id, name, icon, color, is_active, sort_order FROM income_categories ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupCategory {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                color: row.get(3)?,
                is_active: row.get::<_, i64>(4)? == 1,
                sort_order: row.get(5)?,
            })
        })?;
        backup.income_categories = rows.collect::<Result<Vec<_>, _>>()?;

        let mut stmt = self.conn.prepare(
            "SELECT id, amount_cents, currency, category_id, note, spent_at, local_date,
                    evidence_path, rate_cents, rate_day, method, original_amount_cents, original_currency
             FROM expenses ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupExpense {
                id: row.get(0)?,
                amount_cents: row.get(1)?,
                currency: row.get(2)?,
                category_id: row.get(3)?,
                note: row.get(4)?,
                spent_at: row.get(5)?,
                local_date: row.get(6)?,
                evidence_path: row.get(7)?,
                rate_cents: row.get(8)?,
                rate_day: row.get(9)?,
                method: row.get(10)?,
                original_amount_cents: row.get(11)?,
                original_currency: row.get(12)?,
            })
        })?;
        backup.expenses = rows.collect::<Result<Vec<_>, _>>()?;

        let mut stmt = self.conn.prepare(
            "SELECT id, amount_cents, currency, category_id, note, received_at, local_date,
                    rate_cents, rate_day, method, original_amount_cents, original_currency
             FROM incomes ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupIncome {
                id: row.get(0)?,
                amount_cents: row.get(1)?,
                currency: row.get(2)?,
                category_id: row.get(3)?,
                note: row.get(4)?,
                received_at: row.get(5)?,
                local_date: row.get(6)?,
                rate_cents: row.get(7)?,
                rate_day: row.get(8)?,
                method: row.get(9)?,
                original_amount_cents: row.get(10)?,
                original_currency: row.get(11)?,
            })
        })?;
        backup.incomes = rows.collect::<Result<Vec<_>, _>>()?;

        let mut stmt = self.conn.prepare(
            "SELECT id, name, amount_cents, currency, category_id, due_day, due_date,
                    payment_method, kind, note, last_paid_period, is_active
             FROM recurring_payments ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupPayment {
                id: row.get(0)?,
                name: row.get(1)?,
                amount_cents: row.get(2)?,
                currency: row.get(3)?,
                category_id: row.get(4)?,
                due_day: row.get(5)?,
                due_date: row.get(6)?,
                payment_method: row.get(7)?,
                kind: row.get(8)?,
                note: row.get(9)?,
                last_paid_period: row.get(10)?,
                is_active: row.get::<_, i64>(11)? == 1,
            })
        })?;
        backup.payments = rows.collect::<Result<Vec<_>, _>>()?;

        let mut stmt = self
            .conn
            .prepare("SELECT category_id, limit_cents FROM budgets ORDER BY category_id")?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupBudget {
                category_id: row.get(0)?,
                limit_cents: row.get(1)?,
            })
        })?;
        backup.budgets = rows.collect::<Result<Vec<_>, _>>()?;

        let mut stmt = self
            .conn
            .prepare("SELECT day, rate_cents, source FROM exchange_rates ORDER BY day")?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupRate {
                day: row.get(0)?,
                rate_cents: row.get(1)?,
                source: row.get(2)?,
            })
        })?;
        backup.rates = rows.collect::<Result<Vec<_>, _>>()?;

        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM settings ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupSetting {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })?;
        backup.settings = rows.collect::<Result<Vec<_>, _>>()?;

        let mut stmt = self.conn.prepare(
            "SELECT id, kind, entity_id, summary, amount_cents, occurred_at
             FROM activity_log ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(BackupActivity {
                id: row.get(0)?,
                kind: row.get(1)?,
                entity_id: row.get(2)?,
                summary: row.get(3)?,
                amount_cents: row.get(4)?,
                occurred_at: row.get(5)?,
            })
        })?;
        backup.activities = rows.collect::<Result<Vec<_>, _>>()?;

        Ok(backup)
    }

    /// Reemplaza TODOS los datos por los de la copia. Se ejecuta en una transacción:
    /// si algo falla, la base queda como estaba.
    pub fn import_backup(&self, backup: &Backup) -> Result<(), StoreError> {
        backup.validate().map_err(StoreError::Backup)?;
        let tx = self.conn.unchecked_transaction()?;
        for table in [
            "expenses",
            "incomes",
            "budgets",
            "recurring_payments",
            "exchange_rates",
            "settings",
            "categories",
            "income_categories",
            "activity_log",
        ] {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        }

        for category in &backup.categories {
            tx.execute(
                "INSERT INTO categories (id, name, icon, color, is_active, sort_order, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    category.id,
                    category.name,
                    category.icon,
                    category.color,
                    i64::from(category.is_active),
                    category.sort_order,
                    backup.exported_at,
                ],
            )?;
        }
        for category in &backup.income_categories {
            tx.execute(
                "INSERT INTO income_categories (id, name, icon, color, is_active, sort_order, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    category.id,
                    category.name,
                    category.icon,
                    category.color,
                    i64::from(category.is_active),
                    category.sort_order,
                    backup.exported_at,
                ],
            )?;
        }
        for expense in &backup.expenses {
            tx.execute(
                "INSERT INTO expenses (id, amount_cents, currency, category_id, note, spent_at,
                        local_date, evidence_path, rate_cents, rate_day, method, created_at, updated_at,
                        original_amount_cents, original_currency)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    expense.id,
                    expense.amount_cents,
                    expense.currency,
                    expense.category_id,
                    expense.note,
                    expense.spent_at,
                    expense.local_date,
                    expense.evidence_path,
                    expense.rate_cents,
                    expense.rate_day,
                    expense.method,
                    backup.exported_at,
                    backup.exported_at,
                    expense.original_amount_cents,
                    expense.original_currency,
                ],
            )?;
        }
        for income in &backup.incomes {
            tx.execute(
                "INSERT INTO incomes (id, amount_cents, currency, category_id, note, received_at,
                        local_date, rate_cents, rate_day, method, created_at, updated_at,
                        original_amount_cents, original_currency)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    income.id,
                    income.amount_cents,
                    income.currency,
                    income.category_id,
                    income.note,
                    income.received_at,
                    income.local_date,
                    income.rate_cents,
                    income.rate_day,
                    income.method,
                    backup.exported_at,
                    backup.exported_at,
                    income.original_amount_cents,
                    income.original_currency,
                ],
            )?;
        }
        for payment in &backup.payments {
            tx.execute(
                "INSERT INTO recurring_payments (id, name, amount_cents, currency, category_id,
                        due_day, due_date, payment_method, kind, note, last_paid_period, is_active,
                        created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    payment.id,
                    payment.name,
                    payment.amount_cents,
                    payment.currency,
                    payment.category_id,
                    payment.due_day,
                    payment.due_date,
                    payment.payment_method,
                    payment.kind,
                    payment.note,
                    payment.last_paid_period,
                    i64::from(payment.is_active),
                    backup.exported_at,
                    backup.exported_at,
                ],
            )?;
        }
        for budget in &backup.budgets {
            tx.execute(
                "INSERT INTO budgets (category_id, limit_cents) VALUES (?1, ?2)",
                params![budget.category_id, budget.limit_cents],
            )?;
        }
        for rate in &backup.rates {
            tx.execute(
                "INSERT INTO exchange_rates (day, rate_cents, source, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![rate.day, rate.rate_cents, rate.source, backup.exported_at],
            )?;
        }
        for setting in &backup.settings {
            tx.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                params![setting.key, setting.value],
            )?;
        }
        for activity in &backup.activities {
            tx.execute(
                "INSERT INTO activity_log
                    (id, kind, entity_id, summary, amount_cents, occurred_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    activity.id,
                    activity.kind,
                    activity.entity_id,
                    activity.summary,
                    activity.amount_cents,
                    activity.occurred_at,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
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

    pub fn report_between(&self, from: NaiveDate, to: NaiveDate) -> Result<Report, StoreError> {
        let expenses = self.expenses_between(from, to)?;
        let categories = self.categories()?;
        Ok(build_report(&expenses, &categories)?)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
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
            "SELECT id, kind, name, amount_cents, currency, category_id, due_day,
                    due_date, payment_method, note, last_paid_period, is_active
             FROM recurring_payments WHERE is_active = 1 ORDER BY due_date, due_day, name",
        )?;
        let rows = stmt.query_map([], mapping::recurring_from_row)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    pub fn add_recurring_payment(
        &self,
        new: &NewRecurringPayment,
    ) -> Result<RecurringPaymentId, StoreError> {
        new.validate()?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO recurring_payments
             (kind, name, amount_cents, currency, category_id, due_day, due_date,
              payment_method, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                new.kind.as_str(),
                new.name.trim(),
                new.amount.cents(),
                new.amount.currency().as_str(),
                new.category_id.get(),
                new.due_day,
                new.due_date.map(|day| day.to_string()),
                new.method.as_str(),
                new.note,
                now,
                now,
            ],
        )?;
        Ok(RecurringPaymentId::new(self.conn.last_insert_rowid()))
    }

    pub fn delete_recurring_payment(&self, id: RecurringPaymentId) -> Result<bool, StoreError> {
        let deleted = self.conn.execute(
            "DELETE FROM recurring_payments WHERE id = ?1",
            params![id.get()],
        )?;
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
        let period = format!(
            "{:04}-{:02}",
            expense.local_date.year(),
            expense.local_date.month()
        );
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
          evidence_path, rate_cents, rate_day, method, created_at, updated_at,
          original_amount_cents, original_currency)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
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
            new.method.as_str(),
            now,
            now,
            new.original_amount_cents,
            new.original_currency,
        ],
    )?;
    Ok(ExpenseId::new(conn.last_insert_rowid()))
}

#[cfg(test)]
mod migration_tests {
    use super::*;

    #[test]
    fn upgrades_database_created_before_budget_migration() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::from_slice(&MIGRATIONS[..5])
            .to_latest(&mut conn)
            .unwrap();
        conn.execute(
            "INSERT INTO categories (name, color, created_at) VALUES ('Hogar', '#fff', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO recurring_payments
             (name, amount_cents, currency, category_id, due_day, created_at, updated_at)
             VALUES ('Luz', 1500, 'USD', 1, 25, '2026-01-01', '2026-01-01')",
            [],
        )
        .unwrap();

        Migrations::from_slice(MIGRATIONS)
            .to_latest(&mut conn)
            .unwrap();

        let kind: String = conn
            .query_row(
                "SELECT kind FROM recurring_payments WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kind, "payment");
        let budgets_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'budgets')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(budgets_exists);
    }

    #[test]
    fn adds_method_columns_to_databases_created_before_them() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::from_slice(&MIGRATIONS[..6])
            .to_latest(&mut conn)
            .unwrap();
        conn.execute(
            "INSERT INTO categories (name, color, created_at) VALUES ('Comida', '#fff', '2026-01-01')",
            [],
        )
        .unwrap();

        Migrations::from_slice(MIGRATIONS)
            .to_latest(&mut conn)
            .unwrap();

        conn.execute(
            "INSERT INTO expenses (amount_cents, currency, category_id, spent_at, local_date, created_at, updated_at)
             VALUES (100, 'USD', 1, '2026-01-01T00:00:00Z', '2026-01-01', '2026-01-01', '2026-01-01')",
            [],
        )
        .unwrap();
        let saved: String = conn
            .query_row("SELECT method FROM expenses WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(saved, "other");
    }
}
