//! FASE 3 (sin implementar): sesion de juego remoto (red local, luego Internet por PSN).
//! La transmision de video/audio (HDR, H265) sigue en el nucleo chiaki-ng hasta que
//! esta fase tenga prueba real contra una PS5.

#[derive(Debug, PartialEq, Eq)]
pub enum Route {
    LocalNetwork,
    InternetViaPsn,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SessionError {
    NotImplemented,
}

pub fn start(_route: Route) -> Result<(), SessionError> {
    Err(SessionError::NotImplemented)
}
