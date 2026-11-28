//! Importador de packs `.zip` de NitroPlay (formato del Mega Pack 24 Temas).
//!
//! Estructura aceptada dentro del zip:
//! - `catalog.json` en la raiz (indice; solo informativo, no se instala).
//! - `temas/<id>/theme.json` (obligatorio la primera vez), `temas/<id>/frame.png` (obligatorio la primera vez),
//!   `temas/<id>/wallpaper.{png,jpg,jpeg,webp}` (opcional).
//!
//! Todo lo demas (`originales/`, `README.txt`, rutas raras) se ignora o se rechaza. El codigo no depende de la consola:
//! el mismo modulo sirve para Windows, Android y Meta Quest.

use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io::{Read, Seek},
    path::Path,
};

use crate::theme_pack::valid_pack_id;

/// Limites para no dejarse engañar por un zip malicioso (bombas de descompresion, miles de entradas).
const MAX_JSON_BYTES: u64 = 64 * 1024;
const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ENTRIES: usize = 2000;

#[derive(Debug, Deserialize)]
pub struct Catalog {
    pub pack: String,
    pub version: String,
    #[serde(default)]
    pub count: usize,
    pub themes: Vec<CatalogEntry>,
}

#[derive(Debug, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub system: String,
    #[serde(default)]
    pub accent_color: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct RectPx {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// `theme.json` tal cual lo escribe el pack. Los campos extra se ignoran.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeJson {
    pub id: String,
    pub name: String,
    pub system: String,
    #[serde(default)]
    pub frame: Option<String>,
    #[serde(default)]
    pub accent_color: Option<String>,
    #[serde(default)]
    pub accent_rgb: Option<[u8; 3]>,
    #[serde(default)]
    pub viewport_rect: Option<RectF>,
    #[serde(default)]
    pub viewport_px: Option<RectPx>,
    #[serde(default)]
    pub resolution: Option<[u32; 2]>,
}

/// Tema ya validado: lo que la interfaz puede usar sin volver a comprobar nada.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeMeta {
    pub id: String,
    pub name: String,
    pub system: String,
    pub accent_rgb: Option<[u8; 3]>,
    /// Rectangulo del video en fracciones 0..1 del marco. `None` = el video ocupa todo.
    pub viewport: Option<RectF>,
}

#[derive(Debug, Default, PartialEq)]
pub struct ImportReport {
    /// Temas nuevos instalados.
    pub imported: Vec<String>,
    /// Temas que ya existian y a los que se ha añadido un fondo.
    pub merged_wallpaper: Vec<String>,
    /// (id o ruta, motivo) de lo que no se ha instalado.
    pub skipped: Vec<(String, String)>,
}

pub fn parse_catalog(text: &str) -> Result<Catalog, String> {
    serde_json::from_str(text).map_err(|e| format!("catalog.json: {e}"))
}

fn parse_hex(hex: &str) -> Option<[u8; 3]> {
    if hex.len() != 7 || !hex.starts_with('#') || !hex[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some([
        u8::from_str_radix(&hex[1..3], 16).ok()?,
        u8::from_str_radix(&hex[3..5], 16).ok()?,
        u8::from_str_radix(&hex[5..7], 16).ok()?,
    ])
}

/// Valida un `theme.json`. Rechaza datos inconsistentes en vez de adivinarlos.
pub fn validate_theme(t: &ThemeJson) -> Result<ThemeMeta, String> {
    if !valid_pack_id(&t.id) {
        return Err(format!("id invalido: {}", t.id));
    }
    let system = t.system.to_ascii_lowercase();
    if system.is_empty() || system.len() > 12 || !system.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("system invalido".into());
    }
    let name: String = t.name.chars().filter(|c| c.is_alphanumeric() || " .-_()'".contains(*c)).take(60).collect();
    if name.trim().is_empty() {
        return Err("name vacio".into());
    }

    let mut accent_rgb = t.accent_rgb;
    if let Some(hex) = &t.accent_color {
        let rgb = parse_hex(hex).ok_or_else(|| format!("accent_color invalido: {hex}"))?;
        if let Some(declared) = t.accent_rgb {
            if declared != rgb {
                return Err("accent_rgb no coincide con accent_color".into());
            }
        }
        accent_rgb = Some(rgb);
    }

    let viewport = match t.viewport_rect {
        None => None,
        Some(r) => {
            let finite = [r.x, r.y, r.w, r.h].iter().all(|v| v.is_finite());
            let inside = r.x >= 0.0 && r.y >= 0.0 && r.w > 0.0 && r.h > 0.0 && r.x + r.w <= 1.0 + 1e-4 && r.y + r.h <= 1.0 + 1e-4;
            if !finite || !inside {
                return Err("viewport_rect fuera de 0..1".into());
            }
            Some(r)
        }
    };

    if let Some(res) = t.resolution {
        if res[0] == 0 || res[1] == 0 {
            return Err("resolution invalida".into());
        }
    }
    if let (Some(r), Some(px), Some(res)) = (t.viewport_rect, t.viewport_px, t.resolution) {
        let (rw, rh) = (res[0] as f32, res[1] as f32);
        let close = |a: f32, b: f32| (a - b).abs() <= 2.0;
        let ok = close(r.x * rw, px.x as f32) && close(r.y * rh, px.y as f32) && close(r.w * rw, px.w as f32) && close(r.h * rh, px.h as f32);
        if !ok {
            return Err("viewport_px no coincide con viewport_rect y resolution".into());
        }
    }

    Ok(ThemeMeta { id: t.id.clone(), name, system, accent_rgb, viewport })
}

