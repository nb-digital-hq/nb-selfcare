//! Comida: el catálogo local, los registros del día, el agua, los combos y la
//! consulta a Open Food Facts.
//!
//! Los registros guardan sus totales al momento (como en el teléfono), así que
//! cambiar un alimento después no reescribe lo que ya se comió. El catálogo de
//! partida es la semilla del teléfono, escrita en el idioma de la interfaz la
//! primera vez que se abre la aplicación.

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::db::{self, fallo, BaseDeDatos};
use crate::error::{ScError, ScResult};
use crate::fechas;
use crate::formulas::{self, Metas};
use crate::idioma::{self, Idioma};
use crate::net::{Red, Servicio};
use crate::nibi::{self, Accion};
use crate::perfil;
use crate::texto;

/// La semilla de alimentos, con sus nombres en los tres idiomas.
const SEMILLA: &str = include_str!("../semilla/alimentos.json");

#[derive(Deserialize)]
struct Semilla {
    nombre: Textos,
    porcion: Textos,
    nova_group: Option<i64>,
    default_serving_grams: f64,
    kcal_per100g: f64,
    protein_per100g: f64,
    carbs_per100g: f64,
    fat_per100g: f64,
    fiber_per100g: f64,
    sugar_per100g: f64,
    sodium_mg_per100g: f64,
    potassium_mg_per100g: f64,
    magnesium_mg_per100g: f64,
    calcium_mg_per100g: f64,
    iron_mg_per100g: f64,
    zinc_mg_per100g: f64,
    vitamin_c_mg_per100g: f64,
    vitamin_d_ug_per100g: f64,
    vitamin_b12_ug_per100g: f64,
    is_favorite: bool,
}

#[derive(Deserialize)]
struct Textos {
    es: String,
    en: String,
    pt: String,
}

impl Textos {
    fn en(&self, i: Idioma) -> &str {
        match i {
            Idioma::Es => &self.es,
            Idioma::En => &self.en,
            Idioma::Pt => &self.pt,
        }
    }
}

/// Siembra el catálogo si está vacío, en el idioma de la interfaz, y añade los
/// alimentos de la semilla que falten sin tocar los que ya existen. El idioma
/// en que se sembró queda en `ui_preferences` (`semilla_idioma`), la misma clave
/// del teléfono, para no mezclar idiomas en altas posteriores.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la escritura falla.
pub fn sembrar(db: &BaseDeDatos) -> ScResult<()> {
    let semilla: Vec<Semilla> = serde_json::from_str(SEMILLA).map_err(|_| ScError::Internal)?;
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let hay: i64 = tx
        .query_row("SELECT count(*) FROM food_items", [], |f| f.get(0))
        .map_err(fallo)?;
    let guardado: Option<String> = tx
        .query_row(
            "SELECT pref_value FROM ui_preferences WHERE pref_key = 'semilla_idioma'",
            [],
            |f| f.get(0),
        )
        .optional()
        .map_err(fallo)?;
    let lengua = if hay == 0 {
        idioma::actual()
    } else {
        guardado
            .as_deref()
            .and_then(Idioma::desde_codigo)
            .unwrap_or(Idioma::Es)
    };
    if guardado.is_none() {
        tx.execute(
            "INSERT INTO ui_preferences (pref_key, pref_value) VALUES ('semilla_idioma', ?1)",
            [lengua.codigo()],
        )
        .map_err(fallo)?;
    }
    for s in &semilla {
        let nombre = s.nombre.en(lengua);
        let existe: bool = tx
            .query_row(
                "SELECT 1 FROM food_items WHERE source = 'usda_seed' AND name = ?1",
                [nombre],
                |_| Ok(true),
            )
            .optional()
            .map_err(fallo)?
            .unwrap_or(false);
        if existe {
            continue;
        }
        tx.execute(
            "INSERT INTO food_items (name, source, nova_group, default_serving_grams, serving_label,
                kcal_per100g, protein_per100g, carbs_per100g, fat_per100g, fiber_per100g,
                sugar_per100g, sodium_mg_per100g, potassium_mg_per100g, magnesium_mg_per100g,
                calcium_mg_per100g, iron_mg_per100g, zinc_mg_per100g, vitamin_c_mg_per100g,
                vitamin_d_ug_per100g, vitamin_b12_ug_per100g, is_favorite)
             VALUES (?1, 'usda_seed', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                nombre,
                s.nova_group,
                s.default_serving_grams,
                s.porcion.en(lengua),
                s.kcal_per100g,
                s.protein_per100g,
                s.carbs_per100g,
                s.fat_per100g,
                s.fiber_per100g,
                s.sugar_per100g,
                s.sodium_mg_per100g,
                s.potassium_mg_per100g,
                s.magnesium_mg_per100g,
                s.calcium_mg_per100g,
                s.iron_mg_per100g,
                s.zinc_mg_per100g,
                s.vitamin_c_mg_per100g,
                s.vitamin_d_ug_per100g,
                s.vitamin_b12_ug_per100g,
                s.is_favorite && hay == 0,
            ],
        )
        .map_err(fallo)?;
    }
    nibi::asegurar(&tx)?;
    tx.commit().map_err(fallo)?;
    Ok(())
}

/// Un alimento del catálogo, por 100 g.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Alimento {
    pub id: i64,
    pub nombre: String,
    pub marca: Option<String>,
    /// `usda_seed`, `open_food_facts`, `user_custom` o `ai_fallback`.
    pub origen: String,
    pub codigo: Option<String>,
    pub nutriscore: Option<String>,
    pub nova: Option<i64>,
    pub porcion_g: f64,
    pub porcion: String,
    pub kcal: f64,
    pub proteina: f64,
    pub carbohidratos: f64,
    pub grasa: f64,
    pub fibra: f64,
    pub azucar: f64,
    pub sodio: f64,
    pub potasio: f64,
    pub magnesio: f64,
    pub calcio: f64,
    pub hierro: f64,
    pub zinc: f64,
    pub vitamina_c: f64,
    pub vitamina_d: f64,
    pub vitamina_b12: f64,
    pub favorito: bool,
}

