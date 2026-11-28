# NitroPlay - Estado

Fuente de verdad: `rust/PLAN.md` (especificación vigente, sección "Especificación vigente") y `CLAUDE.md`.

## Rama activa
- `rust-paso0-titan` (commit local, pendiente de push). La rama pública de revisión es `rust-paso0-titan` en GitHub (`4d8850cf`).
- `rust-psn-account` en GitHub tiene 3 commits que TITAN no tiene (`b29b18df`, `6237a9bf`, `a7a923fd`). Decisión: `b29b18df` y `a7a923fd` descartados; de `6237a9bf` solo la parte de interfaz/renombrado, pendiente en Bloque 3.

## Paso 0 - [hecho]
- `nitroplay-ref-sys` fuera de `default-members`.
- Test `consola_simulada_en_local` corregido (lee el cuerpo antes de responder).
- Pre-release `v0.1.0-pre.0` apuntando a `4d8850cf`.

## Limpieza de documentación - [hecho]
- `rust/PLAN.md`: secciones anteriores a "Especificación vigente" marcadas como `[HISTÓRICO / OBSOLETO]`.
- `rust/PLAN.md` sección 6: formato real del Mega Pack 24 Temas (verificado en los 24).
- Especificación del actualizador (GitHub Releases + Novedades) añadida a `CLAUDE.md` y `rust/PLAN.md` sección 7.

## Bloque 1 - importador `.zip` (`nitroplay-core::pack`) - [hecho, 29/29 tests]
- Parser de `catalog.json` y `theme.json` (serde) con los campos del pack: `viewport_rect`, `viewport_px`, `resolution`, `accent_rgb`, `wallpaper` opcional.
- Validación previa a escribir: id seguro, `accent_color` hex coherente con `accent_rgb`, rectángulo en 0..1, `viewport_px` consistente con `viewport_rect` × `resolution`.
- Importación: solo `temas/<id>/`; ignora `originales/`, `README.txt` y `catalog.json`; rechaza rutas `../`; límites de tamaño; un tema inválido se descarta entero.
- Fusión: un zip con solo `wallpaper.*` añade el fondo a un tema existente.
- No incluido: auto-recorte de centro negro (requiere crate `image`, aplazado).

## Bloque 3 - interfaz 2.0 en `nitroplay-gui` - [pendiente, siguiente ronda]
- `ui.html`: quitar los 5 degradados lineales; quitar marcos dibujados por código; aplicar la parte de interfaz de `6237a9bf`; color de acento desde `accent_color`; botón "Importar pack (.zip)".
- `main.rs`: conectar `nitroplay_core::pack` y retirar `theme_pack.rs` si ya no se usa; `F9` marco en ventana, `F11` pantalla completa sin marco; sin OpenGL.
- Verificación: `cargo test -p nitroplay-core -p nitroplay-cli` y `cargo build -p nitroplay-gui` (no ejecutado aún).

## Pendiente (no iniciado)
- `WallpaperProvider` (Alpha Coders, PlayStation Oficial, GameWallpapers solo libres, carpeta local).
- Actualizador (`updater.rs`) y vista "Actualizaciones y Novedades" (spec en `CLAUDE.md`).
- Paso 2: sesión real en `session.rs` (hoy stub `NotImplemented`). Sin probar contra la PS5 `192.168.8.44`.
- Paso 3: renderizador wgpu DX12 + HDR10. Paso 4: `nitroplay-ffi`, Android y Quest.

## Decisiones abiertas
- `rpcrypt_tables.rs`: se mantiene (opción a). Son claves del protocolo de Sony generadas desde `lib/src/rpcrypt.c` de chiaki-ng, que ya es público en el repo. La atribución AGPL de `lib.rs` se conserva.
- `Claude outputs/` excluido del repositorio (parche duplicado).

## Cambios sin confirmar en TITAN
- Ninguno tras el commit de este cierre, salvo el log de pruebas `rust/pack_test.log` (ignorado por `.gitignore`).
