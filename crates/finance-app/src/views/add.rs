use chrono::{Local, NaiveDate};
use dioxus::prelude::*;
use finance_core::{format_cents_input, format_rate, CategoryId};

use crate::{
    components::icon::{icon_for_category, Icon},
    state::{money_usd, money_ves, parse_cents, AppState, EntryCurrency},
};

#[derive(Props, Clone, PartialEq)]
pub struct AddProps {
    pub state: Signal<AppState>,
}

#[derive(Props, Clone, PartialEq)]
struct ChipProps {
    id: CategoryId,
    icon: &'static str,
    label: String,
    active: bool,
    state: Signal<AppState>,
}

#[component]
fn Chip(props: ChipProps) -> Element {
    let mut state = props.state;
    let id = props.id;
    let class = if props.active { "chip active" } else { "chip" };
    rsx! {
        button {
            class: class,
            onclick: move |_| { state.write().category_id = Some(id); },
            Icon { name: props.icon, class: "chip-icon" }
            span { "{props.label}" }
        }
    }
}

#[component]
pub fn Add(props: AddProps) -> Element {
    let mut state = props.state;
    let snapshot = state.read();
    let amount = snapshot.amount.clone();
    let amount_display = format_cents_input(&amount, snapshot.entry_currency.code());
    let note = snapshot.note.clone();
    let expense_date = snapshot.expense_date.clone();
    let expense_rate_input = snapshot.expense_rate_input.clone();
    let expense_rate_display = format_cents_input(&expense_rate_input, "VES");
    let evidence_name = snapshot.evidence_name.clone();
    let selected = snapshot.category_id;
    let entry_currency = snapshot.entry_currency;
    let day = NaiveDate::parse_from_str(&expense_date, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().date_naive());
    let history_rate = snapshot.rate_value_for(day).unwrap_or(0);
    let rate = parse_cents(&expense_rate_input).unwrap_or(history_rate);
    let converted = parse_cents(&amount).and_then(|cents| match entry_currency {
        EntryCurrency::Usd => money_ves(cents, rate),
        EntryCurrency::Ves => snapshot
            .convert_amount(cents, rate)
            .map(|usd| format!("≈ {}", money_usd(usd))),
    });
    let chips: Vec<ChipProps> = snapshot
        .categories
        .iter()
        .map(|category| ChipProps {
            id: category.id,
            icon: icon_for_category(&category.name, category.icon.as_deref()),
            label: category.name.clone(),
            active: selected == Some(category.id),
            state,
        })
        .collect();
    let feedback = snapshot.feedback.clone();
    drop(snapshot);

    rsx! {
        div { class: "page",
            div { class: "card",
                h3 { "Nuevo gasto" }

                label { class: "field-label", "Moneda" }
                div { class: "category-grid two",
                    button {
                        class: if entry_currency == EntryCurrency::Usd { "chip active" } else { "chip" },
                        onclick: move |_| state.write().set_entry_currency(EntryCurrency::Usd),
                        "USD"
                    }
                    button {
                        class: if entry_currency == EntryCurrency::Ves { "chip active" } else { "chip" },
                        onclick: move |_| state.write().set_entry_currency(EntryCurrency::Ves),
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
                        state.write().amount = digits;
                    },
                }
                if let Some(preview) = converted {
                    p { class: "muted", "{preview}" }
                }

                label { class: "field-label", "Fecha del gasto" }
                input {
                    class: "text-input",
                    r#type: "date",
                    value: expense_date,
                    oninput: move |event| { state.write().expense_date = event.value(); },
                }
                p { class: "muted", "Se usa la tasa de esa fecha o la última conocida anterior." }

                label { class: "field-label", "Tasa del dólar para este gasto" }
                input {
                    class: "text-input",
                    value: expense_rate_display,
                    inputmode: "numeric",
                    placeholder: if history_rate > 0 {
                        format!("Automática: {}", format_rate(history_rate))
                    } else {
                        "Ej. 857,89".to_owned()
                    },
                    oninput: move |event| {
                        let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).collect();
                        state.write().expense_rate_input = digits;
                    },
                }
                if expense_rate_input.is_empty() {
                    p { class: "muted", "Déjala vacía para usar la tasa automática de la fecha." }
                } else {
                    button { class: "link small-link", onclick: move |_| state.write().expense_rate_input.clear(),
                        "Usar tasa automática"
                    }
                }

                label { class: "field-label", "Categoría" }
                div { class: "category-grid",
                    for chip in chips {
                        Chip { ..chip }
                    }
                }

                label { class: "field-label", "Nota (opcional)" }
                input {
                    class: "text-input",
                    value: note,
                    placeholder: "Ej. almuerzo",
                    maxlength: "200",
                    oninput: move |event| { state.write().note = event.value(); },
                }

                label { class: "field-label", "Evidencia (opcional)" }
                label { class: "file-drop",
                    span { class: "file-drop-icon", Icon { name: "camera", class: "icon" } }
                    span { class: "file-drop-copy",
                        strong { "Adjuntar imagen" }
                        small { class: "muted", "Toca para elegir una foto" }
                    }
                    input {
                        id: "evidence-file",
                        class: "file-input-native",
                        r#type: "file",
                        accept: "image/*",
                    }
                }
                if let Some(name) = evidence_name {
                    div { class: "evidence-row",
                        span { class: "grow muted",
                            Icon { name: "paperclip", class: "inline-icon" }
                            "{name}"
                        }
                        button { class: "link small-link", onclick: move |_| state.write().clear_evidence(),
                            "Quitar"
                        }
                    }
                }

                if let Some(message) = feedback {
                    p { class: "feedback", "{message}" }
                }

                button { class: "primary wide", onclick: move |_| state.write().add_expense(),
                    "Guardar gasto"
                }
            }
        }
    }
}
