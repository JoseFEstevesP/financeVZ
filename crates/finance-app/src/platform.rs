#[cfg(feature = "self-update")]
use std::io::Read;
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::Local;
use serde::Deserialize;
#[cfg(feature = "self-update")]
use sha2::{Digest, Sha256};
#[cfg(feature = "self-update")]
use std::io::Write;
use ureq::tls::TlsConfig;

const BCV_URL: &str = "https://www.bcv.org.ve";
const DOLARAPI_URL: &str = "https://ve.dolarapi.com/v1/dolares/oficial";
#[cfg(feature = "self-update")]
const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/JoseFEstevesP/financeVZ/releases/latest";

#[derive(Deserialize)]
struct DolarApiResponse {
    promedio: Option<f64>,
}

fn parse_bcv_rate(html: &str) -> Option<i64> {
    let section = html.get(html.find("id=\"dolar\"")?..)?;
    let opening = section.find("<strong")?;
    let value_start = opening + section.get(opening..)?.find('>')? + 1;
    let value_end = value_start + section.get(value_start..)?.find("</strong>")?;
    let raw = section.get(value_start..value_end)?.trim();
    let normalized = raw.replace('.', "").replace(',', ".");
    let value: f64 = normalized.parse().ok()?;
    if !value.is_finite() || value <= 0.0 || value >= 100_000.0 {
        return None;
    }
    Some((value * 100.0).round() as i64)
}

fn parse_dolarapi_rate(promedio: Option<f64>) -> Option<i64> {
    let value = promedio?;
    if !value.is_finite() || value <= 0.0 || value >= 100_000.0 {
        return None;
    }
    Some((value * 100.0).round() as i64)
}

fn fetch_rate_from_dolarapi(agent: &ureq::Agent) -> Option<i64> {
    let mut response = agent.get(DOLARAPI_URL).call().ok()?;
    let body: DolarApiResponse = response.body_mut().read_json().ok()?;
    parse_dolarapi_rate(body.promedio)
}

fn fetch_rate_from_bcv(agent: &ureq::Agent) -> Option<i64> {
    let mut response = agent
        .get(BCV_URL)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .call()
        .ok()?;
    let html = response.body_mut().read_to_string().ok()?;
    parse_bcv_rate(&html)
}

pub fn fetch_exchange_rate() -> Option<(i64, &'static str)> {
    let secure_agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .new_agent();
    let bcv_agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        // El BCV sirve una cadena de certificados incompleta; se aísla aquí su workaround.
        .tls_config(TlsConfig::builder().disable_verification(true).build())
        .build()
        .new_agent();
    fetch_rate_from_dolarapi(&secure_agent)
        .map(|rate| (rate, "dolarapi"))
        .or_else(|| fetch_rate_from_bcv(&bcv_agent).map(|rate| (rate, "bcv")))
}

#[cfg(feature = "self-update")]
#[derive(Deserialize, Clone)]
pub struct UpdateInfo {
    #[serde(default, alias = "versionCode")]
    pub version_code: i64,
    #[serde(default, alias = "versionName")]
    pub version_name: String,
    #[serde(default = "default_apk_name", alias = "url")]
    pub apk: String,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[cfg(feature = "self-update")]
fn default_apk_name() -> String {
    "financevz.apk".to_owned()
}

/// Origen de la actualización. GitHub es el predeterminado; el servidor local
/// queda disponible como opción avanzada para pruebas.
#[cfg(feature = "self-update")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateSource {
    GitHub,
    Local(String),
}

#[cfg(feature = "self-update")]
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

#[cfg(feature = "self-update")]
#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    digest: Option<String>,
    browser_download_url: String,
}

#[cfg(feature = "self-update")]
pub struct ResolvedUpdate {
    pub info: UpdateInfo,
    pub apk_url: String,
}

#[cfg(feature = "self-update")]
pub fn checksum_of(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(feature = "self-update")]
fn expected_checksum(expected: Option<&str>) -> Result<&str, String> {
    let value = expected
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "El servidor no envió el sha256 del APK; por seguridad no se descarga nada.".to_owned()
        })?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(
            "El sha256 del servidor no es válido (se esperan 64 caracteres hexadecimales)."
                .to_owned(),
        );
    }
    Ok(value)
}

