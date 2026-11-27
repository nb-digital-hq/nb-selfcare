//! Adaptadores sanitarios de sincronización, declarados en `sincronizacion/adaptadores-v1.json`.
//!
//! La especificación es la misma en SelfCare Mobile. Los nombres de tabla y columna que
//! llegan a SQL salen siempre de ella, nunca del par. Aplicar un cambio escribe la fila
//! tal como llegó: no repite recompensas, consumos de reserva ni avisos.
use super::contrato::{Cambio, Entidad, Identificador, Operacion, MAX_REVISION};
use super::recepcion::PropietarioDeRegistro;
use crate::db::fallo;
use crate::error::{ScError, ScResult};
use rusqlite::types::Value as Sql;
use rusqlite::{params, params_from_iter, OptionalExtension, Transaction};
use serde::Deserialize;
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tipo {
    Texto,
    Real,
    Entero,
    Booleano,
}

#[derive(Debug, Deserialize)]
pub struct Columna {
    pub nombre: String,
    pub tipo: Tipo,
    pub nulo: bool,
}

#[derive(Debug, Deserialize)]
pub struct Natural {
    pub columnas: Vec<String>,
    pub excepto: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct Tabla {
    pub tabla: String,
    pub entidad: Entidad,
    pub clave: String,
    pub unico: bool,
    pub publicar_si: Option<String>,
    pub columnas: Vec<Columna>,
    pub referencias: BTreeMap<String, String>,
    pub natural: Vec<Natural>,
}

#[derive(Debug, Deserialize)]
struct Especificacion {
    version: u32,
    tablas: Vec<Tabla>,
}

static DOMINIO: LazyLock<Especificacion> = LazyLock::new(|| {
    let e: Especificacion =
        serde_json::from_str(include_str!("../../sincronizacion/adaptadores-v1.json"))
            .expect("especificación de sincronización embebida");
    assert_eq!(e.version, 1);
    e
});

/// Tablas compartidas en orden de dependencia (padres antes que hijos).
pub fn tablas() -> &'static [Tabla] {
    &DOMINIO.tablas
}

pub fn por_entidad(entidad: Entidad) -> Option<&'static Tabla> {
    tablas().iter().find(|t| t.entidad == entidad)
}

pub fn por_tabla(tabla: &str) -> Option<&'static Tabla> {
    tablas().iter().find(|t| t.tabla == tabla)
}

fn citar(nombre: &str) -> String {
    format!("\"{nombre}\"")
}

/// Identidad estable de una fila local, si la tiene.
pub(crate) fn identidad_de(
    tx: &Transaction<'_>,
    tabla: &str,
    clave: &str,
) -> ScResult<Option<String>> {
    tx.query_row(
        "SELECT registro FROM sc_sync_identidades WHERE tabla = ?1 AND clave = ?2",
        params![tabla, clave],
        |f| f.get(0),
    )
    .optional()
    .map_err(fallo)
}

struct Local {
    clave: String,
    eliminado: bool,
}

