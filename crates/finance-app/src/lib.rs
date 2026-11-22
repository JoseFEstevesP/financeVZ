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
    add::Add, add_income::AddIncome, categories::Categories, home::Home, income::Incomes,
    payments::Payments, reports::Reports, update::Update,
};

static APP_STATE: OnceLock<AppState> = OnceLock::new();

pub fn run() {
    platform::init_logging();
    let path = platform::database_path().expect("ruta de base de datos");
    log::info!("base de datos: {path:?}");
    let repo = SqliteRepository::open(&path).expect("abrir base de datos");
    platform::cleanup_update_files();
    let mut state = AppState::new(repo);
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
    let view = app_state.read().view;
    let toast = app_state.read().toast.clone();

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
    };

    rsx! {
        style { "{STYLES}" }
        div { class: "shell",
            header { class: "topbar",
                button {
                    class: "brand",
                    aria_label: "Ir a Inicio",
                    onclick: move |_| app_state.write().set_view(View::Home),
                    svg { class: "brand-mark", view_box: "0 0 256 256",
                        path { d: BRAND_SYMBOL_PATH, fill: "currentColor" }
                    }
                    h1 { "finanzasVZ" }
                }
                span { class: "badge", "personal" }
            }
            main { class: "content", {content} }
            if let Some(message) = toast {
                div { class: "toast", role: "status", "{message}" }
            }
            nav { class: "bottom-nav",
                Tab { target: View::Reports, label: "Reportes", icon: "reports", active: view == View::Reports, app_state }
                Tab { target: View::Categories, label: "Categorías", icon: "categories", active: view == View::Categories, app_state }
                Tab { target: View::Payments, label: "Pagos", icon: "payments", active: view == View::Payments, app_state }
                Tab { target: View::Update, label: "Actualizar", icon: "update", active: view == View::Update, app_state }
            }
        }
    }
}

const BRAND_SYMBOL_PATH: &str = "M 58 136 H 90 A 14 14 0 0 1 104 150 V 202 A 14 14 0 0 1 90 216 H 58 A 14 14 0 0 1 44 202 V 150 A 14 14 0 0 1 58 136 Z M 118 96 H 150 A 14 14 0 0 1 164 110 V 202 A 14 14 0 0 1 150 216 H 118 A 14 14 0 0 1 104 202 V 110 A 14 14 0 0 1 118 96 Z M 178 76 H 210 A 14 14 0 0 1 224 90 V 202 A 14 14 0 0 1 210 216 H 178 A 14 14 0 0 1 164 202 V 90 A 14 14 0 0 1 178 76 Z M 192 18 L 226 74 L 158 74 Z";

