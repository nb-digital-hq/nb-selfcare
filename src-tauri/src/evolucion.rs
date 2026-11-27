//! Evolución: la mirada larga que el teléfono no ofrece. Días agregados, semana frente
//! a la anterior, relaciones entre hábitos y un historial unificado para corregir.
//!
//! Las relaciones son descriptivas (coeficiente de Pearson con muestra mínima): nunca
//! se presentan como causa, diagnóstico ni prescripción.
use crate::db::{self, fallo, BaseDeDatos};
use crate::error::{ScError, ScResult};
use crate::fechas;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// Días máximos de una consulta: medio año cabe holgado en una vista.
pub const MAX_DIAS: u32 = 366;
/// Muestra mínima para mostrar una relación.
const MIN_MUESTRA: usize = 10;
/// Por debajo de este valor absoluto no se menciona la relación.
const UMBRAL: f64 = 0.3;

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct DiaEvolucion {
    pub dia: String,
    pub kcal: Option<f64>,
    pub proteina_g: Option<f64>,
    pub agua_ml: Option<f64>,
    pub peso_kg: Option<f64>,
    pub sueno_h: Option<f64>,
    pub pasos: Option<f64>,
    pub animo: Option<f64>,
    pub energia: Option<f64>,
    pub estres: Option<f64>,
    pub entreno_min: Option<f64>,
    pub habitos: Option<f64>,
    pub cafeina_mg: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub enum Medida {
    Kcal,
    ProteinaG,
    AguaMl,
    PesoKg,
    SuenoH,
    Pasos,
    Animo,
    Energia,
    Estres,
    EntrenoMin,
    Habitos,
    CafeinaMg,
}

impl Medida {
    const TODAS: [Medida; 12] = [
        Medida::Kcal,
        Medida::ProteinaG,
        Medida::AguaMl,
        Medida::PesoKg,
        Medida::SuenoH,
        Medida::Pasos,
        Medida::Animo,
        Medida::Energia,
        Medida::Estres,
        Medida::EntrenoMin,
        Medida::Habitos,
        Medida::CafeinaMg,
    ];
    fn de(self, d: &DiaEvolucion) -> Option<f64> {
        match self {
            Self::Kcal => d.kcal,
            Self::ProteinaG => d.proteina_g,
            Self::AguaMl => d.agua_ml,
            Self::PesoKg => d.peso_kg,
            Self::SuenoH => d.sueno_h,
            Self::Pasos => d.pasos,
            Self::Animo => d.animo,
            Self::Energia => d.energia,
            Self::Estres => d.estres,
            Self::EntrenoMin => d.entreno_min,
            Self::Habitos => d.habitos,
            Self::CafeinaMg => d.cafeina_mg,
        }
    }
}

/// Media de una medida en una semana y en la anterior.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Comparacion {
    pub medida: Medida,
    pub esta: Option<f64>,
    pub anterior: Option<f64>,
    pub dias_con_dato: u32,
}

/// Dos medidas que tienden a moverse juntas (o en sentido contrario) en tus días.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Relacion {
    /// La medida del día anterior o del mismo día.
    pub causa_posible: Medida,
    pub efecto: Medida,
    /// Si `causa_posible` se mide el día anterior (p. ej. el sueño de la noche previa).
    pub dia_anterior: bool,
    pub coeficiente: f64,
    pub muestra: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Evolucion {
    pub dias: Vec<DiaEvolucion>,
    pub semana: Vec<Comparacion>,
    pub relaciones: Vec<Relacion>,
    /// Días de los últimos siete con algún registro.
    pub dias_activos: u32,
}

fn sumar(
    conn: &Connection,
    sql: &str,
    desde: &str,
    mapa: &mut BTreeMap<String, DiaEvolucion>,
    poner: impl Fn(&mut DiaEvolucion, f64),
) -> ScResult<()> {
    let mut st = conn.prepare(sql).map_err(fallo)?;
    let filas = st
        .query_map([desde], |f| {
            Ok((f.get::<_, String>(0)?, f.get::<_, Option<f64>>(1)?))
        })
        .map_err(fallo)?;
    for fila in filas {
        let (dia, valor) = fila.map_err(fallo)?;
        if let (Some(d), Some(v)) = (mapa.get_mut(&dia), valor) {
            poner(d, v);
        }
    }
    Ok(())
}

