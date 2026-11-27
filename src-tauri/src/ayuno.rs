//! Ayuno: el cronómetro, sus fases metabólicas orientativas y los avisos de
//! seguridad, con las reglas del teléfono.
//!
//! Las fases son estimaciones, no mediciones: la interfaz lo dice. Un ayuno de
//! 14 h o más da monedas; uno de 18 h o más muestra la guía para romperlo con
//! calma. Antes de empezar, la seguridad personal (antecedentes de TCA,
//! medicación que baja la glucosa) manda sobre la dosificación semanal.

use chrono::NaiveDateTime;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use crate::db::{self, fallo, BaseDeDatos};
use crate::error::{ScError, ScResult};
use crate::fechas;
use crate::nibi::{self, Accion, Recompensa};
use crate::perfil;

/// Ayuno mínimo que da monedas.
pub const HORAS_PARA_MONEDAS: f64 = 14.0;
/// Ayuno a partir del cual se muestra la guía para romperlo.
pub const HORAS_PARA_GUIA: f64 = 18.0;
/// Tope con antecedentes de trastornos de la conducta alimentaria.
pub const TOPE_CON_TCA: i64 = 16;

/// Los protocolos que se ofrecen, con sus horas.
pub const PROTOCOLOS: [(&str, i64); 6] = [
    ("12:12", 12),
    ("14:10", 14),
    ("16:8", 16),
    ("18:6", 18),
    ("20:4", 20),
    ("OMAD", 23),
];

/// La fase metabólica orientativa por horas de ayuno.
#[must_use]
pub fn fase(horas: f64) -> &'static str {
    if horas >= 24.0 {
        "autophagy"
    } else if horas >= 18.0 {
        "ketosis"
    } else if horas >= 12.0 {
        "fatBurning"
    } else if horas >= 4.0 {
        "glycogenolysis"
    } else {
        "anabolic"
    }
}

/// La hora a la que empieza la fase siguiente, si la hay.
#[must_use]
pub fn siguiente_fase(horas: f64) -> Option<f64> {
    [4.0, 12.0, 18.0, 24.0].into_iter().find(|h| horas < *h)
}

/// El ayuno en curso.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct EnCurso {
    pub id: i64,
    pub protocolo: String,
    pub horas_meta: i64,
    /// Inicio en formato ISO local (`YYYY-MM-DDTHH:MM:SS`), para el cronómetro.
    pub inicio: String,
    pub horas: f64,
    pub fase: String,
    pub siguiente_fase: Option<f64>,
    pub progreso: f64,
}

/// Un ayuno terminado.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Pasado {
    pub id: i64,
    pub protocolo: String,
    pub dia: String,
    pub horas: f64,
    pub horas_meta: i64,
    pub completo: bool,
}

/// Lo que muestra Ayuno.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct EstadoAyuno {
    pub activo: bool,
    pub en_curso: Option<EnCurso>,
    pub historial: Vec<Pasado>,
    pub racha_semanal: i64,
    pub media_horas: f64,
}

fn horas_entre(desde: NaiveDateTime, hasta: NaiveDateTime) -> f64 {
    (hasta - desde).num_minutes() as f64 / 60.0
}

fn en_curso(conn: &Connection) -> ScResult<Option<(i64, String, i64, NaiveDateTime)>> {
    let fila: Option<(i64, String, i64, String)> = conn
        .query_row(
            "SELECT id, protocol_name, target_hours, started_at FROM fasting_sessions
             WHERE actual_ended_at IS NULL ORDER BY started_at DESC LIMIT 1",
            [],
            |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?, f.get(3)?)),
        )
        .optional()
        .map_err(fallo)?;
    Ok(fila.and_then(|(id, p, h, i)| Some((id, p, h, fechas::leer(&i)?))))
}

