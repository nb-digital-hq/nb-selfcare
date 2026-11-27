# Contrato de lotes de sincronización

> **Estado:** Implementado.

El [ADR-014](../adr/014-live-mobile-sync.md) delimita la sincronización. Rust implementa el sobre en
`src-tauri/src/sincronizacion/contrato.rs`; SelfCare Mobile mantiene el equivalente en `core/sync/contrato.dart`.
Ambos prueban el mismo `lote-v1.json`, conservado en los fixtures de cada clon para no depender de carpetas vecinas.

| Campo | Contrato |
|---|---|
| `version` | Entero, exactamente 1 |
| `nodo`, `lote` | Identidad opaca de 128 bits, 32 caracteres hexadecimales en minúscula |
| `cambios` | Entre 1 y 200 operaciones |
| `id`, `registro` | Identidad estable de operación y registro; un lote no repite operaciones |
| `entidad` | Enum cerrado de dominios propios de SelfCare |
| `base` | Revisión entera entre 0 y 9007199254740991, representable exactamente en JavaScript |
| `operacion` | `guardar` o `eliminar` |
| `datos` | Objeto no vacío al guardar; nulo o ausente al eliminar |

Cada objeto de registro ocupa como máximo 64 KiB en JSON UTF-8. La profundidad máxima es 16, los objetos
admiten hasta 128 campos y los arrays hasta 512 elementos. Se rechazan campos desconocidos en el sobre y en
la operación. La representación de depuración nunca incluye identificadores ni contenido personal.
Los números son finitos y su valor absoluto no supera 9007199254740991; los enteros mayores perderían precisión
al compartir representación numérica. Las cadenas y claves contienen Unicode válido, sin sustitutos aislados.

## Huella de operación

`sincronizacion/huella.rs` y `core/sync/huella.dart` calculan SHA-256 del mismo cambio validado. El prefijo UTF-8
`NB-SelfCare/operacion/1` seguido de un byte cero separa este uso de otros hashes. Después se codifica el objeto
`{"nodo": nodo, "cambio": cambio}`, con todos los campos del cambio y `datos: null` para una baja. El identificador
del lote no participa: el mismo cambio puede reenviarse en otro lote.

| Valor | Codificación después del byte de tipo |
|---|---|
| Nulo | Tipo 0, sin contenido |
| Falso / verdadero | Tipo 1 / 2, sin contenido |
| Número | Tipo 3 y ocho bytes IEEE-754 binary64 en orden de red; ambos ceros usan cero positivo |
| Cadena | Tipo 4, longitud UTF-8 de cuatro bytes sin signo en orden de red y contenido UTF-8 |
| Lista | Tipo 5, cantidad de cuatro bytes en orden de red y valores en su orden original |
| Objeto | Tipo 6, cantidad de cuatro bytes en orden de red y pares clave/campo ordenados por bytes UTF-8 de la clave |

Las claves usan la misma codificación de cadena. No se normaliza Unicode ni se confunden cadenas con números.
`1` y `1.0` representan el mismo valor; el orden de una lista sigue siendo significativo. Rust activa
`serde_json/float_roundtrip` para conservar la interpretación exacta de decimales y subnormales como binary64.
`huellas-v1.json` fija vectores obtenidos por una implementación independiente: objetos anidados, claves fuera
del plano básico, límites numéricos, subnormales, tipos y bajas. Ambos repositorios mantienen los mismos bytes.

La huella no autentica a un dispositivo ni cifra el contenido, y no se escribe en registros.

Estas comprobaciones no sustituyen autorización del dispositivo, cotas del cuerpo de transporte, validación
sanitaria por entidad, resolución de conflictos ni transacciones de aplicación. Ningún consumidor debe tratar
la deserialización correcta como permiso para escribir en SQLite. El protocolo no transporta claves privadas.

## Recepción transaccional

`sincronizacion/recepcion.rs` y `core/database/sincronizacion/recepcion.dart` implementan la recepción con SQLite.
Sus tablas forman parte del esquema 9; la receta compartida `recepcion-v1.sql` las reproduce en las pruebas.

