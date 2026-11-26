# AGENTS.md — financeVZ

## Estructura real

- Workspace Rust (edition 2024) con tres crates: `finance-core` (dominio), `finance-store` (SQLite + migraciones) y `finance-app` (Dioxus 0.7 dentro de un WebView Android).
- Entrada host: `crates/finance-app/src/main.rs` → `finance_app::run()` en `src/lib.rs`. El estado se inicializa una vez en `OnceLock<AppState>` antes de `dioxus::launch`; vistas en `src/views/`, estilos globales en `STYLES` de `lib.rs`.
- Las migraciones son archivos numerados incluidos con `include_str!` en `finance-store/src/sqlite.rs`. Una nueva requiere crear `migrations/00NN_*.sql` **y** registrar `M::up(...)` en `MIGRATIONS`. Los tests usan `open_in_memory()` y aplican migraciones solos, sin servicios externos.
- `Dioxus.toml` apunta al manifest Android; paquete `com.gato99.financevz`.

## Comandos que no se adivinan

- Verificación de CI: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --no-default-features -- -D warnings`, `cargo test --workspace --no-default-features`. Añade `cargo check --workspace --no-default-features` si quieres un extra.
- Un test suelto / un crate: `cargo test -p finance-app --no-default-features verifies_apk_checksum_before_installing`, `cargo test -p finance-store --no-default-features <filtro>`.
- Usa siempre `--no-default-features` en host: `finance-app` tiene `default = ["desktop"]`, que arrastra GTK/WebKit; CI no configura target Android.
- Build: `bash scripts/build.sh [target]` lee `.env` (ignorado por git; plantilla en `.env.example`) y con `MODE=dev` llama a `build-android.sh` (debug) o con `MODE=prod` a `build-release.sh` (firma con `FINANCEVZ_KEYSTORE_PATH`, `FINANCEVZ_KEYSTORE_PASSWORD`, `FINANCEVZ_KEY_PASSWORD` y `FINANCEVZ_KEY_ALIAS`). Pasar de debug a release obliga a desinstalar la debug.
- Servir actualizaciones: `python3 scripts/serve-update.sh [puerto]`; en la app se escribe solo `http://IP:PUERTO`.

## Generado y versionado

- `dx build` regenera `target/dx/finance-app/debug/android/app` y su Gradle: no edites ese árbol. Las fuentes reales son `crates/finance-app/android/kotlin/dev/dioxus/main/MainActivity.kt`, `crates/finance-app/android/res/xml/*` y `brand/android/res`; los scripts las copian después del build junto con `scripts/patch-android-gradle.sh` (biometría) y `scripts/install-brand-icon.sh`.
- Los scripts de build incrementan `crates/finance-app/android/version-code.txt`, derivan `versionName = 0.1.<code>` y regeneran `updates/update.json` con `sha256`. `updates/` está en `.gitignore`: no edites `update.json` a mano. Los scripts **no** tocan `VERSION`, `CHANGELOG.md` ni `README.md`: sincronízalos a mano en cada release.
- En host, `FINANCEVZ_DATA_DIR` cambia el directorio de datos (por defecto el cwd); en Android es `getFilesDir()` vía JNI/`ndk-context`. Al arrancar se borran el APK cacheado ya instalado y los descartes `.part`.

## Reglas de dominio que se rompen fácil

- Dinero en centavos `i64`, base USD; Bs es solo presentación. Tasa automática desde DolarAPI (`promedio`) con respaldo BCV y corrección manual; se consulta al arrancar y se refresca cada 30 min mientras la app está abierta. Si no hay tasa del día, se usa la última anterior.
- Formatos: USD `en-US`, Bs `es-VE`; fechas visibles `dd/mm/aaaa`, ISO solo internamente. En el input POS los dígitos son centavos y se formatean al escribir, sin convertir dentro del campo.
- Pagos: `due_day` 1–31 o `due_date` concreto; Cashea = inicial + cuotas; una cuota se convierte en gasto solo al pulsar «Pagar».
- Navegación: menú inferior Reportes / Categorías / Pagos / Actualizar; Inicio se abre con el logo; una sola tuerca de ajustes en la cabecera, sin enlace duplicado ni FAB central. La sección Huella aparece siempre en Configuración, con el PIN como respaldo obligatorio.
- Actualizador: el campo URL es solo la raíz (sin `/update.json` ni nombre de APK); descarga y después «Instalar APK descargado» desde un clic real, sin auto-lanzamiento. `sha256` de 64 hexadecimales es obligatorio; sin manifiesto válido no se descarga nada.
- Seguridad: PIN de 4 dígitos guardado como SHA-256 (`state::sha256_hex`); la huella usa `BiometricPrompt` en Kotlin y la app solo persiste `biometric_enabled`.
- Iconos: sprite `crates/finance-app/assets/financeVZ-pack.svg`; los emojis en seeds y `components/icon.rs` son solo compatibilidad.

## Preferencias de este proyecto (sobrescriben la global)

- No usar agentmemory ni guardar memoria nueva para este proyecto.
- Los comandos `sudo` los ejecuta el usuario.
- No hacer commits salvo petición explícita.
