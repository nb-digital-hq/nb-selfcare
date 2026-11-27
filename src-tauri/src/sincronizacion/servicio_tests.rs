use super::*;
use crate::db::BaseDeDatos;
use crate::sincronizacion::contrato::Entidad;
fn base_con_perfil() -> crate::db::BaseDeDatos {
    let db = crate::perfil::pruebas::base_con_perfil();
    crate::comida::sembrar(&db).unwrap();
    db
}

fn id(c: char) -> Identificador {
    Identificador::try_from(c.to_string().repeat(32)).unwrap()
}

fn contar(db: &BaseDeDatos, sql: &str) -> i64 {
    db.conexion()
        .unwrap()
        .query_row(sql, [], |f| f.get(0))
        .unwrap()
}

/// Lo que un teléfono enviaría: sus cambios pendientes con la base compartida.
fn pendientes(telefono: &BaseDeDatos, nodo: &Identificador) -> Option<Lote> {
    let (cambios, _, _) =
        pagina(&mut telefono.conexion().unwrap(), &id('f'), 0, MAX_CAMBIOS).unwrap();
    let cambios: Vec<Cambio> = cambios
        .into_iter()
        .filter(|c| {
            let conn = telefono.conexion().unwrap();
            let (rev, compartida): (i64, i64) = conn
                .query_row(
                    "SELECT coalesce(r.revision, 1), i.compartida FROM sc_sync_identidades i
                     LEFT JOIN sc_sync_revisiones r USING (registro) WHERE i.registro = ?1",
                    [c.registro.exponer()],
                    |f| Ok((f.get(0)?, f.get(1)?)),
                )
                .unwrap();
            rev > compartida
        })
        .collect();
    (!cambios.is_empty()).then(|| Lote {
        version: 1,
        nodo: nodo.clone(),
        lote: Identificador::generar().unwrap(),
        cambios,
    })
}

