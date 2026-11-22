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
        Currency, Money, NewExpense, NewIncome, NewRecurringPayment, PaymentMethod, PaymentStatus,
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
        };
        let expense_id = repo.insert_expense(&new).unwrap();

        let from = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 11, 30).unwrap();
        let expenses = repo.expenses_between(from, to).unwrap();
        assert_eq!(expenses.len(), 1);
        assert_eq!(expenses[0].id, expense_id);
        assert_eq!(expenses[0].amount.cents(), 1250);
        assert_eq!(expenses[0].note.as_deref(), Some("almuerzo"));

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
        let category_id = repo.add_category("Hogar", Some("house"), "#ffffff").unwrap();

        assert_eq!(repo.get_setting("rate").unwrap(), None);
        repo.set_setting("rate", "85789").unwrap();
        assert_eq!(repo.get_setting("rate").unwrap(), Some("85789".into()));

        let payment = NewRecurringPayment {
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
}
