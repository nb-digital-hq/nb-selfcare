# Contratos del puente

> **Estado:** Implementado; recorrido de aplicación y empaquetado en validación.

La interfaz accede a Rust exclusivamente mediante `src/lib/core/ipc/`. Los 83 comandos registrados en
`src-tauri/src/lib.rs` tienen envoltorios tipados en `src/lib/core/ipc/index.ts`. Su implementación está en
`src-tauri/src/comandos.rs`. El guardián comprueba que cada comando tiene envoltorio y un consumidor en las vistas. Esto no sustituye
el recorrido funcional de cada flujo antes de publicar.

## Ejecución y tipos

Los comandos son asíncronos. Las operaciones de base de datos se delegan mediante `en_db` a
`tauri::async_runtime::spawn_blocking`; los selectores nativos se abren desde Rust. Las rutas elegidas permanecen
en el runtime y no se entregan al webview. Los argumentos del IPC usan `camelCase`, también cuando el identificador
Rust usa guiones bajos: `hace_minutos` se envía como `haceMinutos`.

Los tipos de `src/lib/bindings/` se generan desde Rust con `ts-rs`, nunca se mantienen a mano. La configuración
`.cargo/config.toml` establece `TS_RS_LARGE_INT = "number"`: los enteros del protocolo se serializan como números
JSON y el webview no recibe objetos `BigInt`. Los valores intercambiados deben permanecer dentro del intervalo de
enteros seguros de JavaScript; declarar `number` no garantiza por sí mismo esa cota para futuros campos.

`node scripts/check-bindings.mjs` regenera los tipos y su índice, y compara el contenido anterior y posterior.
Una diferencia hace fallar la comprobación, incluso si el archivo ya estaba modificado o todavía no existe un
primer commit. Tras un cambio intencional, se revisan los archivos regenerados y se ejecuta de nuevo el comando.

## Superficie del runtime

| Familia | Responsabilidad |
|---|---|
| Salud y ventana | Versión, arranque en segundo plano y registro del fallo al mostrar la ventana |
| Preferencias | Apariencia, idioma, avisos, inicio con el sistema y consentimiento de red |
| Actualizaciones | Consulta e instalación a través de `net.rs` |
| Hoy y perfil | Resumen diario, bienvenida y cambios del perfil |
| Comida | Catálogo, registros, agua, combinaciones, analizador determinista y código de barras |
| Cuerpo | Pesajes, cinta, entrenamientos y calibración |
| Ayuno | Estado, comprobación de seguridad, comienzo, ajuste y finalización |
| Cuidado | Tratamientos, tomas, hábitos, sesiones, signos vitales, síntomas, chequeos y contadores |
| Diario | Entradas, sugerencias deterministas, sueño, cafeína y ciclo opcional |
| Nibi | Estado, artículos locales, apariencia y nombre |
| Copias | Exportación, selección, inspección y restauración |
| Evolución | Días agregados, semana frente a la anterior, relaciones descriptivas e historial corregible |
| Enlace | Estado, consentimiento, invitación QR, revocación y resolución de cambios simultáneos con el teléfono |

## Errores

Rust devuelve `ScResult<T>` con un objeto que contiene un código estable. `ScError` y su serialización se definen
en `src-tauri/src/error.rs`; ese módulo genera `src/lib/bindings/errorCodes.ts`.

| Código | Significado |
|---|---|
| `FILE_NOT_FOUND` | El archivo solicitado no existe |
| `PATH_NOT_ALLOWED` | Una ruta o clave está fuera de lo permitido |
| `STORAGE_FAILURE` | No se pudo leer o escribir el almacenamiento |
| `SCHEMA_MISMATCH` | El esquema no es compatible |
| `COMPONENT_UNAVAILABLE` | Un componente del sistema no está disponible |
| `NETWORK_DISABLED` | La salida de red está desactivada |
| `CONSENT_REQUIRED` | Falta consentimiento para el servicio |
| `NETWORK_FAILURE` | La petición de red falló |
| `INTEGRITY_MISMATCH` | La integridad o firma no coincide |
| `CANCELLED` | Se canceló la operación |
| `INVALID_INPUT` | Los datos no cumplen las restricciones |
| `NAME_TAKEN` | El nombre ya está en uso |
| `BACKUP_INVALID` | La copia no es válida |
| `NOT_FOUND` | El registro solicitado no existe |
| `INTERNAL` | Fallo interno sin detalle público |

La capa TypeScript solo conserva códigos incluidos en la lista generada. Los rechazos desconocidos se convierten
en `INTERNAL`; nunca se propagan mensajes arbitrarios, rutas ni contenido privado del rechazo.

## Eventos

El runtime emite `avisos:aviso` con el tipo generado `AvisoEscritorio` para las pausas activas, el descanso visual, el agua y
las tomas. `escuchar` devuelve la función que cancela la suscripción; quien se suscribe debe liberarla al desmontarse.
Las notificaciones del sistema se publican desde Rust, independientemente de que la ventana esté visible.

La bandeja emite `avisos:empezar` con `pausa` al solicitar una pausa. Después de restaurar una copia, el runtime
emite `datos:cambio` sin contenido para invalidar las lecturas de las vistas. Estos eventos se consumen desde la carcasa para actualizar las vistas.

## Validación y límites actuales

Las pruebas del puente verifican los nombres de argumentos, la serialización JSON del perfil y la normalización
de errores. Las pruebas de Rust comprueban la lógica de dominio y la restauración de una copia móvil de prueba.
La interfaz tiene los ocho destinos de autocuidado y pasa tipos, lint y sus pruebas. Los controles estáticos
comprueban la conexión de los 83 comandos. El recorrido funcional en el runtime y los paquetes necesitan su
propia evidencia; el producto no se declara publicado por tener esas comprobaciones correctas.

El resultado de `salud` incluye `gestionadaPorLauncher`, resuelto por Rust desde la marca junto al ejecutable.
Solo informa autoridad de actualización; no registra una capacidad sanitaria ni envía datos al Launcher.
