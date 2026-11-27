//! Intercambio con SelfCare Mobile: el escritorio es el centro de la reconciliación.
//!
//! Un teléfono envía sus cambios pendientes con su revisión base; el escritorio los
//! aplica o conserva el conflicto. Después el teléfono recibe, en orden de diario, lo
//! que todavía no conoce. Los cambios que llegaron de un par no se le devuelven.
use super::contrato::{Cambio, Identificador, Lote, Operacion, MAX_CAMBIOS};
use super::dominio::{self, entidad_texto, Propietario};
use super::recepcion::{recibir, ResultadoRecepcion};
use crate::db::{fallo, lectura_coherente, transaccion_de_escritura};
use crate::error::{ScError, ScResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashSet;
use ts_rs::TS;

/// Petición de un teléfono emparejado.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Peticion {
    pub version: u32,
    /// Época del escritorio que el teléfono conoce; `None` la primera vez.
    pub epoca: Option<Identificador>,
    /// Último elemento del diario del escritorio que el teléfono ya aplicó.
    pub cursor: u64,
    pub lote: Option<Lote>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Estado {
    Aplicado,
    Conflicto,
    Rechazado,
}

#[derive(Serialize)]
pub struct Resultado {
    pub id: Identificador,
    pub estado: Estado,
    pub revision: u64,
}

#[derive(Serialize)]
pub struct Respuesta {
    pub version: u32,
    pub epoca: Identificador,
    /// La época cambió desde la última vez: el teléfono vuelve a leer desde cero.
    pub reiniciar: bool,
    pub resultados: Vec<Resultado>,
    pub cambios: Vec<Cambio>,
    pub cursor: u64,
    pub mas: bool,
}

fn epoca(conn: &Connection) -> ScResult<Identificador> {
    let e: String = conn
        .query_row("SELECT epoca FROM sc_sync_contexto WHERE id = 1", [], |f| {
            f.get(0)
        })
        .map_err(fallo)?;
    Identificador::try_from(e).map_err(|_| ScError::StorageFailure)
}

/// Atiende una petición autenticada de `nodo`.
///
/// # Errors
/// [`ScError::InvalidInput`] para sobres inválidos; errores de almacenamiento canónicos.
pub fn atender(
    conn: &mut Connection,
    nodo: &Identificador,
    peticion: Peticion,
) -> ScResult<Respuesta> {
    if peticion.version != super::contrato::VERSION {
        return Err(ScError::InvalidInput);
    }
    if let Some(lote) = &peticion.lote {
        lote.validar()?;
        if lote.nodo != *nodo {
            return Err(ScError::InvalidInput);
        }
    }
    let actual = epoca(conn)?;
    let reiniciar = peticion.epoca.as_ref().is_some_and(|e| *e != actual);
    let mut resultados = Vec::new();
    // Tras una restauración, los cambios basados en la época anterior se reenvían después.
    if let (Some(lote), false) = (&peticion.lote, reiniciar) {
        for cambio in &lote.cambios {
            let estado = match recibir(conn, &actual, nodo, cambio, &Propietario { nodo }) {
                Ok(ResultadoRecepcion::Aplicado { revision }) => (Estado::Aplicado, revision),
                Ok(ResultadoRecepcion::Conflicto { revision }) => {
                    marcar_conflicto(conn, cambio)?;
                    (Estado::Conflicto, revision)
                }
                Ok(_) | Err(ScError::InvalidInput) => (Estado::Rechazado, 0),
                Err(e) => return Err(e),
            };
            resultados.push(Resultado {
                id: cambio.id.clone(),
                estado: estado.0,
                revision: estado.1,
            });
        }
    }
    let cursor = if reiniciar { 0 } else { peticion.cursor };
    let (cambios, hasta, mas) = pagina(conn, nodo, cursor, MAX_CAMBIOS)?;
    let tx = transaccion_de_escritura(conn)?;
    tx.execute(
        "INSERT INTO sc_sync_pares (nodo, epoca, cursor, ultima) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (nodo) DO UPDATE SET epoca = excluded.epoca, cursor = excluded.cursor, ultima = excluded.ultima",
        params![
            nodo.exponer(),
            actual.exponer(),
            i64::try_from(cursor).map_err(|_| ScError::InvalidInput)?,
            crate::fechas::ahora().format("%Y-%m-%dT%H:%M:%S").to_string()
        ],
    )
    .map_err(fallo)?;
    tx.commit().map_err(fallo)?;
    Ok(Respuesta {
        version: super::contrato::VERSION,
        epoca: actual,
        reiniciar,
        resultados,
        cambios,
        cursor: hasta,
        mas,
    })
}

fn marcar_conflicto(conn: &mut Connection, cambio: &Cambio) -> ScResult<()> {
    let tx = transaccion_de_escritura(conn)?;
    tx.execute(
        "UPDATE sc_sync_identidades SET en_conflicto = 1 WHERE registro = ?1",
        [cambio.registro.exponer()],
    )
    .map_err(fallo)?;
    tx.commit().map_err(fallo)
}

/// Cambios del diario posteriores a `cursor` que no vinieron de `nodo`, con sus dependencias primero.
///
/// # Errors
/// Errores de almacenamiento canónicos.
pub fn pagina(
    conn: &mut Connection,
    nodo: &Identificador,
    cursor: u64,
    limite: usize,
) -> ScResult<(Vec<Cambio>, u64, bool)> {
    let tx = lectura_coherente(conn)?;
    let cursor_sql = i64::try_from(cursor).map_err(|_| ScError::InvalidInput)?;
    let filas: Vec<(i64, String)> = {
        let mut st = tx
            .prepare(
                "SELECT seq, registro FROM sc_sync_diario
                 WHERE seq > ?1 AND (origen IS NULL OR origen <> ?2) ORDER BY seq LIMIT ?3",
            )
            .map_err(fallo)?;
        let v = st
            .query_map(
                params![
                    cursor_sql,
                    nodo.exponer(),
                    i64::try_from(limite + 1).unwrap_or(i64::MAX)
                ],
                |f| Ok((f.get(0)?, f.get(1)?)),
            )
            .map_err(fallo)?
            .collect::<Result<_, _>>()
            .map_err(fallo)?;
        v
    };
    let mas = filas.len() > limite;
    let mut cambios = Vec::new();
    let mut emitidos = HashSet::new();
    let mut hasta = cursor;
    for (seq, registro) in filas.into_iter().take(limite) {
        // Una página nunca supera el límite del lote, ni siquiera con sus dependencias.
        if cambios.len() >= limite {
            break;
        }
        emitir(
            &tx,
            &registro,
            (cursor_sql, limite),
            &mut cambios,
            &mut emitidos,
            0,
        )?;
        hasta = u64::try_from(seq).map_err(|_| ScError::StorageFailure)?;
    }
    let mas = mas || hasta < ultimo_seq(&tx, nodo)?;
    Ok((cambios, hasta, mas))
}

fn ultimo_seq(tx: &rusqlite::Transaction<'_>, nodo: &Identificador) -> ScResult<u64> {
    let n: i64 = tx
        .query_row(
            "SELECT coalesce(max(seq), 0) FROM sc_sync_diario WHERE origen IS NULL OR origen <> ?1",
            [nodo.exponer()],
            |f| f.get(0),
        )
        .map_err(fallo)?;
    u64::try_from(n).map_err(|_| ScError::StorageFailure)
}

fn emitir(
    tx: &rusqlite::Transaction<'_>,
    registro: &str,
    (cursor, limite): (i64, usize),
    cambios: &mut Vec<Cambio>,
    emitidos: &mut HashSet<String>,
    profundidad: u8,
) -> ScResult<()> {
    if profundidad > 4 || cambios.len() >= limite || !emitidos.insert(registro.to_owned()) {
        return Ok(());
    }
    let (tabla, clave, eliminado): (String, String, bool) = tx
        .query_row(
            "SELECT tabla, clave, eliminado FROM sc_sync_identidades WHERE registro = ?1",
            [registro],
            |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)),
        )
        .map_err(fallo)?;
    let t = dominio::por_tabla(&tabla).ok_or(ScError::StorageFailure)?;
    let datos = if eliminado {
        None
    } else {
        dominio::leer(tx, t, &clave)?
    };
    if let Some(d) = &datos {
        for (col, destino) in &t.referencias {
            if let Some(Value::String(r)) = d.get(col) {
                // El par no conoce la dependencia si su entrada del diario es posterior al cursor
                // o si nunca entró en el diario (catálogo sembrado).
                let pendiente: Option<i64> = tx
                    .query_row(
                        "SELECT seq FROM sc_sync_diario WHERE registro = ?1",
                        [r],
                        |f| f.get(0),
                    )
                    .optional()
                    .map_err(fallo)?;
                if pendiente.is_none_or(|s| s > cursor) && dominio::por_tabla(destino).is_some() {
                    emitir(tx, r, (cursor, limite), cambios, emitidos, profundidad + 1)?;
                }
            }
        }
    }
    let revision: i64 = tx
        .query_row(
            "SELECT revision FROM sc_sync_revisiones WHERE entidad = ?1 AND registro = ?2",
            params![entidad_texto(t.entidad)?, registro],
            |f| f.get(0),
        )
        .optional()
        .map_err(fallo)?
        .unwrap_or(1);
    cambios.push(Cambio {
        id: Identificador::generar()?,
        registro: Identificador::try_from(registro.to_owned())
            .map_err(|_| ScError::StorageFailure)?,
        entidad: t.entidad,
        base: u64::try_from(revision.max(1) - 1).map_err(|_| ScError::StorageFailure)?,
        operacion: if datos.is_some() {
            Operacion::Guardar
        } else {
            Operacion::Eliminar
        },
        datos,
    });
    Ok(())
}

