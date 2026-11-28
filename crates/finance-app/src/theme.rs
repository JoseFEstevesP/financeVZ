#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub css_class: &'static str,
    pub is_dark: bool,
    pub preview: [&'static str; 4],
}

pub const THEMES: &[ThemeDef] = &[
    ThemeDef {
        id: "dark",
        name: "Oscuro",
        description: "El tema predeterminado actual.",
        css_class: "theme-dark",
        is_dark: true,
        preview: ["#0f1420", "#161d2b", "#4f8cff", "#2dd4a7"],
    },
    ThemeDef {
        id: "light",
        name: "Claro",
        description: "Claro y limpio para buena luz.",
        css_class: "theme-light",
        is_dark: false,
        preview: ["#f7f9fd", "#ffffff", "#316cdb", "#0f9d78"],
    },
    ThemeDef {
        id: "oled",
        name: "OLED negro",
        description: "Negro puro, ideal para pantallas OLED.",
        css_class: "theme-oled",
        is_dark: true,
        preview: ["#000000", "#101010", "#4f8cff", "#2dd4a7"],
    },
    ThemeDef {
        id: "sepia",
        name: "Sepia",
        description: "Tonos cálidos para lectura cómoda.",
        css_class: "theme-sepia",
        is_dark: false,
        preview: ["#f4ecd8", "#fff8e8", "#8b5e34", "#5f8f3a"],
    },
    ThemeDef {
        id: "high-contrast",
        name: "Alto contraste",
        description: "Máxima legibilidad y contraste.",
        css_class: "theme-high-contrast",
        is_dark: true,
        preview: ["#000000", "#111111", "#00aaff", "#00ff88"],
    },
    ThemeDef {
        id: "commit",
        name: "Estilo commit",
        description: "Inspirado en terminal/git, con acentos verdes.",
        css_class: "theme-commit",
        is_dark: true,
        preview: ["#0d1117", "#161b22", "#238636", "#58a6ff"],
    },
];

pub fn find(id: &str) -> Option<&'static ThemeDef> {
    let normalized = normalize_id(id);
    THEMES.iter().find(|theme| theme.id == normalized)
}

pub fn normalize_id(id: &str) -> &str {
    match id.trim() {
        "light" => "light",
        "oled" | "oled-black" => "oled",
        "sepia" => "sepia",
        "high-contrast" | "high_contrast" | "contrast" => "high-contrast",
        "commit" | "terminal" | "git" => "commit",
        _ => "dark",
    }
}

pub fn default_theme() -> &'static ThemeDef {
    &THEMES[0]
}

pub fn resolve(id: &str) -> &'static ThemeDef {
    find(id).unwrap_or_else(default_theme)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_themes_and_aliases() {
        assert_eq!(resolve("dark").id, "dark");
        assert_eq!(resolve("light").id, "light");
        assert_eq!(resolve("oled-black").id, "oled");
        assert_eq!(resolve("sepia").id, "sepia");
        assert_eq!(resolve("high_contrast").id, "high-contrast");
        assert_eq!(resolve("commit").id, "commit");
    }

    #[test]
    fn falls_back_to_default_for_unknown() {
        assert_eq!(resolve("unknown").id, "dark");
        assert_eq!(resolve("").id, "dark");
    }
}
