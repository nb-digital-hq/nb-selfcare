# Política de dependencias

> **Estado:** Aplicable.

Toda dependencia nueva requiere aprobación, finalidad concreta, licencia admitida y verificación del árbol.
Los archivos de bloqueo fijan las versiones resueltas. `deny.toml` y `generate-notices.mjs` deben admitir las mismas
licencias. No se introduce una dependencia si una implementación propia pequeña y verificable resuelve la necesidad.

| Grupo | Dependencias directas actuales |
|---|---|
| Escritorio | Tauri, dialog, updater, notification, single-instance, autostart |
| Datos | rusqlite embebido, r2d2, r2d2_sqlite, chrono |
| Contratos | serde, serde_json, thiserror, ts-rs |
| Registro | tracing, tracing-subscriber, tracing-appender |
| Red y firmas | ureq, tokio, minisign-verify, base64 |
| Enlace local | axum, tokio-rustls, rcgen, mdns-sd, if-addrs, qrcode, futures-util, keyring, sha2, getrandom |
| Analizador | regex |
| Interfaz | Svelte, SvelteKit estático, Material 3 y API Tauri |

La lista completa se genera en [avisos de terceros](../../THIRD-PARTY-NOTICES.md). No hay motor de audio, modelos
ONNX ni intérprete Python. Las excepciones de seguridad deben corresponder a dependencias reales y revisarse.

La pila del enlace local está aprobada para el contrato del ADR-014. TLS usa ring; el llavero usa los almacenes
nativos de macOS y Windows y Secret Service en Linux. No se guarda una clave en texto plano si el llavero falla.
QR y descubrimiento no llevan registros de salud.

`if-addrs` 0.15 está aprobado como dependencia directa (MIT/BSD-3-Clause); ya formaba parte del árbol de
`mdns-sd`. Enumera interfaces locales sin sondear servidores externos. `net.rs` filtra las direcciones utilizables
y no expone nombres de interfaz, direcciones MAC ni identificadores del equipo.

`serde_json` activa `float_roundtrip` para interpretar los números del protocolo con precisión binary64
consistente con Dart. No añade otro paquete; la huella usa `sha2`, ya aprobado. En Mobile utiliza `crypto`,
que ya estaba declarado. Los vectores independientes cubren también decimales y subnormales.
