//! Cuerpo: pesajes, cinta métrica, peso tendencia, falsa subida, hitos,
//! entrenamientos y la recalibración del gasto adaptativo.
//!
//! Un pesaje sigue la misma cascada que en el teléfono: métricas, tendencia,
//! detector de retención, monedas, recalibración si toca e hitos. En el
//! escritorio no hay báscula ni Health Connect, así que las métricas son las
//! estimadas y el origen es siempre `manual` (ADR-008).

use chrono::NaiveDateTime;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{self, fallo, BaseDeDatos};
use crate::error::{ScError, ScResult};
use crate::fechas;
use crate::formulas::{self, Sexo};
use crate::idioma::{self, t, Idioma};
use crate::nibi::{self, Accion, Recompensa};
use crate::perfil;

/// Subida diaria a partir de la cual se busca una causa de retención.
const UMBRAL_SUBIDA_KG: f64 = 0.40;

/// Las causas de una falsa subida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Causa {
    Sodio(i64),
    Glucogeno(i64),
    FaseLutea,
    Creatina,
}

fn decimal(v: f64) -> String {
    let s = format!("{v:.1}");
    if idioma::actual() == Idioma::En {
        s
    } else {
        s.replace('.', ",")
    }
}

/// El texto tranquilizador de una falsa subida, en el idioma de la interfaz, como
/// lo guarda el teléfono (sin culpa: es agua, no grasa).
fn explicar(causas: &[Causa], subida: f64) -> String {
    let razones: Vec<String> = causas
        .iter()
        .map(|c| match *c {
            Causa::Sodio(mg) => match idioma::actual() {
                Idioma::Es => format!("sodio elevado ayer ({mg} mg)"),
                Idioma::En => format!("high sodium yesterday ({mg} mg)"),
                Idioma::Pt => format!("sódio alto ontem ({mg} mg)"),
            },
            Causa::Glucogeno(g) => match idioma::actual() {
                Idioma::Es => format!("recarga de glucógeno ({g} g de carbohidratos ayer)"),
                Idioma::En => format!("glycogen refill ({g} g of carbohydrates yesterday)"),
                Idioma::Pt => format!("recarga de glicogênio ({g} g de carboidratos ontem)"),
            },
            Causa::FaseLutea => t(
                "fase lútea (retención fisiológica por progesterona)",
                "luteal phase (normal retention from progesterone)",
                "fase lútea (retenção normal pela progesterona)",
            )
            .into(),
            Causa::Creatina => t(
                "creatina (agua dentro del músculo, no grasa)",
                "creatine (water inside the muscle, not fat)",
                "creatina (água dentro do músculo, não gordura)",
            )
            .into(),
        })
        .collect();
    let delta = decimal(subida);
    let lista = razones.join(", ");
    match idioma::actual() {
        Idioma::Es => format!(
            "+{delta} kg de agua temporal por {lista}. No es grasa: tu Peso Tendencia apenas se mueve."
        ),
        Idioma::En => format!(
            "+{delta} kg of temporary water from {lista}. It isn't fat: your trend weight barely moves."
        ),
        Idioma::Pt => format!(
            "+{delta} kg de água temporária por {lista}. Não é gordura: seu peso de tendência quase não se move."
        ),
    }
}

/// La fase lútea del ciclo, con la regla del teléfono (lútea son los 14 días
/// anteriores al siguiente periodo).
fn en_fase_lutea(conn: &Connection, hoy: chrono::NaiveDate) -> ScResult<bool> {
    let ultimo: Option<(String, i64)> = conn
        .query_row(
            "SELECT cycle_start_date, cycle_length_days FROM female_cycle_logs
             ORDER BY cycle_start_date DESC LIMIT 1",
            [],
            |f| Ok((f.get(0)?, f.get(1)?)),
        )
        .optional()
        .map_err(fallo)?;
    let Some((inicio, largo)) = ultimo else {
        return Ok(false);
    };
    let Some(inicio) = fechas::leer(&inicio) else {
        return Ok(false);
    };
    Ok(fase_del_ciclo(inicio.date(), largo, hoy).0 == "luteal")
}

/// La fase y el día del ciclo.
#[must_use]
pub fn fase_del_ciclo(
    inicio: chrono::NaiveDate,
    largo: i64,
    dia: chrono::NaiveDate,
) -> (&'static str, i64, i64) {
    let largo = if (21..=45).contains(&largo) {
        largo
    } else {
        28
    };
    let pasados = (dia - inicio).num_days();
    if pasados < 0 {
        return ("unknown", 0, largo);
    }
    let dia_ciclo = pasados % largo + 1;
    let ovulacion = largo - 14;
    let fase = if dia_ciclo <= 5 {
        "menstrual"
    } else if dia_ciclo < ovulacion - 1 {
        "follicular"
    } else if dia_ciclo <= ovulacion + 1 {
        "ovulatory"
    } else {
        "luteal"
    };
    (fase, dia_ciclo, largo)
}

