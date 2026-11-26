use chrono::{Datelike, Local};
use dioxus::prelude::*;
use finance_core::{CategoryId, ExpenseId, IncomeId, RecurringPaymentId};

use crate::{
    components::icon::{Icon, icon_for_category},
    state::{AppState, SearchKind, display_ves, format_date, money_usd, payment_status_text},
};

#[derive(Props, Clone, PartialEq)]
pub struct SearchProps {
    pub state: Signal<AppState>,
}

enum DetailKind {
    Expense(ExpenseId),
    Income(IncomeId),
    Payment(RecurringPaymentId),
}

struct DetailRow {
    sort_key: String,
    date: String,
    kind: &'static str,
    icon: &'static str,
    name: String,
    note: Option<String>,
    method: String,
    amount_cents: i64,
    bs: Option<String>,
    status: Option<String>,
    entity: DetailKind,
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
    let kind = snapshot.search_kind;
    let rate = snapshot.exchange_rate_cents;
    let expense_categories = snapshot.categories.clone();
    let income_categories = snapshot.income_categories.clone();
    let expenses = snapshot.searched_expenses();
    let incomes = snapshot.searched_incomes();
    let payments = snapshot.searched_payments();
    let expense_total: i64 = expenses.iter().map(|item| item.amount.cents()).sum();
    let income_total: i64 = incomes.iter().map(|item| item.amount.cents()).sum();
    let payment_total: i64 = payments.iter().map(|item| item.amount.cents()).sum();
    drop(snapshot);

    let today = Local::now().date_naive();
    let mut rows: Vec<DetailRow> = Vec::new();

    if kind != SearchKind::Income && kind != SearchKind::Payment {
        for expense in expenses {
            let category = expense_categories
                .iter()
                .find(|item| item.id == expense.category_id);
            let name = category
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "Otros".into());
            let icon = icon_for_category(&name, category.and_then(|item| item.icon.as_deref()));
            let amount_cents = expense.amount.cents();
            rows.push(DetailRow {
                sort_key: expense.local_date.to_string(),
                date: format_date(expense.local_date),
                kind: "Gasto",
                icon,
                name,
                note: expense.note.clone(),
                method: expense.method.label().to_owned(),
                bs: expense
                    .rate_cents
                    .and_then(|row_rate| {
                        display_ves(
                            amount_cents,
                            row_rate,
                            expense.original_amount_cents,
                            expense.original_currency.as_deref(),
                        )
                    })
                    .or_else(|| {
                        display_ves(
                            amount_cents,
                            rate,
                            expense.original_amount_cents,
                            expense.original_currency.as_deref(),
                        )
                    }),
                amount_cents,
                status: None,
                entity: DetailKind::Expense(expense.id),
            });
        }
    }

    if kind != SearchKind::Expense && kind != SearchKind::Payment {
        for income in incomes {
            let category = income_categories
                .iter()
                .find(|item| item.id == income.category_id);
            let name = category
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "Otros".into());
            let icon = icon_for_category(&name, category.and_then(|item| item.icon.as_deref()));
            let amount_cents = income.amount.cents();
            rows.push(DetailRow {
                sort_key: income.local_date.to_string(),
                date: format_date(income.local_date),
                kind: "Ingreso",
                icon,
                name,
                note: income.note.clone(),
                method: income.method.label().to_owned(),
                bs: income
                    .rate_cents
                    .and_then(|row_rate| {
                        display_ves(
                            amount_cents,
                            row_rate,
                            income.original_amount_cents,
                            income.original_currency.as_deref(),
                        )
                    })
                    .or_else(|| {
                        display_ves(
                            amount_cents,
                            rate,
                            income.original_amount_cents,
                            income.original_currency.as_deref(),
                        )
                    }),
                amount_cents,
                status: None,
                entity: DetailKind::Income(income.id),
            });
        }
    }

    if kind != SearchKind::Expense && kind != SearchKind::Income {
        for payment in payments {
            let category = expense_categories
                .iter()
                .find(|item| item.id == payment.category_id)
                .map(|item| item.name.clone())
                .unwrap_or_else(|| "Otros".into());
            let amount_cents = payment.amount.cents();
            let due = payment.due_date(today.year(), today.month());
            rows.push(DetailRow {
                sort_key: due.to_string(),
                date: format_date(due),
                kind: "Pago",
                icon: "payments",
                name: format!("{category} · {}", payment.name),
                note: payment.note.clone(),
                method: payment.method.label().to_owned(),
                bs: display_ves(amount_cents, rate, None, None),
                amount_cents,
                status: Some(payment_status_text(payment.status(today))),
                entity: DetailKind::Payment(payment.id),
            });
        }
    }

    rows.sort_by(|left, right| right.sort_key.cmp(&left.sort_key));

    rsx! {
        div { class: "page",
            div { class: "card",
                h3 { "Buscar movimientos" }
                div { class: "row-buttons",
                    for (value, label) in [
                        (SearchKind::All, "Todos"),
                        (SearchKind::Expense, "Gastos"),
                        (SearchKind::Income, "Ingresos"),
                        (SearchKind::Payment, "Pagos"),
                    ] {
                        button {
                            class: if kind == value { "chip active" } else { "chip" },
                            onclick: move |_| app_state.write().search_kind = value,
                            "{label}"
                        }
                    }
                }
                label { class: "field-label", "Texto (nota, categoría o pago)" }
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
                    "Gastos: {money_usd(expense_total)} · Ingresos: {money_usd(income_total)} · Pagos: {money_usd(payment_total)}"
                }
            }

            div { class: "card",
                h3 { "Detalle ({rows.len()})" }
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Fecha" }
                                th { "Tipo" }
                                th { "Categoría / Nombre" }
                                th { "Nota" }
                                th { "Método" }
                                th { class: "num", "USD" }
                                th { class: "num", "Bs" }
                                th { "Estado" }
                                th { "Acciones" }
                            }
                        }
                        tbody {
                            if rows.is_empty() {
                                tr {
                                    td { class: "muted", colspan: "9", "Sin resultados." }
                                }
                            }
                            for row in rows {
                                tr {
                                    td { "{row.date}" }
                                    td { "{row.kind}" }
                                    td {
                                        div { style: "display:flex; align-items:center; gap:.5rem;",
                                            Icon { name: row.icon, class: "icon" }
                                            strong { "{row.name}" }
                                        }
                                    }
                                    td { class: "table-note",
                                        if let Some(note) = row.note { "{note}" }
                                    }
                                    td { "{row.method}" }
                                    td { class: "num", "{money_usd(row.amount_cents)}" }
                                    td { class: "num",
                                        if let Some(bs) = row.bs { "{bs}" }
                                    }
                                    td {
                                        if let Some(status) = row.status { "{status}" }
                                    }
                                    td { class: "actions",
                                        match row.entity {
                                            DetailKind::Expense(id) => rsx! {
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
                                            },
                                            DetailKind::Income(id) => rsx! {
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
                                            },
                                            DetailKind::Payment(id) => rsx! {
                                                button {
                                                    class: "chip",
                                                    onclick: move |_| app_state.write().pay_payment(id),
                                                    "Pagar"
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
