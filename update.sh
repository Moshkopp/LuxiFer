#!/usr/bin/env bash
# Aktualisiert einen bereits installierten Hub-Dienst aus diesem Git-Clone.
set -euo pipefail

SERVICE_NAME="hub.service"
INSTALL_PATH="/usr/local/bin/hub"
BACKUP_PATH="/usr/local/bin/hub.previous"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"
source "${SCRIPT_DIR}/branding.conf"

die() {
    echo "Fehler: $*" >&2
    exit 1
}

command -v git >/dev/null || die "git wurde nicht gefunden"
command -v cargo >/dev/null || die "cargo wurde nicht gefunden"
command -v sudo >/dev/null || die "sudo wurde nicht gefunden"
command -v systemctl >/dev/null || die "systemd/systemctl wurde nicht gefunden"

[[ -d .git ]] || die "update.sh muss im ${PRODUCT_NAME}-Git-Clone liegen"
[[ -z "$(git status --porcelain)" ]] \
    || die "der Git-Clone enthält lokale Änderungen; Update abgebrochen"
git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' >/dev/null 2>&1 \
    || die "der aktuelle Branch hat keinen Upstream"

echo "» Hole aktuellen Quellstand …"
git pull --ff-only

echo "» Prüfe Hub …"
cargo test --locked --package hub

echo "» Baue Hub im Release-Profil …"
cargo build --locked --release --package hub
NEW_BINARY="${SCRIPT_DIR}/target/release/hub"
[[ -x "${NEW_BINARY}" ]] || die "Release-Binary wurde nicht erzeugt"

echo "» Installiere neue Version …"
sudo systemctl cat "${SERVICE_NAME}" >/dev/null \
    || die "${SERVICE_NAME} ist nicht installiert"
[[ -x "${INSTALL_PATH}" ]] || die "installiertes Binary fehlt: ${INSTALL_PATH}"

sudo install -m 0755 -o root -g root "${INSTALL_PATH}" "${BACKUP_PATH}"
INSTALL_TEMP="${INSTALL_PATH}.new"
sudo install -m 0755 -o root -g root "${NEW_BINARY}" "${INSTALL_TEMP}"
sudo mv -f "${INSTALL_TEMP}" "${INSTALL_PATH}"

if sudo systemctl restart "${SERVICE_NAME}" \
    && sudo systemctl is-active --quiet "${SERVICE_NAME}"; then
    echo "Hub wurde erfolgreich aktualisiert."
    sudo systemctl --no-pager --full status "${SERVICE_NAME}" || true
    exit 0
fi

echo "Neuer Hub-Dienst startet nicht; stelle vorheriges Binary wieder her." >&2
sudo install -m 0755 -o root -g root "${BACKUP_PATH}" "${INSTALL_PATH}"
sudo systemctl restart "${SERVICE_NAME}" || true
sudo systemctl --no-pager --full status "${SERVICE_NAME}" || true
die "Update fehlgeschlagen; die vorherige Version wurde wiederhergestellt"
