//! Protocolo 1: lotes acotados, identificadores estables y revisión base explícita.
use crate::error::{ScError, ScResult};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashSet;

pub const VERSION: u32 = 1;
pub const MAX_CAMBIOS: usize = 200;
pub const MAX_REGISTRO_BYTES: usize = 64 * 1024;
pub const MAX_REVISION: u64 = 9_007_199_254_740_991;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Identificador(String);
impl Identificador {
    /// Identidad aleatoria de 128 bits, sin información del equipo o de la persona.
    /// # Errors
    /// [`ScError::ComponentUnavailable`] si falla la fuente segura del sistema.
    pub fn generar() -> ScResult<Self> {
        use std::fmt::Write as _;
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes).map_err(|_| ScError::ComponentUnavailable)?;
        let mut valor = String::with_capacity(32);
        for byte in bytes {
            let _ = write!(valor, "{byte:02x}");
        }
        Ok(Self(valor))
    }
    pub fn exponer(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for Identificador {
    type Error = ScError;
    fn try_from(valor: String) -> ScResult<Self> {
        if valor.len() != 32
            || !valor
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(ScError::InvalidInput);
        }
        Ok(Self(valor))
    }
}
impl From<Identificador> for String {
    fn from(valor: Identificador) -> Self {
        valor.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Entidad {
    Perfil,
    InventarioNibi,
    Descanso,
    Alimento,
    Combo,
    ComboAlimento,
    Receta,
    Ingrediente,
    Comida,
    Agua,
    Peso,
    Entrenamiento,
    Ayuno,
    Diario,
    AccionNibi,
    Tratamiento,
    Toma,
    Ciclo,
    Digestion,
    Cafeina,
    Plan,
    Meta,
    SesionCuidado,
    Habito,
    HabitoHecho,
    Chequeo,
    ChequeoHecho,
    Sintoma,
    Signo,
    Consumo,
    Bienestar,
    Libre,
    EventoLibre,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Operacion {
    Guardar,
    Eliminar,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cambio {
    pub id: Identificador,
    pub registro: Identificador,
    pub entidad: Entidad,
    pub base: u64,
    pub operacion: Operacion,
    pub datos: Option<Map<String, Value>>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lote {
    pub version: u32,
    pub nodo: Identificador,
    pub lote: Identificador,
    pub cambios: Vec<Cambio>,
}
impl std::fmt::Debug for Lote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lote")
            .field("version", &self.version)
            .field("cantidad", &self.cambios.len())
            .finish_non_exhaustive()
    }
}
fn estructura_acotada(valor: &Value, profundidad: usize) -> bool {
    if profundidad > 16 {
        return false;
    }
    match valor {
        Value::Array(v) => {
            v.len() <= 512 && v.iter().all(|x| estructura_acotada(x, profundidad + 1))
        }
        Value::Object(v) => {
            v.len() <= 128 && v.values().all(|x| estructura_acotada(x, profundidad + 1))
        }
        Value::Number(n) => n.as_f64().is_some_and(|n| n.abs() <= MAX_REVISION as f64),
        _ => true,
    }
}
impl Lote {
    /// Valida el sobre, no las reglas sanitarias de cada entidad ni la autorización del peer.
    /// # Errors
    /// [`ScError::InvalidInput`] ante versiones, tamaños o formas no admitidas.
    pub fn validar(&self) -> ScResult<()> {
        if self.version != VERSION || self.cambios.is_empty() || self.cambios.len() > MAX_CAMBIOS {
            return Err(ScError::InvalidInput);
        }
        let mut vistos = HashSet::new();
        for cambio in &self.cambios {
            if !vistos.insert(&cambio.id) || cambio.base > MAX_REVISION {
                return Err(ScError::InvalidInput);
            }
            match (cambio.operacion, &cambio.datos) {
                (Operacion::Guardar, Some(datos)) if !datos.is_empty() => {
                    let valor = Value::Object(datos.clone());
                    if !estructura_acotada(&valor, 0)
                        || serde_json::to_vec(datos)
                            .map_err(|_| ScError::InvalidInput)?
                            .len()
                            > MAX_REGISTRO_BYTES
                    {
                        return Err(ScError::InvalidInput);
                    }
                }
                (Operacion::Eliminar, None) => {}
                _ => return Err(ScError::InvalidInput),
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ejemplo() -> Value {
        serde_json::from_str(include_str!(
            "../../tests/fixtures/sincronizacion/lote-v1.json"
        ))
        .unwrap()
    }
    fn valida(valor: Value) -> bool {
        serde_json::from_value::<Lote>(valor).is_ok_and(|lote| lote.validar().is_ok())
    }
    #[test]
    fn el_lote_grabado_conserva_identidad_revision_y_borrado() {
        let valor = ejemplo();
        let lote: Lote = serde_json::from_value(valor.clone()).unwrap();
        lote.validar().unwrap();
        assert_eq!(serde_json::to_value(lote).unwrap(), valor);
    }
    #[test]
    fn un_reintento_no_puede_repetir_operaciones_en_el_mismo_lote() {
        let mut v = ejemplo();
        let primero = v["cambios"][0].clone();
        v["cambios"].as_array_mut().unwrap().push(primero);
        assert!(!valida(v));
    }
    #[test]
    fn se_rechazan_versiones_identificadores_y_revisiones_incompatibles() {
        for (campo, valor) in [
            ("version", serde_json::json!(2)),
            ("nodo", serde_json::json!("../privado")),
        ] {
            let mut v = ejemplo();
            v[campo] = valor;
            assert!(!valida(v));
        }
        let mut v = ejemplo();
        v["cambios"][0]["base"] = serde_json::json!(MAX_REVISION + 1);
        assert!(!valida(v));
    }
    #[test]
    fn un_borrado_no_admite_datos_y_un_guardado_no_puede_vaciar_el_registro() {
        let mut v = ejemplo();
        v["cambios"][1]["datos"] = serde_json::json!({"texto":"privado"});
        assert!(!valida(v));
        let mut v = ejemplo();
        v["cambios"][0]["datos"] = serde_json::json!({});
        assert!(!valida(v));
    }
    #[test]
    fn las_cotas_se_miden_en_bytes_y_no_en_caracteres() {
        let mut v = ejemplo();
        v["cambios"][0]["datos"] = serde_json::json!({"texto":"ñ".repeat(40_000)});
        assert!(!valida(v));
    }
    #[test]
    fn no_se_aceptan_campos_desconocidos_ni_se_revela_contenido_en_debug() {
        let mut v = ejemplo();
        v["secreto"] = serde_json::json!(true);
        assert!(!valida(v));
        let lote: Lote = serde_json::from_value(ejemplo()).unwrap();
        let depuracion = format!("{lote:?}");
        assert!(!depuracion.contains("mililitros"));
        assert!(!depuracion.contains("11111111"));
    }
}
