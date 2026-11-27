# Convenciones de interfaz

> **Estado:** Aplicable.

La carcasa usa navegación lateral, cabecera y lienzo redondeado, como las apps de escritorio de la suite.
Los colores se consumen mediante tokens. Los campos tienen etiqueta, los botones nombre accesible y el foco
es visible. Ningún gesto de ratón debe ser el único camino a una acción.

Los formularios bloquean envíos duplicados, mantienen datos si falla la escritura y confirman operaciones
irreversibles. Una lectura fallida ofrece reintento; no debe aparentar una lista vacía válida. Las respuestas de
búsquedas antiguas no pueden reemplazar resultados recientes.

Las métricas estimadas se identifican como estimaciones. La medicación no ofrece monedas. El modo de papel y
la preferencia de movimiento reducido suprimen transiciones y sombras innecesarias.

## Distribución por tarea

La cabecera ofrece búsqueda de alimentos y registro directo; la consulta vive en memoria y no entra en URL,
almacenamiento web ni registros. El lateral mantiene los ocho destinos y ancla Ajustes al final. Hoy prioriza la
disposición y la energía, con métricas secundarias y acciones de registro en cajones contextuales. Un dato de
salud ausente se presenta como ausente; los aportes de una puntuación no se rotulan como medidas físicas.

Comida, Cuidado y Perfil agrupan tareas con pestañas navegables por teclado. Ajustes usa navegación interna y
muestras de los 24 temas, agrupadas en oscuros, claros y papel. Los formularios conservan sus validaciones y
las escrituras confirmadas siguen siendo la fuente de estado. Los módulos de salud, copias y la representación
de Nibi se cargan cuando se necesitan, con estado de carga, error canónico y reintento. El presupuesto de
arranque sigue siendo 250 KiB; el rediseño no amplía ese límite.
