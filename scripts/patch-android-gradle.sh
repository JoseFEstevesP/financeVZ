#!/usr/bin/env bash
# dx regenera el Gradle en cada build: inyectamos las dependencias Android extra.
set -euo pipefail

GRADLE="${1:?Uso: patch-android-gradle.sh ruta/app/build.gradle.kts}"

FDROID="${2:-}"

if ! grep -q 'androidx.biometric:biometric' "$GRADLE"; then
    sed -i '/^dependencies {/a\    implementation("androidx.biometric:biometric:1.1.0")' "$GRADLE"
fi
if [ "$FDROID" != "fdroid" ]; then
    if ! grep -q 'androidx.work:work-runtime-ktx' "$GRADLE"; then
        sed -i '/^dependencies {/a\    implementation("androidx.work:work-runtime-ktx:2.9.1")' "$GRADLE"
    fi
fi

grep -q 'androidx.biometric:biometric' "$GRADLE"
if [ "$FDROID" != "fdroid" ]; then
    grep -q 'androidx.work:work-runtime-ktx' "$GRADLE"
fi
