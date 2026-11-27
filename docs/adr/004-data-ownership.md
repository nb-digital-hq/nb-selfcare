# Propiedad y conservación de datos

> **Estado:** Aceptado.

## Contexto

Los datos de salud requieren privacidad, integridad y recuperación local.

## Decisión

Rust es el único dueño de SQLite. Se usa el esquema 8 compatible con las copias móviles, migraciones transaccionales, respaldo previo y rechazo de esquemas futuros. Las escrituras de dominio usan BEGIN IMMEDIATE.

## Alternativas descartadas

Se descartan bases independientes por vista, acceso SQL desde el webview y almacenamiento de salud en localStorage.

## Consecuencias

La caché del webview solo conserva apariencia, idioma y navegación. La copia SQLite contiene datos sensibles y no está cifrada.
