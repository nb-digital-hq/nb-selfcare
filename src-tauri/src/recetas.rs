//! Recetas caseras: las del teléfono llegan por sincronización y aquí se pueden
//! consultar, registrar por raciones o crear a partir de una comida ya anotada.
//!
//! Como en SelfCare Mobile, una ración se registra como un alimento propio donde
//! «100 g» equivale a una ración. El escritorio lo enlaza por la identidad estable
//! de la receta (`RECIPE_<identidad>`) y no por su entero local, que difiere entre equipos.
use crate::comida::Registrado;
use crate::db::{self, fallo, BaseDeDatos};
use crate::error::{ScError, ScResult};
use crate::{fechas, formulas, idioma};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Receta {
    pub id: i64,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub comida: String,
    pub raciones: i64,
    pub kcal: f64,
    pub proteina: f64,
    pub carbohidratos: f64,
    pub grasa: f64,
    pub ingredientes: Vec<String>,
    pub ultimo_uso: Option<String>,
}

/// Recetas, las usadas más recientemente primero. Cota: 200.
///
/// # Errors
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn recetas(db: &BaseDeDatos) -> ScResult<Vec<Receta>> {
    let mut conn = db.conexion()?;
    let tx = db::lectura_coherente(&mut conn)?;
    let base: Vec<Receta> = {
        let mut st = tx
            .prepare(
                "SELECT id, name, description, default_meal_type, total_yield_servings, kcal_per_serving,
                        protein_per_serving, carbs_per_serving, fat_per_serving, last_used_at
                 FROM recipes ORDER BY coalesce(last_used_at, created_at) DESC, name LIMIT 200",
            )
            .map_err(fallo)?;
        let v = st
            .query_map([], |f| {
                Ok(Receta {
                    id: f.get(0)?,
                    nombre: f.get(1)?,
                    descripcion: f.get(2)?,
                    comida: f.get(3)?,
                    raciones: f.get(4)?,
                    kcal: formulas::fijo(f.get(5)?, 0),
                    proteina: formulas::fijo(f.get(6)?, 1),
                    carbohidratos: formulas::fijo(f.get(7)?, 1),
                    grasa: formulas::fijo(f.get(8)?, 1),
                    ingredientes: Vec::new(),
                    ultimo_uso: f.get(9)?,
                })
            })
            .map_err(fallo)?
            .collect::<Result<_, _>>()
            .map_err(fallo)?;
        v
    };
    let mut st = tx
        .prepare(
            "SELECT f.name FROM recipe_ingredients i JOIN food_items f ON f.id = i.food_item_id
             WHERE i.recipe_id = ?1 ORDER BY i.raw_grams DESC LIMIT 20",
        )
        .map_err(fallo)?;
    base.into_iter()
        .map(|mut r| {
            r.ingredientes = st
                .query_map([r.id], |f| f.get(0))
                .map_err(fallo)?
                .collect::<Result<_, _>>()
                .map_err(fallo)?;
            Ok(r)
        })
        .collect()
}

