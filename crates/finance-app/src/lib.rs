pub mod bridge;
pub mod components;
pub mod platform;
pub mod state;
pub mod views;

use std::sync::OnceLock;

use dioxus::prelude::*;
use finance_store::SqliteRepository;

use components::icon::Icon;
use state::{AppState, View};
use views::{
    activity::Activity, add::Add, add_income::AddIncome, categories::Categories, home::Home,
    income::Incomes, payments::Payments, reports::Reports, search::Search, settings::Settings,
    update::Update,
};

static APP_STATE: OnceLock<AppState> = OnceLock::new();

pub fn run() {
    platform::init_logging();
    let path = platform::database_path().expect("ruta de base de datos");
    log::info!("base de datos: {path:?}");
    let repo = SqliteRepository::open(&path).expect("abrir base de datos");
    platform::cleanup_update_files();
    let mut state = AppState::new(repo);
    state::auto_backup(&state.repo.lock().expect("bloquear base de datos"), false);
    let backup_repo = state.repo.clone();
    state::start_auto_backup_loop(backup_repo);
    let rate_repo = state.repo.clone();
    state::start_rate_refresh_loop(rate_repo);
    let update_repo = state.repo.clone();
    state::start_update_check_loop(update_repo.clone());
    state::check_update_on_start(update_repo);
    state.update_rate_from_api();
    let _ = APP_STATE.set(state);
    dioxus::launch(App);
}

#[derive(Props, Clone, PartialEq)]
struct TabProps {
    target: View,
    label: &'static str,
    icon: &'static str,
    active: bool,
    app_state: Signal<AppState>,
}

#[component]
fn Tab(props: TabProps) -> Element {
    let mut app_state = props.app_state;
    let target = props.target;
    let class = if props.active { "tab active" } else { "tab" };
    rsx! {
        button {
            class: class,
            onclick: move |_| app_state.write().set_view(target),
            Icon { name: props.icon, class: "tab-icon" }
            span { "{props.label}" }
        }
    }
}

