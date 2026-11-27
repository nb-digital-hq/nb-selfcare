//! El diario (ánimo, energía, estrés y una nota), el sueño de la noche, la
//! cafeína y el ciclo menstrual.
//!
//! En el escritorio no hay Health Connect: el sueño y el pulso en reposo se
//! anotan a mano y van a `daily_summaries`, la misma tabla que el teléfono
//! llena desde Health Connect, así que la disposición del día se calcula igual.

use chrono::{NaiveDateTime, Timelike};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{self, fallo, BaseDeDatos};
use crate::error::{ScError, ScResult};
use crate::fechas;
use crate::nibi::{self, Accion, Recompensa};
use crate::perfil;

/// Una entrada del diario.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Entrada {
    pub id: i64,
    pub dia: String,
    pub hora: String,
    pub texto: String,
    pub animo: Option<i64>,
    pub energia: Option<i64>,
    pub estres: Option<i64>,
    pub etiquetas: Vec<String>,
}

/// Una entrada nueva.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct NuevaEntrada {
    pub texto: String,
    pub animo: Option<i64>,
    pub energia: Option<i64>,
    pub estres: Option<i64>,
    pub etiquetas: Vec<String>,
}

/// Las últimas entradas.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn entradas(conn: &Connection, limite: i64) -> ScResult<Vec<Entrada>> {
    let mut st = conn
        .prepare(
            "SELECT id, date_key, created_at, content, mood_score, energy_level, stress_level, tags_json
             FROM journal_entries ORDER BY created_at DESC LIMIT ?1",
        )
        .map_err(fallo)?;
    let v = st
        .query_map([limite.clamp(1, 200)], |f| {
            let cuando = fechas::leer(&f.get::<_, String>(2)?).unwrap_or_default();
            Ok(Entrada {
                id: f.get(0)?,
                dia: f.get(1)?,
                hora: cuando.format("%H:%M").to_string(),
                texto: f.get(3)?,
                animo: f.get(4)?,
                energia: f.get(5)?,
                estres: f.get(6)?,
                etiquetas: serde_json::from_str(&f.get::<_, String>(7)?).unwrap_or_default(),
            })
        })
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    Ok(v)
}

/// Escribe una entrada. La primera del día da monedas.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si está vacía del todo o los valores no son 1–5.
pub fn escribir(db: &BaseDeDatos, e: &NuevaEntrada) -> ScResult<Option<Recompensa>> {
    let texto = e.texto.trim();
    let escala = |v: Option<i64>| v.is_none_or(|x| (1..=5).contains(&x));
    if (texto.is_empty() && e.animo.is_none())
        || texto.chars().count() > 10_000
        || !escala(e.animo)
        || !escala(e.energia)
        || !escala(e.estres)
        || e.etiquetas.len() > 8
    {
        return Err(ScError::InvalidInput);
    }
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let ahora = fechas::ahora();
    tx.execute(
        "INSERT INTO journal_entries (date_key, created_at, content, mood_score, energy_level, stress_level, tags_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            fechas::clave(ahora.date()),
            fechas::texto(ahora),
            texto,
            e.animo,
            e.energia,
            e.estres,
            serde_json::to_string(&e.etiquetas).unwrap_or_else(|_| "[]".into())
        ],
    )
    .map_err(fallo)?;
    let r = nibi::premiar(&tx, Accion::Diario)?;
    tx.commit().map_err(fallo)?;
    Ok(r)
}

/// Borra una entrada.
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn borrar(db: &BaseDeDatos, id: i64) -> ScResult<()> {
    let conn = db.conexion()?;
    if conn
        .execute("DELETE FROM journal_entries WHERE id = ?1", [id])
        .map_err(fallo)?
        == 0
    {
        return Err(ScError::NotFound);
    }
    Ok(())
}

// --- Sueño -----------------------------------------------------------------

/// El sueño y el pulso de un día.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Noche {
    pub minutos: i64,
    /// Hora de despertar en minutos desde medianoche.
    pub despertar: Option<i64>,
    pub pulso: Option<i64>,
    pub pasos: Option<i64>,
}

/// Lo anotado hoy, si algo.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn noche(conn: &Connection, dia: &str) -> ScResult<Option<Noche>> {
    conn.query_row(
        "SELECT sleep_duration_minutes, wake_time_minutes, resting_heart_rate_bpm, steps_count
         FROM daily_summaries WHERE date_key = ?1",
        [dia],
        |f| {
            Ok(Noche {
                minutos: f.get(0)?,
                despertar: f.get(1)?,
                pulso: f.get(2)?,
                pasos: Some(f.get::<_, i64>(3)?).filter(|p| *p > 0),
            })
        },
    )
    .optional()
    .map_err(fallo)
}

