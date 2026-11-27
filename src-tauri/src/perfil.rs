//! El perfil de la persona: quién es, qué quiere y qué declara.
//!
//! Hay un solo perfil por base (ADR-017 del teléfono): sin cuentas ni varias
//! personas. La bienvenida lo crea con el primer pesaje, que es el punto de
//! partida del peso tendencia, igual que en NB SelfCare Mobile.

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{self, fallo, BaseDeDatos};
use crate::error::{ScError, ScResult};
use crate::fechas;
use crate::formulas::{self, Arquetipo, FaseDieta, Sexo};

/// Largo máximo del nombre, el de la tabla del teléfono.
pub const MAX_NOMBRE: usize = 60;

/// Los módulos de Cuidado, con las claves del teléfono.
pub const MODULOS: [&str; 8] = [
    "treatments",
    "pelvic_floor",
    "active_breaks",
    "symptoms",
    "checkups",
    "habits",
    "substances",
    "quit",
];

/// Los aparatos de signos vitales que se pueden declarar.
pub const APARATOS: [&str; 4] = ["blood_pressure", "glucose", "oxygen", "temperature"];

/// El perfil tal como lo ve la interfaz.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Perfil {
    pub nombre: String,
    /// `YYYY-MM-DD`.
    pub nacimiento: String,
    pub edad: i64,
    pub sexo: Sexo,
    pub altura_cm: f64,
    pub peso_meta_kg: f64,
    pub ritmo_semanal_kg: f64,
    pub agua_meta_ml: i64,
    pub pasos_meta: i64,
    pub sueno_meta_h: f64,
    pub gasto_adaptativo: i64,
    /// `YYYY-MM-DD` de la última calibración, si la hubo.
    pub ultima_calibracion: Option<String>,
    pub ciclo_activo: bool,
    pub ayuno_activo: bool,
    pub macros_a_mano: bool,
    pub kcal_a_mano: Option<i64>,
    pub proteina_a_mano: Option<i64>,
    pub carbohidratos_a_mano: Option<i64>,
    pub grasa_a_mano: Option<i64>,
    pub arquetipo: Arquetipo,
    pub fase: FaseDieta,
    /// `YYYY-MM-DD` desde el que rige la fase.
    pub fase_desde: Option<String>,
    pub corte_cafeina_h: f64,
    pub antecedentes_tca: bool,
    pub toma_hipoglucemiantes: bool,
    pub modulos: Vec<String>,
    pub aparatos: Vec<String>,
    pub avisos_discretos: bool,
    /// Minutos desde medianoche.
    pub hora_dormir: i64,
}

/// Lo que se lee de la fila con sus fechas ya interpretadas.
#[derive(Debug, Clone)]
pub struct Fila {
    pub perfil: Perfil,
    pub nacimiento: chrono::NaiveDateTime,
    pub fase_desde: Option<chrono::NaiveDateTime>,
    pub ultima_calibracion: Option<chrono::NaiveDateTime>,
}

fn lista_json(texto: &str, validas: &[&str]) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(texto)
        .unwrap_or_default()
        .into_iter()
        .filter(|v| validas.contains(&v.as_str()))
        .collect()
}

