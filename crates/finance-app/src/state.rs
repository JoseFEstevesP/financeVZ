use std::sync::{Arc, Mutex};

use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate};
use finance_core::{
    build_income_report, build_report, format_cents_input, format_money, format_rate, usd_to_ves,
    ves_to_usd, Category, CategoryId, Currency, Expense, ExpenseId, Income, IncomeId, Money,
    NewExpense, NewIncome, NewRecurringPayment, PaymentStatus, RecurringPayment, RecurringPaymentId,
    Report,
};
use finance_store::SqliteRepository;

use crate::platform;

pub const DEFAULT_CURRENCY: &str = "USD";
const RATE_SETTING: &str = "exchange_rate_cents";

fn local_moment_at(day: NaiveDate) -> DateTime<FixedOffset> {
    let now = Local::now();
    let offset = *now.offset();
    day.and_time(now.time())
        .and_local_timezone(offset)
        .single()
        .unwrap_or_else(|| now.fixed_offset())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Home,
    Add,
    Income,
    AddIncome,
    Reports,
    Categories,
    Payments,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryCurrency {
    Usd,
    Ves,
}

impl EntryCurrency {
    pub fn code(self) -> &'static str {
        match self {
            Self::Usd => "USD",
            Self::Ves => "VES",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Usd => "USD",
            Self::Ves => "Bs",
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<Mutex<SqliteRepository>>,
    pub view: View,
    pub expenses: Vec<Expense>,
    pub incomes: Vec<Income>,
    pub categories: Vec<Category>,
    pub income_categories: Vec<Category>,
    pub income_category_id: Option<CategoryId>,
    pub payments: Vec<RecurringPayment>,
    /// Dígitos del importe; el número de centavos es exactamente este valor.
    pub amount: String,
    pub note: String,
    pub expense_date: String,
    /// Tasa manual para este gasto; vacía = usar la última tasa conocida de la fecha.
    pub expense_rate_input: String,
    pub evidence_path: Option<String>,
    pub evidence_name: Option<String>,
    pub income_amount: String,
    pub income_date: String,
    pub income_rate_input: String,
    pub income_note: String,
    pub income_entry_currency: EntryCurrency,
    pub category_id: Option<CategoryId>,
    pub new_category_name: String,
    pub new_category_color: String,
    pub new_income_category_name: String,
    pub new_income_category_color: String,
    pub entry_currency: EntryCurrency,
    pub exchange_rate_cents: i64,
    pub rate_input: String,
    pub payment_name: String,
    pub payment_amount: String,
    pub payment_day: String,
    pub payment_category_id: Option<CategoryId>,
    pub feedback: Option<String>,
    pub toast: Option<String>,
}

impl AppState {
    pub fn new(repo: SqliteRepository) -> Self {
        let mut state = Self {
            repo: Arc::new(Mutex::new(repo)),
            view: View::Home,
            expenses: Vec::new(),
            incomes: Vec::new(),
            categories: Vec::new(),
            income_categories: Vec::new(),
            income_category_id: None,
            payments: Vec::new(),
            amount: String::new(),
            note: String::new(),
            expense_date: Local::now().date_naive().to_string(),
            expense_rate_input: String::new(),
            evidence_path: None,
            evidence_name: None,
            income_amount: String::new(),
            income_date: Local::now().date_naive().to_string(),
            income_rate_input: String::new(),
            income_note: String::new(),
            income_entry_currency: EntryCurrency::Usd,
            category_id: None,
            new_category_name: String::new(),
            new_category_color: "#4f8cff".to_owned(),
            new_income_category_name: String::new(),
            new_income_category_color: "#2dd4a7".to_owned(),
            entry_currency: EntryCurrency::Usd,
            exchange_rate_cents: 0,
            rate_input: String::new(),
            payment_name: String::new(),
            payment_amount: String::new(),
            payment_day: String::new(),
            payment_category_id: None,
            feedback: None,
            toast: None,
        };
        state.seed_categories();
        state.seed_income_categories();
        state.refresh();
        state
    }

    fn seed_categories(&mut self) {
        let seeds = [
            ("Comida", "🍔", "#e76f51"),
            ("Transporte", "🚗", "#2a9d8f"),
            ("Hogar", "🏠", "#e9c46a"),
            ("Ocio", "🎬", "#8e7dbe"),
            ("Salud", "💊", "#57a6a1"),
            ("Otros", "📦", "#888888"),
        ];
        let Ok(repo) = self.repo.lock() else {
            return;
        };
        if repo.categories().map(|c| c.is_empty()).unwrap_or(false) {
            for (name, icon, color) in seeds {
                let _ = repo.add_category(name, Some(icon), color);
            }
        }
    }

    fn seed_income_categories(&mut self) {
        let seeds = [
            ("Sueldo", "💼", "#2dd4a7"),
            ("Freelance", "🧑‍💻", "#4f8cff"),
            ("Ventas", "🛒", "#e9c46a"),
            ("Regalos", "🎁", "#e76f51"),
            ("Otros", "📦", "#888888"),
        ];
        let Ok(repo) = self.repo.lock() else {
            return;
        };
        if repo.income_categories().map(|c| c.is_empty()).unwrap_or(false) {
            for (name, icon, color) in seeds {
                let _ = repo.add_income_category(name, Some(icon), color);
            }
        }
    }

    pub fn refresh(&mut self) {
        let Ok(repo) = self.repo.lock() else {
            return;
        };
        if let Ok(categories) = repo.categories() {
            self.categories = categories;
        }
        if let Ok(categories) = repo.income_categories() {
            self.income_categories = categories;
        }
        if let Ok(payments) = repo.recurring_payments() {
            self.payments = payments;
        }
        if let Ok(Some(rate)) = repo.get_setting(RATE_SETTING) {
            self.exchange_rate_cents = rate.parse().unwrap_or(0);
        }
        let today = Local::now().date_naive();
        if let Ok(Some((_, rate))) = repo.rate_on(today) {
            self.exchange_rate_cents = rate;
        }
        if self.rate_input.is_empty() {
            self.rate_input = input_from_cents(self.exchange_rate_cents);
        }
        let from = NaiveDate::from_ymd_opt(2000, 1, 1).expect("valid date");
        let to = NaiveDate::from_ymd_opt(2100, 12, 31).expect("valid date");
        if let Ok(expenses) = repo.expenses_between(from, to) {
            self.expenses = expenses;
        }
        if let Ok(incomes) = repo.incomes_between(from, to) {
            self.incomes = incomes;
        }
        if self.category_id.is_none() {
            self.category_id = self.categories.first().map(|category| category.id);
        }
        if self.income_category_id.is_none() {
            self.income_category_id = self.income_categories.first().map(|category| category.id);
        }
        if self.payment_category_id.is_none() {
            self.payment_category_id = self.categories.first().map(|category| category.id);
        }
    }

    pub fn set_view(&mut self, view: View) {
        self.view = view;
        self.feedback = None;
        self.toast = None;
        self.expense_rate_input.clear();
        self.income_rate_input.clear();
    }

    pub fn clear_toast(&mut self) {
        self.toast = None;
    }

    pub fn set_entry_currency(&mut self, currency: EntryCurrency) {
        self.entry_currency = currency;
        self.feedback = None;
    }

    pub fn set_income_entry_currency(&mut self, currency: EntryCurrency) {
        self.income_entry_currency = currency;
        self.feedback = None;
    }

    /// Tasa vigente en `day`: la del día o la última anterior.
    pub fn rate_for(&self, day: NaiveDate) -> Option<(NaiveDate, i64)> {
        let historic = self
            .repo
            .lock()
            .ok()
            .and_then(|repo| repo.rate_on(day).ok().flatten());
        historic.or_else(|| {
            (self.exchange_rate_cents > 0).then(|| (day, self.exchange_rate_cents))
        })
    }

    pub fn rate_value_for(&self, day: NaiveDate) -> Option<i64> {
        self.rate_for(day).map(|(_, rate)| rate)
    }

    fn convert_for(currency: EntryCurrency, cents: i64, rate: i64) -> Option<i64> {
        match currency {
            EntryCurrency::Usd => Some(cents),
            EntryCurrency::Ves => ves_to_usd(cents, rate),
        }
    }

    pub fn to_usd(&self, cents: i64, day: NaiveDate) -> Option<i64> {
        if self.entry_currency == EntryCurrency::Usd {
            return Some(cents);
        }
        let rate = self.rate_value_for(day)?;
        Self::convert_for(self.entry_currency, cents, rate)
    }

    pub fn convert_amount(&self, cents: i64, rate: i64) -> Option<i64> {
        Self::convert_for(self.entry_currency, cents, rate)
    }

    pub fn income_to_usd(&self, cents: i64, day: NaiveDate) -> Option<i64> {
        if self.income_entry_currency == EntryCurrency::Usd {
            return Some(cents);
        }
        let rate = self.rate_value_for(day)?;
        Self::convert_for(self.income_entry_currency, cents, rate)
    }

    pub fn convert_income_amount(&self, cents: i64, rate: i64) -> Option<i64> {
        Self::convert_for(self.income_entry_currency, cents, rate)
    }

    pub fn update_rate_from_api(&mut self) {
        match platform::fetch_exchange_rate() {
            Some(rate) => {
                self.exchange_rate_cents = rate;
                self.rate_input = input_from_cents(rate);
                let today = Local::now().date_naive();
                if let Ok(repo) = self.repo.lock() {
                    let _ = repo.set_setting(RATE_SETTING, &rate.to_string());
                    let _ = repo.set_rate_for_day(today, rate, "dolarapi");
                }
                self.feedback = Some(format!("Tasa actualizada: {}", format_rate(rate)));
            }
            None => {
                self.feedback = Some("No se pudo consultar la tasa; escríbela manualmente".into());
            }
        }
    }

    pub fn save_rate(&mut self) {
        let Some(cents) = parse_cents(&self.rate_input) else {
            self.feedback = Some("Escribe una tasa válida, por ejemplo 857,89".into());
            return;
        };
        self.exchange_rate_cents = cents;
        self.rate_input = input_from_cents(cents);
        let today = Local::now().date_naive();
        if let Ok(repo) = self.repo.lock() {
            let _ = repo.set_setting(RATE_SETTING, &cents.to_string());
            let _ = repo.set_rate_for_day(today, cents, "manual");
        }
        self.feedback = Some(format!("Tasa guardada: {}", format_rate(cents)));
    }

    pub fn attach_evidence(&mut self, name: String, bytes: &[u8]) {
        match platform::save_evidence(bytes, &name) {
            Ok((path, name)) => {
                self.evidence_path = Some(path);
                self.evidence_name = Some(name);
                self.feedback = Some("Evidencia adjuntada".into());
            }
            Err(error) => self.feedback = Some(format!("No se pudo guardar la imagen: {error}")),
        }
    }

    pub fn clear_evidence(&mut self) {
        self.evidence_path = None;
        self.evidence_name = None;
    }

    pub fn add_expense(&mut self) {
        let Some(cents) = parse_cents(&self.amount) else {
            self.feedback = Some("Escribe un importe válido, por ejemplo 12.50".into());
            return;
        };
        let day = NaiveDate::parse_from_str(&self.expense_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Local::now().date_naive());
        let raw_rate = self.expense_rate_input.trim();
        let manual_rate = if raw_rate.is_empty() {
            None
        } else {
            match parse_cents(raw_rate) {
                Some(rate) => Some(rate),
                None => {
                    self.feedback = Some("Escribe una tasa válida, por ejemplo 857,89".into());
                    return;
                }
            }
        };
        let Some(usd_cents) = (match manual_rate {
            Some(rate) => self.convert_amount(cents, rate),
            None => self.to_usd(cents, day),
        }) else {
            self.feedback = Some("Configura la tasa del dólar para convertir bolívares".into());
            return;
        };
        let Some(category_id) = self.category_id else {
            self.feedback = Some("Elige una categoría".into());
            return;
        };
        let currency = match Currency::new(DEFAULT_CURRENCY) {
            Ok(currency) => currency,
            Err(error) => {
                self.feedback = Some(error.to_string());
                return;
            }
        };
        let amount = match Money::new(usd_cents, currency) {
            Ok(amount) => amount,
            Err(error) => {
                self.feedback = Some(error.to_string());
                return;
            }
        };
        let (rate_day, rate_cents) = match manual_rate {
            Some(rate) => (Some(day), Some(rate)),
            None => self.rate_for(day).unzip(),
        };
        let new = NewExpense {
            amount,
            category_id,
            note: (!self.note.trim().is_empty()).then(|| self.note.trim().to_owned()),
            spent_at: local_moment_at(day),
            local_date: day,
            evidence_path: self.evidence_path.clone(),
            rate_cents,
            rate_day: rate_day.map(|value| value.to_string()),
        };

        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.insert_expense(&new).map_err(|_| ()));

        match result {
            Ok(_) => {
                if let Some(rate) = manual_rate {
                    if let Ok(repo) = self.repo.lock() {
                        let _ = repo.set_rate_for_day(day, rate, "manual_gasto");
                    }
                }
                self.amount.clear();
                self.note.clear();
                self.expense_rate_input.clear();
                self.clear_evidence();
                self.refresh();
                self.view = View::Home;
                self.feedback = None;
                self.toast = Some("Gasto guardado".into());
            }
            Err(()) => self.feedback = Some("No se pudo guardar el gasto".into()),
        }
    }

    pub fn remove_expense(&mut self, id: ExpenseId) {
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.delete_expense(id).map_err(|_| ()));
        if result.is_ok() {
            self.refresh();
        }
    }

    pub fn add_income(&mut self) {
        let Some(cents) = parse_cents(&self.income_amount) else {
            self.feedback = Some("Escribe un importe válido, por ejemplo 12.50".into());
            return;
        };
        let day = NaiveDate::parse_from_str(&self.income_date, "%Y-%m-%d")
            .unwrap_or_else(|_| Local::now().date_naive());
        let raw_rate = self.income_rate_input.trim();
        let manual_rate = if raw_rate.is_empty() {
            None
        } else {
            match parse_cents(raw_rate) {
                Some(rate) => Some(rate),
                None => {
                    self.feedback = Some("Escribe una tasa válida, por ejemplo 857,89".into());
                    return;
                }
            }
        };
        let Some(usd_cents) = (match manual_rate {
            Some(rate) => self.convert_income_amount(cents, rate),
            None => self.income_to_usd(cents, day),
        }) else {
            self.feedback = Some("Configura la tasa del dólar para convertir bolívares".into());
            return;
        };
        let Some(category_id) = self.income_category_id else {
            self.feedback = Some("Elige una categoría de ingreso".into());
            return;
        };
        let Ok(currency) = Currency::new(DEFAULT_CURRENCY) else {
            return;
        };
        let Ok(amount) = Money::new(usd_cents, currency) else {
            return;
        };
        let (rate_day, rate_cents) = match manual_rate {
            Some(rate) => (Some(day), Some(rate)),
            None => self.rate_for(day).unzip(),
        };
        let new = NewIncome {
            amount,
            category_id,
            note: (!self.income_note.trim().is_empty())
                .then(|| self.income_note.trim().to_owned()),
            received_at: local_moment_at(day),
            local_date: day,
            rate_cents,
            rate_day: rate_day.map(|value| value.to_string()),
        };

        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.insert_income(&new).map_err(|_| ()));

        match result {
            Ok(_) => {
                if let Some(rate) = manual_rate {
                    if let Ok(repo) = self.repo.lock() {
                        let _ = repo.set_rate_for_day(day, rate, "manual_ingreso");
                    }
                }
                self.income_amount.clear();
                self.income_note.clear();
                self.income_rate_input.clear();
                self.refresh();
                self.view = View::Home;
                self.feedback = None;
                self.toast = Some("Ingreso guardado".into());
            }
            Err(()) => self.feedback = Some("No se pudo guardar el ingreso".into()),
        }
    }

    pub fn remove_income(&mut self, id: IncomeId) {
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.delete_income(id).map_err(|_| ()));
        if result.is_ok() {
            self.refresh();
        }
    }

    pub fn add_category(&mut self) {
        let name = self.new_category_name.trim().to_owned();
        if name.is_empty() {
            self.feedback = Some("Escribe un nombre para la categoría".into());
            return;
        }
        let color = if self.new_category_color.trim().is_empty() {
            "#4f8cff".to_owned()
        } else {
            self.new_category_color.trim().to_owned()
        };
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.add_category(&name, None, &color).map_err(|_| ()));
        match result {
            Ok(_) => {
                self.new_category_name.clear();
                self.feedback = Some("Categoría creada".into());
                self.refresh();
            }
            Err(()) => self.feedback = Some("No se pudo crear la categoría".into()),
        }
    }

    pub fn add_income_category(&mut self) {
        let name = self.new_income_category_name.trim().to_owned();
        if name.is_empty() {
            self.feedback = Some("Escribe un nombre para la categoría de ingreso".into());
            return;
        }
        let color = if self.new_income_category_color.trim().is_empty() {
            "#2dd4a7".to_owned()
        } else {
            self.new_income_category_color.trim().to_owned()
        };
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.add_income_category(&name, None, &color).map_err(|_| ()));
        match result {
            Ok(_) => {
                self.new_income_category_name.clear();
                self.feedback = Some("Categoría de ingreso creada".into());
                self.refresh();
            }
            Err(()) => self.feedback = Some("No se pudo crear la categoría de ingreso".into()),
        }
    }

    pub fn add_payment(&mut self) {
        let name = self.payment_name.trim().to_owned();
        if name.is_empty() {
            self.feedback = Some("Escribe el nombre del pago, por ejemplo Luz".into());
            return;
        }
        let Some(cents) = parse_cents(&self.payment_amount) else {
            self.feedback = Some("Escribe un importe válido".into());
            return;
        };
        let Some(usd_cents) = self.to_usd(cents, Local::now().date_naive()) else {
            self.feedback = Some("Configura la tasa del dólar para convertir bolívares".into());
            return;
        };
        let Ok(day) = self.payment_day.trim().parse::<u8>() else {
            self.feedback = Some("Escribe el día de vencimiento (1-31)".into());
            return;
        };
        let Some(category_id) = self.payment_category_id else {
            self.feedback = Some("Elige una categoría".into());
            return;
        };
        let Ok(currency) = Currency::new(DEFAULT_CURRENCY) else {
            return;
        };
        let Ok(amount) = Money::new(usd_cents, currency) else {
            return;
        };
        let payment = NewRecurringPayment {
            name,
            amount,
            category_id,
            due_day: day,
            note: None,
        };
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.add_recurring_payment(&payment).map_err(|_| ()));
        match result {
            Ok(_) => {
                self.payment_name.clear();
                self.payment_amount.clear();
                self.payment_day.clear();
                self.feedback = Some("Pago mensual creado".into());
                self.refresh();
            }
            Err(()) => self.feedback = Some("No se pudo crear el pago".into()),
        }
    }

    pub fn pay_payment(&mut self, id: RecurringPaymentId) {
        let Some(payment) = self.payments.iter().find(|payment| payment.id == id).cloned() else {
            return;
        };
        let now = Local::now();
        let (rate_day, rate_cents) = self.rate_for(now.date_naive()).unzip();
        let new = NewExpense {
            amount: payment.amount,
            category_id: payment.category_id,
            note: Some(format!("{} (pago mensual)", payment.name)),
            spent_at: now.fixed_offset(),
            local_date: now.date_naive(),
            evidence_path: None,
            rate_cents,
            rate_day: rate_day.map(|value| value.to_string()),
        };
        let result = self.repo.lock().map_err(|_| ()).and_then(|repo| {
            repo.pay_recurring_payment(id, &new).map_err(|_| ())
        });
        match result {
            Ok(()) => {
                self.feedback = Some(format!("{} pagado y registrado", payment.name));
                self.refresh();
            }
            Err(()) => self.feedback = Some("No se pudo registrar el pago".into()),
        }
    }

    pub fn remove_payment(&mut self, id: RecurringPaymentId) {
        let result = self.repo.lock().map_err(|_| ()).and_then(|repo| {
            repo.delete_recurring_payment(id).map_err(|_| ())
        });
        if result.is_ok() {
            self.refresh();
        }
    }

    pub fn report(&self) -> Option<Report> {
        build_report(&self.expenses, &self.categories).ok()
    }

    pub fn income_report(&self) -> Option<Report> {
        build_income_report(&self.incomes, &self.income_categories).ok()
    }

    pub fn month_total(&self) -> i64 {
        let now = Local::now();
        let (year, month) = (now.year(), now.month());
        self.expenses
            .iter()
            .filter(|expense| {
                expense.local_date.year() == year && expense.local_date.month() == month
            })
            .map(|expense| expense.amount.cents())
            .sum()
    }

    pub fn month_income_total(&self) -> i64 {
        let now = Local::now();
        let (year, month) = (now.year(), now.month());
        self.incomes
            .iter()
            .filter(|income| {
                income.local_date.year() == year && income.local_date.month() == month
            })
            .map(|income| income.amount.cents())
            .sum()
    }

    pub fn month_balance(&self) -> i64 {
        self.month_income_total() - self.month_total()
    }

    pub fn recent(&self, limit: usize) -> Vec<Expense> {
        self.expenses.iter().take(limit).cloned().collect()
    }

    pub fn income_recent(&self, limit: usize) -> Vec<Income> {
        self.incomes.iter().take(limit).cloned().collect()
    }

    pub fn category_of(&self, id: CategoryId) -> Option<&Category> {
        self.categories.iter().find(|category| category.id == id)
    }

    pub fn income_category_of(&self, id: CategoryId) -> Option<&Category> {
        self.income_categories.iter().find(|category| category.id == id)
    }

    pub fn due_payments(&self) -> Vec<(RecurringPayment, PaymentStatus)> {
        let today = Local::now().date_naive();
        let mut items: Vec<_> = self
            .payments
            .iter()
            .filter_map(|payment| {
                let status = payment.status(today);
                matches!(status, PaymentStatus::Overdue(_) | PaymentStatus::DueSoon(_))
                    .then(|| (payment.clone(), status))
            })
            .collect();
        items.sort_by_key(|(payment, _)| payment.due_date(today.year(), today.month()));
        items
    }
}

