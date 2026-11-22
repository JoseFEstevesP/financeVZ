use std::collections::BTreeMap;

use chrono::Datelike;

use crate::{
    category::Category,
    error::DomainError,
    expense::Expense,
    money::{Currency, Money},
};

#[derive(Debug, Clone, PartialEq)]
pub struct CategoryTotal {
    pub category: Category,
    pub total: Money,
    pub percentage: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlyTotal {
    pub year: i32,
    pub month: u32,
    pub total: Money,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub total: Money,
    pub by_category: Vec<CategoryTotal>,
    pub by_month: Vec<MonthlyTotal>,
}

pub fn build_report(expenses: &[Expense], categories: &[Category]) -> Result<Report, DomainError> {
    let currency = expenses
        .first()
        .map(|expense| expense.amount.currency().clone())
        .unwrap_or_else(|| Currency::new("USD").expect("USD is valid"));

    let total = expenses.iter().try_fold(Money::zero(currency.clone()), |acc, expense| {
        acc.checked_add(&expense.amount)
    })?;

    let mut by_category: BTreeMap<i64, i64> = BTreeMap::new();
    let mut by_month: BTreeMap<(i32, u32), i64> = BTreeMap::new();

    for expense in expenses {
        *by_category.entry(expense.category_id.get()).or_default() += expense.amount.cents();
        let key = (expense.local_date.year(), expense.local_date.month());
        *by_month.entry(key).or_default() += expense.amount.cents();
    }

    let category_totals = by_category
        .into_iter()
        .map(|(category_id, cents)| {
            let category = categories
                .iter()
                .find(|category| category.id.get() == category_id)
                .ok_or(DomainError::UnknownCategory(category_id))?;
            let amount = Money::from_cents(cents, currency.clone());
            let percentage = if total.cents() == 0 {
                0.0
            } else {
                cents as f64 / total.cents() as f64 * 100.0
            };
            Ok(CategoryTotal {
                category: category.clone(),
                total: amount,
                percentage,
            })
        })
        .collect::<Result<Vec<_>, DomainError>>()?;

    let mut category_totals = category_totals;
    category_totals.sort_by(|a, b| b.total.cents().cmp(&a.total.cents()));

    let month_totals = by_month
        .into_iter()
        .map(|((year, month), cents)| MonthlyTotal {
            year,
            month,
            total: Money::from_cents(cents, currency.clone()),
        })
        .collect();

    Ok(Report {
        total,
        by_category: category_totals,
        by_month: month_totals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ids::{CategoryId, ExpenseId}, money::Currency};
    use chrono::{DateTime, NaiveDate};

    fn category(id: i64, name: &str) -> Category {
        Category {
            id: CategoryId::new(id),
            name: name.into(),
            icon: None,
            color: "#ff0000".into(),
            is_active: true,
            sort_order: 0,
        }
    }

    fn expense(id: i64, category_id: i64, cents: i64, day: u32) -> Expense {
        let spent_at = DateTime::parse_from_rfc3339(&format!(
            "2026-11-{day:02}T12:00:00-04:00"
        ))
        .unwrap();
        Expense {
            id: ExpenseId::new(id),
            amount: Money::new(cents, Currency::new("USD").unwrap()).unwrap(),
            category_id: CategoryId::new(category_id),
            note: None,
            spent_at,
            local_date: NaiveDate::from_ymd_opt(2026, 11, day).unwrap(),
            evidence_path: None,
            rate_cents: None,
            rate_day: None,
        }
    }

    #[test]
    fn aggregates_total_categories_and_months() {
        let categories = vec![category(1, "Comida"), category(2, "Transporte")];
        let expenses = vec![
            expense(1, 1, 1000, 3),
            expense(2, 1, 500, 20),
            expense(3, 2, 1500, 20),
        ];

        let report = build_report(&expenses, &categories).unwrap();

        assert_eq!(report.total.cents(), 3000);
        assert_eq!(report.by_category.len(), 2);
        assert_eq!(report.by_category[0].category.name, "Comida");
        assert_eq!(report.by_category[0].total.cents(), 1500);
        assert!((report.by_category[0].percentage - 50.0).abs() < f64::EPSILON);
        assert_eq!(report.by_month.len(), 1);
        assert_eq!(report.by_month[0].total.cents(), 3000);
    }

    #[test]
    fn empty_report_is_zero() {
        let report = build_report(&[], &[]).unwrap();
        assert_eq!(report.total.cents(), 0);
        assert!(report.by_category.is_empty());
        assert!(report.by_month.is_empty());
    }

    #[test]
    fn unknown_category_is_an_error() {
        let expenses = vec![expense(1, 99, 100, 1)];
        assert_eq!(
            build_report(&expenses, &[]),
            Err(DomainError::UnknownCategory(99))
        );
    }
}
