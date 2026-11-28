# Changelog

Todos los cambios notables de financeVZ. El formato sigue Keep a Changelog.

## [Unreleased]

## [0.1.52] - 2026-11-28

### Añadido

- Catálogo web de temas (`themes.html` + `themes/catalog.json`) con temas descargables.
- Configuración: buscar/descargar temas desde el catálogo, instalar/quitar temas locales y aplicar CSS de temas instalados.

### Cambiado

- Sistema de temas extensible por `theme_id`, con validación de temas descargados y persistencia local.

## [0.1.51] - 2026-11-28

### Añadido

- Configuración: catálogo local extensible de temas (Oscuro, Claro, OLED negro, Sepia, Alto contraste, Estilo commit) con preview y persistencia.

### Cambiado

- Tema aplicado por `theme_id` con compatibilidad para valores anteriores; bridge/Kotlin actualizados para barras del sistema por tema.

### Eliminado

- Ruta muerta `View::Update`; la actualización queda embebida en Ajustes.

## [0.1.50] - 2026-11-28

### Corregido

- Exportar copia ya abre el selector de guardado de Android (SAF) para elegir carpeta/archivo, en vez de volcar el JSON en pantalla.
- Copias automáticas: se puede crear y exportar, o exportar la última copia guardada en almacenamiento privado.

## [0.1.48] - 2026-11-28

### Corregido

- El banner de actualización ya no muestra versiones obsoletas guardadas en ajustes; al cargar se valida contra la versión instalada y se limpia si ya no aplica.

## [0.1.47] - 2026-11-28

### Cambiado

- **Cambio incompatible**: el identificador de aplicación Android pasa de `com.gato99.financevz` a `com.joseE.financevz`. Las instalaciones anteriores no se actualizan encima; hay que desinstalar/instalar o migrar datos.

### Añadido

- Metadatos fastlane (es-ES/en-US) para distribución en F-Droid.
- `FINANCEVZ_FDROID=1` en `scripts/build-release.sh` genera APK release sin firma del proyecto y sin escribir `updates/`.

## [0.1.46] - 2026-11-27

### Añadido

- La pestaña Actualizar muestra un botón «Activar avisos de actualización» cuando el permiso de notificaciones está desactivado.
- Inicio y Actividad muestran el saldo total y el saldo acumulado después de cada movimiento.

### Corregido

- Los builds Android ya no compilan Kotlin personalizado sobrante de un build anterior cuando `dx build` regenera el proyecto.

## [0.1.45] - 2026-11-27

### Corregido

- La notificación de actualización disponible se cancela automáticamente al abrir la app cuando la versión instalada ya es igual o superior, evitando avisos obsoletos.
- El worker de comprobación de actualizaciones también limpia notificaciones obsoletas cuando detecta que no hay una versión nueva.

## [0.1.44] - 2026-11-27

### Corregido

- El actualizador limpia APKs temporales y cacheados al volver a la app cuando la versión descargada ya fue instalada.
- El botón de buscar/descargar actualización se deshabilita mientras hay una descarga en segundo plano en curso para evitar descargas duplicadas.

## [0.1.43] - 2026-11-27

### Corregido

- El actualizador en Android localiza correctamente el APK descargado aunque DownloadManager lo renombre (por ejemplo `financevz-update-1.apk`), evitando el falso error “El APK descargado no es válido”.

## [0.1.42] - 2026-11-27

### Añadido

- Variante de build F-Droid: `FINANCEVZ_FDROID=1` compila sin auto-actualizador, sin `REQUEST_INSTALL_PACKAGES` y sin WorkManager/notificaciones de actualización; la actualización queda a cargo de la tienda. El build GitHub mantiene el actualizador (`self-update` por defecto).

### Cambiado

- Las horas visibles en el historial de actividad y en el estado de la copia automática usan formato de 12 horas con `a. m.`/`p. m.` en lugar de 24.

### Corregido