/// Crea una receta con los alimentos de una comida ya registrada, repartidos en `raciones`.
///
/// # Errors
/// [`ScError::InvalidInput`] con nombre vacío, raciones fuera de 1..=50 o una comida sin registros.
pub fn crear(
    db: &BaseDeDatos,
    nombre: &str,
    raciones: i64,
    dia: &str,
    tipo: &str,
) -> ScResult<i64> {
    let nombre = nombre.trim();
    if nombre.is_empty() || nombre.chars().count() > 80 || !(1..=50).contains(&raciones) {
        return Err(ScError::InvalidInput);
    }
    fechas::de_clave(dia).ok_or(ScError::InvalidInput)?;
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let partes: Vec<(i64, f64, [f64; 8])> = {
        let mut st = tx
            .prepare(
                "SELECT food_item_id, grams_consumed, total_kcal, total_protein_g, total_carbs_g, total_fat_g,
                        total_fiber_g, total_sodium_mg, total_potassium_mg, total_magnesium_mg
                 FROM food_logs WHERE date_key = ?1 AND meal_type = ?2",
            )
            .map_err(fallo)?;
        let v = st
            .query_map(params![dia, tipo], |f| {
                Ok((
                    f.get(0)?,
                    f.get(1)?,
                    [
                        f.get(2)?,
                        f.get(3)?,
                        f.get(4)?,
                        f.get(5)?,
                        f.get(6)?,
                        f.get(7)?,
                        f.get(8)?,
                        f.get(9)?,
                    ],
                ))
            })
            .map_err(fallo)?
            .collect::<Result<_, _>>()
            .map_err(fallo)?;
        v
    };
    if partes.is_empty() {
        return Err(ScError::InvalidInput);
    }
    let mut total = [0.0; 8];
    for (_, _, n) in &partes {
        for (t, v) in total.iter_mut().zip(n) {
            *t += v;
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let por = total.map(|v| formulas::fijo(v / raciones as f64, 1));
    tx.execute(
        "INSERT INTO recipes (name, default_meal_type, total_yield_servings, cooked_total_weight_grams,
            kcal_per_serving, protein_per_serving, carbs_per_serving, fat_per_serving, fiber_per_serving,
            sodium_mg_per_serving, potassium_mg_per_serving, magnesium_mg_per_serving, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            nombre,
            tipo,
            raciones,
            partes.iter().map(|p| p.1).sum::<f64>(),
            por[0],
            por[1],
            por[2],
            por[3],
            por[4],
            por[5],
            por[6],
            por[7],
            fechas::texto(fechas::ahora())
        ],
    )
    .map_err(fallo)?;
    let id = tx.last_insert_rowid();
    for (alimento, gramos, _) in partes {
        tx.execute(
            "INSERT INTO recipe_ingredients (recipe_id, food_item_id, raw_grams) VALUES (?1, ?2, ?3)",
            params![id, alimento, gramos],
        )
        .map_err(fallo)?;
    }
    tx.commit().map_err(fallo)?;
    Ok(id)
}

/// Registra raciones de una receta en el día de hoy, con las recompensas propias de una comida.
///
/// # Errors
/// [`ScError::NotFound`] si la receta no existe; [`ScError::InvalidInput`] fuera de 0,25..=10 raciones.
pub fn registrar(db: &BaseDeDatos, id: i64, raciones: f64, tipo: &str) -> ScResult<Registrado> {
    if !(0.25..=10.0).contains(&raciones) {
        return Err(ScError::InvalidInput);
    }
    let alimento = {
        let mut conn = db.conexion()?;
        let tx = db::transaccion_de_escritura(&mut conn)?;
        let r: Option<(String, [f64; 8])> = tx
            .query_row(
                "SELECT name, kcal_per_serving, protein_per_serving, carbs_per_serving, fat_per_serving,
                        fiber_per_serving, sodium_mg_per_serving, potassium_mg_per_serving, magnesium_mg_per_serving
                 FROM recipes WHERE id = ?1",
                [id],
                |f| {
                    Ok((
                        f.get(0)?,
                        [f.get(1)?, f.get(2)?, f.get(3)?, f.get(4)?, f.get(5)?, f.get(6)?, f.get(7)?, f.get(8)?],
                    ))
                },
            )
            .optional()
            .map_err(fallo)?;
        let (nombre, n) = r.ok_or(ScError::NotFound)?;
        let registro =
            crate::sincronizacion::dominio::identidad_de(&tx, "recipes", &id.to_string())?
                .ok_or(ScError::StorageFailure)?;
        let codigo = format!("RECIPE_{registro}");
        let existente: Option<i64> = tx
            .query_row(
                "SELECT id FROM food_items WHERE barcode = ?1",
                [&codigo],
                |f| f.get(0),
            )
            .optional()
            .map_err(fallo)?;
        let alimento = if let Some(a) = existente {
            // La ración refleja la receta actual aunque se haya editado en el teléfono.
            tx.execute(
                "UPDATE food_items SET name = ?2, kcal_per100g = ?3, protein_per100g = ?4, carbs_per100g = ?5,
                    fat_per100g = ?6, fiber_per100g = ?7, sodium_mg_per100g = ?8, potassium_mg_per100g = ?9,
                    magnesium_mg_per100g = ?10 WHERE id = ?1",
                params![a, nombre, n[0], n[1], n[2], n[3], n[4], n[5], n[6], n[7]],
            )
            .map_err(fallo)?;
            a
        } else {
            tx.execute(
                "INSERT INTO food_items (barcode, name, brand, source, default_serving_grams, serving_label,
                    kcal_per100g, protein_per100g, carbs_per100g, fat_per100g, fiber_per100g,
                    sodium_mg_per100g, potassium_mg_per100g, magnesium_mg_per100g)
                 VALUES (?1, ?2, ?3, 'user_custom', 100.0, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    codigo,
                    nombre,
                    idioma::t("Receta casera", "Homemade recipe", "Receita caseira"),
                    idioma::t("1 ración", "1 serving", "1 porção"),
                    n[0],
                    n[1],
                    n[2],
                    n[3],
                    n[4],
                    n[5],
                    n[6],
                    n[7]
                ],
            )
            .map_err(fallo)?;
            tx.last_insert_rowid()
        };
        tx.execute(
            "UPDATE recipes SET last_used_at = ?2 WHERE id = ?1",
            params![id, fechas::texto(fechas::ahora())],
        )
        .map_err(fallo)?;
        tx.commit().map_err(fallo)?;
        alimento
    };
    crate::comida::registrar(db, alimento, raciones * 100.0, tipo)
}

