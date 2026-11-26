use std::{
    sync::{Arc, Mutex},
    time::Duration as StdDuration,
};

use chrono::{DateTime, Datelike, Duration, FixedOffset, Local, NaiveDate, Utc};
use finance_core::{
    ActivityEntry, ActivityKind, Backup, Category, CategoryId, Currency, Expense, ExpenseId,
    Income, IncomeId, Money, MovementMethod, NewExpense, NewIncome, NewRecurringPayment,
    PaymentMethod, PaymentStatus, RecurringKind, RecurringPayment, RecurringPaymentId, Report,
    build_income_report, build_report, format_cents_input, format_money, format_rate, usd_to_ves,
    ves_to_usd,
};
use finance_store::SqliteRepository;
use sha2::{Digest, Sha256};

use crate::{
    bridge,
    platform::{self, UpdateProgress, UpdateSource},
};

pub const DEFAULT_CURRENCY: &str = "USD";
const RATE_SETTING: &str = "exchange_rate_cents";
const UPDATE_URL_SETTING: &str = "update_url";
const PIN_HASH_SETTING: &str = "app_pin_hash";
const TEXT_SCALE_SETTING: &str = "text_scale";
const THEME_SETTING: &str = "theme";
const BIOMETRIC_SETTING: &str = "biometric_enabled";
const AUTO_BACKUP_INTERVAL_SETTING: &str = "auto_backup_hours";
const LAST_AUTO_BACKUP_SETTING: &str = "last_auto_backup_at";

pub fn sha256_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Formato venezolano: dd/mm/aaaa.
pub fn format_date(day: NaiveDate) -> String {
    day.format("%d/%m/%Y").to_string()
}

fn format_timestamp(seconds: i64) -> String {
    DateTime::from_timestamp(seconds, 0)
        .map(|moment| {
            moment
                .with_timezone(&Local)
                .format("%d/%m/%Y %H:%M")
                .to_string()
        })
        .unwrap_or_else(|| "desconocida".to_owned())
}

