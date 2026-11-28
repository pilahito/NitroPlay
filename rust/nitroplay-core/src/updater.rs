//! Actualizador desde GitHub Releases de `pilahito/NitroPlay` y listado de Novedades.
//! Sin dependencias de interfaz: la GUI de Windows y, mas adelante, Android y Quest usan este mismo modulo.
//! La logica (parseo, SemVer, seleccion de asset, SHA-256, script) es pura y se prueba sin red.

use crate::net::{self, Http, HttpError};
use serde::Deserialize;
use std::{cmp::Ordering, path::Path};

pub const RELEASES_API: &str = "https://api.github.com/repos/pilahito/NitroPlay/releases?per_page=30";
/// Limite de descarga de un binario de release (120 MB).
pub const MAX_ASSET_BYTES: u64 = 120 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Android,
    Quest,
}

impl Platform {
    /// Patrones en orden de preferencia (`*` comodin, sin distinguir mayusculas).
    fn patterns(self) -> &'static [&'static str] {
        match self {
            Platform::Windows => &["*windows*x64*.exe", "*windows*x64*.zip"],
            Platform::Android => &["*android*arm64*.apk"],
            Platform::Quest => &["*quest*arm64*.apk"],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    /// Version sin la `v` inicial (`0.1.1`, `0.2.0-pre.1`).
    pub version: String,
    pub title: String,
    /// Fecha de publicacion `AAAA-MM-DD`.
    pub date: String,
    pub is_prerelease: bool,
    pub notes_markdown: String,
    pub html_url: String,
    pub assets: Vec<Asset>,
}

impl Release {
    /// Binario para una plataforma, o `None` si la release no lo trae (la GUI mostrara "Ver release en GitHub").
    pub fn asset_for(&self, platform: Platform) -> Option<&Asset> {
        platform.patterns().iter().find_map(|pat| self.assets.iter().find(|a| glob_match(pat, &a.name)))
    }

    /// `SHA256SUMS.txt` si la release lo incluye.
    pub fn checksums_asset(&self) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name.eq_ignore_ascii_case("SHA256SUMS.txt"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    Network(String),
    RateLimited,
    Http(u16),
    BadData(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Network(m) => write!(f, "Sin conexión con GitHub ({m})."),
            UpdateError::RateLimited => write!(f, "GitHub ha limitado las consultas. Prueba más tarde."),
            UpdateError::Http(c) => write!(f, "GitHub respondió con el error HTTP {c}."),
            UpdateError::BadData(m) => write!(f, "La respuesta de GitHub no es válida: {m}"),
        }
    }
}

impl From<HttpError> for UpdateError {
    fn from(e: HttpError) -> Self {
        match e {
            HttpError::Status(403) | HttpError::Status(429) => UpdateError::RateLimited,
            HttpError::Status(c) => UpdateError::Http(c),
            HttpError::Network(m) => UpdateError::Network(m),
        }
    }
}

// ---- JSON de la API de GitHub (solo los campos que usamos) ----
#[derive(Deserialize)]
struct GhAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    html_url: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

/// Parsea la lista de releases. Descarta borradores y, salvo `include_prerelease`, las pre-releases.
/// Devuelve las releases ordenadas de la mas nueva a la mas antigua segun SemVer.
pub fn parse_releases(json: &str, include_prerelease: bool) -> Result<Vec<Release>, UpdateError> {
    let raw: Vec<GhRelease> = serde_json::from_str(json).map_err(|e| UpdateError::BadData(e.to_string()))?;
    let mut out: Vec<Release> = raw
        .into_iter()
        .filter(|r| !r.draft && (include_prerelease || !r.prerelease))
        .filter(|r| parse_version(&r.tag_name).is_some())
        .map(|r| {
            let version = r.tag_name.trim_start_matches(|c| c == 'v' || c == 'V').to_string();
            Release {
                title: r.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| r.tag_name.clone()),
                version,
                date: r.published_at.as_deref().and_then(|d| d.get(..10)).unwrap_or("").to_string(),
                is_prerelease: r.prerelease,
                notes_markdown: r.body.unwrap_or_default(),
                html_url: r.html_url,
                assets: r
                    .assets
                    .into_iter()
                    .map(|a| Asset { name: a.name, download_url: a.browser_download_url, size: a.size })
                    .collect(),
            }
        })
        .collect();
    out.sort_by(|a, b| compare_versions(&b.version, &a.version).unwrap_or(Ordering::Equal));
    Ok(out)
}