#[cfg(feature = "self-update")]
fn checksum_matches(actual: &str, expected: Option<&str>) -> Result<(), String> {
    let expected = expected_checksum(expected)?;
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "El APK no coincide con la firma del servidor (sha256 {actual}). No se ha instalado nada."
        ))
    }
}

#[cfg(feature = "self-update")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateProgress {
    Checking,
    Downloading {
        downloaded: usize,
        total: Option<usize>,
    },
    Installing,
}

/// Versión instalada (código de Gradle) para decidir si el APK descargado es más nuevo.
#[cfg(not(target_os = "android"))]
pub fn local_version() -> (i64, String) {
    (0, env!("CARGO_PKG_VERSION").to_owned())
}

#[cfg(target_os = "android")]
fn jstring<'a>(
    env: &mut jni::JNIEnv<'a>,
    value: &str,
) -> Result<jni::objects::JString<'a>, String> {
    env.new_string(value)
        .map_err(|error| format!("cadena JNI: {error}"))
}

#[cfg(target_os = "android")]
fn android_local_version() -> Result<(i64, String), String> {
    use jni::objects::{JObject, JValue};

    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|error| format!("JavaVM: {error}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| format!("adjuntar hilo JNI: {error}"))?;
    let context = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };

    let package_obj = env
        .call_method(&context, "getPackageName", "()Ljava/lang/String;", &[])
        .map_err(|error| format!("getPackageName: {error}"))?
        .l()
        .map_err(|error| format!("paquete: {error}"))?;
    let package: String = env
        .get_string((&package_obj).into())
        .map_err(|error| format!("nombre de paquete: {error}"))?
        .into();
    let package_str = jstring(&mut env, &package)?;

    let manager = env
        .call_method(
            &context,
            "getPackageManager",
            "()Landroid/content/pm/PackageManager;",
            &[],
        )
        .map_err(|error| format!("getPackageManager: {error}"))?
        .l()
        .map_err(|error| format!("PackageManager: {error}"))?;
    let info = env
        .call_method(
            manager,
            "getPackageInfo",
            "(Ljava/lang/String;I)Landroid/content/pm/PackageInfo;",
            &[JValue::Object(&package_str), JValue::Int(0)],
        )
        .map_err(|error| format!("getPackageInfo: {error}"))?
        .l()
        .map_err(|error| format!("PackageInfo: {error}"))?;
    let code = env
        .get_field(&info, "versionCode", "I")
        .map_err(|error| format!("versionCode: {error}"))?
        .i()
        .map_err(|error| format!("versionCode: {error}"))? as i64;
    let name_obj = env
        .get_field(&info, "versionName", "Ljava/lang/String;")
        .map_err(|error| format!("versionName: {error}"))?
        .l()
        .map_err(|error| format!("nombre de versión: {error}"))?;
    let name: String = env
        .get_string((&name_obj).into())
        .map_err(|error| format!("versión instalada: {error}"))?
        .into();

    Ok((code, name))
}

#[cfg(target_os = "android")]
pub fn local_version() -> (i64, String) {
    android_local_version().unwrap_or((0, "desconocida".to_owned()))
}

#[cfg(all(feature = "self-update", target_os = "android"))]
fn android_open_install_settings(
    env: &mut jni::JNIEnv,
    context: &jni::objects::JObject,
    package: &str,
) -> Result<(), String> {
    use jni::objects::JValue;

    let action = jstring(env, "android.settings.MANAGE_UNKNOWN_APP_SOURCES")?;
    let intent = env
        .new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )
        .map_err(|error| format!("intent de ajustes: {error}"))?;
    let data = jstring(env, &format!("package:{package}"))?;
    let uri = env
        .call_static_method(
            "android/net/Uri",
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JValue::Object(&data)],
        )
        .map_err(|error| format!("URI de ajustes: {error}"))?
        .l()
        .map_err(|error| format!("URI de ajustes: {error}"))?;
    env.call_method(
        &intent,
        "setData",
        "(Landroid/net/Uri;)Landroid/content/Intent;",
        &[JValue::Object(&uri)],
    )
    .map_err(|error| format!("setData: {error}"))?;
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(0x10000000)],
    )
    .map_err(|error| format!("addFlags: {error}"))?;
    env.call_method(
        &context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&intent)],
    )
    .map_err(|error| format!("abrir ajustes: {error}"))?;
    Ok(())
}