fn alimento_de(f: &Row<'_>) -> rusqlite::Result<Alimento> {
    Ok(Alimento {
        id: f.get("id")?,
        nombre: f.get("name")?,
        marca: f.get("brand")?,
        origen: f.get("source")?,
        codigo: f.get("barcode")?,
        nutriscore: f.get("nutri_score")?,
        nova: f.get("nova_group")?,
        porcion_g: f.get("default_serving_grams")?,
        porcion: f.get("serving_label")?,
        kcal: f.get("kcal_per100g")?,
        proteina: f.get("protein_per100g")?,
        carbohidratos: f.get("carbs_per100g")?,
        grasa: f.get("fat_per100g")?,
        fibra: f.get("fiber_per100g")?,
        azucar: f.get("sugar_per100g")?,
        sodio: f.get("sodium_mg_per100g")?,
        potasio: f.get("potassium_mg_per100g")?,
        magnesio: f.get("magnesium_mg_per100g")?,
        calcio: f.get("calcium_mg_per100g")?,
        hierro: f.get("iron_mg_per100g")?,
        zinc: f.get("zinc_mg_per100g")?,
        vitamina_c: f.get("vitamin_c_mg_per100g")?,
        vitamina_d: f.get("vitamin_d_ug_per100g")?,
        vitamina_b12: f.get("vitamin_b12_ug_per100g")?,
        favorito: f.get("is_favorite")?,
    })
}

fn por_id(conn: &Connection, id: i64) -> ScResult<Alimento> {
    conn.query_row("SELECT * FROM food_items WHERE id = ?1", [id], alimento_de)
        .optional()
        .map_err(fallo)?
        .ok_or(ScError::NotFound)
}

/// Lo que se puede pedir de una búsqueda: como mucho 50.
const TOPE_BUSQUEDA: i64 = 50;

/// Busca en el catálogo local sin diacríticos, por prefijos de palabras. Sin
/// texto, los favoritos y los últimos usados.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn buscar(conn: &Connection, consulta: &str, limite: i64) -> ScResult<Vec<Alimento>> {
    let limite = limite.clamp(1, TOPE_BUSQUEDA);
    let limpio: String = consulta
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let fichas: Vec<String> = limpio
        .split_whitespace()
        .filter(|t| !["AND", "OR", "NOT", "NEAR"].contains(&t.to_uppercase().as_str()))
        .map(|t| format!("\"{t}\"*"))
        .collect();
    if fichas.is_empty() {
        let mut st = conn
            .prepare(
                "SELECT * FROM food_items ORDER BY is_favorite DESC, last_used_at DESC, name ASC LIMIT ?1",
            )
            .map_err(fallo)?;
        let v = st
            .query_map([limite], alimento_de)
            .map_err(fallo)?
            .collect::<Result<_, _>>()
            .map_err(fallo)?;
        return Ok(v);
    }
    let mut st = conn
        .prepare(
            "SELECT f.* FROM food_items f JOIN food_items_fts fts ON f.id = fts.rowid
             WHERE food_items_fts MATCH ?1 ORDER BY f.is_favorite DESC, rank LIMIT ?2",
        )
        .map_err(fallo)?;
    let v = st
        .query_map(params![fichas.join(" "), limite], alimento_de)
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    Ok(v)
}

/// El tipo de comida que toca por la hora: desayuno 5–11, almuerzo 11–16,
/// merienda 16–19 (snack) y cena desde las 19.
#[must_use]
pub fn comida_por_hora(hora: u32) -> &'static str {
    match hora {
        5..=10 => "breakfast",
        11..=15 => "lunch",
        19..=23 => "dinner",
        _ => "snack",
    }
}

const COMIDAS: [&str; 4] = ["breakfast", "lunch", "dinner", "snack"];

fn validar_comida(tipo: &str) -> ScResult<()> {
    if COMIDAS.contains(&tipo) {
        Ok(())
    } else {
        Err(ScError::InvalidInput)
    }
}

/// Inserta un registro con sus totales calculados al momento.
fn insertar_registro(
    conn: &Connection,
    a: &Alimento,
    gramos: f64,
    dia: &str,
    tipo: &str,
) -> ScResult<i64> {
    let g = gramos.clamp(1.0, 5000.0);
    let r = g / 100.0;
    let multiplicador = if a.porcion_g > 0.0 {
        g / a.porcion_g
    } else {
        1.0
    };
    let t = |v: f64| formulas::fijo(v * r, 1);
    conn.execute(
        "INSERT INTO food_logs (date_key, logged_at, meal_type, food_item_id, grams_consumed,
            portion_multiplier, total_kcal, total_protein_g, total_carbs_g, total_fat_g,
            total_fiber_g, total_sodium_mg, total_potassium_mg, total_magnesium_mg)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            dia,
            fechas::texto(fechas::ahora()),
            tipo,
            a.id,
            g,
            formulas::fijo(multiplicador, 2),
            t(a.kcal),
            t(a.proteina),
            t(a.carbohidratos),
            t(a.grasa),
            t(a.fibra),
            t(a.sodio),
            t(a.potasio),
            t(a.magnesio)
        ],
    )
    .map_err(fallo)?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "UPDATE food_items SET last_used_at = ?1 WHERE id = ?2",
        params![fechas::texto(fechas::ahora()), a.id],
    )
    .map_err(fallo)?;
    Ok(id)
}

/// Lo que devuelve un registro: las filas creadas y las monedas ganadas.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Registrado {
    pub ids: Vec<i64>,
    pub monedas: i64,
}

fn registrar_y_premiar(
    db: &BaseDeDatos,
    tipo: &str,
    insertar: impl FnOnce(&Connection) -> ScResult<Vec<i64>>,
) -> ScResult<Registrado> {
    validar_comida(tipo)?;
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let ids = insertar(&tx)?;
    let monedas = if ids.is_empty() {
        0
    } else {
        Accion::de_comida(tipo)
            .map(|a| nibi::premiar(&tx, a))
            .transpose()?
            .flatten()
            .map_or(0, |r| r.monedas)
    };
    tx.commit().map_err(fallo)?;
    Ok(Registrado { ids, monedas })
}

