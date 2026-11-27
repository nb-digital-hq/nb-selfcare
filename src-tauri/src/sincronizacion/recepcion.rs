//! Recepción transaccional aislada. No instala tablas ni habilita rutas sanitarias.
use super::contrato::{Cambio, Identificador, Operacion, MAX_REVISION};
use super::huella::huella_del_cambio;
use crate::db::{fallo, transaccion_de_escritura};
use crate::error::{ScError, ScResult};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoRecepcion {
    Aplicado { revision: u64 },
    Conflicto { revision: u64 },
    EpocaCambiada,
    OperacionAlterada,
    RevisionAgotada,
}

/// El adaptador propio valida campos/referencias y devuelve solo la instantánea compartida.
/// Aplicar no vuelve a ejecutar premios o efectos ya representados en la operación.
pub trait PropietarioDeRegistro {
    /// # Errors
    /// Error canónico de validación o almacenamiento, sin contenido personal.
    fn comprobar(
        &self,
        tx: &Transaction<'_>,
        cambio: &Cambio,
    ) -> ScResult<Option<Map<String, Value>>>;
    /// # Errors
    /// Un error revierte datos, revisión y recibo conjuntamente.
    fn aplicar(&self, tx: &Transaction<'_>, cambio: &Cambio) -> ScResult<()>;
}

/// El llamador autentica al nodo y comprueba consentimiento antes de entrar.
/// Requiere infraestructura instalada por el propietario; nunca la crea silenciosamente.
/// # Errors
/// [`ScError::InvalidInput`] para un cambio inválido; [`ScError::StorageFailure`] ante fallos de SQLite.
pub fn recibir(
    conn: &mut Connection,
    epoca: &Identificador,
    nodo: &Identificador,
    cambio: &Cambio,
    propietario: &impl PropietarioDeRegistro,
) -> ScResult<ResultadoRecepcion> {
    let huella = huella_del_cambio(nodo, cambio)?;
    let tx = transaccion_de_escritura(conn)?;
    if !tx
        .pragma_query_value(None, "foreign_keys", |f| f.get::<_, bool>(0))
        .map_err(fallo)?
    {
        return Err(ScError::StorageFailure);
    }
    let actual: String = tx
        .query_row("SELECT epoca FROM sc_sync_contexto WHERE id=1", [], |f| {
            f.get(0)
        })
        .map_err(fallo)?;
    Identificador::try_from(actual.clone()).map_err(|_| ScError::StorageFailure)?;
    if actual != epoca.exponer() {
        return Ok(ResultadoRecepcion::EpocaCambiada);
    }
    let previo = tx.query_row(
        "SELECT epoca,huella,resultado,revision FROM sc_sync_recibos WHERE nodo=?1 AND operacion=?2",
        params![nodo.exponer(), cambio.id.exponer()],
        |f| Ok((f.get::<_, String>(0)?, f.get::<_, Vec<u8>>(1)?, f.get::<_, String>(2)?, f.get::<_, i64>(3)?)),
    ).optional().map_err(fallo)?;
    if let Some((epoca_previa, huella_previa, resultado, revision)) = previo {
        let revision = u64::try_from(revision).map_err(|_| ScError::StorageFailure)?;
        Identificador::try_from(epoca_previa.clone()).map_err(|_| ScError::StorageFailure)?;
        if huella_previa.len() != 32 || revision > MAX_REVISION {
            return Err(ScError::StorageFailure);
        }
        if epoca_previa != actual {
            return Ok(ResultadoRecepcion::EpocaCambiada);
        }
        if huella_previa != huella {
            return Ok(ResultadoRecepcion::OperacionAlterada);
        }
        return match resultado.as_str() {
            "aplicado" if revision > 0 => Ok(ResultadoRecepcion::Aplicado { revision }),
            "conflicto" => {
                let existe: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM sc_sync_conflictos WHERE nodo=?1 AND operacion=?2)",
                    params![nodo.exponer(), cambio.id.exponer()], |f| f.get(0),
                ).map_err(fallo)?;
                if !existe {
                    return Err(ScError::StorageFailure);
                }
                Ok(ResultadoRecepcion::Conflicto { revision })
            }
            _ => Err(ScError::StorageFailure),
        };
    }
    let entidad = serde_json::to_value(cambio.entidad).map_err(|_| ScError::InvalidInput)?;
    let entidad = entidad.as_str().ok_or(ScError::InvalidInput)?;
    // El propietario puede vincular una identidad natural antes de leer la revisión.
    let instantanea = propietario.comprobar(&tx, cambio)?;
    let revision = tx
        .query_row(
            "SELECT revision FROM sc_sync_revisiones WHERE entidad=?1 AND registro=?2",
            params![entidad, cambio.registro.exponer()],
            |f| f.get::<_, i64>(0),
        )
        .optional()
        .map_err(fallo)?
        .unwrap_or(0);
    let revision = u64::try_from(revision).map_err(|_| ScError::StorageFailure)?;
    if revision > MAX_REVISION {
        return Err(ScError::StorageFailure);
    }
    // Si la última escritura del registro vino de este mismo par, una base anterior solo
    // significa que no recibió la confirmación: su edición nueva continúa la suya.
    let continua_la_suya = cambio.base < revision
        && tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sc_sync_diario WHERE registro = ?1 AND origen = ?2)",
                params![cambio.registro.exponer(), nodo.exponer()],
                |f| f.get::<_, bool>(0),
            )
            .unwrap_or(false);
    let base = if continua_la_suya {
        revision
    } else {
        cambio.base
    };
    if let Some(datos) = &instantanea {
        let mut comprobacion = cambio.clone();
        comprobacion.operacion = Operacion::Guardar;
        comprobacion.datos = Some(datos.clone());
        huella_del_cambio(nodo, &comprobacion).map_err(|_| ScError::StorageFailure)?;
    }
    if base != revision {
        guardar_recibo(&tx, nodo, cambio, epoca, &huella, ("conflicto", revision))?;
        let actual = instantanea
            .map(|v| serde_json::to_string(&v))
            .transpose()
            .map_err(|_| ScError::StorageFailure)?;
        let propuesta = serde_json::to_string(cambio).map_err(|_| ScError::InvalidInput)?;
        tx.execute(
            "INSERT INTO sc_sync_conflictos(nodo,operacion,actual,propuesta) VALUES(?1,?2,?3,?4)",
            params![nodo.exponer(), cambio.id.exponer(), actual, propuesta],
        )
        .map_err(fallo)?;
        tx.commit().map_err(fallo)?;
        return Ok(ResultadoRecepcion::Conflicto { revision });
    }
    if revision == MAX_REVISION {
        return Ok(ResultadoRecepcion::RevisionAgotada);
    }
    let nueva = revision + 1;
    propietario.aplicar(&tx, cambio)?;
    tx.execute(
        "INSERT INTO sc_sync_revisiones(entidad,registro,revision) VALUES(?1,?2,?3) \
         ON CONFLICT(entidad,registro) DO UPDATE SET revision=excluded.revision",
        params![
            entidad,
            cambio.registro.exponer(),
            i64::try_from(nueva).map_err(|_| ScError::StorageFailure)?
        ],
    )
    .map_err(fallo)?;
    guardar_recibo(&tx, nodo, cambio, epoca, &huella, ("aplicado", nueva))?;
    tx.commit().map_err(fallo)?;
    Ok(ResultadoRecepcion::Aplicado { revision: nueva })
}

fn guardar_recibo(
    tx: &Transaction<'_>,
    nodo: &Identificador,
    cambio: &Cambio,
    epoca: &Identificador,
    huella: &[u8; 32],
    (resultado, revision): (&str, u64),
) -> ScResult<()> {
    tx.execute(
        "INSERT INTO sc_sync_recibos(nodo,operacion,epoca,huella,resultado,revision) VALUES(?1,?2,?3,?4,?5,?6)",
        params![nodo.exponer(), cambio.id.exponer(), epoca.exponer(), huella.as_slice(), resultado, i64::try_from(revision).map_err(|_| ScError::StorageFailure)?],
    ).map_err(fallo)?;
    Ok(())
}

#[cfg(test)]
#[path = "recepcion_tests.rs"]
mod tests;