fn de_fila(f: &Row<'_>) -> rusqlite::Result<Fila> {
    let nacimiento_txt: String = f.get("birth_date")?;
    let nacimiento = fechas::leer(&nacimiento_txt).unwrap_or_default();
    let fase_desde = f
        .get::<_, Option<String>>("active_diet_phase_started_at")?
        .and_then(|t| fechas::leer(&t));
    let ultima = f
        .get::<_, Option<String>>("last_tdee_calibration")?
        .and_then(|t| fechas::leer(&t));
    let perfil = Perfil {
        nombre: f.get("name")?,
        nacimiento: fechas::clave(nacimiento.date()),
        edad: fechas::edad(nacimiento, fechas::ahora()).floor() as i64,
        sexo: Sexo::desde_db(&f.get::<_, String>("biological_sex")?),
        altura_cm: f.get("height_cm")?,
        peso_meta_kg: f.get("target_weight_kg")?,
        ritmo_semanal_kg: f.get("weekly_goal_rate_kg")?,
        agua_meta_ml: f.get("daily_water_goal_ml")?,
        pasos_meta: f.get("daily_steps_goal")?,
        sueno_meta_h: f.get("daily_sleep_goal_hours")?,
        gasto_adaptativo: f.get("current_adaptive_tdee_kcal")?,
        ultima_calibracion: ultima.map(|u| fechas::clave(u.date())),
        ciclo_activo: f.get("is_cycle_tracking_enabled")?,
        ayuno_activo: f.get("is_fasting_enabled")?,
        macros_a_mano: f.get("is_manual_macro_mode")?,
        kcal_a_mano: f.get("manual_kcal_target")?,
        proteina_a_mano: f.get("manual_protein_g")?,
        carbohidratos_a_mano: f.get("manual_carbs_g")?,
        grasa_a_mano: f.get("manual_fat_g")?,
        arquetipo: Arquetipo::desde_db(&f.get::<_, String>("active_diet_archetype")?),
        fase: FaseDieta::desde_db(&f.get::<_, String>("active_diet_phase")?),
        fase_desde: fase_desde.map(|d| fechas::clave(d.date())),
        corte_cafeina_h: f.get("caffeine_bedtime_cutoff_hours")?,
        antecedentes_tca: f.get("has_eating_disorder_history")?,
        toma_hipoglucemiantes: f.get("takes_hypoglycemic_medication")?,
        modulos: lista_json(&f.get::<_, String>("care_modules_json")?, &MODULOS),
        aparatos: lista_json(&f.get::<_, String>("vital_devices_json")?, &APARATOS),
        avisos_discretos: f.get("discreet_notifications")?,
        hora_dormir: f.get("target_bedtime_minutes")?,
    };
    Ok(Fila {
        perfil,
        nacimiento,
        fase_desde,
        ultima_calibracion: ultima,
    })
}

/// El perfil, si ya se completó la bienvenida.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn leer(conn: &Connection) -> ScResult<Option<Fila>> {
    conn.query_row(
        "SELECT * FROM user_profiles ORDER BY id LIMIT 1",
        [],
        de_fila,
    )
    .optional()
    .map_err(fallo)
}

/// Como [`leer`], pero sin perfil es un error.
///
/// # Errors
///
/// [`ScError::NotFound`] si no hay perfil.
pub fn exigir(conn: &Connection) -> ScResult<Fila> {
    leer(conn)?.ok_or(ScError::NotFound)
}

/// El último peso real (no las filas de cinta), si lo hay.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn ultimo_peso(conn: &Connection) -> ScResult<Option<(f64, f64)>> {
    conn.query_row(
        "SELECT weight_kg, trend_weight_kg FROM body_measurements
         WHERE source != 'manual_navy_tape'
         ORDER BY measured_at DESC, id DESC LIMIT 1",
        [],
        |f| Ok((f.get(0)?, f.get(1)?)),
    )
    .optional()
    .map_err(fallo)
}

/// Los datos que piden las fórmulas de las metas.
#[must_use]
pub fn datos_metas(fila: &Fila, peso_actual: Option<f64>) -> formulas::DatosMetas {
    let p = &fila.perfil;
    let ahora = fechas::ahora();
    formulas::DatosMetas {
        sexo: p.sexo,
        edad: fechas::edad(fila.nacimiento, ahora),
        altura_cm: p.altura_cm,
        peso_meta_kg: p.peso_meta_kg,
        peso_actual_kg: peso_actual,
        gasto_adaptativo: p.gasto_adaptativo,
        fase: p.fase,
        ritmo_semanal_kg: p.ritmo_semanal_kg,
        dias_en_la_fase: fila.fase_desde.map(|d| fechas::dias_entre(d, ahora)),
        antecedentes_tca: p.antecedentes_tca,
        arquetipo: p.arquetipo,
        manual: (p.macros_a_mano)
            .then_some(p.kcal_a_mano)
            .flatten()
            .map(|k| (k, p.proteina_a_mano, p.carbohidratos_a_mano, p.grasa_a_mano)),
    }
}

