#!/usr/bin/env bash
# Build Android con el icono de marca aplicado. Uso: scripts/build-android.sh [target]
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

bash scripts/install-brand-icon.sh
dx build --android -p finance-app --target "$TARGET"
bash scripts/install-brand-icon.sh

APK="$ROOT/target/dx/finance-app/debug/android/app/app/build/outputs/apk/debug/app-debug.apk"
echo "APK listo: $APK"