const STYLES: &str = r#"
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; height: 100%; overflow: hidden; background: #0f1420; color: #eef2fb;
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif; }
.shell { height: 100dvh; min-height: 100dvh; display: flex; flex-direction: column; }
.topbar { display: flex; align-items: center; justify-content: space-between;
  padding: calc(18px + env(safe-area-inset-top)) 20px 12px; border-bottom: 1px solid #222b3d; }
.brand { display: flex; align-items: center; gap: 10px; background: none; border: 0;
  color: inherit; padding: 0; cursor: pointer; text-align: left; }
.brand:hover { opacity: .85; }
.brand-mark { width: 28px; height: 28px; color: #2dd4a7; flex-shrink: 0; }
.topbar h1 { font-size: 20px; margin: 0; letter-spacing: .5px; }
.badge { font-size: 11px; text-transform: uppercase; letter-spacing: 1px;
  color: #8fb4ff; border: 1px solid #2c4a86; border-radius: 999px; padding: 4px 9px; }
.content { flex: 1; min-height: 0; overflow-y: auto; padding: 16px; }
.page { display: flex; flex-direction: column; gap: 16px; }
.hero { padding: 22px; border-radius: 18px;
  background: linear-gradient(135deg, #1b2a4a, #131a2b); border: 1px solid #26385e; }
.hero.compact { padding: 18px 22px; }
.hero .amount { font-size: 34px; margin: 6px 0 16px; font-weight: 700; }
.muted { color: #93a0b8; font-size: 13px; }
.card { background: #161d2b; border: 1px solid #232d40; border-radius: 16px; padding: 18px; }
.card h3 { margin: 0 0 14px; font-size: 16px; }
.primary { background: #4f8cff; color: #fff; border: 0; border-radius: 12px;
  padding: 12px 18px; font-size: 15px; font-weight: 600; cursor: pointer; }
.primary.wide { width: 100%; margin-top: 16px; }
.link { background: none; border: 0; color: #8fb4ff; padding: 14px 0 0; cursor: pointer; }
.field-label { display: block; color: #93a0b8; font-size: 13px; margin: 16px 0 7px; }
.amount-input { width: 100%; font-size: 30px; font-weight: 700; color: #eef2fb;
  background: #0e1421; border: 1px solid #2b3752;
  border-radius: 12px; padding: 12px 14px; }
.text-input { width: 100%; color: #eef2fb; background: #0e1421;
  border: 1px solid #2b3752; border-radius: 12px; padding: 12px 14px; font-size: 15px; }
.category-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; }
.chip { background: #0e1421; color: #c7d2e8; border: 1px solid #2b3752;
  border-radius: 12px; padding: 11px; font-size: 14px; cursor: pointer; text-align: left;
  display: flex; align-items: center; gap: 8px; }
.chip.active { border-color: #4f8cff; background: #17294b; color: #fff; }
.donut-wrap { width: 190px; height: 190px; margin: 0 auto 12px; }
.legend { display: flex; flex-direction: column; gap: 8px; }
.legend-row { display: flex; align-items: center; gap: 9px; font-size: 14px; }
.dot { width: 11px; height: 11px; border-radius: 50%; display: inline-block; flex: none; }
.legend-name { flex: 1; }
.legend-value { color: #c7d2e8; font-variant-numeric: tabular-nums; }
.expense-row { display: flex; align-items: center; gap: 11px; padding: 10px 0;
  border-bottom: 1px solid #212b3d; }
.expense-icon { width: 36px; height: 36px; display: grid; place-items: center;
  background: #0e1421; border-radius: 10px; font-size: 18px; }
.expense-info { flex: 1; display: flex; flex-direction: column; }
.expense-amount { font-variant-numeric: tabular-nums; font-weight: 600; }
.category-row { display: flex; align-items: center; gap: 10px; padding: 9px 0;
  border-bottom: 1px solid #212b3d; }
.grow { flex: 1; }
.bars { display: flex; align-items: flex-end; gap: 8px; height: 160px; }
.bar-column { flex: 1; display: flex; flex-direction: column; align-items: center; gap: 6px; }
.bar { width: 100%; background: linear-gradient(#4f8cff, #2f5fb8);
  border-radius: 6px 6px 0 0; }
.bar-label { font-size: 11px; color: #93a0b8; }
.feedback { color: #8fb4ff; font-size: 13px; margin: 14px 0 0; }
.toast { position: fixed; left: 50%; bottom: calc(84px + env(safe-area-inset-bottom));
  transform: translateX(-50%); z-index: 20; background: #173b34; color: #d8fff2;
  border: 1px solid #2dd4a7; border-radius: 999px; padding: 11px 20px; font-size: 14px;
  font-weight: 600; box-shadow: 0 10px 30px rgba(0,0,0,.45); white-space: nowrap; }
.bottom-nav { display: flex; border-top: 1px solid #222b3d; background: #121826;
  padding-bottom: env(safe-area-inset-bottom); }
.file-drop { position: relative; display: flex; align-items: center; gap: 12px;
  background: #0e1421; border: 1px dashed #39517d; border-radius: 14px;
  padding: 13px 14px; cursor: pointer; overflow: hidden; }
.file-drop:active { border-color: #4f8cff; background: #14213a; }
.file-drop-icon { width: 40px; height: 40px; flex: none; display: grid; place-items: center;
  background: #17294b; border-radius: 12px; font-size: 18px; }
.file-drop-copy { display: flex; flex-direction: column; gap: 3px; }
.file-drop-copy strong { color: #eef2fb; font-size: 14px; }
.file-drop-copy small { font-size: 12px; }
.file-input-native { position: absolute; inset: 0; width: 100%; height: 100%;
  margin: 0; opacity: 0; cursor: pointer; }
.evidence-row { display: flex; align-items: center; gap: 10px; margin-top: 8px; font-size: 13px;
  background: #14213a; border: 1px solid #2c4a86; border-radius: 12px; padding: 10px 12px; }
.update-input { width: 100%; color: #eef2fb; background: #0e1421; border: 1px solid #2b3752;
  border-radius: 12px; padding: 12px 14px; font-size: 14px; }
.secondary { background: #17294b; color: #dce8ff; border: 1px solid #2c4a86;
  border-radius: 12px; padding: 11px 16px; font-size: 14px; font-weight: 600; cursor: pointer; }
.update-actions { display: flex; flex-direction: column; gap: 12px; margin-top: 14px; }
.update-status { color: #8fb4ff; font-size: 13px; margin: 0; }
.progress-track { width: 100%; height: 10px; background: #0e1421; border: 1px solid #2b3752;
  border-radius: 999px; overflow: hidden; }
.progress-fill { height: 100%; background: linear-gradient(90deg, #4f8cff, #2f5fb8);
  border-radius: 999px; transition: width .2s ease; }
.progress-fill.indeterminate { width: 35% !important; animation: progress-slide 1.2s ease-in-out infinite; }
@keyframes progress-slide { 0% { transform: translateX(-110%); } 100% { transform: translateX(310%); } }
.tab { flex: 1; background: none; border: 0; color: #93a0b8; padding: 11px 1px 13px;
  display: flex; flex-direction: column; align-items: center; gap: 3px; font-size: 10px;
  cursor: pointer; }
.tab.active { color: #8fb4ff; }
.tab-icon { width: 24px; height: 24px; }
.tab .tab-icon { opacity: .55; }
.tab.active .tab-icon { opacity: 1; }
.icon { width: 20px; height: 20px; display: block; flex: none; }
.row-icon { width: 30px; height: 30px; }
.chip-icon { width: 20px; height: 20px; }
.inline-icon { width: 13px; height: 13px; display: inline-block; vertical-align: -2px; margin-left: 4px; }
.icon-button { background: none; border: 0; padding: 4px; cursor: pointer;
  display: grid; place-items: center; color: #93a0b8; }
.icon-button:hover { color: #eef2fb; }
.icon-button .icon { width: 16px; height: 16px; }
.alert { background: #3a2415; border: 1px solid #8a5a24; border-radius: 16px; padding: 14px 16px; }
.alert-row { display: flex; justify-content: space-between; gap: 10px; font-size: 13px; margin-top: 8px; }
.urgent { color: #ffb38a; }
.secondary-amount { color: #8fb4ff; font-size: 16px; margin: -8px 0 16px; }
.payment-row { display: flex; align-items: center; gap: 10px; padding: 11px 0; border-bottom: 1px solid #212b3d; flex-wrap: wrap; }
.payment-row .grow { display: flex; flex-direction: column; gap: 2px; }
.row-buttons { display: flex; gap: 8px; margin-top: 10px; }
.row-buttons .chip { text-align: center; }
.primary.small { padding: 8px 12px; font-size: 13px; }
.link.small-link { padding: 6px 4px; }
.category-grid.two { grid-template-columns: repeat(2, 1fr); }
.summary-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; margin: 14px 0 16px; }
.summary-grid > div { display: flex; flex-direction: column; gap: 4px; background: #0e1421;
  border: 1px solid #2b3752; border-radius: 12px; padding: 10px; }
.summary-grid strong { font-size: 14px; font-variant-numeric: tabular-nums; }
.balance-positive { color: #2dd4a7; }
.balance-negative { color: #ff8a8a; }
"#;
