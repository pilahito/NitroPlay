//! Proveedores de fondos: Alpha Coders, PlayStation Oficial, GameWallpapers (solo los libres) y carpeta local.
//! La descarga va por `net::Http` y el analisis del HTML es puro: asi se prueba con fixtures, sin red.
//!
//! AVISO: la estructura HTML de cada sitio NO esta verificada en vivo. Los parsers siguen el patron de
//! `<img>` + `<a href>` + atributos `data-*`. Antes de dar la funcion por buena hay que comprobarla contra
//! las paginas reales (ver docs/STATE.md).

use crate::net::{self, sha256_hex, url_encode, Http, HttpError};
use std::path::{Path, PathBuf};

/// Limite de una imagen de fondo descargada (16 MB).
pub const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallpaperItem {
    pub title: String,
    /// Imagen a tamano completo (URL https o ruta local).
    pub image_url: String,
    pub thumb_url: String,
    /// 0 si el sitio no informa de las dimensiones.
    pub width: u32,
    pub height: u32,
    pub page_url: String,
    pub source: &'static str,
}

/// Destino de pantalla: en Windows horizontales >= 1920x1080; en Android verticales.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Desktop,
    Mobile,
}

pub fn fits_target(item: &WallpaperItem, target: Target) -> bool {
    match target {
        Target::Desktop => item.width >= 1920 && item.height >= 1080 && item.width > item.height,
        Target::Mobile => item.height > item.width,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// El sitio bloquea el acceso automatizado (403/429/503 o pagina de comprobacion).
    Blocked,
    Network(String),
    BadData(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::Blocked => write!(f, "El sitio bloquea las búsquedas automáticas. Abre la galería en el navegador."),
            ProviderError::Network(m) => write!(f, "Error de red: {m}"),
            ProviderError::BadData(m) => write!(f, "Respuesta no válida: {m}"),
        }
    }
}

fn map_http(e: HttpError) -> ProviderError {
    match e {
        HttpError::Status(403) | HttpError::Status(429) | HttpError::Status(503) => ProviderError::Blocked,
        HttpError::Status(c) => ProviderError::Network(format!("HTTP {c}")),
        HttpError::Network(m) => ProviderError::Network(m),
    }
}

fn looks_blocked(html: &str) -> bool {
    let h = html.to_lowercase();
    h.contains("cf-browser-verification") || h.contains("just a moment...") || h.contains("attention required! | cloudflare")
}

pub trait WallpaperProvider {
    fn id(&self) -> &'static str;
    fn source_name(&self) -> &'static str;
    /// Pagina que se abre en el navegador (galeria completa).
    fn gallery_url(&self) -> &'static str;
    fn search_url(&self, query: &str) -> String;
    /// Analisis puro del HTML de resultados.
    fn parse(&self, html: &str, query: &str) -> Vec<WallpaperItem>;

    fn search(&self, http: &dyn Http, query: &str, target: Target) -> Result<Vec<WallpaperItem>, ProviderError> {
        let html = http.get_text(&self.search_url(query)).map_err(map_http)?;
        if looks_blocked(&html) {
            return Err(ProviderError::Blocked);
        }
        Ok(self.parse(&html, query).into_iter().filter(|i| fits_target(i, target)).collect())
    }
}

pub struct AlphaCoders;

impl WallpaperProvider for AlphaCoders {
    fn id(&self) -> &'static str {
        "alphacoders"
    }
    fn source_name(&self) -> &'static str {
        "Alpha Coders"
    }
    fn gallery_url(&self) -> &'static str {
        "https://wall.alphacoders.com/search.php?search=playstation"
    }
    fn search_url(&self, query: &str) -> String {
        let q = if query.trim().is_empty() { "playstation".to_string() } else { format!("{} playstation", query.trim()) };
        format!("https://wall.alphacoders.com/search.php?search={}", url_encode(&q))
    }
    fn parse(&self, html: &str, _query: &str) -> Vec<WallpaperItem> {
        to_items(html, self.source_name(), false)
    }
}

pub struct PlayStationOficial;

