# Instalación

> **Estado:** Implementado. Linux está verificado en un equipo real; Windows y macOS se publican sin haberse probado
> en un equipo real.

Descarga el paquete de tu sistema desde la publicación más reciente de
[nb-digital-hq/nb-selfcare](https://github.com/nb-digital-hq/nb-selfcare/releases) y, si quieres, verifícalo antes de
instalar ([verificar una descarga](../developer/releases.md#verificar-una-descarga)).

| Sistema | Paquete | Cómo se instala |
|---|---|---|
| Debian, Ubuntu y derivadas | `.deb` | `sudo apt install ./NB.SelfCare_1.0.0_amd64.deb` |
| Fedora, openSUSE y derivadas | `.rpm` | `sudo dnf install ./NB.SelfCare-1.0.0-1.x86_64.rpm` |
| Arch y cualquier otra | `.tar.gz` | `sudo tar -xzf NB.SelfCare_1.0.0_amd64.tar.gz -C /` (usa el WebKitGTK del sistema) |
| Cualquier Linux, sin instalar | AppImage | Darle permiso de ejecución y abrirla |
| Windows 10 y 11 | `-setup.exe` o `.msi` | Abrirlo. SmartScreen avisa porque no lleva firma de Microsoft: "Más información", "Ejecutar de todas formas" |
| macOS en Apple Silicon | `.dmg` | Arrastrar NB SelfCare a Aplicaciones. Como no lleva firma de Apple, la primera vez: clic derecho, "Abrir", y confirmar; o en Ajustes del Sistema, Privacidad y seguridad, "Abrir igualmente" |

No hace falta instalar nada más: la aplicación incluye sus recursos y su base de datos.

## Actualizar

En Ajustes, Actualizaciones, "Buscar actualización" (con la red permitida). La actualización se descarga, se
comprueba su firma y se instala; tus datos se conservan. Si instalaste NB SelfCare con NB Launcher, se actualiza desde
NB Launcher ([contrato](../architecture/launcher.md)).

## Desinstalar

Con el gestor de paquetes de tu sistema, o borrando la AppImage. Los datos de la aplicación quedan en su directorio
por si vuelves a instalarla; si quieres borrarlos, están en `~/.local/share/nb-selfcare` (Linux),
`%APPDATA%\NB SelfCare` (Windows) o `~/Library/Application Support/NB SelfCare` (macOS). La identidad del enlace con el
teléfono está en el llavero del sistema.