/// Registra una porción de un alimento en el día de hoy.
///
/// # Errors
///
/// [`ScError::NotFound`] si el alimento no existe.
pub fn registrar(db: &BaseDeDatos, alimento: i64, gramos: f64, tipo: &str) -> ScResult<Registrado> {
    if !(gramos > 0.0 && gramos <= 5000.0) {
        return Err(ScError::InvalidInput);
    }
    let dia = fechas::clave(fechas::hoy());
    registrar_y_premiar(db, tipo, |c| {
        let a = por_id(c, alimento)?;
        Ok(vec![insertar_registro(c, &a, gramos, &dia, tipo)?])
    })
}

/// Lo que se leyó de una frase, para confirmarlo antes de guardar.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Propuesta {
    pub leido: texto::Leido,
    /// El alimento del catálogo con el que se enlazaría, si hay uno.
    pub alimento: Option<Alimento>,
    pub kcal: f64,
}

/// Lee una frase y propone con qué alimento del catálogo enlazar cada parte.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la búsqueda falla.
pub fn proponer(db: &BaseDeDatos, frase: &str) -> ScResult<Vec<Propuesta>> {
    let conn = db.conexion()?;
    texto::leer_comida(frase, idioma::actual())
        .into_iter()
        .take(20)
        .map(|l| {
            let a = buscar(&conn, &l.termino, 1)?.into_iter().next();
            let kcal = a.as_ref().map_or(l.kcal, |a| a.kcal) * l.gramos / 100.0;
            Ok(Propuesta {
                kcal: formulas::fijo(kcal, 0),
                leido: l,
                alimento: a,
            })
        })
        .collect()
}

/// Registra lo leído de una frase: lo que no está en el catálogo entra como
/// una estimación con los valores de respaldo, para no perder el registro.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si no se entendió nada.
pub fn registrar_frase(db: &BaseDeDatos, frase: &str, tipo: &str) -> ScResult<Registrado> {
    let leidos = texto::leer_comida(frase, idioma::actual());
    if leidos.is_empty() {
        return Err(ScError::InvalidInput);
    }
    let dia = fechas::clave(fechas::hoy());
    registrar_y_premiar(db, tipo, |c| {
        let mut ids = Vec::new();
        for l in leidos.iter().take(20) {
            let a = match buscar(c, &l.termino, 1)?.into_iter().next() {
                Some(a) => a,
                None => {
                    c.execute(
                        "INSERT INTO food_items (name, source, kcal_per100g, protein_per100g, carbs_per100g, fat_per100g)
                         VALUES (?1, 'ai_fallback', ?2, ?3, ?4, ?5)",
                        params![l.termino, l.kcal, l.proteina, l.carbohidratos, l.grasa],
                    )
                    .map_err(fallo)?;
                    por_id(c, c.last_insert_rowid())?
                }
            };
            ids.push(insertar_registro(c, &a, l.gramos, &dia, tipo)?);
        }
        Ok(ids)
    })
}

/// Copia hoy lo que se registró ayer en cada comida.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si falla.
pub fn repetir_ayer(db: &BaseDeDatos) -> ScResult<Registrado> {
    let hoy = fechas::clave(fechas::hoy());
    let ayer = fechas::clave(fechas::mas_dias(fechas::hoy(), -1));
    let mut total = Registrado {
        ids: Vec::new(),
        monedas: 0,
    };
    for tipo in COMIDAS {
        let r = registrar_y_premiar(db, tipo, |c| {
            let filas: Vec<i64> = {
                let mut st = c
                    .prepare("SELECT id FROM food_logs WHERE date_key = ?1 AND meal_type = ?2 ORDER BY logged_at")
                    .map_err(fallo)?;
                let v = st
                    .query_map(params![ayer, tipo], |f| f.get(0))
                    .map_err(fallo)?
                    .collect::<Result<_, _>>()
                    .map_err(fallo)?;
                v
            };
            let mut ids = Vec::new();
            for id in filas {
                c.execute(
                    "INSERT INTO food_logs (date_key, logged_at, meal_type, food_item_id, grams_consumed,
                        portion_multiplier, total_kcal, total_protein_g, total_carbs_g, total_fat_g,
                        total_fiber_g, total_sodium_mg, total_potassium_mg, total_magnesium_mg)
                     SELECT ?1, ?2, meal_type, food_item_id, grams_consumed, portion_multiplier,
                        total_kcal, total_protein_g, total_carbs_g, total_fat_g, total_fiber_g,
                        total_sodium_mg, total_potassium_mg, total_magnesium_mg
                     FROM food_logs WHERE id = ?3",
                    params![hoy, fechas::texto(fechas::ahora()), id],
                )
                .map_err(fallo)?;
                ids.push(c.last_insert_rowid());
            }
            Ok(ids)
        })?;
        total.ids.extend(r.ids);
        total.monedas += r.monedas;
    }
    Ok(total)
}

/// Borra registros (también para deshacer uno múltiple).
///
/// # Errors
///
/// [`ScError::StorageFailure`] si falla.
pub fn borrar(db: &BaseDeDatos, ids: &[i64]) -> ScResult<()> {
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    for id in ids.iter().take(500) {
        tx.execute("DELETE FROM food_logs WHERE id = ?1", [id])
            .map_err(fallo)?;
    }
    tx.commit().map_err(fallo)
}

/// Marca o desmarca un favorito.
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn favorito(db: &BaseDeDatos, id: i64, si: bool) -> ScResult<()> {
    let conn = db.conexion()?;
    if conn
        .execute(
            "UPDATE food_items SET is_favorite = ?1 WHERE id = ?2",
            params![si, id],
        )
        .map_err(fallo)?
        == 0
    {
        return Err(ScError::NotFound);
    }
    Ok(())
}

