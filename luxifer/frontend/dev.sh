#!/usr/bin/env bash
# LuxiFer im Dev-Modus starten.
#
# Unter Wayland braucht WebKitGTK diese Flags, sonst bleibt das Fenster leer
# oder der Prozess stirbt still beim Start (kein Fehler im Log).
set -e
cd "$(dirname "$0")"

export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1

npm run tauri dev
