# Pila tecnológica

> **Estado:** Aceptado.

## Contexto

La suite necesita una base común de escritorio y una instalación autónoma.

## Decisión

Rust con Tauri 2 para el runtime; Svelte 5 y SvelteKit con adaptador estático para la interfaz. SQLite embebido conserva los datos. Se mantienen las dependencias ya declaradas y sus archivos de bloqueo.

## Alternativas descartadas

Se descartan Electron por duplicar un navegador, Flutter en escritorio por romper la base común y un servidor web permanente por añadir operación innecesaria.

## Consecuencias

Las versiones exactas se consultan en los manifiestos: SvelteKit 3 en esta base. No se promete compatibilidad con una versión distinta sin compilarla.