/// Anota la noche de hoy (sueño, despertar, pulso y pasos). Las metas del día
/// quedan fotografiadas en la misma fila, como en el teléfono.
///
/// # Errors
///
/// [`ScError::InvalidInput`] fuera de rango; [`ScError::NotFound`] sin perfil.
pub fn anotar_noche(db: &BaseDeDatos, n: Noche) -> ScResult<()> {
    if !(0..=16 * 60).contains(&n.minutos)
        || n.despertar.is_some_and(|d| !(0..1440).contains(&d))
        || n.pulso.is_some_and(|p| !(30..=140).contains(&p))
        || n.pasos.is_some_and(|p| !(0..=200_000).contains(&p))
    {
        return Err(ScError::InvalidInput);
    }
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let dia = fechas::clave(fechas::hoy());
    let m = crate::comida::metas_del_dia(&tx, &dia)?;
    let pasos = n.pasos.unwrap_or(0);
    tx.execute(
        "INSERT INTO daily_summaries (date_key, steps_count, active_calories_from_steps, sleep_duration_minutes,
            resting_heart_rate_bpm, wake_time_minutes, calorie_target_kcal, protein_target_g, carbs_target_g, fat_target_g)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT (date_key) DO UPDATE SET steps_count = excluded.steps_count,
            active_calories_from_steps = excluded.active_calories_from_steps,
            sleep_duration_minutes = excluded.sleep_duration_minutes,
            resting_heart_rate_bpm = excluded.resting_heart_rate_bpm,
            wake_time_minutes = excluded.wake_time_minutes",
        params![
            dia,
            pasos,
            crate::formulas::kcal_de_pasos(pasos),
            n.minutos,
            n.pulso,
            n.despertar,
            m.kcal,
            m.proteina_g as i64,
            m.carbohidratos_g as i64,
            m.grasa_g as i64
        ],
    )
    .map_err(fallo)?;
    tx.commit().map_err(fallo)
}

/// La disposición del día: sueño (40), pulso en reposo (35) y carga de los
/// últimos 3 días (25), con las reglas del teléfono.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Disposicion {
    pub total: i64,
    /// `optimal`, `favorable` o `recoveryNeeded`.
    pub categoria: String,
    pub sueno: f64,
    pub pulso: f64,
    pub carga: f64,
}

/// La disposición pura, a partir de los datos.
#[must_use]
pub fn disposicion_de(
    horas: f64,
    meta: f64,
    pulso: Option<i64>,
    base: Option<i64>,
    horas_carga: f64,
    rpe: f64,
) -> Disposicion {
    let meta = if meta > 0.0 { meta } else { 8.0 };
    // Sin fases del sueño, se normaliza a los 40 puntos.
    let sueno = ((horas / meta).clamp(0.0, 1.25) * 35.0).clamp(0.0, 35.0) / 35.0 * 40.0;
    let sueno = sueno.clamp(0.0, 40.0);
    let pulso_p = match pulso.filter(|p| *p > 0) {
        None => 32.0,
        Some(p) => {
            let d = p - base.filter(|b| *b > 0).unwrap_or(p);
            if d <= -1 {
                35.0
            } else if d <= 2 {
                32.0
            } else if d <= 5 {
                24.0
            } else {
                14.0
            }
        }
    };
    let carga_v = horas_carga * rpe;
    let carga = if carga_v > 22.0 {
        15.0
    } else if carga_v > 14.0 {
        20.0
    } else {
        25.0
    };
    let total = ((sueno + pulso_p + carga).round() as i64).clamp(0, 100);
    Disposicion {
        total,
        categoria: if total >= 85 {
            "optimal"
        } else if total >= 70 {
            "favorable"
        } else {
            "recoveryNeeded"
        }
        .into(),
        sueno: crate::formulas::fijo(sueno, 1),
        pulso: pulso_p,
        carga,
    }
}