/// Días agregados desde `dias - 1` días atrás hasta hoy, en orden.
///
/// # Errors
/// [`ScError::InvalidInput`] fuera de 7..=366; errores de almacenamiento canónicos.
pub fn dias(conn: &Connection, dias: u32) -> ScResult<Vec<DiaEvolucion>> {
    if !(7..=MAX_DIAS).contains(&dias) {
        return Err(ScError::InvalidInput);
    }
    let hoy = fechas::hoy();
    let primero = fechas::mas_dias(hoy, -(i64::from(dias) - 1));
    let desde = fechas::clave(primero);
    let mut mapa = BTreeMap::new();
    for i in 0..i64::from(dias) {
        let dia = fechas::clave(fechas::mas_dias(primero, i));
        mapa.insert(
            dia.clone(),
            DiaEvolucion {
                dia,
                ..Default::default()
            },
        );
    }
    sumar(
        conn,
        "SELECT date_key, sum(total_kcal) FROM food_logs WHERE date_key >= ?1 GROUP BY date_key",
        &desde,
        &mut mapa,
        |d, v| d.kcal = Some(v),
    )?;
    sumar(
        conn,
        "SELECT date_key, sum(total_protein_g) FROM food_logs WHERE date_key >= ?1 GROUP BY date_key",
        &desde,
        &mut mapa,
        |d, v| d.proteina_g = Some(v),
    )?;
    sumar(
        conn,
        "SELECT date_key, sum(volume_ml) FROM hydration_logs WHERE date_key >= ?1 GROUP BY date_key",
        &desde,
        &mut mapa,
        |d, v| d.agua_ml = Some(v),
    )?;
    sumar(
        conn,
        "SELECT date_key, weight_kg FROM body_measurements b WHERE date_key >= ?1
         AND measured_at = (SELECT max(measured_at) FROM body_measurements c WHERE c.date_key = b.date_key)",
        &desde,
        &mut mapa,
        |d, v| d.peso_kg = Some(v),
    )?;
    sumar(
        conn,
        "SELECT date_key, sleep_duration_minutes / 60.0 FROM daily_summaries
         WHERE date_key >= ?1 AND sleep_duration_minutes > 0",
        &desde,
        &mut mapa,
        |d, v| d.sueno_h = Some(v),
    )?;
    sumar(
        conn,
        "SELECT date_key, steps_count FROM daily_summaries WHERE date_key >= ?1 AND steps_count > 0",
        &desde,
        &mut mapa,
        |d, v| d.pasos = Some(v),
    )?;
    for (col, poner) in [
        (
            "mood_score",
            (|d: &mut DiaEvolucion, v| d.animo = Some(v)) as fn(&mut DiaEvolucion, f64),
        ),
        ("energy_level", |d, v| d.energia = Some(v)),
        ("stress_level", |d, v| d.estres = Some(v)),
    ] {
        sumar(
            conn,
            &format!(
                "SELECT date_key, avg({col}) FROM journal_entries
                 WHERE date_key >= ?1 AND {col} IS NOT NULL GROUP BY date_key"
            ),
            &desde,
            &mut mapa,
            poner,
        )?;
    }
    sumar(
        conn,
        "SELECT date_key, sum(duration_minutes) FROM workout_blocks WHERE date_key >= ?1 GROUP BY date_key",
        &desde,
        &mut mapa,
        |d, v| d.entreno_min = Some(v),
    )?;
    sumar(
        conn,
        "SELECT date_key, count(*) FROM habit_logs WHERE date_key >= ?1 GROUP BY date_key",
        &desde,
        &mut mapa,
        |d, v| d.habitos = Some(v),
    )?;
    sumar(
        conn,
        "SELECT date_key, sum(caffeine_mg) FROM caffeine_logs WHERE date_key >= ?1 GROUP BY date_key",
        &desde,
        &mut mapa,
        |d, v| d.cafeina_mg = Some(v),
    )?;
    Ok(mapa.into_values().collect())
}

fn media(valores: impl Iterator<Item = f64>) -> (Option<f64>, u32) {
    let (suma, n) = valores.fold((0.0, 0u32), |(s, n), v| (s + v, n + 1));
    ((n > 0).then(|| suma / f64::from(n)), n)
}

/// Compara la media de los últimos siete días con la de los siete anteriores.
#[must_use]
pub fn semana(dias: &[DiaEvolucion]) -> Vec<Comparacion> {
    let n = dias.len();
    if n < 14 {
        return Vec::new();
    }
    let esta = &dias[n - 7..];
    let anterior = &dias[n - 14..n - 7];
    Medida::TODAS
        .into_iter()
        .filter_map(|m| {
            let (e, dias_con_dato) = media(esta.iter().filter_map(|d| m.de(d)));
            let (a, _) = media(anterior.iter().filter_map(|d| m.de(d)));
            (e.is_some() || a.is_some()).then_some(Comparacion {
                medida: m,
                esta: e,
                anterior: a,
                dias_con_dato,
            })
        })
        .collect()
}

