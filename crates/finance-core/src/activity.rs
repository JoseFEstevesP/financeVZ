use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    ExpenseCreated,
    ExpenseUpdated,
    ExpenseDeleted,
    ExpenseRefunded,
    IncomeCreated,
    IncomeUpdated,
    IncomeDeleted,
    PaymentCreated,
    PaymentPaid,
    PaymentDeleted,
}

impl ActivityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExpenseCreated => "expense_created",
            Self::ExpenseUpdated => "expense_updated",
            Self::ExpenseDeleted => "expense_deleted",
            Self::ExpenseRefunded => "expense_refunded",
            Self::IncomeCreated => "income_created",
            Self::IncomeUpdated => "income_updated",
            Self::IncomeDeleted => "income_deleted",
            Self::PaymentCreated => "payment_created",
            Self::PaymentPaid => "payment_paid",
            Self::PaymentDeleted => "payment_deleted",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "expense_updated" => Self::ExpenseUpdated,
            "expense_deleted" => Self::ExpenseDeleted,
            "expense_refunded" => Self::ExpenseRefunded,
            "income_created" => Self::IncomeCreated,
            "income_updated" => Self::IncomeUpdated,
            "income_deleted" => Self::IncomeDeleted,
            "payment_created" => Self::PaymentCreated,
            "payment_paid" => Self::PaymentPaid,
            "payment_deleted" => Self::PaymentDeleted,
            _ => Self::ExpenseCreated,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ExpenseCreated => "Gasto creado",
            Self::ExpenseUpdated => "Gasto editado",
            Self::ExpenseDeleted => "Gasto eliminado",
            Self::ExpenseRefunded => "Gasto devuelto",
            Self::IncomeCreated => "Ingreso creado",
            Self::IncomeUpdated => "Ingreso editado",
            Self::IncomeDeleted => "Ingreso eliminado",
            Self::PaymentCreated => "Pago creado",
            Self::PaymentPaid => "Pago realizado",
            Self::PaymentDeleted => "Pago eliminado",
        }
    }

    pub fn is_expense(self) -> bool {
        matches!(
            self,
            Self::ExpenseCreated
                | Self::ExpenseUpdated
                | Self::ExpenseDeleted
                | Self::ExpenseRefunded
        )
    }

    pub fn is_income(self) -> bool {
        matches!(
            self,
            Self::IncomeCreated | Self::IncomeUpdated | Self::IncomeDeleted
        )
    }

    pub fn is_payment(self) -> bool {
        matches!(
            self,
            Self::PaymentCreated | Self::PaymentPaid | Self::PaymentDeleted
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEntry {
    pub id: i64,
    pub kind: ActivityKind,
    pub entity_id: i64,
    pub summary: String,
    pub amount_cents: i64,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupActivity {
    pub id: i64,
    pub kind: String,
    pub entity_id: i64,
    pub summary: String,
    pub amount_cents: i64,
    pub occurred_at: String,
}