fn causas(
    conn: &Connection,
    peso: f64,
    anterior: f64,
    fila: &perfil::Fila,
) -> ScResult<Vec<Causa>> {
    if peso - anterior < UMBRAL_SUBIDA_KG {
        return Ok(Vec::new());
    }
    let hoy = fechas::hoy();
    let ayer = fechas::clave(fechas::mas_dias(hoy, -1));
    let hace8 = fechas::clave(fechas::mas_dias(hoy, -8));
    let (sodio, carbos): (Option<f64>, Option<f64>) = conn
        .query_row(
            "SELECT sum(total_sodium_mg), sum(total_carbs_g) FROM food_logs WHERE date_key = ?1",
            [&ayer],
            |f| Ok((f.get(0)?, f.get(1)?)),
        )
        .map_err(fallo)?;
    let media: Option<f64> = conn
        .query_row(
            "SELECT avg(c) FROM (SELECT sum(total_carbs_g) AS c FROM food_logs
             WHERE date_key >= ?1 AND date_key < ?2 GROUP BY date_key HAVING c > 0)",
            params![hace8, ayer],
            |f| f.get(0),
        )
        .map_err(fallo)?;
    let mut v = Vec::new();
    if let Some(s) = sodio.map(|s| s.round() as i64).filter(|s| *s > 2500) {
        v.push(Causa::Sodio(s));
    }
    if let (Some(c), Some(m)) = (carbos, media) {
        if m > 0.0 && c > 1.30 * m {
            v.push(Causa::Glucogeno(c.round() as i64));
        }
    }
    if fila.perfil.sexo == Sexo::Mujer && fila.perfil.ciclo_activo && en_fase_lutea(conn, hoy)? {
        v.push(Causa::FaseLutea);
    }
    let desde = fechas::clave(fechas::mas_dias(hoy, -6));
    let creatina: i64 = conn
        .query_row(
            "SELECT count(*) FROM supplement_logs l JOIN supplements_tracker s ON s.id = l.supplement_id
             WHERE s.affects_water_retention = 1 AND s.is_enabled = 1 AND l.was_taken = 1
               AND l.date_key >= ?1 AND l.date_key <= ?2",
            params![desde, fechas::clave(hoy)],
            |f| f.get(0),
        )
        .map_err(fallo)?;
    if creatina >= 3 {
        v.push(Causa::Creatina);
    }
    Ok(v)
}

/// Lo que devolvió un pesaje.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct ResultadoPesaje {
    pub tendencia_kg: f64,
    /// La explicación de una falsa subida, si la hubo.
    pub falsa_subida: Option<String>,
    pub recompensa: Option<Recompensa>,
    pub hitos_alcanzados: Vec<f64>,
    /// El gasto nuevo, si este pesaje lo recalibró.
    pub gasto_recalibrado: Option<i64>,
}

/// Inserta un pesaje con toda su cascada dentro de una transacción abierta.
///
/// # Errors
///
/// [`ScError::NotFound`] sin perfil; [`ScError::StorageFailure`] si falla.
pub fn insertar_pesaje(
    conn: &Connection,
    peso: f64,
    cuando: NaiveDateTime,
) -> ScResult<ResultadoPesaje> {
    let fila = perfil::exigir(conn)?;
    let anterior = conn
        .query_row(
            "SELECT weight_kg, trend_weight_kg FROM body_measurements
             WHERE source != 'manual_navy_tape' ORDER BY measured_at DESC, id DESC LIMIT 1",
            [],
            |f| Ok((f.get::<_, f64>(0)?, f.get::<_, f64>(1)?)),
        )
        .optional()
        .map_err(fallo)?;
    let edad = fechas::edad(fila.nacimiento, cuando);
    let m = formulas::metricas_estimadas(peso, fila.perfil.altura_cm, edad, fila.perfil.sexo);
    let tendencia = formulas::tendencia(peso, anterior.map(|a| a.1));
    let causas = match anterior {
        Some((p, _)) => causas(conn, peso, p, &fila)?,
        None => Vec::new(),
    };
    let explicacion =
        (!causas.is_empty()).then(|| explicar(&causas, peso - anterior.map_or(peso, |a| a.0)));
    conn.execute(
        "INSERT INTO body_measurements (
            measured_at, date_key, source, weight_kg, trend_weight_kg, bmi,
            body_fat_percentage, muscle_mass_kg, water_percentage, visceral_fat_index,
            bone_mass_kg, protein_percentage, basal_metabolic_rate_kcal, metabolic_age,
            lean_body_mass_kg, body_score, somatotype, is_water_retention_spike, spike_explanation
         ) VALUES (?1, ?2, 'manual', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        params![
            fechas::texto(cuando),
            fechas::clave(cuando.date()),
            peso,
            tendencia,
            m.imc,
            m.grasa_pct,
            m.musculo_kg,
            m.agua_pct,
            m.grasa_visceral,
            m.hueso_kg,
            m.proteina_pct,
            m.metabolismo_basal,
            m.edad_metabolica,
            m.masa_magra_kg,
            m.puntuacion,
            m.somatotipo,
            explicacion.is_some(),
            explicacion,
        ],
    )
    .map_err(fallo)?;
    let recompensa = nibi::premiar(conn, Accion::Pesaje)?;
    let gasto = calibrar_si_toca(conn, false)?
        .filter(|c| c.calibro)
        .map(|c| c.nuevo);
    let hitos = comprobar_hitos(conn, tendencia)?;
    Ok(ResultadoPesaje {
        tendencia_kg: tendencia,
        falsa_subida: explicacion,
        recompensa,
        hitos_alcanzados: hitos,
        gasto_recalibrado: gasto,
    })
}