/// Un conflicto pendiente, con las dos versiones para decidir.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Conflicto {
    pub nodo: String,
    pub operacion: String,
    pub entidad: String,
    /// Falta en este equipo: se borró aquí mientras el teléfono lo editaba.
    pub borrado_aqui: bool,
    /// El teléfono propone borrarlo.
    pub borrado_en_telefono: bool,
    /// Solo los campos que difieren, como texto para mostrarlos.
    pub campos: Vec<CampoEnConflicto>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct CampoEnConflicto {
    pub campo: String,
    pub este_equipo: Option<String>,
    pub telefono: Option<String>,
}

fn como_texto(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        otro => Some(otro.to_string()),
    }
}

fn diferencias(
    a: Option<&Map<String, Value>>,
    b: Option<&Map<String, Value>>,
) -> Vec<CampoEnConflicto> {
    let mut claves: Vec<&String> = a.into_iter().chain(b).flat_map(Map::keys).collect();
    claves.sort();
    claves.dedup();
    claves
        .into_iter()
        .filter(|k| a.and_then(|m| m.get(*k)) != b.and_then(|m| m.get(*k)))
        .map(|k| CampoEnConflicto {
            campo: k.clone(),
            este_equipo: como_texto(a.and_then(|m| m.get(k))),
            telefono: como_texto(b.and_then(|m| m.get(k))),
        })
        .collect()
}

