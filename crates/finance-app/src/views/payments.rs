use dioxus::prelude::*;
use finance_core::{
    CategoryId, PaymentMethod, PaymentStatus, RecurringKind, RecurringPayment, RecurringPaymentId,
    format_cents_input,
};

use crate::{
    components::icon::{Icon, icon_for_category},
    state::{
        AppState, EntryCurrency, PaymentKind, format_date, money_usd, money_ves,
        payment_status_text,
    },
};

#[derive(Props, Clone, PartialEq)]
pub struct PaymentsProps {
    pub state: Signal<AppState>,
}

#[derive(Props, Clone, PartialEq)]
struct CategoryChipProps {
    id: CategoryId,
    icon: &'static str,
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
            Icon { name: props.icon, class: "chip-icon" }
            span { "{props.label}" }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MethodChipProps {
    method: PaymentMethod,
    icon: &'static str,
    active: bool,
    state: Signal<AppState>,
}

#[component]
fn MethodChip(props: MethodChipProps) -> Element {
    let mut state = props.state;
    let method = props.method;
    let class = if props.active { "chip active" } else { "chip" };
    rsx! {
        button {
            class: class,
            onclick: move |_| { state.write().payment_method = method; },
            Icon { name: props.icon, class: "chip-icon" }
            span { "{method.label()}" }
        }
    }
}

struct PaymentRow {
    id: RecurringPaymentId,
    name: String,
    amount: String,
    secondary: Option<String>,
    status: String,
    kind: RecurringKind,
    detail: String,
    recommendation: Option<String>,
    done: bool,
    urgent: bool,
}

fn next_monthly_day(today: chrono::NaiveDate, day: u8) -> chrono::NaiveDate {
    use chrono::Datelike;
    let clamp = |y: i32, m: u32, d: u8| {
        (1..=d)
            .rev()
            .find_map(|d| chrono::NaiveDate::from_ymd_opt(y, m, d.into()))
            .unwrap_or(today)
    };
    let this = clamp(today.year(), today.month(), day);
    if this >= today {
        return this;
    }
    let (y, m) = if today.month() == 12 {
        (today.year() + 1, 1)
    } else {
        (today.year(), today.month() + 1)
    };
    clamp(y, m, day)
}

fn credit_recommendation(payment: &RecurringPayment, today: chrono::NaiveDate) -> Option<String> {
    if payment.method != PaymentMethod::CreditCard {
        return None;
    }
    let closing = payment.statement_closing_day?;
    let due = payment.payment_due_day?;
    let next_closing = next_monthly_day(today, closing);
    let before_closing = next_closing.pred_opt()?;
    let next_due = next_monthly_day(today, due);
    let amount = crate::state::money_usd(payment.amount.cents());
    let amount_note = if payment.amount.cents() > 0 {
        format!("Paga {amount} ")
    } else {
        "Paga el saldo ".to_owned()
    };
    Some(format!(
        "{amount_note}antes del {} (1 día antes del corte) para reportar menor saldo; paga antes del {} para evitar mora/intereses. Orientación general, confirma con tu emisor.",
        crate::state::format_date(before_closing),
        crate::state::format_date(next_due)
    ))
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
            let schedule = match payment.due_date {
                Some(date) => format!("fecha {}", format_date(date)),
                None => format!("día {}", payment.due_day),
            };
            let mut detail = vec![payment.method.label().to_owned(), schedule];
            if payment.method == PaymentMethod::CreditCard {
                if let (Some(close), Some(due)) =
                    (payment.statement_closing_day, payment.payment_due_day)
                {
                    detail.push(format!("corte día {close}"));
                    detail.push(format!("límite día {due}"));
                }
            }
            if let Some(note) = payment.note.as_deref()
                && !note.trim().is_empty()
            {
                detail.push(note.trim().to_owned());
            }
            let recommendation = credit_recommendation(payment, chrono::Local::now().date_naive());
            PaymentRow {
                id: payment.id,
                name: payment.name.clone(),
                amount: money_usd(payment.amount.cents()),
                secondary: money_ves(payment.amount.cents(), rate),
                status: payment_status_text(status),
                kind: payment.kind,
                detail: detail.join(" · "),
                recommendation,
                done: status == PaymentStatus::Paid,
                urgent: matches!(
                    status,
                    PaymentStatus::Overdue(_) | PaymentStatus::DueSoon(_)
                ),
            }
        })
        .collect();
    let name = snapshot.payment_name.clone();
    let amount = snapshot.payment_amount.clone();
    let amount_display = format_cents_input(&amount, snapshot.entry_currency.code());
    let day = snapshot.payment_day.clone();
    let payment_date = snapshot.payment_date.clone();
    let note = snapshot.payment_note.clone();
    let cashea_initial = snapshot.cashea_initial.clone();
    let cashea_installments = snapshot.cashea_installments.clone();
    let cashea_installment = snapshot.cashea_installment.clone();
    let credit_closing_day = snapshot.credit_closing_day.clone();
    let credit_due_day = snapshot.credit_due_day.clone();
    let entry_currency = snapshot.entry_currency;
    let cashea_total = crate::state::parse_cents(&snapshot.payment_amount)
        .map(|cents| format_cents_input(&cents.to_string(), entry_currency.code()))
        .unwrap_or_else(|| "—".into());
    let payment_kind = snapshot.payment_kind;
    let payment_method = snapshot.payment_method;
    let selected = snapshot.payment_category_id;
    let chips: Vec<CategoryChipProps> = snapshot
        .categories
        .iter()
        .map(|category| CategoryChipProps {
            id: category.id,
            icon: icon_for_category(&category.name, category.icon.as_deref()),
            label: category.name.clone(),
            active: selected == Some(category.id),
            state,
        })
        .collect();
    let feedback = snapshot.feedback.clone();
    let recurring_kind = snapshot.recurring_kind;
    drop(snapshot);
    let methods = [
        PaymentMethod::Cashea,
        PaymentMethod::Transfer,
        PaymentMethod::Cash,
        PaymentMethod::CreditCard,
        PaymentMethod::Other,
    ];

    rsx! {
        div { class: "page",
            div { class: "card",
                h3 { "Pagos por hacer" }
                p { class: "muted",
                    "Los mensuales se programan de nuevo al pagarlos; los de fecha única quedan marcados como pagados."
                }
                if rows.is_empty() {
                    p { class: "muted", "Aún no tienes pagos registrados." }
                } else {
                    for row in rows {
                        div { class: "payment-row",
                            div { class: "grow",
                                strong { "{row.name}" }
                                small { class: "muted", "{row.kind.label()}" }
                                small { class: if row.urgent { "muted urgent" } else { "muted" }, "{row.status}" }
                                small { class: "muted", "{row.detail}" }
                                if let Some(recommendation) = row.recommendation {
                                    small { class: "muted", "{recommendation}" }
                                }
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
                                class: "icon-button",
                                aria_label: "Eliminar pago",
                                onclick: move |_| state.write().request_remove_payment(row.id),
                                Icon { name: "close", class: "icon" }
                            }
                        }
                    }
                }
            }

            div { class: "card",
                h3 { "Nuevo pago" }
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
                if payment_method != PaymentMethod::Cashea && payment_method != PaymentMethod::CreditCard {
                    label { class: "field-label", "Tipo de registro" }
                    div { class: "category-grid two",
                        button {
                            class: if recurring_kind == RecurringKind::Payment { "chip active" } else { "chip" },
                            onclick: move |_| state.write().recurring_kind = RecurringKind::Payment,
                            Icon { name: "payments", class: "chip-icon" }
                            span { "Pago" }
                        }
                        button {
                            class: if recurring_kind == RecurringKind::Expense { "chip active" } else { "chip" },
                            onclick: move |_| state.write().recurring_kind = RecurringKind::Expense,
                            Icon { name: "expense", class: "chip-icon" }
                            span { "Gasto recurrente" }
                        }
                    }
                }
                label { class: "field-label", "Método de pago" }
                div { class: "category-grid two",
                    for method in methods {
                        MethodChip {
                            method,
                            icon: match method {
                                PaymentMethod::Cashea => "cashea",
                                PaymentMethod::Transfer => "transfer",
                                PaymentMethod::Cash => "cash",
                                PaymentMethod::CreditCard => "card",
                                PaymentMethod::Other => "other",
                            },
                            active: payment_method == method,
                            state,
                        }
                    }
                }
                if payment_method != PaymentMethod::Cashea {
                    label { class: "field-label", if payment_method == PaymentMethod::CreditCard { "Monto a pagar (variable, puede ser 0)" } else { "Importe" } }
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
                }

                if payment_method != PaymentMethod::Cashea && payment_method != PaymentMethod::CreditCard {
                    label { class: "field-label", "Tipo de pago" }
                    div { class: "category-grid two",
                        button {
                            class: if payment_kind == PaymentKind::Monthly { "chip active" } else { "chip" },
                            onclick: move |_| state.write().payment_kind = PaymentKind::Monthly,
                            Icon { name: "repeat", class: "chip-icon" }
                            span { "Mensual" }
                        }
                        button {
                            class: if payment_kind == PaymentKind::OneTime { "chip active" } else { "chip" },
                            onclick: move |_| state.write().payment_kind = PaymentKind::OneTime,
                            Icon { name: "calendar", class: "chip-icon" }
                            span { "Fecha única" }
                        }
                    }
                }
                if payment_method == PaymentMethod::Cashea {
                    p { class: "muted",
                        "Cashea sin interés: pagas una inicial hoy y el resto en cuotas cada 14 días. La app calcula cada cuota con la tasa BCV vigente en su fecha."
                    }
                    label { class: "field-label", "Pago inicial" }
                    input {
                        class: "amount-input",
                        value: format_cents_input(&cashea_initial, entry_currency.code()),
                        inputmode: "numeric",
                        placeholder: "0.00 (puede ser 0)",
                        oninput: move |event| {
                            let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).collect();
                            let mut state = state.write();
                            state.cashea_initial = digits;
                            state.sync_cashea_total();
                        },
                    }
                    label { class: "field-label", "¿Cuántas cuotas?" }
                    input {
                        class: "text-input",
                        value: cashea_installments,
                        inputmode: "numeric",
                        placeholder: "3",
                        maxlength: "2",
                        oninput: move |event| {
                            let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).collect();
                            let mut state = state.write();
                            state.cashea_installments = digits;
                            state.sync_cashea_total();
                        },
                    }
                    label { class: "field-label", "Monto de cada cuota" }
                    input {
                        class: "amount-input",
                        value: format_cents_input(&cashea_installment, entry_currency.code()),
                        inputmode: "numeric",
                        placeholder: "0.00",
                        oninput: move |event| {
                            let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).collect();
                            let mut state = state.write();
                            state.cashea_installment = digits;
                            state.sync_cashea_total();
                        },
                    }
                    p { class: "muted", "Total de la compra: {cashea_total}" }
                } else if payment_method == PaymentMethod::CreditCard {
                    p { class: "muted",
                        "Tarjeta de crédito: registra día de corte y día límite. El monto es variable; actualízalo cuando llegue el estado de cuenta."
                    }
                    label { class: "field-label", "Día de corte" }
                    input {
                        class: "text-input",
                        value: credit_closing_day,
                        inputmode: "numeric",
                        placeholder: "20",
                        maxlength: "2",
                        oninput: move |event| {
                            let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).collect();
                            state.write().credit_closing_day = digits;
                        },
                    }
                    label { class: "field-label", "Día límite de pago" }
                    input {
                        class: "text-input",
                        value: credit_due_day,
                        inputmode: "numeric",
                        placeholder: "15",
                        maxlength: "2",
                        oninput: move |event| {
                            let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).collect();
                            state.write().credit_due_day = digits;
                        },
                    }
                } else if payment_kind == PaymentKind::Monthly {
                    label { class: "field-label", "Día de vencimiento" }
                    input {
                        class: "text-input",
                        value: day,
                        inputmode: "numeric",
                        placeholder: "25",
                        maxlength: "2",
                        oninput: move |event| { state.write().payment_day = event.value(); },
                    }
                } else {
                    label { class: "field-label", "Fecha del pago" }
                    input {
                        class: "text-input",
                        r#type: "date",
                        value: payment_date,
                        oninput: move |event| { state.write().payment_date = event.value(); },
                    }
                    p { class: "date-hint", "Formato venezolano: dd/mm/aaaa." }
                }

                label { class: "field-label", "Categoría" }
                div { class: "category-grid",
                    for chip in chips {
                        CategoryChip { ..chip }
                    }
                }

                label { class: "field-label", "Nota (opcional)" }
                input {
                    class: "text-input",
                    value: note,
                    placeholder: "Ej. Cuota 3/6",
                    maxlength: "200",
                    oninput: move |event| { state.write().payment_note = event.value(); },
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
