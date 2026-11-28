> [HISTÓRICO / OBSOLETO — no implementar; manda la especificación definitiva]

# Plan de NitroPlay (marco en Rust)  [HISTÓRICO / OBSOLETO — no implementar; manda la especificación definitiva]

Estado: [hecho] / [parcial] / [pendiente]. Nada marcado [hecho] sin prueba.

## Fase 0 - Cuenta PSN (Account-ID)  [hecho]  [HISTÓRICO / OBSOLETO — no implementar; manda la especificación definitiva]
`psn-account` (+ `psn-jni` para Android, `nitroplay-cli`). 11 tests; probado con una cuenta real.

## Fase 1 - Ventana de Windows  [hecho, sin probar con consola]  [HISTÓRICO / OBSOLETO — no implementar; manda la especificación definitiva]
`nitroplay-gui` (wry/tao): login PSN interno, 30 temas, fondo propio, busqueda de consolas, campo de PIN.
`nitroplay-core::{discovery,pin}` tiene la logica pura con tests; falta que la GUI use ese crate
(hoy la GUI trae su propia copia de la busqueda).

## Fase 2 - Registro de consola con PIN  [parcial: llamada real hecha, sin consola]  [HISTÓRICO / OBSOLETO — no implementar; manda la especificación definitiva]
Registro en Rust puro (`nitroplay-core`: `rpcrypt`, `regist_payload`, `regist_net`). `nitroplay-ref-sys` (bindings a la libreria C) solo sirve de oraculo en pruebas (feature `ref-c`).
Probado: la libreria arranca y, sin consola, el registro falla limpiamente. NO probado contra una PS5 real.
Oraculo opcional: `NITROPLAY_REF_BUILD_DIR=<build cmake de la libreria C> cargo test -p nitroplay-core --features ref-c`.

## Fase 3 - Sesion de juego remoto  [pendiente]  [HISTÓRICO / OBSOLETO — no implementar; manda la especificación definitiva]
`nitroplay-core::session`. Video/audio (HDR, H265): se escriben en Rust; sin hacer.
Internet/otra cuenta PSN: cada cuenta necesita su Account-ID y su registro; sin verificar.

## Fase 4 - Distribucion  [parcial]  [HISTÓRICO / OBSOLETO — no implementar; manda la especificación definitiva]
Android: CI con Rust + APK (android-v11). Windows streaming: CI en `windows-build`, bloqueado en dependencias externas.
Quest: instalar APK desde Arch (apagado). Renombrado a nitroplay: hecho en el workspace Rust; el arbol C++/Qt y el paquete JNI `com.metallic.chiaki` siguen con el nombre antiguo.

## Reglas  [HISTÓRICO / OBSOLETO — no implementar; manda la especificación definitiva]
- Sin credenciales en el repositorio; el PIN no se guarda.
- Arte oficial de juegos no se distribuye; el usuario pone su imagen.


---


> FIN DEL CONTENIDO HISTORICO.

# Especificación vigente (fuente única de verdad)