/// Las horas del ayuno en curso, si lo hay. Lo usan Hoy, Nibi y la barra.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn horas_en_curso(conn: &Connection) -> ScResult<Option<f64>> {
    Ok(en_curso(conn)?.map(|(_, _, _, i)| horas_entre(i, fechas::ahora()).max(0.0)))
}

/// El estado de Ayuno.
///
/// # Errors
///
/// [`ScError::NotFound`] sin perfil.
pub fn estado(db: &BaseDeDatos) -> ScResult<EstadoAyuno> {
    let mut conn = db.conexion()?;
    let tx = db::lectura_coherente(&mut conn)?;
    let activo = perfil::exigir(&tx)?.perfil.ayuno_activo;
    let ahora = fechas::ahora();
    let en_curso = en_curso(&tx)?.map(|(id, protocolo, horas_meta, inicio)| {
        let horas = horas_entre(inicio, ahora).max(0.0);
        EnCurso {
            id,
            protocolo,
            horas_meta,
            inicio: inicio.format("%Y-%m-%dT%H:%M:%S").to_string(),
            horas,
            fase: fase(horas).into(),
            siguiente_fase: siguiente_fase(horas),
            progreso: if horas_meta > 0 {
                (horas / horas_meta as f64).min(1.0)
            } else {
                0.0
            },
        }
    });
    let mut st = tx
        .prepare(
            "SELECT id, protocol_name, target_hours, started_at, actual_ended_at, is_completed
             FROM fasting_sessions WHERE actual_ended_at IS NOT NULL
             ORDER BY started_at DESC LIMIT 30",
        )
        .map_err(fallo)?;
    let historial: Vec<Pasado> = st
        .query_map([], |f| {
            let inicio = fechas::leer(&f.get::<_, String>(3)?).unwrap_or_default();
            let fin = fechas::leer(&f.get::<_, String>(4)?).unwrap_or(inicio);
            Ok(Pasado {
                id: f.get(0)?,
                protocolo: f.get(1)?,
                horas_meta: f.get(2)?,
                dia: fechas::clave(fin.date()),
                horas: (horas_entre(inicio, fin) * 10.0).round() / 10.0,
                completo: f.get(5)?,
            })
        })
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    drop(st);
    let semana = fechas::clave(fechas::mas_dias(ahora.date(), -7));
    let racha_semanal = historial
        .iter()
        .filter(|p| p.dia > semana && p.completo)
        .count() as i64;
    let media_horas = if historial.is_empty() {
        0.0
    } else {
        let n = historial.len().min(10);
        (historial.iter().take(n).map(|p| p.horas).sum::<f64>() / n as f64 * 10.0).round() / 10.0
    };
    Ok(EstadoAyuno {
        activo,
        en_curso,
        historial,
        racha_semanal,
        media_horas,
    })
}

/// Un aviso antes de empezar. Los textos están en los diccionarios.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Aviso {
    /// `tca`, `glucosa`, `seguidos` o `semana`.
    pub motivo: String,
    pub alternativa: Option<i64>,
    /// Horas del último ayuno largo o cuántos van esta semana.
    pub dato: Option<f64>,
}