/// # Errors
/// [`ScError::NotFound`] si la receta no existe.
pub fn borrar(db: &BaseDeDatos, id: i64) -> ScResult<()> {
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    if tx
        .execute("DELETE FROM recipes WHERE id = ?1", [id])
        .map_err(fallo)?
        == 0
    {
        return Err(ScError::NotFound);
    }
    tx.commit().map_err(fallo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perfil::pruebas::base_con_perfil;

    #[test]
    fn una_receta_se_crea_desde_una_comida_y_se_registra_por_raciones() {
        let db = base_con_perfil();
        crate::comida::sembrar(&db).unwrap();
        let alimento: i64 = db
            .conexion()
            .unwrap()
            .query_row("SELECT id FROM food_items ORDER BY id LIMIT 1", [], |f| {
                f.get(0)
            })
            .unwrap();
        crate::comida::registrar(&db, alimento, 400.0, "lunch").unwrap();
        let hoy = fechas::clave(fechas::hoy());
        assert_eq!(
            crear(&db, " ", 2, &hoy, "lunch"),
            Err(ScError::InvalidInput)
        );
        assert_eq!(
            crear(&db, "Guiso", 2, &hoy, "dinner"),
            Err(ScError::InvalidInput)
        );
        let id = crear(&db, "Guiso", 4, &hoy, "lunch").unwrap();
        let r = recetas(&db).unwrap();
        let guiso = r.iter().find(|r| r.id == id).unwrap();
        assert_eq!(guiso.raciones, 4);
        assert_eq!(guiso.ingredientes.len(), 1);
        let antes: f64 = db
            .conexion()
            .unwrap()
            .query_row("SELECT sum(total_kcal) FROM food_logs", [], |f| f.get(0))
            .unwrap();
        registrar(&db, id, 1.0, "dinner").unwrap();
        registrar(&db, id, 1.0, "dinner").unwrap();
        let despues: f64 = db
            .conexion()
            .unwrap()
            .query_row("SELECT sum(total_kcal) FROM food_logs", [], |f| f.get(0))
            .unwrap();
        // Dos raciones de un cuarto del total: la mitad de lo que se comió al crearla.
        assert!(
            ((despues - antes) - antes / 2.0).abs() < 1.0,
            "{antes} {despues}"
        );
        // La ración se materializa una sola vez, enlazada por la identidad estable.
        let n: i64 = db
            .conexion()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM food_items WHERE barcode LIKE 'RECIPE_%'",
                [],
                |f| f.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            registrar(&db, id, 0.0, "dinner").err(),
            Some(ScError::InvalidInput)
        );
        borrar(&db, id).unwrap();
        assert_eq!(borrar(&db, id), Err(ScError::NotFound));
    }
}
