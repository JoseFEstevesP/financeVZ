use dioxus::prelude::*;
use finance_core::MovementMethod;

use crate::{components::icon::Icon, state::AppState};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MethodTarget {
    Expense,
    Income,
}

#[component]
pub fn MethodChips(state: Signal<AppState>, target: MethodTarget) -> Element {
    let selected = {
        let snapshot = state.read();
        match target {
            MethodTarget::Expense => snapshot.expense_method,
            MethodTarget::Income => snapshot.income_method,
        }
    };
    rsx! {
        label { class: "field-label", "Tipo de movimiento" }
        div { class: "category-grid two",
            for method in MovementMethod::ALL {
                button {
                    class: if selected == method { "chip active" } else { "chip" },
                    onclick: move |_| {
                        let mut state = state.write();
                        match target {
                            MethodTarget::Expense => state.expense_method = method,
                            MethodTarget::Income => state.income_method = method,
                        }
                    },
                    Icon { name: method.icon(), class: "chip-icon" }
                    span { "{method.label()}" }
                }
            }
        }
    }
}