/// Un ciclo completo: el teléfono envía, el escritorio responde y el teléfono aplica.
fn sincronizar(
    telefono: &BaseDeDatos,
    pc: &BaseDeDatos,
    nodo: &Identificador,
    cursor: u64,
) -> Respuesta {
    let lote = pendientes(telefono, nodo);
    let enviados: Vec<Cambio> = lote.as_ref().map(|l| l.cambios.clone()).unwrap_or_default();
    let r = atender(
        &mut pc.conexion().unwrap(),
        nodo,
        Peticion {
            version: 1,
            epoca: None,
            cursor,
            lote,
        },
    )
    .unwrap();
    let mut conn = telefono.conexion().unwrap();
    let tx = crate::db::transaccion_de_escritura(&mut conn).unwrap();
    for (res, c) in r.resultados.iter().zip(&enviados) {
        if res.estado == Estado::Aplicado {
            tx.execute(
                "UPDATE sc_sync_identidades SET compartida = ?1 WHERE registro = ?2",
                rusqlite::params![res.revision as i64, c.registro.exponer()],
            )
            .unwrap();
            tx.execute(
                "UPDATE sc_sync_revisiones SET revision = ?1 WHERE registro = ?2",
                rusqlite::params![res.revision as i64, c.registro.exponer()],
            )
            .unwrap();
        }
    }
    for c in &r.cambios {
        dominio::escribir(&tx, id('e').exponer(), c, c.base + 1).unwrap();
        tx.execute(
            "INSERT INTO sc_sync_revisiones (entidad, registro, revision) VALUES (?1, ?2, ?3)
             ON CONFLICT DO UPDATE SET revision = excluded.revision",
            rusqlite::params![
                entidad_texto(c.entidad).unwrap(),
                c.registro.exponer(),
                (c.base + 1) as i64
            ],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    r
}

fn semilla(db: &BaseDeDatos) -> i64 {
    db.conexion()
        .unwrap()
        .query_row(
            "SELECT id FROM food_items WHERE source = 'usda_seed' ORDER BY id LIMIT 1",
            [],
            |f| f.get(0),
        )
        .unwrap()
}

#[test]
fn los_registros_del_telefono_llegan_una_vez_y_vuelven_sin_eco() {
    let (tel, pc, nodo) = (base_con_perfil(), base_con_perfil(), id('a'));
    crate::comida::beber(&tel, 500).unwrap();
    crate::comida::registrar(&tel, semilla(&tel), 150.0, "lunch").unwrap();
    let h = crate::cuidado::crear_habito(&tel, "Leer", "spark", 127).unwrap();
    crate::cuidado::marcar_habito(&tel, h, true).unwrap();
    let r = sincronizar(&tel, &pc, &nodo, 0);
    assert!(
        r.resultados.iter().all(|x| x.estado == Estado::Aplicado),
        "{:?}",
        r.resultados.iter().map(|x| x.estado).collect::<Vec<_>>()
    );
    assert_eq!(
        contar(&pc, "SELECT sum(volume_ml) FROM hydration_logs"),
        500
    );
    assert_eq!(contar(&pc, "SELECT count(*) FROM food_logs"), 1);
    assert_eq!(contar(&pc, "SELECT count(*) FROM habit_logs"), 1);
    // El perfil y Nibi eran equivalentes y sin compartir: se vinculan, no se duplican.
    assert_eq!(contar(&pc, "SELECT count(*) FROM user_profiles"), 1);
    assert_eq!(contar(&pc, "SELECT count(*) FROM pet_status"), 1);
    // El alimento sembrado viajó como dependencia y se reconoció en el catálogo propio.
    assert_eq!(
        contar(&pc, "SELECT count(*) FROM food_items"),
        contar(&tel, "SELECT count(*) FROM food_items")
    );
    // Lo aplicado desde el teléfono no se le devuelve.
    assert!(r.cambios.iter().all(|c| c.entidad != Entidad::Agua));
    let r2 = sincronizar(&tel, &pc, &nodo, r.cursor);
    assert!(
        r2.resultados.is_empty(),
        "{:?} {:?}",
        r.cambios.iter().map(|c| c.entidad).collect::<Vec<_>>(),
        r2.resultados.iter().map(|x| x.estado).collect::<Vec<_>>()
    );
    assert_eq!(
        contar(&pc, "SELECT sum(volume_ml) FROM hydration_logs"),
        500
    );
}

#[test]
fn un_reintento_no_duplica_y_una_baja_se_propaga() {
    let (tel, pc, nodo) = (base_con_perfil(), base_con_perfil(), id('a'));
    crate::comida::beber(&tel, 300).unwrap();
    let lote = pendientes(&tel, &nodo).unwrap();
    for _ in 0..2 {
        atender(
            &mut pc.conexion().unwrap(),
            &nodo,
            Peticion {
                version: 1,
                epoca: None,
                cursor: 0,
                lote: Some(lote.clone()),
            },
        )
        .unwrap();
    }
    assert_eq!(contar(&pc, "SELECT count(*) FROM hydration_logs"), 1);
    sincronizar(&tel, &pc, &nodo, 0);
    tel.conexion()
        .unwrap()
        .execute("DELETE FROM hydration_logs", [])
        .unwrap();
    sincronizar(&tel, &pc, &nodo, 0);
    assert_eq!(contar(&pc, "SELECT count(*) FROM hydration_logs"), 0);
}

#[test]
fn una_edicion_concurrente_queda_para_revisar_y_se_resuelve_sin_perder_nada() {
    let (tel, pc, nodo) = (base_con_perfil(), base_con_perfil(), id('a'));
    crate::comida::beber(&tel, 300).unwrap();
    let r = sincronizar(&tel, &pc, &nodo, 0);
    tel.conexion()
        .unwrap()
        .execute("UPDATE hydration_logs SET volume_ml = 400", [])
        .unwrap();
    pc.conexion()
        .unwrap()
        .execute("UPDATE hydration_logs SET volume_ml = 450", [])
        .unwrap();
    let r2 = sincronizar(&tel, &pc, &nodo, r.cursor);
    let estados: Vec<Estado> = r2.resultados.iter().map(|x| x.estado).collect();
    assert!(estados.contains(&Estado::Conflicto));
    assert_eq!(contar(&pc, "SELECT volume_ml FROM hydration_logs"), 450);
    let lista = conflictos(&mut pc.conexion().unwrap(), 10).unwrap();
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].campos.len(), 1);
    assert_eq!(lista[0].campos[0].telefono.as_deref(), Some("400"));
    assert_eq!(
        estado(&mut pc.conexion().unwrap())
            .unwrap()
            .pendientes_de_revisar,
        1
    );
    resolver(
        &mut pc.conexion().unwrap(),
        &lista[0].nodo,
        &lista[0].operacion,
        true,
    )
    .unwrap();
    assert_eq!(contar(&pc, "SELECT volume_ml FROM hydration_logs"), 400);
    assert!(conflictos(&mut pc.conexion().unwrap(), 10)
        .unwrap()
        .is_empty());
    // La decisión vuelve al teléfono como versión nueva.
    let r3 = sincronizar(&tel, &pc, &nodo, r2.cursor);
    assert!(r3.cambios.iter().any(|c| c.entidad == Entidad::Agua));
}

