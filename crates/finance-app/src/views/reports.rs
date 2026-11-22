use dioxus::prelude::*;

use crate::{
    components::charts::{Bars, Donut},
    state::{money_usd, money_ves, AppState},
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
    let total = report.as_ref().map(|r| r.total.cents()).unwrap_or(0);
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
    drop(snapshot);

    rsx! {
        div { class: "page",
            div { class: "hero compact",
                span { class: "muted", "Total registrado" }
                h2 { class: "amount", "{money_usd(total)}" }
                if let Some(ves) = secondary {
                    p { class: "secondary-amount", "{ves}" }
                }
            }

            div { class: "card",
                h3 { "Tasa del dólar" }
                p { class: "muted", "Dólar oficial (DolarAPI). Si falla, puedes escribirla a mano." }
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
