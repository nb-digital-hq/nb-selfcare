//! Servicio de conexión con SelfCare Mobile: abre la puerta TLS local, invita, revoca y
//! atiende la sincronización. Solo funciona con el consentimiento global y del servicio.
use super::contrato::Identificador;
use super::emparejamiento::Credencial;
use super::identidad::IdentidadTls;
use super::invitacion::SobreInvitacion;
use super::pares::{GestorDePares, LlaveroDePares};
use crate::db::BaseDeDatos;
use crate::error::{ScError, ScResult};
use crate::net::{PuertaLocal, Red, Servicio};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use ts_rs::TS;

/// Puerto preferido; si está ocupado se usa otro y el teléfono lo reencuentra por mDNS.
const PUERTO: u16 = 47_820;

struct Activo {
    puerta: PuertaLocal,
    gestor: Arc<GestorDePares>,
    huella: String,
}

/// Estado compartido del enlace dentro del runtime.
#[derive(Default)]
pub struct Enlace {
    activo: tokio::sync::Mutex<Option<Activo>>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct ParEnlazado {
    pub nodo: String,
    /// Últimos caracteres, para distinguir teléfonos sin mostrar la identidad completa.
    pub alias: String,
    pub ultima: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct EstadoEnlace {
    pub permitido: bool,
    pub activo: bool,
    pub direccion: Option<String>,
    pub pares: Vec<ParEnlazado>,
    pub por_revisar: u32,
    pub registros: u32,
    pub ultima: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct InvitacionEnlace {
    /// Matriz del QR: filas de módulos oscuros.
    pub qr: Vec<Vec<bool>>,
    /// El mismo contenido del QR, para teléfonos sin cámara disponible. De un solo uso.
    pub texto: String,
    pub direccion: String,
    pub caduca_en_segundos: u32,
}

/// Aviso a la interfaz cuando un teléfono aplicó cambios.
pub type Avisar = Arc<dyn Fn() + Send + Sync>;

impl Enlace {
    /// Abre la puerta si el consentimiento lo permite. Idempotente.
    ///
    /// # Errors
    /// [`ScError::ConsentRequired`] sin permiso; errores de red o del llavero.
    pub async fn abrir(&self, red: Arc<Red>, db: BaseDeDatos, avisar: Avisar) -> ScResult<()> {
        red.politica().comprobar(Servicio::Sincronizacion)?;
        let mut activo = self.activo.lock().await;
        if activo.is_some() {
            return Ok(());
        }
        let (identidad, gestor) = tokio::task::spawn_blocking(|| {
            let identidad = IdentidadTls::cargar_o_crear()?;
            let gestor = GestorDePares::cargar(Box::new(LlaveroDePares))?;
            Ok::<_, ScError>((identidad, gestor))
        })
        .await
        .map_err(|_| ScError::Internal)??;
        let gestor = Arc::new(gestor);
        let ip = crate::net::interfaces_locales()?
            .into_iter()
            .next()
            .ok_or(ScError::NetworkFailure)?;
        let router = crate::net::router_del_enlace(Arc::clone(&gestor), db, avisar);
        let puerta = match PuertaLocal::abrir(
            Arc::clone(&red),
            std::net::SocketAddr::new(ip, PUERTO),
            &identidad,
            router.clone(),
            true,
        )
        .await
        {
            Ok(p) => p,
            Err(ScError::NetworkFailure) => {
                PuertaLocal::abrir(
                    red,
                    std::net::SocketAddr::new(ip, 0),
                    &identidad,
                    router,
                    true,
                )
                .await?
            }
            Err(e) => return Err(e),
        };
        tracing::info!("enlace local abierto");
        *activo = Some(Activo {
            puerta,
            gestor,
            huella: identidad.huella().to_owned(),
        });
        Ok(())
    }

    /// Cierra la puerta. Los pares siguen autorizados para la próxima vez.
    pub async fn cerrar(&self) {
        if let Some(a) = self.activo.lock().await.take() {
            // Soltar la puerta la detiene y retira el anuncio mDNS.
            drop(a);
            tracing::info!("enlace local cerrado");
        }
    }

    /// # Errors
    /// [`ScError::ConsentRequired`] si el enlace no está abierto.
    pub async fn invitar(&self) -> ScResult<InvitacionEnlace> {
        let activo = self.activo.lock().await;
        let a = activo.as_ref().ok_or(ScError::ConsentRequired)?;
        let credencial: Credencial = a.gestor.invitar(Instant::now())?;
        let direccion = a.puerta.direccion();
        let sobre =
            SobreInvitacion::crear(direccion.ip(), direccion.port(), &a.huella, &credencial)?;
        Ok(InvitacionEnlace {
            qr: sobre.matriz_qr()?,
            texto: serde_json::to_string(&sobre).map_err(|_| ScError::Internal)?,
            direccion: direccion.to_string(),
            caduca_en_segundos: u32::try_from(Duration::from_secs(300).as_secs()).unwrap_or(300),
        })
    }

    /// # Errors
    /// [`ScError::InvalidInput`] si la identidad no es válida; errores del llavero.
    pub async fn revocar(&self, nodo: String) -> ScResult<()> {
        let nodo = Identificador::try_from(nodo)?;
        let gestor = match self.activo.lock().await.as_ref() {
            Some(a) => Arc::clone(&a.gestor),
            None => Arc::new(
                tokio::task::spawn_blocking(|| GestorDePares::cargar(Box::new(LlaveroDePares)))
                    .await
                    .map_err(|_| ScError::Internal)??,
            ),
        };
        tokio::task::spawn_blocking(move || gestor.revocar(&nodo))
            .await
            .map_err(|_| ScError::Internal)?
    }

    /// # Errors
    /// Errores de almacenamiento canónicos.
    pub async fn estado(&self, red: &Red, db: BaseDeDatos) -> ScResult<EstadoEnlace> {
        let permitido = red.politica().comprobar(Servicio::Sincronizacion).is_ok();
        let (activo, direccion, gestor) = match self.activo.lock().await.as_ref() {
            Some(a) => (
                true,
                Some(a.puerta.direccion().to_string()),
                Some(Arc::clone(&a.gestor)),
            ),
            None => (false, None, None),
        };
        tokio::task::spawn_blocking(move || {
            let nodos = match gestor {
                Some(g) => g.pares()?,
                // Sin enlace abierto no se toca el llavero solo para listar.
                None => Vec::new(),
            };
            let mut conn = db.conexion()?;
            let resumen = super::servicio::estado(&mut conn)?;
            let pares = nodos
                .into_iter()
                .map(|n| {
                    let ultima = conn
                        .query_row(
                            "SELECT ultima FROM sc_sync_pares WHERE nodo = ?1",
                            [n.exponer()],
                            |f| f.get::<_, Option<String>>(0),
                        )
                        .ok()
                        .flatten();
                    ParEnlazado {
                        alias: n.exponer()[28..].to_uppercase(),
                        nodo: n.exponer().to_owned(),
                        ultima,
                    }
                })
                .collect();
            Ok(EstadoEnlace {
                permitido,
                activo,
                direccion,
                pares,
                por_revisar: resumen.pendientes_de_revisar,
                registros: resumen.registros,
                ultima: resumen.ultima,
            })
        })
        .await
        .map_err(|_| ScError::Internal)?
    }
}
