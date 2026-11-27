//! FASE 2 (sin implementar): registro de la consola con PIN + Account-ID.
//! Referencia a portar: lib/src/regist.c y lib/src/rpcrypt.c de chiaki (puerto TCP 9295,
//! /sie/ps5/rp/sess/rgst y /sie/ps4/rp/sess/rgst). Aun no se ha escrito nada de la criptografia.

#[derive(Debug, PartialEq, Eq)]
pub enum RegistError {
    NotImplemented,
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
pub struct RegisteredConsole {
    pub regist_key: Vec<u8>,
    pub morning: Vec<u8>,
}

pub fn register(_req: &RegistRequest) -> Result<RegisteredConsole, RegistError> {
    Err(RegistError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_contrato_dice_la_verdad() {
        let r = RegistRequest { console_ip: "192.168.1.2", is_ps5: true, account_id_b64: "AAAAAAAAAAA=", pin: 12345678 };
        assert_eq!(register(&r).err(), Some(RegistError::NotImplemented));
    }
}
