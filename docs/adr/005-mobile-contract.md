# Intercambio por copias con el móvil

> **Estado:** Aceptado para copias. La exclusión del enlace se sustituye por el [ADR-014](014-live-mobile-sync.md).

## Contexto

El escritorio puede reutilizar datos existentes sin un servidor ni emparejamiento.

## Decisión

Las copias SQLite se exportan y restauran con el mismo esquema que el teléfono. Se inspecciona la copia antes de
confirmar su restauración y se respalda el estado previo. El formato de copia es independiente de la
sincronización en vivo del [ADR-014](014-live-mobile-sync.md).

## Alternativas descartadas

Se descartan un servidor móvil copiado del reproductor y sincronización bidireccional improvisada.

## Consecuencias

Restaurar sustituye los datos; no fusiona historiales.