/// Consulta la API de releases de GitHub.
pub fn fetch_releases(http: &dyn Http, include_prerelease: bool) -> Result<Vec<Release>, UpdateError> {
    let json = http.get_text(RELEASES_API)?;
    parse_releases(&json, include_prerelease)
}

pub struct UpdateStatus {
    pub current: String,
    /// Release mas nueva disponible (o la mas nueva publicada si ya estas al dia).
    pub latest: Option<Release>,
    pub update_available: bool,
    /// Historial completo para la lista de Novedades.
    pub news: Vec<Release>,
}

/// Comprueba si hay una version mas nueva que `current` (version de `CARGO_PKG_VERSION`).
pub fn check(http: &dyn Http, current: &str, include_prerelease: bool) -> Result<UpdateStatus, UpdateError> {
    let news = fetch_releases(http, include_prerelease)?;
    let latest = news.first().cloned();
    let update_available = latest.as_ref().map(|r| is_newer(&r.version, current)).unwrap_or(false);
    Ok(UpdateStatus { current: current.to_string(), latest, update_available, news })
}

/// Descarga el `SHA256SUMS.txt` de una release, si existe.
pub fn fetch_checksums(http: &dyn Http, release: &Release) -> Result<Option<Vec<(String, String)>>, UpdateError> {
    match release.checksums_asset() {
        None => Ok(None),
        Some(a) => Ok(Some(parse_sha256sums(&http.get_text(&a.download_url)?))),
    }
}

/// Descarga un binario con progreso y verificacion SHA-256 si hay sumas. Devuelve la ruta del fichero.
pub fn download_asset(
    asset: &Asset,
    sums: Option<&[(String, String)]>,
    dir: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<std::path::PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let expected = sums.and_then(|s| expected_sha256(s, &asset.name));
    let dest = dir.join(&asset.name);
    net::download_file(&asset.download_url, &dest, MAX_ASSET_BYTES, expected.as_deref(), progress)?;
    Ok(dest)
}

// ---- versiones SemVer ----

/// Separa `[v]MAYOR.MENOR.PARCHE[-prerelease][+build]`. Devuelve `None` si no encaja.
pub fn parse_version(s: &str) -> Option<(u64, u64, u64, Option<String>)> {
    let s = s.trim().trim_start_matches(|c| c == 'v' || c == 'V');
    let s = s.split('+').next()?;
    let (core, pre) = match s.split_once('-') {
        Some((c, p)) => {
            if p.is_empty() {
                return None;
            }
            (c, Some(p.to_string()))
        }
        None => (s, None),
    };
    let mut it = core.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next()?.parse().ok()?;
    let patch = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some((major, minor, patch, pre))
}

/// Orden de identificadores de prerelease (SemVer): numericos por valor, alfanumericos en texto, numerico < texto.
fn cmp_pre(a: &str, b: &str) -> Ordering {
    let (mut ia, mut ib) = (a.split('.'), b.split('.'));
    loop {
        match (ia.next(), ib.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let o = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(p), Ok(q)) => p.cmp(&q),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    _ => x.cmp(y),
                };
                if o != Ordering::Equal {
                    return o;
                }
            }
        }
    }
}

/// Compara dos versiones. Una version estable es mayor que su pre-release (`1.0.0` > `1.0.0-pre.1`).
pub fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
    let (a1, a2, a3, ap) = parse_version(a)?;
    let (b1, b2, b3, bp) = parse_version(b)?;
    let core = (a1, a2, a3).cmp(&(b1, b2, b3));
    if core != Ordering::Equal {
        return Some(core);
    }
    Some(match (ap, bp) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => cmp_pre(&x, &y),
    })
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    compare_versions(candidate, current) == Some(Ordering::Greater)
}

// ---- seleccion de assets ----

/// Coincidencia con `*` como comodin, sin distinguir mayusculas. Sin `*` exige igualdad.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p = pattern.to_ascii_lowercase();
    let n = name.to_ascii_lowercase();
    let parts: Vec<&str> = p.split('*').collect();
    if parts.len() == 1 {
        return n == p;
    }
    let first = parts[0];
    let last = parts[parts.len() - 1];
    if !n.starts_with(first) {
        return false;
    }
    let mut rest = &n[first.len()..];
    for mid in &parts[1..parts.len() - 1] {
        match rest.find(mid) {
            Some(i) => rest = &rest[i + mid.len()..],
            None => return false,
        }
    }
    rest.ends_with(last) && rest.len() >= last.len()
}