#[cfg(all(feature = "self-update", target_os = "android"))]
fn android_install_apk(path: &Path) -> Result<(), String> {
    use jni::objects::{JObject, JValue};

    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|error| format!("JavaVM: {error}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| format!("adjuntar hilo JNI: {error}"))?;
    let context = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };

    let package_obj = env
        .call_method(&context, "getPackageName", "()Ljava/lang/String;", &[])
        .map_err(|error| format!("getPackageName: {error}"))?
        .l()
        .map_err(|error| format!("paquete: {error}"))?;
    let package: String = env
        .get_string((&package_obj).into())
        .map_err(|error| format!("nombre de paquete: {error}"))?
        .into();

    let manager = env
        .call_method(
            &context,
            "getPackageManager",
            "()Landroid/content/pm/PackageManager;",
            &[],
        )
        .map_err(|error| format!("getPackageManager: {error}"))?
        .l()
        .map_err(|error| format!("PackageManager: {error}"))?;
    let can_install = env
        .call_method(manager, "canRequestPackageInstalls", "()Z", &[])
        .and_then(|value| value.z())
        .unwrap_or(false);
    if !can_install {
        let _ = android_open_install_settings(&mut env, &context, &package);
        return Err("Activa «Instalar aplicaciones desconocidas» para finanzasVZ. Al volver, pulsa «Instalar APK descargado»; no hay que descargarlo otra vez.".into());
    }

    let authority = jstring(&mut env, &format!("{package}.fileprovider"))?;
    let path_str = jstring(&mut env, &path.to_string_lossy())?;
    let file = env
        .new_object(
            "java/io/File",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&path_str)],
        )
        .map_err(|error| format!("archivo APK: {error}"))?;
    let uri = env
        .call_static_method(
            "androidx/core/content/FileProvider",
            "getUriForFile",
            "(Landroid/content/Context;Ljava/lang/String;Ljava/io/File;)Landroid/net/Uri;",
            &[
                JValue::Object(&context),
                JValue::Object(&authority),
                JValue::Object(&file),
            ],
        )
        .map_err(|error| format!("FileProvider: {error}"))?
        .l()
        .map_err(|error| format!("URI del APK: {error}"))?;

    let action = jstring(&mut env, "android.intent.action.VIEW")?;
    let intent = env
        .new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )
        .map_err(|error| format!("intent de instalación: {error}"))?;
    let mime = jstring(&mut env, "application/vnd.android.package-archive")?;
    env.call_method(
        &intent,
        "setDataAndType",
        "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&uri), JValue::Object(&mime)],
    )
    .map_err(|error| format!("setDataAndType: {error}"))?;
    let flags: jni::sys::jint = 0x00000001 | 0x10000000;
    env.call_method(
        &intent,
        "addFlags",
        "(I)Landroid/content/Intent;",
        &[JValue::Int(flags)],
    )
    .map_err(|error| format!("addFlags: {error}"))?;
    env.call_method(
        &context,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&intent)],
    )
    .map_err(|error| format!("iniciar instalación: {error}"))?;

    Ok(())
}

#[cfg(feature = "self-update")]
pub fn downloaded_apk_path() -> Option<PathBuf> {
    let path = app_files_dir().ok()?.join("updates").join("financevz.apk");
    path.is_file().then_some(path)
}

#[cfg(not(feature = "self-update"))]
pub fn downloaded_apk_path() -> Option<PathBuf> {
    None
}

