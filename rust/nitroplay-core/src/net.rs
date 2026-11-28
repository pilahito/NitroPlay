//! HTTP compartido por el actualizador y los proveedores de fondos.
//! El trait `Http` separa la red de la logica: en las pruebas se usa `MockHttp` (sin red).

use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpError {
    Status(u16),
    Network(String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Status(c) => write!(f, "HTTP {c}"),
            HttpError::Network(m) => write!(f, "red: {m}"),
        }
    }
}

pub trait Http {
    fn get_text(&self, url: &str) -> Result<String, HttpError>;
    fn get_bytes(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, HttpError>;
}

/// Cliente real (ureq). Identifica a NitroPlay con `User-Agent: NitroPlay/<version>`, como pide la API de GitHub.
pub struct UreqHttp {
    user_agent: String,
}

impl UreqHttp {
    pub fn new() -> Self {
        Self { user_agent: format!("NitroPlay/{}", env!("CARGO_PKG_VERSION")) }
    }

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }
}

impl Default for UreqHttp {
    fn default() -> Self {
        Self::new()
    }
}

fn map_err(e: ureq::Error) -> HttpError {
    match e {
        ureq::Error::StatusCode(c) => HttpError::Status(c),
        other => HttpError::Network(other.to_string()),
    }
}

impl Http for UreqHttp {
    fn get_text(&self, url: &str) -> Result<String, HttpError> {
        let mut resp = ureq::get(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Accept", "*/*")
            .call()
            .map_err(map_err)?;
        resp.body_mut().read_to_string().map_err(|e| HttpError::Network(e.to_string()))
    }

    fn get_bytes(&self, url: &str, max_bytes: u64) -> Result<Vec<u8>, HttpError> {
        let mut resp = ureq::get(url)
            .header("User-Agent", self.user_agent.as_str())
            .header("Accept", "*/*")
            .call()
            .map_err(map_err)?;
        resp.body_mut()
            .with_config()
            .limit(max_bytes)
            .read_to_vec()
            .map_err(|e| HttpError::Network(e.to_string()))
    }
}

/// Descarga a un fichero temporal `.part` con progreso y SHA-256 opcional; solo al final renombra a `dest`.
/// `progress(hechos, total)`: `total` es 0 si el servidor no informa del tamaño.
pub fn download_file(
    url: &str,
    dest: &Path,
    max_bytes: u64,
    expected_sha256: Option<&str>,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<(), String> {
    let ua = UreqHttp::new();
    let mut resp = ureq::get(url)
        .header("User-Agent", ua.user_agent())
        .header("Accept", "*/*")
        .call()
        .map_err(|e| map_err(e).to_string())?;
    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    if total > max_bytes {
        return Err(format!("el archivo pesa {total} bytes, por encima del límite"));
    }
    let tmp = dest.with_extension("part");
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut reader = resp.body_mut().with_config().limit(max_bytes).reader();
    let mut hasher = Sha256::new();
    let mut done: u64 = 0;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            e.to_string()
        })?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        hasher.update(&buf[..n]);
        done += n as u64;
        progress(done, total);
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);
    if let Some(exp) = expected_sha256 {
        let got = hex(&hasher.finalize());
        if !got.eq_ignore_ascii_case(exp) {
            let _ = std::fs::remove_file(&tmp);
            return Err("el SHA-256 no coincide; descarga descartada".into());
        }
    }
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

/// Codificacion de consulta (`application/x-www-form-urlencoded`): espacio -> `+`.
pub fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Simulacion de HTTP para las pruebas: respuestas fijas por URL y contador de peticiones.
#[cfg(test)]
pub struct MockHttp {
    pub texts: std::collections::HashMap<String, Result<String, HttpError>>,
    pub bytes: Vec<u8>,
    pub calls: std::cell::Cell<u32>,
}

#[cfg(test)]
impl MockHttp {
    pub fn new() -> Self {
        Self { texts: Default::default(), bytes: Vec::new(), calls: std::cell::Cell::new(0) }
    }
}

#[cfg(test)]
impl Http for MockHttp {
    fn get_text(&self, url: &str) -> Result<String, HttpError> {
        self.calls.set(self.calls.get() + 1);
        match self.texts.get(url) {
            Some(r) => r.clone(),
            None => Err(HttpError::Status(404)),
        }
    }

    fn get_bytes(&self, _url: &str, _max: u64) -> Result<Vec<u8>, HttpError> {
        self.calls.set(self.calls.get() + 1);
        Ok(self.bytes.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_de_abc() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn codifica_consultas() {
        assert_eq!(url_encode("PlayStation 5"), "PlayStation+5");
        assert_eq!(url_encode("Astro/Bot?"), "Astro%2FBot%3F");
    }
}