impl WallpaperProvider for PlayStationOficial {
    fn id(&self) -> &'static str {
        "psoficial"
    }
    fn source_name(&self) -> &'static str {
        "PlayStation Oficial"
    }
    fn gallery_url(&self) -> &'static str {
        "https://www.playstation.com/es-es/wallpapers/"
    }
    fn search_url(&self, _query: &str) -> String {
        self.gallery_url().to_string()
    }
    fn parse(&self, html: &str, query: &str) -> Vec<WallpaperItem> {
        to_items(html, self.source_name(), true).into_iter().filter(|i| title_matches(&i.title, query)).collect()
    }
}

pub struct GameWallpapers;

impl WallpaperProvider for GameWallpapers {
    fn id(&self) -> &'static str {
        "gamewallpapers"
    }
    fn source_name(&self) -> &'static str {
        "GameWallpapers"
    }
    fn gallery_url(&self) -> &'static str {
        "https://gamewallpapers.com/"
    }
    fn search_url(&self, query: &str) -> String {
        format!("https://gamewallpapers.com/?s={}", url_encode(query.trim()))
    }
    fn parse(&self, html: &str, query: &str) -> Vec<WallpaperItem> {
        to_items(html, self.source_name(), true).into_iter().filter(|i| title_matches(&i.title, query)).collect()
    }
}

/// Proveedor por id (`alphacoders`, `psoficial`, `gamewallpapers`).
pub fn provider_by_id(id: &str) -> Option<Box<dyn WallpaperProvider>> {
    match id {
        "alphacoders" => Some(Box::new(AlphaCoders)),
        "psoficial" => Some(Box::new(PlayStationOficial)),
        "gamewallpapers" => Some(Box::new(GameWallpapers)),
        _ => None,
    }
}

fn title_matches(title: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty() || title.to_lowercase().contains(&q)
}

// ---- analisis de tarjetas <img> ----

struct Card {
    page: String,
    image: String,
    thumb: String,
    title: String,
    width: u32,
    height: u32,
    members_only: bool,
}

/// Valor de un atributo. Busca ` nombre="` con espacio delante, para no confundir `src` con `data-src`.
fn attr(tag: &str, name: &str) -> Option<String> {
    let pat = format!(" {name}=\"");
    let start = tag.find(&pat)? + pat.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].replace("&amp;", "&"))
}

fn is_members_only(block: &str) -> bool {
    let b = block.to_lowercase();
    b.contains("sólo miembros") || b.contains("solo miembros")
}

fn scan_cards(html: &str) -> Vec<Card> {
    let positions: Vec<usize> = html.match_indices("<img").map(|(i, _)| i).collect();
    let mut cards = Vec::new();
    for (k, &p) in positions.iter().enumerate() {
        let Some(e) = html[p..].find('>') else { continue };
        let tag = &html[p..p + e + 1];
        let next = positions.get(k + 1).copied().unwrap_or(html.len());
        let block = &html[p..next];
        let src = attr(tag, "src");
        let data_src = attr(tag, "data-src");
        let image = attr(tag, "data-full")
            .or_else(|| attr(tag, "data-original"))
            .or_else(|| data_src.clone())
            .or_else(|| src.clone())
            .unwrap_or_default();
        let thumb = src.or(data_src).unwrap_or_default();
        let mut a = p.saturating_sub(400);
        while !html.is_char_boundary(a) {
            a += 1;
        }
        let before = &html[a..p];
        let page = before
            .rfind("href=\"")
            .and_then(|h| {
                let s = h + 6;
                before[s..].find('"').map(|e| before[s..s + e].replace("&amp;", "&"))
            })
            .unwrap_or_default();
        cards.push(Card {
            page,
            image,
            thumb,
            title: attr(tag, "alt").unwrap_or_default().trim().to_string(),
            width: attr(tag, "data-width").and_then(|v| v.parse().ok()).unwrap_or(0),
            height: attr(tag, "data-height").and_then(|v| v.parse().ok()).unwrap_or(0),
            members_only: is_members_only(block),
        });
    }
    cards
}

fn is_web_url(s: &str) -> bool {
    (s.starts_with("https://") || s.starts_with("http://")) && !s.contains(char::is_whitespace) && !s.contains('"')
}