/// Registra un pesaje de ahora.
///
/// # Errors
///
/// [`ScError::InvalidInput`] fuera de 25–350 kg.
pub fn pesar(db: &BaseDeDatos, peso: f64) -> ScResult<ResultadoPesaje> {
    if !(25.0..=350.0).contains(&peso) {
        return Err(ScError::InvalidInput);
    }
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let r = insertar_pesaje(&tx, formulas::fijo(peso, 2), fechas::ahora())?;
    tx.commit().map_err(fallo)?;
    Ok(r)
}

/// La desviación respecto a la tendencia si el peso parece de otra persona
/// (más de un 8 %): la interfaz pregunta antes de guardarlo.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn parece_de_otra_persona(db: &BaseDeDatos, peso: f64) -> ScResult<Option<f64>> {
    let conn = db.conexion()?;
    Ok(perfil::ultimo_peso(&conn)?.and_then(|(_, t)| {
        let r = (peso - t).abs() / t;
        (t > 0.0 && r > 0.08).then_some(r)
    }))
}

/// Las medidas de la cinta métrica.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Cinta {
    pub cintura_cm: f64,
    pub cuello_cm: f64,
    pub cadera_cm: Option<f64>,
}

/// Guarda la cinta en una fila propia que arrastra el último peso, y devuelve
/// la grasa validada (60 % estimación, 40 % cinta).
///
/// # Errors
///
/// [`ScError::NotFound`] si aún no hay pesajes; [`ScError::InvalidInput`] fuera
/// de rango.
pub fn medir_cinta(db: &BaseDeDatos, c: Cinta) -> ScResult<f64> {
    let ok = (40.0..=250.0).contains(&c.cintura_cm)
        && (20.0..=80.0).contains(&c.cuello_cm)
        && c.cadera_cm.is_none_or(|h| (50.0..=250.0).contains(&h));
    if !ok {
        return Err(ScError::InvalidInput);
    }
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let fila = perfil::exigir(&tx)?;
    let (peso, tendencia, imc, grasa): (f64, f64, f64, Option<f64>) = tx
        .query_row(
            "SELECT weight_kg, trend_weight_kg, bmi, body_fat_percentage FROM body_measurements
             WHERE source != 'manual_navy_tape' ORDER BY measured_at DESC, id DESC LIMIT 1",
            [],
            |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?, f.get(3)?)),
        )
        .optional()
        .map_err(fallo)?
        .ok_or(ScError::NotFound)?;
    let marina = formulas::grasa_marina(
        fila.perfil.altura_cm,
        c.cintura_cm,
        c.cuello_cm,
        fila.perfil.sexo,
        c.cadera_cm,
    );
    let validada = formulas::grasa_validada(grasa.unwrap_or(marina), marina);
    let ahora = fechas::ahora();
    tx.execute(
        "INSERT INTO body_measurements (measured_at, date_key, source, weight_kg, trend_weight_kg,
            bmi, waist_cm, neck_cm, hip_cm, validated_body_fat_percentage)
         VALUES (?1, ?2, 'manual_navy_tape', ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            fechas::texto(ahora),
            fechas::clave(ahora.date()),
            peso,
            tendencia,
            imc,
            c.cintura_cm,
            c.cuello_cm,
            c.cadera_cm,
            validada
        ],
    )
    .map_err(fallo)?;
    tx.commit().map_err(fallo)?;
    Ok(validada)
}

