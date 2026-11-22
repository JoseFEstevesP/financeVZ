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
}

pub fn use_bridge(mut state: Signal<AppState>) {
    use_effect(move || {
        let mut channel = dioxus::document::eval(BRIDGE_JS);
        spawn(async move {
            while let Ok(message) = channel.recv::<BridgeMessage>().await {
                match message {
                    BridgeMessage::ToastClear => state.write().clear_toast(),
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

  // Dioxus 0.7 replaces the file dialog with a desktop-only IPC dialog that is a
  // no-op on Android. Stop its listener and let the WebView open the real picker.
  window.addEventListener('click', function (event) {
    const target = event.target;
    if (target && target.id === 'evidence-file') {
      event.stopImmediatePropagation();
    }
  }, true);

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
