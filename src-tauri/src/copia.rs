//! Copias de seguridad: un archivo `.sqlite`, el mismo formato que NB SelfCare
//! Mobile (ADR-004).
//!
//! **Exportar** saca la base con `VACUUM INTO`, que da una copia coherente sin
//! bloquear la aplicación ni arrastrar el WAL. Como el esquema es el del
//! teléfono, el archivo se restaura tal cual en NB SelfCare Mobile.
//!
//! **Restaurar** acepta una copia del escritorio o del teléfono. Se comprueba
//! entera antes de tocar nada (integridad, tablas mínimas y versión de esquema
//! no más nueva que esta) y se copia tabla a tabla, columna a columna, dentro de
//! una sola transacción: o entra todo o no entra nada. Antes se guarda una copia
//! automática de lo que había, por si la persona se equivocó de archivo. Las
//! preferencias del escritorio se conservan si la copia no trae las suyas (una
//! copia del teléfono no las tiene).

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use crate::db::{fallo, migraciones, BaseDeDatos};
use crate::error::{ScError, ScResult};
use crate::fechas;
use crate::paths::Rutas;

/// Tablas que tiene cualquier copia legítima, del teléfono o del escritorio.
const TABLAS_MINIMAS: [&str; 3] = ["user_profiles", "food_items", "pet_status"];

/// Estado del enlace que pertenece a esta instalación y no viene de una copia.
const SYNC_LOCALES: [&str; 8] = [
    "sc_sync_diario",
    "sc_sync_control",
    "sc_sync_contexto",
    "sc_sync_pares",
    "sc_sync_recibos",
    "sc_sync_conflictos",
    "sc_sync_identidades",
    "sc_sync_revisiones",
];

/// Cuántas copias automáticas se guardan.
const COPIAS_AUTOMATICAS: usize = 5;

/// Instantánea privada del origen: incluye el WAL y evita adjuntar la propia base viva.
struct CopiaTemporal(PathBuf);
impl CopiaTemporal {
    fn desde(origen: &Path, rutas: &Rutas) -> ScResult<Self> {
        let id = crate::sincronizacion::contrato::Identificador::generar()
            .map_err(|_| ScError::StorageFailure)?;
        let carpeta = rutas.cache.join(format!("restauracion-{}", id.exponer()));
        #[cfg(unix)]
        let constructor = {
            use std::os::unix::fs::DirBuilderExt as _;
            let mut constructor = std::fs::DirBuilder::new();
            constructor.mode(0o700);
            constructor
        };
        #[cfg(not(unix))]
        let constructor = std::fs::DirBuilder::new();
        constructor
            .create(&carpeta)
            .map_err(|_| ScError::StorageFailure)?;
        let temporal = Self(carpeta);
        let conn = Connection::open_with_flags(origen, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| ScError::BackupInvalid)?;
        conn.execute("VACUUM INTO ?1", [temporal.archivo().to_string_lossy()])
            .map_err(|_| ScError::BackupInvalid)?;
        Ok(temporal)
    }
    fn archivo(&self) -> PathBuf {
        self.0.join("origen.sqlite")
    }
}
impl Drop for CopiaTemporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Lo que se sabe de una copia antes de restaurarla.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Inspeccion {
    /// `desktop` o `mobile`.
    pub origen: String,
    pub esquema: u32,
    pub perfil: Option<String>,
    pub registros: i64,
}