#[component]
fn App() -> Element {
    let mut app_state = use_signal(|| APP_STATE.get().expect("estado inicializado").clone());
    bridge::use_bridge(app_state);
    let snapshot = app_state.read();
    let view = snapshot.view;
    let toast = snapshot.toast.clone();
    let locked = snapshot.locked;
    let pin_input = snapshot.pin_input.clone();
    let pin_error = snapshot.pin_error.clone();
    let biometric_unlock = snapshot.biometric_enabled;
    let evidence = snapshot.evidence_viewer.clone();
    let pending_delete_message = snapshot.pending_delete_message();
    let update_available = snapshot.update_available.clone();
    let shell_class = if snapshot.is_light_theme() {
        "shell theme-light"
    } else {
        "shell"
    };
    let root_font_size = match snapshot.text_scale.as_str() {
        "small" => 14,
        "large" => 18,
        _ => 16,
    };
    let styles = format!("{STYLES}\nhtml {{ font-size: {root_font_size}px; }}");
    drop(snapshot);

    use_effect(move || {
        let light = app_state.read().is_light_theme();
        bridge::request_light_theme(light);
    });

    use_effect(move || {
        if app_state.read().toast.is_some() {
            bridge::request_toast_clear();
        }
    });

    let content = match view {
        View::Home => rsx! {
            Home { state: app_state, go_add: move |_| app_state.write().set_view(View::Add) }
        },
        View::Add => rsx! { Add { state: app_state } },
        View::Income => rsx! { Incomes { state: app_state } },
        View::AddIncome => rsx! { AddIncome { state: app_state } },
        View::Reports => rsx! { Reports { state: app_state } },
        View::Categories => rsx! { Categories { state: app_state } },
        View::Payments => rsx! { Payments { state: app_state } },
        View::Update => rsx! { Update { state: app_state } },
        View::Search => rsx! { Search { state: app_state } },
        View::Activity => rsx! { Activity { state: app_state } },
        View::Settings => rsx! { Settings { state: app_state } },
    };

    rsx! {
        style { "{styles}" }
        div { class: shell_class,
            header { class: "topbar",
                button {
                    class: "brand",
                    aria_label: "Ir a Inicio",
                    onclick: move |_| app_state.write().set_view(View::Home),
                    svg { class: "brand-mark", view_box: "0 0 256 256",
                        path { d: BRAND_SYMBOL_PATH, fill: "currentColor" }
                    }
                    div { class: "brand-copy",
                        h1 { "finanzasVZ" }
                        small { class: "muted brand-version", "v{platform::local_version().1}" }
                    }
                }
                button {
                    class: "settings-button",
                    aria_label: "Abrir configuración",
                    onclick: move |_| app_state.write().set_view(View::Settings),
                    Icon { name: "settings", class: "settings-icon" }
                }
            }
            main { class: "content", {content} }
            if let Some(version) = update_available {
                if view != View::Settings {
                    button {
                        class: "update-banner",
                        onclick: move |_| app_state.write().set_view(View::Settings),
                        "Nueva versión disponible: v{version}. Toca para actualizar."
                    }
                }
            }
            if let Some(message) = toast {
                div { class: "toast", role: "status", "{message}" }
            }
            if let Some(message) = pending_delete_message {
                div { class: "overlay", role: "dialog", aria_label: "Confirmar eliminación",
                    div { class: "modal confirm-modal",
                        h3 { "Eliminar registro" }
                        p { class: "muted", "{message}" }
                        div { class: "row-buttons",
                            button {
                                class: "chip grow",
                                onclick: move |_| app_state.write().cancel_remove(),
                                "Cancelar"
                            }
                            button {
                                class: "chip danger grow",
                                onclick: move |_| app_state.write().confirm_remove(),
                                "Eliminar"
                            }
                        }
                    }
                }
            }
            if let Some(data_url) = evidence {
                div { class: "overlay", role: "dialog", aria_label: "Evidencia del gasto",
                    div { class: "modal evidence-modal",
                        h3 { "Evidencia" }
                        img { class: "evidence-image", src: data_url, alt: "Imagen de evidencia adjunta al gasto" }
                        button {
                            class: "secondary wide",
                            onclick: move |_| app_state.write().close_evidence(),
                            "Cerrar"
                        }
                    }
                }
            }
            if locked {
                div { class: "overlay", role: "dialog", aria_label: "App bloqueada",
                    div { class: "modal lock-modal",
                        svg { class: "brand-mark", view_box: "0 0 256 256",
                            path { d: BRAND_SYMBOL_PATH, fill: "currentColor" }
                        }
                        h3 { "finanzasVZ" }
                        p { class: "muted", "Introduce tu PIN de 4 dígitos para continuar." }
                        input {
                            class: "text-input pin-input",
                            value: pin_input,
                            inputmode: "numeric",
                            r#type: "password",
                            maxlength: "4",
                            autofocus: true,
                            placeholder: "••••",
                            oninput: move |event| {
                                let digits: String = event.value().chars().filter(|c| c.is_ascii_digit()).take(4).collect();
                                app_state.write().pin_input = digits;
                            },
                        }
                        if let Some(error) = pin_error {
                            p { class: "feedback", "{error}" }
                        }
                        button { class: "primary wide", onclick: move |_| app_state.write().unlock(),
                            "Desbloquear"
                        }
                        if biometric_unlock {
                            button {
                                class: "secondary wide spaced",
                                onclick: move |_| app_state.write().request_biometric("unlock"),
                                "Entrar con huella"
                            }
                        }
                    }
                }
            }
            nav { class: "bottom-nav",
                Tab { target: View::Home, label: "Inicio", icon: "home", active: view == View::Home, app_state }
                Tab { target: View::Reports, label: "Reportes", icon: "reports", active: view == View::Reports, app_state }
                Tab { target: View::Categories, label: "Categorías", icon: "categories", active: view == View::Categories, app_state }
                Tab { target: View::Payments, label: "Pagos", icon: "payments", active: view == View::Payments, app_state }
            }
        }
    }
}

