//! Autoridad del dispositivo fuera de SQLite: pares revocables y huellas de credenciales.
use super::{
    contrato::{Identificador, VERSION},
    emparejamiento::{Credencial, Invitacion},
};
use crate::error::{ScError, ScResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{collections::HashSet, sync::Mutex, time::Instant};

const MAX_PARES: usize = 8;
const MAX_ALMACEN: usize = 16 * 1024;

pub trait AlmacenDePares: Send + Sync {
    fn leer(&self) -> ScResult<Option<String>>;
    fn guardar(&self, valor: &str) -> ScResult<()>;
}
pub struct LlaveroDePares;
impl LlaveroDePares {
    fn entrada() -> ScResult<keyring::v1::Entry> {
        keyring::v1::Entry::new("org.nbdigital.selfcare.sync", "pares-v1")
            .map_err(|_| ScError::ComponentUnavailable)
    }
}
impl AlmacenDePares for LlaveroDePares {
    fn leer(&self) -> ScResult<Option<String>> {
        match Self::entrada()?.get_password() {
            Ok(valor) => Ok(Some(valor)),
            Err(keyring::v1::Error::NoEntry) => Ok(None),
            Err(_) => Err(ScError::StorageFailure),
        }
    }
    fn guardar(&self, valor: &str) -> ScResult<()> {
        Self::entrada()?
            .set_password(valor)
            .map_err(|_| ScError::StorageFailure)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Par {
    nodo: Identificador,
    huella: [u8; 32],
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registro {
    version: u32,
    nodo: Identificador,
    pares: Vec<Par>,
}
struct Interior {
    registro: Registro,
    invitacion: Option<Invitacion>,
}
/// Serializa altas y revocaciones. No confirma un cambio hasta que el llavero lo persiste.
/// Sus métodos bloqueantes se llaman mediante spawn_blocking, nunca desde el hilo de ventana.
pub struct GestorDePares {
    almacen: Box<dyn AlmacenDePares>,
    interior: Mutex<Interior>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolicitudDeAlta {
    pub version: u32,
    pub nodo: Identificador,
    pub credencial: String,
}
#[derive(Serialize)]
pub struct AltaDePar {
    pub version: u32,
    pub nodo: Identificador,
    pub credencial: String,
}
impl GestorDePares {
    /// # Errors
    /// Un almacén inaccesible o inválido no se sustituye por otro vacío.
    pub fn cargar(almacen: Box<dyn AlmacenDePares>) -> ScResult<Self> {
        let registro = match almacen.leer()? {
            Some(valor) => {
                if valor.len() > MAX_ALMACEN {
                    return Err(ScError::StorageFailure);
                }
                let registro: Registro =
                    serde_json::from_str(&valor).map_err(|_| ScError::StorageFailure)?;
                let mut vistos = HashSet::new();
                if registro.version != VERSION
                    || registro.pares.len() > MAX_PARES
                    || registro
                        .pares
                        .iter()
                        .any(|p| p.nodo == registro.nodo || !vistos.insert(&p.nodo))
                {
                    return Err(ScError::StorageFailure);
                }
                registro
            }
            None => {
                let registro = Registro {
                    version: VERSION,
                    nodo: Identificador::generar()?,
                    pares: Vec::new(),
                };
                guardar(almacen.as_ref(), &registro)?;
                registro
            }
        };
        Ok(Self {
            almacen,
            interior: Mutex::new(Interior {
                registro,
                invitacion: None,
            }),
        })
    }
    /// Identidades de los teléfonos autorizados, sin credenciales.
    /// # Errors
    /// [`ScError::Internal`] si el estado interno quedó envenenado.
    pub fn pares(&self) -> ScResult<Vec<Identificador>> {
        let interior = self.interior.lock().map_err(|_| ScError::Internal)?;
        Ok(interior
            .registro
            .pares
            .iter()
            .map(|p| p.nodo.clone())
            .collect())
    }
    /// Crear una invitación nueva invalida la anterior. No modifica los pares existentes.
    /// # Errors
    /// Errores canónicos de entropía o estado interno.
    pub fn invitar(&self, ahora: Instant) -> ScResult<Credencial> {
        let (invitacion, token) = Invitacion::crear(ahora)?;
        self.interior
            .lock()
            .map_err(|_| ScError::Internal)?
            .invitacion = Some(invitacion);
        Ok(token)
    }
    /// # Errors
    /// Rechaza invitaciones inválidas y conserva la autoridad previa si falla el llavero.
    pub fn aceptar(&self, solicitud: SolicitudDeAlta, ahora: Instant) -> ScResult<AltaDePar> {
        if solicitud.version != VERSION {
            return Err(ScError::InvalidInput);
        }
        let mut interior = self.interior.lock().map_err(|_| ScError::Internal)?;
        interior
            .invitacion
            .as_mut()
            .ok_or(ScError::ConsentRequired)?
            .consumir(&solicitud.credencial, ahora)?;
        if solicitud.nodo == interior.registro.nodo {
            return Err(ScError::InvalidInput);
        }
        let mut registro = interior.registro.clone();
        registro.pares.retain(|p| p.nodo != solicitud.nodo);
        if registro.pares.len() >= MAX_PARES {
            return Err(ScError::InvalidInput);
        }
        let token = Credencial::generar()?;
        registro.pares.push(Par {
            nodo: solicitud.nodo,
            huella: Sha256::digest(token.exponer().as_bytes()).into(),
        });
        guardar(self.almacen.as_ref(), &registro)?;
        let respuesta = AltaDePar {
            version: VERSION,
            nodo: registro.nodo.clone(),
            credencial: token.exponer().into(),
        };
        interior.registro = registro;
        Ok(respuesta)
    }
    /// # Errors
    /// No revela si existe el identificador o si la credencial es incorrecta.
    pub fn autenticar(&self, nodo: &Identificador, token: &str) -> ScResult<()> {
        if token.len() != 43 {
            return Err(ScError::ConsentRequired);
        }
        let huella: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        let interior = self.interior.lock().map_err(|_| ScError::Internal)?;
        if interior
            .registro
            .pares
            .iter()
            .any(|p| &p.nodo == nodo && p.huella == huella)
        {
            Ok(())
        } else {
            Err(ScError::ConsentRequired)
        }
    }
    /// # Errors
    /// Si la persistencia falla, no confirma una revocación que reaparecería tras reiniciar.
    pub fn revocar(&self, nodo: &Identificador) -> ScResult<()> {
        let mut interior = self.interior.lock().map_err(|_| ScError::Internal)?;
        let mut registro = interior.registro.clone();
        registro.pares.retain(|p| &p.nodo != nodo);
        guardar(self.almacen.as_ref(), &registro)?;
        interior.registro = registro;
        Ok(())
    }
}
fn guardar(almacen: &dyn AlmacenDePares, registro: &Registro) -> ScResult<()> {
    let valor = serde_json::to_string(registro).map_err(|_| ScError::StorageFailure)?;
    if valor.len() > MAX_ALMACEN {
        return Err(ScError::StorageFailure);
    }
    almacen.guardar(&valor)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    #[derive(Clone, Default)]
    struct Memoria {
        valor: Arc<Mutex<Option<String>>>,
        falla: Arc<AtomicBool>,
    }
    impl AlmacenDePares for Memoria {
        fn leer(&self) -> ScResult<Option<String>> {
            Ok(self.valor.lock().unwrap().clone())
        }
        fn guardar(&self, valor: &str) -> ScResult<()> {
            if self.falla.load(Ordering::SeqCst) {
                return Err(ScError::StorageFailure);
            }
            *self.valor.lock().unwrap() = Some(valor.into());
            Ok(())
        }
    }
    fn alta(gestor: &GestorDePares, nodo: &Identificador) -> AltaDePar {
        let ahora = Instant::now();
        let token = gestor.invitar(ahora).unwrap();
        gestor
            .aceptar(
                SolicitudDeAlta {
                    version: VERSION,
                    nodo: nodo.clone(),
                    credencial: token.exponer().into(),
                },
                ahora,
            )
            .unwrap()
    }
    #[test]
    fn reiniciar_conserva_autoridad_y_revocar_invalida_el_token() {
        let memoria = Memoria::default();
        let gestor = GestorDePares::cargar(Box::new(memoria.clone())).unwrap();
        let nodo = Identificador::generar().unwrap();
        let par = alta(&gestor, &nodo);
        assert!(!memoria.leer().unwrap().unwrap().contains(&par.credencial));
        let reiniciado = GestorDePares::cargar(Box::new(memoria)).unwrap();
        reiniciado.autenticar(&nodo, &par.credencial).unwrap();
        reiniciado.revocar(&nodo).unwrap();
        assert_eq!(
            reiniciado.autenticar(&nodo, &par.credencial),
            Err(ScError::ConsentRequired)
        );
    }
    #[test]
    fn emparejar_de_nuevo_sustituye_el_token_previo() {
        let gestor = GestorDePares::cargar(Box::<Memoria>::default()).unwrap();
        let nodo = Identificador::generar().unwrap();
        let anterior = alta(&gestor, &nodo);
        let nueva = alta(&gestor, &nodo);
        assert_eq!(
            gestor.autenticar(&nodo, &anterior.credencial),
            Err(ScError::ConsentRequired)
        );
        gestor.autenticar(&nodo, &nueva.credencial).unwrap();
    }
    #[test]
    fn una_escritura_rechazada_no_confirma_una_autoridad_nueva() {
        let memoria = Memoria::default();
        let gestor = GestorDePares::cargar(Box::new(memoria.clone())).unwrap();
        let nodo = Identificador::generar().unwrap();
        let par = alta(&gestor, &nodo);
        memoria.falla.store(true, Ordering::SeqCst);
        assert_eq!(gestor.revocar(&nodo), Err(ScError::StorageFailure));
        gestor.autenticar(&nodo, &par.credencial).unwrap();
    }
    #[test]
    fn una_nueva_invitacion_invalida_la_anterior_y_hay_ocho_cupos() {
        let gestor = GestorDePares::cargar(Box::<Memoria>::default()).unwrap();
        let ahora = Instant::now();
        let anterior = gestor.invitar(ahora).unwrap();
        let _nueva = gestor.invitar(ahora).unwrap();
        assert!(matches!(
            gestor.aceptar(
                SolicitudDeAlta {
                    version: VERSION,
                    nodo: Identificador::generar().unwrap(),
                    credencial: anterior.exponer().into()
                },
                ahora
            ),
            Err(ScError::ConsentRequired)
        ));
        for _ in 0..MAX_PARES {
            alta(&gestor, &Identificador::generar().unwrap());
        }
        let token = gestor.invitar(ahora).unwrap();
        assert!(matches!(
            gestor.aceptar(
                SolicitudDeAlta {
                    version: VERSION,
                    nodo: Identificador::generar().unwrap(),
                    credencial: token.exponer().into()
                },
                ahora
            ),
            Err(ScError::InvalidInput)
        ));
    }
}