- La política de privacidad enumera ahora DolarAPI, BCV y GitHub Releases, que son las únicas conexiones de la app.
- DolarAPI se consulta con verificación TLS normal; el workaround de certificados del BCV queda aislado.
- El actualizador solo acepta URLs `http://` de la red local (localhost, `127.x`, `10.x`, `172.16-31.x`, `192.168.x` o `169.254.x`); para Internet exige `https://`.

## [0.1.40] - 2026-11-26

### Corregido

- Cashea: se puede indicar la **fecha de compra**; las cuotas e inicial se calculan desde esa fecha (cada 14 días), evitando fechas de pago incorrectas al registrar compras pasadas.

## [0.1.39] - 2026-11-26

### Añadido

- Pagos: nuevo método **Tarjeta de crédito** (opción A) con día de corte y día límite de pago; monto variable (puede ser 0).
- Recomendación en fila de tarjeta de crédito: pagar 1 día antes del corte para reportar menor saldo y pagar antes del límite para evitar mora/intereses (orientación general, confirmar con emisor).

## [0.1.38] - 2026-11-26

### Añadido

- La actualización de APK puede descargarse en segundo plano con DownloadManager (notificación y continúa si cierras la app). La instalación sigue siendo manual.

## [0.1.37] - 2026-11-26

### Corregido

- Cashea permite registrar compras con **inicial 0%/0**: no se crea gasto de inicial cuando es 0 y se generan solo las cuotas cada 14 días.

## [0.1.36] - 2026-11-25

### Añadido

- Tarjeta de **Ingresos de este mes** en Inicio, con total en USD, equivalente en Bs y accesos para añadir o ver ingresos.
- Acceso al detalle de movimientos desde Reportes.

### Cambiado

- Los botones de «Últimos gastos» (devolver, editar y eliminar) se muestran en columna, con más espacio entre los datos de cada gasto.

## [0.1.35] - 2026-11-25

### Corregido

- El total mensual en Bs (Inicio/Ingresos) suma los montos originales ingresados en Bs cuando aplica, evitando mostrar redondeos como 20.803,73 en lugar de 20.800,00.

## [0.1.34] - 2026-11-25

### Corregido

- Los montos ingresados en Bs ahora conservan su valor original exacto para mostrarlos y editarlos, evitando redondeos de ida/vuelta Bs→USD→Bs (ej. 20.800,00 Bs).

## [0.1.33] - 2026-11-25

### Corregido

- La tasa automática ahora se obtiene de DolarAPI (`promedio`, dólar oficial) con respaldo del BCV, evitando conversiones Bs→USD con valores desactualizados.

### Añadido

- Refresco automático de la tasa cada 30 minutos mientras la app está abierta, además de la consulta al iniciar.

## [0.1.32] - 2026-11-24

### Cambiado

- El actualizador consulta GitHub Releases por defecto: toma el tag, el asset APK y su SHA-256 de la API pública, sin depender de un servidor local. La instalación sigue siendo manual.
- El campo de URL del servidor local se movió a **Usar un servidor local (opciones avanzadas)**. Con el campo vacío se usa GitHub; borrarlo restablece ese comportamiento.

## [0.1.31] - 2026-11-24

### Añadido

- Tipo de movimiento en ingresos y gastos: Efectivo, Banco · Transferencia, Banco · Pago móvil, Página externa y Otro. Se muestra en los listados y se conserva al editar, devolver o exportar/importar copias.
- Devolución de un gasto desde Inicio y Buscar movimientos: crea un ingreso del mismo importe, fecha, tasa y canal, con la nota «Devolución de …», sin borrar el gasto original.
- Confirmación antes de eliminar un gasto, ingreso o pago desde cualquier vista, con borrado también de la imagen de evidencia del gasto.

### Cambiado

- Los pagos recurrentes pasan su canal al gasto generado; Cashea se registra como Página externa sin perder su esquema de inicial y cuotas.

## [0.1.30] - 2026-11-24

### Añadido

- Botón **Inicio** en el menú inferior, para que no haga falta saber que el logo de la cabecera vuelve al inicio.

## [0.1.29] - 2026-11-24

### Cambiado

- La tasa del dólar se lee directamente del dólar oficial publicado por el BCV, en lugar de DolarAPI, que dejó de actualizarse.

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
