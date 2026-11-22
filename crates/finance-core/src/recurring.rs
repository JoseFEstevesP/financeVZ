use chrono::{Datelike, NaiveDate};

use crate::{
    error::DomainError,
    ids::{CategoryId, RecurringPaymentId},
    money::Money,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRecurringPayment {
    pub name: String,
    pub amount: Money,
    pub category_id: CategoryId,
    pub due_day: u8,
    pub note: Option<String>,
}

impl NewRecurringPayment {
    pub fn validate(&self) -> Result<(), DomainError> {
        let length = self.name.trim().chars().count();
        if length == 0 || length > 40 {
            return Err(DomainError::InvalidPaymentName);
        }
        if !(1..=31).contains(&self.due_day) {
            return Err(DomainError::InvalidDueDay);
        }
        if self.amount.cents() <= 0 {
            return Err(DomainError::InvalidAmount);
        }
        if self.note.as_ref().is_some_and(|note| note.chars().count() > 200) {
            return Err(DomainError::NoteTooLong);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringPayment {
    pub id: RecurringPaymentId,
    pub name: String,
    pub amount: Money,
    pub category_id: CategoryId,
    pub due_day: u8,
    pub note: Option<String>,
    pub last_paid_period: Option<(i32, u32)>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentStatus {
    Paid,
    Pending(i32),
    DueSoon(i32),
    Overdue(i32),
}

impl RecurringPayment {
    pub fn due_date(&self, year: i32, month: u32) -> NaiveDate {
        (1..=self.due_day)
            .rev()
            .find_map(|day| NaiveDate::from_ymd_opt(year, month, day.into()))
            .expect("mes válido")
    }

    pub fn status(&self, today: NaiveDate) -> PaymentStatus {
        if self.last_paid_period == Some((today.year(), today.month())) {
            return PaymentStatus::Paid;
        }
        let days = (self.due_date(today.year(), today.month()) - today).num_days() as i32;
        match days {
            days if days < 0 => PaymentStatus::Overdue(-days),
            days if days <= 3 => PaymentStatus::DueSoon(days),
            days => PaymentStatus::Pending(days),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Currency;

    fn payment(due_day: u8) -> RecurringPayment {
        RecurringPayment {
            id: RecurringPaymentId::new(1),
            name: "Luz".into(),
            amount: Money::new(1500, Currency::new("VES").unwrap()).unwrap(),
            category_id: CategoryId::new(1),
            due_day,
            note: None,
            last_paid_period: None,
            is_active: true,
        }
    }

    #[test]
    fn validates_payment_fields() {
        let mut new = NewRecurringPayment {
            name: "Agua".into(),
            amount: Money::new(1000, Currency::new("VES").unwrap()).unwrap(),
            category_id: CategoryId::new(1),
            due_day: 15,
            note: None,
        };
        assert_eq!(new.validate(), Ok(()));
        new.due_day = 32;
        assert_eq!(new.validate(), Err(DomainError::InvalidDueDay));
        new.due_day = 15;
        new.name = " ".into();
        assert_eq!(new.validate(), Err(DomainError::InvalidPaymentName));
    }

    #[test]
    fn clamps_due_day_to_short_months() {
        let payment = payment(31);
        assert_eq!(payment.due_date(2026, 2), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
        assert_eq!(payment.due_date(2024, 2), NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
    }

    #[test]
    fn computes_status_from_today() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        assert_eq!(payment(25).status(today), PaymentStatus::Pending(5));
        assert_eq!(payment(22).status(today), PaymentStatus::DueSoon(2));
        assert_eq!(payment(18).status(today), PaymentStatus::Overdue(2));
        assert_eq!(payment(20).status(today), PaymentStatus::DueSoon(0));

        let mut paid = payment(5);
        paid.last_paid_period = Some((2026, 11));
        assert_eq!(paid.status(today), PaymentStatus::Paid);
    }
}
