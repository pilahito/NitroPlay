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
