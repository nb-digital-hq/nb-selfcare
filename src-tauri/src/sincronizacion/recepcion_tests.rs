use super::*;
use crate::sincronizacion::contrato::Entidad;
use std::sync::{Arc, Barrier};
use std::time::Duration;

fn id(c: char) -> Identificador {
    Identificador::try_from(c.to_string().repeat(32)).unwrap()
}
fn cambio(operacion: char, base: u64, volumen: i64) -> Cambio {
    serde_json::from_value(serde_json::json!({"id":operacion.to_string().repeat(32),
        "registro":"a".repeat(32),"entidad":"agua","base":base,"operacion":"guardar",
        "datos":{"volumen":volumen}}))
    .unwrap()
}
fn preparar(conn: &Connection) {
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    conn.execute_batch(include_str!(
        "../../tests/fixtures/sincronizacion/recepcion-v1.sql"
    ))
    .unwrap();
    conn.execute(
        "INSERT INTO sc_sync_contexto VALUES(1,?1)",
        [id('e').exponer()],
    )
    .unwrap();
    conn.execute_batch(
        "CREATE TABLE agua(id TEXT PRIMARY KEY, volumen INTEGER NOT NULL);
        CREATE TABLE efectos(cantidad INTEGER NOT NULL); INSERT INTO efectos VALUES(0);",
    )
    .unwrap();
}
fn memoria() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    preparar(&conn);
    conn
}
struct Agua {
    fallar_despues: bool,
}
impl PropietarioDeRegistro for Agua {
    fn comprobar(
        &self,
        tx: &Transaction<'_>,
        cambio: &Cambio,
    ) -> ScResult<Option<Map<String, Value>>> {
        if cambio.entidad != Entidad::Agua {
            return Err(ScError::InvalidInput);
        }
        if cambio.operacion == Operacion::Guardar {
            let volumen = cambio
                .datos
                .as_ref()
                .and_then(|d| d.get("volumen"))
                .and_then(Value::as_i64);
            if !volumen.is_some_and(|v| (1..=10000).contains(&v)) {
                return Err(ScError::InvalidInput);
            }
        }
        let volumen: Option<i64> = tx
            .query_row(
                "SELECT volumen FROM agua WHERE id=?1",
                [cambio.registro.exponer()],
                |f| f.get(0),
            )
            .optional()
            .map_err(fallo)?;
        Ok(volumen.map(|v| {
            serde_json::json!({"volumen":v})
                .as_object()
                .unwrap()
                .clone()
        }))
    }
    fn aplicar(&self, tx: &Transaction<'_>, cambio: &Cambio) -> ScResult<()> {
        match cambio.operacion {
            Operacion::Guardar => {
                tx.execute("INSERT INTO agua VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET volumen=excluded.volumen",
                    params![cambio.registro.exponer(), cambio.datos.as_ref().unwrap()["volumen"].as_i64().unwrap()]).map_err(fallo)?;
            }
            Operacion::Eliminar => {
                tx.execute("DELETE FROM agua WHERE id=?1", [cambio.registro.exponer()])
                    .map_err(fallo)?;
            }
        }
        tx.execute("UPDATE efectos SET cantidad=cantidad+1", [])
            .map_err(fallo)?;
        if self.fallar_despues {
            return Err(ScError::StorageFailure);
        }
        Ok(())
    }
}
const AGUA: Agua = Agua {
    fallar_despues: false,
};
fn entero(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |f| f.get(0)).unwrap()
}

#[test]
fn el_recibo_persistido_sobrevive_al_reinicio_y_no_repite_efectos() {
    let carpeta = std::env::temp_dir().join(format!(
        "nb-selfcare-recepcion-{}",
        Identificador::generar().unwrap().exponer()
    ));
    std::fs::create_dir(&carpeta).unwrap();
    let ruta = carpeta.join("prueba.sqlite");
    {
        let mut conn = Connection::open(&ruta).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        preparar(&conn);
        assert_eq!(
            recibir(&mut conn, &id('e'), &id('1'), &cambio('2', 0, 250), &AGUA).unwrap(),
            ResultadoRecepcion::Aplicado { revision: 1 }
        );
    }
    {
        let mut conn = Connection::open(&ruta).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        assert_eq!(
            recibir(
                &mut conn,
                &id('e'),
                &id('1'),
                &cambio('2', 0, 250),
                &Agua {
                    fallar_despues: true
                }
            )
            .unwrap(),
            ResultadoRecepcion::Aplicado { revision: 1 }
        );
        assert_eq!(entero(&conn, "SELECT cantidad FROM efectos"), 1);
    }
    std::fs::remove_dir_all(carpeta).unwrap();
}

