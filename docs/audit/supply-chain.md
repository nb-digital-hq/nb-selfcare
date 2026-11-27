# Cadena de suministro

> **Estado:** Aplicable.

Los manifiestos y archivos de bloqueo describen el árbol real. Los avisos de terceros se regeneran desde las
dependencias instaladas. `cargo deny` verifica licencias, fuentes y avisos conocidos; sus excepciones necesitan
justificación vigente. No se aceptan binarios externos ni modelos descargados sin procedencia verificable.

Antes de distribuir se debe comprobar el paquete, sus recursos, licencias, firmas y actualización. La existencia
de configuración CI no sustituye una ejecución remota correcta.

## Comprobación del árbol actual

Se actualizó el almacenamiento a `rusqlite` 0.42 y `r2d2_sqlite` 0.37.1, con `libsqlite3-sys` 0.40.0.
Se retiró la excepción de RUSTSEC-2026-0459: la importación ya no depende del SQLite vulnerable a un WAL manipulado.
La verificación de licencias evalúa correctamente conjunciones y alternativas SPDX; una cláusula MIT no permite
aceptar una conjunción con una licencia incompatible.

Permanece una excepción informativa de mantenimiento para `chrono` (RUSTSEC-2026-0543), que sí se utiliza en el
dominio. No corresponde a una vulnerabilidad de memoria. Sustituir esa biblioteca temporal requiere una migración
específica y la aprobación de su reemplazo; no se atribuye el aviso a una dependencia inexistente.

## Estado de la auditoría de mantenimiento

`cookie` 0.18.3 y `url` 2.5.9 corrigen los avisos de vulnerabilidad detectados por el catálogo actualizado.
El árbol estable conserva cuatro avisos de mantenimiento evaluados en
[excepciones de mantenimiento](maintenance-exceptions.md). Los cuatro identificadores tienen excepciones aprobadas, con alcance y motivación explícitos.
No se permite omitir nuevos avisos de vulnerabilidad. La auditoría se ejecuta de nuevo al cambiar el árbol.

El inventario JavaScript recorre el grafo de producción instalado y sus manifiestos, incluidas transitivas y
opcionales. No depende de metadatos presentes en la caché del gestor. Conserva la versión realmente distribuida
y rechaza licencias contradictorias; las pruebas incluyen la normalización de versiones y subárboles repetidos.
