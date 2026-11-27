# Dominio de autocuidado en Rust

> **Estado:** Aceptado.

## Contexto

Los cálculos deben ser consistentes entre vistas y verificables sin webview.

## Decisión

Cada dominio conserva sus validaciones y transacciones en Rust: perfil, fórmulas, comida, cuerpo, cuidado, ayuno, diario y Nibi. La interfaz captura intención y presenta resultados.

## Alternativas descartadas

Se descarta duplicar fórmulas en TypeScript o presentar el analizador determinista como inteligencia artificial.

## Consecuencias

Los resultados derivados son estimaciones de autocuidado, no diagnósticos médicos. Las pruebas verifican reglas y límites.
