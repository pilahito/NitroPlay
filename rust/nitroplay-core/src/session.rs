//! FASE 3 (sin implementar): sesion de juego remoto (red local, luego Internet por PSN).
//! La transmision de video/audio (HDR, H265) aun no esta implementada; se escribira en Rust
//! y no se dara por hecha hasta tener prueba real contra una PS5.

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