/// Borra el APK cacheado cuando su versión ya está instalada, además de descartes de descarga.
pub fn cleanup_update_files() {
    let Ok(dir) = app_files_dir() else {
        return;
    };
    let dir = dir.join("updates");
    #[cfg(feature = "self-update")]
    {
        let version_file = dir.join("version-code.txt");
        let cached_version = std::fs::read_to_string(&version_file)
            .ok()
            .and_then(|value| value.trim().parse::<i64>().ok());
        let (installed, _) = local_version();
        let already_installed = cached_version
            .map(|version| version <= installed)
            .unwrap_or(true);

        if already_installed {
            let _ = std::fs::remove_file(dir.join("financevz.apk"));
            let _ = std::fs::remove_file(&version_file);
        }
        let _ = std::fs::remove_file(dir.join("financevz.apk.part"));
    }
    #[cfg(not(feature = "self-update"))]
    {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Abre el instalador con el APK que ya quedó guardado. Útil tras conceder el permiso.
#[cfg(feature = "self-update")]
pub fn install_downloaded_apk() -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        let path = downloaded_apk_path()
            .ok_or("No hay un APK descargado. Pulsa «Buscar e instalar» primero.")?;
        android_install_apk(&path)?;
        Ok("Instalador abierto. Confirma la instalación en el diálogo de Android.".into())
    }
    #[cfg(not(target_os = "android"))]
    {
        Err("La instalación solo está disponible en Android.".into())
    }
}

#[cfg(not(feature = "self-update"))]
pub fn install_downloaded_apk() -> Result<String, String> {
    Err("Esta versión se actualiza desde la tienda donde la instalaste.".into())
}

#[cfg(feature = "self-update")]
fn fetch_update_info(base: &str) -> Result<UpdateInfo, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .new_agent();
    let mut response = agent
        .get(&format!("{base}/update.json"))
        .call()
        .map_err(|error| format!("no se pudo leer update.json: {error}"))?;
    response
        .body_mut()
        .read_json::<UpdateInfo>()
        .map_err(|error| format!("update.json inválido: {error}"))
}

#[cfg(feature = "self-update")]
fn version_from_tag(tag: &str) -> Result<(i64, String), String> {
    let name = tag
        .strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag);
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(format!(
            "El tag «{tag}» de GitHub no tiene el formato 0.1.31."
        ));
    }
    let code = parts[2]
        .parse::<i64>()
        .map_err(|_| format!("El tag «{tag}» de GitHub no tiene un código válido."))?;
    Ok((code, name.to_owned()))
}

#[cfg(feature = "self-update")]
fn github_release_from_json(body: &str) -> Result<ResolvedUpdate, String> {
    let release: GitHubRelease = serde_json::from_str(body)
        .map_err(|error| format!("respuesta de GitHub Releases inválida: {error}"))?;
    let (version_code, version_name) = version_from_tag(&release.tag_name)?;
    let asset = release
        .assets
        .iter()
        .find(|asset| {
            asset.name.contains(&version_name) && asset.name.to_ascii_lowercase().ends_with(".apk")
        })
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|asset| asset.name.to_ascii_lowercase().ends_with(".apk"))
        })
        .ok_or_else(|| "El release de GitHub no incluye ningún APK.".to_owned())?;
    if !asset.browser_download_url.starts_with("https://") {
        return Err("El enlace del APK en GitHub no usa HTTPS.".into());
    }
    let sha256 = asset.digest.as_deref().map(|value| {
        value
            .trim()
            .strip_prefix("sha256:")
            .unwrap_or(value.trim())
            .trim()
            .to_owned()
    });
    Ok(ResolvedUpdate {
        info: UpdateInfo {
            version_code,
            version_name,
            apk: asset.name.clone(),
            sha256,
        },
        apk_url: asset.browser_download_url.clone(),
    })
}

#[cfg(feature = "self-update")]
fn fetch_github_release() -> Result<ResolvedUpdate, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .new_agent();
    let mut response = agent
        .get(GITHUB_RELEASES_URL)
        .header("User-Agent", "financeVZ")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| {
            format!(
                "no se pudo consultar GitHub Releases: {error}. Si es un límite de peticiones, espera una hora o usa un servidor en opciones avanzadas."
            )
        })?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("no se pudo leer GitHub Releases: {error}"))?;
    github_release_from_json(&body)
}

