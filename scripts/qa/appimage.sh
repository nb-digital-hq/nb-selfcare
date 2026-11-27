#!/usr/bin/env bash
# Extrae y arranca la AppImage sin FUSE y sin tocar datos personales.
set -euo pipefail
RAIZ="$(cd "$(dirname "$0")/../.." && pwd)"
NB_QA_APPIMAGE_DIR="$(mktemp -d /tmp/nb-selfcare-qa-appimage-XXXXXX)"
trap 'rm -rf -- "$NB_QA_APPIMAGE_DIR"' EXIT
mkdir -p "$RAIZ/test-results/appimage"
cd "$NB_QA_APPIMAGE_DIR"
"$RAIZ/src-tauri/target/ubuntu-22.04/release/bundle/appimage/NB SelfCare_1.0.0_amd64.AppImage" --appimage-extract > /dev/null
set +e
env -u WEBKIT_INSPECTOR_HTTP_SERVER NB_SELFCARE_TEST_DIR="$NB_QA_APPIMAGE_DIR/perfil" \
  dbus-run-session -- timeout 12s "$NB_QA_APPIMAGE_DIR/squashfs-root/AppRun" \
  > "$RAIZ/test-results/appimage/arranque.log" 2>&1
estado=$?
set -e
test "$estado" -eq 124
test -s "$NB_QA_APPIMAGE_DIR/perfil/datos/nb-selfcare.sqlite"
test "$(sqlite3 "$NB_QA_APPIMAGE_DIR/perfil/datos/nb-selfcare.sqlite" 'PRAGMA integrity_check;')" = ok
echo 'Correcto: AppImage extraída sin FUSE, arranque sostenido y SQLite íntegro'
