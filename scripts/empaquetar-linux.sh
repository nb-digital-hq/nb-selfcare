#!/usr/bin/env bash
# Empaqueta NB SelfCare para Linux dentro de un Ubuntu 22.04.
#
# Por qué en un contenedor: compilado en Arch, el binario exige la glibc de Arch
# (2.44) y no arranca en Ubuntu 24.04 (2.39) ni en nada más antiguo. Se compila en
# la distribución más antigua que se soporta, y lo compilado ahí corre en todo lo
# posterior. Lo mismo hace el CI de Tauri.
#
# Uso: scripts/empaquetar-linux.sh        (necesita podman)
# Con TAURI_SIGNING_PRIVATE_KEY y su contraseña en el entorno, firma cada paquete
# para el actualizador. Deja el .deb, el .rpm, la AppImage y el .tar.gz en
# src-tauri/target/ubuntu-22.04/release/bundle/.
set -euo pipefail

RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
IMAGEN="docker.io/library/ubuntu:22.04"

# El modo host solo se utiliza si el entorno de construcción no ofrece /dev/net/tun.
opciones_red=()
case "${NB_SELFCARE_CONTAINER_NETWORK:-}" in
  '') ;;
  host) opciones_red=(--network host) ;;
  *) echo 'NB_SELFCARE_CONTAINER_NETWORK solo admite host o vacío'; exit 1 ;;
esac

podman run --rm "${opciones_red[@]}" \
  --memory 8g \
  -v "$RAIZ":/repo:Z \
  -v nb-selfcare-node:/repo/node_modules \
  -v nb-selfcare-cargo:/root/.cargo \
  -v nb-selfcare-rustup:/root/.rustup \
  -v nb-selfcare-tauri:/root/.cache/tauri \
  -w /repo \
  -e CARGO_TARGET_DIR=/repo/src-tauri/target/ubuntu-22.04 \
  -e CARGO_BUILD_JOBS=2 \
  -e DEBIAN_FRONTEND=noninteractive \
  -e TAURI_SIGNING_PRIVATE_KEY \
  -e TAURI_SIGNING_PRIVATE_KEY_PASSWORD \
  "$IMAGEN" bash -euo pipefail -c '
    apt-get update -qq
    apt-get install -y -qq git curl ca-certificates build-essential pkg-config file xz-utils \
      libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev patchelf >/dev/null
    if ! command -v node >/dev/null; then
      curl -fsSL https://deb.nodesource.com/setup_22.x | bash - >/dev/null
      apt-get install -y -qq nodejs >/dev/null
    fi
    corepack enable >/dev/null 2>&1 || npm install -g pnpm >/dev/null
    [ -x /root/.cargo/bin/cargo ] || curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.100.0 >/dev/null
    export PATH=/root/.cargo/bin:$PATH
    pnpm install --frozen-lockfile >/dev/null
    # Las herramientas de la AppImage (linuxdeploy, AppRun y el runtime de tipo 2)
    # quedan en el volumen nb-selfcare-tauri tras la primera descarga. El runtime se
    # pasa por archivo si está ahí: appimagetool lo bajaría en cada compilación.
    if [ -f /root/.cache/tauri/runtime-x86_64 ]; then export LDAI_RUNTIME_FILE=/root/.cache/tauri/runtime-x86_64; fi
    bash scripts/empaquetar-linux-nativo.sh
  '