/// La disposición de hoy, si se anotó el sueño.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn disposicion(conn: &Connection) -> ScResult<Option<Disposicion>> {
    let hoy = fechas::hoy();
    let dia = fechas::clave(hoy);
    let Some(n) = noche(conn, &dia)? else {
        return Ok(None);
    };
    if n.minutos <= 0 {
        return Ok(None);
    }
    let meta = perfil::leer(conn)?.map_or(7.5, |f| f.perfil.sueno_meta_h);
    let desde = fechas::clave(fechas::mas_dias(hoy, -14));
    let base: Vec<i64> = {
        let mut st = conn
            .prepare("SELECT resting_heart_rate_bpm FROM daily_summaries WHERE date_key >= ?1 AND date_key < ?2 AND resting_heart_rate_bpm > 0")
            .map_err(fallo)?;
        let v = st
            .query_map(params![desde, dia], |f| f.get(0))
            .map_err(fallo)?
            .collect::<Result<_, _>>()
            .map_err(fallo)?;
        v
    };
    let base = (base.len() >= 3)
        .then(|| (base.iter().sum::<i64>() as f64 / base.len() as f64).round() as i64);
    let desde3 = fechas::clave(fechas::mas_dias(hoy, -2));
    let (minutos, rpe): (i64, f64) = conn
        .query_row(
            "SELECT coalesce(sum(duration_minutes), 0),
                    coalesce(sum(duration_minutes * CASE rpe_intensity WHEN 'light' THEN 3 WHEN 'moderate' THEN 6 ELSE 9 END), 0)
             FROM workout_blocks WHERE date_key >= ?1 AND date_key <= ?2",
            params![desde3, dia],
            |f| Ok((f.get(0)?, f.get::<_, i64>(1)? as f64)),
        )
        .map_err(fallo)?;
    Ok(Some(disposicion_de(
        n.minutos as f64 / 60.0,
        meta,
        n.pulso,
        base,
        minutos as f64 / 60.0,
        if minutos > 0 {
            rpe / minutos as f64
        } else {
            0.0
        },
    )))
}

// --- Cafeína -----------------------------------------------------------------

/// Vida media de la cafeína en horas.
pub const VIDA_MEDIA: f64 = 5.5;
/// Lo que se tolera a la hora de dormir sin afectar el sueño.
pub const MAX_AL_DORMIR: f64 = 25.0;

/// Lo que queda de una dosis tras unas horas.
#[must_use]
pub fn residuo(mg: f64, horas: f64) -> f64 {
    if horas < 0.0 {
        0.0
    } else {
        mg * 0.5f64.powf(horas / VIDA_MEDIA)
    }
}

/// La cafeína del día.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Cafeina {
    pub hoy_mg: f64,
    pub ahora_mg: f64,
    pub al_dormir_mg: f64,
    /// Minutos desde medianoche de la última taza sugerida.
    pub limite: i64,
    pub pasado_el_limite: bool,
}

/// El estado de la cafeína de hoy.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn cafeina(conn: &Connection) -> ScResult<Cafeina> {
    let ahora = fechas::ahora();
    let dia = fechas::clave(ahora.date());
    let ayer = fechas::clave(fechas::mas_dias(ahora.date(), -1));
    let mut st = conn
        .prepare("SELECT logged_at, caffeine_mg, date_key FROM caffeine_logs WHERE date_key >= ?1")
        .map_err(fallo)?;
    let tomas: Vec<(NaiveDateTime, f64, String)> = st
        .query_map([&ayer], |f| {
            Ok((f.get::<_, String>(0)?, f.get(1)?, f.get(2)?))
        })
        .map_err(fallo)?
        .filter_map(Result::ok)
        .filter_map(|(t, mg, d)| Some((fechas::leer(&t)?, mg, d)))
        .collect();
    let (dormir_min, corte_h) = perfil::leer(conn)?.map_or((23 * 60, 8.0), |f| {
        (f.perfil.hora_dormir, f.perfil.corte_cafeina_h)
    });
    let mut dormir = fechas::inicio(ahora.date()) + chrono::Duration::minutes(dormir_min);
    if dormir_min < 12 * 60 {
        dormir += chrono::Duration::days(1);
    }
    let a = |t: NaiveDateTime| -> f64 {
        tomas
            .iter()
            .filter(|x| x.0 <= t)
            .map(|x| residuo(x.1, (t - x.0).num_minutes() as f64 / 60.0))
            .sum()
    };
    let limite_dt = dormir - chrono::Duration::minutes((corte_h * 60.0).round() as i64);
    let limite = i64::from(limite_dt.hour() * 60 + limite_dt.minute());
    Ok(Cafeina {
        hoy_mg: tomas.iter().filter(|x| x.2 == dia).map(|x| x.1).sum(),
        ahora_mg: crate::formulas::fijo(a(ahora), 1),
        al_dormir_mg: crate::formulas::fijo(a(dormir), 1),
        limite,
        pasado_el_limite: ahora > limite_dt,
    })
}

