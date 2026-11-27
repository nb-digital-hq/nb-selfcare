use super::*;
fn base_con_perfil() -> crate::db::BaseDeDatos {
    let db = crate::perfil::pruebas::base_con_perfil();
    crate::comida::sembrar(&db).unwrap();
    db
}

fn contar(db: &crate::db::BaseDeDatos, sql: &str) -> i64 {
    db.conexion()
        .unwrap()
        .query_row(sql, [], |f| f.get(0))
        .unwrap()
}

#[test]
fn la_especificacion_cubre_tablas_reales_y_entidades_unicas() {
    let db = base_con_perfil();
    let conn = db.conexion().unwrap();
    for t in tablas() {
        for c in &t.columnas {
            let existe: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
                    [&t.tabla, &c.nombre],
                    |f| f.get(0),
                )
                .unwrap();
            assert!(existe, "{}.{}", t.tabla, c.nombre);
        }
        assert!(por_entidad(t.entidad).is_some_and(|x| x.tabla == t.tabla));
    }
}

#[test]
fn cada_escritura_local_tiene_identidad_revision_y_diario_en_su_transaccion() {
    let db = base_con_perfil();
    crate::comida::beber(&db, 250).unwrap();
    assert_eq!(
        contar(
            &db,
            "SELECT count(*) FROM sc_sync_identidades WHERE tabla = 'hydration_logs'"
        ),
        1
    );
    assert_eq!(
        contar(
            &db,
            "SELECT r.revision FROM sc_sync_revisiones r JOIN sc_sync_identidades i USING (registro)
             WHERE i.tabla = 'hydration_logs'"
        ),
        1
    );
    let antes = contar(&db, "SELECT max(seq) FROM sc_sync_diario");
    db.conexion()
        .unwrap()
        .execute("UPDATE hydration_logs SET volume_ml = 300", [])
        .unwrap();
    // Una edición reubica su entrada al final del diario sin duplicarla.
    assert!(contar(&db, "SELECT max(seq) FROM sc_sync_diario") > antes);
    assert_eq!(
        contar(
            &db,
            "SELECT count(*) FROM sc_sync_diario d JOIN sc_sync_identidades i USING (registro)
             WHERE i.tabla = 'hydration_logs'"
        ),
        1
    );
    db.conexion()
        .unwrap()
        .execute("DELETE FROM hydration_logs", [])
        .unwrap();
    assert_eq!(
        contar(
            &db,
            "SELECT eliminado FROM sc_sync_identidades WHERE tabla = 'hydration_logs'"
        ),
        1
    );
}

#[test]
fn el_catalogo_sembrado_no_entra_al_diario() {
    let db = base_con_perfil();
    assert!(
        contar(
            &db,
            "SELECT count(*) FROM food_items WHERE source = 'usda_seed'"
        ) > 0
    );
    assert_eq!(
        contar(
            &db,
            "SELECT count(*) FROM sc_sync_diario d JOIN sc_sync_identidades i USING (registro)
             JOIN food_items f ON i.tabla = 'food_items' AND i.clave = CAST(f.id AS TEXT)
             WHERE f.source = 'usda_seed'"
        ),
        0
    );
}

#[test]
fn una_fila_se_lee_con_referencias_estables_y_sin_columnas_locales() {
    let db = base_con_perfil();
    let alimento: i64 = db
        .conexion()
        .unwrap()
        .query_row("SELECT id FROM food_items ORDER BY id LIMIT 1", [], |f| {
            f.get(0)
        })
        .unwrap();
    crate::comida::registrar(&db, alimento, 120.0, "lunch").unwrap();
    let mut conn = db.conexion().unwrap();
    let tx = crate::db::transaccion_de_escritura(&mut conn).unwrap();
    let clave: i64 = tx
        .query_row("SELECT id FROM food_logs", [], |f| f.get(0))
        .unwrap();
    let datos = leer(&tx, por_tabla("food_logs").unwrap(), &clave.to_string())
        .unwrap()
        .unwrap();
    let referencia = datos["food_item_id"].as_str().unwrap();
    assert_eq!(referencia.len(), 32);
    assert!(!datos.contains_key("id"));
    let perfil = tx
        .query_row("SELECT id FROM user_profiles", [], |f| f.get::<_, i64>(0))
        .unwrap();
    let p = leer(
        &tx,
        por_tabla("user_profiles").unwrap(),
        &perfil.to_string(),
    )
    .unwrap()
    .unwrap();
    assert!(!p.contains_key("avatar_path") && !p.contains_key("current_adaptive_tdee_kcal"));
}
