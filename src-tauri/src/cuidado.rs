//! Cuidado: tratamientos y tomas, hábitos propios, sesiones guiadas (pausas
//! activas, descanso visual, suelo pélvico y respiración), signos vitales,
//! síntomas, chequeos preventivos, alcohol y tabaco, Libre y el índice de
//! bienestar OMS-5.
//!
//! Las reglas son las del teléfono. Los medicamentos **no dan monedas** (un
//! tratamiento no es un juego) y una toma se marca "tomada" u "omitida", nunca
//! "olvidada". Un reinicio de Libre conserva lo ganado. Los rangos de los
//! signos vitales son orientativos y no diagnostican: la interfaz lo dice.

use chrono::{Datelike, Months, NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::{self, fallo, BaseDeDatos};
use crate::error::{ScError, ScResult};
use crate::fechas;
use crate::formulas::Sexo;
use crate::idioma::t;
use crate::nibi::{self, Accion, Recompensa};
use crate::perfil;

fn texto_limpio(v: Option<&str>, max: usize) -> Option<String> {
    let t = v?.trim();
    (!t.is_empty()).then(|| t.chars().take(max).collect())
}

fn listar<T>(
    conn: &Connection,
    sql: &str,
    p: impl rusqlite::Params,
    f: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> ScResult<Vec<T>> {
    let mut st = conn.prepare(sql).map_err(fallo)?;
    let v = st
        .query_map(p, f)
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    Ok(v)
}

fn aplica(mascara: i64, dia: NaiveDate) -> bool {
    mascara & i64::from(fechas::bit_del_dia(dia)) != 0
}

// --- Tratamientos ----------------------------------------------------------

/// Un medicamento o suplemento con su horario.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Tratamiento {
    pub id: i64,
    pub nombre: String,
    pub dosis: String,
    /// `medication` o `supplement`.
    pub tipo: String,
    /// Minutos desde medianoche de cada toma, en orden.
    pub horas: Vec<i64>,
    pub dias: i64,
    pub reserva: Option<f64>,
    pub unidades_por_toma: f64,
    pub avisos: bool,
    pub baja_glucosa: bool,
    pub retiene_agua: bool,
    pub notas: Option<String>,
    pub activo: bool,
    /// Días de reserva que quedan, si se lleva la cuenta.
    pub dias_de_reserva: Option<i64>,
}

/// Un tratamiento nuevo o editado.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct BorradorTratamiento {
    pub nombre: String,
    pub dosis: String,
    pub tipo: String,
    pub horas: Vec<i64>,
    pub dias: i64,
    pub reserva: Option<f64>,
    pub unidades_por_toma: f64,
    pub avisos: bool,
    pub baja_glucosa: bool,
    pub retiene_agua: bool,
    pub notas: Option<String>,
}

fn horas_de(texto: &str) -> Vec<i64> {
    let mut v: Vec<i64> = serde_json::from_str::<Vec<f64>>(texto)
        .unwrap_or_default()
        .into_iter()
        .map(|x| x as i64)
        .filter(|x| (0..1440).contains(x))
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

fn dias_de_reserva(reserva: Option<f64>, por_toma: f64, tomas: usize, dias: i64) -> Option<i64> {
    let reserva = reserva?;
    let activos = (0..7).filter(|i| dias & (1 << i) != 0).count();
    if activos == 0 || tomas == 0 || por_toma <= 0.0 {
        return None;
    }
    let por_dia = por_toma * tomas as f64 * activos as f64 / 7.0;
    Some((reserva / por_dia).floor() as i64)
}

fn tratamiento_de(f: &rusqlite::Row<'_>) -> rusqlite::Result<Tratamiento> {
    let horas = horas_de(&f.get::<_, String>("schedule_json")?);
    let dias: i64 = f.get("days_of_week_bitmask")?;
    let reserva: Option<f64> = f.get("stock_units")?;
    let por_toma: f64 = f.get("units_per_dose")?;
    Ok(Tratamiento {
        id: f.get("id")?,
        nombre: f.get("name")?,
        dosis: f.get("dosage")?,
        tipo: f.get("kind")?,
        dias_de_reserva: dias_de_reserva(reserva, por_toma, horas.len().max(1), dias),
        horas,
        dias,
        reserva,
        unidades_por_toma: por_toma,
        avisos: f.get("reminders_enabled")?,
        baja_glucosa: f.get("lowers_glucose")?,
        retiene_agua: f.get("affects_water_retention")?,
        notas: f.get("notes")?,
        activo: f.get("is_enabled")?,
    })
}

/// Los tratamientos, activos primero.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn tratamientos(conn: &Connection) -> ScResult<Vec<Tratamiento>> {
    listar(
        conn,
        "SELECT * FROM supplements_tracker ORDER BY is_enabled DESC, lower(name) LIMIT 200",
        [],
        tratamiento_de,
    )
}

/// Guarda un tratamiento (nuevo si `id` es `None`).
///
/// # Errors
///
/// [`ScError::InvalidInput`] si el nombre está vacío o los valores no valen.
pub fn guardar_tratamiento(
    db: &BaseDeDatos,
    id: Option<i64>,
    b: &BorradorTratamiento,
) -> ScResult<i64> {
    let nombre = texto_limpio(Some(&b.nombre), 60).ok_or(ScError::InvalidInput)?;
    if !["medication", "supplement"].contains(&b.tipo.as_str())
        || b.horas.len() > 12
        || b.reserva.is_some_and(|r| !(0.0..=100_000.0).contains(&r))
        || !(0.0..=100.0).contains(&b.unidades_por_toma)
    {
        return Err(ScError::InvalidInput);
    }
    let mut horas: Vec<i64> = b
        .horas
        .iter()
        .copied()
        .filter(|h| (0..1440).contains(h))
        .collect();
    horas.sort_unstable();
    horas.dedup();
    let dias = if b.dias & 0x7F == 0 {
        127
    } else {
        b.dias & 0x7F
    };
    let conn = db.conexion()?;
    let valores = params![
        nombre,
        b.dosis.trim().chars().take(40).collect::<String>(),
        b.tipo,
        serde_json::to_string(&horas).unwrap_or_else(|_| "[]".into()),
        horas.first().copied(),
        dias,
        b.reserva,
        if b.unidades_por_toma > 0.0 {
            b.unidades_por_toma
        } else {
            1.0
        },
        b.avisos && !horas.is_empty(),
        b.baja_glucosa,
        b.retiene_agua,
        texto_limpio(b.notas.as_deref(), 300),
    ];
    match id {
        Some(id) => {
            let n = conn
                .execute(
                    "UPDATE supplements_tracker SET name = ?1, dosage = ?2, kind = ?3, schedule_json = ?4,
                        target_time_of_day_minutes = ?5, days_of_week_bitmask = ?6, stock_units = ?7,
                        units_per_dose = ?8, reminders_enabled = ?9, lowers_glucose = ?10,
                        affects_water_retention = ?11, notes = ?12 WHERE id = ?13",
                    rusqlite::params_from_iter(
                        valores.iter().copied().chain(std::iter::once(&id as &dyn rusqlite::ToSql)),
                    ),
                )
                .map_err(fallo)?;
            if n == 0 {
                return Err(ScError::NotFound);
            }
            Ok(id)
        }
        None => {
            conn.execute(
                "INSERT INTO supplements_tracker (name, dosage, kind, schedule_json,
                    target_time_of_day_minutes, days_of_week_bitmask, stock_units, units_per_dose,
                    reminders_enabled, lowers_glucose, affects_water_retention, notes, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                rusqlite::params_from_iter(valores.iter().copied().chain(std::iter::once(
                    &fechas::texto(fechas::ahora()) as &dyn rusqlite::ToSql,
                ))),
            )
            .map_err(fallo)?;
            Ok(conn.last_insert_rowid())
        }
    }
}

