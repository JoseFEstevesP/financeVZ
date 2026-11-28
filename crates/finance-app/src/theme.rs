use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub css_class: String,
    pub is_dark: bool,
    pub preview: [String; 4],
    #[serde(default)]
    pub vars: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub extra_css: String,
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeCatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_dark: bool,
    pub preview: [String; 4],
    pub download_url: String,
    #[serde(default)]
    pub version: String,
}

pub const THEME_CATALOG_URL: &str = "https://josefestevesp.github.io/financeVZ/themes/catalog.json";

pub fn builtin_themes() -> Vec<ThemeDef> {
    vec![
        builtin(
            "dark",
            "Oscuro",
            "El tema predeterminado actual.",
            "theme-dark",
            true,
            ["#0f1420", "#161d2b", "#4f8cff", "#2dd4a7"],
        ),
        builtin(
            "light",
            "Claro",
            "Claro y limpio para buena luz.",
            "theme-light",
            false,
            ["#f7f9fd", "#ffffff", "#316cdb", "#0f9d78"],
        ),
    ]
}

fn builtin(
    id: &str,
    name: &str,
    description: &str,
    css_class: &str,
    is_dark: bool,
    preview: [&str; 4],
) -> ThemeDef {
    ThemeDef {
        id: id.to_owned(),
        name: name.to_owned(),
        description: description.to_owned(),
        css_class: css_class.to_owned(),
        is_dark,
        preview: preview.map(|c| c.to_owned()),
        vars: std::collections::BTreeMap::new(),
        extra_css: String::new(),
        builtin: true,
    }
}

pub fn normalize_id(id: &str) -> String {
    match id.trim() {
        "light" => "light".to_owned(),
        "oled" | "oled-black" => "oled".to_owned(),
        "sepia" => "sepia".to_owned(),
        "high-contrast" | "high_contrast" | "contrast" => "high-contrast".to_owned(),
        "commit" | "terminal" | "git" => "commit".to_owned(),
        "dark" | "" => "dark".to_owned(),
        other => sanitize_id(other).unwrap_or_else(|| "dark".to_owned()),
    }
}

pub fn sanitize_id(id: &str) -> Option<String> {
    let clean: String = id
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let clean = clean.trim_matches('-').to_owned();
    if clean.is_empty() || clean.len() > 48 {
        return None;
    }
    if builtin_themes().iter().any(|t| t.id == clean) {
        return Some(clean);
    }
    Some(clean)
}

pub fn default_theme_id() -> &'static str {
    "dark"
}

pub fn resolve<'a>(id: &str, themes: &'a [ThemeDef]) -> &'a ThemeDef {
    let normalized = normalize_id(id);
    themes
        .iter()
        .find(|theme| theme.id == normalized)
        .or_else(|| themes.iter().find(|theme| theme.id == "dark"))
        .expect("builtin dark theme")
}

pub fn themes_dir() -> Option<PathBuf> {
    crate::platform::app_files_dir()
        .ok()
        .map(|d| d.join("themes"))
}

pub fn load_installed_themes() -> Vec<ThemeDef> {
    let Some(dir) = themes_dir() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut themes = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(mut theme) = serde_json::from_str::<ThemeDef>(&text)
            && validate_theme(&theme).is_ok()
        {
            theme.builtin = false;
            theme.css_class = format!("theme-{}", theme.id);
            themes.push(theme);
        }
    }
    themes.sort_by_key(|a| a.name.to_lowercase());
    themes
}

pub fn save_installed_theme(theme: &ThemeDef) -> Result<PathBuf, String> {
    validate_theme(theme)?;
    let dir = themes_dir().ok_or_else(|| "No se pudo acceder al almacenamiento".to_owned())?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("No se pudo crear carpeta de temas: {e}"))?;
    let path = dir.join(format!("{}.json", theme.id));
    let json = serde_json::to_string_pretty(theme).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("No se pudo guardar el tema: {e}"))?;
    Ok(path)
}

pub fn remove_installed_theme(id: &str) -> Result<(), String> {
    let id = sanitize_id(id).ok_or_else(|| "ID de tema inválido".to_owned())?;
    if builtin_themes().iter().any(|t| t.id == id) {
        return Err("No se puede eliminar un tema integrado".to_owned());
    }
    let dir = themes_dir().ok_or_else(|| "No se pudo acceder al almacenamiento".to_owned())?;
    let path = dir.join(format!("{id}.json"));
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| format!("No se pudo eliminar el tema: {e}"))?;
    }
    Ok(())
}

