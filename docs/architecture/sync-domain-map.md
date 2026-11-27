# Cobertura del dominio de sincronización

> **Estado:** Aplicable. Implementado en `sincronizacion/dominio-v1.json` y sus adaptadores en ambos propietarios.

El [sobre de transporte](sync-contract.md) no es una representación válida de cualquier fila SQLite. Esta matriz
parte de las 37 tablas compartidas del esquema 8 y de las dos tablas exclusivas de escritorio. Define las
fronteras que comprueban los adaptadores.

## Agregados y referencias

Una identidad de registro del protocolo nunca es un entero autoincremental. Cada instalación mantiene la
correspondencia entre identidad estable y clave local. Las referencias a otro agregado usan identidades
estables y se resuelven antes de confirmar. Una referencia ausente no permite crear una fila incompleta.
Los hijos de un agregado se aplican dentro de su misma transacción, sin compartir sus claves locales.

| Tablas actuales | Frontera de aplicación |
|---|---|
| `user_profiles` | Un perfil personal; campos compartidos y locales separados, según la sección siguiente |
| `body_measurements` | Un pesaje con sus lecturas y cinta; `peso` y `cinta` no crean dos identidades para la misma fila |
| `daily_summaries` | Pasos, sueño, fases, despertar y pulso por fecha; totales y objetivos derivados se recalculan |
| `hydration_logs` | Un evento de agua con fecha, instante y volumen; excluye `synced_to_health_connect` |
| `food_items` | Alimentos propios y referencias de catálogo; los identificadores enteros y el orden de la semilla no viajan |
| `meal_combos`, `meal_combo_items` | Una combinación y sus componentes con referencias estables a alimentos |
| `food_logs` | Consumo con alimento, porción y valores nutricionales registrados en ese momento |
| `recipes`, `recipe_ingredients` | Una receta y sus ingredientes; rendimientos y referencias validados antes de aplicar |
| `fasting_sessions` | Una sesión; los dos dispositivos no pueden confirmar dos ayunos activos sin resolver el conflicto |
| `workout_blocks` | Un bloque de actividad; conserva el peso y el esfuerzo utilizados al registrarlo |
| `journal_entries` | Texto, fecha, ánimo, energía, estrés y etiquetas; imágenes y rutas tienen frontera independiente |
| `supplements_tracker` | Definición del tratamiento y pauta; la reserva exige ajustes y consumos explícitos |
| `supplement_logs` | Una toma u omisión de un tratamiento, fecha y franja; no concede recompensas |
| `female_cycle_logs` | Un ciclo con inicio, final y observaciones; conserva fechas de calendario |
| `digestive_logs` | Un registro digestivo con escala, molestias y observaciones |
| `caffeine_logs` | Un consumo con instante, fuente y cantidad |
| `protocol_plans` | Un plan con su horario y estado; la activación simultánea requiere resolución |
| `milestone_goals` | Definición y orden de metas; estados calculados no se suman entre instalaciones |
| `care_sessions` | Una sesión completada con tipo, variante y duración |
| `custom_habits`, `habit_logs` | Definición del hábito y marca por hábito y día; la marca lógica no se duplica |
| `checkups`, `checkup_logs` | Definición del chequeo y cada realización; referencias estables |
| `symptom_logs` | Un síntoma con instante, intensidad y observaciones |
| `vital_readings` | Una lectura con tipo, valores, contexto y procedencia; el identificador externo queda local |
| `substance_logs` | Un consumo con referencia estable a su comida; no genera otra comida al reproducirlo |
| `wellbeing_checks` | Respuestas OMS-5; la puntuación se comprueba contra las respuestas |
| `quit_trackers`, `quit_events` | Un contador y sus eventos; reinicios y ganas superadas no se convierten en contadores nuevos |
| `pet_status`, `coin_ledger` | Estado de Nibi y acciones; saldos, rachas y desbloqueos necesitan proyección y deduplicación |
| `reminders` | Configuración local de avisos; cada sistema conserva sus permisos y programación |
| `tdee_calibrations` | Historial de cálculo local; no se aplica como un nuevo consumo o ajuste remoto |
| `health_sync_checkpoints` | Exclusivamente local: cursores de Health Connect, sin equivalencia en escritorio |
| `ui_preferences` | Exclusivamente local: idioma, disposición, icono y semilla |
| `preferencias`, `instalacion` | Exclusivamente de escritorio: preferencias y autoridad de instalación; nunca se envían |