/// Pausa, reanuda o borra un tratamiento.
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn tratamiento_activo(db: &BaseDeDatos, id: i64, activo: Option<bool>) -> ScResult<()> {
    let conn = db.conexion()?;
    let n = match activo {
        Some(a) => conn.execute(
            "UPDATE supplements_tracker SET is_enabled = ?1 WHERE id = ?2",
            params![a, id],
        ),
        None => conn.execute("DELETE FROM supplements_tracker WHERE id = ?1", [id]),
    }
    .map_err(fallo)?;
    if n == 0 {
        return Err(ScError::NotFound);
    }
    Ok(())
}

/// Repone la reserva de un tratamiento.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si no son unidades positivas.
pub fn reponer(db: &BaseDeDatos, id: i64, unidades: f64) -> ScResult<()> {
    if !(unidades > 0.0 && unidades <= 10_000.0) {
        return Err(ScError::InvalidInput);
    }
    let conn = db.conexion()?;
    if conn
        .execute(
            "UPDATE supplements_tracker SET stock_units = coalesce(stock_units, 0) + ?1 WHERE id = ?2",
            params![unidades, id],
        )
        .map_err(fallo)?
        == 0
    {
        return Err(ScError::NotFound);
    }
    Ok(())
}

/// Una toma del día y su estado.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Toma {
    pub tratamiento: i64,
    pub nombre: String,
    pub dosis: String,
    pub tipo: String,
    /// Minutos desde medianoche, o `None` si es la toma única del día.
    pub hora: Option<i64>,
    /// `pending`, `taken` o `skipped`.
    pub estado: String,
}

/// Las tomas de un día, con su estado.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn tomas_del_dia(conn: &Connection, dia: NaiveDate) -> ScResult<Vec<Toma>> {
    let clave = fechas::clave(dia);
    let registros: Vec<(i64, Option<i64>, bool)> = listar(
        conn,
        "SELECT supplement_id, slot_minutes, was_taken FROM supplement_logs WHERE date_key = ?1 ORDER BY id",
        [&clave],
        |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)),
    )?;
    let mut v = Vec::new();
    for t in tratamientos(conn)?
        .into_iter()
        .filter(|t| t.activo && aplica(t.dias, dia))
    {
        let huecos: Vec<Option<i64>> = if t.horas.is_empty() {
            vec![None]
        } else {
            t.horas.iter().copied().map(Some).collect()
        };
        for h in huecos {
            let estado = registros
                .iter()
                .rev()
                .find(|r| r.0 == t.id && r.1 == h)
                .map_or("pending", |r| if r.2 { "taken" } else { "skipped" });
            v.push(Toma {
                tratamiento: t.id,
                nombre: t.nombre.clone(),
                dosis: t.dosis.clone(),
                tipo: t.tipo.clone(),
                hora: h,
                estado: estado.into(),
            });
        }
    }
    v.sort_by_key(|x| x.hora.unwrap_or(-1));
    Ok(v)
}

fn ajustar_reserva(conn: &Connection, id: i64, tomada: bool) -> ScResult<()> {
    conn.execute(
        "UPDATE supplements_tracker SET stock_units = max(0, stock_units + CASE WHEN ?1 THEN -units_per_dose ELSE units_per_dose END)
         WHERE id = ?2 AND stock_units IS NOT NULL",
        params![tomada, id],
    )
    .map_err(fallo)?;
    Ok(())
}

/// Marca una toma de hoy como tomada u omitida, o la deja pendiente
/// (`estado = None`). La reserva sigue lo que se toma. **No da monedas.**
///
/// # Errors
///
/// [`ScError::StorageFailure`] si falla.
pub fn marcar_toma(
    db: &BaseDeDatos,
    tratamiento: i64,
    hora: Option<i64>,
    tomada: Option<bool>,
) -> ScResult<()> {
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let dia = fechas::clave(fechas::hoy());
    let previa: Option<(i64, bool)> = tx
        .query_row(
            "SELECT id, was_taken FROM supplement_logs WHERE date_key = ?1 AND supplement_id = ?2
             AND slot_minutes IS ?3 LIMIT 1",
            params![dia, tratamiento, hora],
            |f| Ok((f.get(0)?, f.get(1)?)),
        )
        .optional()
        .map_err(fallo)?;
    let antes = previa.is_some_and(|p| p.1);
    match (previa, tomada) {
        (Some((id, _)), Some(t)) => {
            tx.execute(
                "UPDATE supplement_logs SET was_taken = ?1, taken_at = ?2 WHERE id = ?3",
                params![t, fechas::texto(fechas::ahora()), id],
            )
            .map_err(fallo)?;
        }
        (None, Some(t)) => {
            tx.execute(
                "INSERT INTO supplement_logs (date_key, supplement_id, taken_at, was_taken, slot_minutes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![dia, tratamiento, fechas::texto(fechas::ahora()), t, hora],
            )
            .map_err(fallo)?;
        }
        (Some((id, _)), None) => {
            tx.execute("DELETE FROM supplement_logs WHERE id = ?1", [id])
                .map_err(fallo)?;
        }
        (None, None) => {}
    }
    let despues = tomada == Some(true);
    if antes != despues {
        ajustar_reserva(&tx, tratamiento, despues)?;
    }
    tx.commit().map_err(fallo)
}

// --- Hábitos propios -------------------------------------------------------

/// Un hábito propio y si está hecho hoy.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Habito {
    pub id: i64,
    pub nombre: String,
    pub icono: String,
    pub dias: i64,
    pub toca_hoy: bool,
    pub hecho_hoy: bool,
    /// Días seguidos (de los que tocaba) hechos hasta hoy.
    pub racha: i64,
    /// Lo hecho en los últimos 28 días, del más antiguo a hoy.
    pub mes: Vec<bool>,
}

/// Los hábitos propios activos.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn habitos(conn: &Connection) -> ScResult<Vec<Habito>> {
    let hoy = fechas::hoy();
    let base: Vec<(i64, String, String, i64)> = listar(
        conn,
        "SELECT id, name, icon_key, days_of_week_bitmask FROM custom_habits WHERE is_archived = 0 ORDER BY sort_order, id",
        [],
        |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?, f.get(3)?)),
    )?;
    let desde = fechas::clave(fechas::mas_dias(hoy, -400));
    base.into_iter()
        .map(|(id, nombre, icono, dias)| {
            let hechos: std::collections::HashSet<String> = listar(
                conn,
                "SELECT date_key FROM habit_logs WHERE habit_id = ?1 AND date_key >= ?2",
                params![id, desde],
                |f| f.get(0),
            )?
            .into_iter()
            .collect();
            let hecho = |d: NaiveDate| hechos.contains(&fechas::clave(d));
            let mut racha = 0;
            let mut d = hoy;
            if !hecho(d) {
                d = fechas::mas_dias(d, -1);
            }
            for _ in 0..400 {
                if !aplica(dias, d) {
                    d = fechas::mas_dias(d, -1);
                    continue;
                }
                if !hecho(d) {
                    break;
                }
                racha += 1;
                d = fechas::mas_dias(d, -1);
            }
            Ok(Habito {
                id,
                nombre,
                icono,
                dias,
                toca_hoy: aplica(dias, hoy),
                hecho_hoy: hecho(hoy),
                racha,
                mes: (0..28)
                    .rev()
                    .map(|i| hecho(fechas::mas_dias(hoy, -i)))
                    .collect(),
            })
        })
        .collect()
}