Cada operación entra en `BEGIN IMMEDIATE`, exige claves foráneas activas y verifica la época de reconciliación.
El par autorizado y la identidad de operación localizan su recibo. Si época y huella coinciden, devuelve el
resultado persistido sin ejecutar de nuevo al propietario. Reutilizar la identidad con otro contenido se
declara operación alterada. Un recibo de otra época tampoco vuelve a aplicarse.

Sin recibo previo, el adaptador propietario valida campos y referencias y entrega una instantánea compartida
acotada. En Mobile esa instantánea se conserva como copia profunda inmutable antes de cualquier espera.
Si la base coincide, escritura del dominio, revisión y recibo se confirman juntos; un fallo revierte los tres.
Si no coincide, se conservan la instantánea actual y la propuesta con un recibo de conflicto, sin sustituir
el dato confirmado. Las bajas conservan revisión y el límite numérico no desborda. Los relojes no deciden
qué propuesta gana.

El adaptador recibe la transacción del receptor y no abre otra ni ejecuta efectos externos dentro de ella.
Reproducir una operación no vuelve a dar recompensas, consumir reserva, exportar a Health Connect o enviar
avisos; esas reglas pertenecen a los adaptadores del dominio. La autorización y el consentimiento corresponden al llamador antes de entrar al receptor.

Las pruebas del componente usan un propietario sintético y tablas de demostración, incluidos archivo reabierto,
fallo tras escribir, fallo al guardar el recibo y escritores concurrentes. Si la última escritura de un registro
vino del mismo par, una base anterior significa que no recibió la confirmación y su edición continúa la suya.

La [matriz del dominio](sync-domain-map.md) delimita las tablas compartidas, campos locales, agregados y
proyecciones que comprueban los adaptadores.

`net.rs` enumera las interfaces privadas activas mediante `if-addrs`, sin consultas externas y con un máximo
de 32 direcciones. Omite loopback e IPv6 de enlace local sin zona, y no expone nombres de interfaz ni MAC.
El móvil descubre candidatos con `nsd`, con tiempo y concurrencia acotados; un anuncio nunca sustituye el pin TLS.

## Invitación e identidad TLS

El sobre QR de versión 1 contiene exactamente `app` (`nb-selfcare`), `version`, `host`, `puerto`, `huella` y
`credencial`. El puerto está entre 1 y 65535. El anfitrión es una IP numérica privada, de enlace local o de
loopback; no se admite DNS, multicast, direcciones públicas ni IPv4 mapeada en IPv6. El móvil acota el texto
QR a 4096 bytes UTF-8. La huella es SHA-256 del certificado DER en base64 estándar canónica. La credencial
contiene 256 bits aleatorios en base64 URL sin relleno. El fixture `invitacion-v1.json` es sintético y se prueba
en ambos repositorios.

`emparejamiento.rs` conserva solo la huella de la invitación, admite una aceptación, caduca a los cinco minutos
con reloj monótono y se bloquea después de cinco intentos incorrectos. El alta posterior debe solicitar otra
invitación si falla después de consumirla. El propietario serializa el consumo entre peticiones.

`identidad.rs` guarda certificado y clave privada en el llavero del sistema, separados de SQLite y de las copias.
Un secreto dañado o un error del llavero no provoca una regeneración silenciosa. Al cargar se comprueba la
correspondencia entre certificado y clave. La configuración usa el proveedor ring explícito, sin cambiar el
proveedor global del runtime. El certificado vence diez años después del año de creación; renovar una identidad
exige emparejar de nuevo.

## Alta y autorización de pares

`GestorDePares` mantiene una identidad opaca de nodo y hasta ocho pares en el llavero, fuera de SQLite y las
copias. Solo conserva SHA-256 de las credenciales permanentes de los pares. Emparejar de nuevo el mismo nodo
sustituye su credencial; revocar elimina su autorización. Altas y revocaciones se serializan y solo se confirman
si el almacén persiste el nuevo estado. Una invitación nueva invalida la anterior. Un almacén dañado no se
regenera como un registro vacío.