/// # Errors
/// Errores de almacenamiento canónicos.
pub fn conflictos(conn: &mut Connection, limite: usize) -> ScResult<Vec<Conflicto>> {
    let tx = lectura_coherente(conn)?;
    let filas: Vec<(String, String, Option<String>, String)> = {
        let mut st = tx
            .prepare("SELECT nodo, operacion, actual, propuesta FROM sc_sync_conflictos ORDER BY rowid LIMIT ?1")
            .map_err(fallo)?;
        let v = st
            .query_map([i64::try_from(limite).unwrap_or(100)], |f| {
                Ok((f.get(0)?, f.get(1)?, f.get(2)?, f.get(3)?))
            })
            .map_err(fallo)?
            .collect::<Result<_, _>>()
            .map_err(fallo)?;
        v
    };
    filas
        .into_iter()
        .map(|(nodo, operacion, actual, propuesta)| {
            let cambio: Cambio =
                serde_json::from_str(&propuesta).map_err(|_| ScError::StorageFailure)?;
            let este_equipo: Option<Map<String, Value>> = actual
                .map(|a| serde_json::from_str(&a))
                .transpose()
                .map_err(|_| ScError::StorageFailure)?;
            Ok(Conflicto {
                nodo,
                operacion,
                entidad: entidad_texto(cambio.entidad)?,
                borrado_aqui: este_equipo.is_none(),
                borrado_en_telefono: cambio.datos.is_none(),
                campos: diferencias(este_equipo.as_ref(), cambio.datos.as_ref()),
            })
        })
        .collect()
}

