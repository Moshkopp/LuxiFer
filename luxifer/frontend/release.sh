#!/usr/bin/env bash
# LuxiFer als optimiertes Release bauen und starten.
#
# Zweck: das echte Laufverhalten testen (Release ist um Größenordnungen
# schneller als der Dev-Modus — Debug-Zahlen zählen nicht). Gebaut wird NUR
# das Binary (--no-bundle), kein AppImage/deb — das spart die Paketierung.
#
# "falls nötig": neu gebaut wird nur, wenn eine Quelldatei (Rust-Core, das
# src-tauri-Crate oder das Svelte-Frontend) neuer ist als das vorhandene
# Binary. Ändert sich nichts, startet das bestehende Binary sofort.
#
# GDK_BACKEND=x11: gleiche Begründung wie in dev.sh — unter nativem Wayland
# zwingt WebKitGTK das Present über einen langsamen Software-Pfad; über XWayland
# rendert das Fenster mit vollem HW-Compositing und deutlich geringerer Latenz.
set -euo pipefail
cd "$(dirname "$0")"

BIN="src-tauri/target/release/luxifer-app"

# Ist ein Rebuild nötig? Ja, wenn das Binary fehlt oder irgendeine Quelldatei
# jünger ist. Wir betrachten den Rust-Core, das src-tauri-Crate und die
# Frontend-Quellen (das Frontend wird über beforeBuildCommand mitgebaut).
needs_build() {
  [ ! -x "$BIN" ] && return 0
  # Neueste Änderung unter den Quellverzeichnissen finden (ohne Build-Artefakte).
  local newest
  newest=$(find ../core/src src-tauri/src src-tauri/Cargo.toml \
                src/ package.json vite.config.ts index.html \
             -type f -newer "$BIN" -print -quit 2>/dev/null || true)
  [ -n "$newest" ] && return 0
  return 1
}

if needs_build; then
  echo "» Quellen geändert — baue Release-Binary (tauri build --no-bundle) …"
  npm run tauri build -- --no-bundle
else
  echo "» Binary ist aktuell — überspringe Build."
fi

echo "» Starte $BIN"
export GDK_BACKEND=x11
exec "$BIN" "$@"