const BRAND_SYMBOL_PATH: &str = "M 58 136 H 90 A 14 14 0 0 1 104 150 V 202 A 14 14 0 0 1 90 216 H 58 A 14 14 0 0 1 44 202 V 150 A 14 14 0 0 1 58 136 Z M 118 96 H 150 A 14 14 0 0 1 164 110 V 202 A 14 14 0 0 1 150 216 H 118 A 14 14 0 0 1 104 202 V 110 A 14 14 0 0 1 118 96 Z M 178 76 H 210 A 14 14 0 0 1 224 90 V 202 A 14 14 0 0 1 210 216 H 178 A 14 14 0 0 1 164 202 V 90 A 14 14 0 0 1 178 76 Z M 192 18 L 226 74 L 158 74 Z";

const STYLES: &str = r#"
:root {
  --bg: #0f1420; --text: #eef2fb; --muted: #93a0b8; --card: #161d2b; --border: #232d40;
  --input-bg: #0e1421; --input-border: #2b3752; --primary: #4f8cff; --primary-text: #fff;
  --accent: #2dd4a7; --link: #8fb4ff; --link-border: #2c4a86; --chip-bg: #0e1421;
  --chip-text: #c7d2e8; --chip-active-bg: #17294b; --nav-bg: #121826; --nav-border: #222b3d;
  --hero-start: #1b2a4a; --hero-end: #131a2b; --hero-border: #26385e;
  --secondary-bg: #17294b; --secondary-text: #dce8ff; --row-border: #212b3d;
  --toast-bg: #173b34; --toast-text: #d8fff2; --toast-border: #2dd4a7;
  --alert-bg: #3a2415; --alert-border: #8a5a24; --urgent: #ffb38a;
  --positive: #2dd4a7; --negative: #ff8a8a; --overlay: rgba(6, 10, 18, .92);
  --modal-bg: #161d2b; --bar-bg: #0e1421; --bar-start: #4f8cff; --bar-end: #2f5fb8;
  --evidence-bg: #0e1421; --file-border: #39517d; --file-active: #14213a;
  --shadow: rgba(0, 0, 0, .45);
}
.theme-light {
  --bg: #f7f9fd; --text: #16203a; --muted: #5c6b85; --card: #ffffff; --border: #dbe3f0;
  --input-bg: #f1f5fb; --input-border: #c4d0e4; --primary: #316cdb; --primary-text: #fff;
  --accent: #0f9d78; --link: #2358b8; --link-border: #a8c2ee; --chip-bg: #f1f5fb;
  --chip-text: #2a3852; --chip-active-bg: #dce8ff; --nav-bg: #ffffff; --nav-border: #dbe3f0;
  --hero-start: #dbe8ff; --hero-end: #eef4ff; --hero-border: #b9d0f5;
  --secondary-bg: #dce8ff; --secondary-text: #234a91; --row-border: #e6ebf3;
  --toast-bg: #d8fff2; --toast-text: #0c5c46; --toast-border: #0f9d78;
  --alert-bg: #fff2e5; --alert-border: #e0a35a; --urgent: #a85a13;
  --positive: #0f9d78; --negative: #c53d3d; --overlay: rgba(20, 30, 50, .55);
  --modal-bg: #ffffff; --bar-bg: #f1f5fb; --bar-start: #316cdb; --bar-end: #1e4fa3;
  --evidence-bg: #f1f5fb; --file-border: #9ab4e0; --file-active: #dce8ff;
  --shadow: rgba(30, 45, 75, .18);
}
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; height: 100%; overflow: hidden; background: var(--bg); color: var(--text);
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif; }
.shell { height: 100dvh; min-height: 100dvh; display: flex; flex-direction: column;
  background: var(--bg); color: var(--text); }
.topbar { display: flex; align-items: center; justify-content: space-between;
  padding: calc(1.125rem + env(safe-area-inset-top)) 1.25rem .75rem; border-bottom: 1px solid var(--nav-border); }
.brand { display: flex; align-items: center; gap: .625rem; background: none; border: 0;
  color: inherit; padding: 0; cursor: pointer; text-align: left; }
.brand:hover { opacity: .85; }
.brand-mark { width: 1.75rem; height: 1.75rem; color: var(--accent); flex-shrink: 0; }
.topbar h1 { font-size: 1.25rem; margin: 0; letter-spacing: .03125rem; }
.brand-copy { display: flex; flex-direction: column; line-height: 1.1; }
.brand-version { font-size: .72rem; font-weight: 600; opacity: .85; }
.settings-button { display: grid; place-items: center; width: 2.25rem; height: 2.25rem;
  padding: 0; background: var(--chip-bg); border: 1px solid var(--input-border);
  border-radius: .625rem; cursor: pointer; }