/// Las respuestas de la bienvenida.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Bienvenida {
    pub nombre: String,
    /// `YYYY-MM-DD`.
    pub nacimiento: String,
    pub sexo: Sexo,
    pub altura_cm: f64,
    pub peso_actual_kg: f64,
    pub peso_meta_kg: f64,
    pub ritmo_semanal_kg: f64,
    pub agua_meta_ml: i64,
    pub pasos_meta: i64,
    pub sueno_meta_h: f64,
    pub antecedentes_tca: bool,
    pub toma_hipoglucemiantes: bool,
    pub aparatos: Vec<String>,
}

fn nombre_seguro(nombre: &str) -> ScResult<String> {
    let n = nombre.trim();
    if n.is_empty() {
        return Err(ScError::InvalidInput);
    }
    Ok(n.chars()
        .take(MAX_NOMBRE)
        .collect::<String>()
        .trim_end()
        .to_owned())
}

fn validar(b: &Bienvenida) -> ScResult<chrono::NaiveDate> {
    let nacimiento = fechas::de_clave(&b.nacimiento).ok_or(ScError::InvalidInput)?;
    let edad = fechas::edad(fechas::inicio(nacimiento), fechas::ahora());
    let ok = (13.0..=110.0).contains(&edad)
        && (100.0..=250.0).contains(&b.altura_cm)
        && (25.0..=350.0).contains(&b.peso_actual_kg)
        && (25.0..=350.0).contains(&b.peso_meta_kg)
        && (-1.5..=1.0).contains(&b.ritmo_semanal_kg)
        && (500..=6000).contains(&b.agua_meta_ml)
        && (0..=60_000).contains(&b.pasos_meta)
        && (3.0..=14.0).contains(&b.sueno_meta_h)
        && b.aparatos.iter().all(|a| APARATOS.contains(&a.as_str()));
    if ok {
        Ok(nacimiento)
    } else {
        Err(ScError::InvalidInput)
    }
}

/// Crea el perfil y su primer pesaje. Si ya había perfil, no hace nada.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si una respuesta está fuera de rango;
/// [`ScError::StorageFailure`] si la escritura falla.
pub fn completar_bienvenida(db: &BaseDeDatos, b: &Bienvenida) -> ScResult<()> {
    let nombre = nombre_seguro(&b.nombre)?;
    let nacimiento = validar(b)?;
    let ahora = fechas::ahora();
    let edad = fechas::edad(fechas::inicio(nacimiento), ahora);
    let gasto = formulas::gasto_inicial(b.peso_actual_kg, b.altura_cm, edad, b.sexo, b.pasos_meta);
    let fase = if b.ritmo_semanal_kg == 0.0 {
        FaseDieta::Mantenimiento
    } else {
        FaseDieta::Deficit
    };
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let ya: i64 = tx
        .query_row("SELECT count(*) FROM user_profiles", [], |f| f.get(0))
        .map_err(fallo)?;
    if ya > 0 {
        return Ok(());
    }
    let ahora_txt = fechas::texto(ahora);
    tx.execute(
        "INSERT INTO user_profiles (
            name, birth_date, biological_sex, height_cm, target_weight_kg,
            weekly_goal_rate_kg, daily_water_goal_ml, daily_steps_goal,
            daily_sleep_goal_hours, current_adaptive_tdee_kcal, active_diet_phase,
            active_diet_phase_started_at, has_eating_disorder_history,
            takes_hypoglycemic_medication, vital_devices_json, created_at, theme_mode
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?12, 'samsung_dark')",
        params![
            nombre,
            fechas::texto(fechas::inicio(nacimiento)),
            b.sexo.a_db(),
            b.altura_cm,
            b.peso_meta_kg,
            b.ritmo_semanal_kg,
            b.agua_meta_ml,
            b.pasos_meta,
            b.sueno_meta_h,
            gasto,
            fase.a_db(),
            ahora_txt,
            b.antecedentes_tca,
            b.toma_hipoglucemiantes,
            serde_json::to_string(&b.aparatos).unwrap_or_else(|_| "[]".into()),
        ],
    )
    .map_err(fallo)?;
    crate::nibi::asegurar(&tx)?;
    crate::cuerpo::insertar_pesaje(&tx, b.peso_actual_kg, ahora)?;
    crate::cuerpo::regenerar_hitos(&tx, b.peso_actual_kg, b.peso_meta_kg)?;
    tx.commit().map_err(fallo)?;
    Ok(())
}

