#!/usr/bin/env bash
# dx regenera el Gradle en cada build: inyectamos las dependencias Android extra.
set -euo pipefail

GRADLE="${1:?Uso: patch-android-gradle.sh ruta/app/build.gradle.kts}"

if ! grep -q 'androidx.biometric:biometric' "$GRADLE"; then
    sed -i '/^dependencies {/a\    implementation("androidx.biometric:biometric:1.1.0")' "$GRADLE"
fi
grep -q 'androidx.biometric:biometric' "$GRADLE"