/// Un pesaje para el historial y la gráfica.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Pesaje {
    pub id: i64,
    /// `YYYY-MM-DD`.
    pub dia: String,
    /// `HH:MM`.
    pub hora: String,
    pub peso_kg: f64,
    pub tendencia_kg: f64,
    pub imc: f64,
    pub grasa_pct: Option<f64>,
    pub falsa_subida: Option<String>,
    pub cinta: bool,
    pub cintura_cm: Option<f64>,
    pub grasa_validada_pct: Option<f64>,
}

/// El resumen de Cuerpo.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct ResumenCuerpo {
    pub ultimo: Option<Pesaje>,
    pub metricas: Option<formulas::Metricas>,
    /// Pesajes de los últimos 120 días, del más antiguo al más nuevo.
    pub serie: Vec<Pesaje>,
    pub historial: Vec<Pesaje>,
    pub peso_meta_kg: f64,
    pub peso_inicial_kg: Option<f64>,
    pub hitos: Vec<Hito>,
    /// Cambio de la tendencia en 7 y 30 días.
    pub cambio_7: Option<f64>,
    pub cambio_30: Option<f64>,
    pub entrenos: Vec<Entreno>,
    pub calibraciones: Vec<Calibracion>,
}

/// Un hito de 1,5 kg.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Hito {
    pub orden: i64,
    pub peso_kg: f64,
    pub logrado: bool,
    pub dia: Option<String>,
}

fn pesaje_de(f: &rusqlite::Row<'_>) -> rusqlite::Result<Pesaje> {
    let cuando = fechas::leer(&f.get::<_, String>("measured_at")?).unwrap_or_default();
    Ok(Pesaje {
        id: f.get("id")?,
        dia: f.get("date_key")?,
        hora: cuando.format("%H:%M").to_string(),
        peso_kg: f.get("weight_kg")?,
        tendencia_kg: f.get("trend_weight_kg")?,
        imc: f.get("bmi")?,
        grasa_pct: f.get("body_fat_percentage")?,
        falsa_subida: f.get("spike_explanation")?,
        cinta: f.get::<_, String>("source")? == "manual_navy_tape",
        cintura_cm: f.get("waist_cm")?,
        grasa_validada_pct: f.get("validated_body_fat_percentage")?,
    })
}

/// El resumen de Cuerpo.
///
/// # Errors
///
/// [`ScError::NotFound`] sin perfil.
pub fn resumen(db: &BaseDeDatos) -> ScResult<ResumenCuerpo> {
    let mut conn = db.conexion()?;
    let tx = db::lectura_coherente(&mut conn)?;
    let fila = perfil::exigir(&tx)?;
    let desde = fechas::clave(fechas::mas_dias(fechas::hoy(), -120));
    let mut st = tx
        .prepare(
            "SELECT * FROM body_measurements WHERE date_key >= ?1 AND source != 'manual_navy_tape'
             ORDER BY measured_at ASC, id ASC",
        )
        .map_err(fallo)?;
    let serie: Vec<Pesaje> = st
        .query_map([&desde], pesaje_de)
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    let mut st = tx
        .prepare("SELECT * FROM body_measurements ORDER BY measured_at DESC, id DESC LIMIT 60")
        .map_err(fallo)?;
    let historial: Vec<Pesaje> = st
        .query_map([], pesaje_de)
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    let ultimo = historial.iter().find(|p| !p.cinta).cloned();
    let metricas = ultimo.as_ref().map(|u| {
        let edad = fechas::edad(fila.nacimiento, fechas::ahora());
        formulas::metricas_estimadas(u.peso_kg, fila.perfil.altura_cm, edad, fila.perfil.sexo)
    });
    let inicial: Option<f64> = tx
        .query_row(
            "SELECT weight_kg FROM body_measurements WHERE source != 'manual_navy_tape'
             ORDER BY measured_at ASC, id ASC LIMIT 1",
            [],
            |f| f.get(0),
        )
        .optional()
        .map_err(fallo)?;
    let cambio = |dias: i64| -> Option<f64> {
        let u = ultimo.as_ref()?;
        let limite = fechas::clave(fechas::mas_dias(fechas::hoy(), -dias));
        let antes = serie.iter().rev().find(|p| p.dia <= limite)?;
        Some(formulas::fijo(u.tendencia_kg - antes.tendencia_kg, 2))
    };
    let (cambio_7, cambio_30) = (cambio(7), cambio(30));
    Ok(ResumenCuerpo {
        metricas,
        peso_meta_kg: fila.perfil.peso_meta_kg,
        peso_inicial_kg: inicial,
        hitos: hitos(&tx)?,
        cambio_7,
        cambio_30,
        entrenos: entrenos(&tx, 30)?,
        calibraciones: calibraciones(&tx)?,
        ultimo,
        serie,
        historial,
    })
}