#[cfg(feature = "self-update")]
fn is_private_http_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    if host == "localhost" || host == "[::1]" {
        return true;
    }
    let parts = host.split('.').collect::<Vec<_>>();
    let [a, b, c, d] = match parts.as_slice() {
        [a, b, c, d] => [a, b, c, d],
        _ => return false,
    };
    let (Ok(a), Ok(b), Ok(_c), Ok(_d)) = (
        a.parse::<u8>(),
        b.parse::<u8>(),
        c.parse::<u8>(),
        d.parse::<u8>(),
    ) else {
        return false;
    };
    a == 127
        || a == 10
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 168)
        || (a == 169 && b == 254)
}

#[cfg(feature = "self-update")]
pub fn validate_update_base_url(value: &str) -> Result<String, String> {
    let base = value.trim().trim_end_matches('/');
    let incomplete = "Escribe la URL completa, por ejemplo http://192.168.0.10:8000";
    if let Some(rest) = base.strip_prefix("https://") {
        if rest.split('/').next().unwrap_or("").is_empty() {
            return Err(incomplete.into());
        }
        return Ok(base.to_owned());
    }
    let Some(rest) = base.strip_prefix("http://") else {
        return Err(incomplete.into());
    };
    let authority = rest.split('/').next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return Err("La URL local no puede incluir usuario ni contraseña".into());
    }
    let host = if authority.starts_with('[') {
        authority
            .split_once(']')
            .map(|(host, _)| format!("{host}]"))
            .unwrap_or_else(|| authority.to_owned())
    } else {
        authority.rsplit_once(':').map_or_else(
            || authority.to_owned(),
            |(host, port)| {
                if port.is_empty() || port.bytes().all(|byte| byte.is_ascii_digit()) {
                    host.to_owned()
                } else {
                    authority.to_owned()
                }
            },
        )
    };
    if !is_private_http_host(&host) {
        return Err("Las URL http:// solo se permiten en la red local (localhost, 127.x, 10.x, 172.16-31.x, 192.168.x o 169.254.x). Usa https:// para Internet.".into());
    }
    Ok(base.to_owned())
}

#[cfg(feature = "self-update")]
pub fn resolve_update(source: &UpdateSource) -> Result<ResolvedUpdate, String> {
    match source {
        UpdateSource::GitHub => fetch_github_release(),
        UpdateSource::Local(base_url) => {
            let base = validate_update_base_url(base_url)?;
            let info = fetch_update_info(&base)?;
            let filename = Path::new(&info.apk)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("financevz.apk")
                .to_owned();
            Ok(ResolvedUpdate {
                info,
                apk_url: format!("{base}/{filename}"),
            })
        }
    }
}

/// Descarga el APK desde GitHub Releases o desde el servidor local. La instalación
/// se hace después desde la UI, siempre mediante un clic del usuario.
#[cfg(feature = "self-update")]
pub fn download_update(
    source: &UpdateSource,
    progress: &(dyn Fn(UpdateProgress) + Send + Sync),
) -> Result<String, String> {
    progress(UpdateProgress::Checking);
    let ResolvedUpdate { info, apk_url } = resolve_update(source)?;
    expected_checksum(info.sha256.as_deref())?;
    let (local_code, local_name) = local_version();
    if info.version_code <= local_code {
        return Ok(format!(
            "Ya tienes la última versión instalada ({local_name})."
        ));
    }

    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(900)))
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_per_read(Some(Duration::from_secs(60)))
        .timeout_recv_body(Some(Duration::from_secs(900)))
        .build()
        .new_agent();
    let mut response = agent
        .get(&apk_url)
        .call()
        .map_err(|error| format!("no se pudo descargar el APK: {error}"))?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    let dir = app_files_dir()?.join("updates");
    std::fs::create_dir_all(&dir).map_err(|error| format!("crear carpeta updates: {error}"))?;
    let path = dir.join("financevz.apk");
    let tmp_path = dir.join("financevz.apk.part");
    let mut file =
        std::fs::File::create(&tmp_path).map_err(|error| format!("crear APK temporal: {error}"))?;

    let mut reader = response
        .body_mut()
        .with_config()
        .limit(200 * 1024 * 1024)
        .reader();
    let mut downloaded = 0usize;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("no se pudo leer el APK: {error}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|error| format!("guardar APK: {error}"))?;
        hasher.update(&buffer[..read]);
        downloaded += read;
        progress(UpdateProgress::Downloading { downloaded, total });
    }
    if downloaded < 1_000_000 {
        let _ = std::fs::remove_file(&tmp_path);
        return Err("La descarga es demasiado pequeña para ser un APK.".into());
    }
    if let Err(error) =
        checksum_matches(&format!("{:x}", hasher.finalize()), info.sha256.as_deref())
    {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(error);
    }
    drop(file);
    std::fs::rename(&tmp_path, &path).map_err(|error| format!("finalizar APK: {error}"))?;
    let _ = std::fs::write(dir.join("version-code.txt"), info.version_code.to_string());

    log::info!("APK descargado en {path:?} ({downloaded} bytes)");

    let version = if info.version_name.is_empty() {
        "nueva"
    } else {
        info.version_name.as_str()
    };
    Ok(format!(
        "Versión {version} descargada. Pulsa «Instalar APK descargado» para continuar."
    ))
}

