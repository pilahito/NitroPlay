//! Registro de la consola con PIN + Account-ID. La logica esta en `regist_net` (Rust puro).

#[derive(Debug, PartialEq, Eq)]
pub enum RegistError {
    /// El Account-ID no es base64 de exactamente 8 bytes.
    BadAccountId,
    /// La direccion de la consola contiene un caracter nulo.
    BadHost,
    /// Fallo de red (socket, conexion cerrada...).
    Network(String),
    /// La consola contesto con un codigo HTTP distinto de 200 (`reason` = RP-Application-Reason).
    Http { status: u16, reason: Option<u32> },
    /// La respuesta no trae las claves esperadas.
    InvalidResponse,
    /// La consola rechazo el registro (PIN caducado o incorrecto, cuenta distinta...) o no respondio.
    Failed,
    Canceled,
    Timeout,
}

pub struct RegistRequest<'a> {
    pub console_ip: &'a str,
    pub is_ps5: bool,
    /// Account-ID en base64 (de psn-account).
    pub account_id_b64: &'a str,
    /// PIN ya validado con `pin::parse_pin`.
    pub pin: u32,
}

/// Credenciales de la consola tras registrar (se guardan solo en local).
#[derive(Debug, Clone)]
pub struct RegisteredConsole {
    pub nickname: String,
    pub server_mac: [u8; 6],
    pub rp_regist_key: [u8; 16],
    pub rp_key: [u8; 16],
    pub rp_key_type: u32,
}

/// Decodifica el Account-ID (base64 estandar) a sus 8 bytes.
pub fn decode_account_id(b64: &str) -> Option<[u8; 8]> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(8);
    for c in b64.trim().bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xff) as u8);
        }
    }
    <[u8; 8]>::try_from(out.as_slice()).ok()
}

pub use crate::regist_net::register;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_id_de_ejemplo_decodifica_a_8_bytes() {
        let b = decode_account_id("AQIDBAUGBwg=").unwrap();
        assert_eq!(b.len(), 8);
        // Ida y vuelta con el codificador de psn-account no esta disponible aqui: se comprueba un valor conocido.
        assert_eq!(decode_account_id("AAAAAAAAAAA="), Some([0u8; 8]));
        assert_eq!(decode_account_id("AQIDBAUGBwg="), Some([1, 2, 3, 4, 5, 6, 7, 8]));
    }

    #[test]
    fn account_id_invalido_se_rechaza() {
        assert_eq!(decode_account_id("corto"), None);
        assert_eq!(decode_account_id("!!!!"), None);
        assert_eq!(decode_account_id(""), None);
    }
}
