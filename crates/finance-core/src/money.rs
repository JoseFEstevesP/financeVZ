use std::fmt;

use crate::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Currency(String);

impl Currency {
    pub fn new(code: &str) -> Result<Self, DomainError> {
        if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(DomainError::InvalidCurrency);
        }
        Ok(Self(code.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Money {
    cents: i64,
    currency: Currency,
}

impl Money {
    pub fn new(cents: i64, currency: Currency) -> Result<Self, DomainError> {
        if cents <= 0 {
            return Err(DomainError::InvalidAmount);
        }
        Ok(Self { cents, currency })
    }

    pub fn zero(currency: Currency) -> Self {
        Self {
            cents: 0,
            currency,
        }
    }

    pub fn from_cents(cents: i64, currency: Currency) -> Self {
        Self { cents, currency }
    }

    pub fn cents(&self) -> i64 {
        self.cents
    }

    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    pub fn checked_add(&self, other: &Self) -> Result<Self, DomainError> {
        if self.currency != other.currency {
            return Err(DomainError::MixedCurrencies);
        }
        let cents = self.cents.checked_add(other.cents).ok_or(DomainError::Overflow)?;
        Ok(Self {
            cents,
            currency: self.currency.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_rejects_lowercase_and_bad_length() {
        assert_eq!(Currency::new("usd"), Err(DomainError::InvalidCurrency));
        assert_eq!(Currency::new("USD1"), Err(DomainError::InvalidCurrency));
        assert!(Currency::new("EUR").is_ok());
    }

    #[test]
    fn money_rejects_non_positive_amounts() {
        let currency = Currency::new("USD").unwrap();
        assert_eq!(Money::new(0, currency.clone()), Err(DomainError::InvalidAmount));
        assert_eq!(Money::new(-1, currency.clone()), Err(DomainError::InvalidAmount));
        assert!(Money::new(1, currency).is_ok());
    }

    #[test]
    fn adding_mixed_currencies_fails() {
        let usd = Money::from_cents(100, Currency::new("USD").unwrap());
        let eur = Money::from_cents(100, Currency::new("EUR").unwrap());
        assert_eq!(usd.checked_add(&eur), Err(DomainError::MixedCurrencies));
    }

    #[test]
    fn adding_money_keeps_currency() {
        let currency = Currency::new("VES").unwrap();
        let total = Money::from_cents(150, currency.clone())
            .checked_add(&Money::from_cents(250, currency.clone()))
            .unwrap();
        assert_eq!(total.cents(), 400);
        assert_eq!(total.currency(), &currency);
    }
}
