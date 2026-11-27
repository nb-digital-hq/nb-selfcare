//! Capa de persistencia local.
//!
//! Contrato: ADR-004. Rust es el único dueño de la base. Toda conexión se
//! configura igual y esa configuración no es opcional: sin WAL las lecturas
//! bloquean a las escrituras y aparece el `database is locked` que la persona
//! percibe como una aplicación colgada. Es la misma capa que el resto de las
//! aplicaciones de escritorio de la suite.

pub mod migraciones;

use std::path::{Path, PathBuf};
use std::time::Duration;

use r2d2::{ManageConnection, Pool};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::error::{ScError, ScResult};

/// Tiempo que una conexión espera a que se libere un bloqueo antes de fallar.
pub(crate) const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// A partir de cuánto esperar por una conexión se apunta en el registro.
const ESPERA_QUE_SE_DICE: Duration = Duration::from_millis(50);

/// Conexiones simultáneas. Una aplicación de escritorio no necesita más: la
/// interfaz, los avisos y una tarea de fondo. Un pool grande solo aumenta la
/// contención.
const MAX_CONEXIONES: u32 = 4;

/// La configuración de cada conexión. `foreign_keys`, `synchronous` y
/// `busy_timeout` son de la conexión, no del archivo, así que se aplican en cada
/// una. `journal_mode` no: ver [`preparar_archivo`].
fn configurar(conn: &mut Connection) -> Result<(), rusqlite::Error> {
    conn.busy_timeout(BUSY_TIMEOUT)?;
    // Suficiente en WAL; `FULL` cuesta un fsync por escritura.
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    // SQLite las trae apagadas: sin esto las claves foráneas son decoración.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

/// Fija WAL en el archivo una sola vez, antes del pool. Hacerlo desde cada
/// conexión pone a varias a cambiar el modo del diario a la vez sobre una base
/// recién creada, y eso es `database is locked` en cada arranque en frío.
fn preparar_archivo(ruta: &Path) -> ScResult<()> {
    let conn = Connection::open(ruta).map_err(|_error| {
        tracing::error!("no se pudo abrir la base de datos");
        ScError::StorageFailure
    })?;
    conn.busy_timeout(BUSY_TIMEOUT)
        .map_err(|_| ScError::StorageFailure)?;
    let modo: String = conn
        .pragma_update_and_check(None, "journal_mode", "WAL", |fila| fila.get(0))
        .map_err(|_error| {
            tracing::error!("no se pudo activar el modo WAL");
            ScError::StorageFailure
        })?;
    if !modo.eq_ignore_ascii_case("wal") {
        tracing::error!(modo = %modo, "el modo WAL no quedó activo");
        return Err(ScError::StorageFailure);
    }
    Ok(())
}

/// Abre una transacción **de escritura** con `BEGIN IMMEDIATE`.
///
/// Nunca la diferida que da `Connection::transaction`: en WAL, una transacción
/// diferida que lee y después escribe recibe `SQLITE_BUSY` **de inmediato y sin
/// respetar `busy_timeout`** si otro escritor confirmó entre medias, y esa
/// escritura se pierde. Lo vigila
/// `scripts/check-transacciones.mjs`.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si no se puede abrir.
pub fn transaccion_de_escritura(conn: &mut Connection) -> ScResult<rusqlite::Transaction<'_>> {
    conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|_error| {
            tracing::error!("no se pudo abrir una transacción de escritura");
            ScError::StorageFailure
        })
}

/// Abre una lectura que ve una sola foto de la base: varias consultas seguidas
/// sin transacción ven cada una un momento distinto si alguien escribe a la vez.
/// Solo lee, así que no es la diferida que prohíbe [`transaccion_de_escritura`].
///
/// # Errors
///
/// [`ScError::StorageFailure`] si no se puede abrir.
pub fn lectura_coherente(conn: &mut Connection) -> ScResult<rusqlite::Transaction<'_>> {
    conn.transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
        .map_err(|_error| {
            tracing::error!("no se pudo abrir una lectura coherente");
            ScError::StorageFailure
        })
}

/// Convierte un error de SQLite en el del contrato, dejándolo en el registro
/// sin datos de la persona. No se registra el error original: puede incluir
/// rutas o nombres procedentes de una base importada.
pub(crate) fn fallo(_error: rusqlite::Error) -> ScError {
    tracing::error!("fallo de la base de datos");
    ScError::StorageFailure
}

/// Acceso a la base de datos local.
#[derive(Clone)]
pub struct BaseDeDatos {
    pool: Pool<GestorDeConexiones>,
}