#[test]
fn no_se_crea_infraestructura_silenciosa_en_una_base_sin_migrar() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    assert!(matches!(
        recibir(&mut conn, &id('e'), &id('1'), &cambio('2', 0, 250), &AGUA),
        Err(ScError::StorageFailure)
    ));
    assert_eq!(
        entero(
            &conn,
            "SELECT count(*) FROM sqlite_master WHERE name LIKE 'sc_sync_%'"
        ),
        0
    );
    assert!(conn.is_autocommit());
}

#[test]
fn una_conexion_sin_claves_foraneas_no_puede_aplicar_operaciones() {
    let mut conn = memoria();
    conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
    assert!(matches!(
        recibir(&mut conn, &id('e'), &id('1'), &cambio('2', 0, 250), &AGUA),
        Err(ScError::StorageFailure)
    ));
    assert_eq!(entero(&conn, "SELECT cantidad FROM efectos"), 0);
}
#[test]
fn reutilizar_una_operacion_con_otro_contenido_no_es_un_reintento() {
    let mut conn = memoria();
    recibir(&mut conn, &id('e'), &id('1'), &cambio('2', 0, 250), &AGUA).unwrap();
    assert_eq!(
        recibir(&mut conn, &id('e'), &id('1'), &cambio('2', 0, 500), &AGUA).unwrap(),
        ResultadoRecepcion::OperacionAlterada
    );
    assert_eq!(entero(&conn, "SELECT volumen FROM agua"), 250);
    assert_eq!(entero(&conn, "SELECT cantidad FROM efectos"), 1);
}
#[test]
fn una_edicion_concurrente_conserva_ambas_propuestas_y_su_recibo() {
    let mut conn = memoria();
    recibir(&mut conn, &id('e'), &id('1'), &cambio('2', 0, 250), &AGUA).unwrap();
    let desfasado = cambio('3', 0, 500);
    for _ in 0..2 {
        assert_eq!(
            recibir(&mut conn, &id('e'), &id('1'), &desfasado, &AGUA).unwrap(),
            ResultadoRecepcion::Conflicto { revision: 1 }
        );
    }
    let (actual, propuesta): (String, String) = conn
        .query_row("SELECT actual,propuesta FROM sc_sync_conflictos", [], |f| {
            Ok((f.get(0)?, f.get(1)?))
        })
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&actual).unwrap()["volumen"],
        250
    );
    assert_eq!(
        serde_json::from_str::<Value>(&propuesta).unwrap()["datos"]["volumen"],
        500
    );
    assert_eq!(entero(&conn, "SELECT cantidad FROM efectos"), 1);
    conn.execute("DELETE FROM sc_sync_conflictos", []).unwrap();
    assert!(matches!(
        recibir(&mut conn, &id('e'), &id('1'), &desfasado, &AGUA),
        Err(ScError::StorageFailure)
    ));
}
#[test]
fn un_fallo_del_dominio_o_del_recibo_revierte_todas_las_escrituras() {
    for fallo_recibo in [false, true] {
        let mut conn = memoria();
        if fallo_recibo {
            conn.execute_batch("CREATE TRIGGER rechazar BEFORE INSERT ON sc_sync_recibos BEGIN SELECT RAISE(ABORT,'fallo sintetico'); END;").unwrap();
        }
        assert!(recibir(
            &mut conn,
            &id('e'),
            &id('1'),
            &cambio('2', 0, 250),
            &Agua {
                fallar_despues: !fallo_recibo
            }
        )
        .is_err());
        for tabla in ["agua", "sc_sync_revisiones", "sc_sync_recibos"] {
            assert_eq!(entero(&conn, &format!("SELECT count(*) FROM {tabla}")), 0);
        }
        assert_eq!(entero(&conn, "SELECT cantidad FROM efectos"), 0);
        assert!(conn.is_autocommit());
    }
}
#[test]
fn se_validan_los_datos_incluso_si_la_revision_produciria_conflicto() {
    let mut conn = memoria();
    assert!(matches!(
        recibir(&mut conn, &id('e'), &id('1'), &cambio('2', 3, 0), &AGUA),
        Err(ScError::InvalidInput)
    ));
    assert_eq!(entero(&conn, "SELECT count(*) FROM sc_sync_recibos"), 0);
}
#[test]
fn una_epoca_antigua_no_reintroduce_operaciones_y_las_bajas_conservan_revision() {
    let mut conn = memoria();
    recibir(&mut conn, &id('e'), &id('1'), &cambio('2', 0, 250), &AGUA).unwrap();
    let mut baja = cambio('3', 1, 0);
    baja.operacion = Operacion::Eliminar;
    baja.datos = None;
    assert_eq!(
        recibir(&mut conn, &id('e'), &id('1'), &baja, &AGUA).unwrap(),
        ResultadoRecepcion::Aplicado { revision: 2 }
    );
    assert_eq!(entero(&conn, "SELECT count(*) FROM agua"), 0);
    assert_eq!(entero(&conn, "SELECT revision FROM sc_sync_revisiones"), 2);
    assert_eq!(
        recibir(&mut conn, &id('e'), &id('1'), &cambio('4', 1, 500), &AGUA).unwrap(),
        ResultadoRecepcion::Conflicto { revision: 2 }
    );
    assert_eq!(
        entero(
            &conn,
            "SELECT count(*) FROM sc_sync_conflictos WHERE actual IS NULL"
        ),
        1
    );
    conn.execute("UPDATE sc_sync_contexto SET epoca=?1", [id('f').exponer()])
        .unwrap();
    for epoca in [id('e'), id('f')] {
        assert_eq!(
            recibir(&mut conn, &epoca, &id('1'), &cambio('2', 0, 250), &AGUA).unwrap(),
            ResultadoRecepcion::EpocaCambiada
        );
    }
    assert_eq!(entero(&conn, "SELECT count(*) FROM agua"), 0);
}
#[test]
fn el_limite_de_revision_no_desborda_ni_causa_una_escritura_parcial() {
    let mut conn = memoria();
    conn.execute(
        "INSERT INTO sc_sync_revisiones VALUES('agua',?1,?2)",
        params![id('a').exponer(), i64::try_from(MAX_REVISION).unwrap()],
    )
    .unwrap();
    assert_eq!(
        recibir(
            &mut conn,
            &id('e'),
            &id('1'),
            &cambio('2', MAX_REVISION, 250),
            &AGUA
        )
        .unwrap(),
        ResultadoRecepcion::RevisionAgotada
    );
    assert_eq!(entero(&conn, "SELECT cantidad FROM efectos"), 0);
    assert_eq!(entero(&conn, "SELECT count(*) FROM sc_sync_recibos"), 0);
}
#[test]
fn dos_conexiones_no_pueden_confirmar_la_misma_revision_base() {
    let carpeta = std::env::temp_dir().join(format!(
        "nb-selfcare-recepcion-{}",
        Identificador::generar().unwrap().exponer()
    ));
    std::fs::create_dir(&carpeta).unwrap();
    let ruta = carpeta.join("prueba.sqlite");
    {
        let conn = Connection::open(&ruta).unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        preparar(&conn);
    }
    let barrera = Arc::new(Barrier::new(2));
    let hilos: Vec<_> = ['2', '3']
        .into_iter()
        .map(|op| {
            let ruta = ruta.clone();
            let barrera = barrera.clone();
            std::thread::spawn(move || {
                let mut conn = Connection::open(ruta).unwrap();
                conn.pragma_update(None, "foreign_keys", "ON").unwrap();
                conn.busy_timeout(Duration::from_secs(5)).unwrap();
                barrera.wait();
                recibir(&mut conn, &id('e'), &id('1'), &cambio(op, 0, 250), &AGUA).unwrap()
            })
        })
        .collect();
    let resultados: Vec<_> = hilos.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(
        resultados
            .iter()
            .filter(|r| matches!(r, ResultadoRecepcion::Aplicado { .. }))
            .count(),
        1
    );
    assert_eq!(
        resultados
            .iter()
            .filter(|r| matches!(r, ResultadoRecepcion::Conflicto { .. }))
            .count(),
        1
    );
    {
        let conn = Connection::open(&ruta).unwrap();
        assert_eq!(entero(&conn, "SELECT cantidad FROM efectos"), 1);
    }
    std::fs::remove_dir_all(carpeta).unwrap();
}
