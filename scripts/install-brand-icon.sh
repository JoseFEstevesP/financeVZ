#!/usr/bin/env bash
# Copia el icono de launcher de la marca al proyecto Android generado por dx.
# dx sobrescribe solo ic_launcher*; estos archivos tienen otro nombre y sobreviven.
set -euo pipefail

ROOT="$(env -u CDPATH cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GEN="${1:-$ROOT/target/dx/finance-app/debug/android/app}"
DEST="$GEN/app/src/main/res"
SRC="$ROOT/brand/android/res"

mkdir -p "$DEST/mipmap" "$DEST/mipmap-anydpi-v26" "$DEST/drawable"
cp "$SRC/mipmap/ic_launcher_brand.xml" "$DEST/mipmap/"
cp "$SRC/mipmap-anydpi-v26/ic_launcher_brand.xml" "$DEST/mipmap-anydpi-v26/"
cp "$SRC/drawable/ic_launcher_brand_background.xml" "$DEST/drawable/"
cp "$SRC/drawable/ic_launcher_brand_foreground.xml" "$DEST/drawable/"

echo "Icono de marca instalado en $DEST"
