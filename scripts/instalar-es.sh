#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
[[ -x "$project_dir/build-es/bin/chiaki" ]] || { echo 'Falta compilar la aplicación.' >&2; exit 1; }
applications_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
desktop_path="$applications_dir/nitroplay.desktop"
backup_dir="${XDG_STATE_HOME:-$HOME/.local/state}/chiaki-ng-es/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$applications_dir" "$backup_dir"
for old in "$desktop_path" "$applications_dir/NitroPlay.desktop"; do
    if [[ -e "$old" ]]; then cp -p "$old" "$backup_dir/"; fi
done
# nombre antiguo del lanzador (la app ahora se identifica como «nitroplay»)
rm -f "$applications_dir/NitroPlay.desktop"
if [[ -d "${XDG_CONFIG_HOME:-$HOME/.config}/Chiaki" ]]; then
    cp -a "${XDG_CONFIG_HOME:-$HOME/.config}/Chiaki" "$backup_dir/"
fi
cat > "$desktop_path" <<EOF
[Desktop Entry]
Type=Application
Name=NitroPlay
Comment=Uso a distancia de PlayStation · Español y temas de juegos
Exec="$project_dir/scripts/iniciar-es.sh"
Icon=$project_dir/gui/res/nitroplay-icon.svg
Categories=Game;Network;RemoteAccess;
StartupWMClass=nitroplay
Terminal=false
EOF
if command -v desktop-file-validate >/dev/null; then desktop-file-validate "$desktop_path"; fi
if command -v update-desktop-database >/dev/null; then update-desktop-database "$applications_dir"; fi
printf '%s\n' 'Actualizado el icono de NitroPlay en el menú de aplicaciones.'
printf 'Copia de seguridad: %s\n' "$backup_dir"
