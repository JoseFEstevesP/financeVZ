#!/usr/bin/env bash
# Build Android con icono de marca, versionCode creciente y APK listo para el servidor local.
# Uso: scripts/build-android.sh [target]
set -euo pipefail

ROOT="$(env -u CDPATH cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export NDK_HOME="$ANDROID_HOME/ndk/27.2.12479018"
export ANDROID_NDK_HOME="$NDK_HOME"
export JAVA_HOME="$HOME/.local/share/financeVZ-toolchain/jdk"

TARGET="${1:-aarch64-linux-android}"
cd "$ROOT"

GEN="$ROOT/target/dx/finance-app/debug/android/app"
ANDROID_RES="$ROOT/crates/finance-app/android/res/xml"
ANDROID_KOTLIN_DIR="$ROOT/crates/finance-app/android/kotlin/dev/dioxus/main"
VERSION_FILE="$ROOT/crates/finance-app/android/version-code.txt"
UPDATES="$ROOT/updates"

LAST_CODE="$(tr -d '[:space:]' < "$VERSION_FILE" 2>/dev/null || true)"
[[ "$LAST_CODE" =~ ^[0-9]+$ ]] || LAST_CODE=1
CODE=$((LAST_CODE + 1))
VERSION_NAME="0.1.$CODE"

mkdir -p "$GEN/app/src/main/res/xml"
cp "$ANDROID_RES/network_security_config.xml" "$GEN/app/src/main/res/xml/"
cp "$ANDROID_RES/file_paths.xml" "$GEN/app/src/main/res/xml/"
cp "$ROOT/crates/finance-app/android/AndroidManifest.xml" "$GEN/app/src/main/AndroidManifest.xml"

bash scripts/install-brand-icon.sh "$GEN"
dx build --android -p finance-app --target "$TARGET"

# dx regenera el Gradle en cada build: subimos versionCode para que Android acepte la actualización.
GRADLE="$GEN/app/build.gradle.kts"
sed -i -E "s/versionCode = [0-9]+/versionCode = $CODE/; s/versionName = \"[^\"]*\"/versionName = \"$VERSION_NAME\"/" "$GRADLE"
grep -q "versionCode = $CODE" "$GRADLE"
grep -q "versionName = \"$VERSION_NAME\"" "$GRADLE"
bash scripts/patch-android-gradle.sh "$GRADLE"

cp "$ANDROID_RES/network_security_config.xml" "$GEN/app/src/main/res/xml/"
cp "$ANDROID_RES/file_paths.xml" "$GEN/app/src/main/res/xml/"
cp "$ROOT/crates/finance-app/android/AndroidManifest.xml" "$GEN/app/src/main/AndroidManifest.xml"
mkdir -p "$GEN/app/src/main/kotlin/dev/dioxus/main"
cp "$ANDROID_KOTLIN_DIR"/*.kt "$GEN/app/src/main/kotlin/dev/dioxus/main/"
cp "$ROOT/crates/finance-app/android/AndroidManifest.xml" "$GEN/app/src/main/AndroidManifest.xml"
bash scripts/install-brand-icon.sh "$GEN"
(cd "$GEN" && ./gradlew assembleDebug)

APK="$GEN/app/build/outputs/apk/debug/app-debug.apk"
mkdir -p "$UPDATES"
cp "$APK" "$UPDATES/financevz.apk"
SHA256="$(sha256sum "$UPDATES/financevz.apk" | awk '{print $1}')"
printf '{"version_code": %d, "version_name": "%s", "apk": "financevz.apk", "sha256": "%s"}\n' \
  "$CODE" "$VERSION_NAME" "$SHA256" > "$UPDATES/update.json"
printf '%s\n' "$CODE" > "$VERSION_FILE"

IP="$(hostname -I 2>/dev/null | awk '{print $1}')"
echo "APK listo: $APK"
echo "Servidor: python3 scripts/serve-update.sh"
echo "URL para el botón Actualizar: http://${IP:-192.168.0.10}:8000"
