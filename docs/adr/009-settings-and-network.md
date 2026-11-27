# Preferencias y puerta única de red

> **Estado:** Aceptado.

## Contexto

Una función local no debe habilitar servicios remotos en silencio.

## Decisión

La red permanece desactivada de fábrica. Toda salida pasa por net.rs y verifica el interruptor general, consentimiento por servicio y anfitriones permitidos. La lista inicial de servicios Open Food Facts y actualizaciones se amplía con el enlace local del [ADR-014](014-live-mobile-sync.md).

## Alternativas descartadas

Se descartan fetch desde el webview, telemetría, cuentas obligatorias y clientes HTTP en módulos de dominio.

## Consecuencias

El código de barras puede enviarse a Open Food Facts con consentimiento. Open Food Facts y las actualizaciones no reciben diario, perfil ni historial de salud. Las credenciales del enlace local se rigen por el ADR-014 y permanecen fuera de las copias.
