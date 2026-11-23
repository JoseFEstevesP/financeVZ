#!/usr/bin/env bash
# Punto de entrada del build: MODE=dev genera el APK debug; MODE=prod lo firma.
set -euo pipefail

ROOT="$(unset CDPATH; cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
ENV_FILE="$ROOT/.env"

if [[ ! -f "$ENV_FILE" ]]; then
    echo "Falta $ENV_FILE. Copia .env.example a .env y rellénalo." >&2
    exit 1
fi

set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

cd "$ROOT"

case "${MODE:-}" in
    dev)
        exec bash "$ROOT/scripts/build-android.sh" "$@"
        ;;
    prod)
        exec bash "$ROOT/scripts/build-release.sh" "$@"
        ;;
    *)
        echo "MODE debe ser 'dev' o 'prod' (valor: '${MODE:-}')." >&2
        exit 1
        ;;
esac
