use dioxus::prelude::*;

use crate::state::AppState;

#[derive(Props, Clone, PartialEq)]
pub struct CategoriesProps {
    pub state: Signal<AppState>,
}

struct CategoryRow {
    icon: String,
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
            icon: category.icon.clone().unwrap_or_else(|| "📦".to_owned()),
            name: category.name.clone(),
            color: category.color.clone(),
        })
        .collect();
    let name = snapshot.new_category_name.clone();
    let color = snapshot.new_category_color.clone();
    let feedback = snapshot.feedback.clone();
    drop(snapshot);

    rsx! {
        div { class: "page",
            div { class: "card",
                h3 { "Categorías" }
                for category in categories {
                    div { class: "category-row",
                        span { class: "dot", style: "background: {category.color};" }
                        span { "{category.icon}" }
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
                if let Some(message) = feedback {
                    p { class: "feedback", "{message}" }
                }
                button { class: "primary wide", onclick: move |_| state.write().add_category(),
                    "Crear categoría"
                }
            }

        }
    }
}
