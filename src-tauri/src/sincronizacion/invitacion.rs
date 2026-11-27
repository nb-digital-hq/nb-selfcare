//! Sobre QR propio de SelfCare. No abre sockets ni contiene la clave TLS privada.
use super::emparejamiento::Credencial;
use crate::error::{ScError, ScResult};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use qrcode::{Color, QrCode};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Solo direcciones numéricas locales: sin DNS, redirecciones ni destinos públicos.
pub fn direccion_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local(),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SobreInvitacion {
    pub app: String,
    pub version: u32,
    pub host: IpAddr,
    pub puerto: u16,
    pub huella: String,
    pub credencial: String,
}
impl SobreInvitacion {
    /// La identidad pública debe corresponder a la configuración TLS del listener.
    /// # Errors
    /// [`ScError::InvalidInput`] si el destino o la huella no son válidos.
    pub fn crear(
        host: IpAddr,
        puerto: u16,
        huella: &str,
        credencial: &Credencial,
    ) -> ScResult<Self> {
        let sobre = Self {
            app: "nb-selfcare".into(),
            version: 1,
            host,
            puerto,
            huella: huella.into(),
            credencial: credencial.exponer().into(),
        };
        sobre.validar()?;
        Ok(sobre)
    }

    /// # Errors
    /// [`ScError::InvalidInput`] ante un sobre incompatible o un destino no local.
    pub fn validar(&self) -> ScResult<()> {
        if self.app != "nb-selfcare"
            || self.version != 1
            || !direccion_local(self.host)
            || self.puerto == 0
            || self.huella.len() != 44
            || STANDARD.decode(&self.huella).map_or(true, |bytes| {
                bytes.len() != 32 || STANDARD.encode(bytes) != self.huella
            })
            || self.credencial.len() != 43
            || base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(&self.credencial)
                .map_or(true, |bytes| {
                    bytes.len() != 32
                        || base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
                            != self.credencial
                })
        {
            return Err(ScError::InvalidInput);
        }
        Ok(())
    }

    /// Matriz sin colores, escalado ni margen: la presentación añade cuatro módulos de zona libre.
    /// # Errors
    /// [`ScError::InvalidInput`] si el sobre no se puede codificar.
    pub fn matriz_qr(&self) -> ScResult<Vec<Vec<bool>>> {
        self.validar()?;
        let json = serde_json::to_vec(self).map_err(|_| ScError::InvalidInput)?;
        let qr = QrCode::new(json).map_err(|_| ScError::InvalidInput)?;
        Ok(qr
            .to_colors()
            .chunks(qr.width())
            .map(|fila| fila.iter().map(|c| *c == Color::Dark).collect())
            .collect())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn la_invitacion_compartida_con_mobile_conserva_los_campos() {
        let valor: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/sincronizacion/invitacion-v1.json"
        ))
        .unwrap();
        let sobre: SobreInvitacion = serde_json::from_value(valor.clone()).unwrap();
        sobre.validar().unwrap();
        assert_eq!(serde_json::to_value(sobre).unwrap(), valor);
    }

    #[test]
    fn no_se_puede_usar_el_qr_para_alcanzar_redes_publicas_o_multicast() {
        for ip in [
            "8.8.8.8",
            "0.0.0.0",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "ff02::1",
            "2001:4860:4860::8888",
            "::ffff:192.168.1.2",
        ] {
            assert!(!direccion_local(ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "127.0.0.1",
            "192.168.1.4",
            "10.1.2.3",
            "172.16.1.4",
            "169.254.1.4",
            "::1",
            "fd12::4",
            "fe80::4",
        ] {
            assert!(direccion_local(ip.parse().unwrap()), "{ip}");
        }
    }
    #[test]
    fn el_sobre_cabe_en_un_qr_y_solo_expone_una_huella_publica() {
        let token = Credencial::generar().unwrap();
        let sobre = SobreInvitacion::crear(
            "192.168.1.4".parse().unwrap(),
            43127,
            &STANDARD.encode([7; 32]),
            &token,
        )
        .unwrap();
        let matriz = sobre.matriz_qr().unwrap();
        assert!((21..=177).contains(&matriz.len()));
        assert!(matriz.iter().all(|fila| fila.len() == matriz.len()));
        let valor = serde_json::to_value(sobre).unwrap();
        assert_eq!(valor.as_object().unwrap().len(), 6);
        assert!(valor.get("clave").is_none());
    }
}