/// La seguridad de un ayuno propuesto, con las reglas del teléfono.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn comprobar(db: &BaseDeDatos, horas: i64) -> ScResult<Option<Aviso>> {
    let conn = db.conexion()?;
    if let Some(f) = perfil::leer(&conn)? {
        if f.perfil.antecedentes_tca && horas > TOPE_CON_TCA {
            return Ok(Some(Aviso {
                motivo: "tca".into(),
                alternativa: Some(TOPE_CON_TCA),
                dato: None,
            }));
        }
        let tratamiento: bool = conn
            .query_row(
                "SELECT 1 FROM supplements_tracker WHERE lowers_glucose = 1 AND is_enabled = 1 LIMIT 1",
                [],
                |_| Ok(true),
            )
            .optional()
            .map_err(fallo)?
            .unwrap_or(false);
        if f.perfil.toma_hipoglucemiantes || tratamiento {
            return Ok(Some(Aviso {
                motivo: "glucosa".into(),
                alternativa: None,
                dato: None,
            }));
        }
    }
    if horas < 20 {
        return Ok(None);
    }
    let ahora = fechas::ahora();
    let mut st = conn
        .prepare(
            "SELECT started_at, actual_ended_at FROM fasting_sessions
             WHERE actual_ended_at IS NOT NULL ORDER BY started_at DESC LIMIT 10",
        )
        .map_err(fallo)?;
    let recientes: Vec<(NaiveDateTime, NaiveDateTime)> = st
        .query_map([], |f| Ok((f.get::<_, String>(0)?, f.get::<_, String>(1)?)))
        .map_err(fallo)?
        .filter_map(Result::ok)
        .filter_map(|(a, b)| Some((fechas::leer(&a)?, fechas::leer(&b)?)))
        .collect();
    if let Some((i, f)) = recientes.first() {
        let h = horas_entre(*i, *f);
        if h >= 19.5 && (ahora - *f).num_hours() < 36 {
            return Ok(Some(Aviso {
                motivo: "seguidos".into(),
                alternativa: Some(16),
                dato: Some((h * 10.0).round() / 10.0),
            }));
        }
    }
    let largos = recientes
        .iter()
        .filter(|(i, f)| (ahora - *f).num_days() < 7 && horas_entre(*i, *f) >= 19.5)
        .count();
    if largos >= 2 {
        return Ok(Some(Aviso {
            motivo: "semana".into(),
            alternativa: Some(16),
            dato: Some(largos as f64),
        }));
    }
    Ok(None)
}

/// Empieza un ayuno. Si ya hay uno, devuelve ese. Un inicio en el futuro se
/// acota a ahora; uno de hace más de 48 h no se acepta.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si el protocolo o el inicio no valen.
pub fn empezar(db: &BaseDeDatos, protocolo: &str, horas: i64, hace_minutos: i64) -> ScResult<i64> {
    let protocolo = protocolo.trim();
    if protocolo.is_empty()
        || protocolo.chars().count() > 20
        || !(1..=72).contains(&horas)
        || !(0..=48 * 60).contains(&hace_minutos)
    {
        return Err(ScError::InvalidInput);
    }
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    if let Some((id, ..)) = en_curso(&tx)? {
        return Ok(id);
    }
    let inicio = fechas::ahora() - chrono::Duration::minutes(hace_minutos);
    tx.execute(
        "INSERT INTO fasting_sessions (protocol_name, target_hours, started_at, planned_end_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            protocolo,
            horas,
            fechas::texto(inicio),
            fechas::texto(inicio + chrono::Duration::hours(horas))
        ],
    )
    .map_err(fallo)?;
    let id = tx.last_insert_rowid();
    tx.commit().map_err(fallo)?;
    Ok(id)
}

/// Lo que dejó terminar un ayuno.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Terminado {
    pub horas: f64,
    pub completo: bool,
    pub recompensa: Option<Recompensa>,
    pub guia: bool,
}

/// Termina el ayuno en curso.
///
/// # Errors
///
/// [`ScError::NotFound`] si no hay ninguno.
pub fn terminar(db: &BaseDeDatos) -> ScResult<Terminado> {
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let (id, _, meta, inicio) = en_curso(&tx)?.ok_or(ScError::NotFound)?;
    let fin = fechas::ahora();
    let horas = horas_entre(inicio, fin).max(0.0);
    let completo = horas >= meta as f64;
    tx.execute(
        "UPDATE fasting_sessions SET actual_ended_at = ?1, is_completed = ?2 WHERE id = ?3",
        params![fechas::texto(fin), completo, id],
    )
    .map_err(fallo)?;
    let recompensa = if horas >= HORAS_PARA_MONEDAS {
        nibi::premiar(&tx, Accion::AyunoCompleto)?
    } else {
        None
    };
    tx.commit().map_err(fallo)?;
    Ok(Terminado {
        horas: (horas * 10.0).round() / 10.0,
        completo,
        recompensa,
        guia: horas >= HORAS_PARA_GUIA,
    })
}