/// Convierte las tarjetas en fondos. Descarta las de miembros, las que no tienen imagen web valida y las sin titulo.
fn to_items(html: &str, source: &'static str, _title_filtered: bool) -> Vec<WallpaperItem> {
    scan_cards(html)
        .into_iter()
        .filter(|c| !c.members_only && is_web_url(&c.image) && !c.title.is_empty())
        .map(|c| {
            let thumb = if is_web_url(&c.thumb) { c.thumb } else { c.image.clone() };
            WallpaperItem {
                title: c.title,
                image_url: c.image,
                thumb_url: thumb,
                width: c.width,
                height: c.height,
                page_url: c.page,
                source,
            }
        })
        .collect()
}

// ---- cache ----

/// Ruta de cache: hash SHA-256 de la URL, con la extension del fichero (por defecto jpg).
pub fn cache_path(cache_dir: &Path, url: &str) -> PathBuf {
    let path_part = url.split(|c| c == '?' || c == '#').next().unwrap_or(url);
    let ext = match path_part.rsplit('.').next().map(|e| e.to_ascii_lowercase()) {
        Some(e) if matches!(e.as_str(), "jpg" | "jpeg" | "png" | "webp") => e,
        _ => "jpg".to_string(),
    };
    cache_dir.join(format!("{}.{ext}", sha256_hex(url.as_bytes())))
}