/// Un alimento propio, por 100 g.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct NuevoAlimento {
    pub nombre: String,
    pub marca: Option<String>,
    pub porcion_g: f64,
    pub kcal: f64,
    pub proteina: f64,
    pub carbohidratos: f64,
    pub grasa: f64,
    pub fibra: f64,
    pub sodio: f64,
}

/// Crea un alimento propio.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si algo no tiene sentido (más de 900 kcal por
/// 100 g, macros negativos o que suman más de 100 g).
pub fn crear(db: &BaseDeDatos, n: &NuevoAlimento) -> ScResult<Alimento> {
    let nombre = n.nombre.trim();
    let ok = !nombre.is_empty()
        && nombre.chars().count() <= 80
        && (0.0..=900.0).contains(&n.kcal)
        && [n.proteina, n.carbohidratos, n.grasa, n.fibra]
            .iter()
            .all(|v| (0.0..=100.0).contains(v))
        && n.proteina + n.carbohidratos + n.grasa <= 100.5
        && (0.0..=50_000.0).contains(&n.sodio)
        && (1.0..=2000.0).contains(&n.porcion_g);
    if !ok {
        return Err(ScError::InvalidInput);
    }
    let conn = db.conexion()?;
    conn.execute(
        "INSERT INTO food_items (name, brand, source, default_serving_grams, serving_label,
            kcal_per100g, protein_per100g, carbs_per100g, fat_per100g, fiber_per100g, sodium_mg_per100g)
         VALUES (?1, ?2, 'user_custom', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            nombre,
            n.marca.as_deref().map(str::trim).filter(|m| !m.is_empty()),
            n.porcion_g,
            format!("{} g", n.porcion_g),
            n.kcal,
            n.proteina,
            n.carbohidratos,
            n.grasa,
            n.fibra,
            n.sodio
        ],
    )
    .map_err(fallo)?;
    por_id(&conn, conn.last_insert_rowid())
}

// --- El día --------------------------------------------------------------

/// Un registro del día con su alimento.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Registro {
    pub id: i64,
    pub comida: String,
    pub alimento_id: i64,
    pub nombre: String,
    pub gramos: f64,
    pub kcal: f64,
    pub proteina: f64,
    pub carbohidratos: f64,
    pub grasa: f64,
    pub nova: Option<i64>,
    /// `HH:MM`.
    pub hora: String,
}

/// Los totales del día, con los minerales esenciales.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Totales {
    pub kcal: f64,
    pub proteina: f64,
    pub carbohidratos: f64,
    pub grasa: f64,
    pub fibra: f64,
    pub sodio: f64,
    pub potasio: f64,
    pub magnesio: f64,
    pub calcio: f64,
    pub hierro: f64,
    pub zinc: f64,
    pub vitamina_c: f64,
    pub vitamina_d: f64,
    pub vitamina_b12: f64,
    /// kcal de alimentos NOVA 1–3 y NOVA 4.
    pub kcal_poco_procesado: f64,
    pub kcal_ultraprocesado: f64,
}

/// Un vaso de agua.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Vaso {
    pub id: i64,
    pub ml: i64,
    pub hora: String,
}

/// Todo lo de Comida para un día.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct DiaComida {
    pub dia: String,
    pub registros: Vec<Registro>,
    pub totales: Totales,
    pub metas: Metas,
    pub agua: Vec<Vaso>,
    pub agua_ml: i64,
    pub agua_meta_ml: i64,
    /// Si ayer hay algo que repetir.
    pub hay_ayer: bool,
}

/// Los totales de un día.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn totales(conn: &Connection, dia: &str) -> ScResult<Totales> {
    conn.query_row(
        "SELECT coalesce(sum(l.total_kcal), 0), coalesce(sum(l.total_protein_g), 0),
                coalesce(sum(l.total_carbs_g), 0), coalesce(sum(l.total_fat_g), 0),
                coalesce(sum(l.total_fiber_g), 0), coalesce(sum(l.total_sodium_mg), 0),
                coalesce(sum(l.total_potassium_mg), 0), coalesce(sum(l.total_magnesium_mg), 0),
                coalesce(sum(f.calcium_mg_per100g * l.grams_consumed / 100), 0),
                coalesce(sum(f.iron_mg_per100g * l.grams_consumed / 100), 0),
                coalesce(sum(f.zinc_mg_per100g * l.grams_consumed / 100), 0),
                coalesce(sum(f.vitamin_c_mg_per100g * l.grams_consumed / 100), 0),
                coalesce(sum(f.vitamin_d_ug_per100g * l.grams_consumed / 100), 0),
                coalesce(sum(f.vitamin_b12_ug_per100g * l.grams_consumed / 100), 0),
                coalesce(sum(CASE WHEN f.nova_group BETWEEN 1 AND 3 THEN l.total_kcal END), 0),
                coalesce(sum(CASE WHEN f.nova_group = 4 THEN l.total_kcal END), 0)
         FROM food_logs l LEFT JOIN food_items f ON f.id = l.food_item_id
         WHERE l.date_key = ?1",
        [dia],
        |f| {
            Ok(Totales {
                kcal: f.get(0)?,
                proteina: f.get(1)?,
                carbohidratos: f.get(2)?,
                grasa: f.get(3)?,
                fibra: f.get(4)?,
                sodio: f.get(5)?,
                potasio: f.get(6)?,
                magnesio: f.get(7)?,
                calcio: f.get(8)?,
                hierro: f.get(9)?,
                zinc: f.get(10)?,
                vitamina_c: f.get(11)?,
                vitamina_d: f.get(12)?,
                vitamina_b12: f.get(13)?,
                kcal_poco_procesado: f.get(14)?,
                kcal_ultraprocesado: f.get(15)?,
            })
        },
    )
    .map_err(fallo)
}

/// El agua de un día en ml.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn agua_del_dia(conn: &Connection, dia: &str) -> ScResult<i64> {
    conn.query_row(
        "SELECT coalesce(sum(volume_ml), 0) FROM hydration_logs WHERE date_key = ?1",
        [dia],
        |f| f.get(0),
    )
    .map_err(fallo)
}

