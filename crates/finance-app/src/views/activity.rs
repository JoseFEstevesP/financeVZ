use chrono::Local;
use dioxus::prelude::*;
use finance_core::ActivityKind;

use crate::{
    components::icon::Icon,
    state::{AppState, format_activity_moment, format_date, money_usd},
};

#[derive(Props, Clone, PartialEq)]
pub struct ActivityProps {
    pub state: Signal<AppState>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActivityFilter {
    All,
    Expense,
    Income,
    Payment,
}

fn icon_for_kind(kind: ActivityKind) -> &'static str {
    if kind == ActivityKind::ExpenseRefunded {
        return "repeat";
    }
    if kind.is_expense() {
        "expense"
    } else if kind.is_income() {
        "income"
    } else {
        "payments"
    }
}

#[component]
pub fn Activity(props: ActivityProps) -> Element {
    let app_state = props.state;
    let mut filter = use_signal(|| ActivityFilter::All);
    let snapshot = app_state.read();
    let entries: Vec<_> = snapshot.activity.clone();
    let balances = snapshot.activity_running_balances();
    drop(snapshot);

    let selected = filter();
    let entries_with_bal: Vec<_> = entries
        .into_iter()
        .zip(balances.into_iter())
        .filter(|(entry, _)| match selected {
            ActivityFilter::All => true,
            ActivityFilter::Expense => entry.kind.is_expense(),
            ActivityFilter::Income => entry.kind.is_income(),
            ActivityFilter::Payment => entry.kind.is_payment(),
        })
        .collect();

    let mut grouped: Vec<(String, Vec<_>)> = Vec::new();
    for (entry, bal) in entries_with_bal {
        let day = format_date(entry.occurred_at.with_timezone(&Local).date_naive());
        if grouped.last().is_none_or(|(last_day, _)| last_day != &day) {
            grouped.push((day, Vec::new()));
        }
        grouped
            .last_mut()
            .expect("grupo activo")
            .1
            .push((entry, bal));
    }

    rsx! {
        div { class: "page",
            div { class: "card",
                h3 { "Historial de actividad" }
                p { class: "muted",
                    "Registro de altas, cambios y borrados de gastos, ingresos y pagos."
                }
                div { class: "row-buttons",
                    for (value, label) in [
                        (ActivityFilter::All, "Todos"),
                        (ActivityFilter::Expense, "Gastos"),
                        (ActivityFilter::Income, "Ingresos"),
                        (ActivityFilter::Payment, "Pagos"),
                    ] {
                        button {
                            class: if filter() == value { "chip active" } else { "chip" },
                            onclick: move |_| filter.set(value),
                            "{label}"
                        }
                    }
                }
            }

            if grouped.is_empty() {
                div { class: "card",
                    p { class: "muted", "Todavía no hay acciones registradas." }
                }
            } else {
                for (day, items) in grouped {
                    div { class: "card",
                        h3 { "{day}" }
                        for (entry, bal_after) in items {
                            div { class: "expense-row activity-row",
                                div { class: "expense-icon",
                                    Icon { name: icon_for_kind(entry.kind), class: "row-icon" }
                                }
                                div { class: "expense-info",
                                    strong { "{entry.kind.label()}" }
                                    small { class: "muted", "{entry.summary}" }
                                }
                                div { class: "expense-amounts",
                                    span { class: "expense-amount", "{money_usd(entry.amount_cents)}" }
                                    small { class: "muted balance-after", "Total: {money_usd(bal_after)}" }
                                }
                                small { class: "muted",
                                    {format_activity_moment(entry.occurred_at)
                                        .split_once(' ')
                                        .map(|(_, time)| time.to_owned())
                                        .unwrap_or_default()}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
