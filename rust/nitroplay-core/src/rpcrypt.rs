//! Cifrado de Remote Play (RPCrypt) en Rust puro. Portado de lib/src/rpcrypt.c (AGPL-3.0-only con
//! excepcion OpenSSL). Las tablas de claves se generan con tools/gen_rpcrypt_tables.py.
//! Cubierto: derivacion de claves de sesion, de registro (PIN) y cifrado AES-128-CFB con IV por HMAC-SHA256.
//! No cubierto aun: registro por PSN (`init_regist_psn`).

use crate::rpcrypt_tables::*;
use aes::Aes128;
use cfb_mode::cipher::{AsyncStreamCipher, KeyIvInit};
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub const KEY_SIZE: usize = 0x10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Ps4_8,
    Ps4_9,
    Ps4_10,
    Ps5_1,
}

impl Target {
    pub fn is_ps5(self) -> bool {
        matches!(self, Target::Ps5_1)
    }
    fn pre10(self) -> bool {
        matches!(self, Target::Ps4_8 | Target::Ps4_9)
    }
    /// Valor numerico de la libreria de referencia (ChiakiTarget).
    pub fn code(self) -> u32 {
        match self {
            Target::Ps4_8 => 800,
            Target::Ps4_9 => 900,
            Target::Ps4_10 => 1000,
            Target::Ps5_1 => 1_000_100,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CryptError {
    /// Version de consola anterior a PS4 10 donde la operacion no existe.
    InvalidTarget,
    /// Desplazamiento de clave fuera de rango (debe ser < 0x20).
    InvalidOffset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpCrypt {
    pub target: Target,
    pub bright: [u8; KEY_SIZE],
    pub ambassador: [u8; KEY_SIZE],
}

const ECHO_A: [u8; 16] = [0x01, 0x49, 0x87, 0x9b, 0x65, 0x39, 0x8b, 0x39, 0x4b, 0x3a, 0x8d, 0x48, 0xc3, 0x0a, 0xef, 0x51];
const ECHO_B: [u8; 16] = [0xe1, 0xec, 0x9c, 0x3a, 0xdd, 0xbd, 0x08, 0x85, 0xfc, 0x0e, 0x1d, 0x78, 0x90, 0x32, 0xc0, 0x04];
const REGIST_AES_KEY_PRE10: [u8; 16] = [0x3f, 0x1c, 0xc4, 0xb6, 0xdc, 0xbb, 0x3e, 0xcc, 0x50, 0xba, 0xed, 0xef, 0x97, 0x34, 0xc7, 0xc9];
const HMAC_KEY_PS5: [u8; 16] = [0x46, 0x46, 0x87, 0xb3, 0x49, 0xca, 0x8c, 0xe8, 0x59, 0xc5, 0x27, 0x0f, 0x5d, 0x7a, 0x69, 0xd6];
const HMAC_KEY_PS4: [u8; 16] = [0x20, 0xd6, 0x6f, 0x59, 0x04, 0xea, 0x7c, 0x14, 0xe5, 0x57, 0xff, 0xc5, 0x2e, 0x48, 0x8a, 0xc8];
const HMAC_KEY_PS4_PRE10: [u8; 16] = [0xac, 0x07, 0x88, 0x83, 0xc8, 0x3a, 0x1f, 0xe8, 0x11, 0x46, 0x3a, 0xf3, 0x9e, 0xe3, 0xe3, 0x77];

/// (bright, ambassador) a partir del nonce de la consola y la clave "morning".
pub fn bright_ambassador(target: Target, nonce: &[u8; 16], morning: &[u8; 16]) -> ([u8; 16], [u8; 16]) {
    let mut bright = [0u8; 16];
    let mut amb = [0u8; 16];
    if target.pre10() {
        for i in 0..16 {
            amb[i] = nonce[i].wrapping_sub(i as u8).wrapping_sub(0x27) ^ ECHO_A[i];
            bright[i] = (morning[i].wrapping_sub(i as u8).wrapping_add(0x34) ^ ECHO_B[i]) ^ nonce[i];
        }
        return (bright, amb);
    }
    let (keys_a, keys_b): (&[u8], &[u8]) = if target.is_ps5() {
        (&BRIGHT_AMBASSADOR_KEYS_A_PS5, &BRIGHT_AMBASSADOR_KEYS_B_PS5)
    } else {
        (&BRIGHT_AMBASSADOR_KEYS_A_PS4, &BRIGHT_AMBASSADOR_KEYS_B_PS4)
    };
    let ka = &keys_a[(nonce[0] >> 3) as usize * 0x70..];
    for i in 0..16 {
        let mut v = nonce[i];
        if target.is_ps5() {
            v = v.wrapping_sub(0x2d).wrapping_sub(i as u8);
        } else {
            v = v.wrapping_add(0x36).wrapping_add(i as u8);
        }
        amb[i] = v ^ ka[i];
    }
    let kb = &keys_b[(nonce[7] >> 3) as usize * 0x70..];
    for i in 0..16 {
        bright[i] = if target.is_ps5() {
            morning[i].wrapping_add(0x18).wrapping_add(i as u8) ^ nonce[i] ^ kb[i]
        } else {
            (kb[i] ^ morning[i]).wrapping_add(0x21).wrapping_add(i as u8) ^ nonce[i]
        };
    }
    (bright, amb)
}

pub fn aeropause_ps4_pre10(ambassador: &[u8; 16]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = ambassador[i].wrapping_sub(i as u8).wrapping_sub(0x29) ^ ECHO_B[i];
    }
    out
}

fn keys_1(target: Target, table_ps4: &'static [u8], table_ps5: &'static [u8]) -> &'static [u8] {
    if target.is_ps5() {
        table_ps5
    } else {
        table_ps4
    }
}

pub fn aeropause(target: Target, key_1_off: usize, ambassador: &[u8; 16]) -> Result<[u8; 16], CryptError> {
    if target.pre10() {
        return Err(CryptError::InvalidTarget);
    }
    if key_1_off >= 0x20 {
        return Err(CryptError::InvalidOffset);
    }
    let k = keys_1(target, &AEROPAUSE_PS4_KEYS_1, &AEROPAUSE_PS5_KEYS_1);
    let w: u8 = if target.is_ps5() { 0u8.wrapping_sub(0x2d) } else { 0x29 };
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = (ambassador[i] ^ k[i * 0x20 + key_1_off]).wrapping_add(w).wrapping_add(i as u8);
    }
    Ok(out)
}

pub fn aeropause_psn(target: Target, key_1_off: usize, ambassador: &[u8; 16]) -> Result<[u8; 16], CryptError> {
    if target.pre10() {
        return Err(CryptError::InvalidTarget);
    }
    if key_1_off >= 0x20 {
        return Err(CryptError::InvalidOffset);
    }
    let k = keys_1(target, &AEROPAUSE_PSN_PS4_KEYS_1, &AEROPAUSE_PSN_PS5_KEYS_1);
    let w: u8 = if target.is_ps5() { 0x2b } else { 0u8.wrapping_sub(0x29) };
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = ambassador[i].wrapping_sub(i as u8).wrapping_add(w) ^ k[i * 0x20 + key_1_off];
    }
    Ok(out)
}

