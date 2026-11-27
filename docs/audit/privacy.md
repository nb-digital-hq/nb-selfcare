# Privacidad

> **Estado:** Aplicable.

Los registros de salud se conservan en SQLite local. Las copias exportadas no están cifradas. La interfaz solo
almacena en su caché preferencias visuales, idioma y navegación. No hay telemetría, analítica ni informes de fallos
remotos. `telemetry.rs` registra localmente con redacción obligatoria.

La salida a Open Food Facts se limita al código de barras consentido. Las actualizaciones usan su propio servicio
autorizado. Los avisos del sistema pueden aparecer en la pantalla bloqueada: la preferencia discreta evita nombres
de tratamientos. La privacidad del equipo y del destino donde se guarda una copia depende también del sistema.

Validaciones: guardianes de red y registros, pruebas de política y recorrido con red desactivada.
