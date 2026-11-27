# Constitución

> **Estado:** Aplicable.

## 1. Instalación autónoma

El instalador debe contener el runtime y los recursos necesarios. No se invocan intérpretes ni ejecutables ajenos.
Mecanismos: `check-sin-python`, `check-plataforma` y pruebas del paquete instalado.

## 2. Un producto comprensible

Los ocho destinos están definidos en el [ADR-007](../adr/007-information-architecture.md). Las opciones avanzadas
aparecen donde se necesitan; no hay modos de experiencia ni vistas de diagnóstico.

## 3. Los datos se conservan

Rust es dueño de SQLite. Una migración falla completa o revierte. Se respalda antes de migrar una base existente
y antes de restaurar una copia. Desinstalar no debe borrar historiales. Mecanismos: pruebas de migración y copia,
`check-transacciones` y `check-rutas`.

## 4. La interoperabilidad tiene un contrato real

El enlace con SelfCare Mobile debe mantener consentimiento, identidad y conflictos explícitos en ambos lados.
Las copias son independientes de la sincronización en vivo. Mecanismos: pruebas de copias, del dominio de
sincronización y del [ADR-014](../adr/014-live-mobile-sync.md).

## 5. Privacidad por defecto

No existe telemetría. Ningún contenido personal llega a registros. Toda salida pasa por `net.rs` con consentimiento.
Mecanismos: `check-network`, `check-registro` y pruebas de política de red. No hay cuentas obligatorias.

## 6. Fallos y trabajo aislados

Todos los comandos son asíncronos; el acceso a datos se ejecuta fuera del hilo de la ventana. Un fallo de red no
impide registrar datos locales. Mecanismos: `check-comandos`, errores canónicos y pruebas de dominio.

## 7. Una suite coherente y accesible

Rust, Tauri, Svelte, la estructura documental y los tokens visuales se comparten como convenciones. Tres idiomas,
teclado, foco visible, temas de papel y licencias compatibles con Apache 2.0 son parte del contrato.
Mecanismos: lint, tipos, `check-i18n`, `check-ui`, avisos de terceros y `cargo deny`.

## 8. Documentación y evidencia

`docs/` es la fuente de verdad. Todo cambio de arquitectura o comportamiento incluye su documento actualizado.
Se distingue código implementado de producto validado. Mecanismo: `check-docs` y el proceso de verificación.
