#!/usr/bin/env bash
# SleepGuard Linux installer — privileges once (RNF-01).
# Installs headless boot guard + user monitor autostart (RF-07, RNF-05).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Ejecuta con sudo: sudo $0"
  exit 1
fi

TARGET_USER="${SUDO_USER:-}"
if [[ -z "$TARGET_USER" || "$TARGET_USER" == "root" ]]; then
  echo "No se pudo determinar SUDO_USER. Ejecuta con: sudo $0"
  exit 1
fi

USER_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6)"
CONFIG_DIR="${USER_HOME}/.config/sleepguard"
CONFIG_PATH="${CONFIG_DIR}/config.json"
USER_UID="$(id -u "$TARGET_USER")"

mkdir -p "$CONFIG_DIR"
chown "$TARGET_USER:$TARGET_USER" "$CONFIG_DIR"

# Prefer release binaries from cargo workspace
GUARD_SRC="${APP_ROOT}/src-tauri/target/release/sleepguard-guard"
APP_SRC="${APP_ROOT}/src-tauri/target/release/sleepguard-app"
if [[ ! -x "$GUARD_SRC" ]]; then
  echo "No se encontró $GUARD_SRC — compila primero:"
  echo "  cd src-tauri && cargo build --release -p sleepguard-guard -p sleepguard-app"
  exit 1
fi

install -m 755 "$GUARD_SRC" /usr/local/bin/sleepguard-guard

APP_BIN="/usr/local/bin/sleepguard-app"
if [[ -x "$APP_SRC" ]]; then
  install -m 755 "$APP_SRC" "$APP_BIN"
else
  echo "Aviso: no hay binario GUI en release; el monitor user unit apuntará a $APP_BIN"
fi

# System unit (boot guard)
UNIT_TMP="$(mktemp)"
sed "s|@CONFIG_PATH@|${CONFIG_PATH}|g" \
  "$SCRIPT_DIR/sleepguard-guard.service" > "$UNIT_TMP"
install -m 644 "$UNIT_TMP" /etc/systemd/system/sleepguard-guard.service
rm -f "$UNIT_TMP"

systemctl daemon-reload
systemctl enable sleepguard-guard.service

# User unit (monitor with Restart=always)
USER_UNIT_DIR="${USER_HOME}/.config/systemd/user"
mkdir -p "$USER_UNIT_DIR"
chown -R "$TARGET_USER:$TARGET_USER" "${USER_HOME}/.config/systemd"

USER_UNIT_TMP="$(mktemp)"
sed -e "s|@CONFIG_PATH@|${CONFIG_PATH}|g" \
    -e "s|@APP_BIN@|${APP_BIN}|g" \
  "$SCRIPT_DIR/sleepguard-monitor.service" > "$USER_UNIT_TMP"
install -m 644 -o "$TARGET_USER" -g "$TARGET_USER" \
  "$USER_UNIT_TMP" "${USER_UNIT_DIR}/sleepguard-monitor.service"
rm -f "$USER_UNIT_TMP"

# Enable for the real user (lingering helps if needed)
sudo -u "$TARGET_USER" XDG_RUNTIME_DIR="/run/user/${USER_UID}" \
  systemctl --user daemon-reload || true
sudo -u "$TARGET_USER" XDG_RUNTIME_DIR="/run/user/${USER_UID}" \
  systemctl --user enable sleepguard-monitor.service || true

echo ""
echo "Instalación completada."
echo "  Guard:   systemctl status sleepguard-guard"
echo "  Monitor: systemctl --user status sleepguard-monitor  (como $TARGET_USER)"
echo "  Config:  $CONFIG_PATH"
echo ""
echo "Prueba en seco antes de un reboot real:"
echo "  sudo SLEEPGUARD_DRY_RUN=1 systemctl start sleepguard-guard"
echo "  journalctl -u sleepguard-guard -e"
