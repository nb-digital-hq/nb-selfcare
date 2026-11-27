#!/usr/bin/env bash
# Prueba del tar instalado en Ubuntu 22.04; no modifica la instalación anfitriona.
set -euo pipefail
RAIZ="$(cd "$(dirname "$0")/../.." && pwd)"
mkdir -p "$RAIZ/test-results/paquete-linux"
# El modo host solo se utiliza si el entorno de construcción no ofrece /dev/net/tun.
opciones_red=()
case "${NB_SELFCARE_CONTAINER_NETWORK:-}" in
  '') ;;
  host) opciones_red=(--network host) ;;
  *) echo 'NB_SELFCARE_CONTAINER_NETWORK solo admite host o vacío'; exit 1 ;;
esac

receta="$RAIZ/scripts/qa/Containerfile.linux"
huella_receta="$(sha256sum "$receta" | cut -d ' ' -f 1)"
imagen="localhost/nb-selfcare-qa:${huella_receta}"
if ! podman image exists "$imagen"; then
  podman build "${opciones_red[@]}" --memory 2g -t "$imagen" -f "$receta" "$RAIZ/scripts/qa"
fi

podman run --rm "${opciones_red[@]}" --memory 2g \
  -v "$RAIZ/src-tauri/target/ubuntu-22.04/release/bundle/tar:/paquetes:ro" \
  -v "$RAIZ/scripts/qa/autoinicio-instalado.py:/qa-autoinicio.py:ro" \
  -v "$RAIZ/scripts/qa/dialogo-nativo.py:/qa-dialogo.py:ro" \
  -v "$RAIZ/test-results/paquete-linux:/qa-salida" \
  "$imagen" bash -euo pipefail -c '
    mkdir -p /opt/nb-selfcare-qa /tmp/nb-selfcare-qa-instalada
    tar -xzf /paquetes/NB.SelfCare_1.0.0_amd64.tar.gz -C /opt/nb-selfcare-qa
    set +e
    NB_SELFCARE_TEST_DIR=/tmp/nb-selfcare-qa-instalada WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 \
      xvfb-run -a dbus-run-session -- timeout 18s /opt/nb-selfcare-qa/usr/bin/nb-selfcare >/tmp/arranque.log 2>&1
    estado=$?
    set -e
    cat /tmp/arranque.log
    # timeout acredita que no salió prematuramente; no demuestra interacción con la ventana.
    test "$estado" -eq 124
    for recurso in LICENSE NOTICE THIRD-PARTY-NOTICES.md; do
      test -s "/opt/nb-selfcare-qa/usr/lib/NB SelfCare/$recurso"
    done
    base=/tmp/nb-selfcare-qa-instalada/datos/nb-selfcare.sqlite
    test -s "$base"
    test "$(sqlite3 "$base" "PRAGMA integrity_check;")" = ok
    test "$(sqlite3 "$base" "SELECT count(*) FROM sqlite_master WHERE type=\"table\";")" -gt 10
    echo "Correcto: tar instalado, arranque sostenido, SQLite íntegro y recursos legales presentes"
    NB_QA_BINARIO=/opt/nb-selfcare-qa/usr/bin/nb-selfcare LANG=C.UTF-8 WEBKIT_DISABLE_DMABUF_RENDERER=1 xvfb-run -a dbus-run-session -- sh -c "openbox --sm-disable >/tmp/openbox.log 2>&1 & sleep 1; exec python3 /qa-autoinicio.py"
  '