La superficie HTTPS `/v1/emparejar` acepta `version` (1), `nodo` (32 hexadecimales minúsculos) y `credencial`
(de invitación), sin campos adicionales y con cuerpo de hasta 4096 bytes. Devuelve `version`, `nodo` del PC y
una credencial permanente aleatoria. `/v1/estado` exige exactamente una cabecera `Authorization: Bearer …` y
una `x-selfcare-nodo`; responde únicamente con `version`. Los errores usan códigos canónicos sin reproducir
cabeceras, cuerpos ni errores de terceros. `/v1/sincronizar` se describe más abajo.

El cliente de Mobile limita respuestas a 4096 bytes, rechaza redirecciones y compresión implícita, fuerza
conexión directa y crea un contexto TLS sin raíces del sistema. Valida la huella del DER y las fechas del
certificado con la hora inyectada por el propietario. El propietario usa `Clock` en la app. Cada operación
comprueba consentimiento y una revocación cierra el cliente.

## Esquema 9, identidades y diario

`src-tauri/sincronizacion/dominio-v1.json` es la fuente única: entidad, columnas locales, referencias, claves
naturales y condición de publicación de cada tabla. `scripts/generar-sincronizacion.mjs` la resuelve contra el
esquema 8 y genera `adaptadores-v1.json`, `migrations/V9__sincronizacion.sql`, `reconstruir-v1.sql` y, con
`--movil`, `lib/core/sync/esquema_v9.dart` de SelfCare Mobile; `--comprobar` falla si algo quedó desactualizado.

El esquema 9 no modifica tablas del 8. Añade `sc_sync_identidades` (identidad aleatoria de 128 bits por fila,
revisión compartida, baja y marca de revisión pendiente), `sc_sync_revisiones`, `sc_sync_diario` (la última
entrada de cada registro y su origen), `sc_sync_control`, `sc_sync_pares` y las tablas de recepción. Disparadores
por tabla capturan altas, cambios de columnas compartidas y bajas en la misma transacción de la escritura. El
catálogo sembrado recibe identidad pero no entra en el diario. Una edición local deja la revisión en la compartida
más uno; aplicar un cambio remoto marca `sc_sync_control` y no la incrementa.

## Intercambio

El escritorio reconcilia. `POST /v1/sincronizar` exige un par autorizado y un cuerpo de hasta 14 MiB con
`version`, `epoca` conocida, `cursor` y un `lote` opcional. Responde `epoca`, `reiniciar`, `resultados`
(`aplicado`, `conflicto` o `rechazado` con su revisión), `cambios`, `cursor` y `mas`. Los cambios salen del diario
en orden, sin devolver al par lo que vino de él y con sus referencias desconocidas primero.

El teléfono envía sus pendientes con base en la revisión compartida y aplica lo recibido como versión del
escritorio, salvo una edición propia aún sin enviar. Tras un conflicto solo una revisión posterior, la decisión,
sustituye su versión. Las referencias viajan como identidades; una fila equivalente nunca compartida (clave natural
igual) adopta la identidad del par: si el contenido coincide no hay duplicado, y si difiere queda un conflicto.
El agua del día y el saldo de Nibi se proyectan una vez por registro aplicado; las tomas no generan monedas.

## Conflictos y restauración

`sincronizacion::servicio::resolver` conserva la versión de este equipo o aplica la del teléfono y publica una
revisión nueva para todos los pares. Una restauración en cualquiera de los dos equipos cambia la época; el teléfono
vuelve a leer desde cero sin perder sus pendientes.

## Pruebas

Rust prueba captura, catálogo, referencias, reintento, baja, conflicto y resolución, primera unión, validación,
monedas y época. Mobile prueba migración 8→9, pendientes, confirmación, aplicación, conflicto y validación. La
prueba de proceso `dos_telefonos_reales_sincronizan_a_traves_del_escritorio` arranca el servidor real y dos motores
Flutter con SQLite: el primero envía, el segundo recibe por el escritorio y una edición posterior llega a ambos.
