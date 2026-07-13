#!/usr/bin/env bash
# Startet die lokale Charon-Testumgebung in drei getrennten Terminals:
# Charon sowie zwei LuxiFer-Instanzen mit isolierten Datenverzeichnissen.

set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_BASE="${LUXIFER_DEMO_DATA_DIR:-$ROOT/local-data/charon-demo}"
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
        rm -rf -- "$OFFICE_DATA" "$WORKSHOP_DATA"
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

mkdir -p -- "$OFFICE_DATA" "$WORKSHOP_DATA"

echo "Baue Charon und LuxiFer …"
cargo build --manifest-path "$ROOT/Cargo.toml" -p charon -p luxifer-native

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

terminal_command "Charon" \
    "'$ROOT/target/debug/charon'"

terminal_command "LuxiFer — Office" \
    "LUXIFER_DATA_DIR='$OFFICE_DATA' '$ROOT/target/debug/luxifer-native'"

terminal_command "LuxiFer — Workshop" \
    "LUXIFER_DATA_DIR='$WORKSHOP_DATA' '$ROOT/target/debug/luxifer-native'"

echo
echo "Testumgebung gestartet:"
echo "  Charon:   http://127.0.0.1:3737"
echo "  Office:   $OFFICE_DATA"
echo "  Workshop: $WORKSHOP_DATA"
echo
echo "Arbeitsplatznamen beim ersten Start in den Einstellungen setzen."
