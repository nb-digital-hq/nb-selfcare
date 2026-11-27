//! Preferencias de la aplicación: apariencia, idioma, avisos y red.
//!
//! Viven en la tabla `preferencias` como JSON. La interfaz solo puede escribir
//! las claves de [`validar`], cada una con su tipo: una clave nueva se añade aquí
//! con su validación, no se acepta a ciegas lo que llegue del webview. La
//! política de red tiene su propio camino ([`cambiar_politica`]) porque además
//! de guardarse se aplica a la puerta en el acto.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::db::{self, fallo, BaseDeDatos};
use crate::error::{ScError, ScResult};
use crate::net::{PoliticaDeRed, Red};

/// Las tipografías que viajan con la aplicación (`src/fuentes.css`).
pub const TIPOGRAFIAS: [&str; 5] = [
    "Inter",
    "Fira Sans",
    "IBM Plex Sans",
    "Atkinson Hyperlegible",
    "Source Sans 3",
];

/// El tema de fábrica: el mismo que en el teléfono.
pub const TEMA_DE_FABRICA: &str = "samsung_dark";

/// Lo que la interfaz necesita para pintarse como la dejó la persona.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Apariencia {
    /// El id de un tema.
    pub tema: String,
    pub tipografia: String,
    /// En porcentaje.
    pub escala: f64,
    /// `es`, `en` o `pt`; `None` la primera vez, cuando aún no se eligió ninguno.
    pub idioma: Option<String>,
}

/// Cómo avisan las pausas activas y el descanso visual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct AjustesAvisos {
    /// Pausa activa cada tantos minutos de jornada; 0 la apaga.
    pub pausa_cada_min: u32,
    /// Descanso visual 20-20-20 cada tantos minutos; 0 lo apaga.
    pub vista_cada_min: u32,
    /// Inicio y fin de la jornada, en minutos desde medianoche.
    pub jornada_desde: u32,
    pub jornada_hasta: u32,
    /// Días de la jornada como en el teléfono: lunes es el bit 0 (`31` es de lunes
    /// a viernes).
    pub jornada_dias: u8,
    /// Avisos de las tomas de medicamentos y suplementos.
    pub tomas: bool,
    /// Avisos de agua cada tantos minutos dentro de la jornada; 0 los apaga.
    pub agua_cada_min: u32,
    /// Cerrar la ventana deja la aplicación en la bandeja para que los avisos
    /// sigan llegando.
    pub seguir_en_bandeja: bool,
}

impl Default for AjustesAvisos {
    fn default() -> Self {
        Self {
            pausa_cada_min: 60,
            vista_cada_min: 20,
            jornada_desde: 9 * 60,
            jornada_hasta: 18 * 60,
            jornada_dias: 31,
            tomas: true,
            agua_cada_min: 0,
            seguir_en_bandeja: true,
        }
    }
}

impl AjustesAvisos {
    fn valido(&self) -> bool {
        let cada = |v: u32| v == 0 || (10..=240).contains(&v);
        cada(self.pausa_cada_min)
            && (self.vista_cada_min == 0 || (15..=120).contains(&self.vista_cada_min))
            && cada(self.agua_cada_min)
            && self.jornada_desde < self.jornada_hasta
            && self.jornada_hasta <= 24 * 60
            && self.jornada_dias & 0x80 == 0
    }
}

pub(crate) fn leer(conn: &Connection, clave: &str) -> ScResult<Option<Value>> {
    let v: Option<String> = conn
        .query_row(
            "SELECT valor FROM preferencias WHERE clave = ?1",
            [clave],
            |f| f.get(0),
        )
        .optional()
        .map_err(fallo)?;
    Ok(v.and_then(|v| serde_json::from_str(&v).ok()))
}

fn escribir(conn: &Connection, clave: &str, valor: &Value) -> ScResult<()> {
    conn.execute(
        "INSERT INTO preferencias (clave, valor) VALUES (?1, ?2)
         ON CONFLICT (clave) DO UPDATE SET valor = excluded.valor",
        rusqlite::params![clave, valor.to_string()],
    )
    .map_err(fallo)?;
    Ok(())
}

/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn apariencia(db: &BaseDeDatos) -> ScResult<Apariencia> {
    let conn = db.conexion()?;
    let texto = |v: Option<Value>| v.and_then(|v| v.as_str().map(str::to_owned));
    Ok(Apariencia {
        tema: texto(leer(&conn, "tema")?).unwrap_or_else(|| TEMA_DE_FABRICA.to_owned()),
        tipografia: texto(leer(&conn, "tipografia")?)
            .filter(|t| TIPOGRAFIAS.contains(&t.as_str()))
            .unwrap_or_else(|| "Inter".to_owned()),
        escala: leer(&conn, "escala")?
            .and_then(|v| v.as_f64())
            .filter(|e| (75.0..=150.0).contains(e))
            .unwrap_or(100.0),
        idioma: texto(leer(&conn, "idioma")?)
            .filter(|i| crate::idioma::Idioma::desde_codigo(i).is_some()),
    })
}

/// Los ajustes de los avisos; los de fábrica si no hay o no se entienden.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn avisos(db: &BaseDeDatos) -> ScResult<AjustesAvisos> {
    let conn = db.conexion()?;
    Ok(leer(&conn, "avisos")?
        .and_then(|v| serde_json::from_value::<AjustesAvisos>(v).ok())
        .filter(AjustesAvisos::valido)
        .unwrap_or_default())
}

/// # Errors
///
/// [`ScError::InvalidInput`] si los valores no tienen sentido.
pub fn guardar_avisos(db: &BaseDeDatos, a: &AjustesAvisos) -> ScResult<()> {
    if !a.valido() {
        return Err(ScError::InvalidInput);
    }
    let conn = db.conexion()?;
    escribir(
        &conn,
        "avisos",
        &serde_json::to_value(a).map_err(|_| ScError::Internal)?,
    )
}

