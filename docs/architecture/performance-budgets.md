# Rendimiento y límites

> **Estado:** Aplicable.

El runtime acota búsquedas y colecciones antes de cruzar IPC. El trabajo SQLite sale del hilo de la ventana.
Los diccionarios secundarios y el generador de paletas pueden cargarse a demanda. Las fuentes son locales.

Las cifras se miden sobre la aplicación compilada con datos de demostración; las de navegación y búsqueda están en
la [auditoría](../audit/performance.md).
