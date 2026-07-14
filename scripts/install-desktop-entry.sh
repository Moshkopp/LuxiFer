#!/usr/bin/env bash
# Installiert luxifer.desktop + Icon ins Nutzer-Profil, damit der Compositor
# (Wayland: app_id "luxifer" → luxifer.desktop) das App-Icon in Taskleiste und
# Fensterwechsler anzeigt — auch beim direkten Start des Release-Binaries.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="$ROOT_DIR/target/release/luxifer-native"
ICON_SRC_DIR="$ROOT_DIR/luxifer/native/assets/icon"
ICON_BASE="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor"
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"

# Größenspezifische Fassungen (16–32 minimal, ab 48 mit Flügeln) — die Shell
# wählt automatisch die passende Stufe für Taskleiste, Fensterwechsler usw.
for size in 16 24 32 48 64 128 256 512; do
  install -Dm644 "$ICON_SRC_DIR/luxifer-$size.png" \
    "$ICON_BASE/${size}x${size}/apps/luxifer.png"
done

mkdir -p "$APP_DIR"
cat >"$APP_DIR/luxifer.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=LuxiFer
Comment=Nativer Editor für Laserprojekte
Exec=$BINARY
Icon=luxifer
Terminal=false
Categories=Graphics;Engineering;
StartupWMClass=luxifer
EOF

# Icon-Cache auffrischen, falls die Tools da sind (sonst reicht Neuanmeldung).
command -v update-desktop-database >/dev/null && update-desktop-database "$APP_DIR" || true
command -v gtk-update-icon-cache >/dev/null && \
  gtk-update-icon-cache -q "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor" || true

echo "» Installiert: $APP_DIR/luxifer.desktop (Icons: $ICON_BASE/<größe>/apps/luxifer.png)"
