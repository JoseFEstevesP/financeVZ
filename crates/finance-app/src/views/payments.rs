use dioxus::prelude::*;
use finance_core::{format_cents_input, CategoryId, PaymentStatus, RecurringPaymentId};

use crate::state::{money_usd, money_ves, payment_status_text, AppState, EntryCurrency};

#[derive(Props, Clone, PartialEq)]
pub struct PaymentsProps {
    pub state: Signal<AppState>,
}

#[derive(Props, Clone, PartialEq)]
struct CategoryChipProps {
    id: CategoryId,
    label: String,
    active: bool,
    state: Signal<AppState>,
}

#[component]
fn CategoryChip(props: CategoryChipProps) -> Element {
    let mut state = props.state;
    let id = props.id;
    let class = if props.active { "chip active" } else { "chip" };
    rsx! {
        button {
            class: class,
            onclick: move |_| { state.write().payment_category_id = Some(id); },
            "{props.label}"
        }
    }
}

struct PaymentRow {
    id: RecurringPaymentId,
    name: String,
    amount: String,
    secondary: Option<String>,
    status: String,
    done: bool,
    urgent: bool,
}

#[component]
pub fn Payments(props: PaymentsProps) -> Element {
    let mut state = props.state;
    let snapshot = state.read();
    let rate = snapshot.exchange_rate_cents;
    let rows: Vec<PaymentRow> = snapshot
        .payments
        .iter()
        .map(|payment| {
            let status = payment.status(chrono::Local::now().date_naive());
            PaymentRow {
                id: payment.id,
                name: payment.name.clone(),
                amount: money_usd(payment.amount.cents()),
                secondary: money_ves(payment.amount.cents(), rate),
                status: payment_status_text(status),
                done: status == PaymentStatus::Paid,
                urgent: matches!(status, PaymentStatus::Overdue(_) | PaymentStatus::DueSoon(_)),
            }
        })
        .collect();
    let name = snapshot.payment_name.clone();
    let amount = snapshot.payment_amount.clone();
    let amount_display = format_cents_input(&amount, snapshot.entry_currency.code());
    let day = snapshot.payment_day.clone();
    let entry_currency = snapshot.entry_currency;
    let selected = snapshot.payment_category_id;
    let chips: Vec<CategoryChipProps> = snapshot
        .categories
        .iter()
        .map(|category| CategoryChipProps {
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
                h3 { "Pagos por hacer" }
                p { class: "muted", "Se programa automáticamente el mes siguiente al pagar." }
                if rows.is_empty() {
                    p { class: "muted", "Aún no tienes pagos mensuales." }
                } else {
                    for row in rows {
                        div { class: "payment-row",
                            div { class: "grow",
                                strong { "{row.name}" }
                                small { class: if row.urgent { "muted urgent" } else { "muted" }, "{row.status}" }
                                if let Some(secondary) = row.secondary {
                                    small { class: "muted", "{secondary}" }
                                }
                            }
                            span { class: "expense-amount", "{row.amount}" }
                            if !row.done {
                                button {
                                    class: "primary small",
                                    onclick: move |_| state.write().pay_payment(row.id),
                                    "Pagar"
                                }
                            }
                            button {
                                class: "link small-link",
                                onclick: move |_| state.write().remove_payment(row.id),
                                "✕"
                            }
                        }
                    }
                }
            }

            div { class: "card",
                h3 { "Nuevo pago mensual" }
                label { class: "field-label", "Nombre" }
                input {
                    class: "text-input",
                    value: name,
                    placeholder: "Ej. Luz",
                    maxlength: "40",
                    oninput: move |event| { state.write().payment_name = event.value(); },
                }
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
                label { class: "field-label", "Importe" }
                input {
                    class: "amount-input",
                    value: amount_display,
                    inputmode: "numeric",
                    placeholder: "0.00",
                    oninput: move |event| {
                        let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).collect();
                        state.write().payment_amount = digits;
                    },
                }
                label { class: "field-label", "Día de vencimiento" }
                input {
                    class: "text-input",
                    value: day,
                    inputmode: "numeric",
                    placeholder: "25",
                    maxlength: "2",
                    oninput: move |event| { state.write().payment_day = event.value(); },
                }
                label { class: "field-label", "Categoría" }
                div { class: "category-grid",
                    for chip in chips {
                        CategoryChip { ..chip }
                    }
                }
                if let Some(message) = feedback {
                    p { class: "feedback", "{message}" }
                }
                button { class: "primary wide", onclick: move |_| state.write().add_payment(),
                    "Crear pago"
                }
            }
        }
    }
}
