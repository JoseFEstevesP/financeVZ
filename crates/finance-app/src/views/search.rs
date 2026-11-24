use dioxus::prelude::*;
use finance_core::CategoryId;

use crate::{
    components::icon::{Icon, icon_for_category},
    state::{AppState, format_date, money_usd},
};

#[derive(Props, Clone, PartialEq)]
pub struct SearchProps {
    pub state: Signal<AppState>,
}

#[component]
pub fn Search(props: SearchProps) -> Element {
    let mut app_state = props.state;
    let snapshot = app_state.read();
    let query = snapshot.search_query.clone();
    let from = snapshot.search_from.clone();
    let to = snapshot.search_to.clone();
    let categories = snapshot.categories.clone();
    let selected = snapshot.search_category_id;
    let expenses = snapshot.searched_expenses();
    let incomes = snapshot.searched_incomes();
    let expense_total: i64 = expenses.iter().map(|item| item.amount.cents()).sum();
    let income_total: i64 = incomes.iter().map(|item| item.amount.cents()).sum();
    drop(snapshot);

    let expense_rows: Vec<_> = expenses
        .into_iter()
        .map(|expense| {
            let category = app_state
                .read()
                .category_of(expense.category_id)
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "Otros".into());
            let icon = icon_for_category(
                &category,
                app_state
                    .read()
                    .category_of(expense.category_id)
                    .and_then(|item| item.icon.as_deref()),
            );
            (
                expense.id,
                icon,
                category,
                expense.note.clone(),
                format_date(expense.local_date),
                expense.method.label().to_owned(),
                money_usd(expense.amount.cents()),
            )
        })
        .collect();
    let income_rows: Vec<_> = incomes
        .into_iter()
        .map(|income| {
            let category = app_state
                .read()
                .income_category_of(income.category_id)
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "Otros".into());
            let icon = icon_for_category(
                &category,
                app_state
                    .read()
                    .income_category_of(income.category_id)
                    .and_then(|item| item.icon.as_deref()),
            );
            (
                income.id,
                icon,
                category,
                income.note.clone(),
                format_date(income.local_date),
                income.method.label().to_owned(),
                money_usd(income.amount.cents()),
            )
        })
        .collect();

    rsx! {
        div { class: "page",
            div { class: "card",
                h3 { "Buscar movimientos" }
                label { class: "field-label", "Texto (nota o categoría)" }
                input {
                    class: "text-input",
                    value: query,
                    placeholder: "Ej. almuerzo, luz",
                    oninput: move |event| app_state.write().search_query = event.value(),
                }
                label { class: "field-label", "Categoría" }
                select {
                    class: "text-input",
                    value: selected.map(|id| id.get().to_string()).unwrap_or_default(),
                    onchange: move |event| {
                        let value = event.value();
                        app_state.write().search_category_id = value.parse::<i64>().ok().map(CategoryId::new);
                    },
                    option { value: "", "Todas" }
                    for category in categories {
                        option { value: "{category.id.get()}", "{category.name}" }
                    }
                }
                div { class: "date-range",
                    div {
                        label { class: "field-label", "Desde" }
                        input {
                            class: "text-input",
                            r#type: "date",
                            value: from,
                            oninput: move |event| app_state.write().search_from = event.value(),
                        }
                    }
                    div {
                        label { class: "field-label", "Hasta" }
                        input {
                            class: "text-input",
                            r#type: "date",
                            value: to,
                            oninput: move |event| app_state.write().search_to = event.value(),
                        }
                    }
                }
                p { class: "date-hint", "Fechas en formato venezolano: dd/mm/aaaa." }
                p { class: "muted",
                    "Gastos: {money_usd(expense_total)} · Ingresos: {money_usd(income_total)}"
                }
            }

            div { class: "card",
                h3 { "Gastos ({expense_rows.len()})" }
                if expense_rows.is_empty() {
                    p { class: "muted", "Sin resultados." }
                } else {
                    for (id, icon, category, note, date, method, amount) in expense_rows {
                        div { class: "expense-row",
                            div { class: "expense-icon", Icon { name: icon, class: "row-icon" } }
                            div { class: "expense-info",
                                strong { "{category}" }
                                if let Some(note) = note {
                                    small { class: "muted", "{note}" }
                                }
                                small { class: "muted", "{date} · {method}" }
                            }
                            span { class: "expense-amount", "{amount}" }
                            button {
                                class: "icon-button",
                                aria_label: "Devolver gasto",
                                onclick: move |_| app_state.write().refund_expense(id),
                                Icon { name: "income", class: "icon" }
                            }
                            button {
                                class: "icon-button",
                                aria_label: "Editar gasto",
                                onclick: move |_| app_state.write().begin_edit_expense(id),
                                Icon { name: "edit", class: "icon" }
                            }
                            button {
                                class: "icon-button",
                                aria_label: "Eliminar gasto",
                                onclick: move |_| app_state.write().request_remove_expense(id),
                                Icon { name: "delete", class: "icon" }
                            }
                        }
                    }
                }
            }

            div { class: "card",
                h3 { "Ingresos ({income_rows.len()})" }
                if income_rows.is_empty() {
                    p { class: "muted", "Sin resultados." }
                } else {
                    for (id, icon, category, note, date, method, amount) in income_rows {
                        div { class: "expense-row",
                            div { class: "expense-icon", Icon { name: icon, class: "row-icon" } }
                            div { class: "expense-info",
                                strong { "{category}" }
                                if let Some(note) = note {
                                    small { class: "muted", "{note}" }
                                }
                                small { class: "muted", "{date} · {method}" }
                            }
                            span { class: "expense-amount", "{amount}" }
                            button {
                                class: "icon-button",
                                aria_label: "Editar ingreso",
                                onclick: move |_| app_state.write().begin_edit_income(id),
                                Icon { name: "edit", class: "icon" }
                            }
                            button {
                                class: "icon-button",
                                aria_label: "Eliminar ingreso",
                                onclick: move |_| app_state.write().request_remove_income(id),
                                Icon { name: "delete", class: "icon" }
                            }
                        }
                    }
                }
            }
        }
    }
}
