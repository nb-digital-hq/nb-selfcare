//! Una copia real de NB SelfCare Mobile 1.0 (el perfil ficticio de su edición
//! de demostración) se restaura en el escritorio y se lee entera: Hoy, Comida,
//! Cuerpo, Cuidado, Ayuno y Nibi. Es la prueba de que los dos comparten el
//! esquema de datos (ADR-004).

use nb_selfcare_lib::{ayuno, comida, copia, cuerpo, cuidado, db::BaseDeDatos, hoy, paths::Rutas};

#[test]
fn una_copia_del_telefono_se_restaura_y_se_lee_entera() {
    let dir = std::env::temp_dir().join(format!("sc-copia-movil-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let rutas = Rutas::bajo(&dir).unwrap();
    let db = BaseDeDatos::abrir(&rutas.base_de_datos()).unwrap();
    let original = dir.join("movil.sqlite");
    {
        let conexion = rusqlite::Connection::open(&original).unwrap();
        conexion
            .execute_batch(include_str!("fixtures/copia-movil-demo.sql"))
            .unwrap();
    }

    let i = copia::restaurar(&db, &rutas, &original).unwrap();
    assert_eq!(
        (i.origen.as_str(), i.esquema, i.perfil.as_deref()),
        ("mobile", 8, Some("Alex"))
    );

    let h = hoy::hoy(&db).unwrap();
    assert_eq!(h.perfil.as_ref().map(|p| p.nombre.as_str()), Some("Alex"));
    let c = cuerpo::resumen(&db).unwrap();
    assert!(c.historial.len() >= 20, "los pesajes del teléfono");
    assert!(c.historial.iter().all(|p| !p.dia.is_empty()));
    let r = cuidado::resumen(&db).unwrap();
    assert!(!r.tratamientos.is_empty());
    assert!(!r.habitos.is_empty());
    ayuno::estado(&db).unwrap();
    comida::dia(&db, "2026-11-20").unwrap();
    let conn = db.conexion().unwrap();
    let comidas: i64 = conn
        .query_row("SELECT count(*) FROM food_logs", [], |f| f.get(0))
        .unwrap();
    assert!(comidas > 0);
    // Una copia del esquema 8 recibe identidades nuevas: cada comida podrá sincronizarse.
    let identidades: i64 = conn
        .query_row(
            "SELECT count(*) FROM sc_sync_identidades WHERE tabla = 'food_logs'",
            [],
            |f| f.get(0),
        )
        .unwrap();
    assert_eq!(identidades, comidas);
    assert!(
        !comida::buscar(&conn, "avena", 5).unwrap().is_empty(),
        "el índice de búsqueda se rehízo"
    );
    drop(conn);

    // Y de vuelta: el escritorio exporta el esquema 9 compartido con SelfCare Mobile.
    let vuelta = dir.join("vuelta.sqlite");
    copia::exportar(&db, &vuelta).unwrap();
    let j = copia::inspeccionar(&vuelta).unwrap();
    assert_eq!((j.esquema, j.perfil.as_deref()), (9, Some("Alex")));
    let _ = std::fs::remove_dir_all(dir);
}
