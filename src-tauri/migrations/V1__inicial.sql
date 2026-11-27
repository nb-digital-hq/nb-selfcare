-- Esquema inicial de NB SelfCare.
--
-- Las tablas de datos de la persona son las mismas que las de NB SelfCare Mobile en su
-- esquema 8, con los mismos nombres, tipos y codificaciones (fechas en texto ISO-8601,
-- booleanos 0/1): una copia del teléfono se lee sin traducir nada (ADR-004).
-- Las tablas propias del escritorio van al final.

CREATE TABLE "user_profiles" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "name" TEXT NOT NULL, "avatar_path" TEXT NULL, "birth_date" TEXT NOT NULL, "biological_sex" TEXT NOT NULL, "height_cm" REAL NOT NULL, "target_weight_kg" REAL NOT NULL, "weekly_goal_rate_kg" REAL NOT NULL DEFAULT -0.5, "daily_water_goal_ml" INTEGER NOT NULL DEFAULT 2500, "daily_steps_goal" INTEGER NOT NULL DEFAULT 8000, "daily_sleep_goal_hours" REAL NOT NULL DEFAULT 7.5, "current_adaptive_tdee_kcal" INTEGER NOT NULL DEFAULT 2000, "last_tdee_calibration" TEXT NULL, "scale_mac_address" TEXT NULL, "theme_mode" TEXT NOT NULL DEFAULT 'material_you', "local_ai_model_path" TEXT NULL, "is_local_ai_enabled" INTEGER NOT NULL DEFAULT 0 CHECK ("is_local_ai_enabled" IN (0, 1)), "is_cycle_tracking_enabled" INTEGER NOT NULL DEFAULT 0 CHECK ("is_cycle_tracking_enabled" IN (0, 1)), "is_fasting_enabled" INTEGER NOT NULL DEFAULT 1 CHECK ("is_fasting_enabled" IN (0, 1)), "is_manual_macro_mode" INTEGER NOT NULL DEFAULT 0 CHECK ("is_manual_macro_mode" IN (0, 1)), "manual_kcal_target" INTEGER NULL, "manual_protein_g" INTEGER NULL, "manual_carbs_g" INTEGER NULL, "manual_fat_g" INTEGER NULL, "active_diet_archetype" TEXT NOT NULL DEFAULT 'flexible', "active_diet_phase" TEXT NOT NULL DEFAULT 'deficit', "calorie_cycling_schedule_json" TEXT NOT NULL DEFAULT '{}', "caffeine_bedtime_cutoff_hours" REAL NOT NULL DEFAULT 8.0, "active_diet_phase_started_at" TEXT NULL, "has_impedance_contraindication" INTEGER NOT NULL DEFAULT 0 CHECK ("has_impedance_contraindication" IN (0, 1)), "has_eating_disorder_history" INTEGER NOT NULL DEFAULT 0 CHECK ("has_eating_disorder_history" IN (0, 1)), "takes_hypoglycemic_medication" INTEGER NOT NULL DEFAULT 0 CHECK ("takes_hypoglycemic_medication" IN (0, 1)), "has_smart_scale" INTEGER NOT NULL DEFAULT 0 CHECK ("has_smart_scale" IN (0, 1)), "vital_devices_json" TEXT NOT NULL DEFAULT '[]', "care_modules_json" TEXT NOT NULL DEFAULT '["treatments","pelvic_floor","active_breaks","symptoms","checkups","habits","quit"]', "discreet_notifications" INTEGER NOT NULL DEFAULT 1 CHECK ("discreet_notifications" IN (0, 1)), "target_bedtime_minutes" INTEGER NOT NULL DEFAULT 1380, "created_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP));
CREATE TABLE "body_measurements" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "measured_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "date_key" TEXT NOT NULL, "source" TEXT NOT NULL, "weight_kg" REAL NOT NULL, "trend_weight_kg" REAL NOT NULL, "impedance_ohms" INTEGER NULL, "bmi" REAL NOT NULL, "body_fat_percentage" REAL NULL, "muscle_mass_kg" REAL NULL, "water_percentage" REAL NULL, "visceral_fat_index" INTEGER NULL, "bone_mass_kg" REAL NULL, "protein_percentage" REAL NULL, "basal_metabolic_rate_kcal" INTEGER NULL, "metabolic_age" INTEGER NULL, "lean_body_mass_kg" REAL NULL, "body_score" INTEGER NULL, "somatotype" TEXT NULL, "waist_cm" REAL NULL, "neck_cm" REAL NULL, "hip_cm" REAL NULL, "validated_body_fat_percentage" REAL NULL, "is_water_retention_spike" INTEGER NOT NULL DEFAULT 0 CHECK ("is_water_retention_spike" IN (0, 1)), "spike_explanation" TEXT NULL);
CREATE TABLE "daily_summaries" ("date_key" TEXT NOT NULL, "steps_count" INTEGER NOT NULL DEFAULT 0, "active_calories_from_steps" INTEGER NOT NULL DEFAULT 0, "sleep_duration_minutes" INTEGER NOT NULL DEFAULT 0, "sleep_deep_minutes" INTEGER NOT NULL DEFAULT 0, "sleep_rem_minutes" INTEGER NOT NULL DEFAULT 0, "resting_heart_rate_bpm" INTEGER NULL, "wake_time_minutes" INTEGER NULL, "water_consumed_ml" INTEGER NOT NULL DEFAULT 0, "calorie_target_kcal" INTEGER NOT NULL, "protein_target_g" INTEGER NOT NULL, "carbs_target_g" INTEGER NOT NULL, "fat_target_g" INTEGER NOT NULL, PRIMARY KEY ("date_key"));
CREATE TABLE "hydration_logs" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "date_key" TEXT NOT NULL, "logged_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "volume_ml" INTEGER NOT NULL, "synced_to_health_connect" INTEGER NOT NULL DEFAULT 0 CHECK ("synced_to_health_connect" IN (0, 1)));
CREATE TABLE "food_items" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "barcode" TEXT NULL UNIQUE, "name" TEXT NOT NULL, "brand" TEXT NULL, "source" TEXT NOT NULL, "nutri_score" TEXT NULL, "nova_group" INTEGER NULL, "default_serving_grams" REAL NOT NULL DEFAULT 100.0, "serving_label" TEXT NOT NULL DEFAULT '100g', "kcal_per100g" REAL NOT NULL, "protein_per100g" REAL NOT NULL, "carbs_per100g" REAL NOT NULL, "fat_per100g" REAL NOT NULL, "fiber_per100g" REAL NOT NULL DEFAULT 0.0, "sugar_per100g" REAL NOT NULL DEFAULT 0.0, "sodium_mg_per100g" REAL NOT NULL DEFAULT 0.0, "potassium_mg_per100g" REAL NOT NULL DEFAULT 0.0, "magnesium_mg_per100g" REAL NOT NULL DEFAULT 0.0, "calcium_mg_per100g" REAL NOT NULL DEFAULT 0.0, "iron_mg_per100g" REAL NOT NULL DEFAULT 0.0, "zinc_mg_per100g" REAL NOT NULL DEFAULT 0.0, "vitamin_c_mg_per100g" REAL NOT NULL DEFAULT 0.0, "vitamin_d_ug_per100g" REAL NOT NULL DEFAULT 0.0, "vitamin_b12_ug_per100g" REAL NOT NULL DEFAULT 0.0, "is_favorite" INTEGER NOT NULL DEFAULT 0 CHECK ("is_favorite" IN (0, 1)), "last_used_at" TEXT NULL);
CREATE TABLE "meal_combos" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "name" TEXT NOT NULL, "default_meal_type" TEXT NOT NULL, "times_used" INTEGER NOT NULL DEFAULT 0);
CREATE TABLE "meal_combo_items" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "combo_id" INTEGER NOT NULL REFERENCES meal_combos (id) ON DELETE CASCADE, "food_item_id" INTEGER NOT NULL REFERENCES food_items (id), "base_grams" REAL NOT NULL);
CREATE TABLE "food_logs" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "date_key" TEXT NOT NULL, "logged_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "meal_type" TEXT NOT NULL, "food_item_id" INTEGER NOT NULL REFERENCES food_items (id), "grams_consumed" REAL NOT NULL, "portion_multiplier" REAL NOT NULL DEFAULT 1.0, "total_kcal" REAL NOT NULL, "total_protein_g" REAL NOT NULL, "total_carbs_g" REAL NOT NULL, "total_fat_g" REAL NOT NULL, "total_fiber_g" REAL NOT NULL, "total_sodium_mg" REAL NOT NULL, "total_potassium_mg" REAL NOT NULL, "total_magnesium_mg" REAL NOT NULL);
CREATE TABLE "fasting_sessions" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "protocol_name" TEXT NOT NULL, "target_hours" INTEGER NOT NULL, "started_at" TEXT NOT NULL, "planned_end_at" TEXT NOT NULL, "actual_ended_at" TEXT NULL, "is_completed" INTEGER NOT NULL DEFAULT 0 CHECK ("is_completed" IN (0, 1)), "notes" TEXT NULL);
CREATE TABLE "workout_blocks" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "date_key" TEXT NOT NULL, "performed_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "category" TEXT NOT NULL, "duration_minutes" INTEGER NOT NULL, "rpe_intensity" TEXT NOT NULL, "applied_met" REAL NOT NULL, "user_weight_at_time_kg" REAL NOT NULL, "net_calories_burned" INTEGER NOT NULL);
CREATE TABLE "journal_entries" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "date_key" TEXT NOT NULL, "created_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "content" TEXT NOT NULL, "progress_photo_path" TEXT NULL, "progress_photo" BLOB NULL, "mood_score" INTEGER NULL, "energy_level" INTEGER NULL, "stress_level" INTEGER NULL, "tags_json" TEXT NOT NULL DEFAULT '[]', "ai_insight_summary" TEXT NULL);
CREATE TABLE "pet_status" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "pet_name" TEXT NOT NULL DEFAULT 'Nibi', "coins_balance" INTEGER NOT NULL DEFAULT 50, "current_streak_days" INTEGER NOT NULL DEFAULT 0, "highest_streak_days" INTEGER NOT NULL DEFAULT 0, "streak_shields_available" INTEGER NOT NULL DEFAULT 1, "shield_active_until" TEXT NULL, "equipped_hat_id" TEXT NULL, "equipped_accessory_id" TEXT NULL, "equipped_color_id" TEXT NULL, "equipped_background_id" TEXT NOT NULL DEFAULT 'bg_default_room', "unlocked_items_json" TEXT NOT NULL DEFAULT '["bg_default_room"]', "last_ai_greeting" TEXT NULL, "last_greeting_at" TEXT NULL);
CREATE TABLE "coin_ledger" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "created_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "date_key" TEXT NOT NULL, "action_type" TEXT NOT NULL, "amount" INTEGER NOT NULL, "sleep_multiplier_applied" REAL NOT NULL DEFAULT 1.0);
CREATE TABLE "reminders" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "reminder_type" TEXT NOT NULL, "time_of_day_minutes" INTEGER NOT NULL, "days_of_week_bitmask" INTEGER NOT NULL DEFAULT 127, "is_enabled" INTEGER NOT NULL DEFAULT 1 CHECK ("is_enabled" IN (0, 1)), "interval_minutes" INTEGER NULL);
CREATE TABLE "tdee_calibrations" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "calibrated_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "date_key" TEXT NOT NULL, "previous_tdee_kcal" INTEGER NOT NULL, "new_tdee_kcal" INTEGER NOT NULL, "calculated_raw_tdee_kcal" INTEGER NOT NULL, "was_clamped" INTEGER NOT NULL DEFAULT 0 CHECK ("was_clamped" IN (0, 1)), "delta_ema14d" REAL NOT NULL, "average_kcal14d" REAL NOT NULL, "days_evaluated" INTEGER NOT NULL);
CREATE TABLE "health_sync_checkpoints" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "record_type" TEXT NOT NULL UNIQUE, "last_sync_timestamp" TEXT NOT NULL, "last_read_count" INTEGER NOT NULL DEFAULT 0, "last_write_count" INTEGER NOT NULL DEFAULT 0);
CREATE TABLE "recipes" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "name" TEXT NOT NULL, "description" TEXT NULL, "default_meal_type" TEXT NOT NULL DEFAULT 'lunch', "total_yield_servings" INTEGER NOT NULL DEFAULT 1, "cooked_total_weight_grams" REAL NULL, "kcal_per_serving" REAL NOT NULL, "protein_per_serving" REAL NOT NULL, "carbs_per_serving" REAL NOT NULL, "fat_per_serving" REAL NOT NULL, "fiber_per_serving" REAL NOT NULL DEFAULT 0.0, "sodium_mg_per_serving" REAL NOT NULL DEFAULT 0.0, "potassium_mg_per_serving" REAL NOT NULL DEFAULT 0.0, "magnesium_mg_per_serving" REAL NOT NULL DEFAULT 0.0, "created_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "last_used_at" TEXT NULL);
CREATE TABLE "recipe_ingredients" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "recipe_id" INTEGER NOT NULL REFERENCES recipes (id) ON DELETE CASCADE, "food_item_id" INTEGER NOT NULL REFERENCES food_items (id), "raw_grams" REAL NOT NULL);
CREATE TABLE "supplements_tracker" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "name" TEXT NOT NULL, "dosage" TEXT NOT NULL, "target_time_of_day_minutes" INTEGER NULL, "affects_water_retention" INTEGER NOT NULL DEFAULT 0 CHECK ("affects_water_retention" IN (0, 1)), "is_enabled" INTEGER NOT NULL DEFAULT 1 CHECK ("is_enabled" IN (0, 1)), "created_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "kind" TEXT NOT NULL DEFAULT 'supplement', "schedule_json" TEXT NOT NULL DEFAULT '[]', "days_of_week_bitmask" INTEGER NOT NULL DEFAULT 127, "stock_units" REAL NULL, "units_per_dose" REAL NOT NULL DEFAULT 1.0, "reminders_enabled" INTEGER NOT NULL DEFAULT 0 CHECK ("reminders_enabled" IN (0, 1)), "lowers_glucose" INTEGER NOT NULL DEFAULT 0 CHECK ("lowers_glucose" IN (0, 1)), "notes" TEXT NULL);
CREATE TABLE "supplement_logs" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "date_key" TEXT NOT NULL, "supplement_id" INTEGER NOT NULL REFERENCES supplements_tracker (id) ON DELETE CASCADE, "taken_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "was_taken" INTEGER NOT NULL DEFAULT 1 CHECK ("was_taken" IN (0, 1)), "slot_minutes" INTEGER NULL);
CREATE TABLE "female_cycle_logs" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "cycle_start_date" TEXT NOT NULL, "cycle_end_date" TEXT NULL, "cycle_length_days" INTEGER NOT NULL DEFAULT 28, "flow_intensity" TEXT NULL, "symptoms_json" TEXT NOT NULL DEFAULT '[]', "notes" TEXT NULL);
CREATE TABLE "digestive_logs" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "date_key" TEXT NOT NULL, "logged_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "bristol_type" INTEGER NOT NULL, "discomfort_level" INTEGER NOT NULL DEFAULT 1, "symptoms_json" TEXT NOT NULL DEFAULT '[]', "notes" TEXT NULL);
CREATE TABLE "caffeine_logs" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "date_key" TEXT NOT NULL, "logged_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP), "source_name" TEXT NOT NULL, "caffeine_mg" REAL NOT NULL);
CREATE TABLE "protocol_plans" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "plan_name" TEXT NOT NULL DEFAULT 'Mi Protocolo Semanal', "fasting_days_bitmask" TEXT NOT NULL DEFAULT '{"1":16,"2":16,"3":16,"4":16,"5":16,"6":0,"7":0}', "workout_slots_json" TEXT NOT NULL DEFAULT '[]', "diet_archetype" TEXT NOT NULL DEFAULT 'flexible', "is_current_active" INTEGER NOT NULL DEFAULT 1 CHECK ("is_current_active" IN (0, 1)), "updated_at" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP));
CREATE TABLE "milestone_goals" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "milestone_order" INTEGER NOT NULL, "target_weight_kg" REAL NOT NULL, "is_completed" INTEGER NOT NULL DEFAULT 0 CHECK ("is_completed" IN (0, 1)), "completed_at" TEXT NULL, "estimated_completion_date" TEXT NULL);
CREATE TABLE "ui_preferences" ("pref_key" TEXT NOT NULL, "pref_value" TEXT NOT NULL, PRIMARY KEY ("pref_key"));
CREATE TABLE "care_sessions" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "date_key" TEXT NOT NULL, "completed_at" TEXT NOT NULL, "kind" TEXT NOT NULL, "variant" TEXT NOT NULL, "duration_seconds" INTEGER NOT NULL);
CREATE TABLE "custom_habits" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "name" TEXT NOT NULL, "icon_key" TEXT NOT NULL DEFAULT 'spark', "days_of_week_bitmask" INTEGER NOT NULL DEFAULT 127, "sort_order" INTEGER NOT NULL DEFAULT 0, "is_archived" INTEGER NOT NULL DEFAULT 0 CHECK ("is_archived" IN (0, 1)), "created_at" TEXT NOT NULL);
CREATE TABLE "habit_logs" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "habit_id" INTEGER NOT NULL REFERENCES custom_habits (id) ON DELETE CASCADE, "date_key" TEXT NOT NULL, "logged_at" TEXT NOT NULL);
CREATE TABLE "checkups" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "catalog_key" TEXT NOT NULL, "custom_name" TEXT NULL, "interval_months" INTEGER NOT NULL, "is_enabled" INTEGER NOT NULL DEFAULT 1 CHECK ("is_enabled" IN (0, 1)));
CREATE TABLE "checkup_logs" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "checkup_id" INTEGER NOT NULL REFERENCES checkups (id) ON DELETE CASCADE, "done_at" TEXT NOT NULL, "notes" TEXT NULL);
CREATE TABLE "symptom_logs" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "date_key" TEXT NOT NULL, "logged_at" TEXT NOT NULL, "symptom" TEXT NOT NULL, "intensity" INTEGER NOT NULL, "notes" TEXT NULL);
CREATE TABLE "vital_readings" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "kind" TEXT NOT NULL, "measured_at" TEXT NOT NULL, "date_key" TEXT NOT NULL, "primary_value" REAL NOT NULL, "secondary_value" REAL NULL, "context" TEXT NULL, "source" TEXT NOT NULL DEFAULT 'manual', "external_id" TEXT NULL UNIQUE);
CREATE TABLE "substance_logs" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "date_key" TEXT NOT NULL, "logged_at" TEXT NOT NULL, "kind" TEXT NOT NULL, "preset_key" TEXT NOT NULL, "standard_drinks" REAL NOT NULL DEFAULT 0.0, "kcal" REAL NOT NULL DEFAULT 0.0, "units" INTEGER NOT NULL DEFAULT 1, "food_log_id" INTEGER NULL);
CREATE TABLE "wellbeing_checks" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "date_key" TEXT NOT NULL, "created_at" TEXT NOT NULL, "answers_json" TEXT NOT NULL, "percent_score" INTEGER NOT NULL);
CREATE TABLE "quit_trackers" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "preset_key" TEXT NOT NULL, "custom_name" TEXT NULL, "started_at" TEXT NOT NULL, "first_started_at" TEXT NOT NULL, "longest_streak_seconds" INTEGER NOT NULL DEFAULT 0, "banked_seconds" INTEGER NOT NULL DEFAULT 0, "daily_cost" REAL NULL, "daily_units" REAL NULL, "daily_minutes" INTEGER NULL, "motivation" TEXT NULL, "celebrated_milestones" INTEGER NOT NULL DEFAULT 0, "is_archived" INTEGER NOT NULL DEFAULT 0 CHECK ("is_archived" IN (0, 1)), "created_at" TEXT NOT NULL);
CREATE TABLE "quit_events" ("id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, "tracker_id" INTEGER NOT NULL REFERENCES quit_trackers (id) ON DELETE CASCADE, "kind" TEXT NOT NULL, "occurred_at" TEXT NOT NULL, "streak_seconds" INTEGER NULL, "note" TEXT NULL);
CREATE INDEX idx_body_measurements_date ON body_measurements (date_key DESC, measured_at DESC);
CREATE INDEX idx_hydration_logs_date ON hydration_logs (date_key, logged_at);
CREATE INDEX idx_food_logs_date_meal ON food_logs (date_key, meal_type);
CREATE INDEX idx_food_logs_food_item ON food_logs (food_item_id);
CREATE INDEX idx_fasting_sessions_started ON fasting_sessions (started_at);
CREATE INDEX idx_workout_blocks_date ON workout_blocks (date_key);
CREATE INDEX idx_journal_entries_date ON journal_entries (date_key);
CREATE UNIQUE INDEX idx_coin_ledger_daily_unique ON coin_ledger (date_key, action_type) WHERE action_type != 'shop_buy';
CREATE INDEX idx_recipes_name ON recipes (name);
CREATE INDEX idx_supplement_logs_date ON supplement_logs (date_key);
CREATE INDEX idx_digestive_logs_date ON digestive_logs (date_key);
CREATE INDEX idx_caffeine_logs_date ON caffeine_logs (date_key);
CREATE INDEX idx_care_sessions_date ON care_sessions (date_key);
CREATE UNIQUE INDEX idx_habit_logs_unique ON habit_logs (habit_id, date_key);
CREATE INDEX idx_symptom_logs_date ON symptom_logs (date_key);
CREATE INDEX idx_vital_readings_kind ON vital_readings (kind, measured_at);
CREATE INDEX idx_substance_logs_date ON substance_logs (date_key);
CREATE INDEX idx_quit_events_tracker ON quit_events (tracker_id, occurred_at);

-- Índice de búsqueda de alimentos sin diacríticos, el mismo del teléfono.
CREATE VIRTUAL TABLE food_items_fts USING fts5(
    name,
    brand,
    content='food_items',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);
CREATE TRIGGER food_items_ai AFTER INSERT ON food_items BEGIN
    INSERT INTO food_items_fts(rowid, name, brand) VALUES (new.id, new.name, new.brand);
END;
CREATE TRIGGER food_items_ad AFTER DELETE ON food_items BEGIN
    INSERT INTO food_items_fts(food_items_fts, rowid, name, brand) VALUES('delete', old.id, old.name, old.brand);
END;
CREATE TRIGGER food_items_au AFTER UPDATE ON food_items BEGIN
    INSERT INTO food_items_fts(food_items_fts, rowid, name, brand) VALUES('delete', old.id, old.name, old.brand);
    INSERT INTO food_items_fts(rowid, name, brand) VALUES (new.id, new.name, new.brand);
END;

-- Propias del escritorio ------------------------------------------------------

-- Preferencias de la aplicación (idioma, tema, avisos, política de red), cada una
-- como JSON. La interfaz solo escribe las claves que `preferencias.rs` admite.
CREATE TABLE "preferencias" (
    "clave" TEXT NOT NULL PRIMARY KEY,
    "valor" TEXT NOT NULL
) STRICT;

-- Metadatos de la instalación: cuándo se creó la base y con qué versión.
CREATE TABLE "instalacion" (
    "clave" TEXT NOT NULL PRIMARY KEY,
    "valor" TEXT NOT NULL
) STRICT;
INSERT INTO "instalacion" ("clave", "valor") VALUES ('creada', strftime('%Y-%m-%dT%H:%M:%S', 'now'));