/// Borra un pesaje o una medida de cinta.
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn borrar(db: &BaseDeDatos, id: i64) -> ScResult<()> {
    let conn = db.conexion()?;
    let n = conn
        .execute("DELETE FROM body_measurements WHERE id = ?1", [id])
        .map_err(fallo)?;
    if n == 0 {
        return Err(ScError::NotFound);
    }
    Ok(())
}

fn hitos(conn: &Connection) -> ScResult<Vec<Hito>> {
    let mut st = conn
        .prepare("SELECT milestone_order, target_weight_kg, is_completed, completed_at FROM milestone_goals ORDER BY milestone_order")
        .map_err(fallo)?;
    let v = st
        .query_map([], |f| {
            Ok(Hito {
                orden: f.get(0)?,
                peso_kg: f.get(1)?,
                logrado: f.get(2)?,
                dia: f
                    .get::<_, Option<String>>(3)?
                    .and_then(|t| fechas::leer(&t))
                    .map(|d| fechas::clave(d.date())),
            })
        })
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    Ok(v)
}

/// Rehace los hitos de 1,5 kg desde el peso actual hasta la meta.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la escritura falla.
pub fn regenerar_hitos(conn: &Connection, actual: f64, meta: f64) -> ScResult<()> {
    conn.execute("DELETE FROM milestone_goals", [])
        .map_err(fallo)?;
    let diferencia = (actual - meta).abs();
    let pesos: Vec<f64> = if diferencia < 0.5 {
        vec![meta]
    } else {
        let pasos = (diferencia / 1.5).ceil() as i64;
        (1..=pasos)
            .map(|i| {
                if i == pasos {
                    meta
                } else if meta < actual {
                    actual - 1.5 * i as f64
                } else {
                    actual + 1.5 * i as f64
                }
            })
            .collect()
    };
    for (i, p) in pesos.iter().enumerate() {
        conn.execute(
            "INSERT INTO milestone_goals (milestone_order, target_weight_kg, is_completed) VALUES (?1, ?2, 0)",
            params![i as i64 + 1, formulas::fijo(*p, 1)],
        )
        .map_err(fallo)?;
    }
    Ok(())
}

/// Marca los hitos que la tendencia ya alcanzó (la tendencia y no el peso del
/// día: el agua no celebra ni quita hitos).
fn comprobar_hitos(conn: &Connection, tendencia: f64) -> ScResult<Vec<f64>> {
    let pendientes: Vec<(i64, f64)> = {
        let mut st = conn
            .prepare("SELECT id, target_weight_kg FROM milestone_goals WHERE is_completed = 0 ORDER BY milestone_order")
            .map_err(fallo)?;
        let v = st
            .query_map([], |f| Ok((f.get(0)?, f.get(1)?)))
            .map_err(fallo)?
            .collect::<Result<_, _>>()
            .map_err(fallo)?;
        v
    };
    if pendientes.is_empty() {
        return Ok(Vec::new());
    }
    let alcanzados: Vec<(i64, f64)> = pendientes
        .iter()
        .copied()
        .filter(|(_, p)| {
            if pendientes.len() < 2 {
                (tendencia - p).abs() <= 0.25
            } else if pendientes[0].1 > pendientes[1].1 {
                tendencia <= *p
            } else {
                tendencia >= *p
            }
        })
        .collect();
    if alcanzados.is_empty() {
        return Ok(Vec::new());
    }
    let ahora = fechas::texto(fechas::ahora());
    for (id, _) in &alcanzados {
        conn.execute(
            "UPDATE milestone_goals SET is_completed = 1, completed_at = ?1 WHERE id = ?2",
            params![ahora, id],
        )
        .map_err(fallo)?;
    }
    nibi::premiar(conn, Accion::HitoDePeso)?;
    Ok(alcanzados.into_iter().map(|a| a.1).collect())
}

// --- Entrenamientos ------------------------------------------------------

/// La categoría de un bloque de entrenamiento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub enum Categoria {
    Pesas,
    Calistenia,
    Cardio,
    Deporte,
}

impl Categoria {
    fn a_db(self) -> &'static str {
        match self {
            Self::Pesas => "weights",
            Self::Calistenia => "calisthenics",
            Self::Cardio => "cardio",
            Self::Deporte => "sports",
        }
    }

    fn desde_db(v: &str) -> Self {
        match v {
            "calisthenics" => Self::Calistenia,
            "cardio" => Self::Cardio,
            "sports" => Self::Deporte,
            _ => Self::Pesas,
        }
    }
}

/// El esfuerzo percibido de un bloque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub enum Intensidad {
    Suave,
    Moderada,
    Intensa,
}

