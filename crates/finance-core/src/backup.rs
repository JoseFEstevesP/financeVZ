//! Formato estable de copia de seguridad. Usa tipos primitivos para no acoplar
//! el archivo exportado a los cambios internos del dominio.

use serde::{Deserialize, Serialize};

pub const BACKUP_FORMAT: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backup {
    pub format: u32,
    pub exported_at: String,
    pub categories: Vec<BackupCategory>,
    pub income_categories: Vec<BackupCategory>,
    pub expenses: Vec<BackupExpense>,
    pub incomes: Vec<BackupIncome>,
    pub payments: Vec<BackupPayment>,
    pub budgets: Vec<BackupBudget>,
    pub rates: Vec<BackupRate>,
    pub settings: Vec<BackupSetting>,
}

impl Backup {
    pub fn new(exported_at: String) -> Self {
        Self {
            format: BACKUP_FORMAT,
            exported_at,
            categories: Vec::new(),
            income_categories: Vec::new(),
            expenses: Vec::new(),
            incomes: Vec::new(),
            payments: Vec::new(),
            budgets: Vec::new(),
            rates: Vec::new(),
            settings: Vec::new(),
        }
    }

    /// Validación mínima antes de tocar la base de datos.
    pub fn validate(&self) -> Result<(), String> {
        if self.format != BACKUP_FORMAT {
            return Err(format!(
                "Formato de copia no soportado: {} (se espera {BACKUP_FORMAT})",
                self.format
            ));
        }
        for (label, ids) in [
            ("categorías", category_ids(&self.categories)),
            (
                "categorías de ingreso",
                category_ids(&self.income_categories),
            ),
        ] {
            if has_duplicates(&ids) {
                return Err(format!("La copia tiene ids duplicados en {label}"));
            }
        }
        let expense_ids: Vec<i64> = self.expenses.iter().map(|item| item.id).collect();
        let income_ids: Vec<i64> = self.incomes.iter().map(|item| item.id).collect();
        let payment_ids: Vec<i64> = self.payments.iter().map(|item| item.id).collect();
        if has_duplicates(&expense_ids)
            || has_duplicates(&income_ids)
            || has_duplicates(&payment_ids)
        {
            return Err("La copia tiene ids duplicados".into());
        }
        if self.rates.iter().any(|rate| rate.rate_cents <= 0) {
            return Err("La copia tiene una tasa no válida".into());
        }
        Ok(())
    }

    pub fn summary(&self) -> String {
        format!(
            "{} categorías, {} gastos, {} ingresos, {} pagos, {} tasas",
            self.categories.len(),
            self.expenses.len(),
            self.incomes.len(),
            self.payments.len(),
            self.rates.len()
        )
    }
}

fn category_ids(items: &[BackupCategory]) -> Vec<i64> {
    items.iter().map(|item| item.id).collect()
}

fn has_duplicates(ids: &[i64]) -> bool {
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    sorted.windows(2).any(|pair| pair[0] == pair[1])
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupCategory {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: String,
    pub is_active: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupExpense {
    pub id: i64,
    pub amount_cents: i64,
    pub currency: String,
    pub category_id: i64,
    pub note: Option<String>,
    pub spent_at: String,
    pub local_date: String,
    pub evidence_path: Option<String>,
    pub rate_cents: Option<i64>,
    pub rate_day: Option<String>,
    #[serde(default)]
    pub method: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupIncome {
    pub id: i64,
    pub amount_cents: i64,
    pub currency: String,
    pub category_id: i64,
    pub note: Option<String>,
    pub received_at: String,
    pub local_date: String,
    pub rate_cents: Option<i64>,
    pub rate_day: Option<String>,
    #[serde(default)]
    pub method: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupPayment {
    pub id: i64,
    pub name: String,
    pub amount_cents: i64,
    pub currency: String,
    pub category_id: i64,
    pub due_day: i64,
    pub due_date: Option<String>,
    pub payment_method: String,
    pub kind: String,
    pub note: Option<String>,
    pub last_paid_period: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupBudget {
    pub category_id: i64,
    pub limit_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRate {
    pub day: String,
    pub rate_cents: i64,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSetting {
    pub key: String,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_format_and_duplicate_ids() {
        let mut backup = Backup::new("2026-11-22T00:00:00Z".into());
        assert!(backup.validate().is_ok());

        backup.format = 99;
        assert!(backup.validate().is_err());
        backup.format = BACKUP_FORMAT;

        backup.categories.push(BackupCategory {
            id: 1,
            name: "Comida".into(),
            icon: None,
            color: "#fff".into(),
            is_active: true,
            sort_order: 0,
        });
        backup.categories.push(BackupCategory {
            id: 1,
            name: "Casa".into(),
            icon: None,
            color: "#fff".into(),
            is_active: true,
            sort_order: 0,
        });
        assert!(backup.validate().is_err());
    }
}
