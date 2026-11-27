# Integración de distribución con NB Launcher

> **Estado:** Aceptado.

## Contexto

La integración de Launcher se incorpora al alcance de V1. SelfCare debe conservar su autonomía y la propiedad
exclusiva de los registros personales. El Launcher ya gestiona paquetes por propietario con firmas y recibos.

## Decisión

Se sustituye la exclusión de Launcher del ADR-006 por una integración de distribución: detectar, abrir, instalar,
actualizar y quitar una copia administrada. No se comparten registros de salud ni se expone una capacidad de
lectura o escritura del dominio al host. No se incorpora Mesh ni se modifica SelfCare Mobile.

El propietario es `nb.selfcare`, el repositorio oficial es `nb-digital-hq/nb-selfcare` y el ancla editorial es
la clave pública minisign de SelfCare. El catálogo firmado esquema 1 contiene los cuatro formatos Linux x86_64.
Solo el tar declara `selfcare_linux_v1`, con raíz `usr/` y ejecutable `bin/nb-selfcare`.

La marca `.nb-launcher-managed` junto al ejecutable desactiva la búsqueda y la instalación independientes antes
de cualquier conexión. Ajustes muestra la autoridad confirmada por Rust. Quitar binarios administrados conserva
las bases y copias del usuario. Una marca dañada también bloquea el actualizador para no sustituir archivos del gestor.

## Alternativas descartadas

No se duplican tablas ni reglas de salud en Launcher. Tampoco se inventa una capacidad de control remoto para
aprovechar el enlace de Sound: su contrato de frecuencia de copias no representa una función actual de SelfCare.
No se incluyen artefactos sin publicar en el catálogo incorporado de Launcher como si fueran descargas oficiales.

## Consecuencias

Launcher incluye SelfCare en su registro, verificador y tarjeta. SelfCare genera el catálogo firmado con los
paquetes de cada publicación. Las dependencias de ejecución de SelfCare no aumentan.
