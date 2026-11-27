//! Packs de marcos/temas del usuario: carpeta con `frame.png` (PNG con transparencia) y un
//! `theme.json` opcional. El formato no depende de la consola: `system` puede ser `ps5`,
//! `ps4`, `switch`... y asi el mismo sistema sirve para otras consolas en el futuro.
//!
//! Ejemplo de `theme.json`:
//! `{"name":"Mi marco","system":"ps5","viewport_rect":{"x":6,"y":8,"w":88,"h":84}}`
//! (el rectangulo del video va en % del marco; si falta, el video ocupa todo).

#[derive(Debug, Clone, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Viewport {
    pub const FULL: Viewport = Viewport { x: 0.0, y: 0.0, w: 100.0, h: 100.0 };
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackMeta {
    pub name: String,
    pub system: String,
    pub viewport: Viewport,
}

/// El id de un pack es el nombre de su carpeta: solo letras, numeros, `-` y `_`.
pub fn valid_pack_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 40 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Texto visible seguro (se inyecta en la interfaz): sin comillas, barras ni `<`.
fn clean_text(s: &str, max: usize) -> String {
    s.chars().filter(|c| c.is_alphanumeric() || " .-_()".contains(*c)).take(max).collect()
}

/// Valor crudo de `"key": valor` (cadena sin escapes o numero). Sin dependencias de JSON.
fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\"");
    let after = &text[text.find(&pat)? + pat.len()..];
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    if let Some(rest) = after.strip_prefix('"') {
        let end = rest.find('"')?;
        let v = &rest[..end];
        return if v.contains('\\') { None } else { Some(v) };
    }
    let end = after.find(|c: char| c == ',' || c == '}' || c.is_whitespace()).unwrap_or(after.len());
    Some(&after[..end])
}

fn num(text: &str, key: &str) -> Option<f32> {
    field(text, key)?.parse::<f32>().ok().filter(|n| n.is_finite())
}

/// Lee `theme.json`. Si el rectangulo no es valido, el video ocupa todo el marco.
pub fn parse_theme_json(text: &str, fallback_name: &str) -> PackMeta {
    let name = field(text, "name").map(|n| clean_text(n, 40)).filter(|n| !n.is_empty()).unwrap_or_else(|| clean_text(fallback_name, 40));
    let system = field(text, "system")
        .map(|s| s.to_ascii_lowercase())
        .filter(|s| !s.is_empty() && s.len() <= 12 && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or_else(|| "ps5".into());
    // Acepta porcentajes (0-100) o fracciones (0-1): en fracciones el ancho nunca llega a 5.
    let scale = |v: Option<f32>, w: Option<f32>| match (v, w) {
        (Some(v), Some(w)) if w <= 1.0 => Some(v * 100.0),
        (v, _) => v,
    };
    let w_raw = num(text, "w");
    let (x, y, w, h) = (scale(num(text, "x"), w_raw), scale(num(text, "y"), w_raw), scale(w_raw, w_raw), scale(num(text, "h"), w_raw));
    let viewport = match (x, y, w, h) {
        (Some(x), Some(y), Some(w), Some(h)) if x >= 0.0 && y >= 0.0 && w >= 5.0 && h >= 5.0 && x + w <= 100.0 && y + h <= 100.0 => Viewport { x, y, w, h },
        _ => Viewport::FULL,
    };
    PackMeta { name, system, viewport }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_nombre_consola_y_rectangulo() {
        let m = parse_theme_json(r#"{"name":"Kratos Marco","system":"PS5","viewport_rect":{"x":6,"y":8.5,"w":88,"h":84}}"#, "carpeta");
        assert_eq!(m.name, "Kratos Marco");
        assert_eq!(m.system, "ps5");
        assert_eq!(m.viewport, Viewport { x: 6.0, y: 8.5, w: 88.0, h: 84.0 });
    }

    #[test]
    fn sin_json_usa_la_carpeta_y_video_completo() {
        let m = parse_theme_json("", "horizon-zero");
        assert_eq!(m, PackMeta { name: "horizon-zero".into(), system: "ps5".into(), viewport: Viewport::FULL });
    }

    #[test]
    fn rectangulo_fuera_de_rango_se_ignora() {
        for t in [r#"{"x":50,"y":0,"w":80,"h":50}"#, r#"{"x":-1,"y":0,"w":50,"h":50}"#, r#"{"x":0,"y":0,"w":2,"h":50}"#, r#"{"x":"a","y":0,"w":50,"h":50}"#] {
            assert_eq!(parse_theme_json(t, "p").viewport, Viewport::FULL, "{t}");
        }
    }

    #[test]
    fn el_texto_se_sanea() {
        let m = parse_theme_json(r#"{"name":"<b>Hola</b>\"x"}"#, "p");
        assert_eq!(m.name, "p"); // contiene barra invertida: se rechaza
        let m = parse_theme_json(r#"{"name":"<img src=x>Ok"}"#, "p");
        assert_eq!(m.name, "img srcxOk");
    }

    #[test]
    fn ids_de_pack() {
        assert!(valid_pack_id("kratos_1-b"));
        for bad in ["", "../x", "a b", "a/b", &"x".repeat(41)] {
            assert!(!valid_pack_id(bad), "{bad}");
        }
    }

    #[test]
    fn acepta_fracciones() {
        let t = r#"{"id":"astro-bot","name":"Astro Bot","system":"ps5","frame":"frame.png","viewport_rect":{"x":0.0945,"y":0.1081,"w":0.811,"h":0.793},"viewport_px":{"x":130,"y":83,"w":1116,"h":609}}"#;
        let m = parse_theme_json(t, "p");
        assert_eq!(m.name, "Astro Bot");
        let v = m.viewport;
        assert!((v.x - 9.45).abs() < 0.01 && (v.y - 10.81).abs() < 0.01 && (v.w - 81.1).abs() < 0.01 && (v.h - 79.3).abs() < 0.01, "{v:?}");
    }

    #[test]
    fn otra_consola() {
        assert_eq!(parse_theme_json(r#"{"system":"switch"}"#, "p").system, "switch");
    }
}