/// Crea un hábito propio.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si el nombre está vacío o pasa de 40 caracteres.
pub fn crear_habito(db: &BaseDeDatos, nombre: &str, icono: &str, dias: i64) -> ScResult<i64> {
    let nombre = nombre.trim();
    if nombre.is_empty() || nombre.chars().count() > 40 || icono.len() > 24 {
        return Err(ScError::InvalidInput);
    }
    let conn = db.conexion()?;
    let orden: i64 = conn
        .query_row(
            "SELECT coalesce(max(sort_order), 0) + 1 FROM custom_habits",
            [],
            |f| f.get(0),
        )
        .map_err(fallo)?;
    conn.execute(
        "INSERT INTO custom_habits (name, icon_key, days_of_week_bitmask, sort_order, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            nombre,
            if icono.is_empty() { "spark" } else { icono },
            if dias & 0x7F == 0 { 127 } else { dias & 0x7F },
            orden,
            fechas::texto(fechas::ahora())
        ],
    )
    .map_err(fallo)?;
    Ok(conn.last_insert_rowid())
}

/// Archiva un hábito (su historial se conserva) o lo borra del todo.
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn quitar_habito(db: &BaseDeDatos, id: i64, borrar: bool) -> ScResult<()> {
    let conn = db.conexion()?;
    let n = if borrar {
        conn.execute("DELETE FROM custom_habits WHERE id = ?1", [id])
    } else {
        conn.execute(
            "UPDATE custom_habits SET is_archived = 1 WHERE id = ?1",
            [id],
        )
    }
    .map_err(fallo)?;
    if n == 0 {
        return Err(ScError::NotFound);
    }
    Ok(())
}

/// Marca o desmarca un hábito de hoy. Con todos los del día hechos, monedas.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si falla.
pub fn marcar_habito(db: &BaseDeDatos, id: i64, hecho: bool) -> ScResult<Option<Recompensa>> {
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let hoy = fechas::hoy();
    let dia = fechas::clave(hoy);
    if hecho {
        tx.execute(
            "INSERT OR IGNORE INTO habit_logs (habit_id, date_key, logged_at) VALUES (?1, ?2, ?3)",
            params![id, dia, fechas::texto(fechas::ahora())],
        )
        .map_err(fallo)?;
    } else {
        tx.execute(
            "DELETE FROM habit_logs WHERE habit_id = ?1 AND date_key = ?2",
            params![id, dia],
        )
        .map_err(fallo)?;
    }
    let mut r = None;
    if hecho {
        let del_dia: Vec<(i64, bool)> = listar(
            &tx,
            "SELECT h.days_of_week_bitmask, EXISTS(SELECT 1 FROM habit_logs l WHERE l.habit_id = h.id AND l.date_key = ?1)
             FROM custom_habits h WHERE h.is_archived = 0",
            [&dia],
            |f| Ok((f.get(0)?, f.get(1)?)),
        )?;
        let tocan: Vec<bool> = del_dia
            .into_iter()
            .filter(|(m, _)| aplica(*m, hoy))
            .map(|x| x.1)
            .collect();
        if !tocan.is_empty() && tocan.iter().all(|h| *h) {
            r = nibi::premiar(&tx, Accion::HabitosDelDia)?;
        }
    }
    tx.commit().map_err(fallo)?;
    Ok(r)
}

// --- Sesiones guiadas ------------------------------------------------------

/// Guarda una sesión guiada terminada: `active_break` (pausa activa o descanso
/// visual, según la variante), `pelvic_floor` o `breathing`. Las de menos de 20
/// segundos no cuentan.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si el tipo no existe.
pub fn completar_sesion(
    db: &BaseDeDatos,
    tipo: &str,
    variante: &str,
    segundos: i64,
) -> ScResult<Option<Recompensa>> {
    let accion = match tipo {
        "active_break" => Accion::PausaActiva,
        "pelvic_floor" => Accion::SueloPelvico,
        "breathing" => Accion::Respiracion,
        _ => return Err(ScError::InvalidInput),
    };
    if variante.is_empty() || variante.len() > 40 || !(0..=7200).contains(&segundos) {
        return Err(ScError::InvalidInput);
    }
    if segundos < 20 {
        return Ok(None);
    }
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let ahora = fechas::ahora();
    tx.execute(
        "INSERT INTO care_sessions (date_key, completed_at, kind, variant, duration_seconds) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![fechas::clave(ahora.date()), fechas::texto(ahora), tipo, variante, segundos],
    )
    .map_err(fallo)?;
    let r = nibi::premiar(&tx, accion)?;
    tx.commit().map_err(fallo)?;
    Ok(r)
}

/// Lo hecho hoy en sesiones guiadas.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct SesionesDeHoy {
    pub pausas: i64,
    pub vista: i64,
    pub pelvico: i64,
    pub respiracion: i64,
    pub minutos: i64,
    /// Nivel del suelo pélvico y sesiones hechas en ese nivel.
    pub nivel_pelvico: i64,
    pub sesiones_en_nivel: i64,
}

/// Las sesiones guiadas de hoy y el nivel del suelo pélvico.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn sesiones_de_hoy(conn: &Connection) -> ScResult<SesionesDeHoy> {
    let dia = fechas::clave(fechas::hoy());
    let filas: Vec<(String, String, i64)> = listar(
        conn,
        "SELECT kind, variant, duration_seconds FROM care_sessions WHERE date_key = ?1",
        [&dia],
        |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)),
    )?;
    let mut s = SesionesDeHoy::default();
    for (k, v, seg) in &filas {
        match (k.as_str(), v.as_str()) {
            ("active_break", "eyes") => s.vista += 1,
            ("active_break", _) => s.pausas += 1,
            ("pelvic_floor", _) => s.pelvico += 1,
            ("breathing", _) => s.respiracion += 1,
            _ => {}
        }
        s.minutos += seg;
    }
    s.minutos /= 60;
    let niveles: Vec<String> = listar(
        conn,
        "SELECT variant FROM care_sessions WHERE kind = 'pelvic_floor' AND variant LIKE 'level_%' ORDER BY completed_at DESC LIMIT 60",
        [],
        |f| f.get(0),
    )?;
    let nivel = niveles
        .first()
        .and_then(|v| v.strip_prefix("level_"))
        .and_then(|n| n.parse::<i64>().ok())
        .unwrap_or(1)
        .clamp(1, 5);
    s.nivel_pelvico = nivel;
    s.sesiones_en_nivel = niveles
        .iter()
        .take_while(|v| **v == format!("level_{nivel}"))
        .count() as i64;
    Ok(s)
}

// --- Signos vitales --------------------------------------------------------

/// La banda orientativa de una lectura.
#[must_use]
pub fn banda(
    tipo: &str,
    principal: f64,
    secundario: Option<f64>,
    contexto: Option<&str>,
) -> &'static str {
    match tipo {
        "blood_pressure" => {
            let (s, d) = (principal, secundario.unwrap_or(0.0));
            if s >= 180.0 || d >= 120.0 {
                "consultSoon"
            } else if s < 90.0 || d < 60.0 {
                "low"
            } else if s >= 130.0 || d >= 80.0 {
                "high"
            } else if s >= 120.0 {
                "elevated"
            } else {
                "normal"
            }
        }
        "glucose" => {
            if !(54.0..300.0).contains(&principal) {
                "consultSoon"
            } else if principal < 70.0 {
                "low"
            } else if contexto == Some("fasting") {
                if principal >= 126.0 {
                    "high"
                } else if principal >= 100.0 {
                    "elevated"
                } else {
                    "normal"
                }
            } else if principal >= 200.0 {
                "high"
            } else if principal >= 140.0 {
                "elevated"
            } else {
                "normal"
            }
        }
        "oxygen" => {
            if principal < 92.0 {
                "consultSoon"
            } else if principal < 95.0 {
                "elevated"
            } else {
                "normal"
            }
        }
        _ => {
            if principal >= 39.5 || principal < 35.0 {
                "consultSoon"
            } else if principal < 35.5 {
                "low"
            } else if principal >= 38.0 {
                "high"
            } else if principal >= 37.3 {
                "elevated"
            } else {
                "normal"
            }
        }
    }
}