.settings-button:active { border-color: var(--primary); background: var(--chip-active-bg); }
.settings-icon { width: 1.375rem; height: 1.375rem; }
.content { flex: 1; min-height: 0; overflow-y: auto; padding: 1rem; }
.page { display: flex; flex-direction: column; gap: 1rem; }
.hero { padding: 1.375rem; border-radius: 1.125rem;
  background: linear-gradient(135deg, var(--hero-start), var(--hero-end)); border: 1px solid var(--hero-border); }
.hero.compact { padding: 1.125rem 1.375rem; }
.hero .amount { font-size: 2.125rem; margin: .375rem 0 1rem; font-weight: 700; }
.muted { color: var(--muted); font-size: .8125rem; }
.date-hint { color: var(--muted); font-size: .75rem; margin: .375rem 0 0; }
.card { background: var(--card); border: 1px solid var(--border); border-radius: 1rem; padding: 1.125rem; }
.card h3 { margin: 0 0 .875rem; font-size: 1rem; }
.primary { background: var(--primary); color: var(--primary-text); border: 0; border-radius: .75rem;
  padding: .75rem 1.125rem; font-size: .9375rem; font-weight: 600; cursor: pointer; }
.primary.wide { width: 100%; margin-top: 1rem; }
.link { background: none; border: 0; color: var(--link); padding: .875rem 0 0; cursor: pointer; }
.spaced { margin-top: .875rem; }
.field-label { display: block; color: var(--muted); font-size: .8125rem; margin: 1rem 0 .4375rem; }
.amount-input { width: 100%; font-size: 1.875rem; font-weight: 700; color: var(--text);
  background: var(--input-bg); border: 1px solid var(--input-border);
  border-radius: .75rem; padding: .75rem .875rem; }
.text-input { width: 100%; color: var(--text); background: var(--input-bg);
  border: 1px solid var(--input-border); border-radius: .75rem; padding: .75rem .875rem; font-size: .9375rem; }
.category-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: .5rem; }
.chip { background: var(--chip-bg); color: var(--chip-text); border: 1px solid var(--input-border);
  border-radius: .75rem; padding: .6875rem; font-size: .875rem; cursor: pointer; text-align: left;
  display: flex; align-items: center; gap: .5rem; }
.chip.active { border-color: var(--primary); background: var(--chip-active-bg); color: var(--text); }
.chip.danger { border-color: var(--negative); color: var(--negative); font-weight: 600; }
.donut-wrap { width: 11.875rem; height: 11.875rem; margin: 0 auto .75rem; }
.legend { display: flex; flex-direction: column; gap: .5rem; }
.legend-row { display: flex; align-items: center; gap: .5625rem; font-size: .875rem; }
.dot { width: .6875rem; height: .6875rem; border-radius: 50%; display: inline-block; flex: none; }
.legend-name { flex: 1; }
.legend-value { color: var(--chip-text); font-variant-numeric: tabular-nums; }
.expense-row { display: flex; align-items: center; gap: .75rem; padding: .75rem 0;
  border-bottom: 1px solid var(--row-border); }
.expense-icon { width: 2.25rem; height: 2.25rem; display: grid; place-items: center;
  background: var(--input-bg); border-radius: .625rem; font-size: 1.125rem; flex: none; }
.expense-info { flex: 1; display: flex; flex-direction: column; gap: .125rem; min-width: 0; }
.expense-amount { font-variant-numeric: tabular-nums; font-weight: 600; flex: none; }
.expense-actions { display: flex; flex-direction: column; gap: .125rem; flex: none; }
.table-scroll { width: 100%; overflow-x: auto; border: 1px solid var(--row-border);
  border-radius: .75rem; }
.data-table { width: 100%; min-width: 46rem; border-collapse: collapse; font-size: .8125rem; }
.data-table th, .data-table td { padding: .5625rem .625rem; text-align: left;
  border-bottom: 1px solid var(--row-border); white-space: nowrap; }
