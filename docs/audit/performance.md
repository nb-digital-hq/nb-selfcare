# Auditoría de rendimiento

> **Estado:** Aplicable.

El presupuesto automatizado de arranque de interfaz (`check-paquete`) mide 248,6 KB de un máximo de 250 KB; las
vistas contextuales se cargan diferidas. Esta cifra mide JavaScript inicial, no memoria residente ni tiempo de arranque.

La medición local del webview Linux de desarrollo con recursos embebidos y datos sintéticos pequeños dio:

| Operación caliente | Muestras | Mediana | Percentil 95 |
|---|---|---|---|
| Navegación entre los ocho destinos, espera de datos y dos cuadros | 24 | 83 ms | 84 ms |
| Búsqueda local de arroz, límite de 20 resultados, ida y vuelta IPC | 30 | 2 ms | 4 ms |

Se reproduce con `node scripts/qa/rendimiento.mjs` sobre la aplicación de QA aislada descrita en
[pruebas](../developer/testing.md). Guarda las muestras en `test-results/rendimiento/`, excluido del repositorio.
El sondeo y los cuadros forman parte del tiempo de navegación. Estas medidas no representan una carga grande,
un paquete optimizado, otras plataformas ni un compromiso de latencia.

La regresión de contención SQLite comprueba que una escritura bloqueada falla sin confirmar un cambio de
preferencias. No mide por sí sola la capacidad general de respuesta bajo carga.

No se publican valores tomados de otras aplicaciones ni cifras sin medir. Véanse los [límites](../architecture/performance-budgets.md).