fn plausible(tipo: &str, p: f64, s: Option<f64>) -> bool {
    match tipo {
        "blood_pressure" => {
            (60.0..=260.0).contains(&p) && s.is_some_and(|d| (30.0..=160.0).contains(&d) && d < p)
        }
        "glucose" => (20.0..=600.0).contains(&p),
        "oxygen" => (50.0..=100.0).contains(&p),
        "temperature" => (32.0..=43.0).contains(&p),
        _ => false,
    }
}

/// Una lectura.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Lectura {
    pub id: i64,
    pub tipo: String,
    pub dia: String,
    pub hora: String,
    pub principal: f64,
    pub secundario: Option<f64>,
    pub contexto: Option<String>,
    pub banda: String,
}

/// Las últimas lecturas.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn lecturas(conn: &Connection) -> ScResult<Vec<Lectura>> {
    listar(
        conn,
        "SELECT id, kind, measured_at, primary_value, secondary_value, context FROM vital_readings
         ORDER BY measured_at DESC LIMIT 80",
        [],
        |f| {
            let tipo: String = f.get(1)?;
            let cuando = fechas::leer(&f.get::<_, String>(2)?).unwrap_or_default();
            let p: f64 = f.get(3)?;
            let s: Option<f64> = f.get(4)?;
            let c: Option<String> = f.get(5)?;
            Ok(Lectura {
                id: f.get(0)?,
                banda: banda(&tipo, p, s, c.as_deref()).into(),
                tipo,
                dia: fechas::clave(cuando.date()),
                hora: cuando.format("%H:%M").to_string(),
                principal: p,
                secundario: s,
                contexto: c,
            })
        },
    )
}

/// Guarda una lectura a mano.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si no es plausible.
pub fn medir(
    db: &BaseDeDatos,
    tipo: &str,
    principal: f64,
    secundario: Option<f64>,
    contexto: Option<&str>,
) -> ScResult<String> {
    if !plausible(tipo, principal, secundario)
        || contexto.is_some_and(|c| !["fasting", "after_meal", "random"].contains(&c))
    {
        return Err(ScError::InvalidInput);
    }
    let conn = db.conexion()?;
    let ahora = fechas::ahora();
    conn.execute(
        "INSERT INTO vital_readings (kind, measured_at, date_key, primary_value, secondary_value, context, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'manual')",
        params![
            tipo,
            fechas::texto(ahora),
            fechas::clave(ahora.date()),
            principal,
            if tipo == "blood_pressure" { secundario } else { None },
            if tipo == "glucose" { contexto } else { None }
        ],
    )
    .map_err(fallo)?;
    Ok(banda(tipo, principal, secundario, contexto).into())
}

// --- Síntomas ----------------------------------------------------------------

/// Los síntomas del catálogo para esta persona.
#[must_use]
pub fn catalogo_sintomas(sexo: Sexo, edad: i64, ciclo: bool) -> Vec<&'static str> {
    let mut v = vec![
        "headache",
        "migraine",
        "back_pain",
        "joint_pain",
        "muscle_pain",
        "fatigue",
        "anxiety",
        "low_mood",
        "insomnia",
        "dizziness",
        "nausea",
        "allergy",
        "congestion",
    ];
    if sexo == Sexo::Mujer && ciclo {
        v.extend(["cramps", "breast_tenderness", "bloating"]);
    }
    if sexo == Sexo::Mujer && edad >= 40 {
        v.extend([
            "hot_flashes",
            "night_sweats",
            "brain_fog",
            "vaginal_dryness",
        ]);
    }
    v
}

/// Un síntoma registrado.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Sintoma {
    pub id: i64,
    pub clave: String,
    pub intensidad: i64,
    pub dia: String,
    pub hora: String,
    pub notas: Option<String>,
}

/// Los últimos síntomas.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn sintomas(conn: &Connection) -> ScResult<Vec<Sintoma>> {
    listar(
        conn,
        "SELECT id, symptom, intensity, logged_at, notes FROM symptom_logs ORDER BY logged_at DESC LIMIT 80",
        [],
        |f| {
            let cuando = fechas::leer(&f.get::<_, String>(3)?).unwrap_or_default();
            Ok(Sintoma {
                id: f.get(0)?,
                clave: f.get(1)?,
                intensidad: f.get(2)?,
                dia: fechas::clave(cuando.date()),
                hora: cuando.format("%H:%M").to_string(),
                notas: f.get(4)?,
            })
        },
    )
}

/// Registra un síntoma (del catálogo o con texto propio).
///
/// # Errors
///
/// [`ScError::InvalidInput`] si está vacío o la intensidad no es 1–5.
pub fn sentir(
    db: &BaseDeDatos,
    clave: &str,
    intensidad: i64,
    notas: Option<&str>,
) -> ScResult<i64> {
    let clave = texto_limpio(Some(clave), 60).ok_or(ScError::InvalidInput)?;
    if !(1..=5).contains(&intensidad) {
        return Err(ScError::InvalidInput);
    }
    let conn = db.conexion()?;
    let ahora = fechas::ahora();
    conn.execute(
        "INSERT INTO symptom_logs (date_key, logged_at, symptom, intensity, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![fechas::clave(ahora.date()), fechas::texto(ahora), clave, intensidad, texto_limpio(notas, 300)],
    )
    .map_err(fallo)?;
    Ok(conn.last_insert_rowid())
}

// --- Chequeos ----------------------------------------------------------------

/// El catálogo de chequeos: clave, meses, sexo, edad mínima y máxima, y el
/// intervalo de los más jóvenes.
type CriterioChequeo = (
    &'static str,
    i64,
    Option<Sexo>,
    i64,
    i64,
    Option<(i64, i64)>,
);

const CHEQUEOS: &[CriterioChequeo] = &[
    ("dental", 12, None, 0, 200, None),
    ("general", 24, None, 18, 200, None),
    ("vision", 24, None, 0, 200, None),
    ("blood_pressure", 12, None, 18, 200, None),
    ("lipids_glucose", 60, None, 18, 200, None),
    ("cervical", 60, Some(Sexo::Mujer), 25, 65, Some((36, 30))),
    ("mammography", 24, Some(Sexo::Mujer), 50, 69, None),
    ("prostate", 60, Some(Sexo::Hombre), 50, 75, None),
    ("colorectal", 24, None, 50, 75, None),
    ("bone_density", 24, Some(Sexo::Mujer), 65, 200, None),
    ("flu_vaccine", 12, None, 60, 200, None),
    ("tetanus", 120, None, 18, 200, None),
];

/// Siembra los chequeos del catálogo que aplican a la persona (activados los
/// que le tocan por edad), si aún no hay ninguno.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si falla.
pub fn sembrar_chequeos(conn: &Connection) -> ScResult<()> {
    let hay: i64 = conn
        .query_row("SELECT count(*) FROM checkups", [], |f| f.get(0))
        .map_err(fallo)?;
    let Some(fila) = perfil::leer(conn)? else {
        return Ok(());
    };
    if hay > 0 {
        return Ok(());
    }
    let (sexo, edad) = (fila.perfil.sexo, fila.perfil.edad);
    for (clave, meses, s, min, max, joven) in CHEQUEOS {
        if s.is_some_and(|s| s != sexo) {
            continue;
        }
        let intervalo = match joven {
            Some((m, hasta)) if edad < *hasta => *m,
            _ => *meses,
        };
        conn.execute(
            "INSERT INTO checkups (catalog_key, interval_months, is_enabled) VALUES (?1, ?2, ?3)",
            params![clave, intervalo, (*min..=*max).contains(&edad)],
        )
        .map_err(fallo)?;
    }
    Ok(())
}