/// Las metas del día con el ejercicio que ya se hizo.
///
/// # Errors
///
/// [`ScError::NotFound`] sin perfil.
pub fn metas_del_dia(conn: &Connection, dia: &str) -> ScResult<Metas> {
    let fila = perfil::exigir(conn)?;
    let peso = perfil::ultimo_peso(conn)?.map(|p| p.0);
    let ejercicio = crate::cuerpo::ejercicio_del_dia(conn, dia)?;
    let mut d = perfil::datos_metas(&fila, peso);
    // En la fase lútea el gasto basal sube: +150 kcal, como en el teléfono.
    let lutea = fila.perfil.sexo == formulas::Sexo::Mujer
        && fila.perfil.ciclo_activo
        && {
            let inicio: Option<(String, i64)> = conn
            .query_row(
                "SELECT cycle_start_date, cycle_length_days FROM female_cycle_logs ORDER BY cycle_start_date DESC LIMIT 1",
                [],
                |f| Ok((f.get(0)?, f.get(1)?)),
            )
            .optional()
            .map_err(fallo)?;
            inicio
                .and_then(|(i, l)| Some((fechas::leer(&i)?.date(), l)))
                .is_some_and(|(i, l)| {
                    crate::cuerpo::fase_del_ciclo(i, l, fechas::hoy()).0 == "luteal"
                })
        };
    if lutea {
        d.gasto_adaptativo += 150;
    }
    Ok(formulas::metas(&d, ejercicio))
}

/// Todo lo de Comida para un día (`YYYY-MM-DD`).
///
/// # Errors
///
/// [`ScError::InvalidInput`] si el día no se entiende; [`ScError::NotFound`] sin
/// perfil.
pub fn dia(db: &BaseDeDatos, dia: &str) -> ScResult<DiaComida> {
    let d = fechas::de_clave(dia).ok_or(ScError::InvalidInput)?;
    let dia = fechas::clave(d);
    let mut conn = db.conexion()?;
    let tx = db::lectura_coherente(&mut conn)?;
    let mut st = tx
        .prepare(
            "SELECT l.id, l.meal_type, l.food_item_id, coalesce(f.name, '?'), l.grams_consumed,
                    l.total_kcal, l.total_protein_g, l.total_carbs_g, l.total_fat_g, f.nova_group, l.logged_at
             FROM food_logs l LEFT JOIN food_items f ON f.id = l.food_item_id
             WHERE l.date_key = ?1 ORDER BY l.logged_at, l.id",
        )
        .map_err(fallo)?;
    let registros: Vec<Registro> = st
        .query_map([&dia], |f| {
            Ok(Registro {
                id: f.get(0)?,
                comida: f.get(1)?,
                alimento_id: f.get(2)?,
                nombre: f.get(3)?,
                gramos: f.get(4)?,
                kcal: f.get(5)?,
                proteina: f.get(6)?,
                carbohidratos: f.get(7)?,
                grasa: f.get(8)?,
                nova: f.get(9)?,
                hora: fechas::leer(&f.get::<_, String>(10)?)
                    .map(|h| h.format("%H:%M").to_string())
                    .unwrap_or_default(),
            })
        })
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    drop(st);
    let mut st = tx
        .prepare("SELECT id, volume_ml, logged_at FROM hydration_logs WHERE date_key = ?1 ORDER BY logged_at")
        .map_err(fallo)?;
    let agua: Vec<Vaso> = st
        .query_map([&dia], |f| {
            Ok(Vaso {
                id: f.get(0)?,
                ml: f.get(1)?,
                hora: fechas::leer(&f.get::<_, String>(2)?)
                    .map(|h| h.format("%H:%M").to_string())
                    .unwrap_or_default(),
            })
        })
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    drop(st);
    let ayer = fechas::clave(fechas::mas_dias(d, -1));
    let hay_ayer: bool = tx
        .query_row(
            "SELECT 1 FROM food_logs WHERE date_key = ?1 LIMIT 1",
            [&ayer],
            |_| Ok(true),
        )
        .optional()
        .map_err(fallo)?
        .unwrap_or(false);
    let fila = perfil::exigir(&tx)?;
    Ok(DiaComida {
        totales: totales(&tx, &dia)?,
        metas: metas_del_dia(&tx, &dia)?,
        agua_ml: agua.iter().map(|v| v.ml).sum(),
        agua_meta_ml: fila.perfil.agua_meta_ml,
        agua,
        registros,
        hay_ayer,
        dia,
    })
}

/// Un vaso de agua de ahora. Al llegar a la meta del día, se premia.
///
/// # Errors
///
/// [`ScError::InvalidInput`] fuera de 1–5000 ml.
pub fn beber(db: &BaseDeDatos, ml: i64) -> ScResult<i64> {
    if !(1..=5000).contains(&ml) {
        return Err(ScError::InvalidInput);
    }
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let ahora = fechas::ahora();
    let dia = fechas::clave(ahora.date());
    tx.execute(
        "INSERT INTO hydration_logs (date_key, logged_at, volume_ml) VALUES (?1, ?2, ?3)",
        params![dia, fechas::texto(ahora), ml],
    )
    .map_err(fallo)?;
    let total = agua_del_dia(&tx, &dia)?;
    if let Some(f) = perfil::leer(&tx)? {
        if total >= f.perfil.agua_meta_ml {
            nibi::premiar(&tx, Accion::MetaDeAgua)?;
        }
    }
    tx.commit().map_err(fallo)?;
    Ok(total)
}

/// Borra un vaso.
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn borrar_vaso(db: &BaseDeDatos, id: i64) -> ScResult<()> {
    let conn = db.conexion()?;
    if conn
        .execute("DELETE FROM hydration_logs WHERE id = ?1", [id])
        .map_err(fallo)?
        == 0
    {
        return Err(ScError::NotFound);
    }
    Ok(())
}

// --- Combos --------------------------------------------------------------

