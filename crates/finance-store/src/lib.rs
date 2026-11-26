mod error;
mod mapping;
mod sqlite;

pub use error::StoreError;
pub use sqlite::SqliteRepository;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, NaiveDate};
    use finance_core::{
        ActivityKind, Backup, Currency, Money, MovementMethod, NewExpense, NewIncome,
        NewRecurringPayment, PaymentMethod, PaymentStatus,
    };

    #[test]
    fn persists_categories_expenses_and_report() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        let category_id = repo
            .add_category("Comida", Some("utensils"), "#ff0000")
            .unwrap();

        let categories = repo.categories().unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].name, "Comida");
        assert!(categories[0].is_active);

        let spent_at = DateTime::parse_from_rfc3339("2026-11-21T12:30:00-04:00").unwrap();
        let new = NewExpense {
            amount: Money::new(1250, Currency::new("USD").unwrap()).unwrap(),
            category_id,
            note: Some("almuerzo".into()),
            spent_at,
            local_date: NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
            evidence_path: None,
            rate_cents: None,
            rate_day: None,
            method: MovementMethod::Other,
            original_amount_cents: None,
            original_currency: None,
        };
        let expense_id = repo.insert_expense(&new).unwrap();

        let from = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 11, 30).unwrap();
        let expenses = repo.expenses_between(from, to).unwrap();
        assert_eq!(expenses.len(), 1);
        assert_eq!(expenses[0].id, expense_id);
        assert_eq!(expenses[0].amount.cents(), 1250);
        assert_eq!(expenses[0].note.as_deref(), Some("almuerzo"));
        assert_eq!(expenses[0].method, MovementMethod::Other);

        let report = repo.report_between(from, to).unwrap();
        assert_eq!(report.total.cents(), 1250);
        assert_eq!(report.by_category.len(), 1);
        assert_eq!(report.by_month.len(), 1);

        assert!(repo.delete_expense(expense_id).unwrap());
        assert!(repo.expenses_between(from, to).unwrap().is_empty());
    }

    #[test]
    fn persists_income_categories_and_incomes() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        let category_id = repo
            .add_income_category("Sueldo", Some("💼"), "#2dd4a7")
            .unwrap();

        let categories = repo.income_categories().unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].name, "Sueldo");

        let day = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let new = NewIncome {
            amount: Money::new(250000, Currency::new("USD").unwrap()).unwrap(),
            category_id,
            note: Some("quincena".into()),
            received_at: DateTime::parse_from_rfc3339("2026-11-20T09:00:00-04:00").unwrap(),
            local_date: day,
            rate_cents: Some(85789),
            rate_day: Some(day.to_string()),
            method: MovementMethod::Other,
            original_amount_cents: None,
            original_currency: None,
        };
        let id = repo.insert_income(&new).unwrap();
        let incomes = repo.incomes_between(day, day).unwrap();
        assert_eq!(incomes.len(), 1);
        assert_eq!(incomes[0].id, id);
        assert_eq!(incomes[0].amount.cents(), 250000);
        assert_eq!(incomes[0].rate_cents, Some(85789));

        let report = repo.income_report_between(day, day).unwrap();
        assert_eq!(report.total.cents(), 250000);
        assert_eq!(report.by_category.len(), 1);

        assert!(repo.delete_income(id).unwrap());
        assert!(repo.incomes_between(day, day).unwrap().is_empty());
    }

    #[test]
    fn persists_settings_and_recurring_payments() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        let category_id = repo
            .add_category("Hogar", Some("house"), "#ffffff")
            .unwrap();

        assert_eq!(repo.get_setting("rate").unwrap(), None);
        repo.set_setting("rate", "85789").unwrap();
        assert_eq!(repo.get_setting("rate").unwrap(), Some("85789".into()));

        let payment = NewRecurringPayment {
            kind: finance_core::RecurringKind::Payment,
            name: "Luz".into(),
            amount: Money::new(1500, Currency::new("USD").unwrap()).unwrap(),
            category_id,
            due_day: 25,
            due_date: None,
            method: PaymentMethod::Cashea,
            note: Some("Cuota 1/6".into()),
        };
        let id = repo.add_recurring_payment(&payment).unwrap();
        let payments = repo.recurring_payments().unwrap();
        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].due_day, 25);
        assert_eq!(payments[0].method, PaymentMethod::Cashea);
        assert_eq!(payments[0].note.as_deref(), Some("Cuota 1/6"));
        assert_eq!(
            payments[0].status(NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()),
            PaymentStatus::Pending(5)
        );

        let local_date = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let expense = NewExpense {
            amount: Money::new(1500, Currency::new("USD").unwrap()).unwrap(),
            category_id,
            note: Some("Luz".into()),
            spent_at: DateTime::parse_from_rfc3339("2026-11-20T09:00:00-04:00").unwrap(),
            local_date,
            evidence_path: None,
            rate_cents: None,
            rate_day: None,
            method: MovementMethod::Other,
            original_amount_cents: None,
            original_currency: None,
        };
        repo.pay_recurring_payment(id, &expense).unwrap();

        let payments = repo.recurring_payments().unwrap();
        assert_eq!(payments[0].last_paid_period, Some((2026, 11)));
        assert_eq!(payments[0].status(local_date), PaymentStatus::Paid);
        assert_eq!(
            repo.expenses_between(local_date, local_date).unwrap().len(),
            1
        );
        assert!(repo.delete_recurring_payment(id).unwrap());
        assert!(repo.recurring_payments().unwrap().is_empty());
    }

    #[test]
    fn persists_one_time_payment_with_specific_date() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        let category_id = repo.add_category("Compras", None, "#ffffff").unwrap();
        let due = NaiveDate::from_ymd_opt(2026, 11, 25).unwrap();
        let payment = NewRecurringPayment {
            kind: finance_core::RecurringKind::Payment,
            name: "Cashea".into(),
            amount: Money::new(8000, Currency::new("USD").unwrap()).unwrap(),
            category_id,
            due_day: 25,
            due_date: Some(due),
            method: PaymentMethod::Cashea,
            note: Some("Cuota 3/6".into()),
        };
        let id = repo.add_recurring_payment(&payment).unwrap();
        let payments = repo.recurring_payments().unwrap();
        assert_eq!(payments[0].due_date, Some(due));
        assert!(payments[0].is_one_time());
        assert_eq!(payments[0].method, PaymentMethod::Cashea);

        let paid_on = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let expense = NewExpense {
            amount: Money::new(8000, Currency::new("USD").unwrap()).unwrap(),
            category_id,
            note: None,
            spent_at: DateTime::parse_from_rfc3339("2026-11-20T09:00:00-04:00").unwrap(),
            local_date: paid_on,
            evidence_path: None,
            rate_cents: None,
            rate_day: None,
            method: MovementMethod::Other,
            original_amount_cents: None,
            original_currency: None,
        };
        repo.pay_recurring_payment(id, &expense).unwrap();
        assert_eq!(
            repo.recurring_payments().unwrap()[0]
                .status(NaiveDate::from_ymd_opt(2026, 12, 20).unwrap()),
            PaymentStatus::Paid
        );
    }

    #[test]
    fn keeps_rate_history_and_expense_metadata() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        let day20 = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let day21 = NaiveDate::from_ymd_opt(2026, 11, 21).unwrap();
        let day22 = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();

        repo.set_rate_for_day(day20, 85000, "manual").unwrap();
        repo.set_rate_for_day(day21, 85789, "dolarapi").unwrap();
        assert_eq!(repo.rate_on(day21).unwrap(), Some((day21, 85789)));
        assert_eq!(repo.rate_on(day22).unwrap(), Some((day21, 85789)));
        assert_eq!(repo.all_rates().unwrap().len(), 2);

        let category_id = repo.add_category("Comida", None, "#ff0000").unwrap();
        let new = NewExpense {
            amount: Money::new(1250, Currency::new("USD").unwrap()).unwrap(),
            category_id,
            note: None,
            spent_at: DateTime::parse_from_rfc3339("2026-11-22T12:00:00-04:00").unwrap(),
            local_date: day22,
            evidence_path: Some("/tmp/recibo.jpg".into()),
            rate_cents: Some(85789),
            rate_day: Some(day21.to_string()),
            method: MovementMethod::Other,
            original_amount_cents: None,
            original_currency: None,
        };
        let id = repo.insert_expense(&new).unwrap();
        let saved = repo
            .expenses_between(day22, day22)
            .unwrap()
            .into_iter()
            .find(|expense| expense.id == id)
            .unwrap();
        assert_eq!(saved.evidence_path.as_deref(), Some("/tmp/recibo.jpg"));
        assert_eq!(saved.rate_cents, Some(85789));
        assert_eq!(saved.rate_day.as_deref(), Some("2026-11-21"));
    }

    #[test]
    fn persists_income_method() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        let day = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
        let category_id = repo.add_income_category("Nómina", None, "#0f0").unwrap();
        repo.insert_income(&NewIncome {
            amount: Money::new(100000, Currency::new("USD").unwrap()).unwrap(),
            category_id,
            note: Some("Quincena".into()),
            received_at: DateTime::parse_from_rfc3339("2026-11-23T09:00:00-04:00").unwrap(),
            local_date: day,
            rate_cents: None,
            rate_day: None,
            method: MovementMethod::Cash,
            original_amount_cents: None,
            original_currency: None,
        })
        .unwrap();

        assert_eq!(
            repo.incomes_between(day, day).unwrap()[0].method,
            MovementMethod::Cash
        );
    }

    #[test]
    fn backup_roundtrip_replaces_and_restores_all_data() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        let category_id = repo.add_category("Comida", None, "#ff0000").unwrap();
        repo.set_budget(category_id, 10000).unwrap();
        repo.set_rate_for_day(
            NaiveDate::from_ymd_opt(2026, 11, 22).unwrap(),
            85789,
            "dolarapi",
        )
        .unwrap();
        let day = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();
        repo.insert_expense(&NewExpense {
            amount: Money::new(1250, Currency::new("USD").unwrap()).unwrap(),
            category_id,
            note: None,
            spent_at: DateTime::parse_from_rfc3339("2026-11-22T12:00:00-04:00").unwrap(),
            local_date: day,
            evidence_path: None,
            rate_cents: None,
            rate_day: None,
            method: MovementMethod::BankTransfer,
            original_amount_cents: None,
            original_currency: None,
        })
        .unwrap();

        let backup = repo.export_backup().unwrap();
        backup.validate().unwrap();
        assert_eq!(backup.expenses.len(), 1);
        assert_eq!(backup.rates.len(), 1);
        assert_eq!(backup.settings.len(), 0);

        repo.import_backup(&Backup::new("2026-11-22T00:00:00Z".into()))
            .unwrap();
        assert!(repo.expenses_between(day, day).unwrap().is_empty());
        assert!(repo.categories().unwrap().is_empty());

        repo.import_backup(&backup).unwrap();
        assert_eq!(repo.expenses_between(day, day).unwrap().len(), 1);
        assert_eq!(
            repo.expenses_between(day, day).unwrap()[0].method,
            MovementMethod::BankTransfer
        );
        assert_eq!(repo.categories().unwrap().len(), 1);
        assert_eq!(repo.budgets().unwrap().len(), 1);
        assert_eq!(repo.rate_details().unwrap()[0].2, "dolarapi");
    }

    #[test]
    fn ensures_seed_category_without_duplicates() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        repo.ensure_category("Familia", Some("familia"), "#c77dff")
            .unwrap();
        repo.ensure_category("Familia", Some("otro"), "#000000")
            .unwrap();
        let categories = repo.categories().unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].name, "Familia");
    }

    #[test]
    fn persists_and_restores_activity_log() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        repo.insert_activity(ActivityKind::ExpenseCreated, 7, "almuerzo", 1250)
            .unwrap();
        repo.insert_activity(ActivityKind::PaymentPaid, 3, "Luz", 1500)
            .unwrap();

        let entries = repo.activities(10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, ActivityKind::PaymentPaid);
        assert_eq!(entries[0].summary, "Luz");
        assert_eq!(entries[1].amount_cents, 1250);

        let backup = repo.export_backup().unwrap();
        assert_eq!(backup.activities.len(), 2);
        repo.import_backup(&Backup::new("2026-11-22T00:00:00Z".into()))
            .unwrap();
        assert!(repo.activities(10).unwrap().is_empty());

        repo.import_backup(&backup).unwrap();
        let restored = repo.activities(10).unwrap();
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[0].kind, ActivityKind::PaymentPaid);
        assert_eq!(restored[0].occurred_at, entries[0].occurred_at);
        assert_eq!(restored[1].kind, ActivityKind::ExpenseCreated);
    }
}
