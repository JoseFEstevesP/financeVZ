use chrono::{DateTime, NaiveDate};
use finance_core::{
    Category, CategoryId, Currency, Expense, ExpenseId, Income, IncomeId, Money, PaymentMethod,
    RecurringKind, RecurringPayment, RecurringPaymentId,
};
use rusqlite::{Error, Row, types::Type};

fn conversion(error: impl std::error::Error + Send + Sync + 'static) -> Error {
    Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
}

pub fn category_from_row(row: &Row<'_>) -> Result<Category, Error> {
    Ok(Category {
        id: CategoryId::new(row.get("id")?),
        name: row.get("name")?,
        icon: row.get("icon")?,
        color: row.get("color")?,
        is_active: row.get::<_, i64>("is_active")? == 1,
        sort_order: row.get("sort_order")?,
    })
}

pub fn expense_from_row(row: &Row<'_>) -> Result<Expense, Error> {
    let cents: i64 = row.get("amount_cents")?;
    let currency: String = row.get("currency")?;
    let currency = Currency::new(&currency).map_err(conversion)?;
    let spent_at: String = row.get("spent_at")?;
    let spent_at = DateTime::parse_from_rfc3339(&spent_at).map_err(conversion)?;
    let local_date: String = row.get("local_date")?;
    let local_date = NaiveDate::parse_from_str(&local_date, "%Y-%m-%d").map_err(conversion)?;

    Ok(Expense {
        id: ExpenseId::new(row.get("id")?),
        amount: Money::from_cents(cents, currency),
        category_id: CategoryId::new(row.get("category_id")?),
        note: row.get("note")?,
        spent_at,
        local_date,
        evidence_path: row.get("evidence_path")?,
        rate_cents: row.get("rate_cents")?,
        rate_day: row.get("rate_day")?,
    })
}

pub fn income_from_row(row: &Row<'_>) -> Result<Income, Error> {
    let cents: i64 = row.get("amount_cents")?;
    let currency: String = row.get("currency")?;
    let currency = Currency::new(&currency).map_err(conversion)?;
    let received_at: String = row.get("received_at")?;
    let received_at = DateTime::parse_from_rfc3339(&received_at).map_err(conversion)?;
    let local_date: String = row.get("local_date")?;
    let local_date = NaiveDate::parse_from_str(&local_date, "%Y-%m-%d").map_err(conversion)?;

    Ok(Income {
        id: IncomeId::new(row.get("id")?),
        amount: Money::from_cents(cents, currency),
        category_id: CategoryId::new(row.get("category_id")?),
        note: row.get("note")?,
        received_at,
        local_date,
        rate_cents: row.get("rate_cents")?,
        rate_day: row.get("rate_day")?,
    })
}

pub fn recurring_from_row(row: &Row<'_>) -> Result<RecurringPayment, Error> {
    let cents: i64 = row.get("amount_cents")?;
    let currency: String = row.get("currency")?;
    let currency = Currency::new(&currency).map_err(conversion)?;
    let period: Option<String> = row.get("last_paid_period")?;
    let last_paid_period = period.and_then(|value| {
        let mut parts = value.split('-');
        let year = parts.next()?.parse().ok()?;
        let month = parts.next()?.parse().ok()?;
        Some((year, month))
    });
    let due_date = row
        .get::<_, Option<String>>("due_date")?
        .and_then(|value| NaiveDate::parse_from_str(&value, "%Y-%m-%d").ok());
    let method = PaymentMethod::parse(&row.get::<_, String>("payment_method")?);

    Ok(RecurringPayment {
        id: RecurringPaymentId::new(row.get("id")?),
        kind: RecurringKind::parse(&row.get::<_, String>("kind")?),
        name: row.get("name")?,
        amount: Money::from_cents(cents, currency),
        category_id: CategoryId::new(row.get("category_id")?),
        due_day: row.get::<_, i64>("due_day")? as u8,
        due_date,
        method,
        note: row.get("note")?,
        last_paid_period,
        is_active: row.get::<_, i64>("is_active")? == 1,
    })
}
