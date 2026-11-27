# Español, inglés y portugués

> **Estado:** Aceptado.

## Contexto

La app debe poder usarse y cambiar de idioma sin reiniciar.

## Decisión

Todo texto de producto sale de diccionarios con la misma forma en español, inglés y portugués. El runtime conserva sus mensajes de sistema traducidos. El idioma persiste y se aplica al momento.

## Alternativas descartadas

Se descartan cadenas visibles en componentes y traducciones parciales anunciadas como completas.

## Consecuencias

Las claves técnicas de catálogos se traducen antes de mostrarse. El contenido escrito por la persona no se traduce.
