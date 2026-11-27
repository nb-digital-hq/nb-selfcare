# Evaluación de avisos de mantenimiento

> **Estado:** Aplicable. Cuatro excepciones de mantenimiento aprobadas y declaradas en `deny.toml`.

## Vulnerabilidades corregidas

El archivo de bloqueo fija `cookie` 0.18.3 y `url` 2.5.9. Corrigen respectivamente
[RUSTSEC-2026-0594](https://rustsec.org/advisories/RUSTSEC-2026-0594) y
[RUSTSEC-2026-0592](https://rustsec.org/advisories/RUSTSEC-2026-0592). Tauri y sus componentes compatibles se
actualizan a la rama estable 2.12.7 y sus revisiones coordinadas. No se añadieron excepciones para estos avisos.

## Avisos que conserva el árbol estable

| Aviso | Componente resuelto | Ruta y alcance |
|---|---|---|
| [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370) | `proc-macro-error` 1.0.4 | Macros de GLib/GTK 3 que usa Tauri 2 en Linux; se ejecutan durante la compilación |
| [RUSTSEC-2026-0371](https://rustsec.org/advisories/RUSTSEC-2026-0371) | `zbus_names` 4.3.5 | Tauri notification/single-instance y Secret Service del llavero en Linux |
| [RUSTSEC-2026-0370](https://rustsec.org/advisories/RUSTSEC-2026-0370) | `zvariant` 5.15.1 | Serialización de D-Bus en las mismas dependencias Linux |
| [RUSTSEC-2026-0372](https://rustsec.org/advisories/RUSTSEC-2026-0372) | `zvariant_derive` 5.15.1 | Macros de la serialización anterior |

Estos cuatro avisos son de mantenimiento; no describen una vulnerabilidad de memoria concreta. Los componentes
zvariant y zbus_names se integraron en zbus 6, pero los consumidores estables resueltos siguen utilizando la
rama 5. Los plugins Tauri disponibles en la rama 3 son versiones alfa y no sustituyen la pila estable aprobada.
`keyring` 4.2.0 sigue siendo la versión estable resuelta. Retirar el llavero no elimina el árbol D-Bus de Tauri.

## Decisión

Se aceptan únicamente estos cuatro identificadores de mantenimiento, con esta justificación,
manteniendo bloqueados otros avisos y cualquier vulnerabilidad nueva. No implica aceptar una dependencia nueva,
licencia distinta, una versión alfa de Tauri ni cambiar el almacén seguro. La excepción de `chrono` ya existente
se documenta en [cadena de suministro](supply-chain.md).

La excepción es específica de esos identificadores y no cambia el rechazo de nuevas vulnerabilidades.
Licencias, fuentes y restricciones de dependencias se verifican junto a los avisos del árbol resuelto. Al actualizar los consumidores estables se vuelve a comprobar si las excepciones siguen siendo necesarias.