pub fn ambassador_from_aeropause(target: Target, key_1_off: usize, aeropause: &[u8; 16]) -> Result<[u8; 16], CryptError> {
    if target.pre10() {
        return Err(CryptError::InvalidTarget);
    }
    if key_1_off >= 0x20 {
        return Err(CryptError::InvalidOffset);
    }
    let k = keys_1(target, &AMBASSADOR_FROM_AEROPAUSE_PS4_KEYS_1, &AMBASSADOR_FROM_AEROPAUSE_PS5_KEYS_1);
    let w: u8 = if target.is_ps5() { 0x2b } else { 0u8.wrapping_sub(0x29) };
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = (aeropause[i] ^ k[i * 0x20 + key_1_off]).wrapping_add(i as u8).wrapping_sub(w);
    }
    Ok(out)
}

impl RpCrypt {
    pub fn init_auth(target: Target, nonce: &[u8; 16], morning: &[u8; 16]) -> RpCrypt {
        let (bright, ambassador) = bright_ambassador(target, nonce, morning);
        RpCrypt { target, bright, ambassador }
    }

    pub fn init_regist_ps4_pre10(ambassador: &[u8; 16], pin: u32) -> RpCrypt {
        let mut bright = REGIST_AES_KEY_PRE10;
        for (b, p) in bright.iter_mut().zip(pin.to_be_bytes()) {
            *b ^= p;
        }
        RpCrypt { target: Target::Ps4_9, bright, ambassador: *ambassador }
    }

