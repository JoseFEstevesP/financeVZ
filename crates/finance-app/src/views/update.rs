use dioxus::prelude::*;

use crate::{
    platform::{self, UpdateProgress},
    state::AppState,
};

#[derive(Props, Clone, PartialEq)]
pub struct UpdateProps {
    pub state: Signal<AppState>,
}

#[component]
pub fn Update(props: UpdateProps) -> Element {
    let mut state = props.state;
    let progress = use_signal_sync(|| None::<UpdateProgress>);
    let mut result = use_signal_sync(|| None::<String>);
    let snapshot = state.read();
    let update_url = snapshot.update_url.clone();
    let update_status = snapshot.update_status.clone();
    drop(snapshot);
    let (installed_code, installed_name) = platform::local_version();
    let cached_apk = platform::downloaded_apk_path().is_some();
    let source_label = if update_url.trim().is_empty() {
        "GitHub Releases (JoseFEstevesP/financeVZ)".to_owned()
    } else {
        format!("servidor local ({})", update_url.trim())
    };
    let (progress_label, progress_width, indeterminate) = match progress() {
        Some(UpdateProgress::Checking) => ("Buscando actualización…".to_owned(), 0, true),
        Some(UpdateProgress::Downloading { downloaded, total }) => {
            let label = match total {
                Some(total) => format!(
                    "Descargando {:.1} / {:.1} MB",
                    downloaded as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0
                ),
                None => format!("Descargando {:.1} MB", downloaded as f64 / 1_048_576.0),
            };
            let percent = total
                .map(|total| ((downloaded as f64 / total as f64) * 100.0).round() as usize)
                .unwrap_or(0)
                .min(100);
            (label, percent, total.is_none())
        }
        Some(UpdateProgress::Installing) => ("Instalando APK…".to_owned(), 100, true),
        None => (String::new(), 0, false),
    };
    let status = result().or(update_status);

    rsx! {
        div { class: "page",
            div { class: "card",
                h3 { "Actualizar app" }
                p { class: "muted",
                    "Versión instalada: {installed_name} (código {installed_code})."
                }
                p { class: "muted", "Fuente actual: {source_label}." }
                p { class: "muted",
                    "Se busca la última versión publicada y se descarga el APK. La instalación siempre la confirmas tú con un clic; nunca se instala automáticamente."
                }
                details { class: "update-advanced",
                    summary { "Usar un servidor local (opciones avanzadas)" }
                    p { class: "muted",
                        "Deja la URL vacía para usar GitHub Releases. Para probar en local, inicia el servidor en la PC con la misma Wi-Fi y escribe solo la raíz, sin /update.json ni el nombre del APK."
                    }
                    label { class: "field-label", "URL del servidor" }
                    input {
                        class: "update-input",
                        value: update_url,
                        placeholder: "http://192.168.0.10:8000",
                        inputmode: "url",
                        oninput: move |event| state.write().set_update_url(event.value()),
                    }
                }
                div { class: "update-actions",
                    if progress().is_some() {
                        p { class: "update-status", "{progress_label}" }
                        div { class: "progress-track",
                            div {
                                class: if indeterminate { "progress-fill indeterminate" } else { "progress-fill" },
                                style: format!("width: {}%", progress_width),
                            }
                        }
                    }
                    if cached_apk {
                        button {
                            class: "primary",
                            onclick: move |_| {
                                result.set(Some(match platform::install_downloaded_apk() {
                                    Ok(message) | Err(message) => message,
                                }));
                            },
                            "Instalar APK descargado"
                        }
                    }
                    if let Some(message) = status {
                        p { class: "update-status", "{message}" }
                    }
                    button {
                        class: "secondary",
                        disabled: progress().is_some(),
                        onclick: move |_| {
                            let mut target = state;
                            let prepared = {
                                let mut state_guard = target.write();
                                state_guard.prepare_update()
                            };
                            let source = match prepared {
                                Ok(source) => source,
                                Err(message) => {
                                    target.write().update_status = Some(message);
                                    return;
                                }
                            };
                            let bar = std::sync::Arc::new(std::sync::Mutex::new(progress));
                            let message_signal = std::sync::Arc::new(std::sync::Mutex::new(result));
                            if let Ok(mut signal) = message_signal.lock() {
                                signal.set(None);
                            }
                            if let Ok(mut signal) = bar.lock() {
                                signal.set(Some(UpdateProgress::Checking));
                            }
                            std::thread::spawn(move || {
                                let reporter = bar.clone();
                                let callback = move |step| {
                                    if let Ok(mut signal) = reporter.lock() {
                                        signal.set(Some(step));
                                    }
                                };
                                let message = match platform::download_update(&source, &callback) {
                                    Ok(message) | Err(message) => message,
                                };
                                if let Ok(mut signal) = message_signal.lock() {
                                    signal.set(Some(message));
                                }
                                if let Ok(mut signal) = bar.lock() {
                                    signal.set(None);
                                }
                            });
                        },
                        "Buscar y descargar"
                    }
                }
            }
        }
    }
}
