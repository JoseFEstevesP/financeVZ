# finanzasVZ — Logo Guidelines (compact)

> Guía breve para usar la marca de forma consistente. Archivos maestros en `brand/`.

## 1. The logo
- **Idea**: tres **barras ascendentes** con punta de flecha en la última: lectura inmediata de crecimiento y progreso financiero.
- **Versions**: primary (horizontal) · stacked · symbol-only · wordmark-only.
- **Files**: `brand/*.svg` para pantalla e impresión (SVG escala sin pérdida). Exportaciones rasterizadas en `brand/renders/` y `brand/variants/`.

## 2. Clear space
Deja una zona libre de **1 × la altura del trazo del símbolo** alrededor del logo en todos los lados. La zona escala con el logo: nunca uses una distancia fija.

## 3. Minimum size
| Version | Screen | Print |
|---|---|---|
| Horizontal | 120 px de ancho | 30 mm de ancho |
| Símbolo | 24 px (usa `financevz-symbol-small.svg`) | 6 mm |
| Icono de app | 48 px (usa `financevz-symbol-small.svg`) | — |

## 4. Colour
| Name | HEX | RGB |
|---|---|---|
| Mint (primario) | #2DD4A7 | 45, 212, 167 |
| Ink (fondo / texto) | #0F1420 | 15, 20, 32 |
| Cloud (texto claro) | #EEF2FB | 238, 242, 251 |

**Approved logo/background pairs**: mint sobre ink · ink sobre cloud · cloud sobre ink · black sobre white. En fotos, usa la versión blanca o negra sobre un área tranquila o dentro de un contenedor.

## 5. Typography
- Wordmark: **Open Sans Semibold** ("finanzas") + **Open Sans Bold** ("VZ"), convertido a contornos en los SVG (no depende de fuentes instaladas).
- UI de la app: antes de fijar tipografía, revisar `crates/finance-app/src/lib.rs`.
- Licencia: Open Sans bajo **Apache License 2.0**, apta para logotipos y uso comercial.

## 6. Don'ts
No estires ni comprimas · no recolorices fuera de la paleta · no rotes · no añadas sombras, contornos, degradados ni efectos · no reordenes ni redimensiones las partes de un lockup · no lo coloques sobre fondos recargados sin contenedor · no recreated el wordmark escribiendo el texto (usa el SVG outlineado).

## 7. Integración en la app Android
- El icono de launcher se define en `brand/android/res/` y se aplica con `scripts/install-brand-icon.sh`.
- `dx` sobrescribe solo los recursos llamados `ic_launcher*`; los nuestros se llaman `ic_launcher_brand*` y sobreviven a cada build.
- El manifest propio `crates/finance-app/android/AndroidManifest.xml` apunta a `@mipmap/ic_launcher_brand` y fija la etiqueta del launcher como `finanzasVZ` (valor literal, para que `dx` no la regenerue como `FinanceApp`). Se registra en `Dioxus.toml` como `application.android_manifest`.
- Para compilar, usa **`scripts/build-android.sh [target]`** (instala el icono, llama a `dx build` y vuelve a verificarlo). Ejemplo:
  `bash scripts/build-android.sh aarch64-linux-android`

## 8. Archivos
- Símbolo maestro: `brand/financevz-symbol.svg` (contorno relleno, sin trazos vivos).
- Variantes: `brand/variants/` (black / white / mono / square / favicon / app-icon + iconos web).
- Recursos Android: `brand/android/res/`, manifest en `crates/finance-app/android/`.
- Renders y lámina: `brand/renders/`, `brand/presentation.html`. El concepto vigente es D (`renders/concept-d.png`); `concepts.svg` es el tablero histórico de la ronda anterior.
- Generador: `brand/build_kit.py` (requiere `fonttools`).
