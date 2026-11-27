-- Generado por scripts/generar-sincronizacion.mjs. No se edita a mano.
-- Asigna identidades nuevas a una base del esquema 8 restaurada o migrada al 9.

-- user_profiles (perfil)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'user_profiles', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "user_profiles";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'perfil', registro, 1 FROM sc_sync_identidades WHERE tabla = 'user_profiles';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "user_profiles" f JOIN sc_sync_identidades i
    ON i.tabla = 'user_profiles' AND i.clave = CAST(f."id" AS TEXT);

-- pet_status (inventario_nibi)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'pet_status', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "pet_status";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'inventario_nibi', registro, 1 FROM sc_sync_identidades WHERE tabla = 'pet_status';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "pet_status" f JOIN sc_sync_identidades i
    ON i.tabla = 'pet_status' AND i.clave = CAST(f."id" AS TEXT);

-- daily_summaries (descanso)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'daily_summaries', CAST("date_key" AS TEXT), lower(hex(randomblob(16))) FROM "daily_summaries";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'descanso', registro, 1 FROM sc_sync_identidades WHERE tabla = 'daily_summaries';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "daily_summaries" f JOIN sc_sync_identidades i
    ON i.tabla = 'daily_summaries' AND i.clave = CAST(f."date_key" AS TEXT);

-- food_items (alimento)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'food_items', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "food_items";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'alimento', registro, 1 FROM sc_sync_identidades WHERE tabla = 'food_items';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "food_items" f JOIN sc_sync_identidades i
    ON i.tabla = 'food_items' AND i.clave = CAST(f."id" AS TEXT) WHERE f.source <> 'usda_seed';

-- meal_combos (combo)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'meal_combos', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "meal_combos";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'combo', registro, 1 FROM sc_sync_identidades WHERE tabla = 'meal_combos';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "meal_combos" f JOIN sc_sync_identidades i
    ON i.tabla = 'meal_combos' AND i.clave = CAST(f."id" AS TEXT);

-- meal_combo_items (combo_alimento)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'meal_combo_items', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "meal_combo_items";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'combo_alimento', registro, 1 FROM sc_sync_identidades WHERE tabla = 'meal_combo_items';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "meal_combo_items" f JOIN sc_sync_identidades i
    ON i.tabla = 'meal_combo_items' AND i.clave = CAST(f."id" AS TEXT);

-- recipes (receta)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'recipes', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "recipes";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'receta', registro, 1 FROM sc_sync_identidades WHERE tabla = 'recipes';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "recipes" f JOIN sc_sync_identidades i
    ON i.tabla = 'recipes' AND i.clave = CAST(f."id" AS TEXT);

-- recipe_ingredients (ingrediente)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'recipe_ingredients', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "recipe_ingredients";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'ingrediente', registro, 1 FROM sc_sync_identidades WHERE tabla = 'recipe_ingredients';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "recipe_ingredients" f JOIN sc_sync_identidades i
    ON i.tabla = 'recipe_ingredients' AND i.clave = CAST(f."id" AS TEXT);

-- food_logs (comida)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'food_logs', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "food_logs";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'comida', registro, 1 FROM sc_sync_identidades WHERE tabla = 'food_logs';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "food_logs" f JOIN sc_sync_identidades i
    ON i.tabla = 'food_logs' AND i.clave = CAST(f."id" AS TEXT);

-- hydration_logs (agua)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'hydration_logs', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "hydration_logs";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'agua', registro, 1 FROM sc_sync_identidades WHERE tabla = 'hydration_logs';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "hydration_logs" f JOIN sc_sync_identidades i
    ON i.tabla = 'hydration_logs' AND i.clave = CAST(f."id" AS TEXT);

-- body_measurements (peso)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'body_measurements', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "body_measurements";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'peso', registro, 1 FROM sc_sync_identidades WHERE tabla = 'body_measurements';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "body_measurements" f JOIN sc_sync_identidades i
    ON i.tabla = 'body_measurements' AND i.clave = CAST(f."id" AS TEXT);