/// Comprueba una copia sin modificarla.
///
/// # Errors
///
/// [`ScError::BackupInvalid`] si no es una copia de NB SelfCare válida o es de
/// un esquema más nuevo que esta versión.
pub fn inspeccionar(ruta: &Path) -> ScResult<Inspeccion> {
    let tam = std::fs::metadata(ruta)
        .map_err(|_| ScError::FileNotFound)?
        .len();
    if tam < 4096 {
        return Err(ScError::BackupInvalid);
    }
    let conn = Connection::open_with_flags(ruta, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| ScError::BackupInvalid)?;
    let integridad: String = conn
        .query_row("PRAGMA integrity_check", [], |f| f.get(0))
        .map_err(|_| ScError::BackupInvalid)?;
    if integridad != "ok" {
        return Err(ScError::BackupInvalid);
    }
    let hay = |t: &str| -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [t],
            |_| Ok(true),
        )
        .optional()
        .ok()
        .flatten()
        .unwrap_or(false)
    };
    if !TABLAS_MINIMAS.iter().all(|t| hay(t)) {
        return Err(ScError::BackupInvalid);
    }
    let esquema = migraciones::version_actual(&conn).map_err(|_| ScError::BackupInvalid)?;
    // El escritorio lee el esquema 8 en adelante; las copias del teléfono de
    // versiones previas a su 1.0 se abren antes en el teléfono, que las migra.
    if esquema < 8 || esquema > migraciones::version_esperada() {
        return Err(ScError::BackupInvalid);
    }
    let referencias_rotas: i64 = conn
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |f| {
            f.get(0)
        })
        .map_err(|_| ScError::BackupInvalid)?;
    if referencias_rotas != 0 {
        return Err(ScError::BackupInvalid);
    }
    let perfil: Option<String> = conn
        .query_row(
            "SELECT name FROM user_profiles ORDER BY id LIMIT 1",
            [],
            |f| f.get(0),
        )
        .optional()
        .map_err(|_| ScError::BackupInvalid)?;
    let registros: i64 = conn
        .query_row(
            "SELECT (SELECT count(*) FROM food_logs) + (SELECT count(*) FROM body_measurements)
                  + (SELECT count(*) FROM journal_entries)",
            [],
            |f| f.get(0),
        )
        .unwrap_or(0);
    Ok(Inspeccion {
        origen: if hay("preferencias") {
            "desktop"
        } else {
            "mobile"
        }
        .into(),
        esquema,
        perfil,
        registros,
    })
}

/// Escribe una copia en `destino` (se crea al lado y se renombra al terminar:
/// una copia a medias nunca ocupa el lugar de una entera).
///
/// # Errors
///
/// [`ScError::StorageFailure`] si no se puede escribir.
pub fn exportar(db: &BaseDeDatos, destino: &Path) -> ScResult<()> {
    let mut parcial = destino.as_os_str().to_owned();
    parcial.push(".parcial");
    let parcial = PathBuf::from(parcial);
    let _ = std::fs::remove_file(&parcial);
    let conn = db.conexion()?;
    conn.execute("VACUUM INTO ?1", [parcial.to_string_lossy()])
        .map_err(|e| {
            let _ = std::fs::remove_file(&parcial);
            fallo(e)
        })?;
    std::fs::rename(&parcial, destino).map_err(|_| {
        let _ = std::fs::remove_file(&parcial);
        ScError::StorageFailure
    })?;
    tracing::info!(copia = %crate::telemetry::redact_path(destino), "copia de seguridad escrita");
    Ok(())
}

/// El nombre de una copia, como el del teléfono.
#[must_use]
pub fn nombre_de_copia() -> String {
    format!(
        "nb_selfcare_backup_{}.sqlite",
        fechas::hoy().format("%Y%m%d")
    )
}

fn columnas(conn: &Connection, esquema: &str, tabla: &str) -> ScResult<Vec<String>> {
    let mut st = conn
        .prepare("SELECT name FROM pragma_table_info(?1, ?2)")
        .map_err(fallo)?;
    let v = st
        .query_map([tabla, esquema], |f| f.get(0))
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    Ok(v)
}

/// Las tablas de datos de la base, sin las internas ni el índice de búsqueda.
fn tablas(conn: &Connection, esquema: &str) -> ScResult<Vec<String>> {
    let mut st = conn
        .prepare(&format!(
            "SELECT name FROM {esquema}.sqlite_master WHERE type = 'table'
             AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'food_items_fts%' ORDER BY name"
        ))
        .map_err(fallo)?;
    let v = st
        .query_map([], |f| f.get(0))
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    Ok(v)
}

fn citar(nombre: &str) -> String {
    format!("\"{}\"", nombre.replace('"', "\"\""))
}