/// Anota una bebida con cafeína.
///
/// # Errors
///
/// [`ScError::InvalidInput`] fuera de 1–600 mg.
pub fn tomar_cafeina(db: &BaseDeDatos, fuente: &str, mg: f64) -> ScResult<()> {
    if !(1.0..=600.0).contains(&mg) || fuente.is_empty() || fuente.len() > 30 {
        return Err(ScError::InvalidInput);
    }
    let conn = db.conexion()?;
    let ahora = fechas::ahora();
    conn.execute(
        "INSERT INTO caffeine_logs (date_key, logged_at, source_name, caffeine_mg) VALUES (?1, ?2, ?3, ?4)",
        params![fechas::clave(ahora.date()), fechas::texto(ahora), fuente, mg],
    )
    .map_err(fallo)?;
    Ok(())
}

// --- Ciclo -------------------------------------------------------------------

/// El ciclo en curso.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Ciclo {
    pub inicio: String,
    pub largo: i64,
    pub dia: i64,
    pub fase: String,
    pub proximo: String,
}

/// El ciclo en curso, si se lleva.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn ciclo(conn: &Connection) -> ScResult<Option<Ciclo>> {
    let fila: Option<(String, i64)> = conn
        .query_row(
            "SELECT cycle_start_date, cycle_length_days FROM female_cycle_logs ORDER BY cycle_start_date DESC LIMIT 1",
            [],
            |f| Ok((f.get(0)?, f.get(1)?)),
        )
        .optional()
        .map_err(fallo)?;
    Ok(fila.and_then(|(i, l)| {
        let inicio = fechas::leer(&i)?.date();
        let (fase, dia, largo) = crate::cuerpo::fase_del_ciclo(inicio, l, fechas::hoy());
        let pasados = (fechas::hoy() - inicio).num_days().max(0);
        let siguiente = fechas::mas_dias(inicio, (pasados / largo + 1) * largo);
        Some(Ciclo {
            inicio: fechas::clave(inicio),
            largo,
            dia,
            fase: fase.into(),
            proximo: fechas::clave(siguiente),
        })
    }))
}

/// Anota el inicio de un periodo.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si el día no se entiende o el largo no es 21–45.
pub fn empezar_ciclo(db: &BaseDeDatos, dia: &str, largo: i64) -> ScResult<()> {
    let d = fechas::de_clave(dia).ok_or(ScError::InvalidInput)?;
    if !(21..=45).contains(&largo) || d > fechas::hoy() {
        return Err(ScError::InvalidInput);
    }
    let conn = db.conexion()?;
    conn.execute(
        "INSERT INTO female_cycle_logs (cycle_start_date, cycle_length_days) VALUES (?1, ?2)",
        params![fechas::texto(fechas::inicio(d)), largo],
    )
    .map_err(fallo)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perfil::pruebas::base_con_perfil;

    #[test]
    fn una_buena_noche_sin_pulsometro_es_optima() {
        let d = disposicion_de(7.5, 7.5, None, None, 0.0, 0.0);
        assert_eq!((d.total, d.categoria.as_str()), (97, "optimal"));
    }

    #[test]
    fn el_pulso_alto_y_la_carga_bajan_la_disposicion() {
        let d = disposicion_de(6.0, 8.0, Some(68), Some(60), 3.0, 9.0);
        assert_eq!(d.categoria, "recoveryNeeded");
    }

    #[test]
    fn la_noche_anotada_da_la_disposicion_de_hoy() {
        let db = base_con_perfil();
        anotar_noche(
            &db,
            Noche {
                minutos: 450,
                despertar: Some(420),
                pulso: None,
                pasos: Some(6000),
            },
        )
        .unwrap();
        let d = disposicion(&db.conexion().unwrap()).unwrap().unwrap();
        assert!(d.total >= 85);
        assert_eq!(
            anotar_noche(
                &db,
                Noche {
                    minutos: 2000,
                    ..Default::default()
                }
            ),
            Err(ScError::InvalidInput)
        );
    }

    #[test]
    fn la_cafeina_baja_a_la_mitad_cada_5_horas_y_media() {
        assert!((residuo(100.0, 5.5) - 50.0).abs() < 1e-9);
        assert_eq!(residuo(100.0, -1.0), 0.0);
    }

    #[test]
    fn la_primera_entrada_del_dia_premia_y_una_vacia_no_se_guarda() {
        let db = base_con_perfil();
        let e = NuevaEntrada {
            texto: "Buen día".into(),
            animo: Some(4),
            energia: None,
            estres: None,
            etiquetas: vec![],
        };
        assert!(escribir(&db, &e).unwrap().is_some());
        assert!(escribir(&db, &e).unwrap().is_none());
        let vacia = NuevaEntrada {
            texto: " ".into(),
            animo: None,
            energia: None,
            estres: None,
            etiquetas: vec![],
        };
        assert_eq!(escribir(&db, &vacia), Err(ScError::InvalidInput));
        assert_eq!(entradas(&db.conexion().unwrap(), 10).unwrap().len(), 2);
    }
}
