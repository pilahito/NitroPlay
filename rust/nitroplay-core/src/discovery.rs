//! Deteccion de consolas por UDP (protocolo SRCH, leido de lib/src/discovery.c de chiaki).

/// PS4 escucha en 987 y PS5 en 9302.
pub const PS4_PORT: u16 = 987;
pub const PS5_PORT: u16 = 9302;
const PS4_VERSION: &str = "00020020";
const PS5_VERSION: &str = "00030010";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generation {
    Ps4,
    Ps5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Ready,
    Standby,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Console {
    pub name: String,
    pub kind: String,
    pub state: State,
}

pub fn port(g: Generation) -> u16 {
    match g {
        Generation::Ps4 => PS4_PORT,
        Generation::Ps5 => PS5_PORT,
    }
}

/// Paquete de busqueda para la generacion indicada.
pub fn srch_packet(g: Generation) -> String {
    let v = match g {
        Generation::Ps4 => PS4_VERSION,
        Generation::Ps5 => PS5_VERSION,
    };
    format!("SRCH * HTTP/1.1\ndevice-discovery-protocol-version:{v}\n")
}

fn clean(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric() || " .-_()".contains(*c)).take(40).collect()
}

/// Interpreta la respuesta de una consola: 200 = encendida, 620 = en reposo.
pub fn parse_response(data: &[u8]) -> Option<Console> {
    let text = String::from_utf8_lossy(data);
    let mut lines = text.lines();
    let mut first = lines.next()?.split_whitespace();
    if first.next()? != "HTTP/1.1" {
        return None;
    }
    let state = match first.next()? {
        "200" => State::Ready,
        "620" => State::Standby,
        _ => State::Unknown,
    };
    let (mut name, mut kind) = (String::new(), String::new());
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            match k.trim() {
                "host-name" => name = clean(v.trim()),
                "host-type" => kind = clean(v.trim()),
                _ => {}
            }
        }
    }
    Some(Console { name, kind, state })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paquetes_por_generacion() {
        assert!(srch_packet(Generation::Ps5).contains("00030010"));
        assert!(srch_packet(Generation::Ps4).contains("00020020"));
        assert_eq!((port(Generation::Ps4), port(Generation::Ps5)), (987, 9302));
    }

    #[test]
    fn respuesta_encendida_y_reposo() {
        let r = b"HTTP/1.1 200 Ok\nhost-name:Mi PS5\nhost-type:PS5\n";
        let c = parse_response(r).unwrap();
        assert_eq!((c.name.as_str(), c.kind.as_str(), c.state), ("Mi PS5", "PS5", State::Ready));
        assert_eq!(parse_response(b"HTTP/1.1 620 Server Standby\nhost-name:X\n").unwrap().state, State::Standby);
        assert_eq!(parse_response(b"HTTP/1.1 500 Err\n").unwrap().state, State::Unknown);
    }

    #[test]
    fn basura_se_descarta_y_nombre_se_limpia() {
        assert!(parse_response(b"hola").is_none());
        assert!(parse_response(b"").is_none());
        let c = parse_response(b"HTTP/1.1 200 Ok\nhost-name:<b>PS5</b>\n").unwrap();
        assert_eq!(c.name, "bPS5b");
    }
}