Cada tabla compartida tiene una entidad propia en el sobre (33 en total). Los nombres de tabla y columna que llegan
a SQL salen siempre de la especificación local, nunca del par; no existe una entidad genérica.

## Perfil, mediciones e imágenes

En el perfil se comparten nombre elegido, fecha de nacimiento, sexo biológico, estatura, metas, objetivos de
actividad y descanso, preferencias alimentarias, macros manuales, fase y fecha de dieta, ciclado, corte de
cafeína, indicadores de seguridad y funciones de autocuidado habilitadas. `avatar_path`, `scale_mac_address`,
`theme_mode`, `local_ai_model_path`, `is_local_ai_enabled`, `has_smart_scale`, `vital_devices_json`,
`care_modules_json` y `discreet_notifications` permanecen locales. El TDEE adaptativo y su última calibración
son proyecciones; no se suman ni se sustituyen por el resultado de otro equipo.

Las métricas de cuerpo calculadas al registrar un pesaje deben distinguirse de las lecturas de origen.
La tendencia, el IMC y las explicaciones calculadas se recalculan con el historial reconciliado. No se infiere
una impedancia ausente a partir de un porcentaje calculado. Peso y cinta comparten fila en el esquema actual:
una edición de cinta no debe borrar el pesaje al recibir un agregado parcial.

Los valores nutricionales de una comida ya registrada describen ese consumo: no se sustituyen al actualizar
el catálogo. Los totales diarios de agua se reconstruyen desde sus eventos. Las cantidades y unidades de los
alimentos, ingredientes y porciones se validan antes de calcular proyecciones.

Las rutas de avatar y diario son referencias locales y nunca se presentan como disponibles en el otro equipo.
El BLOB `progress_photo` no cabe necesariamente en el límite de 64 KiB del registro. Su sincronización requiere
un contrato de adjuntos con límites, integridad y conservación explícitos; el sobre actual no lo implementa.
Aplicar texto de diario debe conservar cualquier adjunto local que el cambio no esté autorizado a sustituir.

## Tratamientos y recompensas

Una toma local puede modificar `stock_units`. Copiar la toma y la reserva y después volver a ejecutar ese
efecto descontaría dos veces. Dos consumos distintos hechos sin conexión tampoco se pueden fusionar por
igualdad de las reservas resultantes. La implementación debe conservar unidades por evento y ajustes explícitos
de reserva, deduplicar la toma lógica por tratamiento, fecha y franja, y no inferir una dosis de su texto.
Tratamientos creados independientemente no se identifican por coincidencia del nombre.

Las acciones de Nibi se deduplican por su identidad lógica y día. `coins_balance`, rachas y conjuntos de objetos
son proyecciones, no cantidades que se suman al unir historiales. Reproducir una comida, sesión o pesaje no
ejecuta de nuevo su recompensa. Ninguna toma u omisión genera monedas.

## Atomicidad, conflictos y restauración

Una escritura local confirma registro, identidad y operación pendiente en una misma transacción de escritura.
La recepción confirma validación, aplicación, revisión y recibo idempotente conjuntamente. Repetir la misma
operación devuelve su recibo; reutilizar su identidad con otro contenido se rechaza. La revisión base impide
reemplazar silenciosamente una edición concurrente; las dos propuestas se conservan para resolverlas.

Una baja conserva su identidad y revisión hasta que los pares autorizados puedan reconciliarla. La ausencia de
una fila en una página o copia no equivale a una baja. Una restauración conserva la autoridad del llavero,
conserva cambios sin confirmar y cambia la época de reconciliación: no reutiliza un cursor antiguo para borrar
ediciones posteriores. La primera unión de dos instalaciones con historial requiere una elección explícita
antes de sustituir datos del perfil o resolver conflictos. Los relojes de los equipos no eligen un ganador.