fn is_wallpaper(name: &str) -> bool {
    match name.rsplit_once('.') {
        Some(("wallpaper", ext)) => matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp"),
        _ => false,
    }
}

fn extract<R: Read + Seek>(zip: &mut zip::ZipArchive<R>, idx: usize, max: u64) -> Result<Vec<u8>, String> {
    let f = zip.by_index(idx).map_err(|e| e.to_string())?;
    let name = f.name().to_string();
    if f.size() > max {
        return Err(format!("{name} supera el limite de tamaño"));
    }
    let mut buf = Vec::new();
    f.take(max + 1).read_to_end(&mut buf).map_err(|e| format!("{name}: {e}"))?;
    if buf.len() as u64 > max {
        return Err(format!("{name} supera el limite de tamaño"));
    }
    Ok(buf)
}

enum Installed {
    New,
    Merged,
}

fn install_theme<R: Read + Seek>(zip: &mut zip::ZipArchive<R>, id: &str, entries: &[(String, usize)], dest: &Path) -> Result<Installed, String> {
    if !valid_pack_id(id) {
        return Err("id de carpeta invalido".into());
    }
    let find = |want: &dyn Fn(&str) -> bool| entries.iter().find(|(n, _)| want(n)).map(|(n, i)| (n.clone(), *i));
    let json = find(&|n| n == "theme.json");
    let frame = find(&|n| n == "frame.png");
    let wall = find(&is_wallpaper);

    let theme_dir = dest.join("temas").join(id);
    let existed = theme_dir.join("theme.json").is_file();
    if json.is_none() && !existed {
        return Err("falta theme.json".into());
    }
    if frame.is_none() && !existed {
        return Err("falta frame.png".into());
    }

    // Validar todo antes de escribir nada: un tema a medias es peor que un tema descartado.
    let theme_json = match &json {
        Some((_, idx)) => {
            let bytes = extract(zip, *idx, MAX_JSON_BYTES)?;
            let text = String::from_utf8(bytes).map_err(|_| "theme.json no es UTF-8".to_string())?;
            let parsed: ThemeJson = serde_json::from_str(&text).map_err(|e| format!("theme.json: {e}"))?;
            if parsed.id != id {
                return Err(format!("theme.json dice id '{}' pero la carpeta es '{id}'", parsed.id));
            }
            validate_theme(&parsed)?;
            Some(text)
        }
        None => None,
    };
    let frame_bytes = match &frame {
        Some((_, idx)) => Some(extract(zip, *idx, MAX_IMAGE_BYTES)?),
        None => None,
    };
    let wall_bytes = match &wall {
        Some((name, idx)) => Some((name.clone(), extract(zip, *idx, MAX_IMAGE_BYTES)?)),
        None => None,
    };

    std::fs::create_dir_all(&theme_dir).map_err(|e| e.to_string())?;
    if let Some(text) = theme_json {
        std::fs::write(theme_dir.join("theme.json"), text).map_err(|e| e.to_string())?;
    }
    if let Some(bytes) = frame_bytes {
        std::fs::write(theme_dir.join("frame.png"), bytes).map_err(|e| e.to_string())?;
    }
    if let Some((name, bytes)) = wall_bytes {
        let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
        // Un fondo nuevo sustituye al anterior: quitamos cualquier wallpaper.* previo para no dejar dos.
        if let Ok(rd) = std::fs::read_dir(&theme_dir) {
            for e in rd.flatten() {
                if is_wallpaper(&e.file_name().to_string_lossy()) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        std::fs::write(theme_dir.join(format!("wallpaper.{ext}")), bytes).map_err(|e| e.to_string())?;
    }
    Ok(if existed { Installed::Merged } else { Installed::New })
}

/// Instala los temas de un `.zip` en `dest` (normalmente `%APPDATA%\NitroPlay\marcos`).
/// Un tema con error se descarta entero y se anota en `skipped`; los demas siguen.
pub fn import_zip<R: Read + Seek>(reader: R, dest: &Path) -> Result<ImportReport, String> {
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| format!("zip no valido: {e}"))?;
    if zip.len() > MAX_ENTRIES {
        return Err(format!("el zip tiene demasiadas entradas ({})", zip.len()));
    }
    let mut report = ImportReport::default();
    let mut themes: BTreeMap<String, Vec<(String, usize)>> = BTreeMap::new();
    for i in 0..zip.len() {
        let f = zip.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let Some(path) = f.enclosed_name() else {
            report.skipped.push((f.name().to_string(), "ruta no segura".into()));
            continue;
        };
        let parts: Vec<String> = path.iter().map(|p| p.to_string_lossy().into_owned()).collect();
        // Solo temas/<id>/<archivo>. `originales/`, `README.txt` y `catalog.json` no se instalan.
        if parts.len() != 3 || parts[0] != "temas" {
            continue;
        }
        themes.entry(parts[1].clone()).or_default().push((parts[2].clone(), i));
    }
    if themes.is_empty() {
        return Err("el zip no contiene temas en temas/<id>/".into());
    }
    for (id, entries) in themes {
        match install_theme(&mut zip, &id, &entries, dest) {
            Ok(Installed::New) => report.imported.push(id),
            Ok(Installed::Merged) => report.merged_wallpaper.push(id),
            Err(motivo) => report.skipped.push((id, motivo)),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    /// Mismo `theme.json` que trae el Mega Pack para god-of-war.
    const GOW: &str = r##"{
  "id": "god-of-war", "name": "God of War", "system": "ps5", "frame": "frame.png",
  "accent_color": "#DC3E2A", "accent_rgb": [220, 62, 42],
  "viewport_rect": { "x": 0.165, "y": 0.1471, "w": 0.6701, "h": 0.6979 },
  "viewport_px": { "x": 227, "y": 113, "w": 922, "h": 536 },
  "resolution": [1376, 768]
}"##;

    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in entries {
            w.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    fn tmp(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("np-pack-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn theme_del_megapack_es_valido() {
        let t: ThemeJson = serde_json::from_str(GOW).unwrap();
        let m = validate_theme(&t).unwrap();
        assert_eq!(m.name, "God of War");
        assert_eq!(m.accent_rgb, Some([220, 62, 42]));
        let v = m.viewport.unwrap();
        assert!((v.x - 0.165).abs() < 1e-6 && (v.w - 0.6701).abs() < 1e-6);
    }

    #[test]
    fn catalogo_del_megapack() {
        let c = parse_catalog(r##"{"pack":"NitroPlay Mega Pack 24 Temas","version":"1.0","count":1,"themes":[{"id":"god-of-war","name":"God of War","system":"ps5","accent_color":"#DC3E2A","path":"temas/god-of-war"}]}"##).unwrap();
        assert_eq!(c.count, 1);
        assert_eq!(c.themes[0].path, "temas/god-of-war");
        assert!(parse_catalog("{no es json").is_err());
    }

    #[test]
    fn rechaza_viewport_y_color_incoherentes() {
        let fuera = GOW.replace("\"w\": 0.6701", "\"w\": 0.9");
        let t: ThemeJson = serde_json::from_str(&fuera).unwrap();
        assert!(validate_theme(&t).is_err());
        let color = GOW.replace("\"accent_rgb\": [220, 62, 42]", "\"accent_rgb\": [1, 2, 3]");
        let t: ThemeJson = serde_json::from_str(&color).unwrap();
        assert!(validate_theme(&t).unwrap_err().contains("accent_rgb"));
        let px = GOW.replace("\"x\": 227", "\"x\": 400");
        let t: ThemeJson = serde_json::from_str(&px).unwrap();
        assert!(validate_theme(&t).unwrap_err().contains("viewport_px"));
    }

    #[test]
    fn importa_tema_y_descarta_originales_y_readme() {
        let zip = make_zip(&[
            ("catalog.json", b"{}"),
            ("README.txt", b"hola"),
            ("originales/god-of-war.png", b"pesado"),
            ("temas/god-of-war/theme.json", GOW.as_bytes()),
            ("temas/god-of-war/frame.png", b"marco"),
        ]);
        let dest = tmp("importa");
        let r = import_zip(Cursor::new(zip), &dest).unwrap();
        assert_eq!(r.imported, vec!["god-of-war".to_string()]);
        assert!(r.skipped.is_empty());
        assert!(dest.join("temas/god-of-war/frame.png").is_file());
        assert!(!dest.join("originales").exists());
        assert!(!dest.join("README.txt").exists());
    }

    #[test]
    fn fusiona_fondo_en_tema_existente() {
        let dest = tmp("fusiona");
        let primero = make_zip(&[("temas/god-of-war/theme.json", GOW.as_bytes()), ("temas/god-of-war/frame.png", b"marco")]);
        import_zip(Cursor::new(primero), &dest).unwrap();
        let fondo = make_zip(&[("temas/god-of-war/wallpaper.jpg", b"fondo")]);
        let r = import_zip(Cursor::new(fondo), &dest).unwrap();
        assert_eq!(r.merged_wallpaper, vec!["god-of-war".to_string()]);
        assert!(r.imported.is_empty());
        assert!(dest.join("temas/god-of-war/wallpaper.jpg").is_file());
        assert!(dest.join("temas/god-of-war/theme.json").is_file());
    }

    #[test]
    fn tema_invalido_no_deja_archivos_a_medias() {
        let malo = GOW.replace("\"w\": 0.6701", "\"w\": 0.9");
        let zip = make_zip(&[("temas/god-of-war/theme.json", malo.as_bytes()), ("temas/god-of-war/frame.png", b"marco")]);
        let dest = tmp("malo");
        let r = import_zip(Cursor::new(zip), &dest).unwrap();
        assert!(r.imported.is_empty());
        assert_eq!(r.skipped.len(), 1);
        assert!(!dest.join("temas/god-of-war").exists());
    }

    #[test]
    fn rutas_peligrosas_se_rechazan() {
        let zip = make_zip(&[("temas/../../evil.txt", b"x"), ("temas/god-of-war/theme.json", GOW.as_bytes()), ("temas/god-of-war/frame.png", b"m")]);
        let dest = tmp("slip");
        let r = import_zip(Cursor::new(zip), &dest).unwrap();
        assert!(r.skipped.iter().any(|(n, _)| n.contains("evil")));
        assert!(!dest.parent().unwrap().join("evil.txt").exists());
        assert_eq!(r.imported, vec!["god-of-war".to_string()]);
    }

    #[test]
    fn id_de_carpeta_y_json_deben_coincidir() {
        let otro = GOW.replace("\"id\": \"god-of-war\"", "\"id\": \"otro\"");
        let zip = make_zip(&[("temas/god-of-war/theme.json", otro.as_bytes()), ("temas/god-of-war/frame.png", b"m")]);
        let dest = tmp("id");
        let r = import_zip(Cursor::new(zip), &dest).unwrap();
        assert!(r.imported.is_empty());
        assert!(r.skipped[0].1.contains("theme.json dice id"));
    }

    #[test]
    fn zip_sin_temas_es_error() {
        let zip = make_zip(&[("catalog.json", b"{}")]);
        assert!(import_zip(Cursor::new(zip), &tmp("vacio")).is_err());
    }
}