/// Un combo guardado.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Combo {
    pub id: i64,
    pub nombre: String,
    pub comida: String,
    pub usos: i64,
    pub kcal: f64,
    pub partes: Vec<String>,
}

/// Los combos, los más usados primero.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn combos(db: &BaseDeDatos) -> ScResult<Vec<Combo>> {
    let conn = db.conexion()?;
    let mut st = conn
        .prepare("SELECT id, name, default_meal_type, times_used FROM meal_combos ORDER BY times_used DESC, name LIMIT 100")
        .map_err(fallo)?;
    let base: Vec<(i64, String, String, i64)> = st
        .query_map([], |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?, f.get(3)?)))
        .map_err(fallo)?
        .collect::<Result<_, _>>()
        .map_err(fallo)?;
    base.into_iter()
        .map(|(id, nombre, comida, usos)| {
            let mut st = conn
                .prepare(
                    "SELECT f.name, f.kcal_per100g * i.base_grams / 100 FROM meal_combo_items i
                     JOIN food_items f ON f.id = i.food_item_id WHERE i.combo_id = ?1",
                )
                .map_err(fallo)?;
            let partes: Vec<(String, f64)> = st
                .query_map([id], |f| Ok((f.get(0)?, f.get(1)?)))
                .map_err(fallo)?
                .collect::<Result<_, _>>()
                .map_err(fallo)?;
            Ok(Combo {
                id,
                nombre,
                comida,
                usos,
                kcal: formulas::fijo(partes.iter().map(|p| p.1).sum(), 0),
                partes: partes.into_iter().map(|p| p.0).collect(),
            })
        })
        .collect()
}

/// Guarda una comida del día como combo.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si el nombre está vacío o la comida no tiene nada.
pub fn guardar_combo(db: &BaseDeDatos, nombre: &str, dia: &str, tipo: &str) -> ScResult<i64> {
    validar_comida(tipo)?;
    let nombre = nombre.trim();
    if nombre.is_empty() || nombre.chars().count() > 60 {
        return Err(ScError::InvalidInput);
    }
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    let partes: Vec<(i64, f64)> = {
        let mut st = tx
            .prepare("SELECT food_item_id, grams_consumed FROM food_logs WHERE date_key = ?1 AND meal_type = ?2")
            .map_err(fallo)?;
        let v = st
            .query_map(params![dia, tipo], |f| Ok((f.get(0)?, f.get(1)?)))
            .map_err(fallo)?
            .collect::<Result<_, _>>()
            .map_err(fallo)?;
        v
    };
    if partes.is_empty() {
        return Err(ScError::InvalidInput);
    }
    tx.execute(
        "INSERT INTO meal_combos (name, default_meal_type) VALUES (?1, ?2)",
        params![nombre, tipo],
    )
    .map_err(fallo)?;
    let id = tx.last_insert_rowid();
    for (a, g) in partes {
        tx.execute(
            "INSERT INTO meal_combo_items (combo_id, food_item_id, base_grams) VALUES (?1, ?2, ?3)",
            params![id, a, g],
        )
        .map_err(fallo)?;
    }
    tx.commit().map_err(fallo)?;
    Ok(id)
}

/// Registra un combo escalado (0,25× a 3×).
///
/// # Errors
///
/// [`ScError::InvalidInput`] fuera de escala.
pub fn registrar_combo(
    db: &BaseDeDatos,
    combo: i64,
    escala: f64,
    tipo: &str,
) -> ScResult<Registrado> {
    if !(0.25..=3.0).contains(&escala) {
        return Err(ScError::InvalidInput);
    }
    let dia = fechas::clave(fechas::hoy());
    registrar_y_premiar(db, tipo, |c| {
        let partes: Vec<(i64, f64)> = {
            let mut st = c
                .prepare(
                    "SELECT food_item_id, base_grams FROM meal_combo_items WHERE combo_id = ?1",
                )
                .map_err(fallo)?;
            let v = st
                .query_map([combo], |f| Ok((f.get(0)?, f.get(1)?)))
                .map_err(fallo)?
                .collect::<Result<_, _>>()
                .map_err(fallo)?;
            v
        };
        let mut ids = Vec::new();
        for (a, g) in partes {
            let a = por_id(c, a)?;
            ids.push(insertar_registro(c, &a, g * escala, &dia, tipo)?);
        }
        c.execute(
            "UPDATE meal_combos SET times_used = times_used + 1 WHERE id = ?1",
            [combo],
        )
        .map_err(fallo)?;
        Ok(ids)
    })
}

/// Borra un combo.
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn borrar_combo(db: &BaseDeDatos, id: i64) -> ScResult<()> {
    let conn = db.conexion()?;
    if conn
        .execute("DELETE FROM meal_combos WHERE id = ?1", [id])
        .map_err(fallo)?
        == 0
    {
        return Err(ScError::NotFound);
    }
    Ok(())
}

// --- Open Food Facts -------------------------------------------------------

fn numero(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|x: &f64| x.is_finite())
}

fn texto_de(v: Option<&Value>) -> Option<String> {
    let t = match v? {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => a
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", "),
        _ => return None,
    };
    let t = t.trim().to_owned();
    (!t.is_empty()).then_some(t)
}

fn a_mg(v: Option<f64>, umbral: f64) -> Option<f64> {
    let v = v.filter(|v| *v >= 0.0)?;
    Some(if v > 0.0 && v < umbral {
        formulas::fijo(v * 1000.0, 1)
    } else {
        formulas::fijo(v, 1)
    })
}