fn local_de(tx: &Transaction<'_>, t: &Tabla, registro: &str) -> ScResult<Option<Local>> {
    let fila = tx
        .query_row(
            "SELECT tabla, clave, eliminado, compartida FROM sc_sync_identidades WHERE registro = ?1",
            [registro],
            |f| {
                Ok((
                    f.get::<_, String>(0)?,
                    f.get::<_, String>(1)?,
                    f.get::<_, bool>(2)?,
                    f.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(fallo)?;
    match fila {
        None => Ok(None),
        // Una identidad no puede cambiar de tabla: sería otra entidad con el mismo nombre.
        Some((tabla, _, _, _)) if tabla != t.tabla => Err(ScError::InvalidInput),
        Some((_, clave, eliminado, _)) => {
            let existe = existe_fila(tx, t, &clave)?;
            Ok(Some(Local {
                clave,
                eliminado: eliminado || !existe,
            }))
        }
    }
}

fn existe_fila(tx: &Transaction<'_>, t: &Tabla, clave: &str) -> ScResult<bool> {
    tx.query_row(
        &format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE CAST({} AS TEXT) = ?1)",
            citar(&t.tabla),
            citar(&t.clave)
        ),
        [clave],
        |f| f.get(0),
    )
    .map_err(fallo)
}

/// Instantánea compartida de una fila: columnas del contrato con referencias como identidades.
///
/// # Errors
/// [`ScError::StorageFailure`] si la fila o una referencia no tiene identidad.
pub fn leer(tx: &Transaction<'_>, t: &Tabla, clave: &str) -> ScResult<Option<Map<String, Value>>> {
    let lista = t
        .columnas
        .iter()
        .map(|c| citar(&c.nombre))
        .collect::<Vec<_>>()
        .join(", ");
    let valores: Option<Vec<Sql>> = tx
        .query_row(
            &format!(
                "SELECT {lista} FROM {} WHERE CAST({} AS TEXT) = ?1",
                citar(&t.tabla),
                citar(&t.clave)
            ),
            [clave],
            |f| (0..t.columnas.len()).map(|i| f.get::<_, Sql>(i)).collect(),
        )
        .optional()
        .map_err(fallo)?;
    let Some(valores) = valores else {
        return Ok(None);
    };
    let mut datos = Map::new();
    for (c, v) in t.columnas.iter().zip(valores) {
        let valor = match (v, c.tipo) {
            (Sql::Null, _) => Value::Null,
            (Sql::Integer(n), _) if t.referencias.contains_key(&c.nombre) => {
                let destino = &t.referencias[&c.nombre];
                let registro =
                    identidad_de(tx, destino, &n.to_string())?.ok_or(ScError::StorageFailure)?;
                Value::String(registro)
            }
            (Sql::Integer(n), Tipo::Booleano) => Value::Bool(n != 0),
            (Sql::Integer(n), Tipo::Entero) => Value::Number(n.into()),
            (Sql::Integer(n), Tipo::Real) => Number::from_f64(n as f64)
                .map(Value::Number)
                .ok_or(ScError::StorageFailure)?,
            (Sql::Real(x), Tipo::Real | Tipo::Entero) => Number::from_f64(x)
                .map(Value::Number)
                .ok_or(ScError::StorageFailure)?,
            (Sql::Text(s), Tipo::Texto) => Value::String(s),
            // SQLite no tipa con rigor: un texto numérico en una columna real se conserva como texto.
            (Sql::Text(s), _) => Value::String(s),
            (Sql::Integer(n), Tipo::Texto) => Value::String(n.to_string()),
            (Sql::Real(x), _) => Value::String(x.to_string()),
            (Sql::Blob(_), _) => return Err(ScError::StorageFailure),
        };
        datos.insert(c.nombre.clone(), valor);
    }
    Ok(Some(datos))
}

/// Valida los datos del par contra el contrato y los traduce a valores SQL locales.
fn traducir(tx: &Transaction<'_>, t: &Tabla, datos: &Map<String, Value>) -> ScResult<Vec<Sql>> {
    if datos.len() != t.columnas.len() {
        return Err(ScError::InvalidInput);
    }
    let mut salida = Vec::with_capacity(t.columnas.len());
    for c in &t.columnas {
        let v = datos.get(&c.nombre).ok_or(ScError::InvalidInput)?;
        let sql = match (v, c.tipo) {
            (Value::Null, _) if c.nulo => Sql::Null,
            (Value::String(r), _) if t.referencias.contains_key(&c.nombre) => {
                let destino = por_tabla(&t.referencias[&c.nombre]).ok_or(ScError::InvalidInput)?;
                Identificador::try_from(r.clone())?;
                let local = local_de(tx, destino, r)?.ok_or(ScError::InvalidInput)?;
                if local.eliminado {
                    return Err(ScError::InvalidInput);
                }
                Sql::Integer(local.clave.parse().map_err(|_| ScError::StorageFailure)?)
            }
            (_, _) if t.referencias.contains_key(&c.nombre) => return Err(ScError::InvalidInput),
            (Value::Bool(b), Tipo::Booleano) => Sql::Integer(i64::from(*b)),
            (Value::Number(n), Tipo::Entero) => {
                let x = n.as_f64().ok_or(ScError::InvalidInput)?;
                if x.fract() != 0.0 || x.abs() > MAX_REVISION as f64 {
                    return Err(ScError::InvalidInput);
                }
                #[allow(clippy::cast_possible_truncation)]
                Sql::Integer(x as i64)
            }
            (Value::Number(n), Tipo::Real) => Sql::Real(
                n.as_f64()
                    .filter(|x| x.is_finite())
                    .ok_or(ScError::InvalidInput)?,
            ),
            (Value::String(s), Tipo::Texto) => Sql::Text(s.clone()),
            _ => return Err(ScError::InvalidInput),
        };
        salida.push(sql);
    }
    Ok(salida)
}

/// Busca una fila local equivalente que todavía no se haya compartido nunca.
fn equivalente(tx: &Transaction<'_>, t: &Tabla, valores: &[Sql]) -> ScResult<Option<String>> {
    for alternativa in &t.natural {
        let mut condiciones = Vec::new();
        let mut argumentos = Vec::new();
        let mut valida = true;
        for col in &alternativa.columnas {
            let i = t.columnas.iter().position(|c| &c.nombre == col);
            let Some(i) = i else {
                valida = false;
                break;
            };
            argumentos.push(valores[i].clone());
            condiciones.push(format!("f.{} IS ?{}", citar(col), argumentos.len() + 1));
        }
        for (col, excluido) in &alternativa.excepto {
            let i = t.columnas.iter().position(|c| &c.nombre == col);
            if let (Some(i), Some(texto)) = (i, excluido.as_str()) {
                if valores[i] == Sql::Text(texto.to_owned()) {
                    valida = false;
                }
            }
        }
        // Una alternativa sin ningún valor no identifica nada (salvo el registro único).
        if !valida || (!argumentos.is_empty() && argumentos.iter().all(|a| *a == Sql::Null)) {
            continue;
        }
        let donde = if condiciones.is_empty() {
            String::new()
        } else {
            format!(" AND {}", condiciones.join(" AND "))
        };
        let clave: Option<String> = tx
            .query_row(
                &format!(
                    "SELECT i.clave FROM {} f JOIN sc_sync_identidades i
                     ON i.tabla = ?1 AND i.clave = CAST(f.{} AS TEXT)
                     WHERE i.compartida = 0 AND i.eliminado = 0{donde} LIMIT 1",
                    citar(&t.tabla),
                    citar(&t.clave)
                ),
                params_from_iter(
                    std::iter::once(&Sql::Text(t.tabla.clone())).chain(argumentos.iter()),
                ),
                |f| f.get(0),
            )
            .optional()
            .map_err(fallo)?;
        if clave.is_some() {
            return Ok(clave);
        }
    }
    Ok(None)
}

/// Si todas las columnas compartidas conservan su valor por defecto del esquema.
fn de_fabrica(tx: &Transaction<'_>, t: &Tabla, clave: &str) -> ScResult<bool> {
    let mut condiciones = Vec::new();
    for c in &t.columnas {
        let defecto: Option<Option<String>> = tx
            .query_row(
                "SELECT dflt_value FROM pragma_table_info(?1) WHERE name = ?2",
                params![t.tabla, c.nombre],
                |f| f.get(0),
            )
            .optional()
            .map_err(fallo)?;
        match defecto {
            Some(Some(d)) => condiciones.push(format!("{} IS {d}", citar(&c.nombre))),
            Some(None) if c.nulo => condiciones.push(format!("{} IS NULL", citar(&c.nombre))),
            _ => return Ok(false),
        }
    }
    if condiciones.is_empty() {
        return Ok(false);
    }
    tx.query_row(
        &format!(
            "SELECT {} FROM {} WHERE CAST({} AS TEXT) = ?1",
            condiciones.join(" AND "),
            citar(&t.tabla),
            citar(&t.clave)
        ),
        [clave],
        |f| f.get::<_, bool>(0),
    )
    .map_err(fallo)
}

/// Adopta la identidad del par para una fila local equivalente. Si los contenidos
/// difieren, la fila local queda como edición pendiente y el receptor ve el conflicto.
fn vincular(
    tx: &Transaction<'_>,
    t: &Tabla,
    clave: &str,
    registro: &str,
    igual: bool,
) -> ScResult<()> {
    let anterior = identidad_de(tx, &t.tabla, clave)?.ok_or(ScError::StorageFailure)?;
    let entidad = entidad_texto(t.entidad)?;
    tx.execute(
        "DELETE FROM sc_sync_revisiones WHERE entidad = ?1 AND registro = ?2",
        params![entidad, registro],
    )
    .map_err(fallo)?;
    tx.execute(
        "UPDATE sc_sync_identidades SET registro = ?1 WHERE tabla = ?2 AND clave = ?3",
        params![registro, t.tabla, clave],
    )
    .map_err(fallo)?;
    if igual {
        tx.execute(
            "DELETE FROM sc_sync_revisiones WHERE entidad = ?1 AND registro = ?2",
            params![entidad, anterior],
        )
        .map_err(fallo)?;
    } else {
        tx.execute(
            "UPDATE sc_sync_revisiones SET registro = ?1 WHERE entidad = ?2 AND registro = ?3",
            params![registro, entidad, anterior],
        )
        .map_err(fallo)?;
    }
    Ok(())
}

pub(crate) fn entidad_texto(e: Entidad) -> ScResult<String> {
    serde_json::to_value(e)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or(ScError::InvalidInput)
}

fn control(tx: &Transaction<'_>, origen: Option<&str>, registro: Option<&str>) -> ScResult<()> {
    tx.execute(
        "UPDATE sc_sync_control SET aplicando = ?1, origen = ?2, registro = ?3 WHERE id = 1",
        params![i64::from(origen.is_some()), origen, registro],
    )
    .map_err(fallo)?;
    Ok(())
}

/// Escribe el cambio en la fila local con la autoría `origen` y deja la identidad
/// compartida en `revision`. No decide conflictos: eso corresponde al llamador.
///
/// # Errors
/// [`ScError::InvalidInput`] si los datos no cumplen el contrato o falta una referencia.
pub fn escribir(
    tx: &Transaction<'_>,
    origen: &str,
    cambio: &Cambio,
    revision: u64,
) -> ScResult<()> {
    let t = por_entidad(cambio.entidad).ok_or(ScError::InvalidInput)?;
    let registro = cambio.registro.exponer();
    let local = local_de(tx, t, registro)?;
    control(tx, Some(origen), Some(registro))?;
    let resultado = (|| -> ScResult<()> {
        match cambio.operacion {
            Operacion::Eliminar => {
                if let Some(l) = local.as_ref().filter(|l| !l.eliminado) {
                    let antes = leer(tx, t, &l.clave)?;
                    tx.execute(
                        &format!(
                            "DELETE FROM {} WHERE CAST({} AS TEXT) = ?1",
                            citar(&t.tabla),
                            citar(&t.clave)
                        ),
                        [&l.clave],
                    )
                    .map_err(fallo)?;
                    proyectar(tx, t, antes.as_ref(), None)?;
                }
            }
            Operacion::Guardar => {
                let datos = cambio.datos.as_ref().ok_or(ScError::InvalidInput)?;
                let valores = traducir(tx, t, datos)?;
                match local.as_ref() {
                    Some(l) if !l.eliminado => {
                        let antes = leer(tx, t, &l.clave)?;
                        let asignaciones = t
                            .columnas
                            .iter()
                            .enumerate()
                            .map(|(i, c)| format!("{} = ?{}", citar(&c.nombre), i + 1))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let mut args = valores.clone();
                        args.push(Sql::Text(l.clave.clone()));
                        tx.execute(
                            &format!(
                                "UPDATE {} SET {asignaciones} WHERE CAST({} AS TEXT) = ?{}",
                                citar(&t.tabla),
                                citar(&t.clave),
                                args.len()
                            ),
                            params_from_iter(args.iter()),
                        )
                        .map_err(fallo)?;
                        proyectar(tx, t, antes.as_ref(), Some(datos))?;
                    }
                    _ => {
                        // La identidad de una fila borrada se reutiliza para la fila nueva.
                        tx.execute("DELETE FROM sc_sync_diario WHERE registro = ?1", [registro])
                            .map_err(fallo)?;
                        tx.execute(
                            "DELETE FROM sc_sync_identidades WHERE registro = ?1",
                            [registro],
                        )
                        .map_err(fallo)?;
                        let lista = t
                            .columnas
                            .iter()
                            .map(|c| citar(&c.nombre))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let marcas = (1..=valores.len())
                            .map(|i| format!("?{i}"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        tx.execute(
                            &format!(
                                "INSERT INTO {} ({lista}) VALUES ({marcas})",
                                citar(&t.tabla)
                            ),
                            params_from_iter(valores.iter()),
                        )
                        .map_err(fallo)?;
                        proyectar(tx, t, None, Some(datos))?;
                    }
                }
            }
        }
        tx.execute(
            "UPDATE sc_sync_identidades SET compartida = ?1, en_conflicto = 0 WHERE registro = ?2",
            params![
                i64::try_from(revision).map_err(|_| ScError::StorageFailure)?,
                registro
            ],
        )
        .map_err(fallo)?;
        Ok(())
    })();
    control(tx, None, None)?;
    resultado
}

/// Proyecciones que dependen de un registro aplicado: el agua del día y el saldo de Nibi.
fn proyectar(
    tx: &Transaction<'_>,
    t: &Tabla,
    antes: Option<&Map<String, Value>>,
    despues: Option<&Map<String, Value>>,
) -> ScResult<()> {
    match t.tabla.as_str() {
        "hydration_logs" => {
            for dia in [antes, despues]
                .into_iter()
                .flatten()
                .filter_map(|d| d.get("date_key").and_then(Value::as_str))
            {
                tx.execute(
                    "UPDATE daily_summaries SET water_consumed_ml =
                     (SELECT coalesce(sum(volume_ml), 0) FROM hydration_logs WHERE date_key = ?1)
                     WHERE date_key = ?1",
                    [dia],
                )
                .map_err(fallo)?;
            }
        }
        "coin_ledger" => {
            let cantidad = |d: Option<&Map<String, Value>>| {
                d.and_then(|d| d.get("amount"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
            };
            let delta = cantidad(despues) - cantidad(antes);
            if delta != 0 {
                tx.execute(
                    "UPDATE pet_status SET coins_balance = max(0, coins_balance + ?1)",
                    [delta],
                )
                .map_err(fallo)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Propietario de todas las tablas del contrato para el receptor transaccional.
pub struct Propietario<'a> {
    pub nodo: &'a Identificador,
}

impl PropietarioDeRegistro for Propietario<'_> {
    fn comprobar(
        &self,
        tx: &Transaction<'_>,
        cambio: &Cambio,
    ) -> ScResult<Option<Map<String, Value>>> {
        let t = por_entidad(cambio.entidad).ok_or(ScError::InvalidInput)?;
        let registro = cambio.registro.exponer();
        let mut local = local_de(tx, t, registro)?;
        if cambio.operacion == Operacion::Guardar {
            let datos = cambio.datos.as_ref().ok_or(ScError::InvalidInput)?;
            let valores = traducir(tx, t, datos)?;
            if local.is_none() {
                if let Some(clave) = equivalente(tx, t, &valores)? {
                    // Un registro único que este equipo creó solo con valores de fábrica (Nibi
                    // recién instalado) no es una edición: adopta la versión del par sin conflicto.
                    let igual = leer(tx, t, &clave)?.as_ref() == Some(datos)
                        || (t.unico && de_fabrica(tx, t, &clave)?);
                    vincular(tx, t, &clave, registro, igual)?;
                    local = local_de(tx, t, registro)?;
                }
            }
        }
        match local {
            Some(l) if !l.eliminado => leer(tx, t, &l.clave),
            _ => Ok(None),
        }
    }

    fn aplicar(&self, tx: &Transaction<'_>, cambio: &Cambio) -> ScResult<()> {
        // El receptor fija la revisión nueva justo después: la actual más uno.
        let actual: i64 = tx
            .query_row(
                "SELECT revision FROM sc_sync_revisiones WHERE entidad = ?1 AND registro = ?2",
                params![entidad_texto(cambio.entidad)?, cambio.registro.exponer()],
                |f| f.get(0),
            )
            .optional()
            .map_err(fallo)?
            .unwrap_or(0);
        let actual = u64::try_from(actual).map_err(|_| ScError::StorageFailure)?;
        escribir(tx, self.nodo.exponer(), cambio, actual + 1)
    }
}

#[cfg(test)]
#[path = "dominio_tests.rs"]
mod tests;
