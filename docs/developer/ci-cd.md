# Integración continua

> **Estado:** Implementado. Linux se verifica entero; Windows y macOS ejecutan sus pruebas nativas en la CI, pero
> **no se han verificado en un equipo real**.

## Flujos

| Flujo | Cuándo | Qué hace |
|---|---|---|
| `ci.yml` | Cada push a `main`, cada pull request | Ver abajo |
| `ediciones.yml` | A mano, al preparar una versión | Paquetes firmados de Linux, Windows y macOS como artefactos ([empaquetado](packaging.md)) |

Los dos flujos terminan conservando solo su última ejecución (`scripts/limpiar-ejecuciones.mjs`).

## `ci.yml`

| Trabajo | Sistema | Qué comprueba |
|---|---|---|
| Procedencia y convenciones | Linux, solo en pull requests | Asunto de cada commit en Conventional Commits y `Signed-off-by` |
| Contratos de workflows | Linux | `actionlint`, con versión y huella fijadas |
| `pnpm verify` | Ubuntu 22.04 | Lint, tipos, pruebas de la interfaz y guardianes |
| Runtime | Ubuntu 22.04, Windows, macOS | `cargo fmt --check`, `cargo clippy --all-targets --all-features -D warnings`, `cargo test` y tipos del puente al día |
| Dependencias y licencias | Linux | `cargo deny check advisories licenses bans sources` |

## Lo que hay que preparar antes de `cargo`

* **La interfaz compilada** (`pnpm build`): el runtime la incrusta y el script de compilación de Tauri se detiene sin
  ella.
* **La cadena de Rust** fijada en `rust-toolchain.toml`, la misma en local y en la CI.

## Trampas conocidas de las otras plataformas

* Código o importaciones que solo existen bajo `cfg(target_os = "linux")` o `cfg(unix)` quedan sin usar en las otras
  plataformas, y con `-D warnings` son un error. Se declaran con su condición (`check-plataforma`).
* Archivos cuyo nombre solo difiere en mayúsculas rompen Windows y macOS.

## Avisos de dependencias

Un aviso nuevo de `cargo deny` se arregla actualizando la dependencia. Lo que no tiene arreglo se acepta en
`deny.toml` con su motivo y se documenta en [cadena de suministro](../audit/supply-chain.md).