/// Lee un producto de Open Food Facts a un alimento por 100 g. Los datos son
/// colaborativos: cualquier campo puede faltar o venir con otro tipo.
#[must_use]
pub fn leer_producto(p: &Value, codigo: &str, lengua: Idioma) -> NuevoProducto {
    let vacio = serde_json::Map::new();
    let n = p
        .get("nutriments")
        .and_then(Value::as_object)
        .unwrap_or(&vacio);
    let g = |k: &str| numero(n.get(k)).map(|v| v.clamp(0.0, 100.0));
    let nombre = texto_de(p.get(format!("product_name_{}", lengua.codigo()).as_str()))
        .or_else(|| texto_de(p.get("product_name")))
        .or_else(|| texto_de(p.get("generic_name")))
        .unwrap_or_else(|| codigo.to_owned());
    let nutriscore = texto_de(p.get("nutriscore_grade"))
        .map(|g| g.to_uppercase())
        .filter(|g| g.len() == 1 && "ABCDE".contains(g.as_str()));
    let nova = numero(p.get("nova_group").or_else(|| p.get("nova_groups")))
        .map(|v| v as i64)
        .filter(|v| (1..=4).contains(v));
    let kcal = numero(n.get("energy-kcal_100g"))
        .unwrap_or_else(|| numero(n.get("energy_100g")).unwrap_or(0.0) / 4.184)
        .clamp(0.0, 900.0);
    let sodio = g("sodium_100g")
        .map(|s| formulas::fijo(s * 1000.0, 1))
        .or_else(|| g("salt_100g").map(|s| s * 400.0));
    NuevoProducto {
        codigo: codigo.to_owned(),
        nombre,
        marca: texto_de(p.get("brands")),
        nutriscore,
        nova,
        kcal: formulas::fijo(kcal, 1),
        proteina: formulas::fijo(g("proteins_100g").unwrap_or(0.0), 1),
        carbohidratos: formulas::fijo(g("carbohydrates_100g").unwrap_or(0.0), 1),
        grasa: formulas::fijo(g("fat_100g").unwrap_or(0.0), 1),
        fibra: g("fiber_100g").unwrap_or(0.0),
        azucar: g("sugars_100g").unwrap_or(0.0),
        sodio: sodio.unwrap_or(0.0),
        potasio: a_mg(numero(n.get("potassium_100g")), 10.0).unwrap_or(0.0),
        magnesio: a_mg(numero(n.get("magnesium_100g")), 5.0).unwrap_or(0.0),
        calcio: a_mg(numero(n.get("calcium_100g")), 10.0).unwrap_or(0.0),
        hierro: a_mg(numero(n.get("iron_100g")), 1.0).unwrap_or(0.0),
        vitamina_c: a_mg(numero(n.get("vitamin-c_100g")), 5.0).unwrap_or(0.0),
    }
}

/// Un producto leído de Open Food Facts.
#[derive(Debug, Clone, PartialEq)]
pub struct NuevoProducto {
    pub codigo: String,
    pub nombre: String,
    pub marca: Option<String>,
    pub nutriscore: Option<String>,
    pub nova: Option<i64>,
    pub kcal: f64,
    pub proteina: f64,
    pub carbohidratos: f64,
    pub grasa: f64,
    pub fibra: f64,
    pub azucar: f64,
    pub sodio: f64,
    pub potasio: f64,
    pub magnesio: f64,
    pub calcio: f64,
    pub hierro: f64,
    pub vitamina_c: f64,
}

fn por_codigo(conn: &Connection, codigo: &str) -> ScResult<Option<Alimento>> {
    conn.query_row(
        "SELECT * FROM food_items WHERE barcode = ?1",
        [codigo],
        alimento_de,
    )
    .optional()
    .map_err(fallo)
}

/// Busca un código de barras: primero en el catálogo local y, si no está y la
/// persona lo consintió, en Open Food Facts, que solo recibe el código. Lo que
/// llega se guarda en el catálogo para no volver a preguntar.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si no es un código de 8 a 14 cifras;
/// [`ScError::NetworkDisabled`] o [`ScError::ConsentRequired`] si no se puede
/// salir; [`ScError::NotFound`] si no existe.
pub fn por_codigo_de_barras(db: &BaseDeDatos, red: &Red, codigo: &str) -> ScResult<Alimento> {
    let codigo = codigo.trim();
    if !(8..=14).contains(&codigo.len()) || !codigo.chars().all(|c| c.is_ascii_digit()) {
        return Err(ScError::InvalidInput);
    }
    {
        let conn = db.conexion()?;
        if let Some(a) = por_codigo(&conn, codigo)? {
            return Ok(a);
        }
        if codigo.len() == 12 {
            if let Some(a) = por_codigo(&conn, &format!("0{codigo}"))? {
                return Ok(a);
            }
        }
    }
    let mut candidatos = vec![codigo.to_owned()];
    if codigo.len() == 12 {
        candidatos.push(format!("0{codigo}"));
    }
    for c in candidatos {
        let url = format!("https://world.openfoodfacts.org/api/v2/product/{c}.json");
        let (estado, cuerpo) = red.obtener_con_estado(Servicio::OpenFoodFacts, &url, &[])?;
        if estado == 404 {
            continue;
        }
        if !(200..300).contains(&estado) {
            return Err(ScError::NetworkFailure);
        }
        let v: Value = serde_json::from_slice(&cuerpo).map_err(|_| ScError::NetworkFailure)?;
        if v.get("status").and_then(Value::as_i64) != Some(1) {
            continue;
        }
        let Some(producto) = v.get("product") else {
            continue;
        };
        let p = leer_producto(producto, &c, idioma::actual());
        let conn = db.conexion()?;
        if let Some(a) = por_codigo(&conn, &c)? {
            return Ok(a);
        }
        conn.execute(
            "INSERT INTO food_items (barcode, name, brand, source, nutri_score, nova_group,
                kcal_per100g, protein_per100g, carbs_per100g, fat_per100g, fiber_per100g, sugar_per100g,
                sodium_mg_per100g, potassium_mg_per100g, magnesium_mg_per100g, calcium_mg_per100g,
                iron_mg_per100g, vitamin_c_mg_per100g)
             VALUES (?1, ?2, ?3, 'open_food_facts', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                p.codigo,
                p.nombre,
                p.marca,
                p.nutriscore,
                p.nova,
                p.kcal,
                p.proteina,
                p.carbohidratos,
                p.grasa,
                p.fibra,
                p.azucar,
                p.sodio,
                p.potasio,
                p.magnesio,
                p.calcio,
                p.hierro,
                p.vitamina_c
            ],
        )
        .map_err(fallo)?;
        return por_id(&conn, conn.last_insert_rowid());
    }
    Err(ScError::NotFound)
}

