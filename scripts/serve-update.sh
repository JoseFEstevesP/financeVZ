#!/usr/bin/env bash
# Sirve updates/ en la LAN para que el botón "Actualizar" descargue el APK.
set -euo pipefail

ROOT="$(env -u CDPATH cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${1:-8000}"
IP="$(hostname -I 2>/dev/null | awk '{print $1}')"

echo "Actualizaciones en: $ROOT/updates"
echo "Abre en la app: http://${IP:-192.168.0.10}:$PORT"
exec python3 -m http.server "$PORT" --bind 0.0.0.0 --directory "$ROOT/updates"
