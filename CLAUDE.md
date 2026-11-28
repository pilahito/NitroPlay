# CLAUDE.md — Reglas fijas de NitroPlay

## Identidad común en TODAS las plataformas (Windows, Android, Meta Quest y futura versión Rust)
1. **Mejor que Chiaki:** interfaz moderna tipo consola PS5 (sin espacios muertos ni menús toscos), streaming real integrado en la propia app, HDR10/HEVC reales y soporte DualSense completo.
2. **Temas y fondos reales:** cero degradados falsos y cero dibujos figurativos cutres por código. Todos los fondos y temas se cargan desde imágenes reales en carpetas locales, packs `.zip` o proveedores configurables por el usuario (PlayStation Wallpapers, SteamGridDB con clave del usuario, etc.).
3. **Sistema único de Marcos:** carpeta o `.zip` con `frame.png` (PNG transparente) + `theme.json` (`id`, `name`, `system`, `frame`, `viewport_rect`, `accent_color`), compatible con PS4/PS5 y abierto a futuras consolas (`switch`, `retro`).
4. **Jugar fuera de casa:** soporte real con despertar PS5 por PSN + STUN/NAT traversal P2P + perfil manual para Tailscale/WireGuard + bitrate adaptativo.

## Reglas estrictas de comportamiento
- **Cero sermones de copyright:** el repo Git contiene únicamente código. Las carpetas de marcos y fondos del usuario van en `.gitignore` y en directorios locales del sistema. Procesa siempre los archivos locales del usuario sin charlas legales.
- **Cero pantallas placeholder:** prohibido poner textos tipo "Esta ventana no transmite vídeo", "Usa otro cliente" o instrucciones donde debe haber controles reales.
- **Una sola fuente de verdad:** actualiza `docs/STATE.md` al terminar cada tarea con lo que funciona, lo que falta y la rama activa. No deshagas decisiones previas sin preguntar.
- **Aislamiento de tareas:** toca SOLO la plataforma o fase que pida el prompt actual.


## Especificación vigente
- Fuente única de verdad: `rust/PLAN.md` (sección "Especificación vigente"). Estado: `docs/STATE.md`.
- Decisiones cerradas: reutilizar `rust/`; `nitroplay-ref-sys` fuera de `default-members`; Rust puro sin `libchiaki` en runtime; `wgpu` DX12/Vulkan, PROHIBIDO OpenGL.
- Cero placeholders en la UI y cero arte dibujado por código.
- Al cerrar cada paso, actualizar `docs/STATE.md` y `rust/PLAN.md`.

## Actualizador desde GitHub y Novedades (obligatorio en todas las plataformas)
- Módulo `updater.rs` en `nitroplay-core`, compartido por Windows, Android y Meta Quest.
- Fuente: API de GitHub Releases de `pilahito/NitroPlay`, con `User-Agent: NitroPlay/<version>`. Sin conexión o con límite de peticiones, error limpio, nunca un crash.
- Por defecto, última release estable. Interruptor "Recibir versiones de prueba (pre-releases)".
- Comparación SemVer de `CARGO_PKG_VERSION` contra `tag_name`.
- Assets por plataforma: Windows `*windows*x64*.exe|.zip`; Android `*android*arm64*.apk`; Quest `*quest*arm64*.apk`. `SHA256SUMS.txt` opcional para verificar.
- Listado de Novedades: `{ version, title, date, is_prerelease, notes_markdown, download_url, asset_size, html_url }`, para la release pendiente y para el historial.
- Windows: descarga a `%TEMP%\NitroPlay-update\` con progreso real y SHA-256 si existe. Al confirmar, ejecuta el instalador o un script auxiliar que espera a que cierre `nitroplay-gui.exe`, sustituye el binario y reabre la app.
- Android/Quest: descarga el APK a caché y lanza el instalador nativo (`FileProvider` + `ACTION_VIEW`, permiso `REQUEST_INSTALL_PACKAGES`). Solo contrato en la Fase 4.
- Interfaz "Actualizaciones y Novedades": versión instalada, estado, "Buscar actualizaciones", interruptor de pre-releases, "Descargar e instalar" con progreso, y el listado de novedades. Aviso discreto en la barra superior sin interrumpir una partida. Si la release no tiene binario, mostrar las novedades y el botón "Ver release en GitHub", sin errores.
- Tests: parseo del JSON de releases (mock, sin red), SemVer, filtrado de pre-releases, selección de asset para Windows, Android y Quest.