/// Un alimento por su id, para la hoja de detalle.
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe.
pub fn alimento(db: &BaseDeDatos, id: i64) -> ScResult<Alimento> {
    por_id(&*db.conexion()?, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perfil::pruebas::base_con_perfil;

    fn sembrada() -> BaseDeDatos {
        let db = base_con_perfil();
        idioma::fijar(Idioma::Es);
        sembrar(&db).unwrap();
        db
    }

    #[test]
    fn la_semilla_entra_una_vez_en_el_idioma_de_la_interfaz() {
        let db = sembrada();
        sembrar(&db).unwrap();
        let conn = db.conexion().unwrap();
        let n: i64 = conn
            .query_row("SELECT count(*) FROM food_items", [], |f| f.get(0))
            .unwrap();
        assert_eq!(n, 163);
        assert!(!buscar(&conn, "pollo", 5).unwrap().is_empty());
    }

    #[test]
    fn la_busqueda_ignora_tildes_y_operadores() {
        let db = sembrada();
        let conn = db.conexion().unwrap();
        let con = buscar(&conn, "salmón", 5).unwrap();
        let sin = buscar(&conn, "salmon", 5).unwrap();
        assert_eq!(con.first().map(|a| a.id), sin.first().map(|a| a.id));
        assert!(buscar(&conn, "OR \"", 5).is_ok());
    }

    #[test]
    fn registrar_guarda_los_totales_y_premia_el_almuerzo_una_vez() {
        let db = sembrada();
        let id = buscar(&db.conexion().unwrap(), "pollo", 1).unwrap()[0].id;
        let r = registrar(&db, id, 200.0, "lunch").unwrap();
        assert_eq!(r.monedas, 10);
        let r = registrar(&db, id, 100.0, "lunch").unwrap();
        assert_eq!(r.monedas, 0);
        let snack = registrar(&db, id, 50.0, "snack").unwrap();
        assert_eq!(snack.monedas, 0, "picar no da monedas");
        let d = dia(&db, "2026-11-20").unwrap();
        assert_eq!(d.registros.len(), 3);
        assert!(d.totales.kcal > 0.0);
    }

    #[test]
    fn una_frase_sin_catalogo_entra_como_estimacion() {
        let db = sembrada();
        let r = registrar_frase(&db, "200 g de quinotto marciano", "dinner").unwrap();
        assert_eq!(r.ids.len(), 1);
        let conn = db.conexion().unwrap();
        let origen: String = conn
            .query_row(
                "SELECT f.source FROM food_logs l JOIN food_items f ON f.id = l.food_item_id",
                [],
                |f| f.get(0),
            )
            .unwrap();
        assert_eq!(origen, "ai_fallback");
    }

    #[test]
    fn llegar_a_la_meta_de_agua_premia() {
        let db = sembrada();
        assert_eq!(beber(&db, 2000).unwrap(), 2000);
        assert_eq!(beber(&db, 600).unwrap(), 2600);
        let conn = db.conexion().unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM coin_ledger WHERE action_type = 'water_goal'",
                [],
                |f| f.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(beber(&db, 0), Err(ScError::InvalidInput));
    }

    #[test]
    fn un_combo_se_guarda_y_se_registra_escalado() {
        let db = sembrada();
        let id = buscar(&db.conexion().unwrap(), "avena", 1).unwrap()[0].id;
        registrar(&db, id, 80.0, "breakfast").unwrap();
        let c = guardar_combo(&db, "Mi desayuno", "2026-11-20", "breakfast").unwrap();
        let r = registrar_combo(&db, c, 0.5, "breakfast").unwrap();
        assert_eq!(r.ids.len(), 1);
        let g: f64 = db
            .conexion()
            .unwrap()
            .query_row(
                "SELECT grams_consumed FROM food_logs WHERE id = ?1",
                [r.ids[0]],
                |f| f.get(0),
            )
            .unwrap();
        assert_eq!(g, 40.0);
        assert_eq!(combos(&db).unwrap()[0].usos, 1);
    }

    #[test]
    fn un_producto_de_off_se_lee_aunque_venga_raro() {
        let p: Value = serde_json::json!({
            "product_name_es": "Galletas",
            "brands": ["Marca", "Otra"],
            "nutriscore_grade": "d",
            "nova_group": "4",
            "nutriments": { "energy_100g": 2000, "proteins_100g": "6.5", "salt_100g": 1.2, "potassium_100g": 0.2 }
        });
        let n = leer_producto(&p, "7701234567890", Idioma::Es);
        assert_eq!(n.nombre, "Galletas");
        assert_eq!(n.marca.as_deref(), Some("Marca, Otra"));
        assert_eq!((n.nutriscore.as_deref(), n.nova), (Some("D"), Some(4)));
        assert_eq!(n.kcal, 478.0);
        assert_eq!(n.proteina, 6.5);
        assert!((n.sodio - 480.0).abs() < 0.01);
        assert_eq!(n.potasio, 200.0);
    }

    #[test]
    fn sin_consentimiento_open_food_facts_no_se_toca() {
        let db = sembrada();
        let red = Red::new(crate::net::PoliticaDeRed::default());
        assert_eq!(
            por_codigo_de_barras(&db, &red, "7701234567890"),
            Err(ScError::NetworkDisabled)
        );
        assert_eq!(
            por_codigo_de_barras(&db, &red, "12ab"),
            Err(ScError::InvalidInput)
        );
    }

    #[test]
    fn la_comida_sigue_la_hora() {
        assert_eq!(comida_por_hora(7), "breakfast");
        assert_eq!(comida_por_hora(13), "lunch");
        assert_eq!(comida_por_hora(17), "snack");
        assert_eq!(comida_por_hora(21), "dinner");
        assert_eq!(comida_por_hora(2), "snack");
    }
}
