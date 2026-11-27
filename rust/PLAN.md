# Plan de NitroPlay (marco en Rust)

Estado: [hecho] / [parcial] / [pendiente]. Nada marcado [hecho] sin prueba.

## Fase 0 - Cuenta PSN (Account-ID)  [hecho]
`psn-account` (+ `psn-jni` para Android, `nitroplay-cli`). 11 tests; probado con una cuenta real.

## Fase 1 - Ventana de Windows  [hecho, sin probar con consola]
`nitroplay-gui` (wry/tao): login PSN interno, 30 temas, fondo propio, busqueda de consolas, campo de PIN.
`nitroplay-core::{discovery,pin}` tiene la logica pura con tests; falta que la GUI use ese crate
(hoy la GUI trae su propia copia de la busqueda).

## Fase 2 - Registro de consola con PIN  [pendiente]
`nitroplay-core::regist` solo define el contrato. Hay que portar `lib/src/regist.c` y `lib/src/rpcrypt.c`
de chiaki (TCP 9295). Criterio de cierre: registrar una PS5 real. Hasta entonces, el PIN se escribe en Android.

## Fase 3 - Sesion de juego remoto  [pendiente]
`nitroplay-core::session`. Video/audio (HDR, H265) siguen en el nucleo chiaki-ng.
Internet/otra cuenta PSN: cada cuenta necesita su Account-ID y su registro; sin verificar.

## Fase 4 - Distribucion  [parcial]
Android: CI con Rust + APK (android-v11). Windows streaming: CI en `windows-build`, bloqueado en dependencias externas.
Quest: instalar APK desde Arch (apagado). Renombrado chiaki->nitroplay: parcial.

## Reglas
- Sin credenciales en el repositorio; el PIN no se guarda.
- Arte oficial de juegos no se distribuye; el usuario pone su imagen.
