use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;

const DOLARAPI_URL: &str = "https://ve.dolarapi.com/v1/dolares/oficial";

#[derive(Deserialize)]
struct DolarApiResponse {
    promedio: Option<f64>,
}

pub fn fetch_exchange_rate() -> Option<i64> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .new_agent();
    let response = agent.get(DOLARAPI_URL).call().ok()?;
    let mut response = response;
    let body: DolarApiResponse = response.body_mut().read_json().ok()?;
    let rate = body.promedio?;
    if !rate.is_finite() || rate <= 0.0 {
        return None;
    }
    Some((rate * 100.0).round() as i64)
}

#[derive(Deserialize)]
struct UpdateInfo {
    #[serde(default, alias = "versionCode")]
    version_code: i64,
    #[serde(default, alias = "versionName")]
    version_name: String,
    #[serde(default = "default_apk_name", alias = "url")]
    apk: String,
}

fn default_apk_name() -> String {
    "financevz.apk".to_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateProgress {
    Checking,
    Downloading { downloaded: usize, total: Option<usize> },
    Installing,
}

/// Versión instalada (código de Gradle) para decidir si el APK descargado es más nuevo.
#[cfg(not(target_os = "android"))]
pub fn local_version() -> (i64, String) {
    (0, env!("CARGO_PKG_VERSION").to_owned())
}

#[cfg(target_os = "android")]
fn jstring<'a>(env: &mut jni::JNIEnv<'a>, value: &str) -> Result<jni::objects::JString<'a>, String> {
    env.new_string(value).map_err(|error| format!("cadena JNI: {error}"))
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

#[cfg(target_os = "android")]
fn android_open_install_settings(
    env: &mut jni::JNIEnv,
    context: &jni::objects::JObject,
    package: &str,
) -> Result<(), String> {
    use jni::objects::JValue;

    let action = jstring(env, "android.settings.MANAGE_UNKNOWN_APP_SOURCES")?;
    let intent = env
        .new_object("android/content/Intent", "(Ljava/lang/String;)V", &[
            JValue::Object(&action),
        ])
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
    env.call_method(&intent, "addFlags", "(I)Landroid/content/Intent;", &[
        JValue::Int(0x10000000),
    ])
    .map_err(|error| format!("addFlags: {error}"))?;
    env.call_method(&context, "startActivity", "(Landroid/content/Intent;)V", &[
        JValue::Object(&intent),
    ])
    .map_err(|error| format!("abrir ajustes: {error}"))?;
    Ok(())
}

#[cfg(target_os = "android")]
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
        .new_object("java/io/File", "(Ljava/lang/String;)V", &[
            JValue::Object(&path_str),
        ])
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
        .new_object("android/content/Intent", "(Ljava/lang/String;)V", &[
            JValue::Object(&action),
        ])
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
    env.call_method(&intent, "addFlags", "(I)Landroid/content/Intent;", &[
        JValue::Int(flags),
    ])
    .map_err(|error| format!("addFlags: {error}"))?;
    env.call_method(&context, "startActivity", "(Landroid/content/Intent;)V", &[
        JValue::Object(&intent),
    ])
    .map_err(|error| format!("iniciar instalación: {error}"))?;

    Ok(())
}

#[cfg(target_os = "android")]
pub fn downloaded_apk_path() -> Option<PathBuf> {
    let path = app_files_dir().ok()?.join("updates").join("financevz.apk");
    path.is_file().then_some(path)
}

#[cfg(not(target_os = "android"))]
pub fn downloaded_apk_path() -> Option<PathBuf> {
    None
}

/// Borra el APK cacheado cuando su versión ya está instalada, además de descartes de descarga.
pub fn cleanup_update_files() {
    let Ok(dir) = app_files_dir() else { return; };
    let dir = dir.join("updates");
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

/// Abre el instalador con el APK que ya quedó guardado. Útil tras conceder el permiso.
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

/// Descarga el APK desde el servidor local. La instalación se hace después desde la UI.
pub fn download_update(base_url: &str, progress: &(dyn Fn(UpdateProgress) + Send + Sync)) -> Result<String, String> {
    let base = base_url.trim().trim_end_matches('/');
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err("Escribe la URL completa, por ejemplo http://192.168.0.10:8000".into());
    }

    progress(UpdateProgress::Checking);
    let (info, without_manifest) = match fetch_update_info(base) {
        Ok(info) => (info, false),
        Err(_) => (
            UpdateInfo {
                version_code: i64::MAX,
                version_name: String::new(),
                apk: default_apk_name(),
            },
            true,
        ),
    };
    let (local_code, local_name) = local_version();
    if !without_manifest && info.version_code <= local_code {
        return Ok(format!("Ya tienes la última versión instalada ({local_name})."));
    }

    let filename = Path::new(&info.apk)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("financevz.apk");
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(900)))
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_per_read(Some(Duration::from_secs(60)))
        .timeout_recv_body(Some(Duration::from_secs(900)))
        .build()
        .new_agent();
    let mut response = agent
        .get(&format!("{base}/{filename}"))
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
    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|error| format!("crear APK temporal: {error}"))?;

    let mut reader = response
        .body_mut()
        .with_config()
        .limit(200 * 1024 * 1024)
        .reader();
    let mut downloaded = 0usize;
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
        downloaded += read;
        progress(UpdateProgress::Downloading { downloaded, total });
    }
    if downloaded < 1_000_000 {
        return Err("La descarga es demasiado pequeña para ser un APK.".into());
    }
    drop(file);
    std::fs::rename(&tmp_path, &path)
        .map_err(|error| format!("finalizar APK: {error}"))?;
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
pub fn install_update(base_url: &str, progress: &(dyn Fn(UpdateProgress) + Send + Sync)) -> Result<String, String> {
    let message = download_update(base_url, progress)?;
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
}
