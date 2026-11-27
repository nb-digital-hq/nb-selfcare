# Visión del sistema

> **Estado:** Aplicable.

La interfaz estática Svelte accede a Rust mediante `src/lib/core/ipc/`. `comandos.rs` valida el acceso y delega las
operaciones de dominio a un hilo de bloqueo. `Estado` comparte la base, rutas, política de red y servicio de avisos.

| Módulo | Responsabilidad |
|---|---|
| `perfil`, `formulas`, `hoy` | Perfil, estimaciones y resumen diario |
| `comida`, `texto` | Catálogo local, nutrientes y lectura determinista |
| `cuerpo` | Pesajes, cinta, tendencia y actividad |
| `cuidado`, `diario` | Tratamientos, hábitos, sesiones, diario, sueño y ciclo |
| `ayuno`, `nibi` | Sesiones de ayuno y acompañamiento local |
| `db`, `copia`, `paths` | Persistencia, migraciones, recuperación y aislamiento |
| `net`, `telemetry` | Puerta de red y registros redactados |
| `avisos`, `bandeja`, `ventana` | Integración con el escritorio |

No hay servicio móvil conectado al producto, audio, cuenta ni servicio remoto de salud. La infraestructura TLS del [enlace local](sync-contract.md) se valida por separado de la aplicación de registros. La [integración de Launcher](launcher.md)
administra binarios firmados; no comparte los registros personales.
