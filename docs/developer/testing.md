# Pruebas

> **Estado:** Implementado.

## Las suites

| Suite | Dónde | Qué cubre | Cómo se ejecuta |
|---|---|---|---|
| Interfaz | `src/**/*.test.ts` | Estado, formato, navegación, avisos, idiomas, puente y temas | `pnpm test` |
| Runtime | `src-tauri/src/**` (`#[cfg(test)]`) | Fórmulas, comida, cuerpo, cuidado, ayuno, Nibi, evolución, recetas, base, copias, red y sincronización | `cargo test` |
| Copia del teléfono | `src-tauri/tests/copia_del_telefono.rs` | Una copia de NB SelfCare Mobile se restaura, recibe identidades y vuelve exportada en el esquema común | `cargo test` |
| Sincronización entre procesos | `net::sincronizacion_con_movil`, `net::proceso_dart` | El servidor real con dos motores Flutter de SelfCare Mobile y SQLite; el cliente Dart real empareja, fija el certificado y es revocado | `cargo test -- --ignored` con `NB_SELFCARE_FLUTTER`, `NB_SELFCARE_DART` y `NB_SELFCARE_MOBILE_DIR` |
| Guardianes | `scripts/check-*.mjs` | Las reglas de la [Constitución](../architecture/constitution.md), los tipos del puente, el esquema 9 generado, las traducciones y el presupuesto de arranque | `pnpm guardianes` |
| De extremo a extremo con la aplicación real | `scripts/qa/` | La aplicación compilada sobre una carpeta de datos aislada, manejada por el inspector de WebKit | `node scripts/qa/<recorrido>.mjs` |

## La verificación completa

```bash
pnpm verify                                  # lint, tipos, interfaz y guardianes
pnpm build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

La CI lo ejecuta en Linux, Windows y macOS.

## Pruebas con la aplicación real (`scripts/qa/`)

Los recorridos usan la compilación de desarrollo con la interfaz embebida (`--features tauri/custom-protocol` después
de `pnpm build`), iniciada con `NB_SELFCARE_TEST_DIR` en una carpeta temporal `nb-selfcare-qa-*` y
`WEBKIT_INSPECTOR_HTTP_SERVER=127.0.0.1:9333`. `aislamiento.mjs` comprueba el proceso que escucha el inspector y se
niega a continuar sobre datos que no sean de QA.

| Recorrido | Qué comprueba |
|---|---|
| `recorrido.mjs` | Perfil de demostración, diario, comida y nutrientes, agua, peso, hábito, tratamiento sin monedas, ayuno, Nibi e idiomas |
| `regresiones.mjs` | Formularios tras cambios de datos, escritura bloqueada sin activar la red, propuestas tardías, oxígeno, registro rápido y recetas |
| `accesibilidad.mjs` | Teclado en las secciones, ocho vistas con texto al 150 % y lienzo compacto |
| `temas.mjs` | Los 24 temas aplicados mediante el control real |
| `rendimiento.mjs` | Navegación y búsqueda en caliente |
| `copias-nativas.mjs`, `avisos-nativos.mjs` | Diálogos GTK reales y avisos del escritorio |
| `materiales.mjs` | Capturas y vídeo de la documentación |
| `paquete-linux.sh` | El `.tar.gz` instalado en Ubuntu 22.04: arranque, copias, restauración e inicio con el sistema |

El aislamiento de datos no aísla la configuración de inicio con el sistema: esas pruebas se ejecutan dentro del
contenedor de `paquete-linux.sh`, nunca sobre la sesión normal.

## Datos de prueba

* Las pruebas de Rust usan bases en memoria o directorios temporales con un perfil sintético.
* La copia del teléfono es una base de demostración (`tests/fixtures/copia-movil-demo.sql`).
* Los formatos compartidos con SelfCare Mobile (`tests/fixtures/sincronizacion/`) tienen copias idénticas en los dos
  repositorios.
* Los recorridos de QA crean su propio perfil de demostración en una carpeta nueva.
