use chrono::{DateTime, FixedOffset, NaiveDate};

use crate::{
    error::DomainError,
    ids::{CategoryId, IncomeId},
    money::Money,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewIncome {
    pub amount: Money,
    pub category_id: CategoryId,
    pub note: Option<String>,
    pub received_at: DateTime<FixedOffset>,
    pub local_date: NaiveDate,
    pub rate_cents: Option<i64>,
    pub rate_day: Option<String>,
}

impl NewIncome {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.amount.cents() <= 0 {
            return Err(DomainError::InvalidAmount);
        }
        if self.note.as_ref().is_some_and(|note| note.chars().count() > 200) {
            return Err(DomainError::NoteTooLong);
        }
        if self.local_date != self.received_at.date_naive() {
            return Err(DomainError::DateMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Income {
    pub id: IncomeId,
    pub amount: Money,
    pub category_id: CategoryId,
    pub note: Option<String>,
    pub received_at: DateTime<FixedOffset>,
    pub local_date: NaiveDate,
    pub rate_cents: Option<i64>,
    pub rate_day: Option<String>,
}

impl Income {
    pub fn from_new(id: IncomeId, new: NewIncome) -> Self {
        Self {
            id,
            amount: new.amount,
            category_id: new.category_id,
            note: new.note,
            received_at: new.received_at,
            local_date: new.local_date,
            rate_cents: new.rate_cents,
            rate_day: new.rate_day,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Currency;

    fn income(note: Option<String>, local_date: NaiveDate) -> NewIncome {
        NewIncome {
            amount: Money::new(1250, Currency::new("USD").unwrap()).unwrap(),
            category_id: CategoryId::new(1),
            note,
            received_at: DateTime::parse_from_rfc3339("2026-11-21T12:30:00-04:00").unwrap(),
            local_date,
            rate_cents: None,
            rate_day: None,
        }
    }

    #[test]
    fn accepts_a_valid_income() {
        let new = income(Some("sueldo".into()), NaiveDate::from_ymd_opt(2026, 11, 21).unwrap());
        assert_eq!(new.validate(), Ok(()));
    }

    #[test]
    fn rejects_long_notes_and_wrong_local_date() {
        let long = income(Some("x".repeat(201)), NaiveDate::from_ymd_opt(2026, 11, 21).unwrap());
        assert_eq!(long.validate(), Err(DomainError::NoteTooLong));

        let wrong_date = income(None, NaiveDate::from_ymd_opt(2026, 11, 20).unwrap());
        assert_eq!(wrong_date.validate(), Err(DomainError::DateMismatch));
    }
}
