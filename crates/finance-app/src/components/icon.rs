use std::collections::HashMap;
use std::sync::OnceLock;

use dioxus::prelude::*;

const SPRITE: &str = include_str!("../../assets/financeVZ-pack.svg");

fn bodies() -> &'static HashMap<&'static str, &'static str> {
    static BODIES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    BODIES.get_or_init(|| {
        let mut icons = HashMap::new();
        let mut rest = SPRITE;
        const OPEN: &str = "<symbol id='";
        while let Some(index) = rest.find(OPEN) {
            rest = &rest[index + OPEN.len()..];
            let Some(quote) = rest.find('\'') else { break };
            let id = &rest[..quote];
            rest = &rest[quote..];
            let Some(tag_end) = rest.find('>') else { break };
            rest = &rest[tag_end + 1..];
            let Some(close) = rest.find("</symbol>") else {
                break;
            };
            icons.insert(id, &rest[..close]);
            rest = &rest[close + "</symbol>".len()..];
        }
        icons
    })
}

pub fn icon_for(value: &str) -> &'static str {
    let value = value.trim();
    if let Some((name, _)) = bodies().get_key_value(value) {
        return name;
    }
    match value {
        "🍔" => "food",
        "🚗" => "transport",
        "🏠" | "🏡" => "home",
        "🎬" => "entertainment",
        "💊" => "health",
        "💼" => "work",
        "🛒" => "shopping",
        "🎁" => "gift",
        "📷" => "camera",
        "📎" => "paperclip",
        "✕" | "❌" => "close",
        "🔁" => "repeat",
        "📅" => "calendar",
        "💵" => "income",
        "📊" => "reports",
        "🧾" => "payments",
        "📦" => "other",
        v if v.starts_with("🧑") => "tech",
        v if v.starts_with("🏷") => "categories",
        v if v.starts_with("⬆") => "update",
        _ => "other",
    }
}

pub fn icon_for_category(name: &str, icon: Option<&str>) -> &'static str {
    let by_name = match name.trim().to_ascii_lowercase().as_str() {
        "comida" | "alimentos" => Some("food"),
        "transporte" => Some("transport"),
        "hogar" => Some("house"),
        "ocio" | "entretenimiento" => Some("entertainment"),
        "salud" => Some("health"),
        "familia" | "familiares" => Some("house"),
        "sueldo" | "salario" | "trabajo" => Some("work"),
        "freelance" => Some("tech"),
        "ventas" => Some("shopping"),
        "regalos" => Some("gift"),
        "ingresos" | "ingreso" => Some("income"),
        _ => None,
    };
    if let Some(icon) = icon {
        let resolved = icon_for(icon);
        if resolved != "other" {
            return resolved;
        }
    }
    by_name.unwrap_or("other")
}

#[component]
pub fn Icon(name: &'static str, #[props(default = "icon")] class: &'static str) -> Element {
    let body = bodies().get(name).copied().unwrap_or_default();
    rsx! {
        svg {
            class: class,
            view_box: "0 0 24 24",
            dangerous_inner_html: "{body}",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_contiene_los_iconos_de_la_app() {
        let icons = bodies();
        for name in [
            "home",
            "reports",
            "categories",
            "payments",
            "update",
            "camera",
            "cashea",
        ] {
            assert!(icons.contains_key(name), "falta el icono {name}");
            assert!(
                icons[name].contains("<path") || icons[name].contains("<rect"),
                "el icono {name} está vacío"
            );
        }
    }

    #[test]
    fn emojis_y_categorias_se_resuelven_a_iconos() {
        assert_eq!(icon_for("🏷️"), "categories");
        assert_eq!(icon_for("🧑‍💻"), "tech");
        assert_eq!(icon_for("📦"), "other");
        assert_eq!(icon_for_category("Comida", Some("🍔")), "food");
        assert_eq!(icon_for_category("Mascotas", None), "other");
    }
}
