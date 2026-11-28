# NitroPlay - Estado

Fuente de verdad: `rust/PLAN.md` (especificación vigente, sección "Especificación vigente") y `CLAUDE.md`.

## Rama activa
- `rust-paso0-titan` en `597c47b2` (pusheado). Cambios de esta ronda sin commit hasta que el usuario los pruebe.
- `rust-psn-account` en GitHub tiene 3 commits que TITAN no tiene (`b29b18df`, `6237a9bf`, `a7a923fd`). Decisión: `b29b18df` y `a7a923fd` descartados; de `6237a9bf` solo la parte de interfaz/renombrado, aplicada en Bloque 3.

## Paso 0 - [hecho]
- `nitroplay-ref-sys` fuera de `default-members`.
- Test `consola_simulada_en_local` corregido (lee el cuerpo antes de responder).
- Pre-release `v0.1.0-pre.0` apuntando a `4d8850cf`.

## Limpieza de documentación - [hecho]
- `rust/PLAN.md`: secciones anteriores a "Especificación vigente" marcadas como `[HISTÓRICO / OBSOLETO]`.
- `rust/PLAN.md` sección 6: formato real del Mega Pack 24 Temas (verificado en los 24).
- Especificación del actualizador (GitHub Releases + Novedades) en `CLAUDE.md` y `rust/PLAN.md` sección 7.

## Paso 1 - interfaz 2.0 y fondos

### Bloque 1 - importador `.zip` (`nitroplay-core::pack`) - [hecho, probado]
- Parser de `catalog.json` y `theme.json` con `viewport_rect`, `viewport_px`, `resolution`, `accent_rgb`, `wallpaper` opcional.
- Validación previa a escribir: id seguro, `accent_color` coherente con `accent_rgb`, rectángulo en 0..1, `viewport_px` consistente.
- Importación: solo `temas/<id>/`; ignora `originales/`, `README.txt` y `catalog.json`; rechaza `../`; límites de tamaño; un tema inválido se descarta entero.
- Fusión: un zip con solo `wallpaper.*` añade el fondo a un tema existente.
- `valid_pack_id` movido a `pack.rs`. `theme_pack.rs` eliminado (ya no lo usa nada).
- No incluido: auto-recorte de centro negro (requiere crate `image`, aplazado).

### Bloque 3 - interfaz 2.0 en `nitroplay-gui` - [hecho, probado por el usuario en TITAN]
- Commit `597c47b2`.
- Sin degradados lineales, sin marcos dibujados por código, acento dinámico, importador `.zip` desde la GUI (`%APPDATA%\NitroPlay\temas\`), F9 y F11 funcionando.
- Carpeta antigua `marcos\` ya no se lee: hay que reimportar los packs.

### Núcleo del actualizador (`nitroplay-core::updater`) - [hecho, 42/42 tests verdes, sin red]
- `RELEASES_API` apunta a `pilahito/NitroPlay`. `User-Agent: NitroPlay/<version>`.
- Parseo de releases: descarta borradores; pre-releases solo con `include_prerelease`; orden SemVer descendente.
- SemVer: estable > pre-release; `alpha.2` < `alpha.10`; prefijo `v` admitido.
- Selección de asset: Windows `*windows*x64*.exe|.zip`, Android `*android*arm64*.apk`, Quest `*quest*arm64*.apk`. Comodín `*` sin distinguir mayúsculas.
- `SHA256SUMS.txt`: parseo y verificación SHA-256 en la descarga (`net::download_file`).
- Script `.cmd` para Windows: espera al PID, sustituye el ejecutable y reabre la app. Rechaza rutas con comillas o `%`.
- Errores legibles en español; 403/429 se tratan como límite de peticiones.

### Proveedores de fondos (`nitroplay-core::wallpapers`) - [hecho, 8 tests con HTTP simulado]
- Trait `WallpaperProvider`: Alpha Coders, PlayStation Oficial, GameWallpapers (descarta "Sólo Miembros"), carpeta local.
- Filtro de destino: escritorio = horizontal >= 1920x1080; móvil = vertical.
- Detección de bloqueo anti-bot (403/429/503 o página de Cloudflare) → `ProviderError::Blocked`.
- Caché en disco por SHA-256 de la URL, comprobando firma de imagen (JPEG/PNG/WebP).
- Carpeta local: jpg/jpeg/png/webp.
- AVISO: los parsers son sobre HTML de ejemplo (fixtures). La estructura real de los tres sitios NO está verificada en vivo. Hay que probarlo contra las páginas reales antes de confiar en ellos.
- Alpha Coders: sin clave de API en esta ronda (solo el fallback público).
- Pendiente: el filtro de miembros se aplica a todos los proveedores, no solo a GameWallpapers.

## Pendiente (próxima ronda)
- Conectar el actualizador a `nitroplay-gui`: sección "Actualizaciones y Novedades", interruptor de pre-releases, progreso de descarga, "Actualizar y reiniciar" (usa `windows_update_script`) y aviso en la barra superior.
- Conectar `WallpaperProvider` a la pestaña Temas: buscador, selector de proveedor, "Abrir galería" si hay bloqueo, y aplicación del fondo desde la caché.
- Verificar en vivo los parsers de los tres sitios.
- Warning pendiente: `use crate::net::{self, ...}` sin uso en `wallpapers.rs` (quitar `self`).
- Paso 2: sesión real en `session.rs` (hoy stub `NotImplemented`). Sin probar contra la PS5 `192.168.8.44`.
- Paso 3: renderizador wgpu DX12 + HDR10. Paso 4: `nitroplay-ffi`, Android y Quest.

## Decisiones abiertas
- `rpcrypt_tables.rs`: se mantiene (opción a). Son claves del protocolo de Sony generadas desde `lib/src/rpcrypt.c` de chiaki-ng, que ya es público en el repo. La atribución AGPL de `lib.rs` se conserva.
- `Claude outputs/` excluido del repositorio (parche duplicado).

## Cambios sin confirmar en TITAN
- `rust/nitroplay-core/`: `net.rs`, `updater.rs`, `wallpapers.rs` nuevos; `pack.rs`, `lib.rs`, `Cargo.toml` modificados (añade `ureq`); `theme_pack.rs` eliminado.
- Sin commit ni push hasta que el usuario pruebe.