impl Intensidad {
    fn a_db(self) -> &'static str {
        match self {
            Self::Suave => "light",
            Self::Moderada => "moderate",
            Self::Intensa => "intense",
        }
    }

    fn desde_db(v: &str) -> Self {
        match v {
            "moderate" => Self::Moderada,
            "intense" => Self::Intensa,
            _ => Self::Suave,
        }
    }
}

/// El MET de cada categoría e intensidad (tabla ACSM del teléfono).
#[must_use]
pub fn met(c: Categoria, i: Intensidad) -> f64 {
    use Categoria as C;
    use Intensidad as I;
    match (c, i) {
        (C::Pesas, I::Suave) => 3.5,
        (C::Pesas, I::Moderada) => 5.0,
        (C::Pesas, I::Intensa) => 6.5,
        (C::Calistenia, I::Suave) => 3.8,
        (C::Calistenia, I::Moderada) => 5.5,
        (C::Calistenia, I::Intensa) => 8.0,
        (C::Cardio | C::Deporte, I::Suave) => 5.0,
        (C::Cardio, I::Moderada) => 7.5,
        (C::Cardio, I::Intensa) => 10.5,
        (C::Deporte, I::Moderada) => 7.0,
        (C::Deporte, I::Intensa) => 9.5,
    }
}

/// kcal netas de un bloque: `(MET − 1) × 3,5 × peso / 200 × minutos`.
#[must_use]
pub fn kcal_netas(met: f64, peso: f64, minutos: i64) -> i64 {
    if peso <= 0.0 || minutos <= 0 {
        return 0;
    }
    ((met - 1.0).clamp(0.0, 20.0) * 3.5 * peso / 200.0 * minutos as f64).round() as i64
}

/// Un bloque registrado.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Entreno {
    pub id: i64,
    pub dia: String,
    pub categoria: Categoria,
    pub intensidad: Intensidad,
    pub minutos: i64,
    pub kcal: i64,
}

fn entrenos(conn: &Connection, dias: i64) -> ScResult<Vec<Entreno>> {
    let desde = fechas::clave(fechas::mas_dias(fechas::hoy(), -dias));
    let mut st = conn
        .prepare(
            "SELECT id, date_key, category, rpe_intensity, duration_minutes, net_calories_burned
             FROM workout_blocks WHERE date_key >= ?1 ORDER BY performed_at DESC LIMIT 50",
        )
        .map_err(fallo)?;
    let v = st
        .query_map([desde], |f| {
            Ok(Entreno {
                id: f.get(0)?,
                dia: f.get(1)?,
                categoria: Categoria::desde_db(&f.get::<_, String>(2)?),
                intensidad: Intensidad::desde_db(&f.get::<_, String>(3)?),
                minutos: f.get(4)?,
                kcal: f.get(5)?,
            })
        })
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    Ok(v)
}

/// Registra un bloque con el último peso (o el de la meta, sin pesajes).
///
/// # Errors
///
/// [`ScError::InvalidInput`] fuera de 1–600 minutos.
pub fn entrenar(
    db: &BaseDeDatos,
    c: Categoria,
    i: Intensidad,
    minutos: i64,
) -> ScResult<(i64, Option<Recompensa>)> {
    if !(1..=600).contains(&minutos) {
        return Err(ScError::InvalidInput);
    }
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let fila = perfil::exigir(&tx)?;
    let peso = perfil::ultimo_peso(&tx)?.map_or(fila.perfil.peso_meta_kg, |p| p.0);
    let m = met(c, i);
    let kcal = kcal_netas(m, peso, minutos);
    let ahora = fechas::ahora();
    tx.execute(
        "INSERT INTO workout_blocks (date_key, performed_at, category, duration_minutes, rpe_intensity,
            applied_met, user_weight_at_time_kg, net_calories_burned)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            fechas::clave(ahora.date()),
            fechas::texto(ahora),
            c.a_db(),
            minutos,
            i.a_db(),
            m,
            peso,
            kcal
        ],
    )
    .map_err(fallo)?;
    let r = nibi::premiar(&tx, Accion::Entrenamiento)?;
    tx.commit().map_err(fallo)?;
    Ok((kcal, r))
}

/// Borra un bloque.
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn borrar_entreno(db: &BaseDeDatos, id: i64) -> ScResult<()> {
    let conn = db.conexion()?;
    if conn
        .execute("DELETE FROM workout_blocks WHERE id = ?1", [id])
        .map_err(fallo)?
        == 0
    {
        return Err(ScError::NotFound);
    }
    Ok(())
}