/// Un chequeo y cuándo toca.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Chequeo {
    pub id: i64,
    pub clave: String,
    pub nombre: Option<String>,
    pub meses: i64,
    pub activo: bool,
    pub ultimo: Option<String>,
    pub proximo: Option<String>,
    /// `neverDone`, `upToDate`, `dueSoon` o `due`.
    pub estado: String,
    pub dias: Option<i64>,
}

/// Los chequeos con su estado.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn chequeos(conn: &Connection) -> ScResult<Vec<Chequeo>> {
    let hoy = fechas::hoy();
    let mut v = listar(
        conn,
        "SELECT c.id, c.catalog_key, c.custom_name, c.interval_months, c.is_enabled,
                (SELECT max(done_at) FROM checkup_logs l WHERE l.checkup_id = c.id)
         FROM checkups c ORDER BY c.id",
        [],
        |f| {
            let ultimo: Option<NaiveDateTime> = f
                .get::<_, Option<String>>(5)?
                .and_then(|t| fechas::leer(&t));
            let meses: i64 = f.get(3)?;
            let (estado, proximo, dias) = match ultimo {
                None => ("neverDone", None, None),
                Some(u) => {
                    let p = u
                        .date()
                        .checked_add_months(Months::new(meses.clamp(1, 240) as u32))
                        .unwrap_or(u.date());
                    let d = (p - hoy).num_days();
                    let e = if d <= 0 {
                        "due"
                    } else if d <= 30 {
                        "dueSoon"
                    } else {
                        "upToDate"
                    };
                    (e, Some(fechas::clave(p)), Some(d))
                }
            };
            Ok(Chequeo {
                id: f.get(0)?,
                clave: f.get(1)?,
                nombre: f.get(2)?,
                meses,
                activo: f.get(4)?,
                ultimo: ultimo.map(|u| fechas::clave(u.date())),
                proximo,
                estado: estado.into(),
                dias,
            })
        },
    )?;
    let orden = |e: &str| match e {
        "due" => 0,
        "neverDone" => 1,
        "dueSoon" => 2,
        _ => 3,
    };
    v.sort_by_key(|c| (!c.activo, orden(&c.estado), c.dias.unwrap_or(0)));
    Ok(v)
}

/// Marca un chequeo como hecho en un día (no en el futuro).
///
/// # Errors
///
/// [`ScError::InvalidInput`] si el día no se entiende.
pub fn chequeo_hecho(db: &BaseDeDatos, id: i64, dia: &str) -> ScResult<Option<Recompensa>> {
    let d = fechas::de_clave(dia)
        .ok_or(ScError::InvalidInput)?
        .min(fechas::hoy());
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    tx.execute(
        "INSERT INTO checkup_logs (checkup_id, done_at) VALUES (?1, ?2)",
        params![
            id,
            fechas::texto(fechas::inicio(d) + chrono::Duration::hours(12))
        ],
    )
    .map_err(|_| ScError::NotFound)?;
    let r = nibi::premiar(&tx, Accion::Chequeo)?;
    tx.commit().map_err(fallo)?;
    Ok(r)
}

/// Activa, desactiva o cambia el intervalo de un chequeo; o añade uno propio.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si los valores no valen.
pub fn ajustar_chequeo(
    db: &BaseDeDatos,
    id: Option<i64>,
    nombre: Option<&str>,
    meses: i64,
    activo: bool,
) -> ScResult<()> {
    if !(1..=240).contains(&meses) {
        return Err(ScError::InvalidInput);
    }
    let conn = db.conexion()?;
    match id {
        Some(id) => {
            conn.execute(
                "UPDATE checkups SET interval_months = ?1, is_enabled = ?2 WHERE id = ?3",
                params![meses, activo, id],
            )
            .map_err(fallo)?;
        }
        None => {
            let n = texto_limpio(nombre, 60).ok_or(ScError::InvalidInput)?;
            conn.execute(
                "INSERT INTO checkups (catalog_key, custom_name, interval_months, is_enabled) VALUES ('custom', ?1, ?2, 1)",
                params![n, meses],
            )
            .map_err(fallo)?;
        }
    }
    Ok(())
}

// --- Alcohol y tabaco -------------------------------------------------------

/// Las bebidas y el tabaco: clave, tipo, ml, % de alcohol, kcal y carbohidratos
/// por 100 ml.
const SUSTANCIAS: &[(&str, &str, f64, f64, f64, f64)] = &[
    ("beer", "alcohol", 330.0, 4.5, 140.0, 3.6),
    ("wine", "alcohol", 150.0, 12.0, 125.0, 2.6),
    ("spirit", "alcohol", 30.0, 35.0, 70.0, 0.0),
    ("cocktail", "alcohol", 200.0, 8.0, 220.0, 12.0),
    ("cigarette", "tobacco", 0.0, 0.0, 0.0, 0.0),
    ("vape", "tobacco", 0.0, 0.0, 0.0, 0.0),
];

fn nombre_bebida(clave: &str) -> &'static str {
    match clave {
        "beer" => t("Cerveza (330 ml)", "Beer (330 ml)", "Cerveja (330 ml)"),
        "wine" => t(
            "Vino (copa de 150 ml)",
            "Wine (150 ml glass)",
            "Vinho (taça de 150 ml)",
        ),
        "spirit" => t(
            "Aguardiente o ron (trago de 30 ml)",
            "Spirits (30 ml shot)",
            "Destilado (dose de 30 ml)",
        ),
        _ => t("Cóctel (200 ml)", "Cocktail (200 ml)", "Coquetel (200 ml)"),
    }
}

/// Registra bebidas o tabaco. Las bebidas también entran en Comida (como
/// merienda), porque su energía cuenta en el balance y en el gasto adaptativo.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si la clave no existe o las unidades no son 1–40.
pub fn consumir(db: &BaseDeDatos, clave: &str, unidades: i64) -> ScResult<i64> {
    let (clave, tipo, ml, abv, kcal, carbos) = SUSTANCIAS
        .iter()
        .copied()
        .find(|s| s.0 == clave)
        .ok_or(ScError::InvalidInput)?;
    if !(1..=40).contains(&unidades) {
        return Err(ScError::InvalidInput);
    }
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let ahora = fechas::ahora();
    let dia = fechas::clave(ahora.date());
    let registro = if tipo == "alcohol" {
        let codigo = format!("nb-drink-{clave}");
        let alimento: Option<i64> = tx
            .query_row(
                "SELECT id FROM food_items WHERE barcode = ?1",
                [&codigo],
                |f| f.get(0),
            )
            .optional()
            .map_err(fallo)?;
        let alimento = match alimento {
            Some(a) => a,
            None => {
                tx.execute(
                    "INSERT INTO food_items (barcode, name, source, default_serving_grams, serving_label,
                        kcal_per100g, protein_per100g, carbs_per100g, fat_per100g, nova_group)
                     VALUES (?1, ?2, 'user_custom', ?3, ?4, ?5, 0, ?6, 0, 4)",
                    params![codigo, nombre_bebida(clave), ml, format!("{ml} ml"), kcal / ml * 100.0, carbos],
                )
                .map_err(fallo)?;
                tx.last_insert_rowid()
            }
        };
        let gramos = ml * unidades as f64;
        let r = gramos / 100.0;
        tx.execute(
            "INSERT INTO food_logs (date_key, logged_at, meal_type, food_item_id, grams_consumed,
                portion_multiplier, total_kcal, total_protein_g, total_carbs_g, total_fat_g,
                total_fiber_g, total_sodium_mg, total_potassium_mg, total_magnesium_mg)
             VALUES (?1, ?2, 'snack', ?3, ?4, ?5, ?6, 0, ?7, 0, 0, 0, 0, 0)",
            params![
                dia,
                fechas::texto(ahora),
                alimento,
                gramos,
                unidades as f64,
                crate::formulas::fijo(kcal / ml * 100.0 * r, 1),
                crate::formulas::fijo(carbos * r, 1)
            ],
        )
        .map_err(fallo)?;
        Some(tx.last_insert_rowid())
    } else {
        None
    };
    let tragos = ml * abv / 100.0 * 0.789 / 10.0;
    tx.execute(
        "INSERT INTO substance_logs (date_key, logged_at, kind, preset_key, standard_drinks, kcal, units, food_log_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![dia, fechas::texto(ahora), tipo, clave, tragos * unidades as f64, kcal * unidades as f64, unidades, registro],
    )
    .map_err(fallo)?;
    let id = tx.last_insert_rowid();
    tx.commit().map_err(fallo)?;
    Ok(id)
}

