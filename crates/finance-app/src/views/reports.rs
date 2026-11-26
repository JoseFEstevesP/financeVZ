use dioxus::prelude::*;

use crate::{
    components::charts::{Bars, Donut},
    state::{AppState, format_date, money_usd, money_ves},
};

#[derive(Props, Clone, PartialEq)]
pub struct ReportsProps {
    pub state: Signal<AppState>,
}

#[component]
pub fn Reports(props: ReportsProps) -> Element {
    let mut state = props.state;
    let snapshot = state.read();
    let report = snapshot.report();
    let income_report = snapshot.income_report();
    let total = report.as_ref().map(|r| r.total.cents()).unwrap_or(0);
    let income_total = income_report.as_ref().map(|r| r.total.cents()).unwrap_or(0);
    let income_by_category = income_report
        .as_ref()
        .map(|r| r.by_category.clone())
        .unwrap_or_default();
    let month_income = snapshot.month_income_total();
    let month_expense = snapshot.month_total();
    let balance = snapshot.month_balance();
    let by_category = report
        .as_ref()
        .map(|r| r.by_category.clone())
        .unwrap_or_default();
    let by_month = report
        .as_ref()
        .map(|r| r.by_month.clone())
        .unwrap_or_default();
    let rate = snapshot.exchange_rate_cents;
    let rate_input = snapshot.rate_input.clone();
    let feedback = snapshot.feedback.clone();
    let secondary = money_ves(total, rate);
    let balance_class = if balance >= 0 {
        "balance-positive"
    } else {
        "balance-negative"
    };
    let rate_history: Vec<_> = snapshot.rate_history.iter().take(8).cloned().collect();
    drop(snapshot);

    rsx! {
        div { class: "page",
            div { class: "hero compact",
                span { class: "muted", "Gastos registrados" }
                h2 { class: "amount", "{money_usd(total)}" }
                if let Some(ves) = secondary {
                    p { class: "secondary-amount", "{ves}" }
                }
            }

            div { class: "card",
                h3 { "Balance del mes" }
                div { class: "summary-grid",
                    div {
                        span { class: "muted", "Ingresos" }
                        strong { "{money_usd(month_income)}" }
                    }
                    div {
                        span { class: "muted", "Gastos" }
                        strong { "{money_usd(month_expense)}" }
                    }
                    div {
                        span { class: "muted", "Balance" }
                        strong { class: balance_class, "{money_usd(balance)}" }
                    }
                }
            }

            div { class: "card",
                h3 { "Ingresos por categoría" }
                if income_by_category.is_empty() {
                    p { class: "muted", "Sin ingresos registrados." }
                } else {
                    div { class: "donut-wrap",
                        Donut { items: income_by_category.clone(), total: income_total }
                    }
                    div { class: "legend",
                        for item in income_by_category.iter() {
                            div { class: "legend-row",
                                span { class: "dot", style: "background: {item.category.color};" }
                                span { class: "legend-name", "{item.category.name}" }
                                span { class: "legend-value", "{money_usd(item.total.cents())}" }
                            }
                        }
                    }
                }
            }

            div { class: "card",
                h3 { "Tasa del dólar" }
                p { class: "muted", "Tasa automática (DolarAPI, con respaldo BCV). Si falla, puedes escribirla a mano." }
                input {
                    class: "text-input",
                    value: rate_input,
                    inputmode: "decimal",
                    placeholder: "857,89",
                    oninput: move |event| { state.write().rate_input = event.value(); },
                }
                div { class: "row-buttons",
                    button { class: "primary", onclick: move |_| state.write().update_rate_from_api(),
                        "Actualizar"
                    }
                    button { class: "chip", onclick: move |_| state.write().save_rate(),
                        "Guardar tasa"
                    }
                }
                if rate > 0 {
                    p { class: "muted", "Bs por USD: {rate / 100},{rate % 100:02}" }
                }
                if !rate_history.is_empty() {
                    h4 { class: "subheading", "Historial" }
                    for (day, cents, source, _) in rate_history {
                        div { class: "category-row",
                            span { class: "grow", "{format_date(day)}" }
                            span { "{cents / 100},{cents % 100:02}" }
                            small { class: "muted", "{source}" }
                        }
                    }
                }
                if let Some(message) = feedback {
                    p { class: "feedback", "{message}" }
                }
            }

            div { class: "card",
                h3 { "Por categoría" }
                if by_category.is_empty() {
                    p { class: "muted", "Sin datos para mostrar." }
                } else {
                    div { class: "donut-wrap",
                        Donut { items: by_category.clone(), total }
                    }
                    div { class: "legend",
                        for item in by_category.iter() {
                            div { class: "legend-row",
                                span { class: "dot", style: "background: {item.category.color};" }
                                span { class: "legend-name", "{item.category.name}" }
                                span { class: "legend-value", "{money_usd(item.total.cents())}" }
                            }
                        }
                    }
                }
            }

            div { class: "card",
                h3 { "Por mes" }
                if by_month.is_empty() {
                    p { class: "muted", "Sin datos para mostrar." }
                } else {
                    Bars { items: by_month }
                }
            }
        }
    }
}