-- workout_blocks (entrenamiento)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'workout_blocks', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "workout_blocks";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'entrenamiento', registro, 1 FROM sc_sync_identidades WHERE tabla = 'workout_blocks';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "workout_blocks" f JOIN sc_sync_identidades i
    ON i.tabla = 'workout_blocks' AND i.clave = CAST(f."id" AS TEXT);

-- fasting_sessions (ayuno)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'fasting_sessions', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "fasting_sessions";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'ayuno', registro, 1 FROM sc_sync_identidades WHERE tabla = 'fasting_sessions';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "fasting_sessions" f JOIN sc_sync_identidades i
    ON i.tabla = 'fasting_sessions' AND i.clave = CAST(f."id" AS TEXT);

-- journal_entries (diario)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'journal_entries', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "journal_entries";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'diario', registro, 1 FROM sc_sync_identidades WHERE tabla = 'journal_entries';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "journal_entries" f JOIN sc_sync_identidades i
    ON i.tabla = 'journal_entries' AND i.clave = CAST(f."id" AS TEXT);

-- coin_ledger (accion_nibi)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'coin_ledger', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "coin_ledger";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'accion_nibi', registro, 1 FROM sc_sync_identidades WHERE tabla = 'coin_ledger';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "coin_ledger" f JOIN sc_sync_identidades i
    ON i.tabla = 'coin_ledger' AND i.clave = CAST(f."id" AS TEXT);

-- supplements_tracker (tratamiento)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'supplements_tracker', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "supplements_tracker";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'tratamiento', registro, 1 FROM sc_sync_identidades WHERE tabla = 'supplements_tracker';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "supplements_tracker" f JOIN sc_sync_identidades i
    ON i.tabla = 'supplements_tracker' AND i.clave = CAST(f."id" AS TEXT);

-- supplement_logs (toma)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'supplement_logs', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "supplement_logs";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'toma', registro, 1 FROM sc_sync_identidades WHERE tabla = 'supplement_logs';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "supplement_logs" f JOIN sc_sync_identidades i
    ON i.tabla = 'supplement_logs' AND i.clave = CAST(f."id" AS TEXT);

-- female_cycle_logs (ciclo)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'female_cycle_logs', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "female_cycle_logs";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'ciclo', registro, 1 FROM sc_sync_identidades WHERE tabla = 'female_cycle_logs';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "female_cycle_logs" f JOIN sc_sync_identidades i
    ON i.tabla = 'female_cycle_logs' AND i.clave = CAST(f."id" AS TEXT);

-- digestive_logs (digestion)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'digestive_logs', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "digestive_logs";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'digestion', registro, 1 FROM sc_sync_identidades WHERE tabla = 'digestive_logs';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "digestive_logs" f JOIN sc_sync_identidades i
    ON i.tabla = 'digestive_logs' AND i.clave = CAST(f."id" AS TEXT);

-- caffeine_logs (cafeina)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'caffeine_logs', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "caffeine_logs";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'cafeina', registro, 1 FROM sc_sync_identidades WHERE tabla = 'caffeine_logs';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "caffeine_logs" f JOIN sc_sync_identidades i
    ON i.tabla = 'caffeine_logs' AND i.clave = CAST(f."id" AS TEXT);

-- protocol_plans (plan)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'protocol_plans', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "protocol_plans";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'plan', registro, 1 FROM sc_sync_identidades WHERE tabla = 'protocol_plans';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "protocol_plans" f JOIN sc_sync_identidades i
    ON i.tabla = 'protocol_plans' AND i.clave = CAST(f."id" AS TEXT);

-- milestone_goals (meta)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'milestone_goals', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "milestone_goals";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'meta', registro, 1 FROM sc_sync_identidades WHERE tabla = 'milestone_goals';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "milestone_goals" f JOIN sc_sync_identidades i
    ON i.tabla = 'milestone_goals' AND i.clave = CAST(f."id" AS TEXT);

