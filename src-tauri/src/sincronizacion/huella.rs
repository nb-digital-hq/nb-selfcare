//! Huella estable de una operación validada, independiente de la escritura del JSON.
use super::contrato::{Cambio, Identificador, Lote, VERSION};
use crate::error::{ScError, ScResult};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// No autoriza al nodo ni aplica datos. La huella tampoco debe escribirse en registros.
/// # Errors
/// [`ScError::InvalidInput`] si el cambio incumple el contrato.
pub fn huella_del_cambio(nodo: &Identificador, cambio: &Cambio) -> ScResult<[u8; 32]> {
    Lote {
        version: VERSION,
        nodo: nodo.clone(),
        lote: cambio.id.clone(),
        cambios: vec![cambio.clone()],
    }
    .validar()?;
    let mut bytes = b"NB-SelfCare/operacion/1\0".to_vec();
    codificar(
        &serde_json::json!({"nodo": nodo, "cambio": cambio}),
        &mut bytes,
    )?;
    Ok(Sha256::digest(bytes).into())
}

fn longitud(n: usize, salida: &mut Vec<u8>) -> ScResult<()> {
    salida.extend_from_slice(
        &u32::try_from(n)
            .map_err(|_| ScError::InvalidInput)?
            .to_be_bytes(),
    );
    Ok(())
}

fn codificar(valor: &Value, salida: &mut Vec<u8>) -> ScResult<()> {
    match valor {
        Value::Null => salida.push(0),
        Value::Bool(false) => salida.push(1),
        Value::Bool(true) => salida.push(2),
        Value::Number(n) => {
            salida.push(3);
            let n = n.as_f64().ok_or(ScError::InvalidInput)?;
            // Un cero negativo no representa una edición distinta del mismo valor numérico.
            salida.extend_from_slice(&(if n == 0.0 { 0.0 } else { n }).to_be_bytes());
        }
        Value::String(s) => {
            salida.push(4);
            longitud(s.len(), salida)?;
            salida.extend_from_slice(s.as_bytes());
        }
        Value::Array(elementos) => {
            salida.push(5);
            longitud(elementos.len(), salida)?;
            for elemento in elementos {
                codificar(elemento, salida)?;
            }
        }
        Value::Object(campos) => {
            salida.push(6);
            longitud(campos.len(), salida)?;
            let mut pares: Vec<_> = campos.iter().collect();
            pares.sort_unstable_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            for (clave, dato) in pares {
                codificar(&Value::String(clave.clone()), salida)?;
                codificar(dato, salida)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_huellas_coinciden_con_los_vectores_independientes_y_el_movil() {
        let casos: Vec<Value> = serde_json::from_str(include_str!(
            "../../tests/fixtures/sincronizacion/huellas-v1.json"
        ))
        .unwrap();
        for caso in casos {
            let nodo = serde_json::from_value(caso["nodo"].clone()).unwrap();
            let cambio = serde_json::from_value(caso["cambio"].clone()).unwrap();
            let huella = huella_del_cambio(&nodo, &cambio).unwrap();
            let esperada = caso["sha256"].as_str().unwrap();
            assert_eq!(esperada.len(), 64);
            for (byte, par) in huella.iter().zip(esperada.as_bytes().as_chunks::<2>().0) {
                assert_eq!(
                    *byte,
                    u8::from_str_radix(std::str::from_utf8(par).unwrap(), 16).unwrap()
                );
            }
        }
    }

    #[test]
    fn una_huella_distingue_contenido_revisiones_identidades_y_tipo_de_valor() {
        let nodo = Identificador::try_from("1".repeat(32)).unwrap();
        let valor = serde_json::json!({"id":"3".repeat(32),"registro":"5".repeat(32),"entidad":"agua","base":0,"operacion":"guardar","datos":{"valor":1}});
        let huella = |v| huella_del_cambio(&nodo, &serde_json::from_value(v).unwrap()).unwrap();
        let original = huella(valor.clone());
        for (campo, dato) in [
            ("id", Value::String("4".repeat(32))),
            ("registro", Value::String("6".repeat(32))),
            ("entidad", Value::String("diario".into())),
            ("base", serde_json::json!(1)),
            ("datos", serde_json::json!({"valor":"1"})),
        ] {
            let mut alterado = valor.clone();
            alterado[campo] = dato;
            assert_ne!(original, huella(alterado));
        }
        let cambio: Cambio = serde_json::from_value(valor.clone()).unwrap();
        assert_ne!(
            original,
            huella_del_cambio(&Identificador::try_from("2".repeat(32)).unwrap(), &cambio).unwrap()
        );
        let mut equivalente = valor;
        equivalente["datos"]["valor"] = serde_json::json!(1.0);
        assert_eq!(original, huella(equivalente));
    }

    #[test]
    fn se_rechazan_numeros_que_perderian_precision_entre_propietarios() {
        let nodo = Identificador::try_from("1".repeat(32)).unwrap();
        for numero in [
            serde_json::json!(9_007_199_254_740_992_u64),
            serde_json::json!(-9_007_199_254_740_992_i64),
            serde_json::json!(1e100),
        ] {
            let cambio: Cambio = serde_json::from_value(serde_json::json!({"id":"3".repeat(32),"registro":"5".repeat(32),"entidad":"agua","base":0,"operacion":"guardar","datos":{"valor":numero}})).unwrap();
            assert!(matches!(
                huella_del_cambio(&nodo, &cambio),
                Err(ScError::InvalidInput)
            ));
        }
    }
}