/// Los dígitos del input son centavos, igual que en el POS de inventorySystem.
pub fn parse_cents(input: &str) -> Option<i64> {
    let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let cents: i64 = digits.parse().ok()?;
    (cents > 0).then_some(cents)
}

pub fn input_from_cents(cents: i64) -> String {
    format_cents_input(&cents.to_string(), "VES")
}

pub fn payment_status_text(status: PaymentStatus) -> String {
    match status {
        PaymentStatus::Paid => "Pagado este mes".into(),
        PaymentStatus::Pending(days) => format!("Vence en {days} días"),
        PaymentStatus::DueSoon(0) => "Vence hoy".into(),
        PaymentStatus::DueSoon(days) => format!("Vence en {days} días"),
        PaymentStatus::Overdue(days) => format!("Vencido hace {days} días"),
    }
}

pub fn money_usd(cents: i64) -> String {
    format_money(cents, &Currency::new("USD").expect("USD válida"))
}

pub fn money_ves(usd_cents: i64, rate_cents: i64) -> Option<String> {
    usd_to_ves(usd_cents, rate_cents).map(|cents| format_money(cents, &Currency::new("VES").expect("VES válida")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_amounts_as_digit_centavos() {
        assert_eq!(parse_cents("1250"), Some(1250));
        assert_eq!(parse_cents("1,250"), Some(1250));
        assert_eq!(parse_cents("1.250.00"), Some(125000));
        assert_eq!(parse_cents("0"), None);
        assert_eq!(parse_cents("abc"), None);
        assert_eq!(parse_cents(""), None);
    }

    #[test]
    fn formats_money_using_each_currency_convention() {
        assert_eq!(money_usd(123456), "USD 1,234.56");
        assert_eq!(money_ves(100, 85789).as_deref(), Some("Bs.S 857,89"));
        assert_eq!(money_ves(1000, 0), None);
    }

    #[test]
    fn rate_input_round_trips_without_losing_zeros() {
        assert_eq!(input_from_cents(85789), "857,89");
        assert_eq!(input_from_cents(85705), "857,05");
        assert_eq!(parse_cents(&input_from_cents(85789)), Some(85789));
        assert_eq!(parse_cents(&input_from_cents(85705)), Some(85705));
    }

    #[test]
    fn manual_expense_rate_overrides_history_and_is_saved() {
        let repo = finance_store::SqliteRepository::open_in_memory().expect("repo en memoria");
        let mut state = AppState::new(repo);
        let day = NaiveDate::from_ymd_opt(2026, 10, 15).expect("fecha válida");
        state
            .repo
            .lock()
            .unwrap()
            .set_rate_for_day(day, 10000, "test")
            .unwrap();

        state.expense_date = day.to_string();
        state.entry_currency = EntryCurrency::Ves;
        state.amount = "10000".into();
        state.expense_rate_input = "120,00".into();
        state.add_expense();

        assert!(state.toast.is_some(), "feedback: {:?}", state.feedback);
        let repo = state.repo.lock().unwrap();
        assert_eq!(repo.rate_on(day).unwrap().unwrap().1, 12000);
        let expenses = repo.expenses_between(day, day).unwrap();
        assert_eq!(expenses.len(), 1);
        assert_eq!(expenses[0].rate_cents, Some(12000));
    }

    #[test]
    fn adds_an_income_and_updates_the_balance() {
        let repo = finance_store::SqliteRepository::open_in_memory().expect("repo en memoria");
        let mut state = AppState::new(repo);

        state.income_amount = "25000000".into();
        state.income_note = "sueldo".into();
        state.add_income();

        assert!(state.toast.is_some(), "feedback: {:?}", state.feedback);
        assert_eq!(state.view, View::Home);
        assert_eq!(state.month_income_total(), 25000000);
        assert_eq!(state.month_balance(), 25000000);
        assert_eq!(state.incomes.len(), 1);
        assert_eq!(state.income_recent(1)[0].note.as_deref(), Some("sueldo"));

        let id = state.incomes[0].id;
        state.remove_income(id);
        assert!(state.incomes.is_empty());
        assert_eq!(state.month_balance(), 0);
    }
}