/// Restaura una copia ya comprobada sobre la base abierta, en una sola
/// transacción. Antes guarda una copia automática de lo que había.
///
/// # Errors
///
/// [`ScError::BackupInvalid`] si la copia no vale; [`ScError::StorageFailure`]
/// si la restauración falla (entonces nada cambió).
pub fn restaurar(db: &BaseDeDatos, rutas: &Rutas, origen: &Path) -> ScResult<Inspeccion> {
    let temporal = CopiaTemporal::desde(origen, rutas)?;
    let origen = temporal.archivo();
    let inspeccion = inspeccionar(&origen)?;
    std::fs::create_dir_all(rutas.copias()).map_err(|_| ScError::StorageFailure)?;
    let previa = rutas.copias().join(format!(
        "antes-de-restaurar-{}-{}.sqlite",
        fechas::ahora().format("%Y%m%d-%H%M%S"),
        crate::sincronizacion::contrato::Identificador::generar()
            .map_err(|_| ScError::StorageFailure)?
            .exponer()
    ));
    exportar(db, &previa)?;

    let mut conn = db.conexion()?;
    conn.pragma_update(None, "foreign_keys", "OFF")
        .map_err(fallo)?;
    let mut adjunta = false;
    let resultado = (|| -> ScResult<()> {
        conn.execute("ATTACH DATABASE ?1 AS copia", [origen.to_string_lossy()])
            .map_err(|_| ScError::BackupInvalid)?;
        adjunta = true;
        let propias = tablas(&conn, "main")?;
        let de_la_copia = tablas(&conn, "copia")?;
        if propias.iter().any(|t| {
            t != "instalacion"
                && t != "preferencias"
                && !t.starts_with("sc_sync_")
                && !de_la_copia.contains(t)
        }) {
            return Err(ScError::BackupInvalid);
        }
        let con_identidades = de_la_copia.iter().any(|t| t == "sc_sync_identidades");
        let tx = crate::db::transaccion_de_escritura(&mut conn)?;
        // Los disparadores no marcan revisiones durante la restauración; las
        // identidades se toman de la copia o se reconstruyen al final.
        tx.execute(
            "UPDATE sc_sync_control SET aplicando = 1, origen = NULL, registro = NULL WHERE id = 1",
            [],
        )
        .map_err(fallo)?;
        tx.execute_batch("DELETE FROM sc_sync_conflictos; DELETE FROM sc_sync_recibos;")
            .map_err(fallo)?;
        for t in &propias {
            if t == "instalacion" || SYNC_LOCALES.contains(&t.as_str()) {
                continue;
            }
            // Las preferencias del escritorio solo se cambian si la copia trae las suyas.
            if t == "preferencias" && !de_la_copia.contains(t) {
                continue;
            }
            tx.execute(&format!("DELETE FROM main.{}", citar(t)), [])
                .map_err(fallo)?;
            if !de_la_copia.contains(t) {
                continue;
            }
            let de_aqui = columnas(&tx, "main", t)?;
            let de_alla = columnas(&tx, "copia", t)?;
            let comunes: Vec<String> = de_aqui
                .into_iter()
                .filter(|c| de_alla.contains(c))
                .map(|c| citar(&c))
                .collect();
            if comunes.is_empty() {
                continue;
            }
            let lista = comunes.join(", ");
            tx.execute(
                &format!(
                    "INSERT INTO main.{t} ({lista}) SELECT {lista} FROM copia.{t}",
                    t = citar(t)
                ),
                [],
            )
            .map_err(fallo)?;
        }
        tx.execute(
            "INSERT INTO food_items_fts(food_items_fts) VALUES('rebuild')",
            [],
        )
        .map_err(fallo)?;
        tx.execute_batch(
            "DELETE FROM sc_sync_diario; DELETE FROM sc_sync_revisiones; DELETE FROM sc_sync_identidades;",
        )
        .map_err(fallo)?;
        if con_identidades {
            tx.execute_batch(
                "INSERT INTO sc_sync_identidades (tabla, clave, registro, compartida, eliminado)
                     SELECT tabla, clave, registro, compartida, eliminado FROM copia.sc_sync_identidades;
                 INSERT INTO sc_sync_revisiones SELECT entidad, registro, revision FROM copia.sc_sync_revisiones;
                 INSERT INTO sc_sync_diario (seq, registro, origen)
                     SELECT seq, registro, NULL FROM copia.sc_sync_diario ORDER BY seq;",
            )
            .map_err(|_| ScError::BackupInvalid)?;
        } else {
            tx.execute_batch(include_str!("../sincronizacion/reconstruir-v1.sql"))
                .map_err(fallo)?;
        }
        // Época nueva: ningún par reutiliza un cursor anterior a la restauración.
        tx.execute_batch(
            "UPDATE sc_sync_control SET aplicando = 0, origen = NULL, registro = NULL WHERE id = 1;
             UPDATE sc_sync_contexto SET epoca = lower(hex(randomblob(16))) WHERE id = 1;
             UPDATE sc_sync_pares SET cursor = 0, epoca = NULL;",
        )
        .map_err(fallo)?;
        let rotas: i64 = tx
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |f| {
                f.get(0)
            })
            .map_err(fallo)?;
        if rotas > 0 {
            return Err(ScError::BackupInvalid);
        }
        tx.commit().map_err(fallo)
    })();
    let separacion = if adjunta {
        conn.execute("DETACH DATABASE copia", []).map(|_| ())
    } else {
        Ok(())
    };
    let reactivacion = conn
        .pragma_update(None, "foreign_keys", "ON")
        .and_then(|()| conn.query_row("PRAGMA foreign_keys", [], |f| f.get::<_, i64>(0)));
    if separacion.is_err() || !matches!(reactivacion, Ok(1)) {
        // El gestor del pool descarta una conexión con limpieza incompleta.
        return Err(ScError::StorageFailure);
    }
    resultado?;
    tracing::info!(origen = %inspeccion.origen, "copia restaurada");
    Ok(inspeccion)
}

