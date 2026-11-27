//! FASE 1: registro de la consola con PIN + Account-ID, sobre libchiaki (feature `chiaki`).
//! Sin la feature, `register` devuelve `NotImplemented` y los tests de logica pura siguen funcionando.

#[derive(Debug, PartialEq, Eq)]
pub enum RegistError {
    NotImplemented,
    /// El Account-ID no es base64 de exactamente 8 bytes.
    BadAccountId,
    /// La direccion de la consola contiene un caracter nulo.
    BadHost,
    /// libchiaki no pudo arrancar el registro (codigo de ChiakiErrorCode).
    Start(i32),
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

#[cfg(not(feature = "chiaki"))]
pub fn register(_req: &RegistRequest) -> Result<RegisteredConsole, RegistError> {
    Err(RegistError::NotImplemented)
}

#[cfg(feature = "chiaki")]
pub fn register(req: &RegistRequest) -> Result<RegisteredConsole, RegistError> {
    use chiaki_sys as c;
    use std::ffi::{c_void, CString};
    use std::sync::mpsc;
    use std::time::Duration;

    enum Outcome {
        Done(RegisteredConsole),
        Failed,
        Canceled,
    }

    unsafe extern "C" fn on_event(event: *mut c::ChiakiRegistEvent, user: *mut c_void) {
        let tx = &*(user as *const mpsc::Sender<Outcome>);
        let ev = &*event;
        let out = if ev.type_ == c::chiaki_regist_event_type_t_CHIAKI_REGIST_EVENT_TYPE_FINISHED_SUCCESS && !ev.registered_host.is_null() {
            let h = &*ev.registered_host;
            let name = std::ffi::CStr::from_ptr(h.server_nickname.as_ptr()).to_string_lossy().into_owned();
            let mut regist_key = [0u8; 16];
            for (d, s) in regist_key.iter_mut().zip(h.rp_regist_key.iter()) {
                *d = *s as u8;
            }
            Outcome::Done(RegisteredConsole { nickname: name, server_mac: h.server_mac, rp_regist_key: regist_key, rp_key: h.rp_key, rp_key_type: h.rp_key_type })
        } else if ev.type_ == c::chiaki_regist_event_type_t_CHIAKI_REGIST_EVENT_TYPE_FINISHED_CANCELED {
            Outcome::Canceled
        } else {
            Outcome::Failed
        };
        let _ = tx.send(out);
    }

    unsafe extern "C" fn on_log(_level: c::ChiakiLogLevel, _msg: *const std::os::raw::c_char, _user: *mut c_void) {}

    let account = decode_account_id(req.account_id_b64).ok_or(RegistError::BadAccountId)?;
    let host = CString::new(req.console_ip).map_err(|_| RegistError::BadHost)?;
    let (tx, rx) = mpsc::channel::<Outcome>();
    let tx_ptr = &tx as *const mpsc::Sender<Outcome> as *mut c_void;

    unsafe {
        c::chiaki_lib_init();
        let mut log: c::ChiakiLog = std::mem::zeroed();
        c::chiaki_log_init(&mut log, 31, Some(on_log), std::ptr::null_mut()); // 31 = todos los niveles (CHIAKI_LOG_ALL)
        let mut info: c::ChiakiRegistInfo = std::mem::zeroed();
        info.target = if req.is_ps5 { c::ChiakiTarget_CHIAKI_TARGET_PS5_1 } else { c::ChiakiTarget_CHIAKI_TARGET_PS4_10 };
        info.host = host.as_ptr();
        info.broadcast = false;
        info.psn_online_id = std::ptr::null();
        info.psn_account_id = account;
        info.pin = req.pin;
        info.holepunch_info = std::ptr::null_mut();
        let mut regist: c::ChiakiRegist = std::mem::zeroed();
        let err = c::chiaki_regist_start(&mut regist, &mut log, &info, Some(on_event), tx_ptr);
        if err != c::ChiakiErrorCode_CHIAKI_ERR_SUCCESS {
            return Err(RegistError::Start(err as i32));
        }
        let res = rx.recv_timeout(Duration::from_secs(30));
        c::chiaki_regist_stop(&mut regist);
        c::chiaki_regist_fini(&mut regist);
        match res {
            Ok(Outcome::Done(h)) => Ok(h),
            Ok(Outcome::Failed) => Err(RegistError::Failed),
            Ok(Outcome::Canceled) => Err(RegistError::Canceled),
            Err(_) => Err(RegistError::Timeout),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_id_de_ejemplo_decodifica_a_8_bytes() {
        let b = decode_account_id("W29FIiEkolw=").unwrap();
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

    #[cfg(not(feature = "chiaki"))]
    #[test]
    fn sin_la_feature_el_contrato_dice_la_verdad() {
        let r = RegistRequest { console_ip: "192.168.1.2", is_ps5: true, account_id_b64: "AAAAAAAAAAA=", pin: 12345678 };
        assert_eq!(register(&r).err(), Some(RegistError::NotImplemented));
    }

    /// Sin consola en esa direccion, el registro debe fallar limpiamente, no colgarse ni romper.
    #[cfg(feature = "chiaki")]
    #[test]
    fn sin_consola_falla_sin_colgarse() {
        let r = RegistRequest { console_ip: "127.0.0.1", is_ps5: true, account_id_b64: "W29FIiEkolw=", pin: 12345678 };
        let res = register(&r);
        assert!(matches!(res, Err(RegistError::Failed) | Err(RegistError::Timeout) | Err(RegistError::Start(_))), "{res:?}");
    }
}
