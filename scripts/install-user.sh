#!/usr/bin/env bash
set -euo pipefail

app_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
app_bin_dir="${HOME}/.local/bin"
app_data_root="${XDG_DATA_HOME:-${HOME}/.local/share}"
app_id="io.github.jangustavo.HwMonitor"
app_bin_path="${app_bin_dir}/hw-monitor-native"
app_desktop_path="${app_data_root}/applications/${app_id}.desktop"

cd "$app_root"
cargo build --locked --release --bin hw-monitor-native
install -Dm755 target/release/hw-monitor-native "$app_bin_path"
install -Dm644 static/icon-256.png \
  "${app_data_root}/icons/hicolor/256x256/apps/${app_id}.png"

python3 - "$app_bin_path" "$app_desktop_path" <<'PY'
from pathlib import Path
import sys

executable = sys.argv[1]
destination = Path(sys.argv[2])
if "\n" in executable or "\r" in executable:
    raise ValueError("Caminho inválido para a entrada Exec")
escaped = executable.replace("\\", "\\\\").replace('"', '\\"').replace("$", "\\$").replace("`", "\\`")
template = Path("packaging/io.github.jangustavo.HwMonitor.desktop").read_text()
destination.parent.mkdir(parents=True, exist_ok=True)
destination.write_text(template.replace("Exec=hw-monitor-native", f'Exec="{escaped}"'))
PY

echo "Instalado. Abra HW Monitor pelo menu de aplicativos."