/// Descarga e instala. La UI usa `download_update` + `install_downloaded_apk` por separado
/// para que `startActivity` se ejecute siempre en el hilo principal de Android.
#[cfg(feature = "self-update")]
pub fn install_update(
    source: &UpdateSource,
    progress: &(dyn Fn(UpdateProgress) + Send + Sync),
) -> Result<String, String> {
    let message = download_update(source, progress)?;
    #[cfg(target_os = "android")]
    {
        install_downloaded_apk()?;
        Ok(format!("{message} Instalador abierto."))
    }
    #[cfg(not(target_os = "android"))]
    Ok(message)
}

#[cfg(target_os = "android")]
fn app_files_dir() -> Result<PathBuf, String> {
    use jni::objects::JObject;

    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|error| format!("JavaVM: {error}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| format!("attach JNI thread: {error}"))?;
    let context = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };

    let file = env
        .call_method(&context, "getFilesDir", "()Ljava/io/File;", &[])
        .map_err(|error| format!("getFilesDir: {error}"))?
        .l()
        .map_err(|error| format!("getFilesDir result: {error}"))?;
    let path = env
        .call_method(file, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .map_err(|error| format!("getAbsolutePath: {error}"))?
        .l()
        .map_err(|error| format!("getAbsolutePath result: {error}"))?;
    let dir: String = env
        .get_string((&path).into())
        .map_err(|error| format!("JNI string: {error}"))?
        .into();

    Ok(PathBuf::from(dir))
}

#[cfg(not(target_os = "android"))]
fn app_files_dir() -> Result<PathBuf, String> {
    Ok(std::env::var("FINANCEVZ_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))))
}

pub fn database_path() -> Result<PathBuf, String> {
    Ok(app_files_dir()?.join("financevz.db"))
}

/// Guarda el JSON de la copia en la carpeta privada y conserva solo las últimas `keep`.
pub fn write_auto_backup(json: &str, keep: usize) -> Option<String> {
    let dir = app_files_dir().ok()?.join("backups");
    std::fs::create_dir_all(&dir).ok()?;
    let filename = format!("financevz-{}.json", Local::now().format("%Y%m%d-%H%M%S"));
    let path = dir.join(filename);
    std::fs::write(&path, json).ok()?;

    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|candidate| {
            candidate
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("financevz-") && name.ends_with(".json"))
                .unwrap_or(false)
        })
        .collect();
    files.sort_by(|left, right| right.cmp(left));
    for stale in files.into_iter().skip(keep) {
        let _ = std::fs::remove_file(stale);
    }
    Some(path.to_string_lossy().into_owned())
}

/// Borra la imagen de evidencia indicada por su ruta guardada.
pub fn delete_evidence(path: &str) {
    let Some(root) = app_files_dir().ok() else {
        return;
    };
    let Some(name) = Path::new(path).file_name() else {
        return;
    };
    let _ = std::fs::remove_file(root.join("evidence").join(name));
}

pub fn save_evidence(bytes: &[u8], name: &str) -> Result<(String, String), String> {
    let dir = app_files_dir()?.join("evidence");
    std::fs::create_dir_all(&dir).map_err(|error| format!("crear carpeta evidence: {error}"))?;

    let extension = Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("jpg")
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(5)
        .collect::<String>()
        .to_lowercase();
    let extension = if extension.is_empty() {
        "jpg".to_owned()
    } else {
        extension
    };
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let filename = format!("ev_{millis}.{extension}");
    let path = dir.join(&filename);
    std::fs::write(&path, bytes).map_err(|error| format!("guardar evidencia: {error}"))?;

    Ok((path.to_string_lossy().into_owned(), name.to_owned()))
}

/// Devuelve la evidencia como data URL para mostrarla dentro de la WebView.
pub fn read_evidence_data_url(path: &str) -> Option<String> {
    let root = app_files_dir().ok()?.join("evidence");
    let candidate = Path::new(path);
    let name = candidate.file_name()?;
    let bytes = std::fs::read(root.join(name)).ok()?;
    let mime = match name
        .to_string_lossy()
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/jpeg",
    };
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:{mime};base64,{encoded}"))
}