/// Las kcal netas de ejercicio de un día.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn ejercicio_del_dia(conn: &Connection, dia: &str) -> ScResult<i64> {
    conn.query_row(
        "SELECT coalesce(sum(net_calories_burned), 0) FROM workout_blocks WHERE date_key = ?1",
        [dia],
        |f| f.get(0),
    )
    .map_err(fallo)
}

// --- Gasto adaptativo ----------------------------------------------------

/// Una recalibración del gasto.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Calibracion {
    pub dia: String,
    pub anterior: i64,
    pub nuevo: i64,
    pub acotada: bool,
    pub media_kcal: f64,
    pub cambio_tendencia: f64,
}

fn calibraciones(conn: &Connection) -> ScResult<Vec<Calibracion>> {
    let mut st = conn
        .prepare(
            "SELECT date_key, previous_tdee_kcal, new_tdee_kcal, was_clamped, average_kcal14d, delta_ema14d
             FROM tdee_calibrations ORDER BY calibrated_at DESC LIMIT 12",
        )
        .map_err(fallo)?;
    let v = st
        .query_map([], |f| {
            Ok(Calibracion {
                dia: f.get(0)?,
                anterior: f.get(1)?,
                nuevo: f.get(2)?,
                acotada: f.get(3)?,
                media_kcal: f.get(4)?,
                cambio_tendencia: f.get(5)?,
            })
        })
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    Ok(v)
}

/// Lo que dio un intento de recalibrar.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct IntentoCalibracion {
    pub calibro: bool,
    pub dias_con_comida: i64,
    pub dias_con_pesaje: i64,
    pub anterior: i64,
    pub nuevo: i64,
}

/// Recalibra el gasto con los últimos 14 días si toca (o si se pide). Hacen
/// falta 10 días con comida (de al menos 600 kcal) y 6 con pesaje.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta o la escritura fallan.
pub fn calibrar_si_toca(conn: &Connection, forzar: bool) -> ScResult<Option<IntentoCalibracion>> {
    let Some(fila) = perfil::leer(conn)? else {
        return Ok(None);
    };
    let ahora = fechas::ahora();
    if fila.perfil.macros_a_mano && !forzar {
        return Ok(None);
    }
    let toca = fila
        .ultima_calibracion
        .is_none_or(|u| fechas::dias_entre(u, ahora) >= 14);
    if !toca && !forzar {
        return Ok(None);
    }
    let hasta = fechas::clave(ahora.date());
    let desde = fechas::clave(fechas::mas_dias(ahora.date(), -13));
    let dias_comida: Vec<f64> = {
        let mut st = conn
            .prepare(
                "SELECT sum(total_kcal) AS k FROM food_logs WHERE date_key >= ?1 AND date_key <= ?2
                 GROUP BY date_key HAVING k >= 600",
            )
            .map_err(fallo)?;
        let v = st
            .query_map(params![desde, hasta], |f| f.get(0))
            .map_err(fallo)?
            .collect::<Result<_, _>>()
            .map_err(fallo)?;
        v
    };
    let pesajes: Vec<(String, f64)> = {
        let mut st = conn
            .prepare(
                "SELECT date_key, trend_weight_kg FROM body_measurements
                 WHERE date_key >= ?1 AND date_key <= ?2 AND source != 'manual_navy_tape'
                 ORDER BY measured_at ASC",
            )
            .map_err(fallo)?;
        let v = st
            .query_map(params![desde, hasta], |f| Ok((f.get(0)?, f.get(1)?)))
            .map_err(fallo)?
            .collect::<Result<_, _>>()
            .map_err(fallo)?;
        v
    };
    let dias_pesaje = pesajes
        .iter()
        .map(|p| p.0.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len() as i64;
    let anterior = fila.perfil.gasto_adaptativo;
    let intento = IntentoCalibracion {
        calibro: false,
        dias_con_comida: dias_comida.len() as i64,
        dias_con_pesaje: dias_pesaje,
        anterior,
        nuevo: anterior,
    };
    if dias_comida.len() < 10 || dias_pesaje < 6 {
        return Ok(Some(intento));
    }
    let media = dias_comida.iter().sum::<f64>() / dias_comida.len() as f64;
    let (inicio, fin) = (pesajes[0].1, pesajes[pesajes.len() - 1].1);
    let edad = fechas::edad(fila.nacimiento, ahora);
    let basal = formulas::basal_estimado(fin, fila.perfil.altura_cm, edad, fila.perfil.sexo);
    let (nuevo, bruto, _) = formulas::calibrar_gasto(media, inicio, fin, anterior, basal);
    let acotada = (nuevo - bruto).abs() > 1;
    conn.execute(
        "INSERT INTO tdee_calibrations (calibrated_at, date_key, previous_tdee_kcal, new_tdee_kcal,
            calculated_raw_tdee_kcal, was_clamped, delta_ema14d, average_kcal14d, days_evaluated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            fechas::texto(ahora),
            hasta,
            anterior,
            nuevo,
            bruto,
            acotada,
            formulas::fijo(fin - inicio, 2),
            formulas::fijo(media, 1),
            dias_comida.len() as i64
        ],
    )
    .map_err(fallo)?;
    conn.execute(
        "UPDATE user_profiles SET current_adaptive_tdee_kcal = ?1, last_tdee_calibration = ?2",
        params![nuevo, fechas::texto(ahora)],
    )
    .map_err(fallo)?;
    Ok(Some(IntentoCalibracion {
        calibro: true,
        nuevo,
        ..intento
    }))
}

