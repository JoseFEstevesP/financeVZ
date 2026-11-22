use dioxus::prelude::*;

use crate::state::{money_usd, money_ves, AppState, View};

#[derive(Props, Clone, PartialEq)]
pub struct IncomeProps {
    pub state: Signal<AppState>,
}

#[component]
pub fn Incomes(props: IncomeProps) -> Element {
    let mut app_state = props.state;
    let state = app_state.read();
    let rate = state.exchange_rate_cents;
    let income_total = state.month_income_total();
    let expense_total = state.month_total();
    let balance = state.month_balance();
    let recent = state.income_recent(30);
    let rows: Vec<_> = recent
        .iter()
        .map(|income| {
            let category = state.income_category_of(income.category_id);
            let cents = income.amount.cents();
            let income_rate = income
                .rate_cents
                .or_else(|| state.rate_value_for(income.local_date))
                .unwrap_or(rate);
            (
                income.id,
                category
                    .and_then(|category| category.icon.clone())
                    .unwrap_or_else(|| "💵".into()),
                category
                    .map(|category| category.name.clone())
                    .unwrap_or_else(|| "Otros".into()),
                income.note.clone(),
                income.local_date.to_string(),
                money_usd(cents),
                money_ves(cents, income_rate),
            )
        })
        .collect();
    drop(state);

    let balance_class = if balance >= 0 { "balance-positive" } else { "balance-negative" };

    rsx! {
        div { class: "page",
            div { class: "hero compact",
                span { class: "muted", "Ingresos de este mes" }
                h2 { class: "amount", "{money_usd(income_total)}" }
                if let Some(ves) = money_ves(income_total, rate) {
                    p { class: "secondary-amount", "{ves}" }
                }
                div { class: "summary-grid",
                    div {
                        span { class: "muted", "Ingresos" }
                        strong { "{money_usd(income_total)}" }
                    }
                    div {
                        span { class: "muted", "Gastos" }
                        strong { "{money_usd(expense_total)}" }
                    }
                    div {
                        span { class: "muted", "Balance" }
                        strong { class: balance_class, "{money_usd(balance)}" }
                    }
                }
                button {
                    class: "primary",
                    onclick: move |_| app_state.write().set_view(View::AddIncome),
                    "+ Añadir ingreso"
                }
            }

            div { class: "card",
                h3 { "Últimos ingresos" }
                if rows.is_empty() {
                    p { class: "muted", "Tus ingresos aparecerán aquí." }
                } else {
                    for (id, icon, category, note, date, amount, secondary) in rows {
                        div { class: "expense-row",
                            div { class: "expense-icon", "{icon}" }
                            div { class: "expense-info",
                                strong { "{category}" }
                                if let Some(note) = note {
                                    small { class: "muted", "{note}" }
                                }
                                small { class: "muted", "{date}" }
                                if let Some(secondary) = secondary {
                                    small { class: "muted", "{secondary}" }
                                }
                            }
                            span { class: "expense-amount", "{amount}" }
                            button {
                                class: "link small-link",
                                onclick: move |_| app_state.write().remove_income(id),
                                "Eliminar"
                            }
                        }
                    }
                }
            }
        }
    }
}