#[test]
fn una_union_inicial_con_datos_distintos_no_sustituye_en_silencio() {
    let (tel, pc, nodo) = (base_con_perfil(), base_con_perfil(), id('a'));
    tel.conexion()
        .unwrap()
        .execute("UPDATE user_profiles SET name = 'Sam'", [])
        .unwrap();
    let r = sincronizar(&tel, &pc, &nodo, 0);
    assert!(r.resultados.iter().any(|x| x.estado == Estado::Conflicto));
    assert_eq!(contar(&pc, "SELECT count(*) FROM user_profiles"), 1);
    let nombre: String = pc
        .conexion()
        .unwrap()
        .query_row("SELECT name FROM user_profiles", [], |f| f.get(0))
        .unwrap();
    assert_eq!(nombre, "Alex");
}

#[test]
fn los_datos_del_par_se_validan_contra_el_contrato() {
    let (pc, nodo) = (base_con_perfil(), id('a'));
    let mut datos = serde_json::Map::new();
    datos.insert("date_key".into(), "2026-11-20".into());
    datos.insert("logged_at".into(), "2026-11-20T10:00:00".into());
    datos.insert("volume_ml".into(), serde_json::json!("DROP TABLE"));
    let cambio = Cambio {
        id: id('1'),
        registro: id('2'),
        entidad: Entidad::Agua,
        base: 0,
        operacion: Operacion::Guardar,
        datos: Some(datos.clone()),
    };
    let mut otro = cambio.clone();
    otro.id = id('3');
    otro.registro = id('4');
    let mut d = datos;
    d.insert("volume_ml".into(), serde_json::json!(250));
    d.insert("id".into(), serde_json::json!(1));
    otro.datos = Some(d);
    let r = atender(
        &mut pc.conexion().unwrap(),
        &nodo,
        Peticion {
            version: 1,
            epoca: None,
            cursor: 0,
            lote: Some(Lote {
                version: 1,
                nodo: nodo.clone(),
                lote: id('5'),
                cambios: vec![cambio, otro],
            }),
        },
    )
    .unwrap();
    assert!(r.resultados.iter().all(|x| x.estado == Estado::Rechazado));
    assert_eq!(contar(&pc, "SELECT count(*) FROM hydration_logs"), 0);
}

#[test]
fn las_monedas_se_proyectan_una_vez_y_las_tomas_no_premian() {
    let (tel, pc, nodo) = (base_con_perfil(), base_con_perfil(), id('a'));
    let h = crate::cuidado::crear_habito(&tel, "Caminar", "spark", 127).unwrap();
    crate::cuidado::marcar_habito(&tel, h, true).unwrap();
    sincronizar(&tel, &pc, &nodo, 0);
    sincronizar(&tel, &pc, &nodo, 0);
    assert_eq!(
        contar(&pc, "SELECT coins_balance FROM pet_status"),
        contar(&tel, "SELECT coins_balance FROM pet_status")
    );
}

#[test]
fn una_restauracion_cambia_la_epoca_y_obliga_a_releer() {
    let (pc, nodo) = (base_con_perfil(), id('a'));
    let r = atender(
        &mut pc.conexion().unwrap(),
        &nodo,
        Peticion {
            version: 1,
            epoca: None,
            cursor: 0,
            lote: None,
        },
    )
    .unwrap();
    assert!(!r.reiniciar);
    pc.conexion()
        .unwrap()
        .execute(
            "UPDATE sc_sync_contexto SET epoca = lower(hex(randomblob(16)))",
            [],
        )
        .unwrap();
    let r2 = atender(
        &mut pc.conexion().unwrap(),
        &nodo,
        Peticion {
            version: 1,
            epoca: Some(r.epoca),
            cursor: r.cursor,
            lote: None,
        },
    )
    .unwrap();
    assert!(r2.reiniciar);
    assert!(!r2.cambios.is_empty());
}

