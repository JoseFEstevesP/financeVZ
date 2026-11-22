use dioxus::prelude::*;

use crate::{
    components::charts::Donut,
    state::{
        money_usd, money_ves, payment_status_text, AppState, View,
    },
};

#[derive(Props, Clone, PartialEq)]
pub struct HomeProps {
    pub state: Signal<AppState>,
    pub go_add: EventHandler<()>,
}

struct RecentRow {
    icon: String,
    category: String,
    date: String,
    amount: String,
    secondary: Option<String>,
    evidence: bool,
}

impl RecentRow {
    fn evidence_label(&self) -> &'static str {
        if self.evidence { " · 📎" } else { "" }
    }
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
                icon: category
                    .and_then(|category| category.icon.clone())
                    .unwrap_or_else(|| "📦".into()),
                category: category
                    .map(|category| category.name.clone())
                    .unwrap_or_else(|| "Otros".into()),
                date: expense.local_date.to_string(),
                amount: money_usd(cents),
                secondary: money_ves(cents, expense_rate),
                evidence: expense.evidence_path.is_some(),
            }
        })
        .collect();
    let segments = report
        .as_ref()
        .map(|report| report.by_category.clone())
        .unwrap_or_default();
    let chart_total = report.as_ref().map(|report| report.total.cents()).unwrap_or(0);
    let balance_class = if balance >= 0 { "balance-positive" } else { "balance-negative" };
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
                if let Some(ves) = money_ves(month_total, rate) {
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
                            div { class: "expense-icon", "{expense.icon}" }
                            div { class: "expense-info",
                                strong { "{expense.category}" }
                                small { class: "muted", "{expense.date}{expense.evidence_label()}" }
                                if let Some(secondary) = expense.secondary {
                                    small { class: "muted", "{secondary}" }
                                }
                            }
                            span { class: "expense-amount", "{expense.amount}" }
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