#[cfg(target_os = "android")]
pub fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Trace)
            .with_tag("finanzasVZ"),
    );
}

#[cfg(not(target_os = "android"))]
pub fn init_logging() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bcv_dollar_rate() {
        let html = r#"<div id="dolar" class="col-sm-12 col-xs-12 ">
            <strong class="strong-tb">919,82270000</strong>
        </div>"#;
        assert_eq!(parse_bcv_rate(html), Some(91_982));
    }

    #[test]
    fn parses_bcv_rate_with_thousands_separator() {
        let html = r#"id="euro"><strong>1.011,17949056</strong></div>
        id="dolar"><strong>1.011,17949056</strong>"#;
        assert_eq!(parse_bcv_rate(html), Some(101_118));
    }

    #[test]
    fn rejects_missing_or_invalid_bcv_rate() {
        assert_eq!(
            parse_bcv_rate("id=\"euro\"><strong>919,82270000</strong>"),
            None
        );
        assert_eq!(parse_bcv_rate("id=\"dolar\">sin valor"), None);
        assert_eq!(parse_bcv_rate("id=\"dolar\"><strong>abc</strong>"), None);
        assert_eq!(parse_bcv_rate("id=\"dolar\"><strong>0,00</strong>"), None);
    }

    #[test]
    fn parses_dolarapi_rate() {
        assert_eq!(parse_dolarapi_rate(Some(922.5637)), Some(92_256));
        assert_eq!(parse_dolarapi_rate(Some(857.89)), Some(85_789));
        assert_eq!(parse_dolarapi_rate(None), None);
        assert_eq!(parse_dolarapi_rate(Some(0.0)), None);
        assert_eq!(parse_dolarapi_rate(Some(f64::NAN)), None);
    }

    #[cfg(feature = "self-update")]
    #[test]
    fn parses_update_json_in_both_formats() {
        let current: UpdateInfo = serde_json::from_str(
            r#"{"version_code":5,"version_name":"0.1.5","apk":"financevz.apk"}"#,
        )
        .unwrap();
        assert_eq!(current.version_code, 5);
        assert_eq!(current.version_name, "0.1.5");
        assert_eq!(current.apk, "financevz.apk");

        let legacy: UpdateInfo = serde_json::from_str(
            r#"{"versionCode":4,"versionName":"0.1.4","url":"financevz.apk"}"#,
        )
        .unwrap();
        assert_eq!(legacy.version_code, 4);
        assert_eq!(legacy.version_name, "0.1.4");
        assert_eq!(legacy.apk, "financevz.apk");
    }

    #[cfg(feature = "self-update")]
    #[test]
    fn verifies_apk_checksum_before_installing() {
        let hash = checksum_of(b"apk de prueba");
        assert!(checksum_matches(&hash, Some(&hash)).is_ok());
        assert!(checksum_matches(&hash, None).is_err());
        assert!(checksum_matches(&hash, Some("000000")).is_err());
        assert!(checksum_matches(&hash, Some(&"0".repeat(64))).is_err());
        let manifest_hash = "ab".repeat(32);
        let with_checksum: UpdateInfo = serde_json::from_str(&format!(
            r#"{{"version_code":6,"version_name":"0.1.6","apk":"financevz.apk","sha256":"{manifest_hash}"}}"#
        ))
        .unwrap();
        assert_eq!(
            with_checksum.sha256.as_deref(),
            Some(manifest_hash.as_str())
        );
        assert!(expected_checksum(with_checksum.sha256.as_deref()).is_ok());
    }

    #[cfg(feature = "self-update")]
    #[test]
    fn rejects_manifest_without_checksum() {
        let current: UpdateInfo = serde_json::from_str(
            r#"{"version_code":6,"version_name":"0.1.6","apk":"financevz.apk"}"#,
        )
        .unwrap();
        assert!(expected_checksum(current.sha256.as_deref()).is_err());

        let legacy: UpdateInfo = serde_json::from_str(
            r#"{"versionCode":6,"versionName":"0.1.6","url":"financevz.apk","sha256":""}"#,
        )
        .unwrap();
        assert!(expected_checksum(legacy.sha256.as_deref()).is_err());
    }

    #[cfg(feature = "self-update")]
    #[test]
    fn derives_version_from_github_tag() {
        assert_eq!(
            version_from_tag("v0.1.31").unwrap(),
            (31, "0.1.31".to_owned())
        );
        assert_eq!(version_from_tag("0.2.7").unwrap(), (7, "0.2.7".to_owned()));
        assert!(version_from_tag("0.1").is_err());
        assert!(version_from_tag("vX.Y.Z").is_err());
        assert!(version_from_tag("v0.1.31.4").is_err());
    }

    #[cfg(feature = "self-update")]
    #[test]
    fn parses_github_release_with_sha256_digest() {
        let body = r#"{"tag_name":"v0.1.31","assets":[{"name":"financevz-0.1.31.apk","digest":"sha256:9ff3d99b3bfabc08820f3044f135b8b9e47d75ae83e134bf5ad81019bb7ab137","browser_download_url":"https://github.com/JoseFEstevesP/financeVZ/releases/download/v0.1.31/financevz-0.1.31.apk"}]}"#;
        let resolved = github_release_from_json(body).unwrap();
        assert_eq!(resolved.info.version_code, 31);
        assert_eq!(resolved.info.version_name, "0.1.31");
        assert_eq!(resolved.info.apk, "financevz-0.1.31.apk");
        assert!(expected_checksum(resolved.info.sha256.as_deref()).is_ok());
        assert!(resolved.apk_url.ends_with("/financevz-0.1.31.apk"));
    }

    #[cfg(feature = "self-update")]
    #[test]
    fn rejects_github_release_without_checksum_or_apk() {
        let without_digest = r#"{"tag_name":"v0.1.31","assets":[{"name":"financevz-0.1.31.apk","browser_download_url":"https://github.com/x.apk"}]}"#;
        let resolved = github_release_from_json(without_digest).unwrap();
        assert!(expected_checksum(resolved.info.sha256.as_deref()).is_err());

        let no_apk = r#"{"tag_name":"v0.1.31","assets":[{"name":"notas.txt","browser_download_url":"https://github.com/x.txt"}]}"#;
        assert!(github_release_from_json(no_apk).is_err());
    }

    #[cfg(feature = "self-update")]
    #[test]
    fn rejects_local_update_source_without_scheme() {
        assert!(resolve_update(&UpdateSource::Local("192.168.0.10:8000".into())).is_err());
        assert_eq!(UpdateSource::GitHub, UpdateSource::GitHub);
    }

    #[cfg(feature = "self-update")]
    #[test]
    fn validates_local_update_urls() {
        assert_eq!(
            validate_update_base_url("http://192.168.0.10:8000/").unwrap(),
            "http://192.168.0.10:8000"
        );
        assert!(validate_update_base_url("http://127.0.0.1:8000").is_ok());
        assert!(validate_update_base_url("http://10.0.0.5:8000").is_ok());
        assert!(validate_update_base_url("https://example.com/updates").is_ok());
        assert!(validate_update_base_url("http://example.com:8000").is_err());
        assert!(validate_update_base_url("http://192.168.0.10@evil.com").is_err());
        assert!(validate_update_base_url("ftp://192.168.0.10").is_err());
    }
}