/// Coeficiente de Pearson de pares completos; `None` sin variación o con poca muestra.
#[must_use]
pub fn pearson(pares: &[(f64, f64)]) -> Option<f64> {
    if pares.len() < MIN_MUESTRA {
        return None;
    }
    let n = pares.len() as f64;
    let (mx, my) = pares
        .iter()
        .fold((0.0, 0.0), |(a, b), (x, y)| (a + x / n, b + y / n));
    let (mut cov, mut vx, mut vy) = (0.0, 0.0, 0.0);
    for (x, y) in pares {
        cov += (x - mx) * (y - my);
        vx += (x - mx).powi(2);
        vy += (y - my).powi(2);
    }
    if vx <= f64::EPSILON || vy <= f64::EPSILON {
        return None;
    }
    Some((cov / (vx.sqrt() * vy.sqrt())).clamp(-1.0, 1.0))
}

/// Relaciones con sentido para el autocuidado. El sueño y la cafeína se cruzan con el
/// día siguiente; el resto, con el mismo día.
const CRUCES: [(Medida, Medida, bool); 9] = [
    (Medida::SuenoH, Medida::Animo, false),
    (Medida::SuenoH, Medida::Energia, false),
    (Medida::SuenoH, Medida::Kcal, false),
    (Medida::CafeinaMg, Medida::SuenoH, true),
    (Medida::EntrenoMin, Medida::Animo, false),
    (Medida::EntrenoMin, Medida::SuenoH, true),
    (Medida::AguaMl, Medida::Energia, false),
    (Medida::Estres, Medida::Kcal, false),
    (Medida::Pasos, Medida::Animo, false),
];

#[must_use]
pub fn relaciones(dias: &[DiaEvolucion]) -> Vec<Relacion> {
    let mut salida: Vec<Relacion> = CRUCES
        .into_iter()
        .filter_map(|(a, b, anterior)| {
            let pares: Vec<(f64, f64)> = if anterior {
                dias.windows(2)
                    .filter_map(|w| Some((a.de(&w[0])?, b.de(&w[1])?)))
                    .collect()
            } else {
                dias.iter()
                    .filter_map(|d| Some((a.de(d)?, b.de(d)?)))
                    .collect()
            };
            let r = pearson(&pares)?;
            (r.abs() >= UMBRAL).then(|| Relacion {
                causa_posible: a,
                efecto: b,
                dia_anterior: anterior,
                coeficiente: (r * 100.0).round() / 100.0,
                muestra: u32::try_from(pares.len()).unwrap_or(u32::MAX),
            })
        })
        .collect();
    salida.sort_by(|x, y| y.coeficiente.abs().total_cmp(&x.coeficiente.abs()));
    salida
}

/// # Errors
/// Los de [`dias`].
pub fn evolucion(db: &BaseDeDatos, n: u32) -> ScResult<Evolucion> {
    let mut conn = db.conexion()?;
    let tx = db::lectura_coherente(&mut conn)?;
    // La comparación semanal necesita catorce días aunque la vista pida menos.
    let todos = dias(&tx, n.max(14))?;
    let activos = todos
        .iter()
        .rev()
        .take(7)
        .filter(|d| Medida::TODAS.iter().any(|m| m.de(d).is_some()))
        .count();
    let semana = semana(&todos);
    let relaciones = relaciones(&todos);
    let visibles = todos.len().saturating_sub(n as usize);
    Ok(Evolucion {
        dias: todos[visibles..].to_vec(),
        semana,
        relaciones,
        dias_activos: u32::try_from(activos).unwrap_or(7),
    })
}

/// Un registro del historial unificado.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Movimiento {
    pub tipo: TipoDeMovimiento,
    pub id: i64,
    pub dia: String,
    pub instante: String,
    pub titulo: String,
    pub valor: Option<f64>,
    pub detalle: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub enum TipoDeMovimiento {
    Comida,
    Agua,
    Peso,
    Entreno,
    Diario,
    Cafeina,
    Sintoma,
    Signo,
    Digestion,
    Sesion,
}

