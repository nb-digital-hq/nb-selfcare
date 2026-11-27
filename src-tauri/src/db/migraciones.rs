//! Migraciones de esquema.
//!
//! La base vive en la máquina del usuario: si una migración falla, esa persona
//! pierde su historial de salud. De ahí las reglas, las mismas en toda la suite:
//!
//! 1. **Embebidas en el binario** con `include_str!`.
//! 2. **Transaccionales:** cada una se aplica entera o no se aplica.
//! 3. **Solo hacia adelante.**
//! 4. **Respaldo previo** antes de tocar una base con datos.
//! 5. Una versión antigua que encuentra una base más nueva **se niega a operar**.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{ScError, ScResult};

#[derive(Clone, Copy)]
struct Migracion {
    version: u32,
    nombre: &'static str,
    sql: &'static str,
}

/// En orden estricto. **Nunca se edita una entrada publicada**: un error en una
/// migración aplicada se corrige con una migración nueva.
///
/// La numeración es la del esquema de datos de NB SelfCare, compartida con el
/// teléfono (ADR-004): el primer esquema del escritorio es el 8, el mismo que el
/// de NB SelfCare Mobile 1.0. Por eso una copia de cualquiera de los dos se lee
/// en el otro.
const MIGRACIONES: &[Migracion] = &[
    Migracion {
        version: 8,
        nombre: "esquema 8 de NB SelfCare",
        sql: include_str!("../../migrations/V1__inicial.sql"),
    },
    // Generada por `scripts/generar-sincronizacion.mjs`; idéntica en SelfCare Mobile (ADR-014).
    Migracion {
        version: 9,
        nombre: "esquema 9: identidades y diario de sincronización",
        sql: include_str!("../../migrations/V9__sincronizacion.sql"),
    },
];

/// Versión de esquema aplicada (`PRAGMA user_version`).
///
/// # Errors
///
/// [`ScError::StorageFailure`] si no se puede leer.
pub fn version_actual(conn: &Connection) -> ScResult<u32> {
    conn.query_row("PRAGMA user_version", [], |f| f.get::<_, i64>(0))
        .map(|v| v.max(0) as u32)
        .map_err(|_| ScError::StorageFailure)
}

/// La versión de esquema que deja esta compilación: la de su última migración.
#[must_use]
pub fn version_esperada() -> u32 {
    MIGRACIONES.last().map_or(0, |m| m.version)
}

/// Aplica las migraciones pendientes. `ruta` es la del archivo, para el
/// respaldo; `None` en las bases en memoria de las pruebas.
///
/// # Errors
///
/// - [`ScError::SchemaMismatch`] si la base es más nueva que esta versión, o si
///   una migración dejó referencias rotas.
/// - [`ScError::StorageFailure`] si una migración falla; se revierte y el
///   respaldo queda en disco.
pub fn aplicar_pendientes(conn: &mut Connection, ruta: Option<&Path>) -> ScResult<u32> {
    aplicar(conn, ruta, MIGRACIONES)
}

/// El motor, con la lista como parámetro para poder probar con listas sintéticas
/// las propiedades que importan (reversión, salto de versiones, respaldo).
fn aplicar(conn: &mut Connection, ruta: Option<&Path>, migraciones: &[Migracion]) -> ScResult<u32> {
    let actual = version_actual(conn)?;
    let objetivo = migraciones.last().map_or(0, |m| m.version);

    if actual > objetivo {
        tracing::error!(
            esquema_actual = actual,
            esquema_soportado = objetivo,
            "la base de datos es más nueva que esta versión de la aplicación"
        );
        return Err(ScError::SchemaMismatch);
    }
    if actual == objetivo {
        return Ok(actual);
    }
    if actual > 0 {
        if let Some(ruta) = ruta {
            respaldar(conn, ruta, actual)?;
        }
    }

    // Las claves foráneas se apagan alrededor de las migraciones y no dentro:
    // reconstruir una tabla con ellas encendidas dispara los `ON DELETE CASCADE`
    // de quien la referencia y borra hijos en silencio (un defecto conocido de SQLite). El
    // `PRAGMA` no hace nada dentro de una transacción, por eso va aquí, y la
    // comprobación de después convierte cualquier daño en un error ruidoso.
    conn.pragma_update(None, "foreign_keys", "OFF")
        .map_err(|_| ScError::StorageFailure)?;
    let resultado = aplicar_en_orden(conn, migraciones, actual);
    let reencendido = conn
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|_| ScError::StorageFailure);
    resultado?;
    reencendido?;
    comprobar_referencias(conn)?;

    tracing::info!(esquema = objetivo, "esquema actualizado");
    Ok(objetivo)
}

