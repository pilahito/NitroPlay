# NitroPlay

Cliente comunitario de juego remoto para PlayStation 4 y PlayStation 5, basado en chiaki-ng. Incluye la interfaz en español, nitidez ajustable en Configuración → Vídeo y 42 estilos visuales: 41 temas y el estilo NitroPlay original.

La pantalla de inicio usa la identidad NitroPlay y presenta cada consola en una tarjeta con acceso directo a «Jugar ahora». La conexión por Internet se muestra solo cuando pulsas «Acceso remoto» en la parte inferior; ocultarla no borra el emparejamiento. La búsqueda de consolas se activa con «Buscar consolas en la red». Las direcciones guardadas se consultan antes al iniciar (primer intento a los 100 ms, posteriores cada 500 ms).


## Temas

**Consolas:** PS5 clásico, PS4, PlayStation 3, PlayStation 2 y PlayStation 1 - clásico.

**Juegos:** Marvel's Spider-Man, God of War, Horizon, The Last of Us, Ghost of Tsushima, Astro Bot, Gran Turismo, Ratchet & Clank, Returnal, Uncharted, Bloodborne, Demon's Souls, Death Stranding, Crash Bandicoot, Spyro the Dragon, Metal Gear Solid, Final Fantasy VII, Tekken 3, Shadow of the Colossus, ICO, LittleBigPlanet, inFAMOUS, Killzone, Sackboy: A Big Adventure, Jak and Daxter, Sly Cooper, Days Gone, Detroit: Become Human, Persona 5, Resident Evil, Silent Hill, Kingdom Hearts, Stellar Blade, Helldivers 2, WipEout y PaRappa the Rapper.

**Original:** NitroPlay original.

En **Configuración → General → Tema** puedes previsualizar y elegir un estilo. La elección se guarda para el siguiente inicio. Los motivos ilustrados son propios, están inspirados en cada juego y solo aparecen en el menú de consolas; no cubren la transmisión.

## Abrir en Arch Linux

Desde la carpeta del repositorio, una vez compilado:

```bash
./scripts/iniciar-es.sh
```

Esta compilación nativa utiliza la configuración nativa de Chiaki (`~/.config/Chiaki`). La configuración de la aplicación Flatpak se guarda por separado; el lanzador no la migra. Para vincular la consola hace falta el *PSN Account-ID* (o el *PSN Online-ID* en una PS4 con firmware anterior a 7.0) y un PIN de Remote Play de 8 dígitos.

El lanzador selecciona español por defecto. Si abres el binario directamente, la traducción puede seguir el idioma del sistema.

## Compilar en Arch Linux

Con las dependencias de desarrollo de chiaki-ng y las herramientas Linguist de Qt 6 instaladas:

```bash
cmake -S . -B build-es/cmake -G Ninja -DCMAKE_BUILD_TYPE=Release -DCHIAKI_USE_SYSTEM_CURL=ON -DCHIAKI_ENABLE_STEAMDECK_NATIVE=OFF -DCMAKE_POLICY_VERSION_MINIMUM=3.5
cmake --build build-es/cmake --parallel
ctest --test-dir build-es/cmake --output-on-failure
mkdir -p build-es/bin build-es/lib
cp build-es/cmake/gui/chiaki build-es/bin/
cp build-es/cmake/third-party/cpp-steam-tools/libcpp-steam-tools.so build-es/lib/
./scripts/iniciar-es.sh
```

La compilación depende de las versiones locales de Qt y de las bibliotecas multimedia. Vuelve a compilar después de actualizarlas.

## Windows y macOS

Hay un flujo de compilación de Windows x64 en `.github/workflows/build-windows-x86.yml` que prepara un paquete portátil y un instalador de NitroPlay. El instalador usa un identificador propio para convivir con chiaki-ng y mantiene la configuración de emparejamiento existente. Aún no se ha ejecutado una compilación de Windows de esta revisión, por lo que su funcionamiento debe comprobarse en Windows antes de distribuirla. Para macOS tampoco hay paquete publicado. La guía general de compilación está en [docs/diy/buildit.md](docs/diy/buildit.md).

## HDR en PS5

En **Configuración → Vídeo** selecciona **Vulkan**; en **Transmisión → PS5** selecciona **H265 HDR** para la conexión local y, si lo necesitas, para la conexión por Internet. NitroPlay ya acepta el flujo HDR de la PS5 mediante H.265 y libplacebo adapta la imagen a SDR cuando la salida no es HDR. Para ver HDR real en pantalla deben admitirlo el monitor, el sistema gráfico, el controlador y la salida de vídeo. Esta revisión no fuerza HDR en monitores SDR ni asegura que Windows o Hyprland estén emitiendo en HDR. OpenGL ofrece H.264/H.265 sin HDR.

La documentación original de instalación de chiaki-ng (paquetes de Flathub, Homebrew o Releases) no sirve para instalar NitroPlay: esos paquetes no incluyen la interfaz en español, la nitidez ni los temas de esta versión.

## Créditos y licencia

NitroPlay se publica bajo la GNU Affero General Public License v3.0 (AGPL-3.0). Es un proyecto comunitario no oficial basado en chiaki-ng y en Chiaki. No está afiliado, patrocinado ni certificado por Sony Interactive Entertainment.

La guía principal, con requisitos, primeros pasos y solución de problemas, está en [README.md](README.md).
