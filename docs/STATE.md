# NitroPlay - Estado

Fuente de verdad: `rust/PLAN.md` (especificación vigente, sección "Especificación vigente") y `CLAUDE.md`.

## Rama activa
- `rust-paso0-titan` (commit local, pendiente de push). La rama pública de revisión es `rust-paso0-titan` en GitHub (`4d8850cf`).
- `rust-psn-account` en GitHub tiene 3 commits que TITAN no tiene (`b29b18df`, `6237a9bf`, `a7a923fd`). Decisión: `b29b18df` y `a7a923fd` descartados; de `6237a9bf` solo la parte de interfaz/renombrado, aplicada en Bloque 3.

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

## Bloque 3 - interfaz 2.0 en `nitroplay-gui` - [hecho, compila; pendiente de prueba manual del usuario]
- Verificado: `cargo test -p nitroplay-core -p nitroplay-cli` (29/29 en core, verde) y `cargo build -p nitroplay-gui` (MSVC, dev, sin errores).
- Ejecutable: `D:\Proyectos\nitroplay-rust\rust\target\debug\nitroplay.exe`.
- `ui.html`: sin degradados lineales (body, borde del fuego, CRT, fondo con velo). Marcos Espadas gemelas, Cajas de peligro, Máquina cibernética y Atraco retirados. Acento dinámico (`--acc`): el del pack activo si tiene `accent_color`, si no el del tema. Botón "Importar pack (.zip)" en Marcos.
- `main.rs`: usa `nitroplay_core::pack` (`ThemeJson`, `validate_theme`, `import_zip`). Packs importados en `%APPDATA%\NitroPlay\temas\<id>\`. Importación con selector de archivos `rfd`. `F9` marco en ventana, `F11` pantalla completa con marco oculto. Sin OpenGL.
- Dependencias nuevas en `nitroplay-gui`: `rfd = "0.15"`, `serde_json = "1"`.
- Cambio de comportamiento: la carpeta de marcos antigua (`%APPDATA%\NitroPlay\marcos\`) ya no se lee. Los packs hay que reimportarlos desde el `.zip`.
- `theme_pack.rs` NO eliminado: `pack.rs` y `main.rs` aún usan `theme_pack::valid_pack_id`. Retirarlo requiere mover esa función a `pack.rs`.
- Pendiente de prueba: importar `nitroplay-megapack-24-temas.zip` desde la GUI antes de hacer commit.

## Pendiente (no iniciado)
- `WallpaperProvider` (Alpha Coders, PlayStation Oficial, GameWallpapers solo libres, carpeta local).
- Actualizador (`updater.rs`) y vista "Actualizaciones y Novedades" (spec en `CLAUDE.md`).
- Paso 2: sesión real en `session.rs` (hoy stub `NotImplemented`). Sin probar contra la PS5 `192.168.8.44`.
- Paso 3: renderizador wgpu DX12 + HDR10. Paso 4: `nitroplay-ffi`, Android y Quest.

## Decisiones abiertas
- `rpcrypt_tables.rs`: se mantiene (opción a). Son claves del protocolo de Sony generadas desde `lib/src/rpcrypt.c` de chiaki-ng, que ya es público en el repo. La atribución AGPL de `lib.rs` se conserva.
- `Claude outputs/` excluido del repositorio (parche duplicado).

## Cambios sin confirmar en TITAN
- `rust/nitroplay-gui/src/main.rs`, `ui.html` y `Cargo.toml` modificados (Bloque 3). Sin commit hasta que el usuario pruebe el `.exe`.
- Log de pruebas `rust/pack_test.log` (ignorado por `.gitignore`).
