# Distribución con NB Launcher

> **Estado:** Implementado.

El [ADR-013](../adr/013-launcher-distribution.md) define la integración. Launcher detecta y abre el ejecutable,
verifica el catálogo editorial con la clave pública de SelfCare y administra únicamente los binarios del tar Linux
x86_64. `nb.selfcare` es el propietario; no se anuncia un recurso de salud ni un enlace de capacidades al host.

El paquete administrado tiene raíz `usr/`, ejecutable `bin/nb-selfcare`, licencia, NOTICE, avisos de terceros y
entrada de escritorio. Su contrato es `selfcare_linux_v1`. La marca del gestor vive en `usr/bin/` y no se
exporta en las copias personales. Los datos y su migración permanecen en las rutas estándar de SelfCare.

Rust detecta la marca junto al ejecutable canónico e informa `gestionadaPorLauncher` por el comando `salud`.
La interfaz muestra quién administra las actualizaciones. Ambas operaciones independientes, buscar e instalar,
comprueban la autoridad antes de consultar la red. La presencia de una marca dañada también bloquea su uso.

## Catálogo editorial

`node scripts/catalogo-launcher.mjs <bundle> <salida> [unix-seconds]` reúne los cuatro formatos, sus firmas,
tamaños y SHA-256. Los nombres de publicación usan puntos en lugar de espacios, conservando los bytes firmados.
Solo el tar declara gestión administrada. También genera `latest.json` para el actualizador independiente, con el mismo tar y firma. No inventa
entradas Windows/macOS sin sus paquetes. El JSON de catálogo esquema 1 caduca a los catorce días y se firma por separado
con la clave de SelfCare. Generar el catálogo no lo publica ni habilita ninguna conexión.

La clave privada no entra en argumentos, repositorio ni salida. El catálogo se publica con cada versión
([publicación](../developer/releases.md)).

## Comprobación en Linux

El catálogo y el tar firmados pasaron el verificador de propietario, el almacén de metadatos, la caché y el
gestor de instalación reales. La retirada comprobó que SQLite externo al almacén se conserva. En la ventana
de Launcher se comprobaron las tres tarjetas, la detección de SelfCare administrado, apertura mediante su
botón, proceso con perfil aislado y retirada confirmada conservando la base. El host pasó autenticación,
permisos, renovación y revocación; SelfCare no añadió recursos de salud al catálogo de capacidades.

Windows y macOS no se han comprobado en un equipo real.

## Inicio con el sistema

El autoinicio independiente conserva la ruta del ejecutable actual. El almacén de Launcher usa rutas por
versión; sustituir y retirar una copia puede dejar esa referencia antigua. Por eso una copia administrada no crea
entradas de inicio de sesión; `inicio_con_sistema` lo rechaza y Ajustes lo explica, aunque permite retirar una entrada previa. La
instalación independiente conserva el autoinicio. Launcher no modifica las preferencias privadas de SelfCare.
