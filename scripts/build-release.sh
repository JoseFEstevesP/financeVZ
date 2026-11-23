#!/usr/bin/env bash
# Build Android firmado para distribución fuera de Play Store.
# Requiere FINANCEVZ_KEYSTORE_PATH, FINANCEVZ_KEYSTORE_PASSWORD,
# FINANCEVZ_KEY_PASSWORD y FINANCEVZ_KEY_ALIAS (ver setup-keystore.sh).
set -euo pipefail

ROOT="$(env -u CDPATH cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export NDK_HOME="$ANDROID_HOME/ndk/27.2.12479018"
export ANDROID_NDK_HOME="$NDK_HOME"
export JAVA_HOME="$HOME/.local/share/financeVZ-toolchain/jdk"

for name in FINANCEVZ_KEYSTORE_PATH FINANCEVZ_KEYSTORE_PASSWORD FINANCEVZ_KEY_PASSWORD; do
    if [[ -z "${!name:-}" ]]; then
        echo "Falta la variable de entorno $name. Consulta scripts/setup-keystore.sh" >&2
        exit 1
    fi
done
KEY_ALIAS="${FINANCEVZ_KEY_ALIAS:-financevz}"
[[ -f "$FINANCEVZ_KEYSTORE_PATH" ]] || {
    echo "No existe el keystore: $FINANCEVZ_KEYSTORE_PATH" >&2
    exit 1
}

TARGET="${1:-aarch64-linux-android}"
cd "$ROOT"

GEN="$ROOT/target/dx/finance-app/debug/android/app"
ANDROID_RES="$ROOT/crates/finance-app/android/res/xml"
ANDROID_KOTLIN="$ROOT/crates/finance-app/android/kotlin/dev/dioxus/main/MainActivity.kt"
VERSION_FILE="$ROOT/crates/finance-app/android/version-code.txt"
UPDATES="$ROOT/updates"

LAST_CODE="$(tr -d '[:space:]' < "$VERSION_FILE" 2>/dev/null || true)"
[[ "$LAST_CODE" =~ ^[0-9]+$ ]] || LAST_CODE=1
CODE=$((LAST_CODE + 1))
VERSION_NAME="0.1.$CODE"

mkdir -p "$GEN/app/src/main/res/xml"
cp "$ANDROID_RES/network_security_config.xml" "$GEN/app/src/main/res/xml/"
cp "$ANDROID_RES/file_paths.xml" "$GEN/app/src/main/res/xml/"

dx build --android -p finance-app --target "$TARGET"

GRADLE="$GEN/app/build.gradle.kts"
APP_GRADLE="$GRADLE"
sed -i -E "s/versionCode = [0-9]+/versionCode = $CODE/; s/versionName = \"[^\"]*\"/versionName = \"$VERSION_NAME\"/" "$APP_GRADLE"
bash scripts/patch-android-gradle.sh "$APP_GRADLE"

cp "$ANDROID_RES/network_security_config.xml" "$GEN/app/src/main/res/xml/"
cp "$ANDROID_RES/file_paths.xml" "$GEN/app/src/main/res/xml/"
cp "$ANDROID_KOTLIN" "$GEN/app/src/main/kotlin/dev/dioxus/main/MainActivity.kt"
bash scripts/install-brand-icon.sh "$GEN"

# El Gradle se regenera en cada dx build: inyectamos la firma leyendo secretos
# del entorno en tiempo de build, nunca escribiéndolos en el archivo.
FINANCEVZ_GRADLE="$APP_GRADLE" python3 - <<'PY'
import os

path = os.environ["FINANCEVZ_GRADLE"]
with open(path, encoding="utf-8") as handle:
    text = handle.read()

keystore_path = os.environ["FINANCEVZ_KEYSTORE_PATH"].replace("\\", "\\\\").replace('"', '\\"')
block = '''    signingConfigs {
        create("release") {
            storeFile = file("%s")
            storePassword = System.getenv("FINANCEVZ_KEYSTORE_PASSWORD")
            keyAlias = System.getenv("FINANCEVZ_KEY_ALIAS") ?: "financevz"
            keyPassword = System.getenv("FINANCEVZ_KEY_PASSWORD")
        }
    }
''' % keystore_path

assert "signingConfigs" not in text, "El Gradle ya contiene una configuración de firma"
text = text.replace("android {\n", "android {\n" + block, 1)
text = text.replace(
    'getByName("release") {\n            isMinifyEnabled = true',
    'getByName("release") {\n            signingConfig = signingConfigs.getByName("release")\n            isMinifyEnabled = false',
    1,
)
assert "signingConfig = signingConfigs.getByName(\"release\")" in text, "No se pudo configurar la firma release"
with open(path, "w", encoding="utf-8") as handle:
    handle.write(text)
PY

(cd "$GEN" && ./gradlew assembleRelease)

APK="$GEN/app/build/outputs/apk/release/app-release.apk"
[[ -f "$APK" ]] || { echo "No se generó el APK firmado: $APK" >&2; exit 1; }
mkdir -p "$UPDATES"
cp "$APK" "$UPDATES/financevz.apk"
SHA256="$(sha256sum "$UPDATES/financevz.apk" | awk '{print $1}')"
printf '{"version_code": %d, "version_name": "%s", "apk": "financevz.apk", "sha256": "%s"}\n' \
    "$CODE" "$VERSION_NAME" "$SHA256" > "$UPDATES/update.json"
printf '%s\n' "$CODE" > "$VERSION_FILE"

echo "APK firmado: $APK"
echo "Copia de seguridad: $UPDATES/financevz.apk + $UPDATES/update.json"
echo "Versión: $VERSION_NAME (código $CODE)"
echo "Recuerda: desinstala la versión debug antes de instalar esta firma."