#[test]
fn una_receta_viaja_con_sus_ingredientes_y_la_racion_registrada() {
    let (tel, pc, nodo) = (base_con_perfil(), base_con_perfil(), id('a'));
    crate::comida::registrar(&tel, semilla(&tel), 300.0, "lunch").unwrap();
    let hoy = crate::fechas::clave(crate::fechas::hoy());
    let receta = crate::recetas::crear(&tel, "Guiso", 3, &hoy, "lunch").unwrap();
    crate::recetas::registrar(&tel, receta, 1.0, "dinner").unwrap();
    let r = sincronizar(&tel, &pc, &nodo, 0);
    assert!(r.resultados.iter().all(|x| x.estado == Estado::Aplicado));
    assert_eq!(contar(&pc, "SELECT count(*) FROM recipes"), 1);
    assert_eq!(contar(&pc, "SELECT count(*) FROM recipe_ingredients"), 1);
    assert_eq!(contar(&pc, "SELECT count(*) FROM food_logs"), 2);
    // La receta llegada se registra en el escritorio sin crear otra ración materializada.
    let local: i64 = pc
        .conexion()
        .unwrap()
        .query_row("SELECT id FROM recipes", [], |f| f.get(0))
        .unwrap();
    crate::recetas::registrar(&pc, local, 1.0, "snack").unwrap();
    assert_eq!(
        contar(
            &pc,
            "SELECT count(*) FROM food_items WHERE barcode LIKE 'RECIPE_%'"
        ),
        1
    );
}

#[test]
fn borrar_en_el_escritorio_mientras_el_telefono_edita_queda_para_revisar() {
    let (tel, pc, nodo) = (base_con_perfil(), base_con_perfil(), id('a'));
    crate::comida::beber(&tel, 300).unwrap();
    let r = sincronizar(&tel, &pc, &nodo, 0);
    pc.conexion()
        .unwrap()
        .execute("DELETE FROM hydration_logs", [])
        .unwrap();
    tel.conexion()
        .unwrap()
        .execute("UPDATE hydration_logs SET volume_ml = 350", [])
        .unwrap();
    let r2 = sincronizar(&tel, &pc, &nodo, r.cursor);
    assert!(r2.resultados.iter().any(|x| x.estado == Estado::Conflicto));
    let lista = conflictos(&mut pc.conexion().unwrap(), 10).unwrap();
    assert!(lista[0].borrado_aqui && !lista[0].borrado_en_telefono);
    // Conservar la del teléfono revive el registro con su contenido.
    resolver(
        &mut pc.conexion().unwrap(),
        &lista[0].nodo,
        &lista[0].operacion,
        true,
    )
    .unwrap();
    assert_eq!(contar(&pc, "SELECT volume_ml FROM hydration_logs"), 350);
}

#[test]
fn un_par_no_puede_escribir_en_nombre_de_otro_nodo() {
    let (tel, pc) = (base_con_perfil(), base_con_perfil());
    crate::comida::beber(&tel, 300).unwrap();
    let lote = pendientes(&tel, &id('a')).unwrap();
    assert!(atender(
        &mut pc.conexion().unwrap(),
        &id('b'),
        Peticion {
            version: 1,
            epoca: None,
            cursor: 0,
            lote: Some(lote)
        },
    )
    .is_err());
    assert_eq!(contar(&pc, "SELECT count(*) FROM hydration_logs"), 0);
}

#[test]
fn un_nibi_de_fabrica_adopta_el_del_telefono_sin_conflicto() {
    let (tel, pc, nodo) = (base_con_perfil(), base_con_perfil(), id('a'));
    tel.conexion()
        .unwrap()
        .execute(
            "UPDATE pet_status SET pet_name = 'Ethan', equipped_hat_id = 'hat_cap'",
            [],
        )
        .unwrap();
    let r = sincronizar(&tel, &pc, &nodo, 0);
    assert!(r.resultados.iter().all(|x| x.estado == Estado::Aplicado));
    let nombre: String = pc
        .conexion()
        .unwrap()
        .query_row("SELECT pet_name FROM pet_status", [], |f| f.get(0))
        .unwrap();
    assert_eq!(nombre, "Ethan");
}
