# Contrato de copias móviles

> **Estado:** Aplicable.

El contrato V1 es un archivo SQLite de esquema 8. La copia seleccionada se inspecciona en modo de solo lectura;
se rechazan integridad incorrecta, tablas mínimas ausentes o esquema distinto del admitido. La ruta elegida por el
selector permanece en Rust. La interfaz recibe origen, versión, perfil y cantidad de registros para confirmar.

Restaurar reemplaza los datos locales y conserva una copia previa. El [enlace local](sync-contract.md) se desarrolla coordinadamente en ambas aplicaciones mediante el ADR-014; no convierte la restauración en una fusión. La prueba `src-tauri/tests/copia_del_telefono.rs` verifica una copia de demostración. La exportación no
incluye un protocolo de transporte, emparejamiento o sincronización.
