# Licencias y marcas

> **Estado:** Aceptado.

## Contexto

El código de la suite comunitaria debe permitir distribución bajo Apache 2.0.

## Decisión

El código propio se ofrece bajo Apache-2.0. Se conservan avisos y licencias de terceros. Las marcas y logotipos no se conceden como derecho de suplantación.

## Alternativas descartadas

Se descartan dependencias con licencias incompatibles y un cambio de licencia silencioso de código ajeno.

## Consecuencias

cargo deny y el generador de avisos deben comprobar el árbol real. La MPL se admite solo con sus obligaciones por archivo; las fuentes conservan OFL. Añadir dependencias o cambiar licencias requiere aprobación.
