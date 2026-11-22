use std::{
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
        .call_method(context, "getFilesDir", "()Ljava/io/File;", &[])
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