pub fn format_activity_moment(moment: DateTime<Utc>) -> String {
    let local = moment.with_timezone(&Local);
    format!(
        "{} {}",
        format_date(local.date_naive()),
        local.format("%H:%M")
    )
}

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
    Update,
    Search,
    Activity,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaymentKind {
    Monthly,
    OneTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingDelete {
    Expense(ExpenseId),
    Income(IncomeId),
    Payment(RecurringPaymentId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchKind {
    All,
    Expense,
    Income,
    Payment,
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
    pub expense_method: MovementMethod,
    pub income_method: MovementMethod,
    pub pending_delete: Option<PendingDelete>,
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
    pub payment_kind: PaymentKind,
    pub payment_date: String,
    pub payment_method: PaymentMethod,
    pub payment_note: String,
    pub cashea_initial: String,
    pub cashea_installments: String,
    pub cashea_installment: String,
    pub payment_category_id: Option<CategoryId>,
    pub feedback: Option<String>,
    pub toast: Option<String>,
    pub update_url: String,
    pub update_status: Option<String>,
    pub editing_expense: Option<ExpenseId>,
    pub editing_income: Option<IncomeId>,
    pub budgets: Vec<(CategoryId, i64)>,
    pub budget_category_id: Option<CategoryId>,
    pub budget_input: String,
    pub search_query: String,
    pub search_category_id: Option<CategoryId>,
    pub search_kind: SearchKind,
    pub search_from: String,
    pub search_to: String,
    pub import_input: String,
    pub pin_hash: Option<String>,
    pub pin_input: String,
    pub setup_pin: String,
    pub confirm_pin: String,
    pub pin_error: Option<String>,
    pub locked: bool,
    pub text_scale: String,
    pub theme: String,
    pub biometric_enabled: bool,
    pub auto_backup_hours: i64,
    pub evidence_viewer: Option<String>,
    pub rate_history: Vec<(NaiveDate, i64, String, String)>,
    pub activity: Vec<ActivityEntry>,
    pub recurring_kind: RecurringKind,
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
            expense_method: MovementMethod::Other,
            income_method: MovementMethod::Other,
            pending_delete: None,
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
            payment_kind: PaymentKind::Monthly,
            payment_date: Local::now().date_naive().to_string(),
            payment_method: PaymentMethod::Other,
            payment_note: String::new(),
            cashea_initial: String::new(),
            cashea_installments: "3".to_owned(),
            cashea_installment: String::new(),
            payment_category_id: None,
            feedback: None,
            toast: None,
            update_url: String::new(),
            update_status: None,
            editing_expense: None,
            editing_income: None,
            budgets: Vec::new(),
            budget_category_id: None,
            budget_input: String::new(),
            search_query: String::new(),
            search_category_id: None,
            search_kind: SearchKind::All,
            search_from: String::new(),
            search_to: String::new(),
            import_input: String::new(),
            pin_hash: None,
            pin_input: String::new(),
            setup_pin: String::new(),
            confirm_pin: String::new(),
            pin_error: None,
            locked: false,
            text_scale: "normal".to_owned(),
            theme: "dark".to_owned(),
            biometric_enabled: false,
            auto_backup_hours: 12,
            evidence_viewer: None,
            rate_history: Vec::new(),
            activity: Vec::new(),
            recurring_kind: RecurringKind::Payment,
        };
        if let Ok(repo) = state.repo.lock() {
            state.pin_hash = repo.get_setting(PIN_HASH_SETTING).ok().flatten();
            state.text_scale = repo
                .get_setting(TEXT_SCALE_SETTING)
                .ok()
                .flatten()
                .unwrap_or_else(|| "normal".to_owned());
            state.theme = repo
                .get_setting(THEME_SETTING)
                .ok()
                .flatten()
                .unwrap_or_else(|| "dark".to_owned());
            state.biometric_enabled = repo
                .get_setting(BIOMETRIC_SETTING)
                .ok()
                .flatten()
                .as_deref()
                == Some("1");
            state.auto_backup_hours = repo
                .get_setting(AUTO_BACKUP_INTERVAL_SETTING)
                .ok()
                .flatten()
                .and_then(|value| value.parse().ok())
                .unwrap_or(12);
        }
        state.locked = state.pin_hash.is_some();
        state.seed_categories();
        state.seed_income_categories();
        state.refresh();
        state
    }

    fn seed_categories(&self) {
        let seeds = [
            ("Comida", "🍔", "#e76f51"),
            ("Transporte", "🚗", "#2a9d8f"),
            ("Hogar", "🏠", "#e9c46a"),
            ("Familia", "👨‍👩‍👧", "#c77dff"),
            ("Ocio", "🎬", "#8e7dbe"),
            ("Salud", "💊", "#57a6a1"),
            ("Otros", "📦", "#888888"),
        ];
        let Ok(repo) = self.repo.lock() else {
            return;
        };
        for (name, icon, color) in seeds {
            let _ = repo.ensure_category(name, Some(icon), color);
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
        if repo
            .income_categories()
            .map(|c| c.is_empty())
            .unwrap_or(false)
        {
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
        if let Ok(budgets) = repo.budgets() {
            self.budgets = budgets;
        }
        if let Ok(history) = repo.rate_details() {
            self.rate_history = history;
        }
        if let Ok(activity) = repo.activities(300) {
            self.activity = activity;
        }
        if self.budget_category_id.is_none() {
            self.budget_category_id = self.categories.first().map(|category| category.id);
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
        if self.update_url.is_empty()
            && let Ok(Some(url)) = repo.get_setting(UPDATE_URL_SETTING)
        {
            self.update_url = url;
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
        self.editing_expense = None;
        self.editing_income = None;
        self.evidence_viewer = None;
        self.pending_delete = None;
    }

    /// Texto del aviso de confirmación; `None` cuando no hay borrado pendiente.
    pub fn pending_delete_message(&self) -> Option<String> {
        let target = self.pending_delete?;
        let (label, detail) = match target {
            PendingDelete::Expense(id) => (
                "el gasto",
                self.expenses
                    .iter()
                    .find(|item| item.id == id)
                    .and_then(|item| item.note.clone())
                    .unwrap_or_else(|| "seleccionado".into()),
            ),
            PendingDelete::Income(id) => (
                "el ingreso",
                self.incomes
                    .iter()
                    .find(|item| item.id == id)
                    .and_then(|item| item.note.clone())
                    .unwrap_or_else(|| "seleccionado".into()),
            ),
            PendingDelete::Payment(id) => (
                "el pago",
                self.payments
                    .iter()
                    .find(|item| item.id == id)
                    .map(|item| item.name.clone())
                    .unwrap_or_else(|| "seleccionado".into()),
            ),
        };
        Some(format!(
            "¿Eliminar {label} «{detail}»? Esta acción no se puede deshacer."
        ))
    }

    pub fn clear_toast(&mut self) {
        self.toast = None;
    }

    fn log_activity(
        &self,
        kind: ActivityKind,
        entity_id: i64,
        summary: impl Into<String>,
        amount_cents: i64,
    ) {
        if let Ok(repo) = self.repo.lock() {
            let _ = repo.insert_activity(kind, entity_id, &summary.into(), amount_cents);
        }
    }

    pub fn set_update_url(&mut self, value: String) {
        self.update_url = value;
        self.update_status = None;
    }

    /// El importe de una compra Cashea es la inicial más todas las cuotas.
    pub fn sync_cashea_total(&mut self) {
        let initial = self.cashea_initial.trim().parse::<i64>().unwrap_or(0);
        let count = self
            .cashea_installments
            .trim()
            .parse::<i64>()
            .unwrap_or(0)
            .clamp(0, 12);
        let installment = self.cashea_installment.trim().parse::<i64>().unwrap_or(0);
        self.payment_amount = (initial + count * installment).to_string();
    }

    pub fn prepare_update(&mut self) -> Result<UpdateSource, String> {
        let base = self.update_url.trim().trim_end_matches('/').to_owned();
        if let Ok(repo) = self.repo.lock() {
            let _ = repo.set_setting(UPDATE_URL_SETTING, &base);
        }
        if base.is_empty() {
            return Ok(UpdateSource::GitHub);
        }
        if !(base.starts_with("http://") || base.starts_with("https://")) {
            return Err("Escribe la URL completa, por ejemplo http://192.168.0.10:8000".into());
        }
        Ok(UpdateSource::Local(base))
    }

    pub fn update_app(&mut self, progress: &(dyn Fn(UpdateProgress) + Send + Sync)) {
        let source = match self.prepare_update() {
            Ok(source) => source,
            Err(message) => {
                self.update_status = Some(message);
                return;
            }
        };
        self.update_status = Some("Buscando actualización…".into());
        self.update_status = Some(match platform::install_update(&source, progress) {
            Ok(message) | Err(message) => message,
        });
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
        historic
            .or_else(|| (self.exchange_rate_cents > 0).then_some((day, self.exchange_rate_cents)))
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
            Some((rate, source)) => {
                self.exchange_rate_cents = rate;
                self.rate_input = input_from_cents(rate);
                let today = Local::now().date_naive();
                if let Ok(repo) = self.repo.lock() {
                    let _ = repo.set_setting(RATE_SETTING, &rate.to_string());
                    let _ = repo.set_rate_for_day(today, rate, source);
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
        let (original_amount_cents, original_currency) =
            if self.entry_currency == EntryCurrency::Ves {
                (Some(cents), Some("VES".to_owned()))
            } else {
                (None, None)
            };
        let new = NewExpense {
            amount: amount.clone(),
            category_id,
            note: (!self.note.trim().is_empty()).then(|| self.note.trim().to_owned()),
            spent_at: local_moment_at(day),
            local_date: day,
            evidence_path: self.evidence_path.clone(),
            rate_cents,
            rate_day: rate_day.map(|value| value.to_string()),
            method: self.expense_method,
            original_amount_cents,
            original_currency,
        };

        let editing = self.editing_expense;
        let result = if let Some(id) = editing {
            match self.expenses.iter().find(|item| item.id == id).cloned() {
                Some(mut expense) => {
                    expense.amount = amount.clone();
                    expense.category_id = category_id;
                    expense.note = new.note.clone();
                    expense.spent_at = new.spent_at;
                    expense.local_date = new.local_date;
                    expense.evidence_path = new.evidence_path.clone();
                    expense.rate_cents = new.rate_cents;
                    expense.rate_day = new.rate_day.clone();
                    expense.method = new.method;
                    expense.original_amount_cents = new.original_amount_cents;
                    expense.original_currency = new.original_currency.clone();
                    self.repo
                        .lock()
                        .map_err(|_| ())
                        .and_then(|repo| repo.update_expense(&expense).map(|_| id).map_err(|_| ()))
                }
                None => Err(()),
            }
        } else {
            self.repo
                .lock()
                .map_err(|_| ())
                .and_then(|repo| repo.insert_expense(&new).map_err(|_| ()))
        };

        match result {
            Ok(id) => {
                self.log_activity(
                    if editing.is_some() {
                        ActivityKind::ExpenseUpdated
                    } else {
                        ActivityKind::ExpenseCreated
                    },
                    id.get(),
                    new.note.clone().unwrap_or_else(|| "Gasto".into()),
                    usd_cents,
                );
                if let Some(rate) = manual_rate
                    && let Ok(repo) = self.repo.lock()
                {
                    let _ = repo.set_rate_for_day(day, rate, "manual_gasto");
                }
                self.amount.clear();
                self.note.clear();
                self.expense_rate_input.clear();
                self.clear_evidence();
                self.editing_expense = None;
                self.refresh();
                self.view = View::Home;
                self.feedback = None;
                self.toast = Some(if editing.is_some() {
                    "Gasto actualizado".into()
                } else {
                    "Gasto guardado".into()
                });
            }
            Err(()) => self.feedback = Some("No se pudo guardar el gasto".into()),
        }
    }

    pub fn request_remove_expense(&mut self, id: ExpenseId) {
        self.pending_delete = Some(PendingDelete::Expense(id));
    }

    pub fn remove_expense(&mut self, id: ExpenseId) -> bool {
        let expense = self.expenses.iter().find(|item| item.id == id).cloned();
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.delete_expense(id).map_err(|_| ()));
        if result.is_err() {
            return false;
        }
        if let Some(expense) = expense {
            if let Some(path) = expense.evidence_path {
                platform::delete_evidence(&path);
            }
            self.log_activity(
                ActivityKind::ExpenseDeleted,
                id.get(),
                expense.note.unwrap_or_else(|| "Gasto".into()),
                expense.amount.cents(),
            );
        }
        self.refresh();
        true
    }

    /// Una devolución conserva el gasto y crea el ingreso inverso, para que el
    /// historial no pierda información y el balance vuelva a cuadrar.
    pub fn refund_expense(&mut self, id: ExpenseId) {
        let Some(expense) = self.expenses.iter().find(|item| item.id == id).cloned() else {
            return;
        };
        let Some(category_id) = self
            .income_categories
            .iter()
            .find(|category| category.name.trim().eq_ignore_ascii_case("otros"))
            .or_else(|| self.income_categories.first())
            .map(|category| category.id)
        else {
            self.feedback = Some("No hay categoría de ingreso para la devolución".into());
            return;
        };
        let original = expense
            .note
            .as_deref()
            .unwrap_or("")
            .trim()
            .chars()
            .take(185)
            .collect::<String>();
        let detail = if original.is_empty() {
            "gasto sin nota".to_owned()
        } else {
            original
        };
        let refund_amount = expense.amount.cents();
        let new = NewIncome {
            amount: expense.amount,
            category_id,
            note: Some(format!("Devolución de {detail}")),
            received_at: expense.spent_at,
            local_date: expense.local_date,
            rate_cents: expense.rate_cents,
            rate_day: expense.rate_day.clone(),
            method: expense.method,
            original_amount_cents: expense.original_amount_cents,
            original_currency: expense.original_currency.clone(),
        };
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.insert_income(&new).map_err(|_| ()));
        match result {
            Ok(_) => {
                self.log_activity(
                    ActivityKind::ExpenseRefunded,
                    expense.id.get(),
                    format!("Devolución de {detail}"),
                    refund_amount,
                );
                self.refresh();
                self.toast = Some("Devolución registrada como ingreso".into());
            }
            Err(()) => self.feedback = Some("No se pudo registrar la devolución".into()),
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
        let (original_amount_cents, original_currency) =
            if self.income_entry_currency == EntryCurrency::Ves {
                (Some(cents), Some("VES".to_owned()))
            } else {
                (None, None)
            };
        let new = NewIncome {
            amount: amount.clone(),
            category_id,
            note: (!self.income_note.trim().is_empty()).then(|| self.income_note.trim().to_owned()),
            received_at: local_moment_at(day),
            local_date: day,
            rate_cents,
            rate_day: rate_day.map(|value| value.to_string()),
            method: self.income_method,
            original_amount_cents,
            original_currency,
        };

        let editing = self.editing_income;
        let result = if let Some(id) = editing {
            match self.incomes.iter().find(|item| item.id == id).cloned() {
                Some(mut income) => {
                    income.amount = amount.clone();
                    income.category_id = category_id;
                    income.note = new.note.clone();
                    income.received_at = new.received_at;
                    income.local_date = new.local_date;
                    income.rate_cents = new.rate_cents;
                    income.rate_day = new.rate_day.clone();
                    income.method = new.method;
                    income.original_amount_cents = new.original_amount_cents;
                    income.original_currency = new.original_currency.clone();
                    self.repo
                        .lock()
                        .map_err(|_| ())
                        .and_then(|repo| repo.update_income(&income).map(|_| id).map_err(|_| ()))
                }
                None => Err(()),
            }
        } else {
            self.repo
                .lock()
                .map_err(|_| ())
                .and_then(|repo| repo.insert_income(&new).map_err(|_| ()))
        };

        match result {
            Ok(id) => {
                self.log_activity(
                    if editing.is_some() {
                        ActivityKind::IncomeUpdated
                    } else {
                        ActivityKind::IncomeCreated
                    },
                    id.get(),
                    new.note.clone().unwrap_or_else(|| "Ingreso".into()),
                    usd_cents,
                );
                if let Some(rate) = manual_rate
                    && let Ok(repo) = self.repo.lock()
                {
                    let _ = repo.set_rate_for_day(day, rate, "manual_ingreso");
                }
                self.income_amount.clear();
                self.income_note.clear();
                self.income_rate_input.clear();
                self.editing_income = None;
                self.refresh();
                self.view = View::Home;
                self.feedback = None;
                self.toast = Some(if editing.is_some() {
                    "Ingreso actualizado".into()
                } else {
                    "Ingreso guardado".into()
                });
            }
            Err(()) => self.feedback = Some("No se pudo guardar el ingreso".into()),
        }
    }

    pub fn request_remove_income(&mut self, id: IncomeId) {
        self.pending_delete = Some(PendingDelete::Income(id));
    }

    pub fn remove_income(&mut self, id: IncomeId) -> bool {
        let income = self.incomes.iter().find(|item| item.id == id).cloned();
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.delete_income(id).map_err(|_| ()));
        if result.is_err() {
            return false;
        }
        if let Some(income) = income {
            self.log_activity(
                ActivityKind::IncomeDeleted,
                id.get(),
                income.note.unwrap_or_else(|| "Ingreso".into()),
                income.amount.cents(),
            );
        }
        self.refresh();
        true
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
        let result = self.repo.lock().map_err(|_| ()).and_then(|repo| {
            repo.add_income_category(&name, None, &color)
                .map_err(|_| ())
        });
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
        let Some(category_id) = self.payment_category_id else {
            self.feedback = Some("Elige una categoría".into());
            return;
        };
        if self.payment_method == PaymentMethod::Cashea {
            return self.add_cashea_payment(name, category_id);
        }
        let Some(cents) = parse_cents(&self.payment_amount) else {
            self.feedback = Some("Escribe un importe válido".into());
            return;
        };
        let Some(usd_cents) = self.to_usd(cents, Local::now().date_naive()) else {
            self.feedback = Some("Configura la tasa del dólar para convertir bolívares".into());
            return;
        };
        let (due_day, due_date) = match self.payment_kind {
            PaymentKind::Monthly => {
                let Ok(day) = self.payment_day.trim().parse::<u8>() else {
                    self.feedback = Some("Escribe el día de vencimiento (1-31)".into());
                    return;
                };
                if !(1..=31).contains(&day) {
                    self.feedback = Some("El día debe estar entre 1 y 31".into());
                    return;
                }
                (day, None)
            }
            PaymentKind::OneTime => {
                let Ok(date) = NaiveDate::parse_from_str(&self.payment_date, "%Y-%m-%d") else {
                    self.feedback = Some("Elige la fecha del pago".into());
                    return;
                };
                (date.day() as u8, Some(date))
            }
        };
        let Ok(currency) = Currency::new(DEFAULT_CURRENCY) else {
            return;
        };
        let Ok(amount) = Money::new(usd_cents, currency) else {
            return;
        };
        let note = self.payment_note.trim();
        let note = (!note.is_empty()).then(|| note.to_owned());
        let summary = match &note {
            Some(note) => format!("{name} · {note}"),
            None => name.clone(),
        };
        let payment = NewRecurringPayment {
            kind: self.recurring_kind,
            name,
            amount,
            category_id,
            due_day,
            due_date,
            method: self.payment_method,
            note,
        };
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.add_recurring_payment(&payment).map_err(|_| ()));
        match result {
            Ok(id) => {
                self.log_activity(ActivityKind::PaymentCreated, id.get(), summary, usd_cents);
                self.payment_name.clear();
                self.payment_amount.clear();
                self.payment_day.clear();
                self.payment_note.clear();
                self.feedback = Some(match self.payment_kind {
                    PaymentKind::Monthly => "Pago mensual creado".into(),
                    PaymentKind::OneTime => "Pago en fecha única creado".into(),
                });
                self.refresh();
            }
            Err(()) => self.feedback = Some("No se pudo crear el pago".into()),
        }
    }

    pub fn add_cashea_payment(&mut self, name: String, category_id: CategoryId) {
        let Some(initial) = parse_cents(&self.cashea_initial) else {
            self.feedback = Some("Escribe el pago inicial".into());
            return;
        };
        let Ok(count) = self.cashea_installments.trim().parse::<u8>() else {
            self.feedback = Some("Escribe cuántas cuotas (1-12)".into());
            return;
        };
        if !(1..=12).contains(&count) {
            self.feedback = Some("Las cuotas deben estar entre 1 y 12".into());
            return;
        }
        let Some(installment) = parse_cents(&self.cashea_installment) else {
            self.feedback = Some("Escribe el monto de cada cuota".into());
            return;
        };
        let today = Local::now().date_naive();
        let Some((_, initial_rate)) = self.rate_for(today) else {
            self.feedback = Some("Configura la tasa del dólar para convertir bolívares".into());
            return;
        };
        let Ok(currency) = Currency::new(DEFAULT_CURRENCY) else {
            return;
        };
        let detail = self.payment_note.trim();

        let mut initial_ok = false;
        if let Some(usd) = self.convert_amount(initial, initial_rate)
            && let Ok(amount) = Money::new(usd, currency.clone())
        {
            let new = NewExpense {
                amount,
                category_id,
                note: Some(format!("{name} · inicial Cashea")),
                spent_at: local_moment_at(today),
                local_date: today,
                evidence_path: None,
                rate_cents: Some(initial_rate),
                rate_day: self.rate_for(today).map(|(day, _)| day.to_string()),
                method: MovementMethod::ExternalPage,
                original_amount_cents: (self.entry_currency == EntryCurrency::Ves)
                    .then_some(initial),
                original_currency: (self.entry_currency == EntryCurrency::Ves)
                    .then(|| "VES".to_owned()),
            };
            let initial_summary = new.note.clone().unwrap_or_else(|| "Gasto".into());
            let initial_amount = new.amount.cents();
            match self
                .repo
                .lock()
                .map_err(|_| ())
                .and_then(|repo| repo.insert_expense(&new).map_err(|_| ()))
            {
                Ok(id) => {
                    self.log_activity(
                        ActivityKind::ExpenseCreated,
                        id.get(),
                        initial_summary,
                        initial_amount,
                    );
                    initial_ok = true;
                }
                Err(()) => initial_ok = false,
            }
        }

        let mut created = 0usize;
        let mut failed = 0usize;
        for index in 1..=count {
            let due = today + Duration::days(14 * index as i64);
            let Some((rate_day, usd_rate)) = self.rate_for(due) else {
                failed += 1;
                continue;
            };
            let Some(usd) = self.convert_amount(installment, usd_rate) else {
                failed += 1;
                continue;
            };
            let Ok(amount) = Money::new(usd, currency.clone()) else {
                failed += 1;
                continue;
            };
            let note = if detail.is_empty() {
                format!("Cuota {index}/{count}")
            } else {
                format!("Cuota {index}/{count} · {detail} (tasa {rate_day})")
            };
            let payment = NewRecurringPayment {
                kind: RecurringKind::Payment,
                name: format!("{name} · cuota {index}/{count}"),
                amount,
                category_id,
                due_day: due.day() as u8,
                due_date: Some(due),
                method: PaymentMethod::Cashea,
                note: Some(note),
            };
            match self
                .repo
                .lock()
                .map_err(|_| ())
                .and_then(|repo| repo.add_recurring_payment(&payment).map_err(|_| ()))
            {
                Ok(id) => {
                    self.log_activity(
                        ActivityKind::PaymentCreated,
                        id.get(),
                        payment.name.clone(),
                        payment.amount.cents(),
                    );
                    created += 1;
                }
                Err(()) => failed += 1,
            }
        }
        self.payment_name.clear();
        self.cashea_initial.clear();
        self.cashea_installments = "3".to_owned();
        self.cashea_installment.clear();
        self.payment_note.clear();
        self.payment_amount.clear();
        self.feedback = Some(if failed == 0 && initial_ok {
            format!("Cashea: inicial + {created} cuotas creadas cada 14 días")
        } else {
            format!("Cashea: {created} cuotas creadas, {failed} fallidas")
        });
        self.refresh();
    }

    pub fn pay_payment(&mut self, id: RecurringPaymentId) {
        let Some(payment) = self
            .payments
            .iter()
            .find(|payment| payment.id == id)
            .cloned()
        else {
            return;
        };
        let now = Local::now();
        let (rate_day, rate_cents) = self.rate_for(now.date_naive()).unzip();
        let detail = payment.note.as_deref().unwrap_or("").trim();
        let kind = if payment.is_one_time() {
            "pago único"
        } else {
            "pago mensual"
        };
        let note = if detail.is_empty() {
            format!("{} ({} · {})", payment.name, kind, payment.method.label())
        } else {
            format!(
                "{} · {} ({} · {})",
                payment.name,
                detail,
                kind,
                payment.method.label()
            )
        };
        let paid_amount = payment.amount.cents();
        let new = NewExpense {
            amount: payment.amount,
            category_id: payment.category_id,
            note: Some(note),
            spent_at: now.fixed_offset(),
            local_date: now.date_naive(),
            evidence_path: None,
            rate_cents,
            rate_day: rate_day.map(|value| value.to_string()),
            method: MovementMethod::from_payment_method(payment.method),
            original_amount_cents: None,
            original_currency: None,
        };
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.pay_recurring_payment(id, &new).map_err(|_| ()));
        match result {
            Ok(()) => {
                self.log_activity(
                    ActivityKind::PaymentPaid,
                    id.get(),
                    payment.name.clone(),
                    paid_amount,
                );
                self.feedback = Some(format!("{} pagado y registrado", payment.name));
                self.refresh();
            }
            Err(()) => self.feedback = Some("No se pudo registrar el pago".into()),
        }
    }

    pub fn request_remove_payment(&mut self, id: RecurringPaymentId) {
        self.pending_delete = Some(PendingDelete::Payment(id));
    }

    pub fn remove_payment(&mut self, id: RecurringPaymentId) -> bool {
        let payment = self.payments.iter().find(|item| item.id == id).cloned();
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.delete_recurring_payment(id).map_err(|_| ()));
        if result.is_err() {
            return false;
        }
        if let Some(payment) = payment {
            self.log_activity(
                ActivityKind::PaymentDeleted,
                id.get(),
                payment.name,
                payment.amount.cents(),
            );
        }
        self.refresh();
        true
    }

    pub fn confirm_remove(&mut self) {
        let Some(target) = self.pending_delete else {
            return;
        };
        self.pending_delete = None;
        let deleted = match target {
            PendingDelete::Expense(id) => self.remove_expense(id),
            PendingDelete::Income(id) => self.remove_income(id),
            PendingDelete::Payment(id) => self.remove_payment(id),
        };
        if deleted {
            self.toast = Some("Registro eliminado".into());
        }
    }

    pub fn cancel_remove(&mut self) {
        self.pending_delete = None;
    }

    pub fn begin_edit_expense(&mut self, id: ExpenseId) {
        let Some(expense) = self.expenses.iter().find(|item| item.id == id).cloned() else {
            return;
        };
        self.editing_expense = Some(id);
        self.editing_income = None;
        let original_ves = expense.original_currency.as_deref() == Some("VES")
            && expense.original_amount_cents.is_some();
        if original_ves {
            self.amount = format_cents_input(
                &expense
                    .original_amount_cents
                    .unwrap_or_default()
                    .to_string(),
                "VES",
            );
            self.entry_currency = EntryCurrency::Ves;
        } else {
            self.amount = format_cents_input(&expense.amount.cents().to_string(), "USD");
            self.entry_currency = EntryCurrency::Usd;
        }
        self.note = expense.note.clone().unwrap_or_default();
        self.expense_date = expense.local_date.to_string();
        self.category_id = Some(expense.category_id);
        self.expense_method = expense.method;
        self.expense_rate_input = expense.rate_cents.map(input_from_cents).unwrap_or_default();
        self.evidence_path = expense.evidence_path.clone();
        self.evidence_name = expense
            .evidence_path
            .as_deref()
            .and_then(|path| path.rsplit('/').next())
            .map(str::to_owned);
        self.feedback = None;
        self.view = View::Add;
    }

    pub fn begin_edit_income(&mut self, id: IncomeId) {
        let Some(income) = self.incomes.iter().find(|item| item.id == id).cloned() else {
            return;
        };
        self.editing_income = Some(id);
        self.editing_expense = None;
        let original_ves = income.original_currency.as_deref() == Some("VES")
            && income.original_amount_cents.is_some();
        if original_ves {
            self.income_amount = format_cents_input(
                &income.original_amount_cents.unwrap_or_default().to_string(),
                "VES",
            );
            self.income_entry_currency = EntryCurrency::Ves;
        } else {
            self.income_amount = format_cents_input(&income.amount.cents().to_string(), "USD");
            self.income_entry_currency = EntryCurrency::Usd;
        }
        self.income_note = income.note.clone().unwrap_or_default();
        self.income_date = income.local_date.to_string();
        self.income_category_id = Some(income.category_id);
        self.income_method = income.method;
        self.income_rate_input = income.rate_cents.map(input_from_cents).unwrap_or_default();
        self.feedback = None;
        self.view = View::AddIncome;
    }

    pub fn cancel_edit(&mut self) {
        self.editing_expense = None;
        self.editing_income = None;
        self.amount.clear();
        self.note.clear();
        self.income_amount.clear();
        self.income_note.clear();
        self.clear_evidence();
        self.view = View::Home;
    }

    pub fn budget_for(&self, id: CategoryId) -> Option<i64> {
        self.budgets
            .iter()
            .find(|(category_id, _)| *category_id == id)
            .map(|(_, limit)| *limit)
    }

    pub fn spent_this_month(&self, id: CategoryId) -> i64 {
        let now = Local::now();
        self.expenses
            .iter()
            .filter(|expense| {
                expense.category_id == id
                    && expense.local_date.year() == now.year()
                    && expense.local_date.month() == now.month()
            })
            .map(|expense| expense.amount.cents())
            .sum()
    }

    pub fn save_budget(&mut self) {
        let Some(category_id) = self.budget_category_id else {
            self.feedback = Some("Elige una categoría".into());
            return;
        };
        let Some(cents) = parse_cents(&self.budget_input) else {
            self.feedback = Some("Escribe un límite válido, por ejemplo 100.00".into());
            return;
        };
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.set_budget(category_id, cents).map_err(|_| ()));
        if result.is_ok() {
            self.budget_input.clear();
            self.feedback = Some("Presupuesto guardado".into());
            self.refresh();
        } else {
            self.feedback = Some("No se pudo guardar el presupuesto".into());
        }
    }

    pub fn remove_budget(&mut self, id: CategoryId) {
        let result = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.remove_budget(id).map_err(|_| ()));
        if result.is_ok() {
            self.refresh();
        }
    }

    fn matches_search(&self, text: &str, category_id: CategoryId) -> bool {
        let query = self.search_query.trim().to_lowercase();
        if let Some(selected) = self.search_category_id
            && selected != category_id
        {
            return false;
        }
        query.is_empty() || text.to_lowercase().contains(&query)
    }

    pub fn searched_expenses(&self) -> Vec<Expense> {
        let from = self.search_from.trim();
        let to = self.search_to.trim();
        self.expenses
            .iter()
            .filter(|expense| {
                let category = self
                    .category_of(expense.category_id)
                    .map(|c| c.name.as_str())
                    .unwrap_or("");
                if !self.matches_search(
                    &format!("{} {}", expense.note.as_deref().unwrap_or(""), category),
                    expense.category_id,
                ) {
                    return false;
                }
                if !from.is_empty() {
                    let Ok(start) = NaiveDate::parse_from_str(from, "%Y-%m-%d") else {
                        return false;
                    };
                    if expense.local_date < start {
                        return false;
                    }
                }
                if !to.is_empty() {
                    let Ok(end) = NaiveDate::parse_from_str(to, "%Y-%m-%d") else {
                        return false;
                    };
                    if expense.local_date > end {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    pub fn searched_incomes(&self) -> Vec<Income> {
        let from = self.search_from.trim();
        let to = self.search_to.trim();
        self.incomes
            .iter()
            .filter(|income| {
                let category = self
                    .income_category_of(income.category_id)
                    .map(|c| c.name.as_str())
                    .unwrap_or("");
                if !self.matches_search(
                    &format!("{} {}", income.note.as_deref().unwrap_or(""), category),
                    income.category_id,
                ) {
                    return false;
                }
                if !from.is_empty() {
                    let Ok(start) = NaiveDate::parse_from_str(from, "%Y-%m-%d") else {
                        return false;
                    };
                    if income.local_date < start {
                        return false;
                    }
                }
                if !to.is_empty() {
                    let Ok(end) = NaiveDate::parse_from_str(to, "%Y-%m-%d") else {
                        return false;
                    };
                    if income.local_date > end {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    pub fn searched_payments(&self) -> Vec<RecurringPayment> {
        let from = self.search_from.trim();
        let to = self.search_to.trim();
        let today = Local::now().date_naive();
        self.payments
            .iter()
            .filter(|payment| {
                let category = self
                    .category_of(payment.category_id)
                    .map(|item| item.name.as_str())
                    .unwrap_or("");
                if !self.matches_search(
                    &format!(
                        "{} {} {}",
                        payment.name,
                        payment.note.as_deref().unwrap_or(""),
                        category
                    ),
                    payment.category_id,
                ) {
                    return false;
                }
                let due = payment.due_date(today.year(), today.month());
                if !from.is_empty() {
                    let Ok(start) = NaiveDate::parse_from_str(from, "%Y-%m-%d") else {
                        return false;
                    };
                    if due < start {
                        return false;
                    }
                }
                if !to.is_empty() {
                    let Ok(end) = NaiveDate::parse_from_str(to, "%Y-%m-%d") else {
                        return false;
                    };
                    if due > end {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    pub fn export_backup_json(&mut self) -> Option<String> {
        let result = self
            .repo
            .lock()
            .map_err(|_| "No se pudo abrir la base de datos".to_owned())
            .and_then(|repo| repo.export_backup().map_err(|error| error.to_string()));
        match result.and_then(|backup| {
            let summary = backup.summary();
            serde_json::to_string_pretty(&backup)
                .map(|json| (json, summary))
                .map_err(|error| error.to_string())
        }) {
            Ok((json, summary)) => {
                self.import_input = json.clone();
                self.feedback = Some(format!("Copia preparada: {summary}."));
                Some(json)
            }
            Err(error) => {
                self.feedback = Some(format!("No se pudo exportar: {error}"));
                None
            }
        }
    }

    pub fn import_backup_text(&mut self, json: &str) {
        let backup: Backup = match serde_json::from_str(json) {
            Ok(backup) => backup,
            Err(error) => {
                self.feedback = Some(format!("Copia no válida: {error}"));
                return;
            }
        };
        let result = self
            .repo
            .lock()
            .map_err(|_| "No se pudo abrir la base de datos".to_owned())
            .and_then(|repo| {
                repo.import_backup(&backup)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(()) => {
                self.seed_categories();
                self.refresh();
                self.feedback = Some(format!("Copia restaurada: {}.", backup.summary()));
            }
            Err(error) => self.feedback = Some(format!("No se pudo restaurar: {error}")),
        }
    }

    pub fn set_pin(&mut self) {
        let pin = self.setup_pin.trim();
        if pin.len() != 4 || !pin.bytes().all(|byte| byte.is_ascii_digit()) {
            self.pin_error = Some("El PIN debe tener 4 dígitos".into());
            return;
        }
        if pin != self.confirm_pin.trim() {
            self.pin_error = Some("Los PIN no coinciden".into());
            return;
        }
        let hash = sha256_hex(pin);
        let saved = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.set_setting(PIN_HASH_SETTING, &hash).map_err(|_| ()));
        if saved.is_ok() {
            self.pin_hash = Some(hash);
            self.setup_pin.clear();
            self.confirm_pin.clear();
            self.pin_error = None;
            self.feedback = Some("PIN activado".into());
        } else {
            self.pin_error = Some("No se pudo guardar el PIN".into());
        }
    }

    pub fn remove_pin(&mut self) {
        let hash = sha256_hex(self.pin_input.trim());
        if self.pin_hash.as_deref() != Some(hash.as_str()) {
            self.pin_error = Some("PIN incorrecto".into());
            return;
        }
        let saved = self
            .repo
            .lock()
            .map_err(|_| ())
            .and_then(|repo| repo.set_setting(PIN_HASH_SETTING, "").map_err(|_| ()));
        if saved.is_ok() {
            self.pin_hash = None;
            self.pin_input.clear();
            self.pin_error = None;
            self.feedback = Some("PIN desactivado".into());
        }
    }

    pub fn unlock(&mut self) {
        let hash = sha256_hex(self.pin_input.trim());
        if self.pin_hash.as_deref() == Some(hash.as_str()) {
            self.locked = false;
            self.pin_input.clear();
            self.pin_error = None;
        } else {
            self.pin_error = Some("PIN incorrecto".into());
            self.pin_input.clear();
        }
    }

    pub fn lock(&mut self) {
        self.locked = self.pin_hash.is_some();
        self.pin_input.clear();
        self.evidence_viewer = None;
    }

    pub fn set_text_scale(&mut self, value: &str) {
        let scale = match value {
            "small" | "large" => value.to_owned(),
            _ => "normal".to_owned(),
        };
        if let Ok(repo) = self.repo.lock() {
            let _ = repo.set_setting(TEXT_SCALE_SETTING, &scale);
        }
        self.text_scale = scale;
        self.feedback = Some("Tamaño de texto actualizado".into());
    }

    pub fn text_scale_factor(&self) -> f64 {
        match self.text_scale.as_str() {
            "small" => 0.92,
            "large" => 1.14,
            _ => 1.0,
        }
    }

    pub fn is_light_theme(&self) -> bool {
        self.theme == "light"
    }

    pub fn set_theme(&mut self, value: &str) {
        let theme = if value == "light" { "light" } else { "dark" }.to_owned();
        if let Ok(repo) = self.repo.lock() {
            let _ = repo.set_setting(THEME_SETTING, &theme);
        }
        self.theme = theme;
        self.feedback = Some("Tema actualizado".into());
    }

    pub fn request_biometric(&mut self, action: &str) {
        if action == "enroll" && self.pin_hash.is_none() {
            self.pin_error = Some("Activa el PIN antes de guardar la huella.".into());
            return;
        }
        self.pin_error = None;
        bridge::request_biometric(action);
    }

    pub fn finish_biometric(&mut self, action: &str, ok: bool, message: &str) {
        if !ok {
            self.pin_error = Some(if message.trim().is_empty() {
                "No se pudo verificar la huella".to_owned()
            } else {
                message.to_owned()
            });
            return;
        }
        match action {
            "enroll" => {
                if let Ok(repo) = self.repo.lock() {
                    let _ = repo.set_setting(BIOMETRIC_SETTING, "1");
                }
                self.biometric_enabled = true;
                self.feedback = Some("Huella guardada. Podrás usarla al abrir la app.".into());
            }
            "unlock" => {
                self.locked = false;
                self.pin_input.clear();
                self.pin_error = None;
            }
            _ => {}
        }
    }

    pub fn remove_biometric(&mut self) {
        if let Ok(repo) = self.repo.lock() {
            let _ = repo.set_setting(BIOMETRIC_SETTING, "");
        }
        self.biometric_enabled = false;
        self.feedback = Some("Huella desactivada. El PIN sigue activo.".into());
    }

    pub fn set_auto_backup_hours(&mut self, hours: i64) {
        let hours = hours.clamp(1, 168);
        if let Ok(repo) = self.repo.lock() {
            let _ = repo.set_setting(AUTO_BACKUP_INTERVAL_SETTING, &hours.to_string());
        }
        self.auto_backup_hours = hours;
        self.feedback = Some(format!("Copia automática cada {hours} h."));
    }

    pub fn auto_backup_status(&self) -> String {
        let last = self
            .repo
            .lock()
            .ok()
            .and_then(|repo| repo.get_setting(LAST_AUTO_BACKUP_SETTING).ok().flatten())
            .and_then(|value| value.parse::<i64>().ok());
        match last {
            Some(seconds) => format!("Última copia automática: {}", format_timestamp(seconds)),
            None => "Aún no se ha generado ninguna copia automática.".to_owned(),
        }
    }

    pub fn auto_backup_now(&mut self) {
        let result = self
            .repo
            .lock()
            .map_err(|_| "No se pudo abrir la base de datos".to_owned())
            .and_then(|repo| {
                auto_backup(&repo, true).ok_or_else(|| "No se pudo guardar".to_owned())
            });
        self.feedback = Some(match result {
            Ok(path) => format!("Copia automática guardada en {path}"),
            Err(error) => format!("No se pudo crear la copia: {error}"),
        });
    }

    pub fn show_evidence(&mut self, path: &str) {
        self.evidence_viewer = platform::read_evidence_data_url(path);
    }

    pub fn close_evidence(&mut self) {
        self.evidence_viewer = None;
    }

    pub fn latest_rate_label(&self) -> String {
        match self.rate_history.first() {
            Some((day, rate, source, _)) => {
                let origin = match source.as_str() {
                    "dolarapi" => "automática (DolarAPI)",
                    "bcv" => "automática (BCV)",
                    value if value.starts_with("manual") => "manual",
                    _ => source.as_str(),
                };
                format!("{} · {origin} · {}", format_date(*day), format_rate(*rate))
            }
            None => "Sin tasa registrada".into(),
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
            .filter(|income| income.local_date.year() == year && income.local_date.month() == month)
            .map(|income| income.amount.cents())
            .sum()
    }

    pub fn month_total_ves(&self, rate_cents: i64) -> Option<String> {
        let now = Local::now();
        let (year, month) = (now.year(), now.month());
        let mut original_sum: i64 = 0;
        let mut any = false;
        for expense in self.expenses.iter().filter(|expense| {
            expense.local_date.year() == year && expense.local_date.month() == month
        }) {
            any = true;
            match (
                expense.original_currency.as_deref(),
                expense.original_amount_cents,
            ) {
                (Some("VES"), Some(cents)) => original_sum += cents,
                _ => return money_ves(self.month_total(), rate_cents),
            }
        }
        if !any {
            return money_ves(0, rate_cents);
        }
        Some(format_money(
            original_sum,
            &Currency::new("VES").expect("VES válida"),
        ))
    }

    pub fn month_income_total_ves(&self, rate_cents: i64) -> Option<String> {
        let now = Local::now();
        let (year, month) = (now.year(), now.month());
        let mut original_sum: i64 = 0;
        let mut any = false;
        for income in self
            .incomes
            .iter()
            .filter(|income| income.local_date.year() == year && income.local_date.month() == month)
        {
            any = true;
            match (
                income.original_currency.as_deref(),
                income.original_amount_cents,
            ) {
                (Some("VES"), Some(cents)) => original_sum += cents,
                _ => return money_ves(self.month_income_total(), rate_cents),
            }
        }
        if !any {
            return money_ves(0, rate_cents);
        }
        Some(format_money(
            original_sum,
            &Currency::new("VES").expect("VES válida"),
        ))
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
        self.income_categories
            .iter()
            .find(|category| category.id == id)
    }

    pub fn due_payments(&self) -> Vec<(RecurringPayment, PaymentStatus)> {
        let today = Local::now().date_naive();
        let mut items: Vec<_> = self
            .payments
            .iter()
            .filter_map(|payment| {
                let status = payment.status(today);
                matches!(
                    status,
                    PaymentStatus::Overdue(_) | PaymentStatus::DueSoon(_)
                )
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
        PaymentStatus::Paid => "Pagado".into(),
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
    usd_to_ves(usd_cents, rate_cents)
        .map(|cents| format_money(cents, &Currency::new("VES").expect("VES válida")))
}

pub fn display_ves(
    usd_cents: i64,
    rate_cents: i64,
    original_amount_cents: Option<i64>,
    original_currency: Option<&str>,
) -> Option<String> {
    if original_currency == Some("VES")
        && let Some(cents) = original_amount_cents
    {
        return Some(format_money(
            cents,
            &Currency::new("VES").expect("VES válida"),
        ));
    }
    money_ves(usd_cents, rate_cents)
}

/// Escribe una copia automática local si han pasado las horas configuradas.
/// Devuelve la ruta cuando se creó una copia.
pub fn auto_backup(repo: &SqliteRepository, force: bool) -> Option<String> {
    let hours = repo
        .get_setting(AUTO_BACKUP_INTERVAL_SETTING)
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(12)
        .clamp(1, 168);
    let now = Local::now().timestamp();
    if !force {
        let last = repo
            .get_setting(LAST_AUTO_BACKUP_SETTING)
            .ok()
            .flatten()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(0);
        if now - last < hours * 3600 {
            return None;
        }
    }
    let backup = repo.export_backup().ok()?;
    let json = serde_json::to_string(&backup).ok()?;
    let path = platform::write_auto_backup(&json, 3)?;
    let _ = repo.set_setting(LAST_AUTO_BACKUP_SETTING, &now.to_string());
    log::info!("copia automática guardada en {path}");
    Some(path)
}

/// Revisa periódicamente si toca una copia automática mientras la app está abierta.
pub fn start_auto_backup_loop(repo: Arc<Mutex<SqliteRepository>>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(StdDuration::from_secs(600));
            if let Ok(guard) = repo.lock() {
                let _ = auto_backup(&guard, false);
            }
        }
    });
}

/// Actualiza la tasa en segundo plano y la persiste; la UI la recoge en el próximo refresh.
pub fn start_rate_refresh_loop(repo: Arc<Mutex<SqliteRepository>>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(StdDuration::from_secs(30 * 60));
            if let Some((rate, source)) = platform::fetch_exchange_rate() {
                let today = Local::now().date_naive();
                if let Ok(guard) = repo.lock() {
                    let _ = guard.set_setting(RATE_SETTING, &rate.to_string());
                    let _ = guard.set_rate_for_day(today, rate, source);
                }
            }
        }
    });
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
    fn prepare_update_uses_github_unless_local_url_is_set() {
        let repo = finance_store::SqliteRepository::open_in_memory().expect("repo en memoria");
        let mut state = AppState::new(repo);
        assert_eq!(state.prepare_update().unwrap(), UpdateSource::GitHub);

        state.set_update_url("http://192.168.0.10:8000".into());
        assert_eq!(
            state.prepare_update().unwrap(),
            UpdateSource::Local("http://192.168.0.10:8000".into())
        );

        state.set_update_url(String::new());
        assert_eq!(state.prepare_update().unwrap(), UpdateSource::GitHub);
    }

    #[test]
    fn formats_money_using_each_currency_convention() {
        assert_eq!(money_usd(123456), "USD 1,234.56");
        assert_eq!(money_ves(100, 85789).as_deref(), Some("Bs.S 857,89"));
        assert_eq!(money_ves(1000, 0), None);
    }

    #[test]
    fn formats_visible_dates_for_venezuela() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 5).expect("fecha válida");
        assert_eq!(format_date(day), "05/11/2026");
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
    fn refund_creates_income_and_keeps_the_expense() {
        let repo = finance_store::SqliteRepository::open_in_memory().expect("repo en memoria");
        let mut state = AppState::new(repo);
        let day = chrono::NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
        state.refresh();
        state.category_id = Some(state.categories[0].id);
        state.amount = "1550".into();
        state.note = "Almuerzo".into();
        state.expense_date = day.to_string();
        state.expense_method = MovementMethod::BankTransfer;
        state.add_expense();

        let expense = state.expenses[0].clone();
        state.refund_expense(expense.id);

        assert_eq!(state.expenses.len(), 1, "el gasto original se conserva");
        assert_eq!(state.incomes.len(), 1);
        let income = &state.incomes[0];
        assert_eq!(income.amount, expense.amount);
        assert_eq!(income.local_date, expense.local_date);
        assert_eq!(income.method, MovementMethod::BankTransfer);
        assert_eq!(income.note.as_deref(), Some("Devolución de Almuerzo"));
        let category = state
            .income_categories
            .iter()
            .find(|c| c.id == income.category_id)
            .unwrap();
        assert!(
            category.name.eq_ignore_ascii_case("otros")
                || category.name.eq_ignore_ascii_case("other")
        );
    }

    #[test]
    fn cashea_inputs_add_up_to_the_purchase_total() {
        let repo = finance_store::SqliteRepository::open_in_memory().expect("repo en memoria");
        let mut state = AppState::new(repo);
        state.cashea_initial = "1000".into();
        state.cashea_installments = "3".into();
        state.cashea_installment = "5000".into();
        state.sync_cashea_total();
        assert_eq!(state.payment_amount, "16000");
        state.cashea_installments = "abc".into();
        state.sync_cashea_total();
        assert_eq!(state.payment_amount, "1000");
    }

    #[test]
    fn cashea_creates_initial_expense_and_installments_every_14_days() {
        let repo = finance_store::SqliteRepository::open_in_memory().expect("repo en memoria");
        let mut state = AppState::new(repo);
        let today = Local::now().date_naive();
        state
            .repo
            .lock()
            .unwrap()
            .set_rate_for_day(today, 10000, "test")
            .unwrap();
        state.refresh();
        let category = state.categories[0].id;
        state.payment_name = "Nevera".into();
        state.payment_note = "prueba".into();
        state.cashea_initial = "10000".into();
        state.cashea_installments = "3".into();
        state.cashea_installment = "5000".into();

        state.add_cashea_payment("Nevera".into(), category);

        assert!(
            state.feedback.as_deref().unwrap_or("").contains("3 cuotas"),
            "{:?}",
            state.feedback
        );
        let repo = state.repo.lock().unwrap();
        assert_eq!(repo.expenses_between(today, today).unwrap().len(), 1);
        let payments = repo.recurring_payments().unwrap();
        assert_eq!(payments.len(), 3);
        assert!(payments.iter().all(|p| p.method == PaymentMethod::Cashea));
        assert!(payments.iter().all(|p| p.is_one_time()));
        let mut dates: Vec<_> = payments.iter().filter_map(|p| p.due_date).collect();
        dates.sort();
        assert_eq!(dates[0], today + Duration::days(14));
        assert_eq!(dates[1], today + Duration::days(28));
        assert_eq!(dates[2], today + Duration::days(42));
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

    #[test]
    fn searches_payments_and_logs_their_creation() {
        let repo = finance_store::SqliteRepository::open_in_memory().expect("repo en memoria");
        let mut state = AppState::new(repo);
        state.refresh();
        state.payment_category_id = Some(state.categories[0].id);
        state.payment_name = "Luz".into();
        state.payment_amount = "1500".into();
        state.payment_day = "5".into();
        state.add_payment();

        assert!(
            state.feedback.as_deref().unwrap_or("").contains("creado"),
            "feedback: {:?}",
            state.feedback
        );
        assert_eq!(state.searched_payments().len(), 1);
        state.search_query = "luz".into();
        assert_eq!(state.searched_payments().len(), 1);
        state.search_query = "agua".into();
        assert!(state.searched_payments().is_empty());
        assert!(state.activity.iter().any(
            |entry| entry.kind == ActivityKind::PaymentCreated && entry.summary.contains("Luz")
        ));
    }
}
