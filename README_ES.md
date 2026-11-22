# PS5 Remoto: versión en español y temas PlayStation

PS5 Remoto es una versión comunitaria basada en chiaki-ng. Incluye la interfaz en español (710 traducciones), nitidez ajustable en los ajustes de vídeo y 16 estilos visuales en total: 15 temas y el estilo original.

## Temas

**Consolas:** PS5 clásico y PS4.

**Juegos:** God of War, Marvel's Spider-Man, Horizon, The Last of Us, Ghost of Tsushima, Astro Bot, Gran Turismo, Ratchet & Clank, Returnal, Uncharted, Bloodborne, Demon's Souls y Death Stranding.

**Original:** chiaki-ng.

En **Ajustes > General > Tema** puedes previsualizar y elegir un estilo. La elección se guarda para el siguiente inicio. Los motivos vectoriales son ilustraciones propias inspiradas en cada juego y solo aparecen en el menú de consolas; no cubren la transmisión.

## Abrir en Arch Linux

Desde esta carpeta:

```bash
./scripts/iniciar-es.sh
```

Esta compilación nativa utiliza la configuración nativa de Chiaki. La configuración de la aplicación Flatpak se guarda por separado; el lanzador no la migra. La vinculación de la consola sigue requiriendo el Account-ID de PSN y un PIN válido.

## Compilar

Con las dependencias de desarrollo de chiaki-ng y Qt 6 Linguist Tools instaladas:

```bash
cmake -S . -B build-es/cmake -G Ninja -DCMAKE_BUILD_TYPE=Release -DCHIAKI_USE_SYSTEM_CURL=ON -DCHIAKI_ENABLE_STEAMDECK_NATIVE=OFF -DCMAKE_POLICY_VERSION_MINIMUM=3.5
cmake --build build-es/cmake --parallel 6
ctest --test-dir build-es/cmake --output-on-failure
mkdir -p build-es/bin build-es/lib
cp build-es/cmake/gui/chiaki build-es/bin/
cp build-es/cmake/third-party/cpp-steam-tools/libcpp-steam-tools.so build-es/lib/
./scripts/iniciar-es.sh
```

La compilación depende de las versiones locales de Qt y las bibliotecas multimedia. Recompila después de actualizarlas. La traducción también puede seguir el idioma del sistema al abrir el binario directamente; el lanzador selecciona español por defecto.