/// Borra un registro de bebida o tabaco (y su comida, si la tenía).
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn borrar_consumo(db: &BaseDeDatos, id: i64) -> ScResult<()> {
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let comida: Option<Option<i64>> = tx
        .query_row(
            "SELECT food_log_id FROM substance_logs WHERE id = ?1",
            [id],
            |f| f.get(0),
        )
        .optional()
        .map_err(fallo)?;
    let Some(comida) = comida else {
        return Err(ScError::NotFound);
    };
    if let Some(c) = comida {
        tx.execute("DELETE FROM food_logs WHERE id = ?1", [c])
            .map_err(fallo)?;
    }
    tx.execute("DELETE FROM substance_logs WHERE id = ?1", [id])
        .map_err(fallo)?;
    tx.commit().map_err(fallo)
}

/// El resumen de alcohol y tabaco de esta semana y la anterior.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Consumo {
    pub tragos_semana: f64,
    pub tragos_semana_pasada: f64,
    pub tragos_hoy: f64,
    pub tabaco_semana: i64,
    pub tabaco_semana_pasada: i64,
    pub tabaco_hoy: i64,
    pub ultimos: Vec<UnConsumo>,
}

/// Un registro de bebida o tabaco.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct UnConsumo {
    pub id: i64,
    pub clave: String,
    pub unidades: i64,
    pub dia: String,
    pub hora: String,
}

/// El consumo de esta semana y la anterior.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn consumo(conn: &Connection) -> ScResult<Consumo> {
    let hoy = fechas::hoy();
    let lunes = fechas::mas_dias(hoy, -i64::from(hoy.weekday().num_days_from_monday()));
    let pasado = fechas::mas_dias(lunes, -7);
    let suma = |tipo: &str, desde: NaiveDate, hasta: NaiveDate| -> ScResult<(f64, i64)> {
        conn.query_row(
            "SELECT coalesce(sum(standard_drinks), 0), coalesce(sum(units), 0) FROM substance_logs
             WHERE kind = ?1 AND date_key >= ?2 AND date_key < ?3",
            params![tipo, fechas::clave(desde), fechas::clave(hasta)],
            |f| Ok((f.get(0)?, f.get(1)?)),
        )
        .map_err(fallo)
    };
    let manana = fechas::mas_dias(hoy, 1);
    Ok(Consumo {
        tragos_semana: suma("alcohol", lunes, manana)?.0,
        tragos_semana_pasada: suma("alcohol", pasado, lunes)?.0,
        tragos_hoy: suma("alcohol", hoy, manana)?.0,
        tabaco_semana: suma("tobacco", lunes, manana)?.1,
        tabaco_semana_pasada: suma("tobacco", pasado, lunes)?.1,
        tabaco_hoy: suma("tobacco", hoy, manana)?.1,
        ultimos: listar(
            conn,
            "SELECT id, preset_key, units, logged_at FROM substance_logs ORDER BY logged_at DESC LIMIT 30",
            [],
            |f| {
                let cuando = fechas::leer(&f.get::<_, String>(3)?).unwrap_or_default();
                Ok(UnConsumo {
                    id: f.get(0)?,
                    clave: f.get(1)?,
                    unidades: f.get(2)?,
                    dia: fechas::clave(cuando.date()),
                    hora: cuando.format("%H:%M").to_string(),
                })
            },
        )?,
    })
}

// --- Libre -----------------------------------------------------------------

/// Los hitos de una racha de Libre, en días.
pub const HITOS_LIBRE: [i64; 10] = [1, 3, 7, 14, 30, 66, 90, 180, 365, 730];

/// Los hitos alcanzados por una racha en segundos.
#[must_use]
pub fn hitos_alcanzados(segundos: i64) -> i64 {
    HITOS_LIBRE
        .iter()
        .filter(|d| segundos >= *d * 86_400)
        .count() as i64
}

/// Un contador de Libre.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Contador {
    pub id: i64,
    pub clave: String,
    pub nombre: Option<String>,
    /// Inicio de la racha actual, ISO local, para contar en vivo.
    pub inicio: String,
    pub primer_inicio: String,
    pub racha_maxima: i64,
    pub acumulado: i64,
    pub coste_diario: Option<f64>,
    pub unidades_diarias: Option<f64>,
    pub minutos_diarios: Option<i64>,
    pub motivo: Option<String>,
    pub hitos_celebrados: i64,
    pub ganas_superadas: i64,
    pub reinicios: i64,
}

/// Los contadores activos.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn contadores(conn: &Connection) -> ScResult<Vec<Contador>> {
    listar(
        conn,
        "SELECT q.*, (SELECT count(*) FROM quit_events e WHERE e.tracker_id = q.id AND e.kind = 'urge') AS ganas,
                (SELECT count(*) FROM quit_events e WHERE e.tracker_id = q.id AND e.kind = 'reset') AS reinicios
         FROM quit_trackers q WHERE q.is_archived = 0 ORDER BY q.created_at",
        [],
        |f| {
            let iso = |c: &str| -> rusqlite::Result<String> {
                Ok(fechas::leer(&f.get::<_, String>(c)?)
                    .unwrap_or_default()
                    .format("%Y-%m-%dT%H:%M:%S")
                    .to_string())
            };
            Ok(Contador {
                id: f.get("id")?,
                clave: f.get("preset_key")?,
                nombre: f.get("custom_name")?,
                inicio: iso("started_at")?,
                primer_inicio: iso("first_started_at")?,
                racha_maxima: f.get("longest_streak_seconds")?,
                acumulado: f.get("banked_seconds")?,
                coste_diario: f.get("daily_cost")?,
                unidades_diarias: f.get("daily_units")?,
                minutos_diarios: f.get("daily_minutes")?,
                motivo: f.get("motivation")?,
                hitos_celebrados: f.get("celebrated_milestones")?,
                ganas_superadas: f.get("ganas")?,
                reinicios: f.get("reinicios")?,
            })
        },
    )
}

/// Un contador nuevo.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct NuevoContador {
    pub clave: String,
    pub nombre: Option<String>,
    /// Hace cuántos días empezó (0 = ahora).
    pub hace_dias: i64,
    pub coste_diario: Option<f64>,
    pub unidades_diarias: Option<f64>,
    pub minutos_diarios: Option<i64>,
    pub motivo: Option<String>,
}

const CLAVES_LIBRE: [&str; 14] = [
    "tobacco",
    "vape",
    "alcohol",
    "sugar",
    "soda",
    "junk_food",
    "caffeine",
    "social_media",
    "gaming",
    "gambling",
    "cannabis",
    "adult_content",
    "nail_biting",
    "custom",
];