/// Recalibra ahora, a petición de la persona.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si falla.
pub fn calibrar_ahora(db: &BaseDeDatos) -> ScResult<IntentoCalibracion> {
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let r = calibrar_si_toca(&tx, true)?.ok_or(ScError::NotFound)?;
    tx.commit().map_err(fallo)?;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perfil::pruebas::base_con_perfil;

    #[test]
    fn la_tendencia_se_mueve_un_decimo_y_el_pesaje_premia_una_vez() {
        let db = base_con_perfil();
        let r = pesar(&db, 83.0).unwrap();
        assert_eq!(r.tendencia_kg, 82.1);
        assert!(
            r.recompensa.is_none(),
            "el de la bienvenida ya se premió hoy"
        );
    }

    #[test]
    fn una_subida_tras_un_dia_salado_es_agua() {
        let db = base_con_perfil();
        {
            let conn = db.conexion().unwrap();
            conn.execute(
                "INSERT INTO food_items (name, source, kcal_per100g, protein_per100g, carbs_per100g, fat_per100g)
                 VALUES ('x', 'user_custom', 100, 1, 1, 1)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO food_logs (date_key, meal_type, food_item_id, grams_consumed, total_kcal,
                    total_protein_g, total_carbs_g, total_fat_g, total_fiber_g, total_sodium_mg,
                    total_potassium_mg, total_magnesium_mg)
                 VALUES ('2026-11-19', 'lunch', 1, 100, 100, 1, 1, 1, 0, 3200, 0, 0)",
                [],
            )
            .unwrap();
        }
        idioma::fijar(Idioma::Es);
        let r = pesar(&db, 82.6).unwrap();
        let t = r.falsa_subida.expect("explicada");
        assert!(
            t.starts_with("+0,6 kg de agua temporal por sodio elevado ayer (3200 mg)"),
            "{t}"
        );
    }

    #[test]
    fn los_hitos_van_de_1_5_en_1_5_y_el_ultimo_es_la_meta() {
        let db = BaseDeDatos::en_memoria().unwrap();
        let conn = db.conexion().unwrap();
        regenerar_hitos(&conn, 80.0, 75.8).unwrap();
        let v: Vec<f64> = conn
            .prepare("SELECT target_weight_kg FROM milestone_goals ORDER BY milestone_order")
            .unwrap()
            .query_map([], |f| f.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(v, vec![78.5, 77.0, 75.8]);
    }

    #[test]
    fn la_tendencia_que_cruza_un_hito_lo_celebra() {
        let db = base_con_perfil();
        for p in [79.0, 79.0, 79.0, 79.0, 79.0, 79.0, 79.0, 79.0, 79.0, 79.0] {
            pesar(&db, p).unwrap();
        }
        let r = resumen(&db).unwrap();
        assert!(r.hitos[0].logrado, "{:?}", r.hitos);
    }

    #[test]
    fn el_ejercicio_descuenta_el_reposo() {
        assert_eq!(
            kcal_netas(met(Categoria::Cardio, Intensidad::Moderada), 80.0, 30),
            273
        );
        assert_eq!(kcal_netas(5.0, 0.0, 30), 0);
    }

    #[test]
    fn sin_datos_suficientes_no_se_recalibra() {
        let db = base_con_perfil();
        let r = calibrar_ahora(&db).unwrap();
        assert!(!r.calibro);
        assert_eq!(r.dias_con_pesaje, 1);
    }

    #[test]
    fn la_fase_lutea_son_los_dias_antes_del_periodo() {
        let inicio = fechas::de_clave("2026-11-01").unwrap();
        assert_eq!(
            fase_del_ciclo(inicio, 28, fechas::de_clave("2026-11-03").unwrap()).0,
            "menstrual"
        );
        assert_eq!(
            fase_del_ciclo(inicio, 28, fechas::de_clave("2026-11-20").unwrap()).0,
            "luteal"
        );
        assert_eq!(
            fase_del_ciclo(inicio, 28, fechas::de_clave("2026-10-20").unwrap()).0,
            "unknown"
        );
    }
}