/// Historial de un rango de días, más reciente primero y acotado a 500 entradas.
///
/// # Errors
/// [`ScError::InvalidInput`] con fechas inválidas o un rango mayor de un año.
pub fn historial(db: &BaseDeDatos, desde: &str, hasta: &str) -> ScResult<Vec<Movimiento>> {
    let (Some(d), Some(h)) = (fechas::de_clave(desde), fechas::de_clave(hasta)) else {
        return Err(ScError::InvalidInput);
    };
    if h < d || (h - d).num_days() > i64::from(MAX_DIAS) {
        return Err(ScError::InvalidInput);
    }
    let mut conn = db.conexion()?;
    let tx = db::lectura_coherente(&mut conn)?;
    let consultas: [(TipoDeMovimiento, &str); 10] = [
        (
            TipoDeMovimiento::Comida,
            "SELECT l.id, l.date_key, l.logged_at, f.name, l.total_kcal, CAST(round(l.grams_consumed) AS INTEGER) || ' g'
             FROM food_logs l JOIN food_items f ON f.id = l.food_item_id WHERE l.date_key BETWEEN ?1 AND ?2",
        ),
        (
            TipoDeMovimiento::Agua,
            "SELECT id, date_key, logged_at, '', volume_ml, NULL FROM hydration_logs WHERE date_key BETWEEN ?1 AND ?2",
        ),
        (
            TipoDeMovimiento::Peso,
            "SELECT id, date_key, measured_at, '', weight_kg,
                    CASE WHEN waist_cm IS NOT NULL THEN 'waist:' || waist_cm END
             FROM body_measurements WHERE date_key BETWEEN ?1 AND ?2",
        ),
        (
            TipoDeMovimiento::Entreno,
            "SELECT id, date_key, performed_at, category, duration_minutes, rpe_intensity
             FROM workout_blocks WHERE date_key BETWEEN ?1 AND ?2",
        ),
        (
            TipoDeMovimiento::Diario,
            "SELECT id, date_key, created_at, substr(content, 1, 120), mood_score, NULL
             FROM journal_entries WHERE date_key BETWEEN ?1 AND ?2",
        ),
        (
            TipoDeMovimiento::Cafeina,
            "SELECT id, date_key, logged_at, source_name, caffeine_mg, NULL FROM caffeine_logs WHERE date_key BETWEEN ?1 AND ?2",
        ),
        (
            TipoDeMovimiento::Sintoma,
            "SELECT id, date_key, logged_at, symptom, intensity, notes FROM symptom_logs WHERE date_key BETWEEN ?1 AND ?2",
        ),
        (
            TipoDeMovimiento::Signo,
            "SELECT id, date_key, measured_at, kind, primary_value,
                    CASE WHEN secondary_value IS NOT NULL THEN CAST(secondary_value AS TEXT) END
             FROM vital_readings WHERE date_key BETWEEN ?1 AND ?2",
        ),
        (
            TipoDeMovimiento::Digestion,
            "SELECT id, date_key, logged_at, 'Bristol ' || bristol_type, discomfort_level, notes
             FROM digestive_logs WHERE date_key BETWEEN ?1 AND ?2",
        ),
        (
            TipoDeMovimiento::Sesion,
            "SELECT id, date_key, completed_at, kind, round(duration_seconds / 60.0, 1), NULL
             FROM care_sessions WHERE date_key BETWEEN ?1 AND ?2",
        ),
    ];
    let mut salida = Vec::new();
    for (tipo, sql) in consultas {
        let mut st = tx.prepare(sql).map_err(fallo)?;
        let filas = st
            .query_map(params![desde, hasta], |f| {
                Ok(Movimiento {
                    tipo,
                    id: f.get(0)?,
                    dia: f.get(1)?,
                    instante: f.get(2)?,
                    titulo: f.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    valor: f.get(4)?,
                    detalle: f.get(5)?,
                })
            })
            .map_err(fallo)?;
        for fila in filas {
            salida.push(fila.map_err(fallo)?);
        }
    }
    salida.sort_by(|a, b| b.instante.cmp(&a.instante));
    salida.truncate(500);
    Ok(salida)
}

