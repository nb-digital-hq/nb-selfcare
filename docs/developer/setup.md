# Preparar el entorno

> **Estado:** Aplicable.

Requisitos de desarrollo: Node y pnpm según package.json, Rust según rust-toolchain.toml y bibliotecas Tauri 2 de
la plataforma. Los usuarios finales no necesitan estas herramientas.

```bash
pnpm install
pnpm tauri dev
```

El puerto de desarrollo es 1440. Para QA, fijar `NB_SELFCARE_TEST_DIR` a una carpeta temporal antes de ejecutar la
app. No usar una base personal. No hay comando de descarga de binarios nativos externos en este producto.
