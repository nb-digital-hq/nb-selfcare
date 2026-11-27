//! Invitaciones efímeras: la credencial solo se entrega una vez y no se registra.
use crate::error::{ScError, ScResult};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest as _, Sha256};
use std::time::{Duration, Instant};

const VIGENCIA: Duration = Duration::from_secs(300);
const MAX_INTENTOS: u8 = 5;

/// Token aleatorio de 256 bits. No implementa Debug, Clone ni serialización implícita.
pub struct Credencial(String);
impl Credencial {
    /// Genera una credencial mediante la fuente criptográfica del sistema.
    /// # Errors
    /// [`ScError::ComponentUnavailable`] si no hay entropía segura.
    pub fn generar() -> ScResult<Self> {
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).map_err(|_| ScError::ComponentUnavailable)?;
        Ok(Self(URL_SAFE_NO_PAD.encode(bytes)))
    }

    /// Solo para entregar por el canal cifrado o escribir en el almacén seguro.
    pub fn exponer(&self) -> &str {
        &self.0
    }
}

/// Conserva únicamente la huella; cada invitación admite una sola aceptación.
/// El propietario debe proteger esta instancia con exclusión mutua entre peticiones.
pub struct Invitacion {
    huella: [u8; 32],
    vence: Instant,
    intentos: u8,
    consumida: bool,
}
impl Invitacion {
    /// Crea una invitación de cinco minutos usando tiempo monótono.
    /// # Errors
    /// [`ScError::ComponentUnavailable`] si falla la generación segura.
    pub fn crear(ahora: Instant) -> ScResult<(Self, Credencial)> {
        let credencial = Credencial::generar()?;
        let invitacion = Self {
            huella: Sha256::digest(credencial.exponer().as_bytes()).into(),
            vence: ahora + VIGENCIA,
            intentos: 0,
            consumida: false,
        };
        Ok((invitacion, credencial))
    }

    /// Consume antes de continuar el alta del dispositivo; un fallo posterior requiere otra invitación.
    /// # Errors
    /// [`ScError::ConsentRequired`] si caducó, fue consumida o la credencial no coincide.
    pub fn consumir(&mut self, presentada: &str, ahora: Instant) -> ScResult<()> {
        if self.consumida || ahora >= self.vence || self.intentos >= MAX_INTENTOS {
            return Err(ScError::ConsentRequired);
        }
        self.intentos += 1;
        // Forma canónica: evita codificaciones alternativas y acota antes del hash.
        if presentada.len() != 43 {
            return Err(ScError::ConsentRequired);
        }
        let Ok(bytes) = URL_SAFE_NO_PAD.decode(presentada) else {
            return Err(ScError::ConsentRequired);
        };
        if bytes.len() != 32 || URL_SAFE_NO_PAD.encode(bytes) != presentada {
            return Err(ScError::ConsentRequired);
        }
        let recibida: [u8; 32] = Sha256::digest(presentada.as_bytes()).into();
        if recibida != self.huella {
            return Err(ScError::ConsentRequired);
        }
        self.consumida = true;
        Ok(())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn una_invitacion_solo_autoriza_un_alta() {
        let ahora = Instant::now();
        let (mut invitacion, token) = Invitacion::crear(ahora).unwrap();
        assert_eq!(URL_SAFE_NO_PAD.decode(token.exponer()).unwrap().len(), 32);
        assert!(invitacion.consumir(token.exponer(), ahora).is_ok());
        assert_eq!(
            invitacion.consumir(token.exponer(), ahora),
            Err(ScError::ConsentRequired)
        );
    }

    #[test]
    fn el_limite_de_caducidad_es_exclusivo() {
        let ahora = Instant::now();
        let (mut invitacion, token) = Invitacion::crear(ahora).unwrap();
        assert_eq!(
            invitacion.consumir(token.exponer(), ahora + VIGENCIA),
            Err(ScError::ConsentRequired)
        );
    }

    #[test]
    fn cinco_intentos_incorrectos_invalidan_incluso_el_token_original() {
        let ahora = Instant::now();
        let (mut invitacion, token) = Invitacion::crear(ahora).unwrap();
        for _ in 0..MAX_INTENTOS {
            assert_eq!(
                invitacion.consumir("incorrecta", ahora),
                Err(ScError::ConsentRequired)
            );
        }
        assert_eq!(
            invitacion.consumir(token.exponer(), ahora),
            Err(ScError::ConsentRequired)
        );
    }

    #[test]
    fn una_credencial_ajena_no_consume_la_invitacion() {
        let ahora = Instant::now();
        let (mut invitacion, token) = Invitacion::crear(ahora).unwrap();
        let ajena = Credencial::generar().unwrap();
        assert_ne!(token.exponer(), ajena.exponer());
        assert_eq!(
            invitacion.consumir(ajena.exponer(), ahora),
            Err(ScError::ConsentRequired)
        );
        assert!(invitacion.consumir(token.exponer(), ahora).is_ok());
    }
}
