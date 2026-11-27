//! Identidad TLS persistente del dispositivo, separada de SQLite y las copias.
use crate::error::{ScError, ScResult};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Datelike as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::sync::Arc;
use tokio_rustls::rustls::{
    self,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
};

const SERVICIO: &str = "org.nbdigital.selfcare.sync";
const CUENTA: &str = "identidad-tls-v1";
const MAX_SECRETO: usize = 16 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Secreto {
    version: u8,
    certificado: String,
    clave: String,
}

/// No expone la clave ni permite serializar o depurar accidentalmente la identidad.
pub struct IdentidadTls {
    secreto: Secreto,
    configuracion: Arc<rustls::ServerConfig>,
    huella: String,
}
impl IdentidadTls {
    /// Obtiene o crea la identidad en el llavero. Un error nunca regenera silenciosamente el pin.
    /// Debe ejecutarse fuera del hilo de ventana; no autoriza abrir un listener.
    /// # Errors
    /// [`ScError::ComponentUnavailable`] si el llavero no está disponible;
    /// [`ScError::StorageFailure`] si falla la operación o el secreto existente está dañado.
    pub fn cargar_o_crear() -> ScResult<Self> {
        let entrada =
            keyring::v1::Entry::new(SERVICIO, CUENTA).map_err(|_| ScError::ComponentUnavailable)?;
        match entrada.get_password() {
            Ok(valor) => Self::desde_secreto(&valor),
            Err(keyring::v1::Error::NoEntry) => {
                let identidad = Self::generar()?;
                let valor = serde_json::to_string(&identidad.secreto)
                    .map_err(|_| ScError::StorageFailure)?;
                entrada
                    .set_password(&valor)
                    .map_err(|_| ScError::StorageFailure)?;
                Ok(identidad)
            }
            Err(_) => Err(ScError::StorageFailure),
        }
    }

    #[cfg(test)]
    pub(crate) fn efimera_de_prueba() -> Self {
        Self::generar().expect("identidad efímera")
    }

    fn generar() -> ScResult<Self> {
        let mut parametros = rcgen::CertificateParams::new(vec!["nbselfcare.local".to_owned()])
            .map_err(|_| ScError::ComponentUnavailable)?;
        let ano = chrono::Utc::now().year();
        parametros.not_before = rcgen::date_time_ymd(ano - 1, 1, 1);
        parametros.not_after = rcgen::date_time_ymd(ano + 10, 1, 1);
        let clave = rcgen::KeyPair::generate().map_err(|_| ScError::ComponentUnavailable)?;
        let certificado = parametros
            .self_signed(&clave)
            .map_err(|_| ScError::ComponentUnavailable)?;
        Self::construir(Secreto {
            version: 1,
            certificado: STANDARD.encode(certificado.der()),
            clave: STANDARD.encode(clave.serialize_der()),
        })
    }

    fn desde_secreto(valor: &str) -> ScResult<Self> {
        if valor.len() > MAX_SECRETO {
            return Err(ScError::StorageFailure);
        }
        Self::construir(serde_json::from_str(valor).map_err(|_| ScError::StorageFailure)?)
    }

    fn construir(secreto: Secreto) -> ScResult<Self> {
        if secreto.version != 1 {
            return Err(ScError::StorageFailure);
        }
        let certificado = STANDARD
            .decode(&secreto.certificado)
            .map_err(|_| ScError::StorageFailure)?;
        let clave = STANDARD
            .decode(&secreto.clave)
            .map_err(|_| ScError::StorageFailure)?;
        let huella = STANDARD.encode(Sha256::digest(&certificado));
        // Proveedor explícito: no modifica el proveedor global de Tauri/ureq.
        // with_single_cert comprueba que certificado y clave privada se corresponden.
        let configuracion = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| ScError::StorageFailure)?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(certificado)],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(clave)),
        )
        .map_err(|_| ScError::StorageFailure)?;
        Ok(Self {
            secreto,
            configuracion: Arc::new(configuracion),
            huella,
        })
    }

    pub fn configuracion(&self) -> Arc<rustls::ServerConfig> {
        Arc::clone(&self.configuracion)
    }

    /// SHA-256 del DER en base64 estándar; el cliente debe fijarla en cada conexión.
    pub fn huella(&self) -> &str {
        &self.huella
    }

    /// Certificado público DER en base64, para establecer la confianza durante el emparejamiento.
    pub fn certificado(&self) -> &str {
        &self.secreto.certificado
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn restaurar_el_secreto_conserva_el_pin_y_no_regenera_la_identidad() {
        let identidad = IdentidadTls::generar().unwrap();
        let secreto = serde_json::to_string(&identidad.secreto).unwrap();
        let restaurada = IdentidadTls::desde_secreto(&secreto).unwrap();
        assert_eq!(identidad.huella(), restaurada.huella());
        assert_eq!(identidad.certificado(), restaurada.certificado());
    }

    #[test]
    fn certificado_y_clave_de_dispositivos_distintos_se_rechazan() {
        let mut identidad = IdentidadTls::generar().unwrap();
        let otra = IdentidadTls::generar().unwrap();
        identidad.secreto.clave = otra.secreto.clave;
        assert!(matches!(
            IdentidadTls::construir(identidad.secreto),
            Err(ScError::StorageFailure)
        ));
    }

    #[test]
    fn un_secreto_danado_no_se_sustituye_por_otro() {
        for valor in ["{}", "no-json", &"a".repeat(MAX_SECRETO + 1)] {
            assert!(matches!(
                IdentidadTls::desde_secreto(valor),
                Err(ScError::StorageFailure)
            ));
        }
        let mut identidad = IdentidadTls::generar().unwrap();
        identidad.secreto.version = 2;
        assert!(matches!(
            IdentidadTls::construir(identidad.secreto),
            Err(ScError::StorageFailure)
        ));
    }
}
