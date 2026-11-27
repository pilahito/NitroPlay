//! Transporte del registro en Rust puro: busqueda UDP, peticion TCP y lectura de la respuesta.
//! Portado de regist.c (rama sin holepunch/PSN). Puerto, tiempos y cabeceras son los de la referencia.

use crate::regist::{decode_account_id, RegisteredConsole, RegistError, RegistRequest};
use crate::regist_payload::format_payload;
use crate::rpcrypt::{RpCrypt, Target};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, UdpSocket};
use std::time::{Duration, Instant};

pub const REGIST_PORT: u16 = 9295;
const SEARCH_REQUEST_SLEEP: Duration = Duration::from_millis(100);

#[derive(Clone, Copy)]
pub struct Timeouts {
    pub search: Duration,
    pub response: Duration,
}
impl Default for Timeouts {
    fn default() -> Self {
        Timeouts { search: Duration::from_millis(3000), response: Duration::from_millis(3000) }
    }
}

fn rp_version(t: Target) -> &'static str {
    match t {
        Target::Ps4_8 => "8.0",
        Target::Ps4_9 => "9.0",
        Target::Ps4_10 => "10.0",
        Target::Ps5_1 => "1.0",
    }
}

fn request_path(t: Target) -> &'static str {
    match t {
        Target::Ps5_1 => "/sie/ps5/rp/sess/rgst",
        Target::Ps4_8 | Target::Ps4_9 => "/sce/rp/regist",
        Target::Ps4_10 => "/sie/ps4/rp/sess/rgst",
    }
}

