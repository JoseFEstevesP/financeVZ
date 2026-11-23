#!/usr/bin/env bash
# Crea el keystore de release FUERA del repositorio. Las contraseñas se escriben
# interactivamente en keytool: no se pasan por argumentos ni se guardan aquí.
set -euo pipefail

DIR="${FINANCEVZ_KEYSTORE_DIR:-$HOME/.config/financevz}"
ALIAS="${FINANCEVZ_KEY_ALIAS:-financevz}"
KEYSTORE="$DIR/financevz-release.jks"

mkdir -p "$DIR"
chmod 700 "$DIR"

if [[ -e "$KEYSTORE" ]]; then
    echo "Ya existe un keystore en: $KEYSTORE"
    echo "No se sobrescribe nada. Usa ese archivo para firmar."
else
    echo "Se creará un keystore de release en: $KEYSTORE"
    echo "keytool pedirá la contraseña del almacén y la de la clave. Guárdalas en un gestor de contraseñas."
    keytool -genkeypair -v \
        -keystore "$KEYSTORE" \
        -alias "$ALIAS" \
        -keyalg RSA \
        -keysize 2048 \
        -validity 10000
    chmod 600 "$KEYSTORE"
fi

echo
echo "Antes de firmar, exporta en tu shell (sin mostrar los valores):"
echo "  export FINANCEVZ_KEYSTORE_PATH='$KEYSTORE'"
echo "  read -rsp 'Contraseña del keystore: ' FINANCEVZ_KEYSTORE_PASSWORD; export FINANCEVZ_KEYSTORE_PASSWORD; echo"
echo "  read -rsp 'Contraseña de la clave: ' FINANCEVZ_KEY_PASSWORD; export FINANCEVZ_KEY_PASSWORD; echo"
echo "  export FINANCEVZ_KEY_ALIAS='$ALIAS'"
echo
echo "Después ejecuta: scripts/build-release.sh"
echo "Aviso: pasar de la firma debug a la de release obliga a desinstalar la versión instalada."