/// Borra un registro del historial con la regla de su propio módulo.
///
/// # Errors
/// Los del módulo propietario.
pub fn borrar(db: &BaseDeDatos, tipo: TipoDeMovimiento, id: i64) -> ScResult<()> {
    match tipo {
        TipoDeMovimiento::Comida => crate::comida::borrar(db, &[id]),
        TipoDeMovimiento::Agua => crate::comida::borrar_vaso(db, id),
        TipoDeMovimiento::Peso => crate::cuerpo::borrar(db, id),
        TipoDeMovimiento::Entreno => crate::cuerpo::borrar_entreno(db, id),
        TipoDeMovimiento::Diario => crate::diario::borrar(db, id),
        TipoDeMovimiento::Cafeina
        | TipoDeMovimiento::Sintoma
        | TipoDeMovimiento::Signo
        | TipoDeMovimiento::Digestion
        | TipoDeMovimiento::Sesion => {
            let tabla = match tipo {
                TipoDeMovimiento::Cafeina => "caffeine_logs",
                TipoDeMovimiento::Sintoma => "symptom_logs",
                TipoDeMovimiento::Digestion => "digestive_logs",
                TipoDeMovimiento::Sesion => "care_sessions",
                _ => "vital_readings",
            };
            let mut conn = db.conexion()?;
            let tx = db::transaccion_de_escritura(&mut conn)?;
            let n = tx
                .execute(&format!("DELETE FROM {tabla} WHERE id = ?1"), [id])
                .map_err(fallo)?;
            if n == 0 {
                return Err(ScError::NotFound);
            }
            tx.commit().map_err(fallo)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perfil::pruebas::base_con_perfil;

    #[test]
    fn pearson_detecta_relaciones_y_descarta_muestras_pequenas() {
        let lineal: Vec<(f64, f64)> = (0..12)
            .map(|i| (f64::from(i), f64::from(i) * 2.0 + 1.0))
            .collect();
        assert!((pearson(&lineal).unwrap() - 1.0).abs() < 1e-9);
        let inversa: Vec<(f64, f64)> = (0..12).map(|i| (f64::from(i), -f64::from(i))).collect();
        assert!((pearson(&inversa).unwrap() + 1.0).abs() < 1e-9);
        assert_eq!(pearson(&lineal[..5]), None);
        let plana: Vec<(f64, f64)> = (0..12).map(|i| (f64::from(i), 3.0)).collect();
        assert_eq!(pearson(&plana), None);
    }

    #[test]
    fn la_semana_compara_con_la_anterior_y_cuenta_dias_con_dato() {
        let mut dias: Vec<DiaEvolucion> = (0..14)
            .map(|i| DiaEvolucion {
                dia: format!("d{i}"),
                ..Default::default()
            })
            .collect();
        for (i, d) in dias.iter_mut().enumerate() {
            d.agua_ml = Some(if i < 7 { 1000.0 } else { 2000.0 });
        }
        dias[13].agua_ml = None;
        let s = semana(&dias);
        let agua = s.iter().find(|c| c.medida == Medida::AguaMl).unwrap();
        assert_eq!(
            (agua.esta, agua.anterior, agua.dias_con_dato),
            (Some(2000.0), Some(1000.0), 6)
        );
        assert!(s.iter().all(|c| c.medida != Medida::PesoKg));
    }

    #[test]
    fn el_sueno_de_la_noche_se_cruza_con_el_dia_y_la_cafeina_con_la_noche_siguiente() {
        let dias: Vec<DiaEvolucion> = (0..20)
            .map(|i| {
                let s = 5.0 + f64::from(i % 5);
                DiaEvolucion {
                    dia: format!("d{i}"),
                    sueno_h: Some(s),
                    animo: Some(s - 3.0),
                    cafeina_mg: Some(f64::from(i % 3) * 100.0),
                    ..Default::default()
                }
            })
            .collect();
        let r = relaciones(&dias);
        let animo = r
            .iter()
            .find(|x| x.causa_posible == Medida::SuenoH && x.efecto == Medida::Animo)
            .unwrap();
        assert!(animo.coeficiente > 0.99 && !animo.dia_anterior && animo.muestra == 20);
        assert!(r.iter().all(|x| x.coeficiente.abs() >= UMBRAL));
    }

    #[test]
    fn los_dias_reunen_cada_registro_y_el_historial_permite_corregir() {
        let db = base_con_perfil();
        crate::comida::beber(&db, 750).unwrap();
        let e = evolucion(&db, 30).unwrap();
        assert_eq!(e.dias.len(), 30);
        assert_eq!(e.dias.last().unwrap().agua_ml, Some(750.0));
        assert_eq!(e.dias_activos, 1);
        assert!(dias(&db.conexion().unwrap(), 3).is_err());
        let hoy = fechas::clave(fechas::hoy());
        let h = historial(&db, &hoy, &hoy).unwrap();
        let agua = h.iter().find(|m| m.tipo == TipoDeMovimiento::Agua).unwrap();
        borrar(&db, TipoDeMovimiento::Agua, agua.id).unwrap();
        assert!(historial(&db, &hoy, &hoy)
            .unwrap()
            .iter()
            .all(|m| m.tipo != TipoDeMovimiento::Agua));
        assert_eq!(
            historial(&db, &hoy, "2000-01-01"),
            Err(ScError::InvalidInput)
        );
        assert_eq!(
            borrar(&db, TipoDeMovimiento::Signo, 999),
            Err(ScError::NotFound)
        );
    }
}
