#!/usr/bin/env bash
# Etapa común para Ubuntu 22.04 local aislado y CI nativa. No instala en el anfitrión.
set -euo pipefail
cd "$(dirname "$0")/.."
pnpm tauri build --bundles deb,rpm,appimage --config '{"bundle":{"createUpdaterArtifacts":true}}'
version=$(node -p 'require("./package.json").version')
bundle="${CARGO_TARGET_DIR:-$PWD/src-tauri/target}/release/bundle"
mkdir -p "$bundle/tar"
archivo="$bundle/tar/NB.SelfCare_${version}_amd64.tar.gz"
datos=("$bundle"/deb/*/data)
[ "${#datos[@]}" -eq 1 ] && [ -d "${datos[0]}" ] || { echo 'El árbol Debian debe ser único para construir el tar'; exit 1; }
tar -czf "$archivo" --owner=0 --group=0 -C "${datos[0]}" usr
contenido=$(tar -ztf "$archivo")
for recurso in 'usr/bin/nb-selfcare' 'usr/lib/NB SelfCare/LICENSE' 'usr/lib/NB SelfCare/NOTICE' \
  'usr/lib/NB SelfCare/THIRD-PARTY-NOTICES.md' 'usr/share/applications/NB SelfCare.desktop'; do
  grep -qxF "$recurso" <<<"$contenido" || { echo "Falta un recurso obligatorio del tar: $recurso"; exit 1; }
done
[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ] || { echo 'La entrega requiere la clave de firma'; exit 1; }
pnpm tauri signer sign --app-version "$version" "$archivo" >/dev/null