/// Cabecera HTTP exacta de la referencia (incluida la repeticion rara de "HTTP/1.1" en la primera linea).
pub fn request_header(t: Target, payload_len: usize) -> Vec<u8> {
    let mut h = format!(
        "POST {} HTTP/1.1\r\n HTTP/1.1\r\nHOST: 10.0.2.15\r\nUser-Agent: remoteplay Windows\r\nConnection: close\r\nContent-Length: {}\r\n",
        request_path(t),
        payload_len
    );
    if t.code() >= Target::Ps4_9.code() {
        h.push_str(&format!("RP-Version: {}\r\n", rp_version(t)));
    }
    h.push_str("\r\n");
    h.into_bytes()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// Parsea las lineas "Clave: valor" del cuerpo descifrado.
pub fn parse_response_payload(text: &str, ps5: bool) -> Result<RegisteredConsole, RegistError> {
    let (k_regist, k_mac, k_nick) = if ps5 { ("PS5-RegistKey", "PS5-Mac", "PS5-Nickname") } else { ("PS4-RegistKey", "PS4-Mac", "PS4-Nickname") };
    let (mut regist, mut key, mut mac) = (None, None, None);
    let mut nick = String::new();
    let mut key_type = 0u32;
    for line in text.split("\r\n") {
        let Some((k, v)) = line.split_once(':') else { continue };
        let v = v.trim();
        if k == k_regist {
            regist = hex_decode(v).and_then(|b| <[u8; 16]>::try_from(b.as_slice()).ok().or_else(|| {
                // la referencia admite menos de 16 bytes; se rellena con ceros a la derecha
                if b.len() < 16 { let mut a = [0u8; 16]; a[..b.len()].copy_from_slice(&b); Some(a) } else { None }
            }));
        } else if k == "RP-Key" {
            key = hex_decode(v).and_then(|b| <[u8; 16]>::try_from(b.as_slice()).ok());
        } else if k == k_mac {
            mac = hex_decode(v).and_then(|b| <[u8; 6]>::try_from(b.as_slice()).ok());
        } else if k == "RP-KeyType" {
            key_type = v.parse().unwrap_or(0);
        } else if k == k_nick {
            nick = v.to_string();
        }
    }
    match (regist, key, mac) {
        (Some(rp_regist_key), Some(rp_key), Some(server_mac)) => Ok(RegisteredConsole { nickname: nick, server_mac, rp_regist_key, rp_key, rp_key_type: key_type }),
        _ => Err(RegistError::InvalidResponse),
    }
}

fn net<E: std::fmt::Display>(e: E) -> RegistError {
    RegistError::Network(e.to_string())
}

/// Registro completo. `ambassador` se inyecta para poder probar sin consola; en uso real es aleatorio.
pub fn register_with(req: &RegistRequest, port: u16, ambassador: [u8; 16], t: Timeouts) -> Result<RegisteredConsole, RegistError> {
    let account = decode_account_id(req.account_id_b64).ok_or(RegistError::BadAccountId)?;
    let ip: IpAddr = req.console_ip.parse().map_err(|_| RegistError::BadHost)?;
    let target = if req.is_ps5 { Target::Ps5_1 } else { Target::Ps4_10 };

    let (payload, crypt) = format_payload(target, &ambassador, &account, req.pin).map_err(|_| RegistError::Failed)?;

    // 1) busqueda UDP: SRC3/RES3 (PS5) o SRC2/RES2 (PS4), con el byte nulo final
    let (src, res) = if req.is_ps5 { (&b"SRC3\0"[..], b"RES3" as &[u8]) } else { (&b"SRC2\0"[..], b"RES2" as &[u8]) };
    let udp = UdpSocket::bind(if ip.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" }).map_err(net)?;
    udp.send_to(src, SocketAddr::new(ip, port)).map_err(net)?;
    let deadline = Instant::now() + t.search;
    let found: SocketAddr = loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(RegistError::Timeout);
        }
        udp.set_read_timeout(Some(left)).map_err(net)?;
        let mut buf = [0u8; 0x100];
        match udp.recv_from(&mut buf) {
            Ok((n, from)) if n >= res.len() && &buf[..res.len()] == res => break from,
            Ok(_) => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => return Err(RegistError::Timeout),
            Err(e) => return Err(net(e)),
        }
    };
    std::thread::sleep(SEARCH_REQUEST_SLEEP); // la PS4 no acepta peticiones de inmediato

    // 2) peticion TCP
    let mut tcp = TcpStream::connect_timeout(&found, t.response).map_err(net)?;
    tcp.set_read_timeout(Some(t.response)).map_err(net)?;
    tcp.set_write_timeout(Some(t.response)).map_err(net)?;
    tcp.write_all(&request_header(target, payload.len())).map_err(net)?;
    tcp.write_all(&payload).map_err(net)?;

    // 3) respuesta: cabecera HTTP, luego Content-Length bytes cifrados
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1500];
    let header_end = loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p + 4;
        }
        if buf.len() > 1500 {
            return Err(RegistError::InvalidResponse);
        }
        let n = tcp.read(&mut chunk).map_err(|e| if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) { RegistError::Timeout } else { net(e) })?;
        if n == 0 {
            return Err(RegistError::Network("la consola cerro la conexion".into()));
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = head.split("\r\n");
    let status: u16 = lines.next().and_then(|l| l.split_whitespace().nth(1)).and_then(|c| c.parse().ok()).ok_or(RegistError::InvalidResponse)?;
    let mut content_len = 0usize;
    let mut reason = None;
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            if k.eq_ignore_ascii_case("Content-Length") {
                content_len = v.trim().parse().unwrap_or(0);
            } else if k.eq_ignore_ascii_case("RP-Application-Reason") {
                reason = u32::from_str_radix(v.trim().trim_start_matches("0x"), 16).ok();
            }
        }
    }
    if status != 200 {
        return Err(RegistError::Http { status, reason });
    }
    if content_len == 0 || content_len > 1500 - header_end.min(1500) {
        return Err(RegistError::InvalidResponse);
    }
    while buf.len() < header_end + content_len {
        let n = tcp.read(&mut chunk).map_err(|e| if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) { RegistError::Timeout } else { net(e) })?;
        if n == 0 {
            return Err(RegistError::Network("respuesta incompleta".into()));
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let mut body = buf[header_end..].to_vec();
    let crypt: RpCrypt = crypt;
    crypt.decrypt(0, &mut body);
    parse_response_payload(&String::from_utf8_lossy(&body), req.is_ps5)
}

