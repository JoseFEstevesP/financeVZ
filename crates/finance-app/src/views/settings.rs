use dioxus::prelude::*;

#[cfg(feature = "self-update")]
use crate::views::update::Update;
use crate::{
    bridge,
    state::{AppState, View},
};

#[derive(Props, Clone, PartialEq)]
pub struct SettingsProps {
    pub state: Signal<AppState>,
}

#[component]
pub fn Settings(props: SettingsProps) -> Element {
    let mut app_state = props.state;
    let snapshot = app_state.read();
    let feedback = snapshot.feedback.clone();
    let backup_json = snapshot.import_input.clone();
    let has_pin = snapshot.pin_hash.is_some();
    let biometric_enabled = snapshot.biometric_enabled;
    let setup_pin = snapshot.setup_pin.clone();
    let confirm_pin = snapshot.confirm_pin.clone();
    let pin_input = snapshot.pin_input.clone();
    let pin_error = snapshot.pin_error.clone();
    let text_scale = snapshot.text_scale.clone();
    let theme = snapshot.theme.clone();
    let auto_backup_hours = snapshot.auto_backup_hours;
    let auto_backup_status = snapshot.auto_backup_status();
    drop(snapshot);

    let biometric_hint = if has_pin {
        "Guarda tu huella del teléfono para entrar sin escribir el PIN."
    } else {
        "La huella necesita un PIN de respaldo: activa el PIN y luego guarda tu huella."
    };

    #[cfg(feature = "self-update")]
    let update_section = rsx! { Update { state: app_state } };
    #[cfg(not(feature = "self-update"))]
    let update_section = rsx! {
        div { class: "card",
            h3 { "Actualizaciones" }
            p { class: "muted",
                "Esta versión se actualiza desde la tienda donde la instalaste."
            }
        }
    };

    rsx! {
        div { class: "page",
            div { class: "card",
                h3 { "Tema" }
                p { class: "muted", "El tema oscuro es el predeterminado. Puedes cambiarlo cuando quieras." }
                div { class: "row-buttons",
                    for (value, label) in [("dark", "Oscuro"), ("light", "Claro")] {
                        button {
                            class: if theme == value { "chip active" } else { "chip" },
                            onclick: move |_| app_state.write().set_theme(value),
                            "{label}"
                        }
                    }
                }
            }

            div { class: "card",
                h3 { "Historial" }
                p { class: "muted", "Altas, cambios y borrados de gastos, ingresos y pagos." }
                button {
                    class: "secondary wide",
                    onclick: move |_| app_state.write().set_view(View::Activity),
                    "Ver historial de actividad"
                }
            }

            div { class: "card",
                h3 { "Copia de seguridad" }
                p { class: "muted",
                    "Guarda una copia completa (categorías, gastos, ingresos, pagos, tasas y ajustes) en un archivo JSON."
                }
                div { class: "row-buttons",
                    button {
                        class: "primary",
                        onclick: move |_| {
                            if let Some(json) = app_state.write().export_backup_json() {
                                bridge::request_backup_download(&json);
                            }
                        },
                        "Exportar y descargar"
                    }
                }
                label { class: "field-label", "Restaurar desde un archivo .json" }
                label { class: "file-drop",
                    span { class: "file-drop-icon", crate::components::icon::Icon { name: "note", class: "icon" } }
                    span { class: "file-drop-copy",
                        strong { "Elegir copia" }
                        small { class: "muted", "Reemplaza TODOS los datos actuales" }
                    }
                    input {
                        id: "backup-file",
                        class: "file-input-native",
                        r#type: "file",
                        accept: "application/json,.json",
                    }
                }
                label { class: "field-label", "O pega el JSON de la copia" }
                textarea {
                    id: "backup-json",
                    class: "text-input backup-json",
                    value: backup_json,
                    rows: "5",
                    oninput: move |event| app_state.write().import_input = event.value(),
                }
                button {
                    class: "secondary spaced",
                    onclick: move |_| {
                        let json = app_state.read().import_input.clone();
                        app_state.write().import_backup_text(&json);
                    },
                    "Restaurar copia pegada"
                }

                h4 { class: "subheading", "Copia automática" }
                p { class: "muted", "{auto_backup_status}" }
                p { class: "muted",
                    "Se guarda en la carpeta privada de la app y se conservan las últimas 3 copias. Para sacarla, usa «Exportar última copia automática»."
                }
                div { class: "row-buttons",
                    for hours in [6_i64, 12, 24, 48] {
                        button {
                            class: if auto_backup_hours == hours { "chip active" } else { "chip" },
                            onclick: move |_| app_state.write().set_auto_backup_hours(hours),
                            "{hours} h"
                        }
                    }
                }
                div { class: "row-buttons",
                    button {
                        class: "secondary spaced",
                        onclick: move |_| {
                            let json = app_state.write().auto_backup_now();
                            if let Some(json) = json {
                                bridge::request_backup_download(&json);
                            }
                        },
                        "Crear y exportar copia ahora"
                    }
                    button {
                        class: "secondary spaced",
                        onclick: move |_| {
                            if let Some(json) = app_state.write().latest_auto_backup_json() {
                                bridge::request_backup_download(&json);
                            } else {
                                app_state.write().feedback = Some("No hay copia automática para exportar.".into());
                            }
                        },
                        "Exportar última copia automática"
                    }
                }
            }

            div { class: "card",
                h3 { "Seguridad" }
                if has_pin {
                    p { class: "muted", "PIN activado. Se pide al abrir la app." }
                    label { class: "field-label", "PIN actual (para quitarlo)" }
                    input {
                        class: "text-input",
                        value: pin_input,
                        inputmode: "numeric",
                        maxlength: "4",
                        r#type: "password",
                        oninput: move |event| {
                            let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).take(4).collect();
                            app_state.write().pin_input = digits;
                        },
                    }
                    button { class: "secondary spaced", onclick: move |_| app_state.write().remove_pin(),
                        "Quitar PIN"
                    }
                    button { class: "link", onclick: move |_| app_state.write().lock(),
                        "Bloquear ahora"
                    }
                } else {
                    p { class: "muted", "Protege el acceso con un PIN de 4 dígitos." }
                    label { class: "field-label", "Nuevo PIN" }
                    input {
                        class: "text-input",
                        value: setup_pin,
                        inputmode: "numeric",
                        maxlength: "4",
                        r#type: "password",
                        oninput: move |event| {
                            let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).take(4).collect();
                            app_state.write().setup_pin = digits;
                        },
                    }
                    label { class: "field-label", "Repite el PIN" }
                    input {
                        class: "text-input",
                        value: confirm_pin,
                        inputmode: "numeric",
                        maxlength: "4",
                        r#type: "password",
                        oninput: move |event| {
                            let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).take(4).collect();
                            app_state.write().confirm_pin = digits;
                        },
                    }
                    button { class: "primary spaced", onclick: move |_| app_state.write().set_pin(),
                        "Activar PIN"
                    }
                }
                h4 { class: "subheading", "Huella" }
                if biometric_enabled {
                    p { class: "muted spaced",
                        "La huella está guardada. Puedes entrar con ella o con el PIN."
                    }
                    button {
                        class: "secondary spaced",
                        onclick: move |_| app_state.write().remove_biometric(),
                        "Quitar huella"
                    }
                } else {
                    p { class: "muted spaced", "{biometric_hint}" }
                    button {
                        class: "primary spaced",
                        onclick: move |_| app_state.write().request_biometric("enroll"),
                        "Guardar huella"
                    }
                }
                if let Some(error) = pin_error {
                    p { class: "feedback", "{error}" }
                }
            }

            div { class: "card",
                h3 { "Tamaño de texto" }
                div { class: "category-grid two",
                    for (value, label) in [("small", "Pequeño"), ("normal", "Normal"), ("large", "Grande")] {
                        button {
                            class: if text_scale == value { "chip active" } else { "chip" },
                            onclick: move |_| app_state.write().set_text_scale(value),
                            "{label}"
                        }
                    }
                }
            }

            {update_section}

            if let Some(message) = feedback {
                p { class: "feedback", "{message}" }
            }
        }
    }
}
