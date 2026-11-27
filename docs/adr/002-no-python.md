# Sin intérprete en ejecución

> **Estado:** Aceptado.

## Contexto

El producto debe funcionar sin configurar un entorno de desarrollo.

## Decisión

Las reglas de dominio, el analizador determinista y las copias se ejecutan en Rust. No se distribuye ni se invoca Python.

## Alternativas descartadas

Se descartan scripts externos y procesos que dependan del PATH del equipo.

## Consecuencias

Las herramientas usadas para preparar código durante el desarrollo no son dependencias de la aplicación.