-- care_sessions (sesion_cuidado)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'care_sessions', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "care_sessions";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'sesion_cuidado', registro, 1 FROM sc_sync_identidades WHERE tabla = 'care_sessions';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "care_sessions" f JOIN sc_sync_identidades i
    ON i.tabla = 'care_sessions' AND i.clave = CAST(f."id" AS TEXT);

-- custom_habits (habito)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'custom_habits', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "custom_habits";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'habito', registro, 1 FROM sc_sync_identidades WHERE tabla = 'custom_habits';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "custom_habits" f JOIN sc_sync_identidades i
    ON i.tabla = 'custom_habits' AND i.clave = CAST(f."id" AS TEXT);

-- habit_logs (habito_hecho)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'habit_logs', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "habit_logs";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'habito_hecho', registro, 1 FROM sc_sync_identidades WHERE tabla = 'habit_logs';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "habit_logs" f JOIN sc_sync_identidades i
    ON i.tabla = 'habit_logs' AND i.clave = CAST(f."id" AS TEXT);

-- checkups (chequeo)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'checkups', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "checkups";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'chequeo', registro, 1 FROM sc_sync_identidades WHERE tabla = 'checkups';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "checkups" f JOIN sc_sync_identidades i
    ON i.tabla = 'checkups' AND i.clave = CAST(f."id" AS TEXT);

-- checkup_logs (chequeo_hecho)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'checkup_logs', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "checkup_logs";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'chequeo_hecho', registro, 1 FROM sc_sync_identidades WHERE tabla = 'checkup_logs';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "checkup_logs" f JOIN sc_sync_identidades i
    ON i.tabla = 'checkup_logs' AND i.clave = CAST(f."id" AS TEXT);

-- symptom_logs (sintoma)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'symptom_logs', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "symptom_logs";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'sintoma', registro, 1 FROM sc_sync_identidades WHERE tabla = 'symptom_logs';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "symptom_logs" f JOIN sc_sync_identidades i
    ON i.tabla = 'symptom_logs' AND i.clave = CAST(f."id" AS TEXT);

-- vital_readings (signo)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'vital_readings', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "vital_readings";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'signo', registro, 1 FROM sc_sync_identidades WHERE tabla = 'vital_readings';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "vital_readings" f JOIN sc_sync_identidades i
    ON i.tabla = 'vital_readings' AND i.clave = CAST(f."id" AS TEXT);

-- substance_logs (consumo)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'substance_logs', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "substance_logs";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'consumo', registro, 1 FROM sc_sync_identidades WHERE tabla = 'substance_logs';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "substance_logs" f JOIN sc_sync_identidades i
    ON i.tabla = 'substance_logs' AND i.clave = CAST(f."id" AS TEXT);

-- wellbeing_checks (bienestar)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'wellbeing_checks', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "wellbeing_checks";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'bienestar', registro, 1 FROM sc_sync_identidades WHERE tabla = 'wellbeing_checks';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "wellbeing_checks" f JOIN sc_sync_identidades i
    ON i.tabla = 'wellbeing_checks' AND i.clave = CAST(f."id" AS TEXT);

-- quit_trackers (libre)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'quit_trackers', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "quit_trackers";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'libre', registro, 1 FROM sc_sync_identidades WHERE tabla = 'quit_trackers';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "quit_trackers" f JOIN sc_sync_identidades i
    ON i.tabla = 'quit_trackers' AND i.clave = CAST(f."id" AS TEXT);

-- quit_events (evento_libre)
INSERT INTO sc_sync_identidades (tabla, clave, registro)
    SELECT 'quit_events', CAST("id" AS TEXT), lower(hex(randomblob(16))) FROM "quit_events";
INSERT INTO sc_sync_revisiones (entidad, registro, revision)
    SELECT 'evento_libre', registro, 1 FROM sc_sync_identidades WHERE tabla = 'quit_events';
INSERT INTO sc_sync_diario (registro)
    SELECT i.registro FROM "quit_events" f JOIN sc_sync_identidades i
    ON i.tabla = 'quit_events' AND i.clave = CAST(f."id" AS TEXT);
