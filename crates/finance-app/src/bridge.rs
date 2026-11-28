use base64::Engine;
use dioxus::prelude::*;
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum BridgeMessage {
    #[serde(rename = "evidence")]
    Evidence { name: String, data: String },
    #[serde(rename = "toast_clear")]
    ToastClear,
    #[serde(rename = "backup_import")]
    BackupImport { data: String },
    #[serde(rename = "biometric_result")]
    BiometricResult {
        action: String,
        ok: bool,
        message: String,
    },
    #[cfg(feature = "self-update")]
    #[serde(rename = "download_result")]
    DownloadResult { status: String, message: String },
    #[cfg(feature = "self-update")]
    #[serde(rename = "notification_status")]
    NotificationStatus { granted: bool },
    #[serde(rename = "open_settings")]
    OpenSettings,
}

pub fn use_bridge(mut state: Signal<AppState>) {
    use_effect(move || {
        let mut channel = dioxus::document::eval(BRIDGE_JS);
        spawn(async move {
            while let Ok(message) = channel.recv::<BridgeMessage>().await {
                match message {
                    BridgeMessage::ToastClear => state.write().clear_toast(),
                    BridgeMessage::BackupImport { data } => state.write().import_backup_text(&data),
                    BridgeMessage::BiometricResult {
                        action,
                        ok,
                        message,
                    } => state.write().finish_biometric(&action, ok, &message),
                    #[cfg(feature = "self-update")]
                    BridgeMessage::DownloadResult { status, message } => {
                        state.write().finish_background_download(&status, &message);
                    }
                    #[cfg(feature = "self-update")]
                    BridgeMessage::NotificationStatus { granted } => {
                        state.write().notifications_enabled = granted;
                    }
                    BridgeMessage::OpenSettings => {
                        state.write().set_view(crate::state::View::Settings);
                    }
                    BridgeMessage::Evidence { name, data } => {
                        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) {
                            state.write().attach_evidence(name, &bytes);
                        }
                    }
                }
            }
        });
    });
}

pub fn request_toast_clear() {
    dioxus::document::eval("if (window.__fvzClearToast) window.__fvzClearToast();");
}

pub fn request_backup_download(json: &str) {
    let payload = serde_json::to_string(json).unwrap_or_else(|_| "\"\"".to_owned());
    let script = format!("if (window.__fvzDownloadBackup) window.__fvzDownloadBackup({payload});");
    dioxus::document::eval(&script);
}

pub fn request_biometric(action: &str) {
    let action = serde_json::to_string(action).unwrap_or_else(|_| "\"unlock\"".to_owned());
    let script = format!(
        "if (window.FinanzasVZBiometric) window.FinanzasVZBiometric.authenticate({action}); \
         else if (window.__fvzBridge) window.__fvzBridge.send({{kind:'biometric_result',action:{action},ok:false,message:'La huella no está disponible en este dispositivo.'}});"
    );
    dioxus::document::eval(&script);
}

pub fn request_light_theme(light: bool) {
    dioxus::document::eval(&format!(
        "if (window.FinanzasVZBiometric) window.FinanzasVZBiometric.setLightTheme({light});"
    ));
}

#[cfg(feature = "self-update")]
pub fn request_background_download(url: &str, sha256: &str, version_code: i64, version_name: &str) {
    let url = serde_json::to_string(url).unwrap_or_else(|_| "\"\"".to_owned());
    let sha = serde_json::to_string(sha256).unwrap_or_else(|_| "\"\"".to_owned());
    let name = serde_json::to_string(version_name).unwrap_or_else(|_| "\"\"".to_owned());
    let script = format!(
        "if (window.FinanzasVZBiometric && window.FinanzasVZBiometric.downloadUpdate) \
         window.FinanzasVZBiometric.downloadUpdate({url},{sha},{version_code},{name}); \
         else if (window.__fvzBridge) window.__fvzBridge.send({{kind:'download_result',status:'error',message:'Descarga en segundo plano no disponible en este dispositivo.'}});"
    );
    dioxus::document::eval(&script);
}

#[cfg(feature = "self-update")]
pub fn request_notifications() {
    dioxus::document::eval(
        "if (window.FinanzasVZBiometric && window.FinanzasVZBiometric.requestNotifications) \
         window.FinanzasVZBiometric.requestNotifications();",
    );
}

// The channel above stays open for the lifetime of the app; these are one-shot commands.
const BRIDGE_JS: &str = r#"
window.__fvzBridge = dioxus;

if (!window.__fvzBridgeReady) {
  window.__fvzBridgeReady = true;

  document.addEventListener('change', async function (event) {
    const input = event.target;
    if (!input || input.id !== 'evidence-file' || !input.files || !input.files[0]) return;
    const file = input.files[0];
    const bytes = new Uint8Array(await file.arrayBuffer());
    let binary = '';
    for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
    window.__fvzBridge.send({ kind: 'evidence', name: file.name, data: btoa(binary) });
  }, true);

  document.addEventListener('change', async function (event) {
    const input = event.target;
    if (!input || input.id !== 'backup-file' || !input.files || !input.files[0]) return;
    const text = await input.files[0].text();
    window.__fvzBridge.send({ kind: 'backup_import', data: text });
    input.value = '';
  }, true);

  // Dioxus 0.7 replaces the file dialog with a desktop-only IPC dialog that is a
  // no-op on Android. Stop its listener and let the WebView open the real picker.
  window.addEventListener('click', function (event) {
    const target = event.target;
    if (target && (target.id === 'evidence-file' || target.id === 'backup-file')) {
      event.stopImmediatePropagation();
    }
  }, true);

  window.__fvzDownloadBackup = function (text) {
    const blob = new Blob([text], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = 'financevz-backup.json';
    document.body.appendChild(link);
    link.click();
    link.remove();
    setTimeout(function () { URL.revokeObjectURL(url); }, 1000);
  };

  window.__fvzClearToast = function () {
    setTimeout(function () {
      window.__fvzBridge.send({ kind: 'toast_clear' });
    }, 2600);
  };
}

// Keep the eval pending: resolving it makes Dioxus close the query and the
// channel above would stop receiving messages.
await new Promise(function () {});
"#;