.data-table th { color: var(--muted); font-weight: 600; background: var(--input-bg); }
.data-table tr:last-child td { border-bottom: 0; }
.data-table .num { text-align: right; font-variant-numeric: tabular-nums; font-weight: 600; }
.data-table .actions { display: flex; gap: .25rem; }
.data-table .table-note { color: var(--muted); font-size: .75rem; }
.category-row { display: flex; align-items: center; gap: .625rem; padding: .5625rem 0;
  border-bottom: 1px solid var(--row-border); }
.grow { flex: 1; }
.bars { display: flex; align-items: flex-end; gap: .5rem; height: 10rem; }
.bar-column { flex: 1; display: flex; flex-direction: column; align-items: center; gap: .375rem; }
.bar { width: 100%; background: linear-gradient(var(--bar-start), var(--bar-end));
  border-radius: .375rem .375rem 0 0; }
.bar-label { font-size: .6875rem; color: var(--muted); }
.feedback { color: var(--link); font-size: .8125rem; margin: .875rem 0 0; }
.toast { position: fixed; left: 50%; bottom: calc(5.25rem + env(safe-area-inset-bottom));
  transform: translateX(-50%); z-index: 20; background: var(--toast-bg); color: var(--toast-text);
  border: 1px solid var(--toast-border); border-radius: 999px; padding: .6875rem 1.25rem; font-size: .875rem;
  font-weight: 600; box-shadow: 0 .625rem 1.875rem var(--shadow); white-space: nowrap; }
.update-banner { position: fixed; left: 1rem; right: 1rem; bottom: calc(5.25rem + env(safe-area-inset-bottom));
  z-index: 19; background: var(--chip-active-bg); color: var(--link); border: 1px solid var(--link-border);
  border-radius: .875rem; padding: .75rem 1rem; font-size: .8125rem; font-weight: 600; text-align: center;
  box-shadow: 0 .625rem 1.875rem var(--shadow); cursor: pointer; }
.bottom-nav { display: flex; border-top: 1px solid var(--nav-border); background: var(--nav-bg);
  padding-bottom: env(safe-area-inset-bottom); }
.file-drop { position: relative; display: flex; align-items: center; gap: .75rem;
  background: var(--input-bg); border: 1px dashed var(--file-border); border-radius: .875rem;
  padding: .8125rem .875rem; cursor: pointer; overflow: hidden; }
.file-drop:active { border-color: var(--primary); background: var(--file-active); }
.file-drop-icon { width: 2.5rem; height: 2.5rem; flex: none; display: grid; place-items: center;
  background: var(--chip-active-bg); border-radius: .75rem; font-size: 1.125rem; }
.file-drop-copy { display: flex; flex-direction: column; gap: .1875rem; }
.file-drop-copy strong { color: var(--text); font-size: .875rem; }
.file-drop-copy small { font-size: .75rem; }
.file-input-native { position: absolute; inset: 0; width: 100%; height: 100%;
  margin: 0; opacity: 0; cursor: pointer; }
.evidence-row { display: flex; align-items: center; gap: .625rem; margin-top: .5rem; font-size: .8125rem;
  background: var(--chip-active-bg); border: 1px solid var(--link-border); border-radius: .75rem; padding: .625rem .75rem; }
.update-advanced { margin-top: .5rem; }
.update-advanced summary { cursor: pointer; color: var(--link); font-size: .8125rem; font-weight: 600; }
.update-advanced .update-input { margin-top: .25rem; }
.update-input { width: 100%; color: var(--text); background: var(--input-bg); border: 1px solid var(--input-border);
  border-radius: .75rem; padding: .75rem .875rem; font-size: .875rem; }
.secondary { background: var(--secondary-bg); color: var(--secondary-text); border: 1px solid var(--link-border);
  border-radius: .75rem; padding: .6875rem 1rem; font-size: .875rem; font-weight: 600; cursor: pointer; }
.update-actions { display: flex; flex-direction: column; gap: .75rem; margin-top: .875rem; }
.update-status { color: var(--link); font-size: .8125rem; margin: 0; }
.progress-track { width: 100%; height: .625rem; background: var(--bar-bg); border: 1px solid var(--input-border);
  border-radius: 999px; overflow: hidden; }