    pub fn init_regist(target: Target, ambassador: &[u8; 16], key_0_off: usize, pin: u32) -> Result<RpCrypt, CryptError> {
        if target.pre10() {
            return Err(CryptError::InvalidTarget);
        }
        if key_0_off >= 0x20 {
            return Err(CryptError::InvalidOffset);
        }
        let k: &[u8] = if target.is_ps5() { &INIT_REGIST_PS5_KEYS_0 } else { &INIT_REGIST_PS4_KEYS_0 };
        let mut bright = [0u8; 16];
        for i in 0..16 {
            bright[i] = k[i * 0x20 + key_0_off];
        }
        for (b, p) in bright[12..16].iter_mut().zip(pin.to_be_bytes()) {
            *b ^= p;
        }
        Ok(RpCrypt { target, bright, ambassador: *ambassador })
    }

    fn hmac_key(&self) -> &'static [u8; 16] {
        match self.target {
            Target::Ps5_1 => &HMAC_KEY_PS5,
            Target::Ps4_8 | Target::Ps4_9 => &HMAC_KEY_PS4_PRE10,
            Target::Ps4_10 => &HMAC_KEY_PS4,
        }
    }

    /// IV = primeros 16 bytes de HMAC-SHA256(clave por version, ambassador || contador big-endian).
    pub fn generate_iv(&self, counter: u64) -> [u8; 16] {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(self.hmac_key()).expect("HMAC acepta cualquier longitud de clave");
        mac.update(&self.ambassador);
        mac.update(&counter.to_be_bytes());
        let full = mac.finalize().into_bytes();
        let mut iv = [0u8; 16];
        iv.copy_from_slice(&full[..16]);
        iv
    }

    /// AES-128-CFB128, sin relleno, en el sitio.
    pub fn encrypt(&self, counter: u64, data: &mut [u8]) {
        let iv = self.generate_iv(counter);
        cfb_mode::Encryptor::<Aes128>::new((&self.bright).into(), (&iv).into()).encrypt(data);
    }

    pub fn decrypt(&self, counter: u64, data: &mut [u8]) {
        let iv = self.generate_iv(counter);
        cfb_mode::Decryptor::<Aes128>::new((&self.bright).into(), (&iv).into()).decrypt(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lcg(seed: &mut u64) -> u8 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (*seed >> 33) as u8
    }
    fn rnd16(seed: &mut u64) -> [u8; 16] {
        let mut a = [0u8; 16];
        a.iter_mut().for_each(|b| *b = lcg(seed));
        a
    }

    #[test]
    fn cifrar_y_descifrar_es_reversible() {
        let mut s = 7u64;
        let rp = RpCrypt::init_auth(Target::Ps5_1, &rnd16(&mut s), &rnd16(&mut s));
        let original: Vec<u8> = (0..200u32).map(|i| (i * 7) as u8).collect();
        let mut buf = original.clone();
        rp.encrypt(42, &mut buf);
        assert_ne!(buf, original);
        rp.decrypt(42, &mut buf);
        assert_eq!(buf, original);
    }

    #[test]
    fn el_contador_cambia_el_resultado() {
        let mut s = 9u64;
        let rp = RpCrypt::init_auth(Target::Ps4_10, &rnd16(&mut s), &rnd16(&mut s));
        let (mut a, mut b) = ([5u8; 32], [5u8; 32]);
        rp.encrypt(1, &mut a);
        rp.encrypt(2, &mut b);
        assert_ne!(a, b);
    }

    #[test]
    fn rangos_invalidos_se_rechazan() {
        let amb = [0u8; 16];
        assert_eq!(aeropause(Target::Ps4_9, 0, &amb), Err(CryptError::InvalidTarget));
        assert_eq!(aeropause(Target::Ps5_1, 0x20, &amb), Err(CryptError::InvalidOffset));
        assert!(RpCrypt::init_regist(Target::Ps4_8, &amb, 0, 1).is_err());
    }

    /// Comparacion con la implementacion en C de referencia (solo como oraculo de pruebas).
    #[cfg(feature = "ref-c")]
    #[test]
    fn coincide_con_la_referencia_en_c() {
        use nitroplay_ref_sys as c;
        let targets = [Target::Ps4_8, Target::Ps4_9, Target::Ps4_10, Target::Ps5_1];
        let mut s = 12345u64;
        for round in 0..150 {
            let t = targets[round % 4];
            let nonce = rnd16(&mut s);
            let morning = rnd16(&mut s);
            let amb_in = rnd16(&mut s);
            let off = (lcg(&mut s) % 0x20) as usize;
            let pin = ((lcg(&mut s) as u32) << 24) | ((lcg(&mut s) as u32) << 16) | ((lcg(&mut s) as u32) << 8) | lcg(&mut s) as u32;
            unsafe {
                // bright / ambassador
                let (mut cb, mut ca) = ([0u8; 16], [0u8; 16]);
                c::chiaki_rpcrypt_bright_ambassador(t.code(), cb.as_mut_ptr(), ca.as_mut_ptr(), nonce.as_ptr(), morning.as_ptr());
                assert_eq!(bright_ambassador(t, &nonce, &morning), (cb, ca), "bright_ambassador {t:?}");

                if !t.pre10() {
                    let mut o = [0u8; 16];
                    assert_eq!(c::chiaki_rpcrypt_aeropause(t.code(), off, o.as_mut_ptr(), amb_in.as_ptr()), 0);
                    assert_eq!(aeropause(t, off, &amb_in).unwrap(), o, "aeropause {t:?}");
                    assert_eq!(c::chiaki_rpcrypt_aeropause_psn(t.code(), off, o.as_mut_ptr(), amb_in.as_ptr()), 0);
                    assert_eq!(aeropause_psn(t, off, &amb_in).unwrap(), o, "aeropause_psn {t:?}");
                    assert_eq!(c::chiaki_rpcrypt_ambassador_from_aeropause(t.code(), off, amb_in.as_ptr(), o.as_mut_ptr()), 0);
                    assert_eq!(ambassador_from_aeropause(t, off, &amb_in).unwrap(), o, "from_aeropause {t:?}");

                    // registro con PIN
                    let mut crp: c::ChiakiRPCrypt = std::mem::zeroed();
                    assert_eq!(c::chiaki_rpcrypt_init_regist(&mut crp, t.code(), amb_in.as_ptr(), off, pin), 0);
                    let rp = RpCrypt::init_regist(t, &amb_in, off, pin).unwrap();
                    assert_eq!((rp.bright, rp.ambassador), (crp.bright, crp.ambassador), "init_regist {t:?}");
                } else {
                    let mut o = [0u8; 16];
                    c::chiaki_rpcrypt_aeropause_ps4_pre10(o.as_mut_ptr(), amb_in.as_ptr());
                    assert_eq!(aeropause_ps4_pre10(&amb_in), o);
                    let mut crp: c::ChiakiRPCrypt = std::mem::zeroed();
                    c::chiaki_rpcrypt_init_regist_ps4_pre10(&mut crp, amb_in.as_ptr(), pin);
                    let rp = RpCrypt::init_regist_ps4_pre10(&amb_in, pin);
                    assert_eq!((rp.bright, rp.ambassador), (crp.bright, crp.ambassador));
                }

                // IV y cifrado
                let mut crp: c::ChiakiRPCrypt = std::mem::zeroed();
                c::chiaki_rpcrypt_init_auth(&mut crp, t.code(), nonce.as_ptr(), morning.as_ptr());
                let rp = RpCrypt::init_auth(t, &nonce, &morning);
                let counter = (round as u64) * 0x1_0000_0001;
                let mut iv = [0u8; 16];
                assert_eq!(c::chiaki_rpcrypt_generate_iv(&mut crp, iv.as_mut_ptr(), counter), 0);
                assert_eq!(rp.generate_iv(counter), iv, "iv {t:?}");
                let data: Vec<u8> = (0..77).map(|_| lcg(&mut s)).collect();
                let mut enc_c = vec![0u8; data.len()];
                assert_eq!(c::chiaki_rpcrypt_encrypt(&mut crp, counter, data.as_ptr(), enc_c.as_mut_ptr(), data.len()), 0);
                let mut enc_r = data.clone();
                rp.encrypt(counter, &mut enc_r);
                assert_eq!(enc_r, enc_c, "cifrado {t:?}");
            }
        }
    }
}