/// Un cambio del perfil desde Perfil o Ajustes. Solo se aplican los campos
/// presentes.
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct CambioPerfil {
    #[ts(optional)]
    pub nombre: Option<String>,
    #[ts(optional)]
    pub altura_cm: Option<f64>,
    #[ts(optional)]
    pub peso_meta_kg: Option<f64>,
    #[ts(optional)]
    pub ritmo_semanal_kg: Option<f64>,
    #[ts(optional)]
    pub agua_meta_ml: Option<i64>,
    #[ts(optional)]
    pub pasos_meta: Option<i64>,
    #[ts(optional)]
    pub sueno_meta_h: Option<f64>,
    #[ts(optional)]
    pub ciclo_activo: Option<bool>,
    #[ts(optional)]
    pub ayuno_activo: Option<bool>,
    #[ts(optional)]
    pub arquetipo: Option<Arquetipo>,
    #[ts(optional)]
    pub fase: Option<FaseDieta>,
    #[ts(optional)]
    pub antecedentes_tca: Option<bool>,
    #[ts(optional)]
    pub toma_hipoglucemiantes: Option<bool>,
    #[ts(optional)]
    pub modulos: Option<Vec<String>>,
    #[ts(optional)]
    pub aparatos: Option<Vec<String>>,
    #[ts(optional)]
    pub avisos_discretos: Option<bool>,
    #[ts(optional)]
    pub hora_dormir: Option<i64>,
    /// Metas a mano: `Some(None)` las apaga.
    #[ts(optional)]
    pub macros_a_mano: Option<Option<MacrosAMano>>,
}

/// Las metas fijadas a mano (prescripción médica o deportiva).
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct MacrosAMano {
    pub kcal: i64,
    pub proteina_g: Option<i64>,
    pub carbohidratos_g: Option<i64>,
    pub grasa_g: Option<i64>,
}

fn rango<T: PartialOrd>(v: Option<T>, min: T, max: T) -> ScResult<()> {
    match v {
        Some(x) if !(min..=max).contains(&x) => Err(ScError::InvalidInput),
        _ => Ok(()),
    }
}