/// Formato de `sha256sum`: `<64 hex>  <nombre>` (o `*<nombre>` en modo binario).
pub fn parse_sha256sums(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let (hash, name) = line.trim().split_once(char::is_whitespace)?;
            let name = name.trim().trim_start_matches('*');
            if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) && !name.is_empty() {
                Some((hash.to_ascii_lowercase(), name.to_string()))
            } else {
                None
            }
        })
        .collect()
}

pub fn expected_sha256(sums: &[(String, String)], asset_name: &str) -> Option<String> {
    sums.iter().find(|(_, n)| n == asset_name).map(|(h, _)| h.clone())
}

// ---- instalacion en Windows ----

/// Script `.cmd` que espera a que cierre el proceso `pid`, sustituye el ejecutable y vuelve a abrirlo.
/// Rechaza rutas con comillas, `%` o saltos de linea, porque se escriben tal cual en el script.
pub fn windows_update_script(pid: u32, new_exe: &Path, target_exe: &Path) -> Result<String, String> {
    let new_s = safe_path(new_exe)?;
    let target_s = safe_path(target_exe)?;
    Ok(format!(
        "@echo off\r\n\
         :wait\r\n\
         tasklist /FI \"PID eq {pid}\" 2>nul | find \"{pid}\" >nul\r\n\
         if not errorlevel 1 (\r\n\
         \x20 timeout /t 1 /nobreak >nul\r\n\
         \x20 goto wait\r\n\
         )\r\n\
         copy /Y \"{new_s}\" \"{target_s}\" >nul\r\n\
         if errorlevel 1 (\r\n\
         \x20 echo No se pudo sustituir el ejecutable de NitroPlay.\r\n\
         \x20 pause\r\n\
         \x20 exit /b 1\r\n\
         )\r\n\
         start \"\" \"{target_s}\"\r\n\
         del \"%~f0\"\r\n"
    ))
}