/// Registro real: ambassador aleatorio y tiempos por defecto.
pub fn register(req: &RegistRequest) -> Result<RegisteredConsole, RegistError> {
    let mut amb = [0u8; 16];
    getrandom::getrandom(&mut amb).map_err(net)?;
    register_with(req, REGIST_PORT, amb, Timeouts::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn cabecera_ps5_y_ps4() {
        let h = String::from_utf8(request_header(Target::Ps5_1, 0x1f0)).unwrap();
        assert_eq!(h, "POST /sie/ps5/rp/sess/rgst HTTP/1.1\r\n HTTP/1.1\r\nHOST: 10.0.2.15\r\nUser-Agent: remoteplay Windows\r\nConnection: close\r\nContent-Length: 496\r\nRP-Version: 1.0\r\n\r\n");
        let h8 = String::from_utf8(request_header(Target::Ps4_8, 10)).unwrap();
        assert!(h8.starts_with("POST /sce/rp/regist ") && !h8.contains("RP-Version"));
    }

    #[test]
    fn respuesta_valida_e_incompleta() {
        let ok = "PS5-RegistKey: 0102030405060708090a0b0c0d0e0f10\r\nRP-KeyType: 2\r\nRP-Key: 000102030405060708090a0b0c0d0e0f\r\nPS5-Mac: aabbccddeeff\r\nPS5-Nickname: PS5-test\r\n";
        let r = parse_response_payload(ok, true).unwrap();
        assert_eq!(r.server_mac, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
        assert_eq!(r.rp_key_type, 2);
        assert_eq!(r.nickname, "PS5-test");
        assert_eq!(parse_response_payload("PS5-Mac: aabbccddeeff\r\n", true).err(), Some(RegistError::InvalidResponse));
    }

    /// Consola simulada en 127.0.0.1: valida el transporte, la cabecera y el descifrado de ida y vuelta.
    /// No demuestra que una PS5 real acepte el registro.
    #[test]
    fn consola_simulada_en_local() {
        let amb = [7u8; 16];
        let acc = decode_account_id("AQIDBAUGBwg=").unwrap();
        let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = tcp.local_addr().unwrap().port();
        let udp = UdpSocket::bind(("127.0.0.1", port)).unwrap();
        let h = std::thread::spawn(move || {
            let mut b = [0u8; 64];
            let (n, from) = udp.recv_from(&mut b).unwrap();
            assert_eq!(&b[..n], b"SRC3\0");
            udp.send_to(b"RES3", from).unwrap();
            let (mut s, _) = tcp.accept().unwrap();
            let mut got = Vec::new();
            let mut c = [0u8; 2048];
            let hdr = request_header(Target::Ps5_1, 0);
            let _ = hdr;
            while !got.windows(4).any(|w| w == b"\r\n\r\n") { let n = s.read(&mut c).unwrap(); got.extend_from_slice(&c[..n]); }
            assert!(got.starts_with(b"POST /sie/ps5/rp/sess/rgst HTTP/1.1"));
            let (_, crypt) = format_payload(Target::Ps5_1, &amb, &acc, 12345678).unwrap();
            let mut body = b"PS5-RegistKey: 0102030405060708090a0b0c0d0e0f10\r\nRP-KeyType: 2\r\nRP-Key: 000102030405060708090a0b0c0d0e0f\r\nPS5-Mac: aabbccddeeff\r\nPS5-Nickname: SIM\r\n".to_vec();
            crypt.encrypt(0, &mut body);
            s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len()).as_bytes()).unwrap();
            s.write_all(&body).unwrap();
        });
        let req = RegistRequest { console_ip: "127.0.0.1", is_ps5: true, account_id_b64: "AQIDBAUGBwg=", pin: 12345678 };
        let r = register_with(&req, port, amb, Timeouts::default()).unwrap();
        h.join().unwrap();
        assert_eq!(r.nickname, "SIM");
        assert_eq!(r.rp_key[15], 0x0f);
    }

    #[test]
    fn sin_consola_da_timeout_sin_colgarse() {
        let req = RegistRequest { console_ip: "127.0.0.1", is_ps5: true, account_id_b64: "AQIDBAUGBwg=", pin: 12345678 };
        let t = Timeouts { search: Duration::from_millis(300), response: Duration::from_millis(300) };
        let r = register_with(&req, 9, [1u8; 16], t); // puerto 9 (discard): nadie responde
        assert!(matches!(r, Err(RegistError::Timeout) | Err(RegistError::Network(_))), "{r:?}");
    }
}
