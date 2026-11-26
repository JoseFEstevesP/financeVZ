use dioxus::prelude::*;

use crate::{
    components::{
        charts::Donut,
        icon::{Icon, icon_for_category},
    },
    state::{AppState, View, display_ves, format_date, money_usd, money_ves, payment_status_text},
};

#[derive(Props, Clone, PartialEq)]
pub struct HomeProps {
    pub state: Signal<AppState>,
    pub go_add: EventHandler<()>,
}

struct RecentRow {
    id: finance_core::ExpenseId,
    icon: &'static str,
    category: String,
    date: String,
    method: String,
    amount: String,
    secondary: Option<String>,
    evidence: Option<String>,
}

#[component]
pub fn Home(props: HomeProps) -> Element {
    let mut app_state = props.state;
    let state = app_state.read();
    let month_total = state.month_total();
    let month_income = state.month_income_total();
    let balance = state.month_balance();
    let rate = state.exchange_rate_cents;
    let due = state.due_payments();
    let report = state.report();
    let recent: Vec<RecentRow> = state
        .recent(6)
        .iter()
        .map(|expense| {
            let category = state.category_of(expense.category_id);
            let cents = expense.amount.cents();
            let expense_rate = expense
                .rate_cents
                .or_else(|| state.rate_value_for(expense.local_date))
                .unwrap_or(rate);
            RecentRow {
                id: expense.id,
                icon: icon_for_category(
                    category
                        .map(|category| category.name.as_str())
                        .unwrap_or("Otros"),
                    category.and_then(|category| category.icon.as_deref()),
                ),
                category: category
                    .map(|category| category.name.clone())
                    .unwrap_or_else(|| "Otros".into()),
                date: format_date(expense.local_date),
                method: expense.method.label().to_owned(),
                amount: money_usd(cents),
                secondary: display_ves(
                    cents,
                    expense_rate,
                    expense.original_amount_cents,
                    expense.original_currency.as_deref(),
                ),
                evidence: expense.evidence_path.clone(),
            }
        })
        .collect();
    let segments = report
        .as_ref()
        .map(|report| report.by_category.clone())
        .unwrap_or_default();
    let chart_total = report
        .as_ref()
        .map(|report| report.total.cents())
        .unwrap_or(0);
    let balance_class = if balance >= 0 {
        "balance-positive"
    } else {
        "balance-negative"
    };
    let month_total_ves = state
        .month_total_ves(rate)
        .or_else(|| money_ves(month_total, rate));
    let rate_label = state.latest_rate_label();
    drop(state);

    rsx! {
        div { class: "page",
            if !due.is_empty() {
                div { class: "alert",
                    strong { "Pagos por hacer: {due.len()}" }
                    for (payment, status) in due.iter().take(2) {
                        div { class: "alert-row",
                            span { class: "grow", "{payment.name}" }
                            span { "{payment_status_text(*status)}" }
                        }
                    }
                    button {
                        class: "link",
                        onclick: move |_| app_state.write().set_view(View::Payments),
                        "Ver pagos →"
                    }
                }
            }

            div { class: "hero",
                span { class: "muted", "Gastos de este mes" }
                h2 { class: "amount", "{money_usd(month_total)}" }
                if let Some(ves) = month_total_ves.clone() {
                    p { class: "secondary-amount", "{ves}" }
                }
                div { class: "row-buttons",
                    button { class: "primary", onclick: move |_| props.go_add.call(()), "+ Añadir gasto" }
                    button { class: "chip", onclick: move |_| app_state.write().set_view(View::AddIncome),
                        "+ Ingreso"
                    }
                }
            }

            div { class: "card",
                h3 { "Resumen del mes" }
                div { class: "summary-grid",
                    div {
                        span { class: "muted", "Ingresos" }
                        strong { "{money_usd(month_income)}" }
                    }
                    div {
                        span { class: "muted", "Gastos" }
                        strong { "{money_usd(month_total)}" }
                    }
                    div {
                        span { class: "muted", "Balance" }
                        strong { class: balance_class, "{money_usd(balance)}" }
                    }
                }
                button {
                    class: "link",
                    onclick: move |_| app_state.write().set_view(View::Income),
                    "Ver ingresos →"
                }
            }

            div { class: "card",
                h3 { "Tasa de cambio" }
                p { class: "muted", "{rate_label}" }
                p { class: "muted",
                    "Los importes se guardan en USD. Los bolívares son solo una referencia visual calculada con la tasa vigente en la fecha de cada movimiento."
                }
            }

            div { class: "card",
                h3 { "Más opciones" }
                div { class: "row-buttons",
                    button {
                        class: "chip",
                        onclick: move |_| app_state.write().set_view(View::Search),
                        Icon { name: "search", class: "chip-icon" }
                        span { "Buscar movimientos" }
                    }
                    button {
                        class: "chip",
                        onclick: move |_| app_state.write().set_view(View::Activity),
                        Icon { name: "note", class: "chip-icon" }
                        span { "Historial de actividad" }
                    }
                }
            }

            div { class: "card",
                h3 { "Distribución total" }
                if segments.is_empty() {
                    p { class: "muted", "Aún no hay gastos registrados." }
                } else {
                    div { class: "donut-wrap",
                        Donut { items: segments.clone(), total: chart_total }
                    }
                    div { class: "legend",
                        for item in segments.iter().take(4) {
                            div { class: "legend-row",
                                span { class: "dot", style: "background: {item.category.color};" }
                                span { class: "legend-name", "{item.category.name}" }
                                span { class: "legend-value", "{item.percentage as i32}%" }
                            }
                        }
                    }
                }
            }

            div { class: "card",
                h3 { "Últimos gastos" }
                if recent.is_empty() {
                    p { class: "muted", "Tus gastos aparecerán aquí." }
                } else {
                    for expense in recent {
                        div { class: "expense-row",
                            div { class: "expense-icon", Icon { name: expense.icon, class: "row-icon" } }
                            div { class: "expense-info",
                                strong { "{expense.category}" }
                                small { class: "muted",
                                    "{expense.date} · {expense.method}"
                                    if let Some(path) = expense.evidence {
                                        button {
                                            class: "icon-button",
                                            aria_label: "Ver evidencia",
                                            onclick: move |_| app_state.write().show_evidence(&path),
                                            Icon { name: "paperclip", class: "inline-icon" }
                                        }
                                    }
                                }
                                if let Some(secondary) = expense.secondary {
                                    small { class: "muted", "{secondary}" }
                                }
                            }
                            span { class: "expense-amount", "{expense.amount}" }
                            button {
                                class: "icon-button",
                                aria_label: "Devolver gasto",
                                onclick: move |_| app_state.write().refund_expense(expense.id),
                                Icon { name: "income", class: "icon" }
                            }
                            button {
                                class: "icon-button",
                                aria_label: "Editar gasto",
                                onclick: move |_| app_state.write().begin_edit_expense(expense.id),
                                Icon { name: "edit", class: "icon" }
                            }
                            button {
                                class: "icon-button",
                                aria_label: "Eliminar gasto",
                                onclick: move |_| app_state.write().request_remove_expense(expense.id),
                                Icon { name: "close", class: "icon" }
                            }
                        }
                    }
                }
                button {
                    class: "link",
                    onclick: move |_| app_state.write().set_view(View::Reports),
                    "Ver reportes →"
                }
            }
        }
    }
}
