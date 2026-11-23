use dioxus::prelude::*;
use finance_core::CategoryId;

use crate::{
    components::icon::{Icon, icon_for_category},
    state::{AppState, money_usd},
};

#[derive(Props, Clone, PartialEq)]
pub struct CategoriesProps {
    pub state: Signal<AppState>,
}

struct CategoryRow {
    icon: &'static str,
    name: String,
    color: String,
}

#[component]
pub fn Categories(props: CategoriesProps) -> Element {
    let mut state = props.state;
    let snapshot = state.read();
    let categories: Vec<CategoryRow> = snapshot
        .categories
        .iter()
        .map(|category| CategoryRow {
            icon: icon_for_category(&category.name, category.icon.as_deref()),
            name: category.name.clone(),
            color: category.color.clone(),
        })
        .collect();
    let income_categories: Vec<CategoryRow> = snapshot
        .income_categories
        .iter()
        .map(|category| CategoryRow {
            icon: icon_for_category(&category.name, category.icon.as_deref()),
            name: category.name.clone(),
            color: category.color.clone(),
        })
        .collect();
    let name = snapshot.new_category_name.clone();
    let color = snapshot.new_category_color.clone();
    let income_name = snapshot.new_income_category_name.clone();
    let income_color = snapshot.new_income_category_color.clone();
    let feedback = snapshot.feedback.clone();
    let budget_categories = snapshot.categories.clone();
    let budget_selected = snapshot.budget_category_id;
    let budget_input = snapshot.budget_input.clone();
    let budgets: Vec<(CategoryId, String, i64, i64)> = snapshot
        .categories
        .iter()
        .filter_map(|category| {
            snapshot.budget_for(category.id).map(|limit| {
                let spent = snapshot.spent_this_month(category.id);
                (category.id, category.name.clone(), spent, limit)
            })
        })
        .collect();
    drop(snapshot);

    rsx! {
        div { class: "page",
            div { class: "card",
                h3 { "Categorías" }
                for category in categories {
                    div { class: "category-row",
                        span { class: "dot", style: "background: {category.color};" }
                        Icon { name: category.icon, class: "icon" }
                        span { class: "grow", "{category.name}" }
                    }
                }
            }

            div { class: "card",
                h3 { "Presupuestos mensuales" }
                p { class: "muted",
                    "Define un límite por categoría. El gasto se compara con lo registrado este mes."
                }
                if budgets.is_empty() {
                    p { class: "muted", "Aún no hay presupuestos." }
                } else {
                    for (id, name, spent, limit) in budgets {
                        div { class: "category-row",
                            span { class: "grow",
                                strong { "{name}" }
                                small { class: if spent > 0 && spent >= limit { "muted urgent" } else { "muted" },
                                    "Gastado {money_usd(spent)} de {money_usd(limit)}" }
                            }
                            button {
                                class: "icon-button",
                                aria_label: "Quitar presupuesto",
                                onclick: move |_| state.write().remove_budget(id),
                                Icon { name: "delete", class: "icon" }
                            }
                        }
                    }
                }
                label { class: "field-label", "Categoría" }
                select {
                    class: "text-input",
                    value: budget_selected.map(|id| id.get().to_string()).unwrap_or_default(),
                    onchange: move |event| {
                        state.write().budget_category_id = event.value().parse::<i64>().ok().map(CategoryId::new);
                    },
                    for category in budget_categories {
                        option { value: "{category.id.get()}", "{category.name}" }
                    }
                }
                label { class: "field-label", "Límite en USD (por ejemplo 100.00)" }
                input {
                    class: "text-input",
                    value: budget_input,
                    inputmode: "decimal",
                    placeholder: "100.00",
                    oninput: move |event| state.write().budget_input = event.value(),
                }
                button { class: "secondary wide", onclick: move |_| state.write().save_budget(),
                    "Guardar presupuesto"
                }
            }

            div { class: "card",
                h3 { "Categorías de ingreso" }
                for category in income_categories {
                    div { class: "category-row",
                        span { class: "dot", style: "background: {category.color};" }
                        Icon { name: category.icon, class: "icon" }
                        span { class: "grow", "{category.name}" }
                    }
                }
            }

            div { class: "card",
                h3 { "Nueva categoría" }
                label { class: "field-label", "Nombre" }
                input {
                    class: "text-input",
                    value: name,
                    placeholder: "Ej. Mascotas",
                    maxlength: "40",
                    oninput: move |event| { state.write().new_category_name = event.value(); },
                }
                label { class: "field-label", "Color" }
                input {
                    class: "text-input",
                    value: color,
                    placeholder: "#4f8cff",
                    oninput: move |event| { state.write().new_category_color = event.value(); },
                }
                if let Some(message) = feedback.clone() {
                    p { class: "feedback", "{message}" }
                }
                button { class: "primary wide", onclick: move |_| state.write().add_category(),
                    "Crear categoría"
                }
            }

            div { class: "card",
                h3 { "Nueva categoría de ingreso" }
                label { class: "field-label", "Nombre" }
                input {
                    class: "text-input",
                    value: income_name,
                    placeholder: "Ej. Bonos",
                    maxlength: "40",
                    oninput: move |event| { state.write().new_income_category_name = event.value(); },
                }
                label { class: "field-label", "Color" }
                input {
                    class: "text-input",
                    value: income_color,
                    placeholder: "#2dd4a7",
                    oninput: move |event| { state.write().new_income_category_color = event.value(); },
                }
                if let Some(message) = feedback.clone() {
                    p { class: "feedback", "{message}" }
                }
                button { class: "primary wide", onclick: move |_| state.write().add_income_category(),
                    "Crear categoría de ingreso"
                }
            }
        }
    }
}