/// Comprueba la firma de la imagen (JPEG, PNG, WebP) para no guardar paginas de error como fondo.
pub fn is_image(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(&[0x89, b'P', b'N', b'G'])
        || (bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
}

/// Devuelve la imagen desde la cache o la descarga una sola vez.
pub fn fetch_cached(http: &dyn Http, cache_dir: &Path, url: &str) -> Result<PathBuf, ProviderError> {
    if !is_web_url(url) {
        return Err(ProviderError::BadData("URL de imagen no válida".into()));
    }
    let path = cache_path(cache_dir, url);
    if path.is_file() {
        return Ok(path);
    }
    let bytes = http.get_bytes(url, MAX_IMAGE_BYTES).map_err(map_http)?;
    if !is_image(&bytes) {
        return Err(ProviderError::BadData("el recurso descargado no es una imagen".into()));
    }
    std::fs::create_dir_all(cache_dir).map_err(|e| ProviderError::BadData(e.to_string()))?;
    std::fs::write(&path, &bytes).map_err(|e| ProviderError::BadData(e.to_string()))?;
    Ok(path)
}

// ---- carpeta local ----

/// Imagenes de una carpeta local (jpg, jpeg, png, webp). `image_url` es la ruta del fichero.
pub fn list_local(dir: &Path) -> Vec<WallpaperItem> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut files: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| matches!(e.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp"))
                    .unwrap_or(false)
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| WallpaperItem {
            title: p.file_stem().and_then(|s| s.to_str()).unwrap_or("fondo").to_string(),
            image_url: p.to_string_lossy().into_owned(),
            thumb_url: String::new(),
            width: 0,
            height: 0,
            page_url: String::new(),
            source: "Carpeta local",
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::MockHttp;

    const ALPHA: &str = r#"<div class="thumb"><a href="/big.php?i=1234"><img class="img-responsive" src="https://img.example/t1.jpg" alt="PS5 Wallpaper 4K" data-width="3840" data-height="2160"></a></div>
      <div class="thumb"><a href="/big.php?i=99"><img src="https://img.example/t2.jpg" alt="Movil" data-width="1080" data-height="1920"></a></div>
      <div class="thumb"><a href="/big.php?i=7"><img src="https://img.example/t3.jpg" alt="Pequeno" data-width="800" data-height="600"></a></div>"#;

    const GAME: &str = r#"<a href="/wp/halo"><img data-src="https://gw.example/halo.jpg" alt="Halo" data-width="1920" data-height="1080"></a>
      <span>Sólo Miembros</span>
      <a href="/wp/zelda"><img src="https://gw.example/zelda.jpg" alt="Zelda" data-width="1920" data-height="1080"></a>"#;

    const PS: &str = r#"<a href="/es-es/wallpapers/astro"><img src="https://ps.example/astro.jpg" data-full="https://ps.example/astro-4k.jpg" alt="Astro Bot" data-width="3840" data-height="2160"></a>
      <a href="/es-es/wallpapers/gow"><img src="https://ps.example/gow.jpg" alt="God of War" data-width="3840" data-height="2160"></a>"#;

    #[test]
    fn alpha_coders_filtra_horizontales_hd() {
        let items = AlphaCoders.parse(ALPHA, "");
        assert_eq!(items.len(), 3);
        let desk: Vec<&str> = items.iter().filter(|i| fits_target(i, Target::Desktop)).map(|i| i.title.as_str()).collect();
        assert_eq!(desk, vec!["PS5 Wallpaper 4K"]);
        let movil: Vec<&str> = items.iter().filter(|i| fits_target(i, Target::Mobile)).map(|i| i.title.as_str()).collect();
        assert_eq!(movil, vec!["Movil"]);
        assert_eq!(items[0].page_url, "/big.php?i=1234");
        assert_eq!(items[0].source, "Alpha Coders");
    }

    #[test]
    fn game_wallpapers_descarta_los_de_miembros() {
        let items = GameWallpapers.parse(GAME, "");
        let titulos: Vec<&str> = items.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titulos, vec!["Zelda"]);
        assert_eq!(items[0].image_url, "https://gw.example/zelda.jpg");
    }

    #[test]
    fn playstation_prefiere_data_full_y_filtra_por_nombre() {
        let items = PlayStationOficial.parse(PS, "astro");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].image_url, "https://ps.example/astro-4k.jpg");
        assert_eq!(items[0].thumb_url, "https://ps.example/astro.jpg");
        assert_eq!(PlayStationOficial.parse(PS, "").len(), 2);
    }

    #[test]
    fn bloqueo_anti_bot_se_reporta() {
        let mut http = MockHttp::new();
        let url = AlphaCoders.search_url("halo");
        http.texts.insert(url.clone(), Err(HttpError::Status(403)));
        assert_eq!(AlphaCoders.search(&http, "halo", Target::Desktop), Err(ProviderError::Blocked));
        http.texts.insert(url, Ok("<title>Just a moment...</title>".into()));
        assert_eq!(AlphaCoders.search(&http, "halo", Target::Desktop), Err(ProviderError::Blocked));
    }

    #[test]
    fn busqueda_completa_con_http_simulado() {
        let mut http = MockHttp::new();
        http.texts.insert(AlphaCoders.search_url("ps5"), Ok(ALPHA.into()));
        let r = AlphaCoders.search(&http, "ps5", Target::Desktop).unwrap();
        assert_eq!(r.len(), 1);
        assert!(provider_by_id("alphacoders").is_some());
        assert!(provider_by_id("desconocido").is_none());
    }

    #[test]
    fn cache_descarga_una_sola_vez() {
        let dir = std::env::temp_dir().join(format!("np-wp-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut http = MockHttp::new();
        http.bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3];
        let url = "https://img.example/fondo.jpg?w=3840";
        let p1 = fetch_cached(&http, &dir, url).unwrap();
        let p2 = fetch_cached(&http, &dir, url).unwrap();
        assert_eq!(p1, p2);
        assert_eq!(http.calls.get(), 1);
        assert_eq!(p1.extension().unwrap(), "jpg");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rechaza_paginas_que_no_son_imagen() {
        let dir = std::env::temp_dir().join(format!("np-wp-bad-{}", std::process::id()));
        let mut http = MockHttp::new();
        http.bytes = b"<html>error</html>".to_vec();
        assert!(matches!(fetch_cached(&http, &dir, "https://x.example/a.png"), Err(ProviderError::BadData(_))));
        assert!(!dir.join(cache_path(&dir, "https://x.example/a.png").file_name().unwrap()).exists());
    }

    #[test]
    fn carpeta_local_solo_imagenes() {
        let dir = std::env::temp_dir().join(format!("np-wp-local-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("b.PNG"), b"x").unwrap();
        std::fs::write(dir.join("a.jpg"), b"x").unwrap();
        std::fs::write(dir.join("c.mp4"), b"x").unwrap();
        let items = list_local(&dir);
        let nombres: Vec<&str> = items.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(nombres, vec!["a", "b"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