/// Resuelve un conflicto. La versión elegida se publica con una revisión nueva y el
/// teléfono la recibe en su próxima sincronización.
///
/// # Errors
/// [`ScError::NotFound`] si el conflicto ya no existe.
pub fn resolver(
    conn: &mut Connection,
    nodo: &str,
    operacion: &str,
    usar_telefono: bool,
) -> ScResult<()> {
    let tx = transaccion_de_escritura(conn)?;
    let propuesta: String = tx
        .query_row(
            "SELECT propuesta FROM sc_sync_conflictos WHERE nodo = ?1 AND operacion = ?2",
            params![nodo, operacion],
            |f| f.get(0),
        )
        .optional()
        .map_err(fallo)?
        .ok_or(ScError::NotFound)?;
    let cambio: Cambio = serde_json::from_str(&propuesta).map_err(|_| ScError::StorageFailure)?;
    let entidad = entidad_texto(cambio.entidad)?;
    let registro = cambio.registro.exponer();
    let actual: i64 = tx
        .query_row(
            "SELECT revision FROM sc_sync_revisiones WHERE entidad = ?1 AND registro = ?2",
            params![entidad, registro],
            |f| f.get(0),
        )
        .optional()
        .map_err(fallo)?
        .unwrap_or(0);
    let nueva = u64::try_from(actual)
        .map_err(|_| ScError::StorageFailure)?
        .max(cambio.base + 1)
        + 1;
    if usar_telefono {
        // Se aplica como edición local: entra al diario sin origen y viaja a todos los pares.
        let local = Identificador::generar()?;
        dominio::escribir(&tx, local.exponer(), &cambio, nueva)?;
        tx.execute(
            "UPDATE sc_sync_diario SET origen = NULL WHERE registro = ?1",
            [registro],
        )
        .map_err(fallo)?;
    } else {
        tx.execute("DELETE FROM sc_sync_diario WHERE registro = ?1", [registro])
            .map_err(fallo)?;
        tx.execute(
            "INSERT INTO sc_sync_diario (registro, origen) SELECT registro, NULL FROM sc_sync_identidades WHERE registro = ?1",
            [registro],
        )
        .map_err(fallo)?;
    }
    tx.execute(
        "INSERT INTO sc_sync_revisiones (entidad, registro, revision) VALUES (?1, ?2, ?3)
         ON CONFLICT (entidad, registro) DO UPDATE SET revision = excluded.revision",
        params![
            entidad,
            registro,
            i64::try_from(nueva).map_err(|_| ScError::StorageFailure)?
        ],
    )
    .map_err(fallo)?;
    tx.execute(
        "UPDATE sc_sync_identidades SET en_conflicto = 0, compartida = ?2 WHERE registro = ?1",
        params![
            registro,
            i64::try_from(nueva - 1).map_err(|_| ScError::StorageFailure)?
        ],
    )
    .map_err(fallo)?;
    tx.execute(
        "DELETE FROM sc_sync_conflictos WHERE nodo = ?1 AND operacion = ?2",
        params![nodo, operacion],
    )
    .map_err(fallo)?;
    tx.commit().map_err(fallo)
}

/// Resumen para la interfaz de conexión.
#[derive(Debug)]
pub struct EstadoSincronizacion {
    pub pendientes_de_revisar: u32,
    pub ultima: Option<String>,
    pub registros: u32,
}

/// # Errors
/// Errores de almacenamiento canónicos.
pub fn estado(conn: &mut Connection) -> ScResult<EstadoSincronizacion> {
    let tx = lectura_coherente(conn)?;
    let pendientes: i64 = tx
        .query_row("SELECT count(*) FROM sc_sync_conflictos", [], |f| f.get(0))
        .map_err(fallo)?;
    let ultima: Option<String> = tx
        .query_row("SELECT max(ultima) FROM sc_sync_pares", [], |f| f.get(0))
        .map_err(fallo)?;
    let registros: i64 = tx
        .query_row(
            "SELECT count(*) FROM sc_sync_identidades WHERE compartida > 0 AND eliminado = 0",
            [],
            |f| f.get(0),
        )
        .map_err(fallo)?;
    Ok(EstadoSincronizacion {
        pendientes_de_revisar: u32::try_from(pendientes).unwrap_or(u32::MAX),
        ultima,
        registros: u32::try_from(registros).unwrap_or(u32::MAX),
    })
}

#[cfg(test)]
#[path = "servicio_tests.rs"]
mod tests;
