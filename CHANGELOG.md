# Changelog

Todos los cambios notables de financeVZ. El formato sigue Keep a Changelog.

## [0.1.28] - 2026-11-23

### Añadido

- APK firmado publicado en GitHub Releases y página de descarga en GitHub Pages.

## [0.1.26] - 2026-11-22

### Corregido

- El actualizador exige `sha256` en `update.json` (64 caracteres hexadecimales) y rechaza la descarga si falta o no es válido; ya no descarga un APK de respaldo cuando no puede leer el manifiesto.
- El manifiesto se valida antes de transferir el APK, para no descargar un archivo que sería rechazado.

## [0.1.25] - 2026-11-22

### Cambiado

- La hoja de estilos usa `rem` para tamaños, espaciados e iconos; `1px` queda reservado a bordes finos y `999px` a las píldoras.
- El selector de tamaño de texto ajusta directamente el `font-size` de `html` (14 / 16 / 18 px), con lo que escala toda la interfaz con una sola regla.
- El WebView fija `textZoom = 100` para que el tamaño de fuente del sistema no modifique el diseño; el control lo tiene el selector de la app.

## [0.1.23] - 2026-11-22

### Corregido

- La opción de huella siempre aparece en Configuración, incluso antes de activar el PIN, con una explicación de que el PIN es el respaldo.

## [0.1.22] - 2026-11-22

### Añadido

- Vista **Configuración** accesible desde el engranaje de la cabecera; reúne tema, copias, seguridad y tamaño de texto.
- Tema claro además del oscuro, aplicado también a la barra de estado y navegación de Android y recordado entre reinicios.
- Desbloqueo con huella real mediante `BiometricPrompt`, con el PIN como respaldo obligatorio.
- Copia automática local en `backups/` al iniciar y cada 12 horas (intervalo ajustable), con rotación de 3 copias y sin permisos de almacenamiento.

### Cambiado

- La cabecera muestra un engranaje como único acceso a Configuración; se eliminó el enlace duplicado de Inicio.
- Las fechas visibles usan `dd/mm/aaaa`; los campos de fecha siguen guardando ISO internamente.

### Corregido

- Separación visual entre el campo del PIN y el botón **Activar PIN**.

## [0.1.20] - 2026-11-22

### Añadido

- Copia de seguridad completa en JSON (exportar/descargar y restaurar desde archivo o texto), con importación transaccional.
- Edición de gastos e ingresos desde Inicio y desde el buscador.
- Presupuestos mensuales por categoría y gasto real del mes.
- Búsqueda y filtros de movimientos por texto, categoría y rango de fechas.
- Tipo de gasto recurrente general, además de los pagos.
- Visor de evidencia dentro de la WebView.
- Origen y fecha de cada tasa, con historial en Reportes.
- Bloqueo por PIN de 4 dígitos (SHA-256) y tamaño de texto ajustable.
- Verificación SHA-256 del APK descargado por el actualizador.
- Build release firmado (`scripts/setup-keystore.sh` y `scripts/build-release.sh`), CI y documentación.

## [0.1.18] - 2026-11-22

### Cambiado

- El actualizador ya no instala automáticamente: descarga el APK y deja que la persona pulse **Instalar APK descargado**. Así el intent de instalación se ejecuta desde la UI, en el hilo principal de Android.
- El menú inferior ya no incluye la entrada **Inicio**.

### Añadido

- El símbolo de marca de la cabecera es clicable y sirve para volver a **Inicio**.

## [0.1.16] - 2026-11-22

### Añadido

- Botón flotante (FAB) en la interfaz principal.

### Corregido

- Limpieza al iniciar de archivos de actualización: se descarta `financevz.apk` y su `version-code.txt` cuando la versión cacheada ya está instalada, y se borra siempre el descarte `financevz.apk.part`.

## [0.1.15] - 2026-11-22

### Añadido

- Barra de estado y barra de navegación de Android con los colores de la app, iconos claros sobre fondo oscuro.
