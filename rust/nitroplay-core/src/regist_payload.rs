//! Cuerpo de la peticion de registro (POST /sie/ps5/rp/sess/rgst o /sie/ps4/...).
//! Portado de chiaki_regist_request_payload_format (lib/src/regist.c), rama PIN + Account-ID,
//! para PS4 10+ y PS5. Las cabeceras HTTP y el transporte van en otro modulo.

use crate::rpcrypt::{aeropause, aeropause_ps4_pre10, ambassador_from_aeropause, CryptError, RpCrypt, Target};

/// Desplazamiento donde empieza la cabecera interna cifrada.
pub const INNER_HEADER_OFF: usize = 0x1e0;

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 estandar con relleno (igual que chiaki_base64_encode).
pub fn base64_encode(input: &[u8]) -> String {
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(B64[((n >> (18 - 6 * i)) & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Cliente de tipo para PS4 10 y PS5 (valor fijo de chiaki).
pub const CLIENT_TYPE: &str = "dabfa2ec873de5839bee8d3f4c0239c4282c07c25c6077a2931afcf0adc0d34f";
/// Cliente de tipo para PS4 anterior a 10.
pub const CLIENT_TYPE_PS4_PRE10: &str = "Windows";

#[derive(Debug, PartialEq, Eq)]
pub enum PayloadError {
    /// El registro por PSN (holepunch) aun no esta portado.
    Unsupported,
    Crypt(CryptError),
}

impl From<CryptError> for PayloadError {
    fn from(e: CryptError) -> Self {
        PayloadError::Crypt(e)
    }
}

/// Devuelve (cuerpo, cifrado para el resto de la sesion de registro).
///
/// `filler` son los bytes de relleno fuera de la cabecera cifrada (chiaki usa 'A').
/// Los offsets de clave salen de esos mismos bytes; con relleno 'A' son constantes.
pub fn format_payload(
    target: Target,
    ambassador: &[u8; 16],
    account_id: &[u8; 8],
    pin: u32,
) -> Result<(Vec<u8>, RpCrypt), PayloadError> {
    let mut buf = vec![b'A'; INNER_HEADER_OFF];

    let crypt = if target.code() < Target::Ps4_10.code() {
        let rp = RpCrypt::init_regist_ps4_pre10(ambassador, pin);
        let amb = aeropause_ps4_pre10(&rp.ambassador);
        buf[0x11c..0x11c + 16].copy_from_slice(&amb);
        rp
    } else {
        let key_0_off = (buf[0x18D] & 0x1f) as usize;
        let key_1_off = (buf[0] >> 3) as usize;
        let rp = RpCrypt::init_regist(target, ambassador, key_0_off, pin)?;
        let aero = aeropause(target, key_1_off, &rp.ambassador)?;
        buf[0xc7..0xc7 + 8].copy_from_slice(&aero[8..16]);
        buf[0x191..0x191 + 8].copy_from_slice(&aero[0..8]);
        rp
    };

    let _ = ambassador_from_aeropause; // reservado para el registro por PSN
    let client = if target.code() < Target::Ps4_10.code() {
        CLIENT_TYPE_PS4_PRE10
    } else {
        CLIENT_TYPE
    };
    let inner = format!("Client-Type: {}\r\nNp-AccountId: {}\r\n", client, base64_encode(account_id));
    let mut inner = inner.into_bytes();
    crypt.encrypt(0, &mut inner);

    buf.truncate(INNER_HEADER_OFF);
    buf.extend_from_slice(&inner);
    Ok((buf, crypt))
}

#[cfg(all(test, feature = "ref-c"))]
mod oracle {
    use super::*;
    use nitroplay_ref_sys as c;

    /// Compara el cuerpo de registro byte a byte con chiaki_regist_request_payload_format.
    #[test]
    fn cuerpo_igual_a_la_referencia() {
        let targets = [Target::Ps4_10, Target::Ps5_1, Target::Ps4_8];
        let mut s = 99u64;
        let mut next = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (s >> 33) as u8
        };
        for round in 0..60 {
            let t = targets[round % 3];
            let amb: [u8; 16] = std::array::from_fn(|_| next());
            let acc: [u8; 8] = std::array::from_fn(|_| next());
            let pin = u32::from_be_bytes(std::array::from_fn(|_| next()));
            let (mine, _) = format_payload(t, &amb, &acc, pin).unwrap();

            let mut theirs = vec![0u8; 0x400];
            let mut len = theirs.len();
            let mut crypt: c::ChiakiRPCrypt = unsafe { std::mem::zeroed() };
            let err = unsafe {
                c::chiaki_regist_request_payload_format(
                    t.code(),
                    amb.as_ptr(),
                    theirs.as_mut_ptr(),
                    &mut len,
                    &mut crypt,
                    std::ptr::null(),
                    acc.as_ptr(),
                    pin,
                    std::ptr::null_mut(),
                )
            };
            assert_eq!(err, 0, "la referencia devolvio error {err} para {t:?}");
            theirs.truncate(len);
            assert_eq!(mine, theirs, "cuerpo distinto para {t:?}");
        }
    }
}
