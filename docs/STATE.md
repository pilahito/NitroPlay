# NitroPlay - Estado

Fuente de verdad del proyecto: `rust/PLAN.md` (especificación vigente al final de ese archivo).

## Rama activa
- `rust-psn-account` (en GitHub: `a7a923fd`). No se toca `main`.

## Paso 0 (contrato + build limpio) - [hecho]
- `nitroplay-ref-sys` fuera de `default-members` (en `rust/Cargo.toml`); sigue en `members` y solo se usa con la feature `ref-c`.
- `cargo test -p nitroplay-core -p nitroplay-cli`: 20/20 en `nitroplay-core`, 0 tests en `nitroplay-cli`. Tres pasadas seguidas en verde.
- Test corregido: `regist_net::tests::consola_simulada_en_local` fallaba de forma intermitente (10054). La consola simulada cerraba el socket sin leer el cuerpo de la petición. Ahora lee el cuerpo completo (`Content-Length`) antes de responder.

## Funciona (con prueba)
- Descubrimiento LAN (tests de paquetes, encendido y reposo).
- Validación de PIN de 8 dígitos.
- Account-ID: decodificación y rechazo de valores inválidos.
- Cifrado `rpcrypt`: ida y vuelta, contador y rangos.
- Packs de temas: fracciones, saneado de texto, rectángulo fuera de rango, carpeta sin JSON.
- Registro contra consola simulada en 127.0.0.1 (transporte y descifrado). **No demuestra que una PS5 real acepte el registro.**

## Pendiente
- Paso 1: interfaz 2.0, importador `.zip`, proveedores de fondos, F9/F11. No empezado.
- Paso 2: sesión real en `nitroplay-core/src/session.rs` (hoy es un stub que devuelve `NotImplemented`). Sin probar contra la PS5 `192.168.8.44`.
- Paso 3: renderizador wgpu DX12 + HDR10. No empezado.
- Paso 4: `nitroplay-ffi` (UniFFI + cargo-ndk), Android y Quest. No empezado.

## Cambios sin confirmar en TITAN
- Los cambios staged del usuario (incluido `rpcrypt_tables.rs`) van en el commit de pre-release `v0.1.0-pre.0`. Excluidos a propósito: `Claude outputs/` (parche duplicado) y los logs de prueba.

## Limpieza de rust/PLAN.md y formato del pack (pre-release Paso 0)
- Contenido anterior a "Especificación vigente" marcado como `[HISTÓRICO / OBSOLETO]` (aviso inicial y cada encabezado). No implementar nada de esas secciones.
- Sección 6 añadida a PLAN.md: formato real del Mega Pack 24 Temas (catalog.json, frame.png 1376x768 con centro alfa 0 verificado en los 24, theme.json con viewport_px y resolution, sin wallpaper.png).
- `originales/` y `README.txt` del zip no se versionan.