fn safe_path(p: &Path) -> Result<&str, String> {
    let s = p.to_str().ok_or("ruta no UTF-8")?;
    if s.contains(['"', '%', '\n', '\r']) {
        return Err(format!("ruta no admitida en el script: {s}"));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::MockHttp;
    use std::path::PathBuf;

    const RELEASES: &str = r#"[
      {"tag_name":"v0.2.0-pre.1","name":"Pre 0.2.0","published_at":"2026-11-20T10:00:00Z","body":"Cambios de prueba",
       "draft":false,"prerelease":true,"html_url":"https://github.com/pilahito/NitroPlay/releases/tag/v0.2.0-pre.1",
       "assets":[{"name":"NitroPlay-windows-x64.exe","size":123,"browser_download_url":"https://github.com/p/n/releases/download/v0.2.0-pre.1/NitroPlay-windows-x64.exe"}]},
      {"tag_name":"v0.1.1","name":"0.1.1","published_at":"2026-11-10T10:00:00Z","body":"Estable",
       "draft":false,"prerelease":false,"html_url":"https://github.com/pilahito/NitroPlay/releases/tag/v0.1.1",
       "assets":[{"name":"SHA256SUMS.txt","size":90,"browser_download_url":"https://h/SHA256SUMS.txt"},
                 {"name":"nitroplay-android-arm64.apk","size":5,"browser_download_url":"https://h/a.apk"},
                 {"name":"nitroplay-quest-arm64.apk","size":5,"browser_download_url":"https://h/q.apk"}]},
      {"tag_name":"v0.3.0","name":"Borrador","published_at":null,"body":null,
       "draft":true,"prerelease":false,"html_url":"https://github.com/pilahito/NitroPlay/releases/tag/v0.3.0","assets":[]}
    ]"#;

    #[test]
    fn sin_pre_releases_solo_la_estable() {
        let r = parse_releases(RELEASES, false).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].version, "0.1.1");
        assert_eq!(r[0].date, "2026-11-10");
    }

    #[test]
    fn con_pre_releases_ordena_de_mas_nueva_a_mas_antigua() {
        let r = parse_releases(RELEASES, true).unwrap();
        let v: Vec<&str> = r.iter().map(|x| x.version.as_str()).collect();
        assert_eq!(v, vec!["0.2.0-pre.1", "0.1.1"]);
        assert!(r[0].is_prerelease);
    }

    #[test]
    fn semver_basico_y_prerelease() {
        assert_eq!(compare_versions("0.10.0", "0.9.0"), Some(Ordering::Greater));
        assert_eq!(compare_versions("v0.1.1", "0.1.1"), Some(Ordering::Equal));
        assert_eq!(compare_versions("1.0.0-pre.1", "1.0.0"), Some(Ordering::Less));
        assert_eq!(compare_versions("1.0.0-alpha.2", "1.0.0-alpha.10"), Some(Ordering::Less));
        assert_eq!(compare_versions("1.0.0-alpha", "1.0.0-1"), Some(Ordering::Greater));
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.2.0-pre.1", "0.2.0"));
    }

    #[test]
    fn versiones_mal_formadas_se_rechazan() {
        for bad in ["", "1.2", "1.2.3.4", "a.b.c", "1.2.3-", "latest"] {
            assert!(parse_version(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn comodines_sin_distinguir_mayusculas() {
        assert!(glob_match("*windows*x64*.exe", "NitroPlay-Windows-X64.exe"));
        assert!(!glob_match("*windows*x64*.exe", "NitroPlay-windows-x64.zip"));
        assert!(glob_match("*android*arm64*.apk", "nitroplay-android-arm64.apk"));
        assert!(!glob_match("*quest*arm64*.apk", "nitroplay-android-arm64.apk"));
        assert!(glob_match("SHA256SUMS.txt", "sha256sums.txt"));
    }

    #[test]
    fn asset_por_plataforma() {
        let r = parse_releases(RELEASES, true).unwrap();
        let pre = &r[0];
        assert_eq!(pre.asset_for(Platform::Windows).unwrap().name, "NitroPlay-windows-x64.exe");
        assert!(pre.asset_for(Platform::Android).is_none());
        let estable = &r[1];
        assert!(estable.asset_for(Platform::Windows).is_none());
        assert_eq!(estable.asset_for(Platform::Android).unwrap().name, "nitroplay-android-arm64.apk");
        assert_eq!(estable.asset_for(Platform::Quest).unwrap().name, "nitroplay-quest-arm64.apk");
        assert!(estable.checksums_asset().is_some());
    }

    #[test]
    fn sha256sums_y_busqueda() {
        let txt = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  NitroPlay-windows-x64.exe\n\
                   linea rota\n\
                   BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD *otro.zip\n";
        let sums = parse_sha256sums(txt);
        assert_eq!(sums.len(), 2);
        assert_eq!(expected_sha256(&sums, "NitroPlay-windows-x64.exe").unwrap().len(), 64);
        assert_eq!(expected_sha256(&sums, "otro.zip").unwrap(), sums[1].0);
        assert!(expected_sha256(&sums, "falta.exe").is_none());
    }

    #[test]
    fn check_con_http_simulado() {
        let mut http = MockHttp::new();
        http.texts.insert(RELEASES_API.to_string(), Ok(RELEASES.to_string()));
        let s = check(&http, "0.1.0", false).unwrap();
        assert!(s.update_available);
        assert_eq!(s.latest.unwrap().version, "0.1.1");
        assert_eq!(s.news.len(), 1);
        let al_dia = check(&http, "0.1.1", false).unwrap();
        assert!(!al_dia.update_available);
    }

    #[test]
    fn errores_de_red_y_limite_de_peticiones() {
        let mut http = MockHttp::new();
        http.texts.insert(RELEASES_API.to_string(), Err(HttpError::Status(403)));
        assert_eq!(check(&http, "0.1.0", false).err(), Some(UpdateError::RateLimited));
        http.texts.insert(RELEASES_API.to_string(), Err(HttpError::Network("dns".into())));
        assert!(matches!(check(&http, "0.1.0", false), Err(UpdateError::Network(_))));
        assert!(matches!(parse_releases("{no json", false), Err(UpdateError::BadData(_))));
    }

    #[test]
    fn script_de_actualizacion_espera_al_pid() {
        let s = windows_update_script(4242, &PathBuf::from(r"C:\Temp\NitroPlay-update\n.exe"), &PathBuf::from(r"C:\Apps\nitroplay.exe")).unwrap();
        assert!(s.contains("PID eq 4242"));
        assert!(s.contains(r#"copy /Y "C:\Temp\NitroPlay-update\n.exe" "C:\Apps\nitroplay.exe""#));
        assert!(windows_update_script(1, &PathBuf::from("a\"b.exe"), &PathBuf::from("c.exe")).is_err());
    }
}
