# Almacenamiento y copias

> **Estado:** Aplicable.

SQLite embebido usa el esquema 9, el mismo de SelfCare Mobile. El esquema 8 vive en
`src-tauri/migrations/V1__inicial.sql` y el 9 en `V9__sincronizacion.sql`, generado desde el dominio de
sincronización ([contrato](sync-contract.md)); las migraciones en `db/migraciones.rs`. `db::transaccion_de_escritura`
abre `BEGIN IMMEDIATE`. Las migraciones comprueban claves foráneas y respaldan la base existente.

`paths.rs` resuelve datos, caché y registros por plataforma. `NB_SELFCARE_TEST_DIR` aísla las tres rutas para QA.
El archivo principal es `nb-selfcare.sqlite`. El webview no guarda datos de salud en localStorage.

`copia.rs` exporta mediante `VACUUM INTO` y renombra el parcial al terminar. Inspecciona integridad, tablas y versión
antes de restaurar, incluidas sus referencias. La restauración sustituye tablas en `BEGIN IMMEDIATE`, crea un
respaldo previo con nombre único y reconstruye índices. Comprueba las referencias del destino antes de confirmar:
si el origen omite sus restricciones y deja filas huérfanas al copiar, revierte todo. Dos restauraciones en el
mismo segundo conservan sus dos estados anteriores. Conserva hasta cinco copias automáticas; los respaldos previos
a restaurar son independientes. Las copias contienen datos sensibles sin cifrar.

Antes de adjuntar el origen se crea una instantánea privada con `VACUUM INTO` y se valida esa misma instantánea.
Incluye los cambios confirmados que siguen en el WAL y evita leer la base viva mientras se sustituyen sus tablas.
Si falta una tabla compartida, se rechaza la restauración: su ausencia no autoriza vaciar el destino. La carpeta
temporal se retira al terminar, también al fallar. El archivo seleccionado se abre en modo de solo lectura.

El pool comprueba que cada conexión devuelta esté sin transacción abierta, con claves foráneas activas y sin
bases adjuntas ajenas a `main` o `temp`. Descarta una conexión insegura; no la entrega a otra operación. La
restauración retira el adjunto y reactiva las claves también cuando falla. Las pruebas comprueban rechazo de
referencias rotas, reversión y conservación de datos confirmados al descartar conexiones.

Cada migración comprueba referencias dentro de su transacción antes de confirmar la nueva versión. Si introduce
una referencia rota, revierte tanto el esquema como sus datos y conserva la versión anterior.

La compatibilidad se prueba con una copia de demostración móvil. Una restauración no fusiona historiales: toma
las identidades de sincronización de la copia, o las crea para una copia del esquema 8, y cambia la época para que
ningún teléfono reutilice un cursor anterior. La autoridad del enlace vive en el llavero y no cambia.
