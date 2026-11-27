# Estructura del código

> **Estado:** Aplicable.

`src/routes` contiene las vistas; `src/lib/ui` los componentes y tokens; `src/lib/i18n` los diccionarios.
`src/lib/core/ipc` es la única frontera con Tauri. `src/lib/bindings` se genera desde Rust.

`src-tauri/src/comandos.rs` expone contratos asíncronos. Los módulos de dominio concentran validaciones y reglas;
`db` administra conexiones y migraciones. `net.rs` es la única salida; `paths.rs` resuelve directorios y permite
QA aislado. `scripts/` contiene los guardianes. [Arquitectura](../architecture/system-overview.md).
