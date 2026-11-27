//! Nucleo de NitroPlay en Rust puro. Cada modulo es una fase del plan (ver ../PLAN.md).
//! Hecho: `discovery`, `pin`, `rpcrypt`, `regist_payload`, `regist_net` (registro; falta probarlo con una PS5 real).
//! Pendiente: `session` (vinculacion de streaming, video, audio, mando).
//!
//! Licencia: AGPL-3.0-only. Los modulos `discovery`, `rpcrypt`, `rpcrypt_tables`, `regist_payload` y `regist_net`
//! estan portados del protocolo documentado en chiaki-ng (AGPL-3.0-only con excepcion OpenSSL); se conserva su atribucion.
pub mod discovery;
pub mod pin;
pub mod regist;
pub mod regist_net;
pub mod regist_payload;
pub mod rpcrypt;
#[allow(clippy::all)]
mod rpcrypt_tables;
pub mod session;
pub mod theme_pack;