/// Crea un contador. Quien empieza con días ya ganados no recibe de golpe las
/// celebraciones de lo anterior.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si la clave no existe o uno propio no tiene nombre.
pub fn crear_contador(db: &BaseDeDatos, n: &NuevoContador) -> ScResult<i64> {
    if !CLAVES_LIBRE.contains(&n.clave.as_str()) || !(0..=3650).contains(&n.hace_dias) {
        return Err(ScError::InvalidInput);
    }
    let nombre = texto_limpio(n.nombre.as_deref(), 40);
    if n.clave == "custom" && nombre.is_none() {
        return Err(ScError::InvalidInput);
    }
    let positivo = |v: Option<f64>| v.filter(|x| x.is_finite() && *x > 0.0);
    let ahora = fechas::ahora();
    let inicio = ahora - chrono::Duration::days(n.hace_dias);
    let conn = db.conexion()?;
    conn.execute(
        "INSERT INTO quit_trackers (preset_key, custom_name, started_at, first_started_at, daily_cost,
            daily_units, daily_minutes, motivation, celebrated_milestones, created_at)
         VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            n.clave,
            if n.clave == "custom" { nombre } else { None },
            fechas::texto(inicio),
            positivo(n.coste_diario),
            positivo(n.unidades_diarias),
            n.minutos_diarios.filter(|m| *m > 0),
            texto_limpio(n.motivo.as_deref(), 200),
            hitos_alcanzados((ahora - inicio).num_seconds()),
            fechas::texto(ahora)
        ],
    )
    .map_err(fallo)?;
    Ok(conn.last_insert_rowid())
}

/// Unas ganas que pasaron sin ceder: se guardan y dan monedas una vez al día.
///
/// # Errors
///
/// [`ScError::NotFound`] si el contador no existe.
pub fn ganas_superadas(db: &BaseDeDatos, id: i64) -> ScResult<Option<Recompensa>> {
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    tx.execute(
        "INSERT INTO quit_events (tracker_id, kind, occurred_at) VALUES (?1, 'urge', ?2)",
        params![id, fechas::texto(fechas::ahora())],
    )
    .map_err(|_| ScError::NotFound)?;
    let r = nibi::premiar(&tx, Accion::GanasSuperadas)?;
    tx.commit().map_err(fallo)?;
    Ok(r)
}

/// Reinicia la racha. Lo ganado queda en el tiempo total y en la mejor racha.
///
/// # Errors
///
/// [`ScError::NotFound`] si el contador no existe.
pub fn reiniciar_contador(db: &BaseDeDatos, id: i64) -> ScResult<()> {
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let (inicio, acumulado, maxima): (String, i64, i64) = tx
        .query_row(
            "SELECT started_at, banked_seconds, longest_streak_seconds FROM quit_trackers WHERE id = ?1",
            [id],
            |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)),
        )
        .optional()
        .map_err(fallo)?
        .ok_or(ScError::NotFound)?;
    let ahora = fechas::ahora();
    let racha = fechas::leer(&inicio).map_or(0, |i| (ahora - i).num_seconds().max(0));
    tx.execute(
        "INSERT INTO quit_events (tracker_id, kind, occurred_at, streak_seconds) VALUES (?1, 'reset', ?2, ?3)",
        params![id, fechas::texto(ahora), racha],
    )
    .map_err(fallo)?;
    tx.execute(
        "UPDATE quit_trackers SET started_at = ?1, banked_seconds = ?2, longest_streak_seconds = ?3,
            celebrated_milestones = 0 WHERE id = ?4",
        params![fechas::texto(ahora), acumulado + racha, maxima.max(racha), id],
    )
    .map_err(fallo)?;
    tx.commit().map_err(fallo)
}

/// Si la racha alcanzó un hito sin celebrar, lo marca y da monedas: devuelve
/// los días del hito.
///
/// # Errors
///
/// [`ScError::NotFound`] si el contador no existe.
pub fn celebrar_si_toca(db: &BaseDeDatos, id: i64) -> ScResult<Option<i64>> {
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let (inicio, celebrados): (String, i64) = tx
        .query_row(
            "SELECT started_at, celebrated_milestones FROM quit_trackers WHERE id = ?1",
            [id],
            |f| Ok((f.get(0)?, f.get(1)?)),
        )
        .optional()
        .map_err(fallo)?
        .ok_or(ScError::NotFound)?;
    let alcanzados =
        fechas::leer(&inicio).map_or(0, |i| hitos_alcanzados((fechas::ahora() - i).num_seconds()));
    if alcanzados <= celebrados {
        return Ok(None);
    }
    tx.execute(
        "UPDATE quit_trackers SET celebrated_milestones = ?1 WHERE id = ?2",
        params![alcanzados, id],
    )
    .map_err(fallo)?;
    nibi::premiar(&tx, Accion::HitoLibre)?;
    tx.commit().map_err(fallo)?;
    Ok(Some(HITOS_LIBRE[(alcanzados - 1) as usize]))
}

/// Archiva un contador o lo borra del todo.
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn quitar_contador(db: &BaseDeDatos, id: i64, borrar: bool) -> ScResult<()> {
    let conn = db.conexion()?;
    let n = if borrar {
        conn.execute("DELETE FROM quit_trackers WHERE id = ?1", [id])
    } else {
        conn.execute(
            "UPDATE quit_trackers SET is_archived = 1 WHERE id = ?1",
            [id],
        )
    }
    .map_err(fallo)?;
    if n == 0 {
        return Err(ScError::NotFound);
    }
    Ok(())
}

// --- Bienestar OMS-5 ---------------------------------------------------------

/// Guarda las cinco respuestas (0–5) del OMS-5 y devuelve el porcentaje.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si no son cinco respuestas de 0 a 5.
pub fn bienestar(db: &BaseDeDatos, respuestas: &[i64]) -> ScResult<i64> {
    if respuestas.len() != 5 || respuestas.iter().any(|r| !(0..=5).contains(r)) {
        return Err(ScError::InvalidInput);
    }
    let pct = respuestas.iter().sum::<i64>() * 4;
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let ahora = fechas::ahora();
    tx.execute(
        "INSERT INTO wellbeing_checks (date_key, created_at, answers_json, percent_score) VALUES (?1, ?2, ?3, ?4)",
        params![fechas::clave(ahora.date()), fechas::texto(ahora), serde_json::to_string(respuestas).unwrap_or_default(), pct],
    )
    .map_err(fallo)?;
    nibi::premiar(&tx, Accion::RevisionSemanal)?;
    tx.commit().map_err(fallo)?;
    Ok(pct)
}

/// Los últimos índices OMS-5: `(día, porcentaje)`.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn indices_bienestar(conn: &Connection) -> ScResult<Vec<(String, i64)>> {
    listar(
        conn,
        "SELECT date_key, percent_score FROM wellbeing_checks ORDER BY created_at DESC LIMIT 12",
        [],
        |f| Ok((f.get(0)?, f.get(1)?)),
    )
}

// --- Resumen ---------------------------------------------------------------

/// Todo lo de Cuidado.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct ResumenCuidado {
    pub modulos: Vec<String>,
    pub aparatos: Vec<String>,
    pub tomas: Vec<Toma>,
    pub tratamientos: Vec<Tratamiento>,
    pub habitos: Vec<Habito>,
    pub sesiones: SesionesDeHoy,
    pub lecturas: Vec<Lectura>,
    pub sintomas: Vec<Sintoma>,
    pub catalogo_sintomas: Vec<String>,
    pub chequeos: Vec<Chequeo>,
    pub consumo: Consumo,
    pub contadores: Vec<Contador>,
    pub bienestar: Vec<(String, i64)>,
}