pub fn validate_theme(theme: &ThemeDef) -> Result<(), String> {
    let id = sanitize_id(&theme.id).ok_or_else(|| "ID de tema inválido".to_owned())?;
    if theme.builtin && id != "dark" && id != "light" {
        return Err("Solo oscuro y claro pueden ser temas integrados".to_owned());
    }
    if !theme.builtin && (id == "dark" || id == "light") {
        return Err("No se puede reemplazar un tema integrado".to_owned());
    }
    if theme.name.trim().is_empty() {
        return Err("El tema no tiene nombre".to_owned());
    }
    if theme.preview.len() != 4 {
        return Err("El tema debe tener 4 colores de preview".to_owned());
    }
    for c in &theme.preview {
        validate_color(c)?;
    }
    if theme.extra_css.len() > 64_000 {
        return Err("El CSS extra del tema es demasiado grande".to_owned());
    }
    if !theme.builtin {
        if theme.vars.is_empty() {
            return Err("El tema descargado no tiene variables CSS".to_owned());
        }
        for (k, v) in &theme.vars {
            if !k.starts_with("--") || k.chars().any(|c| !(c.is_ascii_alphanumeric() || c == '-')) {
                return Err(format!("Variable CSS inválida: {k}"));
            }
            validate_color(v)?;
        }
        for required in REQUIRED_VARS {
            if !theme.vars.contains_key(*required) {
                return Err(format!("Falta la variable {required}"));
            }
        }
    }
    Ok(())
}

fn validate_color(value: &str) -> Result<(), String> {
    let v = value.trim();
    let ok = if let Some(hex) = v.strip_prefix('#') {
        (hex.len() == 3 || hex.len() == 6 || hex.len() == 8)
            && hex.chars().all(|c| c.is_ascii_hexdigit())
    } else if v.starts_with("rgb(")
        || v.starts_with("rgba(")
        || v.starts_with("hsl(")
        || v.starts_with("hsla(")
    {
        v.ends_with(')') && v.len() < 64
    } else {
        false
    };
    if ok {
        Ok(())
    } else {
        Err(format!("Color inválido: {v}"))
    }
}

pub const REQUIRED_VARS: &[&str] = &[
    "--bg",
    "--text",
    "--muted",
    "--card",
    "--border",
    "--input-bg",
    "--input-border",
    "--primary",
    "--primary-text",
    "--accent",
    "--link",
    "--link-border",
    "--chip-bg",
    "--chip-text",
    "--chip-active-bg",
    "--nav-bg",
    "--nav-border",
    "--hero-start",
    "--hero-end",
    "--hero-border",
    "--secondary-bg",
    "--secondary-text",
    "--row-border",
    "--toast-bg",
    "--toast-text",
    "--toast-border",
    "--alert-bg",
    "--alert-border",
    "--urgent",
    "--positive",
    "--negative",
    "--overlay",
    "--modal-bg",
    "--bar-bg",
    "--bar-start",
    "--bar-end",
    "--evidence-bg",
    "--file-border",
    "--file-active",
    "--shadow",
];

pub fn css_for_installed(themes: &[ThemeDef]) -> String {
    let mut out = String::new();
    for theme in themes.iter().filter(|t| !t.builtin) {
        if validate_theme(theme).is_err() {
            continue;
        }
        out.push_str(&format!(".{} {{\n", theme.css_class));
        for (k, v) in &theme.vars {
            out.push_str(&format!("  {k}: {v};\n"));
        }
        out.push_str("}\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_themes_and_aliases() {
        let themes = builtin_themes();
        assert_eq!(resolve("dark", &themes).id, "dark");
        assert_eq!(resolve("light", &themes).id, "light");
        // aliases normalize; resolve falls back to dark if not installed
        assert_eq!(normalize_id("oled-black"), "oled");
        assert_eq!(normalize_id("high_contrast"), "high-contrast");
        assert_eq!(resolve("oled-black", &themes).id, "dark");
    }

    #[test]
    fn falls_back_to_default_for_unknown() {
        let themes = builtin_themes();
        assert_eq!(resolve("unknown", &themes).id, "dark");
        assert_eq!(resolve("", &themes).id, "dark");
    }

    #[test]
    fn validates_custom_theme() {
        let mut vars = std::collections::BTreeMap::new();
        for k in REQUIRED_VARS {
            vars.insert((*k).to_owned(), "#112233".to_owned());
        }
        let theme = ThemeDef {
            id: "nord".to_owned(),
            name: "Nord".to_owned(),
            description: "Test".to_owned(),
            css_class: "theme-nord".to_owned(),
            is_dark: true,
            preview: [
                "#112233".to_owned(),
                "#223344".to_owned(),
                "#334455".to_owned(),
                "#445566".to_owned(),
            ],
            vars,
            extra_css: String::new(),
            builtin: false,
        };
        assert!(validate_theme(&theme).is_ok());
    }
}
