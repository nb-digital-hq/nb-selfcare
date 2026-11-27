//! Hoy: lo del día en una sola consulta, para la pantalla de inicio.

use chrono::Timelike;
use serde::Serialize;
use ts_rs::TS;

use crate::comida::{self, Totales};
use crate::cuidado::{self, SesionesDeHoy, Toma};
use crate::db::{self, BaseDeDatos};
use crate::diario::{self, Cafeina, Disposicion, Entrada, Noche};
use crate::error::ScResult;
use crate::fechas;
use crate::formulas::Metas;
use crate::nibi::{self, DatosDelDia, EstadoNibi};
use crate::perfil;

/// Todo lo de Hoy. Sin perfil, `perfil` es `None` y la interfaz abre la
/// bienvenida.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Hoy {
    pub perfil: Option<perfil::Perfil>,
    pub dia: String,
    pub hora: u32,
    pub metas: Option<Metas>,
    pub totales: Totales,
    pub agua_ml: i64,
    pub noche: Option<Noche>,
    pub disposicion: Option<Disposicion>,
    pub horas_de_ayuno: Option<f64>,
    pub nibi: Option<EstadoNibi>,
    pub tomas: Vec<Toma>,
    pub habitos_hechos: i64,
    pub habitos_del_dia: i64,
    pub sesiones: SesionesDeHoy,
    pub diario: Option<Entrada>,
    pub cafeina: Option<Cafeina>,
    pub peso: Option<f64>,
    pub tendencia_7: Option<f64>,
    pub comidas: i64,
    pub chequeos_pendientes: i64,
}

/// Lo del día.
///
/// # Errors
///
/// [`crate::error::ScError::StorageFailure`] si una consulta falla.
pub fn hoy(db: &BaseDeDatos) -> ScResult<Hoy> {
    let ahora = fechas::ahora();
    let dia = fechas::clave(ahora.date());
    let mut conn = db.conexion()?;
    let tx = db::lectura_coherente(&mut conn)?;
    let fila = perfil::leer(&tx)?;
    let totales = comida::totales(&tx, &dia)?;
    let agua_ml = comida::agua_del_dia(&tx, &dia)?;
    let Some(fila) = fila else {
        return Ok(Hoy {
            perfil: None,
            dia,
            hora: ahora.hour(),
            metas: None,
            totales,
            agua_ml,
            noche: None,
            disposicion: None,
            horas_de_ayuno: None,
            nibi: None,
            tomas: Vec::new(),
            habitos_hechos: 0,
            habitos_del_dia: 0,
            sesiones: SesionesDeHoy::default(),
            diario: None,
            cafeina: None,
            peso: None,
            tendencia_7: None,
            comidas: 0,
            chequeos_pendientes: 0,
        });
    };
    let noche = diario::noche(&tx, &dia)?;
    let horas_de_ayuno = crate::ayuno::horas_en_curso(&tx)?;
    let habitos = cuidado::habitos(&tx)?;
    let comidas: i64 = tx
        .query_row(
            "SELECT count(DISTINCT meal_type) FROM food_logs WHERE date_key = ?1",
            [&dia],
            |f| f.get(0),
        )
        .map_err(db::fallo)?;
    let entreno: bool = tx
        .query_row(
            "SELECT count(*) > 0 FROM workout_blocks WHERE date_key = ?1",
            [&dia],
            |f| f.get(0),
        )
        .map_err(db::fallo)?;
    let peso = perfil::ultimo_peso(&tx)?;
    let hace7 = fechas::clave(fechas::mas_dias(ahora.date(), -7));
    let tendencia_antes: Option<f64> = tx
        .query_row(
            "SELECT trend_weight_kg FROM body_measurements WHERE date_key <= ?1 AND source != 'manual_navy_tape'
             ORDER BY measured_at DESC LIMIT 1",
            [&hace7],
            |f| f.get(0),
        )
        .ok();
    let chequeos_pendientes = cuidado::chequeos(&tx)?
        .iter()
        .filter(|c| c.activo && c.estado == "due")
        .count() as i64;
    let agua_meta = fila.perfil.agua_meta_ml.max(1);
    drop(tx);
    let durmio_bien = noche.is_some_and(|n| n.minutos as f64 >= fila.perfil.sueno_meta_h * 60.0);
    let nibi_estado = nibi::estado(
        db,
        DatosDelDia {
            hora: ahora.hour(),
            agua: agua_ml as f64 / agua_meta as f64,
            durmio_bien,
            horas_de_ayuno,
            entreno,
            racha: 0,
            comidas,
        },
    )?;
    let mut conn = db.conexion()?;
    let tx = db::lectura_coherente(&mut conn)?;
    Ok(Hoy {
        metas: Some(comida::metas_del_dia(&tx, &dia)?),
        disposicion: diario::disposicion(&tx)?,
        tomas: cuidado::tomas_del_dia(&tx, ahora.date())?,
        sesiones: cuidado::sesiones_de_hoy(&tx)?,
        diario: diario::entradas(&tx, 1)?.into_iter().find(|e| e.dia == dia),
        cafeina: Some(diario::cafeina(&tx)?),
        habitos_hechos: habitos.iter().filter(|h| h.toca_hoy && h.hecho_hoy).count() as i64,
        habitos_del_dia: habitos.iter().filter(|h| h.toca_hoy).count() as i64,
        tendencia_7: peso
            .zip(tendencia_antes)
            .map(|((_, t), a)| crate::formulas::fijo(t - a, 2)),
        peso: peso.map(|p| p.0),
        perfil: Some(fila.perfil),
        dia,
        hora: ahora.hour(),
        totales,
        agua_ml,
        noche,
        horas_de_ayuno,
        nibi: Some(nibi_estado),
        comidas,
        chequeos_pendientes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_perfil_hoy_pide_la_bienvenida() {
        let db = BaseDeDatos::en_memoria().unwrap();
        assert!(hoy(&db).unwrap().perfil.is_none());
    }

    #[test]
    fn con_perfil_hoy_trae_metas_y_a_nibi() {
        let db = crate::perfil::pruebas::base_con_perfil();
        let h = hoy(&db).unwrap();
        assert!(h.metas.is_some());
        assert_eq!(h.peso, Some(82.0));
        assert!(h.nibi.is_some());
    }
}