.progress-fill { height: 100%; background: linear-gradient(90deg, var(--bar-start), var(--bar-end));
  border-radius: 999px; transition: width .2s ease; }
.progress-fill.indeterminate { width: 35% !important; animation: progress-slide 1.2s ease-in-out infinite; }
@keyframes progress-slide { 0% { transform: translateX(-110%); } 100% { transform: translateX(310%); } }
.tab { flex: 1; background: none; border: 0; color: var(--muted); padding: .6875rem .0625rem .8125rem;
  display: flex; flex-direction: column; align-items: center; gap: .1875rem; font-size: .625rem;
  cursor: pointer; }
.tab.active { color: var(--link); }
.tab-icon { width: 1.5rem; height: 1.5rem; }
.tab .tab-icon { opacity: .55; }
.tab.active .tab-icon { opacity: 1; }
.icon { width: 1.25rem; height: 1.25rem; display: block; flex: none; }
.row-icon { width: 1.875rem; height: 1.875rem; }
.chip-icon { width: 1.25rem; height: 1.25rem; }
.inline-icon { width: .8125rem; height: .8125rem; display: inline-block; vertical-align: -.125rem; margin-left: .25rem; }
.icon-button { background: none; border: 0; padding: .25rem; cursor: pointer;
  display: grid; place-items: center; color: var(--muted); }
.icon-button:hover { color: var(--text); }
.icon-button .icon { width: 1rem; height: 1rem; }
.alert { background: var(--alert-bg); border: 1px solid var(--alert-border); border-radius: 1rem; padding: .875rem 1rem; }
.alert-row { display: flex; justify-content: space-between; gap: .625rem; font-size: .8125rem; margin-top: .5rem; }
.urgent { color: var(--urgent); }
.secondary-amount { color: var(--link); font-size: 1rem; margin: -.5rem 0 1rem; }
.payment-row { display: flex; align-items: center; gap: .625rem; padding: .6875rem 0; border-bottom: 1px solid var(--row-border); flex-wrap: wrap; }
.payment-row .grow { display: flex; flex-direction: column; gap: .125rem; }
.row-buttons { display: flex; gap: .5rem; margin-top: .625rem; }
.row-buttons .chip { text-align: center; }
.primary.small { padding: .5rem .75rem; font-size: .8125rem; }
.link.small-link { padding: .375rem .25rem; }
.category-grid.two { grid-template-columns: repeat(2, 1fr); }
.summary-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: .5rem; margin: .875rem 0 1rem; }
.summary-grid > div { display: flex; flex-direction: column; gap: .25rem; background: var(--input-bg);
  border: 1px solid var(--input-border); border-radius: .75rem; padding: .625rem; }
.summary-grid strong { font-size: .875rem; font-variant-numeric: tabular-nums; }
.balance-positive { color: var(--positive); }
.balance-negative { color: var(--negative); }
.secondary.wide { width: 100%; margin-top: 1rem; }
.subheading { color: var(--chip-text); font-size: .8125rem; margin: 1rem 0 .375rem; }
.date-range { display: grid; grid-template-columns: 1fr 1fr; gap: .625rem; }
.backup-json { resize: vertical; font-family: ui-monospace, monospace; font-size: .75rem; }
.overlay { position: fixed; inset: 0; z-index: 50; display: grid; place-items: center;
  background: var(--overlay); padding: 1.25rem; }
.modal { width: min(26.25rem, 100%); background: var(--modal-bg); border: 1px solid var(--input-border);
  border-radius: 1.125rem; padding: 1.375rem; box-shadow: 0 1.5rem 3.75rem var(--shadow); }
.modal h3 { margin: .5rem 0; font-size: 1.125rem; }
.lock-modal { text-align: center; }
.lock-modal .brand-mark { width: 2.75rem; height: 2.75rem; margin: 0 auto; }
.pin-input { text-align: center; letter-spacing: .5rem; font-size: 1.375rem; }
.evidence-modal .evidence-image { width: 100%; max-height: 65dvh; object-fit: contain;
  border-radius: .75rem; background: var(--evidence-bg); margin: .375rem 0 .25rem; }
"#;
