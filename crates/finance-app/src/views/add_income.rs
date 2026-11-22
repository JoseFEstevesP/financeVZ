use chrono::{Local, NaiveDate};
use dioxus::prelude::*;
use finance_core::{format_cents_input, format_rate, CategoryId};

use crate::state::{money_usd, money_ves, parse_cents, AppState, EntryCurrency};

#[derive(Props, Clone, PartialEq)]
pub struct AddIncomeProps {
    pub state: Signal<AppState>,
}

#[derive(Props, Clone, PartialEq)]
struct IncomeChipProps {
    id: CategoryId,
    label: String,
    active: bool,
    state: Signal<AppState>,
}

#[component]
fn IncomeChip(props: IncomeChipProps) -> Element {
    let mut state = props.state;
    let id = props.id;
    let class = if props.active { "chip active" } else { "chip" };
    rsx! {
        button {
            class: class,
            onclick: move |_| { state.write().income_category_id = Some(id); },
            "{props.label}"
        }
    }
}

#[component]
pub fn AddIncome(props: AddIncomeProps) -> Element {
    let mut state = props.state;
    let snapshot = state.read();
    let amount = snapshot.income_amount.clone();
    let amount_display = format_cents_input(&amount, snapshot.income_entry_currency.code());
    let note = snapshot.income_note.clone();
    let income_date = snapshot.income_date.clone();
    let income_rate_input = snapshot.income_rate_input.clone();
    let income_rate_display = format_cents_input(&income_rate_input, "VES");
    let selected = snapshot.income_category_id;
    let entry_currency = snapshot.income_entry_currency;
    let day = NaiveDate::parse_from_str(&income_date, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().date_naive());
    let history_rate = snapshot.rate_value_for(day).unwrap_or(0);
    let rate = parse_cents(&income_rate_input).unwrap_or(history_rate);
    let converted = parse_cents(&amount).and_then(|cents| match entry_currency {
        EntryCurrency::Usd => money_ves(cents, rate),
        EntryCurrency::Ves => snapshot
            .convert_income_amount(cents, rate)
            .map(|usd| format!("≈ {}", money_usd(usd))),
    });
    let chips: Vec<IncomeChipProps> = snapshot
        .income_categories
        .iter()
        .map(|category| IncomeChipProps {
            id: category.id,
            label: format!(
                "{} {}",
                category.icon.as_deref().unwrap_or("📦"),
                category.name
            ),
            active: selected == Some(category.id),
            state,
        })
        .collect();
    let feedback = snapshot.feedback.clone();
    drop(snapshot);

    rsx! {
        div { class: "page",
            div { class: "card",
                h3 { "Nuevo ingreso" }

                label { class: "field-label", "Moneda" }
                div { class: "category-grid two",
                    button {
                        class: if entry_currency == EntryCurrency::Usd { "chip active" } else { "chip" },
                        onclick: move |_| state.write().set_income_entry_currency(EntryCurrency::Usd),
                        "USD"
                    }
                    button {
                        class: if entry_currency == EntryCurrency::Ves { "chip active" } else { "chip" },
                        onclick: move |_| state.write().set_income_entry_currency(EntryCurrency::Ves),
                        "Bs"
                    }
                }

                label { class: "field-label", "Importe ({entry_currency.label()})" }
                input {
                    class: "amount-input",
                    value: amount_display,
                    inputmode: "numeric",
                    placeholder: "0.00",
                    oninput: move |event| {
                        let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).collect();
                        state.write().income_amount = digits;
                    },
                }
                if let Some(preview) = converted {
                    p { class: "muted", "{preview}" }
                }

                label { class: "field-label", "Fecha del ingreso" }
                input {
                    class: "text-input",
                    r#type: "date",
                    value: income_date,
                    oninput: move |event| { state.write().income_date = event.value(); },
                }
                p { class: "muted", "Se usa la tasa de esa fecha o la última conocida anterior." }

                label { class: "field-label", "Tasa del dólar para este ingreso" }
                input {
                    class: "text-input",
                    value: income_rate_display,
                    inputmode: "numeric",
                    placeholder: if history_rate > 0 {
                        format!("Automática: {}", format_rate(history_rate))
                    } else {
                        "Ej. 857,89".to_owned()
                    },
                    oninput: move |event| {
                        let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).collect();
                        state.write().income_rate_input = digits;
                    },
                }
                if income_rate_input.is_empty() {
                    p { class: "muted", "Déjala vacía para usar la tasa automática de la fecha." }
                } else {
                    button { class: "link small-link", onclick: move |_| state.write().income_rate_input.clear(),
                        "Usar tasa automática"
                    }
                }

                label { class: "field-label", "Categoría de ingreso" }
                div { class: "category-grid",
                    for chip in chips {
                        IncomeChip { ..chip }
                    }
                }

                label { class: "field-label", "Nota (opcional)" }
                input {
                    class: "text-input",
                    value: note,
                    placeholder: "Ej. sueldo de noviembre",
                    maxlength: "200",
                    oninput: move |event| { state.write().income_note = event.value(); },
                }

                if let Some(message) = feedback {
                    p { class: "feedback", "{message}" }
                }

                button { class: "primary wide", onclick: move |_| state.write().add_income(),
                    "Guardar ingreso"
                }
            }
        }
    }
}
