use chrono::{Datelike, NaiveDate};

use crate::{
    error::DomainError,
    ids::{CategoryId, RecurringPaymentId},
    money::Money,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaymentMethod {
    Cashea,
    Transfer,
    Cash,
    #[default]
    Other,
}

impl PaymentMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cashea => "cashea",
            Self::Transfer => "transfer",
            Self::Cash => "cash",
            Self::Other => "other",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cashea => "Cashea",
            Self::Transfer => "Transferencia",
            Self::Cash => "Efectivo",
            Self::Other => "Otro",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "cashea" => Self::Cashea,
            "transfer" => Self::Transfer,
            "cash" => Self::Cash,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecurringKind {
    #[default]
    Payment,
    Expense,
}

impl RecurringKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Payment => "payment",
            Self::Expense => "expense",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Payment => "Pago",
            Self::Expense => "Gasto recurrente",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "expense" => Self::Expense,
            _ => Self::Payment,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRecurringPayment {
    pub kind: RecurringKind,
    pub name: String,
    pub amount: Money,
    pub category_id: CategoryId,
    pub due_day: u8,
    /// Fecha concreta: se paga una sola vez. `None` = pago mensual por `due_day`.
    pub due_date: Option<NaiveDate>,
    pub method: PaymentMethod,
    pub note: Option<String>,
}

impl NewRecurringPayment {
    pub fn validate(&self) -> Result<(), DomainError> {
        let length = self.name.trim().chars().count();
        if length == 0 || length > 40 {
            return Err(DomainError::InvalidPaymentName);
        }
        if self.due_date.is_none() && !(1..=31).contains(&self.due_day) {
            return Err(DomainError::InvalidDueDay);
        }
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
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringPayment {
    pub id: RecurringPaymentId,
    pub kind: RecurringKind,
    pub name: String,
    pub amount: Money,
    pub category_id: CategoryId,
    pub due_day: u8,
    pub due_date: Option<NaiveDate>,
    pub method: PaymentMethod,
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
    pub fn is_one_time(&self) -> bool {
        self.due_date.is_some()
    }

    pub fn due_date(&self, year: i32, month: u32) -> NaiveDate {
        if let Some(day) = self.due_date {
            return day;
        }
        (1..=self.due_day)
            .rev()
            .find_map(|day| NaiveDate::from_ymd_opt(year, month, day.into()))
            .expect("mes válido")
    }

    pub fn status(&self, today: NaiveDate) -> PaymentStatus {
        if self.is_one_time() && self.last_paid_period.is_some() {
            return PaymentStatus::Paid;
        }
        if !self.is_one_time() && self.last_paid_period == Some((today.year(), today.month())) {
            return PaymentStatus::Paid;
        }
        Self::status_from_days(
            (self.due_date(today.year(), today.month()) - today).num_days() as i32,
        )
    }

    fn status_from_days(days: i32) -> PaymentStatus {
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
            kind: RecurringKind::Payment,
            name: "Luz".into(),
            amount: Money::new(1500, Currency::new("VES").unwrap()).unwrap(),
            category_id: CategoryId::new(1),
            due_day,
            due_date: None,
            method: PaymentMethod::Other,
            note: None,
            last_paid_period: None,
            is_active: true,
        }
    }

    #[test]
    fn validates_payment_fields() {
        let mut new = NewRecurringPayment {
            kind: RecurringKind::Payment,
            name: "Agua".into(),
            amount: Money::new(1000, Currency::new("VES").unwrap()).unwrap(),
            category_id: CategoryId::new(1),
            due_day: 15,
            due_date: None,
            method: PaymentMethod::Other,
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
        assert_eq!(
            payment.due_date(2026, 2),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
        assert_eq!(
            payment.due_date(2024, 2),
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );
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

    #[test]
    fn one_time_payment_keeps_its_date_and_stays_paid() {
        let mut one = payment(25);
        one.due_date = NaiveDate::from_ymd_opt(2026, 11, 25);
        one.method = PaymentMethod::Cashea;
        one.note = Some("Cuota 3/6".into());

        let today = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        assert!(one.is_one_time());
        assert_eq!(one.status(today), PaymentStatus::Pending(5));
        assert_eq!(
            one.due_date(2026, 12),
            NaiveDate::from_ymd_opt(2026, 11, 25).unwrap()
        );

        one.last_paid_period = Some((2026, 11));
        assert_eq!(
            one.status(NaiveDate::from_ymd_opt(2026, 12, 20).unwrap()),
            PaymentStatus::Paid
        );
    }

    #[test]
    fn parses_payment_methods() {
        assert_eq!(PaymentMethod::parse("cashea"), PaymentMethod::Cashea);
        assert_eq!(PaymentMethod::parse("transfer"), PaymentMethod::Transfer);
        assert_eq!(PaymentMethod::parse("cash"), PaymentMethod::Cash);
        assert_eq!(PaymentMethod::parse("otro"), PaymentMethod::Other);
        assert_eq!(PaymentMethod::Cashea.label(), "Cashea");
    }
}