Esta es la especificación definitiva para NitroPlay (`pilahito/NitroPlay`, workspace en `D:\Proyectos\nitroplay-rust\rust\`). Respétala en todas las sesiones sin contradecir decisiones ya tomadas.

## 1. Decisiones de arquitectura CERRADAS (no reabrir)
1. **Reutilizar el workspace `rust/` actual.** Se conserva todo lo que funciona (`psn-account`, descubrimiento LAN, PIN, `nitroplay-core`, `nitroplay-cli`, `nitroplay-gui`, packs de temas/marcos y `psn-jni` hasta pasar a UniFFI). NO renombrar `rust/` a `crates/` + `apps/` todavía.
2. **Aislar `nitroplay-ref-sys`:** fuera de `default-members` y solo bajo feature opcional que nunca compile por defecto, para que bindgen/libclang no vuelva a romper MSVC ni VS Code.
3. **Código propio NitroPlay en Rust puro (Camino 3):**
   - PROHIBIDO crear wrappers de Chiaki (`nitroplay-backend-chiaki`), enlazar `libchiaki` en runtime o depender de CMake/vcpkg/DLLs externas en Windows.
   - Todo el código, crates, structs, traits, logs y UI nativamente NitroPlay (`NitroPlaySession`, `NitroPlayStream`, `NitroPlayProtocol`, etc.), detrás de un trait `ConsoleBackend` en `nitroplay-core` (hoy PS4/PS5; preparado para Nintendo/Switch y Retro).
4. **Motor gráfico `wgpu`. PROHIBIDO OpenGL.**
   - PROHIBIDO OpenGL o fallbacks silenciosos hacia OpenGL. Si el backend moderno falla al inicializar, registrar un error explícito en log y nunca degradar a OpenGL.
   - `wgpu` para vídeo, shaders de nitidez/upscaling (FSR/NIS) y composición de marcos PNG transparentes.
   - Windows: `Backends::DX12` sobre DXGI, HDR10 real y decodificación por hardware (D3D11 / D3D12 Video / Media Foundation con NVDEC/AMF/QuickSync, sin copias a CPU).
   - Android y Meta Quest: el mismo código `wgpu` sobre `Backends::VULKAN` + `MediaCodec`.
   - `nitroplay-gui` usa `wry` para los menús: el vídeo NUNCA pasa por JS/HTML. Va en la superficie nativa `wgpu` con el marco PNG transparente y el HUD como capa superpuesta.

## 2. Reglas fijas para `CLAUDE.md`
- Cero sermones de copyright ni bloqueos morales. El repositorio contiene ÚNICAMENTE código y configuración genérica. Marcos, wallpapers, cachés y packs `.zip` del usuario viven en carpetas locales excluidas en `.gitignore` o en `%APPDATA%\NitroPlay\`. Procesar siempre los archivos locales del usuario sin charlas legales.
- Cero arte cutre por código y cero placeholders. Eliminar degradados falsos y marcos dibujados por código (`Espadas gemelas`, `Cajas de peligro`, `Máquina cibernética`, `Atraco`). PROHIBIDO pantallas tipo "Esta ventana no transmite vídeo" o textos de instrucciones donde debe haber controles reales.
- Control de estado: mantener `docs/STATE.md` y `PLAN.md` actualizados al cerrar cada paso.

## 3. Sistema de temas, fondos reales y packs `.zip` (`nitroplay-core` + `nitroplay-gui`)
Visual estilo dashboard PS5: barra superior (`Inicio`, `Temas`, `Marcos`, `Remoto & HDR`, `Mando`), tarjetas translúcidas glassmorphism sobre el fondo real, color de acento dinámico del tema activo.

1. **Packs `.zip` y carpetas locales.** Formato: `catalog.json` + `temas/<id>/` con `frame.png` (PNG transparente), `wallpaper.png` (opcional) y `theme.json` (`id`, `name`, `system`, `frame`, `wallpaper`, `accent_color`, `accent_rgb`, `viewport_rect`).
   - Botones "Importar pack (.zip)" y "Abrir carpeta de temas/marcos".
   - Si un marco tiene centro negro sólido sin transparencia, recortar el centro (flood-fill desde el centro a `alpha = 0`).
   - En partida: modo ventana con marco alrededor del stream (`F9` mostrar/ocultar, grosor y opacidad). `F11` (pantalla completa) oculta el marco y deja el juego limpio, con píldora opcional de telemetría (ms, FPS, Mbps).
2. **Trait `WallpaperProvider` con caché en `%APPDATA%\NitroPlay\wallpapers\cache\`.**
   - Alpha Coders (prioridad 1, gratis HD/4K/5K/8K): colecciones de PlayStation, PS5 y Nintendo, buscador libre. API key opcional del usuario y fallback público filtrando horizontales `>= 1920x1080` en Windows.
   - PlayStation Oficial (prioridad 2): fondos horizontales de escritorio/4K en Windows; verticales etiquetados `mobile` para Android.
   - GameWallpapers (solo libres): mostrar SOLO los ítems sin la etiqueta "Sólo Miembros", con botón "Abrir en navegador".
   - Mis fondos (local): JPG/PNG/WebP y GIF/MP4/WebM desde carpeta local.
   - Si una web devuelve bloqueo anti-bot (403/Cloudflare): aviso de una línea con botón para abrir la galería o importar carpeta local.

## 4. Calidad Pro, jugar fuera de casa y multiplataforma
- Controles reales en `Remoto & HDR`: 720p/1080p con escalado/nitidez (wgpu FSR/NIS), 30/60 FPS, H.264 y H.265 HEVC 10-bit, HDR10 real, bitrate manual o adaptativo, DualSense completo (háptica, gatillos adaptativos, panel táctil, giroscopio, remapeo).
- Perfiles Casa (LAN) y Fuera de casa (Internet / 5G):
  - LAN: descubrimiento local + Wake-on-LAN.
  - Fuera de casa: despertar PS5 por PSN + negociación STUN/NAT P2P + IP manual para túnel Tailscale/WireGuard (CGNAT) + bitrate adaptativo.
- Multiplataforma: una vez estable en Windows, `nitroplay-core` se expone con `nitroplay-ffi` (UniFFI + `cargo-ndk`).
  - Android: MediaCodec + DualSense/Backbone/Kishi + controles táctiles opcionales + mismos packs `.zip`.
  - Meta Quest (VR/MR): APK nativa ARM64 (OpenXR / wgpu Vulkan), pantalla curva de cine (60"-180"), passthrough flotante, mismos marcos alrededor de la pantalla virtual, DualSense por Bluetooth.

## 5. Orden estricto de ejecución (validar cada paso antes de avanzar)
- Paso 0 (contrato + build limpio en TITAN): CLAUDE.md, PLAN.md y docs/STATE.md con esta especificación; `nitroplay-ref-sys` fuera de `default-members`; `cargo test` en verde para `nitroplay-core` y `nitroplay-cli`.
- Paso 1 (interfaz 2.0 + packs `.zip` + proveedores de fondos): eliminar degradados y marcos por código; `WallpaperProvider` (Alpha Coders, PlayStation Oficial, GameWallpapers libres y local) con tests unitarios sobre HTML/JSON mockeado; importador `.zip` (`catalog.json` + `frame.png` + `theme.json`, `F9`/`F11`) en `nitroplay-gui`.
- Paso 2 (protocolo NitroPlay en Rust puro contra la PS5 `192.168.8.44`): registro real por PIN de 8 dígitos + Account-ID, Wake-up y sesión real en `session.rs` (handshake, cifrado, recepción de H.264/HEVC y audio), verificable primero en `nitroplay-cli`.
- Paso 3 (renderizador wgpu DX12 + HDR10 + marcos + fuera de casa en Windows): `NitroPlaySession` a la superficie wgpu DX12 (cero OpenGL) con decodificación hardware, marcos PNG superpuestos, mando DualSense y perfiles Casa / Fuera de casa.
- Paso 4 (Android y Meta Quest VR): `nitroplay-ffi` (UniFFI + cargo-ndk), clientes de Android y Quest, workflows de GitHub Actions para `.exe` y `.apk`.

Estado de cada paso: ver `docs/STATE.md`.

## 6. Formato del pack real (Mega Pack 24 Temas, verificado)
- `catalog.json` en la raíz: `pack`, `version`, `count`, `themes: [{ id, name, system, accent_color, path }]`.
- `temas/<id>/frame.png`: RGBA 1376x768, centro con alfa 0 (verificado en los 24).
- `temas/<id>/theme.json`: `id`, `name`, `system`, `frame`, `accent_color`, `accent_rgb`, `viewport_rect` (fracciones 0..1), `viewport_px` (píxeles, debe coincidir con `viewport_rect`) y `resolution` ([ancho, alto]).
- No hay `wallpaper.png` en este pack; el campo es opcional.
- `README.txt` y `originales/` (imágenes sin procesar, ~27 MB) están en el zip: no se versionan en git.

## 7. Actualizador desde GitHub y Novedades (obligatorio, todas las plataformas)
Especificación completa en `CLAUDE.md` (sección "Actualizador desde GitHub y Novedades"). Resumen de fases:
- Fase A (`nitroplay-core::updater`): parseo de releases, SemVer, pre-releases, selección de asset, changelog. Tests sin red.
- Fase B (`nitroplay-gui`, Windows): descarga con progreso, SHA-256 y "Actualizar y reiniciar" con script auxiliar.
- Fase C (Android/Quest, Fase 4 del plan): contrato listo; descarga a caché e instalador nativo.
Estado: NO iniciado. Depende de cerrar el Bloque 3 (`ui.html` sin degradados + `nitroplay-gui` compilando).