fn aplicar_en_orden(conn: &mut Connection, migraciones: &[Migracion], actual: u32) -> ScResult<()> {
    for m in migraciones.iter().filter(|m| m.version > actual) {
        tracing::info!(
            version = m.version,
            nombre = m.nombre,
            "aplicando migración"
        );
        let tx = super::transaccion_de_escritura(conn)?;
        tx.execute_batch(m.sql).map_err(|_error| {
            tracing::error!(version = m.version, "la migración falló; se revierte");
            ScError::StorageFailure
        })?;
        comprobar_referencias(&tx)?;
        // `user_version` no admite parámetros ligados.
        tx.pragma_update(None, "user_version", m.version)
            .map_err(|_| ScError::StorageFailure)?;
        tx.commit().map_err(|_| ScError::StorageFailure)?;
    }
    Ok(())
}

fn comprobar_referencias(conn: &Connection) -> ScResult<()> {
    let violaciones: i64 = conn
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |f| {
            f.get(0)
        })
        .map_err(|_| ScError::StorageFailure)?;
    if violaciones > 0 {
        tracing::error!(violaciones, "una migración dejó referencias rotas");
        return Err(ScError::SchemaMismatch);
    }
    Ok(())
}

/// Copia de seguridad antes de migrar, con `VACUUM INTO`: ve la base entera,
/// incluido lo que aún está en el `-wal` (una copia del archivo no lo vería). Se escribe aparte y se renombra
/// al terminar, así que un respaldo a medias nunca sustituye a uno entero.
fn respaldar(conn: &Connection, ruta: &Path, desde: u32) -> ScResult<()> {
    let mut destino = ruta.as_os_str().to_owned();
    destino.push(format!(".v{desde}.respaldo"));
    let destino = PathBuf::from(destino);
    let mut parcial = destino.as_os_str().to_owned();
    parcial.push(".parcial");
    let parcial = PathBuf::from(parcial);
    let _ = std::fs::remove_file(&parcial);

    conn.execute("VACUUM INTO ?1", [parcial.to_string_lossy()])
        .map_err(|_error| {
            let _ = std::fs::remove_file(&parcial);
            tracing::error!("no se pudo crear el respaldo previo a la migración");
            ScError::StorageFailure
        })?;
    std::fs::rename(&parcial, &destino).map_err(|_error| {
        let _ = std::fs::remove_file(&parcial);
        tracing::error!("no se pudo dejar en su sitio el respaldo");
        ScError::StorageFailure
    })?;
    tracing::info!(
        desde_version = desde,
        respaldo = %crate::telemetry::redact_path(&destino),
        "respaldo previo a la migración creado"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNA: &[Migracion] = &[Migracion {
        version: 1,
        nombre: "una",
        sql: "CREATE TABLE a (x INTEGER) STRICT;",
    }];

    #[test]
    fn una_migracion_con_referencias_rotas_revierte_antes_de_confirmar_su_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        aplicar(&mut conn, None, UNA).unwrap();
        conn.execute("INSERT INTO a VALUES (12)", []).unwrap();
        let rotas = [
            UNA[0],
            Migracion {
                version: 2,
                nombre: "referencias inválidas",
                sql: "CREATE TABLE padre (id INTEGER PRIMARY KEY);
                  CREATE TABLE hijo (padre INTEGER REFERENCES padre(id));
                  INSERT INTO hijo VALUES (99);",
            },
        ];
        assert_eq!(
            aplicar(&mut conn, None, &rotas),
            Err(ScError::SchemaMismatch)
        );
        assert_eq!(version_actual(&conn).unwrap(), 1);
        assert_eq!(
            conn.query_row("SELECT x FROM a", [], |f| f.get::<_, i64>(0))
                .unwrap(),
            12
        );
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name IN ('padre', 'hijo')",
                [],
                |f| f.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row("PRAGMA foreign_keys", [], |f| f.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    const DOS: &[Migracion] = &[
        Migracion {
            version: 1,
            nombre: "una",
            sql: "CREATE TABLE a (x INTEGER) STRICT;",
        },
        Migracion {
            version: 2,
            nombre: "dos",
            sql: "ALTER TABLE a ADD COLUMN y TEXT;",
        },
    ];
    const ROTA: &[Migracion] = &[
        Migracion {
            version: 1,
            nombre: "una",
            sql: "CREATE TABLE a (x INTEGER) STRICT;",
        },
        Migracion {
            version: 2,
            nombre: "rota",
            sql: "INSERT INTO a VALUES (1); ESTO NO ES SQL;",
        },
    ];

    fn temporal(nombre: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sc-mig-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("carpeta");
        dir
    }

    #[test]
    fn las_migraciones_reales_se_aplican_y_son_idempotentes() {
        let mut conn = Connection::open_in_memory().unwrap();
        let v = aplicar_pendientes(&mut conn, None).unwrap();
        assert_eq!(v, MIGRACIONES.last().unwrap().version);
        assert_eq!(aplicar_pendientes(&mut conn, None).unwrap(), v);
    }

    #[test]
    fn una_migracion_que_falla_no_deja_nada_a_medias() {
        let mut conn = Connection::open_in_memory().unwrap();
        aplicar(&mut conn, None, UNA).unwrap();
        assert!(aplicar(&mut conn, None, ROTA).is_err());
        assert_eq!(version_actual(&conn).unwrap(), 1);
        let filas: i64 = conn
            .query_row("SELECT count(*) FROM a", [], |f| f.get(0))
            .unwrap();
        assert_eq!(filas, 0, "el INSERT de la migración rota se revirtió");
    }

    #[test]
    fn una_base_mas_nueva_se_rechaza() {
        let mut conn = Connection::open_in_memory().unwrap();
        aplicar(&mut conn, None, DOS).unwrap();
        assert_eq!(aplicar(&mut conn, None, UNA), Err(ScError::SchemaMismatch));
    }

    #[test]
    fn migrar_una_base_con_datos_deja_un_respaldo_legible() {
        let dir = temporal("respaldo");
        let ruta = dir.join("base.sqlite");
        let mut conn = Connection::open(&ruta).unwrap();
        aplicar(&mut conn, Some(&ruta), UNA).unwrap();
        conn.execute("INSERT INTO a VALUES (42)", []).unwrap();
        aplicar(&mut conn, Some(&ruta), DOS).unwrap();
        let respaldo = Connection::open(dir.join("base.sqlite.v1.respaldo")).unwrap();
        let x: i64 = respaldo
            .query_row("SELECT x FROM a", [], |f| f.get(0))
            .unwrap();
        assert_eq!(x, 42);
        assert!(!dir.join("base.sqlite.v1.respaldo.parcial").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn una_referencia_rota_tras_migrar_es_un_error() {
        let mut conn = Connection::open_in_memory().unwrap();
        const HUERFANA: &[Migracion] = &[Migracion {
            version: 1,
            nombre: "huérfana",
            sql: "CREATE TABLE p (id INTEGER PRIMARY KEY) STRICT;
                  CREATE TABLE h (p INTEGER REFERENCES p(id)) STRICT;
                  INSERT INTO h VALUES (7);",
        }];
        assert_eq!(
            aplicar(&mut conn, None, HUERFANA),
            Err(ScError::SchemaMismatch)
        );
    }
}
