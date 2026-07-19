#!/usr/bin/env bash
# SleepGuard Linux installer — privileges once (RNF-01).
# Installs headless boot guard (+ optional user monitor unit).
#
# Usage:
#   sudo ./install-guard.sh              # guard + monitor
#   sudo ./install-guard.sh --guard-only # only early-boot guard
#
# Optional env:
#   GUARD_SRC  path to sleepguard-guard binary
#   APP_BIN    path to sleepguard-app (for monitor unit)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

GUARD_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --guard-only) GUARD_ONLY=1 ;;
    -h|--help)
      echo "Usage: sudo $0 [--guard-only]"
      exit 0
      ;;
  esac
done

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Ejecuta con sudo / pkexec: sudo $0"
  exit 1
fi

TARGET_USER="${SUDO_USER:-}"
if [[ -z "$TARGET_USER" || "$TARGET_USER" == "root" ]]; then
  if [[ -n "${PKEXEC_UID:-}" ]]; then
    TARGET_USER="$(getent passwd "$PKEXEC_UID" | cut -d: -f1 || true)"
  fi
fi
if [[ -z "${TARGET_USER:-}" || "$TARGET_USER" == "root" ]]; then
  echo "No se pudo determinar el usuario destino (SUDO_USER / PKEXEC_UID)."
  exit 1
fi

USER_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6)"
CONFIG_DIR="${USER_HOME}/.config/sleepguard"
CONFIG_PATH="${CONFIG_DIR}/config.json"
USER_UID="$(id -u "$TARGET_USER")"

mkdir -p "$CONFIG_DIR"
chown "$TARGET_USER:$TARGET_USER" "$CONFIG_DIR"

resolve_guard_src() {
  local candidates=(
    "${GUARD_SRC:-}"
    "${SCRIPT_DIR}/sleepguard-guard"
    "/usr/local/bin/sleepguard-guard"
    "/usr/bin/sleepguard-guard"
    "${APP_ROOT}/src-tauri/target/release/sleepguard-guard"
    "${APP_ROOT}/src-tauri/binaries/sleepguard-guard"
  )
  # Sibling of packaged app binary if APP_BIN known
  if [[ -n "${APP_BIN:-}" ]]; then
    candidates+=("$(dirname "$APP_BIN")/sleepguard-guard")
  fi
  for c in "${candidates[@]}"; do
    [[ -z "$c" ]] && continue
    if [[ -x "$c" ]]; then
      echo "$c"
      return 0
    fi
  done
  return 1
}

resolve_app_bin() {
  local candidates=(
    "${APP_BIN:-}"
    "/usr/bin/sleepguard-app"
    "/usr/local/bin/sleepguard-app"
    "${APP_ROOT}/src-tauri/target/release/sleepguard-app"
  )
  for c in "${candidates[@]}"; do
    [[ -z "$c" ]] && continue
    if [[ -x "$c" ]]; then
      echo "$c"
      return 0
    fi
  done
  return 1
}

GUARD_FOUND="$(resolve_guard_src)" || {
  echo "No se encontró el binario sleepguard-guard."
  echo "Compila con: cd src-tauri && cargo build --release -p sleepguard-guard"
  echo "O define GUARD_SRC=/ruta/a/sleepguard-guard"
  exit 1
}

install -m 755 "$GUARD_FOUND" /usr/local/bin/sleepguard-guard
echo "Guard instalado desde: $GUARD_FOUND → /usr/local/bin/sleepguard-guard"

# System unit (boot guard)
UNIT_TMP="$(mktemp)"
sed "s|@CONFIG_PATH@|${CONFIG_PATH}|g" \
  "$SCRIPT_DIR/sleepguard-guard.service" > "$UNIT_TMP"
install -m 644 "$UNIT_TMP" /etc/systemd/system/sleepguard-guard.service
rm -f "$UNIT_TMP"

systemctl daemon-reload
systemctl enable sleepguard-guard.service

if [[ "$GUARD_ONLY" -eq 0 ]]; then
  APP_FOUND="$(resolve_app_bin)" || APP_FOUND="/usr/local/bin/sleepguard-app"
  if [[ -x "${APP_ROOT}/src-tauri/target/release/sleepguard-app" && "$APP_FOUND" == "/usr/local/bin/sleepguard-app" ]]; then
    install -m 755 "${APP_ROOT}/src-tauri/target/release/sleepguard-app" /usr/local/bin/sleepguard-app
    APP_FOUND="/usr/local/bin/sleepguard-app"
  fi

  USER_UNIT_DIR="${USER_HOME}/.config/systemd/user"
  mkdir -p "$USER_UNIT_DIR"
  chown -R "$TARGET_USER:$TARGET_USER" "${USER_HOME}/.config/systemd"

  USER_UNIT_TMP="$(mktemp)"
  sed -e "s|@CONFIG_PATH@|${CONFIG_PATH}|g" \
      -e "s|@APP_BIN@|${APP_FOUND}|g" \
    "$SCRIPT_DIR/sleepguard-monitor.service" > "$USER_UNIT_TMP"
  install -m 644 -o "$TARGET_USER" -g "$TARGET_USER" \
    "$USER_UNIT_TMP" "${USER_UNIT_DIR}/sleepguard-monitor.service"
  rm -f "$USER_UNIT_TMP"

  sudo -u "$TARGET_USER" XDG_RUNTIME_DIR="/run/user/${USER_UID}" \
    systemctl --user daemon-reload || true
  sudo -u "$TARGET_USER" XDG_RUNTIME_DIR="/run/user/${USER_UID}" \
    systemctl --user enable sleepguard-monitor.service || true
fi

echo ""
echo "Instalación completada."
echo "  Guard:   systemctl status sleepguard-guard"
if [[ "$GUARD_ONLY" -eq 0 ]]; then
  echo "  Monitor: systemctl --user status sleepguard-monitor  (como $TARGET_USER)"
fi
echo "  Config:  $CONFIG_PATH"
echo ""
echo "Prueba en seco antes de un reboot real:"
echo "  sudo SLEEPGUARD_DRY_RUN=1 systemctl start sleepguard-guard"
echo "  journalctl -u sleepguard-guard -e"