/// Todo lo de Cuidado, con los chequeos sembrados la primera vez.
///
/// # Errors
///
/// [`ScError::NotFound`] sin perfil.
pub fn resumen(db: &BaseDeDatos) -> ScResult<ResumenCuidado> {
    {
        let mut conn = db.conexion()?;
        let tx = db::transaccion_de_escritura(&mut conn)?;
        sembrar_chequeos(&tx)?;
        tx.commit().map_err(fallo)?;
    }
    let mut conn = db.conexion()?;
    let tx = db::lectura_coherente(&mut conn)?;
    let p = perfil::exigir(&tx)?.perfil;
    Ok(ResumenCuidado {
        catalogo_sintomas: catalogo_sintomas(p.sexo, p.edad, p.ciclo_activo)
            .into_iter()
            .map(str::to_owned)
            .collect(),
        modulos: p.modulos,
        aparatos: p.aparatos,
        tomas: tomas_del_dia(&tx, fechas::hoy())?,
        tratamientos: tratamientos(&tx)?,
        habitos: habitos(&tx)?,
        sesiones: sesiones_de_hoy(&tx)?,
        lecturas: lecturas(&tx)?,
        sintomas: sintomas(&tx)?,
        chequeos: chequeos(&tx)?,
        consumo: consumo(&tx)?,
        contadores: contadores(&tx)?,
        bienestar: indices_bienestar(&tx)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perfil::pruebas::base_con_perfil;

    fn borrador(nombre: &str, horas: Vec<i64>) -> BorradorTratamiento {
        BorradorTratamiento {
            nombre: nombre.into(),
            dosis: "1 comprimido".into(),
            tipo: "medication".into(),
            horas,
            dias: 127,
            reserva: Some(10.0),
            unidades_por_toma: 1.0,
            avisos: true,
            baja_glucosa: false,
            retiene_agua: false,
            notas: None,
        }
    }

    #[test]
    fn una_toma_baja_la_reserva_y_no_da_monedas() {
        let db = base_con_perfil();
        let id = guardar_tratamiento(&db, None, &borrador("Vitamina", vec![480, 1200])).unwrap();
        let monedas_antes: i64 = db
            .conexion()
            .unwrap()
            .query_row("SELECT coins_balance FROM pet_status", [], |f| f.get(0))
            .unwrap();
        marcar_toma(&db, id, Some(480), Some(true)).unwrap();
        let conn = db.conexion().unwrap();
        let tomas = tomas_del_dia(&conn, fechas::hoy()).unwrap();
        assert_eq!(tomas.len(), 2);
        assert_eq!(tomas[0].estado, "taken");
        let t = &tratamientos(&conn).unwrap()[0];
        assert_eq!(t.reserva, Some(9.0));
        let monedas: i64 = conn
            .query_row("SELECT coins_balance FROM pet_status", [], |f| f.get(0))
            .unwrap();
        assert_eq!(monedas, monedas_antes, "un tratamiento no es un juego");
        drop(conn);
        marcar_toma(&db, id, Some(480), None).unwrap();
        assert_eq!(
            tratamientos(&db.conexion().unwrap()).unwrap()[0].reserva,
            Some(10.0)
        );
    }

    #[test]
    fn la_reserva_se_cuenta_en_dias() {
        assert_eq!(dias_de_reserva(Some(14.0), 1.0, 2, 127), Some(7));
        assert_eq!(dias_de_reserva(None, 1.0, 2, 127), None);
    }

    #[test]
    fn todos_los_habitos_del_dia_dan_monedas() {
        let db = base_con_perfil();
        let a = crear_habito(&db, "Leer", "book", 127).unwrap();
        let b = crear_habito(&db, "Caminar", "walk", 127).unwrap();
        assert_eq!(marcar_habito(&db, a, true).unwrap(), None);
        assert!(marcar_habito(&db, b, true).unwrap().is_some());
        let h = habitos(&db.conexion().unwrap()).unwrap();
        assert!(h.iter().all(|x| x.hecho_hoy && x.racha == 1));
    }

    #[test]
    fn las_bandas_de_la_presion_siguen_la_guia() {
        assert_eq!(banda("blood_pressure", 118.0, Some(76.0), None), "normal");
        assert_eq!(banda("blood_pressure", 125.0, Some(76.0), None), "elevated");
        assert_eq!(banda("blood_pressure", 135.0, Some(85.0), None), "high");
        assert_eq!(
            banda("blood_pressure", 185.0, Some(95.0), None),
            "consultSoon"
        );
        assert_eq!(banda("glucose", 110.0, None, Some("fasting")), "elevated");
        assert_eq!(banda("glucose", 110.0, None, Some("after_meal")), "normal");
    }

    #[test]
    fn una_lectura_imposible_no_se_guarda() {
        let db = base_con_perfil();
        assert_eq!(
            medir(&db, "blood_pressure", 80.0, Some(120.0), None),
            Err(ScError::InvalidInput)
        );
        assert_eq!(medir(&db, "oxygen", 97.0, None, None).unwrap(), "normal");
    }

    #[test]
    fn los_chequeos_se_siembran_segun_sexo_y_edad() {
        let db = base_con_perfil();
        let r = resumen(&db).unwrap();
        assert!(
            r.chequeos
                .iter()
                .any(|c| c.clave == "prostate" && !c.activo),
            "36 años: aún no"
        );
        assert!(!r.chequeos.iter().any(|c| c.clave == "mammography"));
        assert!(r
            .chequeos
            .iter()
            .any(|c| c.clave == "dental" && c.activo && c.estado == "neverDone"));
    }

    #[test]
    fn una_cerveza_entra_tambien_en_comida() {
        let db = base_con_perfil();
        consumir(&db, "beer", 2).unwrap();
        let conn = db.conexion().unwrap();
        let kcal: f64 = conn
            .query_row("SELECT sum(total_kcal) FROM food_logs", [], |f| f.get(0))
            .unwrap();
        assert!((kcal - 280.0).abs() < 0.5, "{kcal}");
        let c = consumo(&conn).unwrap();
        assert!((c.tragos_hoy - 2.0 * 330.0 * 4.5 / 100.0 * 0.789 / 10.0).abs() < 0.01);
        let id = c.ultimos[0].id;
        drop(conn);
        borrar_consumo(&db, id).unwrap();
        let n: i64 = db
            .conexion()
            .unwrap()
            .query_row("SELECT count(*) FROM food_logs", [], |f| f.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn reiniciar_libre_conserva_lo_ganado() {
        let db = base_con_perfil();
        let id = crear_contador(
            &db,
            &NuevoContador {
                clave: "tobacco".into(),
                nombre: None,
                hace_dias: 10,
                coste_diario: Some(5.0),
                unidades_diarias: None,
                minutos_diarios: None,
                motivo: Some("Respirar mejor".into()),
            },
        )
        .unwrap();
        let c = &contadores(&db.conexion().unwrap()).unwrap()[0];
        assert_eq!(
            c.hitos_celebrados, 3,
            "1, 3 y 7 días no se celebran de golpe"
        );
        reiniciar_contador(&db, id).unwrap();
        let c = &contadores(&db.conexion().unwrap()).unwrap()[0];
        assert_eq!(c.acumulado, 10 * 86_400);
        assert_eq!(c.racha_maxima, 10 * 86_400);
        assert_eq!(c.reinicios, 1);
        assert_eq!(celebrar_si_toca(&db, id).unwrap(), None);
    }

    #[test]
    fn el_oms5_va_de_0_a_100() {
        let db = base_con_perfil();
        assert_eq!(bienestar(&db, &[5, 5, 5, 5, 5]).unwrap(), 100);
        assert_eq!(bienestar(&db, &[5, 5]), Err(ScError::InvalidInput));
    }

    #[test]
    fn una_sesion_corta_no_cuenta_y_una_pausa_premia() {
        let db = base_con_perfil();
        assert_eq!(
            completar_sesion(&db, "active_break", "back", 10).unwrap(),
            None
        );
        assert!(completar_sesion(&db, "active_break", "back", 120)
            .unwrap()
            .is_some());
        assert!(
            completar_sesion(&db, "active_break", "eyes", 60)
                .unwrap()
                .is_none(),
            "una vez al día"
        );
        let s = sesiones_de_hoy(&db.conexion().unwrap()).unwrap();
        assert_eq!((s.pausas, s.vista), (1, 1));
    }
}