/// Una conexión prestada debe volver sin transacciones, adjuntos ni claves desactivadas.
/// El gestor base no detecta esos estados al devolver una conexión al pool.
pub struct GestorDeConexiones(SqliteConnectionManager);

impl ManageConnection for GestorDeConexiones {
    type Connection = Connection;
    type Error = rusqlite::Error;

    fn connect(&self) -> Result<Connection, Self::Error> {
        self.0.connect()
    }

    fn is_valid(&self, conn: &mut Connection) -> Result<(), Self::Error> {
        if self.has_broken(conn) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        self.0.is_valid(conn)
    }

    fn has_broken(&self, conn: &mut Connection) -> bool {
        !conn.is_autocommit()
            || !matches!(
                conn.query_row("PRAGMA foreign_keys", [], |f| f.get::<_, i64>(0)),
                Ok(1)
            )
            || !matches!(
                conn.query_row(
                    "SELECT count(*) FROM pragma_database_list WHERE name NOT IN ('main', 'temp')",
                    [],
                    |f| f.get::<_, i64>(0)
                ),
                Ok(0)
            )
    }
}

impl BaseDeDatos {
    /// Abre la base en `ruta`, la configura y aplica las migraciones pendientes.
    ///
    /// # Errors
    ///
    /// - [`ScError::SchemaMismatch`] si la base es más nueva que esta versión.
    /// - [`ScError::StorageFailure`] si no se puede abrir o migrar.
    pub fn abrir(ruta: &Path) -> ScResult<Self> {
        preparar_archivo(ruta)?;
        let gestor = SqliteConnectionManager::file(ruta).with_init(configurar);
        Self::construir(gestor, Some(ruta.to_path_buf()))
    }

    /// Base en memoria compartida por el pool. Solo para pruebas.
    ///
    /// # Errors
    ///
    /// [`ScError::StorageFailure`] si no se puede inicializar.
    pub fn en_memoria() -> ScResult<Self> {
        // Un nombre único por base: `memory()` daría a cada conexión del pool su
        // propia base vacía, y una prueba con dos conexiones vería dos mundos.
        static CONTADOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = CONTADOR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let uri = format!(
            "file:sc-memoria-{}-{n}?mode=memory&cache=shared",
            std::process::id()
        );
        let gestor = SqliteConnectionManager::file(uri)
            .with_flags(
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                    | rusqlite::OpenFlags::SQLITE_OPEN_URI,
            )
            .with_init(configurar);
        Self::construir(gestor, None)
    }

    fn construir(gestor: SqliteConnectionManager, ruta: Option<PathBuf>) -> ScResult<Self> {
        let pool = Pool::builder()
            .max_size(MAX_CONEXIONES)
            // Sin conexiones ociosas al arrancar: crearlas en paralelo solo da
            // contención, y en escritorio no hay carga que lo justifique.
            .min_idle(Some(0))
            .connection_timeout(BUSY_TIMEOUT)
            .build(GestorDeConexiones(gestor))
            .map_err(|_error| {
                tracing::error!("no se pudo crear el pool de conexiones");
                ScError::StorageFailure
            })?;
        let db = Self { pool };
        let mut conn = db.conexion()?;
        migraciones::aplicar_pendientes(&mut conn, ruta.as_deref())?;
        drop(conn);
        Ok(db)
    }

    /// Toma una conexión del pool. Si hubo que esperar más de lo que se nota,
    /// queda en el registro con cuántas estaban en uso.
    ///
    /// # Errors
    ///
    /// [`ScError::StorageFailure`] si el pool está agotado.
    pub fn conexion(&self) -> ScResult<r2d2::PooledConnection<GestorDeConexiones>> {
        let inicio = std::time::Instant::now();
        let conexion = self.pool.get().map_err(|_error| {
            tracing::error!("no se pudo obtener una conexión del pool");
            ScError::StorageFailure
        })?;
        let espera = inicio.elapsed();
        if espera >= ESPERA_QUE_SE_DICE {
            let estado = self.pool.state();
            tracing::warn!(
                ms = espera.as_millis(),
                en_uso = estado.connections - estado.idle_connections,
                "se esperó por una conexión del pool"
            );
        }
        Ok(conexion)
    }