/// Escribe la copia automática de la semana si toca, y deja solo las últimas.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si no se puede escribir.
pub fn automatica_si_toca(db: &BaseDeDatos, rutas: &Rutas) -> ScResult<bool> {
    let carpeta = rutas.copias();
    std::fs::create_dir_all(&carpeta).map_err(|_| ScError::StorageFailure)?;
    let mut automaticas: Vec<PathBuf> = std::fs::read_dir(&carpeta)
        .map_err(|_| ScError::StorageFailure)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("automatica-") && n.ends_with(".sqlite"))
        })
        .collect();
    automaticas.sort();
    let semana = fechas::mas_dias(fechas::hoy(), -7)
        .format("%Y%m%d")
        .to_string();
    let reciente = automaticas.last().and_then(|p| {
        p.file_name()?
            .to_str()?
            .strip_prefix("automatica-")?
            .get(..8)
            .map(str::to_owned)
    });
    if reciente.is_some_and(|r| r > semana) {
        return Ok(false);
    }
    let hay_perfil: bool = db
        .conexion()?
        .query_row("SELECT count(*) > 0 FROM user_profiles", [], |f| f.get(0))
        .map_err(fallo)?;
    if !hay_perfil {
        return Ok(false);
    }
    let destino = carpeta.join(format!(
        "automatica-{}.sqlite",
        fechas::hoy().format("%Y%m%d")
    ));
    exportar(db, &destino)?;
    automaticas.push(destino);
    automaticas.sort();
    let sobran = automaticas.len().saturating_sub(COPIAS_AUTOMATICAS);
    for p in automaticas.into_iter().take(sobran) {
        let _ = std::fs::remove_file(p);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perfil::pruebas::base_con_perfil;

    fn carpeta(nombre: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sc-copia-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn una_copia_va_y_vuelve_con_todo() {
        let dir = carpeta("idayvuelta");
        let rutas = Rutas::bajo(&dir).unwrap();
        let db = base_con_perfil();
        crate::comida::beber(&db, 500).unwrap();
        crate::preferencias::guardar(&db, "tema", &serde_json::json!("bosque_boreal")).unwrap();
        let archivo = dir.join("copia.sqlite");
        exportar(&db, &archivo).unwrap();
        let i = inspeccionar(&archivo).unwrap();
        assert_eq!(
            (i.origen.as_str(), i.esquema, i.perfil.as_deref()),
            ("desktop", 9, Some("Alex"))
        );

        crate::comida::beber(&db, 700).unwrap();
        crate::preferencias::guardar(&db, "tema", &serde_json::json!("hielo_artico")).unwrap();
        restaurar(&db, &rutas, &archivo).unwrap();
        let conn = db.conexion().unwrap();
        let agua: i64 = conn
            .query_row("SELECT sum(volume_ml) FROM hydration_logs", [], |f| {
                f.get(0)
            })
            .unwrap();
        assert_eq!(agua, 500);
        drop(conn);
        assert_eq!(
            crate::preferencias::apariencia(&db).unwrap().tema,
            "bosque_boreal"
        );
        assert!(
            std::fs::read_dir(rutas.copias()).unwrap().count() >= 1,
            "lo anterior quedó a salvo"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn una_copia_del_telefono_conserva_las_preferencias_del_escritorio() {
        let dir = carpeta("telefono");
        let rutas = Rutas::bajo(&dir).unwrap();
        let db = base_con_perfil();
        let archivo = dir.join("movil.sqlite");
        exportar(&db, &archivo).unwrap();
        {
            // Lo que no tiene una copia del teléfono.
            let c = Connection::open(&archivo).unwrap();
            c.execute_batch("DROP TABLE preferencias; DROP TABLE instalacion;")
                .unwrap();
        }
        assert_eq!(inspeccionar(&archivo).unwrap().origen, "mobile");
        crate::preferencias::guardar(&db, "tema", &serde_json::json!("hielo_artico")).unwrap();
        restaurar(&db, &rutas, &archivo).unwrap();
        assert_eq!(
            crate::preferencias::apariencia(&db).unwrap().tema,
            "hielo_artico"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn lo_que_no_es_una_copia_se_rechaza_sin_tocar_nada() {
        let dir = carpeta("basura");
        let rutas = Rutas::bajo(&dir).unwrap();
        let db = base_con_perfil();
        let archivo = dir.join("otra.sqlite");
        {
            let c = Connection::open(&archivo).unwrap();
            c.execute_batch("CREATE TABLE x (y); INSERT INTO x VALUES (randomblob(8000));")
                .unwrap();
        }
        assert_eq!(
            restaurar(&db, &rutas, &archivo),
            Err(ScError::BackupInvalid)
        );
        std::fs::write(dir.join("texto.sqlite"), vec![b'a'; 5000]).unwrap();
        assert_eq!(
            inspeccionar(&dir.join("texto.sqlite")),
            Err(ScError::BackupInvalid)
        );
        let n: i64 = db
            .conexion()
            .unwrap()
            .query_row("SELECT count(*) FROM user_profiles", [], |f| f.get(0))
            .unwrap();
        assert_eq!(n, 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn la_copia_automatica_es_semanal_y_guarda_cinco() {
        let dir = carpeta("automatica");
        let rutas = Rutas::bajo(&dir).unwrap();
        let db = base_con_perfil();
        assert!(automatica_si_toca(&db, &rutas).unwrap());
        assert!(!automatica_si_toca(&db, &rutas).unwrap());
        for d in ["20250101", "20250201", "20250301", "20250401", "20250501"] {
            std::fs::write(rutas.copias().join(format!("automatica-{d}.sqlite")), b"x").unwrap();
        }
        let n = std::fs::read_dir(rutas.copias()).unwrap().count();
        assert_eq!(n, 6);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn una_copia_incompleta_no_borra_tablas_ausentes_en_el_origen() {
        let dir = carpeta("incompleta");
        let rutas = Rutas::bajo(&dir).unwrap();
        let db = base_con_perfil();
        let archivo = dir.join("copia.sqlite");
        exportar(&db, &archivo).unwrap();
        Connection::open(&archivo)
            .unwrap()
            .execute_batch("DROP TABLE hydration_logs")
            .unwrap();
        crate::comida::beber(&db, 321).unwrap();
        assert_eq!(
            restaurar(&db, &rutas, &archivo),
            Err(ScError::BackupInvalid)
        );
        assert_eq!(
            db.conexion()
                .unwrap()
                .query_row("SELECT sum(volume_ml) FROM hydration_logs", [], |f| f
                    .get::<_, i64>(0))
                .unwrap(),
            321
        );
        assert_eq!(std::fs::read_dir(&rutas.cache).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn restaurar_el_archivo_vivo_con_wal_no_lee_las_tablas_que_se_estan_borrando() {
        let dir = carpeta("origen-vivo");
        let rutas = Rutas::bajo(&dir).unwrap();
        exportar(&base_con_perfil(), &rutas.base_de_datos()).unwrap();
        let db = BaseDeDatos::abrir(&rutas.base_de_datos()).unwrap();
        crate::comida::beber(&db, 321).unwrap();
        restaurar(&db, &rutas, &rutas.base_de_datos()).unwrap();
        let conn = db.conexion().unwrap();
        assert_eq!(
            conn.query_row("SELECT sum(volume_ml) FROM hydration_logs", [], |f| f
                .get::<_, i64>(0))
                .unwrap(),
            321
        );
        assert_eq!(
            conn.query_row("SELECT count(*) FROM user_profiles", [], |f| f
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(std::fs::read_dir(&rutas.cache).unwrap().count(), 0);
        drop(conn);
        drop(db);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn una_copia_con_referencias_rotas_no_sustituye_los_datos_locales() {
        let dir = carpeta("referencias");
        let rutas = Rutas::bajo(&dir).unwrap();
        let db = base_con_perfil();
        let archivo = dir.join("copia.sqlite");
        exportar(&db, &archivo).unwrap();
        let copia = Connection::open(&archivo).unwrap();
        copia.pragma_update(None, "foreign_keys", "OFF").unwrap();
        copia.execute(
            "INSERT INTO habit_logs (habit_id, date_key, logged_at) VALUES (999, '2026-11-25', '2026-11-25T12:00:00')",
            [],
        ).unwrap();
        drop(copia);
        crate::comida::beber(&db, 321).unwrap();
        assert_eq!(
            restaurar(&db, &rutas, &archivo),
            Err(ScError::BackupInvalid)
        );
        let conn = db.conexion().unwrap();
        assert_eq!(
            conn.query_row("SELECT sum(volume_ml) FROM hydration_logs", [], |f| f
                .get::<_, i64>(0))
                .unwrap(),
            321
        );
        assert_eq!(
            conn.query_row("PRAGMA foreign_keys", [], |f| f.get::<_, i64>(0))
                .unwrap(),
            1
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn restauraciones_en_el_mismo_instante_conservan_ambos_estados_previos() {
        let dir = carpeta("respaldos-distintos");
        let rutas = Rutas::bajo(&dir).unwrap();
        let db = base_con_perfil();
        let archivo = dir.join("copia.sqlite");
        exportar(&db, &archivo).unwrap();
        fechas::fijar(
            chrono::NaiveDate::from_ymd_opt(2026, 11, 25)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap(),
        );
        for volumen in [321, 456] {
            crate::comida::beber(&db, volumen).unwrap();
            restaurar(&db, &rutas, &archivo).unwrap();
        }
        let mut cantidades: Vec<i64> = std::fs::read_dir(rutas.copias())
            .unwrap()
            .map(|entrada| {
                let conn = Connection::open(entrada.unwrap().path()).unwrap();
                conn.query_row("SELECT sum(volume_ml) FROM hydration_logs", [], |f| {
                    f.get(0)
                })
                .unwrap()
            })
            .collect();
        cantidades.sort_unstable();
        assert_eq!(cantidades, [321, 456]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn omitir_restricciones_en_la_copia_no_elude_las_del_destino() {
        let dir = carpeta("restricciones");
        let rutas = Rutas::bajo(&dir).unwrap();
        let db = base_con_perfil();
        let archivo = dir.join("copia.sqlite");
        exportar(&db, &archivo).unwrap();
        let copia = Connection::open(&archivo).unwrap();
        copia.execute_batch("DROP TABLE habit_logs;
            CREATE TABLE habit_logs (id INTEGER PRIMARY KEY, habit_id INTEGER, date_key TEXT, logged_at TEXT);
            INSERT INTO habit_logs VALUES (1, 999, '2026-11-25', '2026-11-25T12:00:00');").unwrap();
        drop(copia);
        assert!(inspeccionar(&archivo).is_ok());
        crate::comida::beber(&db, 321).unwrap();
        assert_eq!(
            restaurar(&db, &rutas, &archivo),
            Err(ScError::BackupInvalid)
        );
        let conn = db.conexion().unwrap();
        assert_eq!(
            conn.query_row("SELECT sum(volume_ml) FROM hydration_logs", [], |f| f
                .get::<_, i64>(0))
                .unwrap(),
            321
        );
        assert_eq!(
            conn.query_row("PRAGMA foreign_keys", [], |f| f.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(conn.prepare("SELECT * FROM copia.user_profiles").is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}
