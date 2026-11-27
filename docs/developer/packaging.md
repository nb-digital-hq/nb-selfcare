# Empaquetado

> **Estado:** Implementado. Linux se verifica en un equipo real; Windows y macOS se construyen en la CI y se publican
> sin haberse probado en un equipo real.

## Qué lleva cada paquete

El binario de NB SelfCare con la interfaz incrustada, los iconos, `LICENSE`, `NOTICE` y `THIRD-PARTY-NOTICES.md`.
No incluye intérpretes, modelos ni ejecutables auxiliares.

| Plataforma | Paquetes | Notas |
|---|---|---|
| Linux x86-64 | `.deb`, `.rpm`, AppImage, `.tar.gz` | El `.tar.gz` es el árbol `usr/` del `.deb`, para distribuciones sin `dpkg` ni `rpm` y para NB Launcher; usa el WebKitGTK del sistema |
| Windows x86-64 | `-setup.exe` (NSIS), `.msi` | Sin firma de Microsoft: SmartScreen avisa la primera vez. WebView2 se instala sin conexión si falta |
| macOS Apple Silicon | `.dmg` | Sin firma de Apple ni notarización: macOS pide permitirla la primera vez |

Cada paquete de las tres plataformas lleva su firma minisign (`.sig`) para el actualizador.

## Linux

```bash
scripts/empaquetar-linux.sh      # dentro de un Ubuntu 22.04 con podman
```

Se compila en la distribución más antigua que se soporta porque el binario hereda su glibc: compilado en una
distribución reciente no arrancaría en una más antigua. Los paquetes (`.deb`, `.rpm`, AppImage y `.tar.gz`) quedan en
`src-tauri/target/ubuntu-22.04/release/bundle/`. Para firmarlos para el actualizador hacen falta
`TAURI_SIGNING_PRIVATE_KEY` y su contraseña en el entorno. El flujo **Ediciones** produce los mismos paquetes en un
ejecutor Ubuntu 22.04 con `scripts/empaquetar-linux-nativo.sh`.

## Windows y macOS, en la CI

El flujo manual **Ediciones** (`.github/workflows/ediciones.yml`) ejecuta las pruebas nativas de cada sistema, compila
los paquetes con `createUpdaterArtifacts`, los firma con la clave del actualizador guardada en los secretos del
repositorio y los deja como artefactos de la ejecución. Se descargan con `gh run download` y entran en la publicación
([publicación](releases.md)). Cada edición avisa en su resumen de que no está verificada en un equipo real.

## Lo que se comprueba antes de publicar

* Que el `.tar.gz` contiene `usr/bin/nb-selfcare`, los recursos legales y la entrada de escritorio.
* Que cada paquete tiene su `.sig` y que verifica contra la clave pública de `tauri.conf.json`
  (`los_paquetes_locales_tienen_la_firma_y_version_de_la_aplicacion`, con `NB_SELFCARE_BUNDLE_DIR`).
* Que la aplicación instalada desde el `.tar.gz` arranca, exporta, restaura y conserva los datos
  (`scripts/qa/paquete-linux.sh`).