    /// Versión de esquema aplicada.
    ///
    /// # Errors
    ///
    /// [`ScError::StorageFailure`] si no se puede consultar.
    pub fn version_de_esquema(&self) -> ScResult<u32> {
        let conn = self.conexion()?;
        migraciones::version_actual(&conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporal(nombre: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sc-db-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("carpeta");
        dir
    }

    #[test]
    fn la_base_queda_en_wal_con_claves_foraneas() {
        let dir = temporal("wal");
        let db = BaseDeDatos::abrir(&dir.join("prueba.sqlite")).expect("abre");
        let conn = db.conexion().expect("conexión");
        let modo: String = conn
            .query_row("PRAGMA journal_mode", [], |f| f.get(0))
            .unwrap();
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |f| f.get(0))
            .unwrap();
        assert_eq!(modo.to_lowercase(), "wal");
        assert_eq!(fk, 1);
        drop(conn);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn la_base_en_memoria_es_la_misma_para_todas_las_conexiones() {
        let db = BaseDeDatos::en_memoria().expect("abre");
        let a = db.conexion().unwrap();
        a.execute_batch("CREATE TABLE prueba (x INTEGER); INSERT INTO prueba VALUES (1);")
            .unwrap();
        let b = db.conexion().unwrap();
        let n: i64 = b
            .query_row("SELECT count(*) FROM prueba", [], |f| f.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    /// La propiedad: ocho escritores concurrentes sin un solo
    /// `SQLITE_BUSY`.
    #[test]
    fn ocho_escritores_concurrentes_no_reciben_busy() {
        let dir = temporal("concurrencia");
        let db = BaseDeDatos::abrir(&dir.join("prueba.sqlite")).expect("abre");
        db.conexion()
            .unwrap()
            .execute_batch("CREATE TABLE contador (id INTEGER PRIMARY KEY, n INTEGER NOT NULL); INSERT INTO contador VALUES (1, 0);")
            .unwrap();
        let hilos: Vec<_> = (0..8)
            .map(|_| {
                let db = db.clone();
                std::thread::spawn(move || {
                    for _ in 0..50 {
                        let mut conn = db.conexion().expect("conexión");
                        let tx = transaccion_de_escritura(&mut conn).expect("transacción");
                        // Leer y después escribir: exactamente lo que falla en diferido.
                        let n: i64 = tx
                            .query_row("SELECT n FROM contador WHERE id = 1", [], |f| f.get(0))
                            .unwrap();
                        tx.execute("UPDATE contador SET n = ?1 WHERE id = 1", [n + 1])
                            .unwrap();
                        tx.commit().expect("confirma sin BUSY");
                    }
                })
            })
            .collect();
        for h in hilos {
            h.join().expect("ningún hilo falla");
        }
        let n: i64 = db
            .conexion()
            .unwrap()
            .query_row("SELECT n FROM contador", [], |f| f.get(0))
            .unwrap();
        assert_eq!(n, 400, "ninguna escritura se perdió");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn el_pool_descarta_conexiones_con_estado_inseguro_sin_perder_datos_confirmados() {
        let dir = temporal("descartar-conexion");
        let db = BaseDeDatos::abrir(&dir.join("prueba.sqlite")).unwrap();
        db.conexion()
            .unwrap()
            .execute_batch("CREATE TABLE prueba (valor INTEGER); INSERT INTO prueba VALUES (1);")
            .unwrap();
        for alteracion in [
            "PRAGMA foreign_keys = OFF",
            "BEGIN IMMEDIATE; UPDATE prueba SET valor = 99;",
            "ATTACH DATABASE ':memory:' AS inesperada",
        ] {
            let conn = db.conexion().unwrap();
            conn.execute_batch(alteracion).unwrap();
            drop(conn);
            let conn = db.conexion().unwrap();
            assert!(conn.is_autocommit());
            assert_eq!(
                conn.query_row("PRAGMA foreign_keys", [], |f| f.get::<_, i64>(0))
                    .unwrap(),
                1
            );
            assert_eq!(
                conn.query_row("SELECT valor FROM prueba", [], |f| f.get::<_, i64>(0))
                    .unwrap(),
                1
            );
            assert_eq!(
                conn.query_row(
                    "SELECT count(*) FROM pragma_database_list WHERE name = 'inesperada'",
                    [],
                    |f| f.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
        }
        drop(db);
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod privacidad_del_error {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct Captura(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Captura {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn un_error_sqlite_con_ruta_privada_no_la_registra() {
        let salida = Arc::new(Mutex::new(Vec::new()));
        let captura = Arc::clone(&salida);
        let registro = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(move || Captura(Arc::clone(&captura)))
            .finish();
        tracing::subscriber::with_default(registro, || {
            assert_eq!(
                fallo(rusqlite::Error::InvalidPath(PathBuf::from(
                    "/privado/detalle_salud.sqlite"
                ))),
                ScError::StorageFailure
            );
        });
        let texto = String::from_utf8(salida.lock().unwrap().clone()).unwrap();
        assert!(texto.contains("fallo de la base"));
        assert!(!texto.contains("privado"));
        assert!(!texto.contains("detalle_salud"));
    }
}