/// Aplica un cambio del perfil.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si un valor está fuera de rango;
/// [`ScError::NotFound`] si aún no hay perfil.
pub fn cambiar(db: &BaseDeDatos, c: &CambioPerfil) -> ScResult<()> {
    rango(c.altura_cm, 100.0, 250.0)?;
    rango(c.peso_meta_kg, 25.0, 350.0)?;
    rango(c.ritmo_semanal_kg, -1.5, 1.0)?;
    rango(c.agua_meta_ml, 500, 6000)?;
    rango(c.pasos_meta, 0, 60_000)?;
    rango(c.sueno_meta_h, 3.0, 14.0)?;
    rango(c.hora_dormir, 0, 1439)?;
    if let Some(Some(m)) = c.macros_a_mano {
        rango(Some(m.kcal), 800, 6000)?;
    }
    if c.modulos
        .as_ref()
        .is_some_and(|m| m.iter().any(|x| !MODULOS.contains(&x.as_str())))
        || c.aparatos
            .as_ref()
            .is_some_and(|m| m.iter().any(|x| !APARATOS.contains(&x.as_str())))
    {
        return Err(ScError::InvalidInput);
    }
    let nombre = c.nombre.as_deref().map(nombre_seguro).transpose()?;
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let actual = exigir(&tx)?;
    let mut set: Vec<(&str, rusqlite::types::Value)> = Vec::new();
    use rusqlite::types::Value as V;
    let real = |v: f64| V::Real(v);
    let ent = |v: i64| V::Integer(v);
    let bool_ = |v: bool| V::Integer(i64::from(v));
    if let Some(n) = nombre {
        set.push(("name", V::Text(n)));
    }
    if let Some(v) = c.altura_cm {
        set.push(("height_cm", real(v)));
    }
    if let Some(v) = c.peso_meta_kg {
        set.push(("target_weight_kg", real(v)));
    }
    if let Some(v) = c.ritmo_semanal_kg {
        set.push(("weekly_goal_rate_kg", real(v)));
    }
    if let Some(v) = c.agua_meta_ml {
        set.push(("daily_water_goal_ml", ent(v)));
    }
    if let Some(v) = c.pasos_meta {
        set.push(("daily_steps_goal", ent(v)));
    }
    if let Some(v) = c.sueno_meta_h {
        set.push(("daily_sleep_goal_hours", real(v)));
    }
    if let Some(v) = c.ciclo_activo {
        set.push(("is_cycle_tracking_enabled", bool_(v)));
    }
    if let Some(v) = c.ayuno_activo {
        set.push(("is_fasting_enabled", bool_(v)));
    }
    if let Some(v) = c.arquetipo {
        set.push(("active_diet_archetype", V::Text(v.a_db().into())));
    }
    if let Some(v) = c.fase {
        if v != actual.perfil.fase {
            set.push(("active_diet_phase", V::Text(v.a_db().into())));
            set.push((
                "active_diet_phase_started_at",
                V::Text(fechas::texto(fechas::ahora())),
            ));
        }
    }
    if let Some(v) = c.antecedentes_tca {
        set.push(("has_eating_disorder_history", bool_(v)));
    }
    if let Some(v) = c.toma_hipoglucemiantes {
        set.push(("takes_hypoglycemic_medication", bool_(v)));
    }
    if let Some(v) = &c.modulos {
        // En el orden del catálogo, como el teléfono.
        let ordenados: Vec<&str> = MODULOS
            .iter()
            .copied()
            .filter(|m| v.iter().any(|x| x == m))
            .collect();
        set.push((
            "care_modules_json",
            V::Text(serde_json::to_string(&ordenados).unwrap_or_default()),
        ));
    }
    if let Some(v) = &c.aparatos {
        let ordenados: Vec<&str> = APARATOS
            .iter()
            .copied()
            .filter(|m| v.iter().any(|x| x == m))
            .collect();
        set.push((
            "vital_devices_json",
            V::Text(serde_json::to_string(&ordenados).unwrap_or_default()),
        ));
    }
    if let Some(v) = c.avisos_discretos {
        set.push(("discreet_notifications", bool_(v)));
    }
    if let Some(v) = c.hora_dormir {
        set.push(("target_bedtime_minutes", ent(v)));
    }
    if let Some(m) = c.macros_a_mano {
        let opt = |v: Option<i64>| v.map_or(V::Null, V::Integer);
        match m {
            Some(m) => {
                set.push(("is_manual_macro_mode", bool_(true)));
                set.push(("manual_kcal_target", ent(m.kcal)));
                set.push(("manual_protein_g", opt(m.proteina_g)));
                set.push(("manual_carbs_g", opt(m.carbohidratos_g)));
                set.push(("manual_fat_g", opt(m.grasa_g)));
            }
            None => set.push(("is_manual_macro_mode", bool_(false))),
        }
    }
    if set.is_empty() {
        return Ok(());
    }
    let columnas: Vec<String> = set
        .iter()
        .enumerate()
        .map(|(i, (c, _))| format!("{c} = ?{}", i + 1))
        .collect();
    let sql = format!("UPDATE user_profiles SET {}", columnas.join(", "));
    tx.execute(
        &sql,
        rusqlite::params_from_iter(set.into_iter().map(|(_, v)| v)),
    )
    .map_err(fallo)?;
    if c.peso_meta_kg.is_some() {
        if let Some((peso, _)) = ultimo_peso(&tx)? {
            let meta = c.peso_meta_kg.unwrap_or(actual.perfil.peso_meta_kg);
            crate::cuerpo::regenerar_hitos(&tx, peso, meta)?;
        }
    }
    tx.commit().map_err(fallo)?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod pruebas {
    use super::*;

    pub fn bienvenida() -> Bienvenida {
        Bienvenida {
            nombre: "Alex".into(),
            nacimiento: "1990-05-10".into(),
            sexo: Sexo::Hombre,
            altura_cm: 178.0,
            peso_actual_kg: 82.0,
            peso_meta_kg: 76.0,
            ritmo_semanal_kg: -0.5,
            agua_meta_ml: 2500,
            pasos_meta: 8000,
            sueno_meta_h: 7.5,
            antecedentes_tca: false,
            toma_hipoglucemiantes: false,
            aparatos: vec![],
        }
    }

    /// Una base con perfil, como la deja la bienvenida.
    pub fn base_con_perfil() -> BaseDeDatos {
        fechas::fijar(
            chrono::NaiveDateTime::parse_from_str("2026-11-20 09:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
        );
        let db = BaseDeDatos::en_memoria().unwrap();
        completar_bienvenida(&db, &bienvenida()).unwrap();
        db
    }

    #[test]
    fn la_bienvenida_deja_perfil_primer_pesaje_hitos_y_a_nibi() {
        let db = base_con_perfil();
        let conn = db.conexion().unwrap();
        let p = exigir(&conn).unwrap().perfil;
        assert_eq!((p.nombre.as_str(), p.edad), ("Alex", 36));
        assert_eq!(p.fase, FaseDieta::Deficit);
        assert_eq!(ultimo_peso(&conn).unwrap(), Some((82.0, 82.0)));
        let hitos: i64 = conn
            .query_row("SELECT count(*) FROM milestone_goals", [], |f| f.get(0))
            .unwrap();
        assert_eq!(hitos, 4);
        let monedas: i64 = conn
            .query_row("SELECT coins_balance FROM pet_status", [], |f| f.get(0))
            .unwrap();
        assert!(monedas >= 50);
    }

    #[test]
    fn la_bienvenida_rechaza_lo_imposible() {
        fechas::fijar(
            chrono::NaiveDateTime::parse_from_str("2026-11-20 09:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
        );
        let db = BaseDeDatos::en_memoria().unwrap();
        let mut b = bienvenida();
        b.altura_cm = 20.0;
        assert_eq!(completar_bienvenida(&db, &b), Err(ScError::InvalidInput));
        let mut b = bienvenida();
        b.nombre = "   ".into();
        assert_eq!(completar_bienvenida(&db, &b), Err(ScError::InvalidInput));
    }

    #[test]
    fn cambiar_de_fase_apunta_cuando_empezo() {
        let db = base_con_perfil();
        cambiar(
            &db,
            &CambioPerfil {
                fase: Some(FaseDieta::Inversa),
                ..Default::default()
            },
        )
        .unwrap();
        let conn = db.conexion().unwrap();
        let f = exigir(&conn).unwrap();
        assert_eq!(f.perfil.fase, FaseDieta::Inversa);
        assert_eq!(f.perfil.fase_desde.as_deref(), Some("2026-11-20"));
    }

    #[test]
    fn los_modulos_se_guardan_en_el_orden_del_catalogo_y_solo_los_conocidos() {
        let db = base_con_perfil();
        cambiar(
            &db,
            &CambioPerfil {
                modulos: Some(vec!["quit".into(), "treatments".into()]),
                ..Default::default()
            },
        )
        .unwrap();
        let conn = db.conexion().unwrap();
        assert_eq!(
            exigir(&conn).unwrap().perfil.modulos,
            vec!["treatments", "quit"]
        );
        drop(conn);
        assert_eq!(
            cambiar(
                &db,
                &CambioPerfil {
                    modulos: Some(vec!["casino".into()]),
                    ..Default::default()
                }
            ),
            Err(ScError::InvalidInput)
        );
    }
}