/// Comprueba que un valor vale para su clave. Las claves fuera de la lista se
/// rechazan.
fn validar(clave: &str, valor: &Value) -> ScResult<()> {
    let ok = match clave {
        "tema" => valor.as_str().is_some_and(|t| {
            !t.is_empty()
                && t.len() <= 64
                && t.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        }),
        "tipografia" => valor.as_str().is_some_and(|t| TIPOGRAFIAS.contains(&t)),
        "escala" => valor.as_f64().is_some_and(|e| (75.0..=150.0).contains(&e)),
        "idioma" => valor
            .as_str()
            .is_some_and(|i| crate::idioma::Idioma::desde_codigo(i).is_some()),
        _ => return Err(ScError::PathNotAllowed),
    };
    if ok {
        Ok(())
    } else {
        Err(ScError::InvalidInput)
    }
}

/// # Errors
///
/// - [`ScError::PathNotAllowed`] si la clave no es una de las que la interfaz
///   puede cambiar.
/// - [`ScError::InvalidInput`] si el valor no vale para esa clave.
pub fn guardar(db: &BaseDeDatos, clave: &str, valor: &Value) -> ScResult<()> {
    validar(clave, valor)?;
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    escribir(&tx, clave, valor)?;
    // El tema también en el perfil, para que una copia llevada al teléfono abra
    // con el mismo (los ids de los temas son los del teléfono).
    if clave == "tema" {
        tx.execute("UPDATE user_profiles SET theme_mode = ?1", [valor.as_str()])
            .map_err(fallo)?;
    }
    tx.commit().map_err(fallo)?;
    if clave == "idioma" {
        if let Some(i) = valor.as_str().and_then(crate::idioma::Idioma::desde_codigo) {
            crate::idioma::fijar(i);
        }
    }
    Ok(())
}

/// Fija el idioma del runtime con el que está guardado. Se llama al arrancar.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn cargar_idioma(db: &BaseDeDatos) -> ScResult<()> {
    let conn = db.conexion()?;
    if let Some(i) = leer(&conn, "idioma")?
        .as_ref()
        .and_then(Value::as_str)
        .and_then(crate::idioma::Idioma::desde_codigo)
    {
        crate::idioma::fijar(i);
    }
    Ok(())
}

/// La política de red guardada; la de fábrica, que no deja salir nada, si no hay
/// ninguna o no se puede leer.
#[must_use]
pub fn politica(db: &BaseDeDatos) -> PoliticaDeRed {
    db.conexion()
        .ok()
        .and_then(|c| leer(&c, "politica_de_red").ok().flatten())
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

/// Guarda la política y la aplica a la puerta. Si no se puede guardar, la puerta
/// sigue como estaba, para que lo que se ve y lo que se cumple no se separen.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si no se puede guardar.
pub fn cambiar_politica(db: &BaseDeDatos, red: &Red, nueva: PoliticaDeRed) -> ScResult<()> {
    let conn = db.conexion()?;
    escribir(
        &conn,
        "politica_de_red",
        &serde_json::to_value(&nueva).map_err(|_| ScError::Internal)?,
    )?;
    red.cambiar_politica(nueva);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sin_preferencias_la_apariencia_es_la_de_fabrica() {
        let db = BaseDeDatos::en_memoria().unwrap();
        let a = apariencia(&db).unwrap();
        assert_eq!(
            (a.tema.as_str(), a.tipografia.as_str(), a.escala),
            (TEMA_DE_FABRICA, "Inter", 100.0)
        );
        assert_eq!(a.idioma, None);
    }

    #[test]
    fn solo_se_guardan_las_claves_de_la_lista_con_su_tipo() {
        let db = BaseDeDatos::en_memoria().unwrap();
        guardar(&db, "tema", &json!("bosque_boreal")).unwrap();
        assert_eq!(apariencia(&db).unwrap().tema, "bosque_boreal");
        assert_eq!(
            guardar(&db, "politica_de_red", &json!({})),
            Err(ScError::PathNotAllowed)
        );
        assert_eq!(
            guardar(&db, "escala", &json!(400)),
            Err(ScError::InvalidInput)
        );
        assert_eq!(
            guardar(&db, "tema", &json!("<script>")),
            Err(ScError::InvalidInput)
        );
    }

    #[test]
    fn los_avisos_sin_sentido_se_rechazan_y_los_de_fabrica_son_una_jornada() {
        let db = BaseDeDatos::en_memoria().unwrap();
        let a = avisos(&db).unwrap();
        assert_eq!(
            (a.pausa_cada_min, a.vista_cada_min, a.jornada_dias),
            (60, 20, 31)
        );
        let mut malo = a;
        malo.jornada_desde = 20 * 60;
        assert_eq!(guardar_avisos(&db, &malo), Err(ScError::InvalidInput));
        let mut bueno = a;
        bueno.pausa_cada_min = 45;
        guardar_avisos(&db, &bueno).unwrap();
        assert_eq!(avisos(&db).unwrap().pausa_cada_min, 45);
    }

    #[test]
    fn la_politica_guardada_se_aplica_a_la_puerta() {
        let db = BaseDeDatos::en_memoria().unwrap();
        let red = Red::new(politica(&db));
        let nueva = PoliticaDeRed {
            general: true,
            servicios: [crate::net::Servicio::OpenFoodFacts].into_iter().collect(),
        };
        cambiar_politica(&db, &red, nueva.clone()).unwrap();
        assert_eq!(red.politica(), nueva);
        assert_eq!(politica(&db), nueva);
    }
}
