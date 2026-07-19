#!/usr/bin/env bash
# Startet die lokale Hub-Testumgebung in drei getrennten Terminals:
# Hub sowie zwei Studio-Instanzen mit isolierten Datenverzeichnissen.

set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/branding.conf"
DATA_BASE="${STUDIO_DEMO_DATA_DIR:-$ROOT/local-data/hub-demo}"
HUB_DATA="$DATA_BASE/hub"
OFFICE_DATA="$DATA_BASE/office"
WORKSHOP_DATA="$DATA_BASE/workshop"

usage() {
    echo "Aufruf: $0 [--reset]"
    echo
    echo "  --reset  Löscht die lokalen Office-/Workshop-Testdaten vor dem Start."
}

case "${1:-}" in
    "") ;;
    --reset)
        if [[ -z "$DATA_BASE" || "$DATA_BASE" == "/" ]]; then
            echo "Unsicheres Testdaten-Verzeichnis: '$DATA_BASE'" >&2
            exit 1
        fi
        rm -rf -- "$HUB_DATA" "$OFFICE_DATA" "$WORKSHOP_DATA"
        ;;
    -h|--help)
        usage
        exit 0
        ;;
    *)
        usage >&2
        exit 2
        ;;
esac

mkdir -p -- "$HUB_DATA" "$OFFICE_DATA" "$WORKSHOP_DATA"

echo "Baue Hub und ${PRODUCT_NAME} …"
cargo build --manifest-path "$ROOT/Cargo.toml" -p hub -p studio

terminal_command() {
    local title="$1"
    local command="$2"
    local wrapped="$command; status=\$?; echo; echo '$title beendet (Code '\$status').'; exec bash"

    if command -v konsole >/dev/null 2>&1; then
        konsole --separate --workdir "$ROOT" -p "tabtitle=$title" \
            -e bash -lc "$wrapped" >/dev/null 2>&1 &
    elif command -v gnome-terminal >/dev/null 2>&1; then
        gnome-terminal --title="$title" --working-directory="$ROOT" -- \
            bash -lc "$wrapped" >/dev/null 2>&1 &
    elif command -v xfce4-terminal >/dev/null 2>&1; then
        xfce4-terminal --title="$title" --working-directory="$ROOT" \
            --command="bash -lc $(printf '%q' "$wrapped")" >/dev/null 2>&1 &
    elif command -v xterm >/dev/null 2>&1; then
        xterm -T "$title" -e bash -lc "$wrapped" &
    else
        echo "Kein unterstützter Terminal-Emulator gefunden." >&2
        echo "Benötigt: konsole, gnome-terminal, xfce4-terminal oder xterm." >&2
        exit 1
    fi
}

terminal_command "Hub" \
    "HUB_DATA_DIR='$HUB_DATA' '$ROOT/target/debug/hub'"

terminal_command "${PRODUCT_NAME} — Office" \
    "STUDIO_DATA_DIR='$OFFICE_DATA' '$ROOT/target/debug/studio'"

terminal_command "${PRODUCT_NAME} — Workshop" \
    "STUDIO_DATA_DIR='$WORKSHOP_DATA' '$ROOT/target/debug/studio'"

echo
echo "Testumgebung gestartet:"
echo "  Hub:   http://127.0.0.1:3737"
echo "  Ablage:   $HUB_DATA"
echo "  Office:   $OFFICE_DATA"
echo "  Workshop: $WORKSHOP_DATA"
echo
echo "Arbeitsplatznamen beim ersten Start in den Einstellungen setzen."
