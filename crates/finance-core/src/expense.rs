use chrono::{DateTime, FixedOffset, NaiveDate};

use crate::{
    error::DomainError,
    ids::{CategoryId, ExpenseId},
    method::MovementMethod,
    money::Money,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewExpense {
    pub amount: Money,
    pub category_id: CategoryId,
    pub note: Option<String>,
    pub spent_at: DateTime<FixedOffset>,
    pub local_date: NaiveDate,
    pub evidence_path: Option<String>,
    pub rate_cents: Option<i64>,
    pub rate_day: Option<String>,
    pub method: MovementMethod,
    pub original_amount_cents: Option<i64>,
    pub original_currency: Option<String>,
}

impl NewExpense {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.amount.cents() <= 0 {
            return Err(DomainError::InvalidAmount);
        }
        if self
            .note
            .as_ref()
            .is_some_and(|note| note.chars().count() > 200)
        {
            return Err(DomainError::NoteTooLong);
        }
        if self.local_date != self.spent_at.date_naive() {
            return Err(DomainError::DateMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expense {
    pub id: ExpenseId,
    pub amount: Money,
    pub category_id: CategoryId,
    pub note: Option<String>,
    pub spent_at: DateTime<FixedOffset>,
    pub local_date: NaiveDate,
    pub evidence_path: Option<String>,
    pub rate_cents: Option<i64>,
    pub rate_day: Option<String>,
    pub method: MovementMethod,
    pub original_amount_cents: Option<i64>,
    pub original_currency: Option<String>,
}

impl Expense {
    pub fn from_new(id: ExpenseId, new: NewExpense) -> Self {
        Self {
            id,
            amount: new.amount,
            category_id: new.category_id,
            note: new.note,
            spent_at: new.spent_at,
            local_date: new.local_date,
            evidence_path: new.evidence_path,
            rate_cents: new.rate_cents,
            rate_day: new.rate_day,
            method: new.method,
            original_amount_cents: new.original_amount_cents,
            original_currency: new.original_currency,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Currency;

    fn expense(note: Option<String>, local_date: NaiveDate) -> NewExpense {
        let spent_at = DateTime::parse_from_rfc3339("2026-11-21T12:30:00-04:00").unwrap();
        NewExpense {
            amount: Money::new(1250, Currency::new("USD").unwrap()).unwrap(),
            category_id: CategoryId::new(1),
            note,
            spent_at,
            local_date,
            evidence_path: None,
            rate_cents: None,
            rate_day: None,
            method: MovementMethod::Other,
            original_amount_cents: None,
            original_currency: None,
        }
    }

    #[test]
    fn accepts_a_valid_expense() {
        let new = expense(
            Some("almuerzo".into()),
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
        );
        assert_eq!(new.validate(), Ok(()));
    }

    #[test]
    fn rejects_long_notes_and_wrong_local_date() {
        let long = expense(
            Some("x".repeat(201)),
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
        );
        assert_eq!(long.validate(), Err(DomainError::NoteTooLong));

        let wrong_date = expense(None, NaiveDate::from_ymd_opt(2026, 11, 20).unwrap());
        assert_eq!(wrong_date.validate(), Err(DomainError::DateMismatch));
    }
}