/// Corrige la hora de inicio del ayuno en curso (no puede quedar en el futuro).
///
/// # Errors
///
/// [`ScError::NotFound`] sin ayuno; [`ScError::InvalidInput`] fuera de rango.
pub fn ajustar_inicio(db: &BaseDeDatos, hace_minutos: i64) -> ScResult<()> {
    if !(0..=72 * 60).contains(&hace_minutos) {
        return Err(ScError::InvalidInput);
    }
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let (id, _, horas, _) = en_curso(&tx)?.ok_or(ScError::NotFound)?;
    let inicio = fechas::ahora() - chrono::Duration::minutes(hace_minutos);
    tx.execute(
        "UPDATE fasting_sessions SET started_at = ?1, planned_end_at = ?2 WHERE id = ?3",
        params![
            fechas::texto(inicio),
            fechas::texto(inicio + chrono::Duration::hours(horas)),
            id
        ],
    )
    .map_err(fallo)?;
    tx.commit().map_err(fallo)
}

/// Borra un ayuno (en curso o pasado).
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn borrar(db: &BaseDeDatos, id: i64) -> ScResult<()> {
    let conn = db.conexion()?;
    if conn
        .execute("DELETE FROM fasting_sessions WHERE id = ?1", [id])
        .map_err(fallo)?
        == 0
    {
        return Err(ScError::NotFound);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perfil::pruebas::base_con_perfil;

    #[test]
    fn las_fases_son_las_del_telefono() {
        assert_eq!(fase(2.0), "anabolic");
        assert_eq!(fase(4.0), "glycogenolysis");
        assert_eq!(fase(13.0), "fatBurning");
        assert_eq!(fase(18.0), "ketosis");
        assert_eq!(fase(30.0), "autophagy");
        assert_eq!(siguiente_fase(13.0), Some(18.0));
        assert_eq!(siguiente_fase(30.0), None);
    }

    #[test]
    fn un_ayuno_de_16_horas_completo_da_monedas_y_no_hay_dos_a_la_vez() {
        let db = base_con_perfil();
        let id = empezar(&db, "16:8", 16, 16 * 60 + 5).unwrap();
        assert_eq!(empezar(&db, "18:6", 18, 0).unwrap(), id);
        let e = estado(&db).unwrap();
        assert_eq!(
            e.en_curso.as_ref().map(|c| c.fase.as_str()),
            Some("fatBurning")
        );
        let t = terminar(&db).unwrap();
        assert!(t.completo);
        assert!(t.recompensa.is_some());
        assert!(!t.guia);
        assert_eq!(terminar(&db), Err(ScError::NotFound));
    }

    #[test]
    fn con_antecedentes_un_ayuno_largo_avisa_y_propone_16() {
        let db = base_con_perfil();
        crate::perfil::cambiar(
            &db,
            &crate::perfil::CambioPerfil {
                antecedentes_tca: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        let a = comprobar(&db, 18).unwrap().unwrap();
        assert_eq!((a.motivo.as_str(), a.alternativa), ("tca", Some(16)));
        assert_eq!(comprobar(&db, 14).unwrap(), None);
    }

    #[test]
    fn no_se_encadenan_dos_ayunos_de_20_horas() {
        let db = base_con_perfil();
        empezar(&db, "20:4", 20, 21 * 60).unwrap();
        terminar(&db).unwrap();
        let a = comprobar(&db, 20).unwrap().unwrap();
        assert_eq!(a.motivo, "seguidos");
        assert_eq!(
            comprobar(&db, 16).unwrap(),
            None,
            "lo moderado siempre se puede"
        );
    }
}
