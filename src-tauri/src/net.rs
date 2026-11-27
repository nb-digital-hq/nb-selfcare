//! La única puerta de red.
//!
//! Contrato: ADR-005 y la regla 3 de la Constitución. Toda petición saliente pasa por [`Red`], declara a qué servicio
//! va, y la puerta comprueba tres cosas antes de salir:
//!
//! 1. que el interruptor general esté encendido;
//! 2. que el usuario haya consentido ese servicio;
//! 3. que el anfitrión de la URL sea uno de los de ese servicio, para que una
//!    URL armada con datos de fuera (una respuesta de Open Food Facts, por ejemplo)
//!    no pueda sacar nada a otro sitio con el permiso de un servicio distinto.
//!
//! `scripts/check-network.mjs` rechaza cualquier uso del cliente HTTP fuera de
//! este archivo, así que esta lista es, literalmente, todo lo que la aplicación
//! puede contactar.

use std::collections::BTreeSet;
use std::io::Read as _;
use std::sync::RwLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{ScError, ScResult};

/// Un servicio externo. El orden es el de Ajustes > Privacidad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub enum Servicio {
    /// Datos de un producto envasado por su código de barras, cuando la persona
    /// lo busca y no está en el catálogo local (ADR-005).
    OpenFoodFacts,
    Actualizaciones,
    /// Enlace local cifrado con un dispositivo emparejado (ADR-014).
    Sincronizacion,
}

impl Servicio {
    pub const TODOS: [Servicio; 3] = [
        Servicio::OpenFoodFacts,
        Servicio::Actualizaciones,
        Servicio::Sincronizacion,
    ];

    /// Los anfitriones a los que puede ir cada servicio. Un subdominio cuenta
    /// solo si se declara con el punto delante (`.githubusercontent.com`).
    fn anfitriones(self) -> &'static [&'static str] {
        match self {
            Self::OpenFoodFacts => &["world.openfoodfacts.org"],
            // Se autoriza como dirección numérica local por la puerta TLS, nunca por HTTP genérico.
            Self::Sincronizacion => &[],
            // El manifiesto `latest.json` y los instaladores de las publicaciones; GitHub
            // redirige las descargas a `objects.githubusercontent.com`.
            Self::Actualizaciones => &["github.com", ".githubusercontent.com"],
        }
    }

    fn admite(self, anfitrion: &str) -> bool {
        let anfitrion = anfitrion.to_ascii_lowercase();
        self.anfitriones()
            .iter()
            .any(|permitido| match permitido.strip_prefix('.') {
                Some(dominio) => anfitrion == dominio || anfitrion.ends_with(permitido),
                None => anfitrion == *permitido,
            })
    }
}

/// Lo que el usuario consintió. De fábrica **no sale nada**: el primer
/// arranque pregunta, y Ajustes lo cambia servicio a servicio.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct PoliticaDeRed {
    pub general: bool,
    pub servicios: BTreeSet<Servicio>,
}

impl PoliticaDeRed {
    pub(crate) fn comprobar(&self, servicio: Servicio) -> ScResult<()> {
        if !self.general {
            return Err(ScError::NetworkDisabled);
        }
        if !self.servicios.contains(&servicio) {
            return Err(ScError::ConsentRequired);
        }
        Ok(())
    }
}

/// Tope de una respuesta que se lee entera en memoria. Las descargas grandes
/// (los paquetes de una actualización) no pasan por aquí sino por una escritura a disco.
const TOPE_EN_MEMORIA: u64 = 16 * 1024 * 1024;

/// La puerta.
pub struct Red {
    politica: RwLock<PoliticaDeRed>,
    agente: ureq::Agent,
    cambio_politica: tokio::sync::watch::Sender<()>,
}

impl Red {
    pub fn new(politica: PoliticaDeRed) -> Self {
        let agente = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent(concat!(
                "NBSelfCare/",
                env!("CARGO_PKG_VERSION"),
                " ( https://github.com/nb-digital-hq/nb-selfcare )"
            ))
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            politica: RwLock::new(politica),
            agente,
            cambio_politica: tokio::sync::watch::channel(()).0,
        }
    }

    pub fn politica(&self) -> PoliticaDeRed {
        self.politica.read().map(|p| p.clone()).unwrap_or_default()
    }

    pub fn cambiar_politica(&self, nueva: PoliticaDeRed) {
        if let Ok(mut actual) = self.politica.write() {
            *actual = nueva;
            self.cambio_politica.send_replace(());
        }
    }

    /// Comprueba si la petición podría salir, sin hacerla.
    ///
    /// # Errors
    ///
    /// [`ScError::NetworkDisabled`], [`ScError::ConsentRequired`] o
    /// [`ScError::PathNotAllowed`] si el anfitrión no es de ese servicio.
    pub fn autorizar(&self, servicio: Servicio, url: &str) -> ScResult<()> {
        self.politica().comprobar(servicio)?;
        let anfitrion = anfitrion_de(url).ok_or(ScError::PathNotAllowed)?;
        if !servicio.admite(anfitrion) {
            tracing::warn!(
                ?servicio,
                "se rechazó una URL fuera de los anfitriones del servicio"
            );
            return Err(ScError::PathNotAllowed);
        }
        tracing::debug!(?servicio, "petición de red autorizada");
        Ok(())
    }

    /// `GET` que devuelve el cuerpo entero.
    ///
    /// # Errors
    ///
    /// Los de [`Red::autorizar`], y [`ScError::NetworkFailure`] si la petición
    /// falla, devuelve un estado que no es 2xx o pasa del tope en memoria.
    pub fn obtener(
        &self,
        servicio: Servicio,
        url: &str,
        cabeceras: &[(&str, &str)],
    ) -> ScResult<Vec<u8>> {
        let (estado, cuerpo) = self.obtener_con_estado(servicio, url, cabeceras)?;
        if !(200..300).contains(&estado) {
            tracing::debug!(?servicio, estado, "respuesta no 2xx");
            return Err(ScError::NetworkFailure);
        }
        Ok(cuerpo)
    }

    /// `GET` que devuelve también el estado, para los servicios en los que un 404
    /// es una respuesta ("no existe") y no un fallo (Open Food Facts).
    ///
    /// # Errors
    ///
    /// Los de [`Red::autorizar`], y [`ScError::NetworkFailure`] si la petición no
    /// se completa o pasa del tope en memoria.
    pub fn obtener_con_estado(
        &self,
        servicio: Servicio,
        url: &str,
        cabeceras: &[(&str, &str)],
    ) -> ScResult<(u16, Vec<u8>)> {
        self.autorizar(servicio, url)?;
        let mut peticion = self.agente.get(url);
        for (nombre, valor) in cabeceras {
            peticion = peticion.header(*nombre, *valor);
        }
        let respuesta = peticion.call().map_err(|e| {
            tracing::debug!(?servicio, error = %e, "la petición no se completó");
            ScError::NetworkFailure
        })?;
        let estado = respuesta.status().as_u16();
        let mut cuerpo = Vec::new();
        respuesta
            .into_body()
            .into_reader()
            .take(TOPE_EN_MEMORIA + 1)
            .read_to_end(&mut cuerpo)
            .map_err(|_| ScError::NetworkFailure)?;
        if cuerpo.len() as u64 > TOPE_EN_MEMORIA {
            return Err(ScError::NetworkFailure);
        }
        Ok((estado, cuerpo))
    }
}

/// Adónde va una descarga. Antes del primer byte se le dice si el servidor
/// continúa lo que ya había (206) o lo manda entero desde el principio (200,
/// porque ignoró el `Range`), para que vacíe lo anterior en ese caso.
pub trait Destino {
    /// # Errors
    ///
    /// Los de entrada y salida del destino.
    fn empezar(&mut self, continuada: bool) -> std::io::Result<()>;
    /// # Errors
    ///
    /// Los de entrada y salida del destino.
    fn escribir(&mut self, bloque: &[u8]) -> std::io::Result<()>;
}

impl Red {
    /// `GET` que escribe el cuerpo en `destino` por bloques, sin pasar por
    /// memoria (un paquete de la aplicación pesa decenas de MB). Con `desde > 0` pide el resto
    /// con `Range` para reanudar. `progreso` recibe lo escrito en esta llamada.
    ///
    /// # Errors
    ///
    /// Los de [`Red::autorizar`], y [`ScError::NetworkFailure`] si la petición no
    /// se completa, el estado no es 200 ni 206, o se corta a mitad.
    pub fn descargar(
        &self,
        servicio: Servicio,
        url: &str,
        desde: u64,
        destino: &mut dyn Destino,
        progreso: &mut dyn FnMut(u64),
    ) -> ScResult<()> {
        self.autorizar(servicio, url)?;
        // Sin el tope global de 30 s del agente: un paquete puede tardar minutos. La
        // respuesta sí tiene que empezar a llegar a tiempo.
        let mut peticion = self
            .agente
            .get(url)
            .config()
            .timeout_global(None)
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .build();
        if desde > 0 {
            peticion = peticion.header("Range", format!("bytes={desde}-"));
        }
        let respuesta = peticion.call().map_err(|e| {
            tracing::debug!(?servicio, error = %e, "la descarga no se completó");
            ScError::NetworkFailure
        })?;
        let estado = respuesta.status().as_u16();
        if estado != 200 && estado != 206 {
            tracing::debug!(?servicio, estado, "la descarga no empezó");
            return Err(ScError::NetworkFailure);
        }
        destino
            .empezar(estado == 206)
            .map_err(|_| ScError::StorageFailure)?;
        let mut lector = respuesta.into_body().into_reader();
        let mut buf = vec![0u8; 256 * 1024];
        let mut bytes = 0u64;
        loop {
            let n = lector.read(&mut buf).map_err(|_| ScError::NetworkFailure)?;
            if n == 0 {
                break;
            }
            destino
                .escribir(&buf[..n])
                .map_err(|_| ScError::StorageFailure)?;
            bytes += n as u64;
            progreso(bytes);
        }
        Ok(())
    }
}

// --- Actualizaciones de la aplicación -----------------------------------

/// El manifiesto de la última publicación, el mismo que fija `tauri.conf.json`.
/// Se repite aquí para comprobarlo en la puerta antes de que el plugin salga.
const MANIFIESTO: &str =
    "https://github.com/nb-digital-hq/nb-selfcare/releases/latest/download/latest.json";

/// El plugin de actualizaciones. Lleva su propio cliente HTTP, así que no se usa
/// fuera de este archivo: cada consulta y cada descarga pasa antes por
/// [`Red::autorizar`], como lo demás. Las redirecciones de GitHub las sigue el
/// plugin; lo que llega se comprueba con la firma de la publicación.
pub fn plugin_de_actualizaciones<R: tauri::Runtime>(
) -> tauri::plugin::TauriPlugin<R, tauri_plugin_updater::Config> {
    tauri_plugin_updater::Builder::new().build()
}

/// Una versión publicada más nueva que la instalada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Actualizacion {
    pub version: String,
    /// Las notas de la publicación, tal como se escribieron en GitHub.
    pub notas: Option<String>,
}

async fn consultar<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    red: &Red,
) -> ScResult<Option<tauri_plugin_updater::Update>> {
    use tauri_plugin_updater::UpdaterExt as _;
    if crate::launcher::gestionada() {
        return Err(ScError::ComponentUnavailable);
    }
    red.autorizar(Servicio::Actualizaciones, MANIFIESTO)?;
    let manifiesto = MANIFIESTO.parse().map_err(|_| ScError::Internal)?;
    let actualizador = app
        .updater_builder()
        .endpoints(vec![manifiesto])
        .and_then(tauri_plugin_updater::UpdaterBuilder::build)
        .map_err(|_| {
            tracing::warn!("el actualizador no está disponible");
            ScError::ComponentUnavailable
        })?;
    // Los errores del plugin no se registran: pueden llevar la ruta del ejecutable.
    let nueva = actualizador.check().await.map_err(|e| {
        tracing::debug!("no se pudo consultar la última publicación");
        match e {
            // Un paquete del sistema (deb o rpm sin permisos, o un formato que el
            // plugin no sabe instalar) se actualiza con su gestor de paquetes.
            tauri_plugin_updater::Error::UnsupportedOs => ScError::ComponentUnavailable,
            _ => ScError::NetworkFailure,
        }
    })?;
    if let Some(n) = &nueva {
        red.autorizar(Servicio::Actualizaciones, n.download_url.as_str())?;
    }
    Ok(nueva)
}

/// Mira si hay una versión más nueva publicada.
///
/// # Errors
///
/// Los de [`Red::autorizar`]; [`ScError::NetworkFailure`] si la consulta falla y
/// [`ScError::ComponentUnavailable`] si esta instalación no se actualiza sola.
pub async fn buscar_actualizacion<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    red: &Red,
) -> ScResult<Option<Actualizacion>> {
    Ok(consultar(app, red).await?.map(|n| Actualizacion {
        version: n.version,
        notas: n.body,
    }))
}

/// Un destino de descarga en memoria, para lo que se verifica entero antes de
/// usarse (la actualización de un paquete del sistema).
#[cfg(target_os = "linux")]
#[derive(Default)]
struct EnMemoria(Vec<u8>);

#[cfg(target_os = "linux")]
impl Destino for EnMemoria {
    fn empezar(&mut self, continuada: bool) -> std::io::Result<()> {
        if !continuada {
            self.0.clear();
        }
        Ok(())
    }

    fn escribir(&mut self, bloque: &[u8]) -> std::io::Result<()> {
        self.0.extend_from_slice(bloque);
        Ok(())
    }
}

/// El texto de minisign que Tauri guarda otra vez en base64 (clave o firma).
#[cfg(target_os = "linux")]
fn texto_de_tauri(b64: &[u8]) -> Option<String> {
    use base64::Engine as _;
    let b = base64::engine::general_purpose::STANDARD
        .decode(std::str::from_utf8(b64).ok()?.trim())
        .ok()?;
    String::from_utf8(b).ok()
}

/// La clave pública de `tauri.conf.json`, como la escribe `tauri signer`.
#[cfg(target_os = "linux")]
fn clave_de_tauri(b64: &str) -> Option<minisign_verify::PublicKey> {
    minisign_verify::PublicKey::decode(&texto_de_tauri(b64.as_bytes())?).ok()
}

/// Comprueba `contenido` con una firma de `tauri signer`. Las dos llegan como las
/// escribe Tauri: el texto de minisign (clave o firma) codificado otra vez en
/// base64. Es la misma comprobación que hace el plugin con lo que instala él.
///
/// # Errors
///
/// [`ScError::IntegrityMismatch`] si la clave o la firma no se entienden o no
/// corresponden al contenido.
#[cfg(target_os = "linux")]
fn verificar_firma(
    clave_publica: &str,
    firma: &[u8],
    contenido: &[u8],
    version: &str,
) -> ScResult<()> {
    let clave = clave_de_tauri(clave_publica).ok_or(ScError::IntegrityMismatch)?;
    let firma = texto_de_tauri(firma)
        .and_then(|t| minisign_verify::Signature::decode(&t).ok())
        .ok_or(ScError::IntegrityMismatch)?;
    clave
        .verify(contenido, &firma, false)
        .map_err(|_| ScError::IntegrityMismatch)?;
    let mut versiones = firma
        .trusted_comment()
        .split('\t')
        .filter_map(|campo| campo.strip_prefix("version:"));
    let firmada = versiones.next().ok_or(ScError::IntegrityMismatch)?;
    if versiones.next().is_some()
        || firmada.trim_start_matches('v') != version.trim_start_matches('v')
    {
        return Err(ScError::IntegrityMismatch);
    }
    Ok(())
}

/// El archivo `.tar.gz` de la misma publicación que `descarga` (la URL del
/// paquete que eligió el plugin): se cambia solo el último tramo, para no repetir
/// aquí el repositorio.
#[cfg(target_os = "linux")]
fn url_hermana(descarga: &str, version: &str) -> Option<String> {
    let (base, _) = descarga.rsplit_once('/')?;
    Some(format!("{base}/NB.SelfCare_{version}_amd64.tar.gz"))
}

/// Si esta instalación es un paquete del sistema que su gestor no puede
/// actualizar: el `.tar.gz` de Linux se hace con el contenido del `.deb` (y el
/// binario dice ser un `.deb`), pero se instala en distribuciones sin `dpkg`,
/// como Arch. Ahí el plugin descargaría el `.deb` para fallar al instalarlo.
#[cfg(target_os = "linux")]
fn paquete_sin_gestor() -> bool {
    use tauri::utils::config::BundleType;
    let gestor = match tauri::utils::platform::bundle_type() {
        Some(BundleType::Deb) => "dpkg",
        Some(BundleType::Rpm) => "rpm",
        _ => return false,
    };
    let en_path = std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|d| d.join(gestor).is_file()));
    !en_path
}

/// Instala el `.tar.gz` firmado de la publicación sobre `/usr`. Se verifica en
/// memoria y se le pasa a `tar` por la entrada estándar: entre comprobar la firma
/// y extraer no hay ningún archivo que otro proceso pueda cambiar. Pide permisos
/// con `sudo -n` (sin contraseña, si el usuario lo configuró) o, si no, con
/// `pkexec`, que pregunta en una ventana del sistema.
#[cfg(target_os = "linux")]
fn instalar_tar(red: &Red, clave_publica: &str, descarga: &str, version: &str) -> ScResult<()> {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    let url = url_hermana(descarga, version).ok_or(ScError::Internal)?;
    let firma = red.obtener(Servicio::Actualizaciones, &format!("{url}.sig"), &[])?;
    let mut paquete = EnMemoria::default();
    red.descargar(
        Servicio::Actualizaciones,
        &url,
        0,
        &mut paquete,
        &mut |_| {},
    )?;
    verificar_firma(clave_publica, &firma, &paquete.0, version)?;

    // Solo lo que cuelga de `usr/`, y sin cambiar los permisos de las carpetas
    // que ya existen.
    let argumentos = ["tar", "--no-overwrite-dir", "-xzf", "-", "-C", "/", "usr"];
    let extraer = |programa: &str, previos: &[&str]| -> bool {
        let Ok(mut hijo) = Command::new(programa)
            .args(previos)
            .args(argumentos)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            return false;
        };
        let escrito = hijo
            .stdin
            .take()
            .is_some_and(|mut entrada| entrada.write_all(&paquete.0).is_ok());
        hijo.wait().is_ok_and(|s| s.success()) && escrito
    };
    if extraer("sudo", &["-n"]) || extraer("pkexec", &[]) {
        Ok(())
    } else {
        tracing::warn!("no se pudo instalar el paquete del sistema");
        Err(ScError::ComponentUnavailable)
    }
}

/// Descarga la versión más nueva, comprueba su firma y la instala. Quien llama
/// reinicia la aplicación. Devuelve `false` si ya no había nada que instalar.
///
/// # Errors
///
/// Los de [`buscar_actualizacion`], e [`ScError::IntegrityMismatch`] si la firma
/// no es la de NB SelfCare.
pub async fn instalar_actualizacion<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    red: &std::sync::Arc<Red>,
) -> ScResult<bool> {
    let Some(nueva) = consultar(app, red).await? else {
        return Ok(false);
    };

    #[cfg(target_os = "linux")]
    if paquete_sin_gestor() {
        let clave = app
            .config()
            .plugins
            .0
            .get("updater")
            .and_then(|u| u.get("pubkey"))
            .and_then(serde_json::Value::as_str)
            .ok_or(ScError::Internal)?
            .to_owned();
        let red = std::sync::Arc::clone(red);
        let descarga = nueva.download_url.to_string();
        let version = nueva.version.clone();
        tauri::async_runtime::spawn_blocking(move || {
            instalar_tar(&red, &clave, &descarga, &version)
        })
        .await
        .map_err(|_| ScError::Internal)??;
        return Ok(true);
    }

    nueva
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| {
            tracing::warn!("no se pudo instalar la actualización");
            match e {
                tauri_plugin_updater::Error::Minisign(_)
                | tauri_plugin_updater::Error::SignatureUtf8(_)
                | tauri_plugin_updater::Error::SignedVersionMismatch { .. }
                | tauri_plugin_updater::Error::MissingSignedVersion => ScError::IntegrityMismatch,
                // Sin permisos para escribir donde está instalada.
                tauri_plugin_updater::Error::Io(_) => ScError::ComponentUnavailable,
                _ => ScError::NetworkFailure,
            }
        })?;
    Ok(true)
}

/// El anfitrión de una URL `https://`. Las demás no se admiten: nada sale en
/// claro.
fn anfitrion_de(url: &str) -> Option<&str> {
    let resto = url.strip_prefix("https://")?;
    let fin = resto.find(['/', '?', '#']).unwrap_or(resto.len());
    let autoridad = &resto[..fin];
    if autoridad.contains('@') {
        return None;
    }
    let anfitrion = autoridad.split(':').next()?;
    (!anfitrion.is_empty()).then_some(anfitrion)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn la_clave_de_tauri_conf_se_entiende_y_rechaza_una_firma_ajena() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let clave = conf["plugins"]["updater"]["pubkey"].as_str().unwrap();
        assert!(clave_de_tauri(clave).is_some());
        assert_eq!(
            verificar_firma(clave, b"no es una firma", b"contenido", "1.0.0"),
            Err(ScError::IntegrityMismatch)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn el_tar_se_busca_en_la_misma_publicacion_que_el_paquete() {
        assert_eq!(
            url_hermana(
                "https://github.com/x/y/releases/download/v2.1.0/NB.SelfCare_2.1.0_amd64.deb",
                "2.1.0"
            )
            .as_deref(),
            Some("https://github.com/x/y/releases/download/v2.1.0/NB.SelfCare_2.1.0_amd64.tar.gz")
        );
    }

    fn con(servicios: &[Servicio]) -> Red {
        Red::new(PoliticaDeRed {
            general: true,
            servicios: servicios.iter().copied().collect(),
        })
    }

    #[test]
    fn de_fabrica_no_sale_nada() {
        let red = Red::new(PoliticaDeRed::default());
        assert_eq!(
            red.autorizar(Servicio::OpenFoodFacts, URL_OFF),
            Err(ScError::NetworkDisabled)
        );
    }

    const URL_OFF: &str = "https://world.openfoodfacts.org/api/v2/product/737628064502.json";

    #[test]
    fn con_la_red_encendida_cada_servicio_pide_su_consentimiento() {
        let red = con(&[Servicio::Actualizaciones]);
        assert_eq!(
            red.autorizar(Servicio::OpenFoodFacts, URL_OFF),
            Err(ScError::ConsentRequired)
        );
        assert_eq!(
            con(&[Servicio::OpenFoodFacts]).autorizar(Servicio::OpenFoodFacts, URL_OFF),
            Ok(())
        );
    }

    #[test]
    fn el_interruptor_general_manda_sobre_los_consentimientos() {
        let red = con(&Servicio::TODOS);
        let mut politica = red.politica();
        politica.general = false;
        red.cambiar_politica(politica);
        assert_eq!(
            red.autorizar(Servicio::OpenFoodFacts, URL_OFF),
            Err(ScError::NetworkDisabled)
        );
    }

    /// Una URL que viene en una respuesta de un servicio no puede usar el
    /// permiso de ese servicio para ir a otro sitio.
    #[test]
    fn un_servicio_solo_alcanza_sus_anfitriones() {
        let red = con(&Servicio::TODOS);
        for url in [
            "https://evil.example/x",
            "https://world.openfoodfacts.org.evil.example/x",
            "https://world.openfoodfacts.org@evil.example/x",
            "http://world.openfoodfacts.org/api/v2/product/1.json",
            "world.openfoodfacts.org/api",
            "https://images.openfoodfacts.org/x.jpg",
        ] {
            assert_eq!(
                red.autorizar(Servicio::OpenFoodFacts, url),
                Err(ScError::PathNotAllowed),
                "{url}"
            );
        }
        assert_eq!(
            red.autorizar(
                Servicio::OpenFoodFacts,
                "https://world.openfoodfacts.org:443/"
            ),
            Ok(())
        );
    }

    #[test]
    fn los_servicios_http_tienen_anfitriones_y_el_enlace_local_no_usa_http_generico() {
        for servicio in Servicio::TODOS {
            assert_eq!(
                servicio.anfitriones().is_empty(),
                servicio == Servicio::Sincronizacion
            );
        }
    }
    #[test]
    fn las_actualizaciones_piden_su_propio_consentimiento_y_solo_van_a_github() {
        let red = con(&[Servicio::Actualizaciones]);
        assert_eq!(red.autorizar(Servicio::Actualizaciones, MANIFIESTO), Ok(()));
        assert_eq!(
            red.autorizar(
                Servicio::Actualizaciones,
                "https://objects.githubusercontent.com/github-production-release-asset/x"
            ),
            Ok(())
        );
        assert_eq!(
            red.autorizar(
                Servicio::Actualizaciones,
                "https://world.openfoodfacts.org/x"
            ),
            Err(ScError::PathNotAllowed)
        );
        assert_eq!(
            con(&[Servicio::OpenFoodFacts]).autorizar(Servicio::Actualizaciones, MANIFIESTO),
            Err(ScError::ConsentRequired)
        );
    }

    /// La puerta comprueba la URL que el plugin va a usar de verdad.
    #[test]
    fn el_manifiesto_es_el_de_la_configuracion() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            conf["plugins"]["updater"]["endpoints"],
            serde_json::json!([MANIFIESTO])
        );
    }
}

#[cfg(all(test, target_os = "linux"))]
mod version_de_paquetes_firmados {
    use super::*;
    // Vectores públicos del plugin Tauri, bajo Apache-2.0/MIT: contenido `test`.
    const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEE0OUVGNDc0NkZBNkU2RDcKUldUWDVxWnZkUFNlcERHV2cyYno4bjZBclFuQk13Z1FLMWdlVm9zb3JjYkx6Y3VuRCtLSitjWWU=";
    const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVUWDVxWnZkUFNlcFBGMnpzanZQL0wvVFF0QkxSbldSdmFxKzRvbUsxVUV1Z3RRcC9VU3JHaUkxdjc2MnNEeTNpZDNhb3I3YlJDOUVUeWVvdDZsTUhIcldvWjVoalVNMVFjPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkxNjQ5MDUxCWZpbGU6cGF5bG9hZAl2ZXJzaW9uOjIuMC4wCkREcTJDTE9FbDVjUW1DMmVKUkhsblpQVlZpT1pzdTFDanBQS1FvZWk0QS9YQmZrRHFHTzBCK1FNcmNySGdFUTVxVjNtbmU3NXRkRldoSnBxVVhoOUFBPT0=";
    const UNVERSIONED_PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXkgRTc2MjBGMTg0MkI0RTgxRgpSV1FmNkxSQ0dBOWk1M21sWWVjTzRJelQ1MVRHUHB2V3VjTlNDaDFDQk0wUVRhTG43M1k3R0ZPMw==";
    const UNVERSIONED_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIG1pbmlzaWduIHNlY3JldCBrZXkKUlVRZjZMUkNHQTlpNTU5cjNnN1YxcU55SkRBcEdpcDhNZnFjYWRJZ1Q5Q3VoVjNFTWhIb04xbUdUa1VpZEYvejdTcmxRZ1hkeThvZmpiN2JOSkp5bERPb2NyQ284S0x6WndvPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNjMzNzAwODM1CWZpbGU6dGVzdAlwcmVoYXNoZWQKd0xNRGp5OUZMQXV4WjNxNE5sRXZrZ3R5aHJyMGd0VHU2S0M0S0JKZElUYmJPZUFpMXpCSVlvMHY0aVRndDhqSnBJaWRSSm5wOTRBQlFrSkFnQW9vQlE9PQ==";

    #[test]
    fn una_firma_valida_no_permite_anunciar_otra_version() {
        assert_eq!(
            verificar_firma(PUBLIC_KEY, SIGNATURE.as_bytes(), b"test", "2.0.0"),
            Ok(())
        );
        assert_eq!(
            verificar_firma(PUBLIC_KEY, SIGNATURE.as_bytes(), b"test", "v2.0.0"),
            Ok(())
        );
        assert_eq!(
            verificar_firma(PUBLIC_KEY, SIGNATURE.as_bytes(), b"test", "9.9.9"),
            Err(ScError::IntegrityMismatch)
        );
        assert_eq!(
            verificar_firma(PUBLIC_KEY, SIGNATURE.as_bytes(), b"otro", "2.0.0"),
            Err(ScError::IntegrityMismatch)
        );
    }

    #[test]
    fn una_firma_sin_version_no_autoriza_una_actualizacion() {
        assert_eq!(
            verificar_firma(
                UNVERSIONED_PUBLIC_KEY,
                UNVERSIONED_SIGNATURE.as_bytes(),
                b"test",
                "1.0.0"
            ),
            Err(ScError::IntegrityMismatch)
        );
    }

    #[test]
    #[ignore = "requiere NB_SELFCARE_BUNDLE_DIR con paquetes locales firmados"]
    fn los_paquetes_locales_tienen_la_firma_y_version_de_la_aplicacion() {
        fn recorrer(carpeta: &std::path::Path, archivos: &mut Vec<std::path::PathBuf>) {
            for entrada in std::fs::read_dir(carpeta).unwrap() {
                let ruta = entrada.unwrap().path();
                if ruta.is_dir() {
                    recorrer(&ruta, archivos);
                } else if ruta.extension().is_some_and(|e| e == "sig") {
                    archivos.push(ruta);
                }
            }
        }
        let carpeta = std::path::PathBuf::from(
            std::env::var_os("NB_SELFCARE_BUNDLE_DIR").expect("falta carpeta de paquetes"),
        );
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let clave = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
        let mut firmas = Vec::new();
        recorrer(&carpeta, &mut firmas);
        assert!(firmas.len() >= 4, "faltan firmas de paquetes");
        for firma in firmas {
            let paquete = firma.with_extension("");
            assert_eq!(
                verificar_firma(
                    clave,
                    &std::fs::read(&firma).unwrap(),
                    &std::fs::read(&paquete).unwrap(),
                    env!("CARGO_PKG_VERSION")
                ),
                Ok(())
            );
        }
    }
}

/// Listener TLS del enlace. No acepta tráfico HTTP plano ni pares de redes públicas.
/// Las tareas de handshake quedan acotadas y se cancelan al soltar el listener.
struct ListenerLocalTls {
    cupos: std::sync::Arc<tokio::sync::Semaphore>,
    tcp: tokio::net::TcpListener,
    tls: tokio_rustls::TlsAcceptor,
    pendientes: tokio::task::JoinSet<Option<(ConexionLocalTls, std::net::SocketAddr)>>,
}
impl axum::serve::Listener for ListenerLocalTls {
    type Io = ConexionLocalTls;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            tokio::select! {
                resultado = self.tcp.accept(), if self.pendientes.len() < 16 => {
                    match resultado {
                        Ok((socket, remoto)) if crate::sincronizacion::invitacion::direccion_local(remoto.ip()) => {
                            let Ok(cupo) = std::sync::Arc::clone(&self.cupos).try_acquire_owned() else { continue; };
                            let tls = self.tls.clone();
                            self.pendientes.spawn(async move {
                                match tokio::time::timeout(Duration::from_secs(5), tls.accept(socket)).await {
                                    Ok(Ok(conexion)) => Some((ConexionLocalTls {
                                        conexion, _cupo: cupo,
                                        plazo: Box::pin(tokio::time::sleep(Duration::from_secs(120))),
                                    }, remoto)),
                                    _ => None,
                                }
                            });
                        }
                        Ok(_) => {}
                        Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
                    }
                }
                resultado = self.pendientes.join_next(), if !self.pendientes.is_empty() => {
                    if let Some(Ok(Some(conexion))) = resultado { return conexion; }
                }
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.tcp.local_addr()
    }
}

/// Un cupo dura toda la conexión, no solo la negociación. La cota temporal evita
/// que conexiones o cabeceras incompletas retengan todos los cupos indefinidamente.
struct ConexionLocalTls {
    conexion: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    _cupo: tokio::sync::OwnedSemaphorePermit,
    plazo: std::pin::Pin<Box<tokio::time::Sleep>>,
}
impl ConexionLocalTls {
    fn comprobar_plazo(&mut self, contexto: &mut std::task::Context<'_>) -> std::io::Result<()> {
        use std::future::Future as _;
        if self.plazo.as_mut().poll(contexto).is_ready() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        Ok(())
    }
}
impl tokio::io::AsyncRead for ConexionLocalTls {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        contexto: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        this.comprobar_plazo(contexto)?;
        std::pin::Pin::new(&mut this.conexion).poll_read(contexto, buf)
    }
}
impl tokio::io::AsyncWrite for ConexionLocalTls {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        contexto: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        this.comprobar_plazo(contexto)?;
        std::pin::Pin::new(&mut this.conexion).poll_write(contexto, buf)
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        contexto: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        this.comprobar_plazo(contexto)?;
        std::pin::Pin::new(&mut this.conexion).poll_flush(contexto)
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        contexto: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.get_mut().conexion).poll_shutdown(contexto)
    }
}

/// Infraestructura del enlace: el router propietario debe autenticar al par y validar el dominio.
/// Abrir esta puerta no constituye una sincronización aplicada.
pub struct PuertaLocal {
    direccion: std::net::SocketAddr,
    detener: Option<tokio::sync::oneshot::Sender<()>>,
    tarea: Option<tokio::task::JoinHandle<ScResult<()>>>,
}
impl PuertaLocal {
    /// Abre una interfaz concreta, nunca una dirección comodín; no habilita permisos por su cuenta.
    /// El anuncio mDNS es opcional y no contiene identidad personal, credenciales ni huellas.
    /// # Errors
    /// Errores canónicos de consentimiento, dirección o red.
    pub async fn abrir(
        red: std::sync::Arc<Red>,
        direccion: std::net::SocketAddr,
        identidad: &crate::sincronizacion::identidad::IdentidadTls,
        router: axum::Router,
        anunciar: bool,
    ) -> ScResult<Self> {
        red.politica().comprobar(Servicio::Sincronizacion)?;
        if !crate::sincronizacion::invitacion::direccion_local(direccion.ip()) {
            return Err(ScError::PathNotAllowed);
        }
        // Suscribir antes de enlazar impide perder una revocación concurrente.
        let mut cambios = red.cambio_politica.subscribe();
        let tcp = tokio::net::TcpListener::bind(direccion)
            .await
            .map_err(|_| ScError::NetworkFailure)?;
        let direccion = tcp.local_addr().map_err(|_| ScError::NetworkFailure)?;
        red.politica().comprobar(Servicio::Sincronizacion)?;
        let listener = ListenerLocalTls {
            cupos: std::sync::Arc::new(tokio::sync::Semaphore::new(32)),
            tcp,
            tls: tokio_rustls::TlsAcceptor::from(identidad.configuracion()),
            pendientes: tokio::task::JoinSet::new(),
        };
        let router = router.layer(axum::middleware::from_fn_with_state(
            std::sync::Arc::clone(&red),
            vigilar_enlace,
        ));
        let (detener, detenida) = tokio::sync::oneshot::channel();
        let anuncio = AnuncioLocal(if anunciar && !direccion.ip().is_loopback() {
            anunciar_enlace(direccion)
        } else {
            None
        });
        let tarea = tokio::spawn(async move {
            axum::serve(listener, router).with_graceful_shutdown(async move {
                tokio::select! {
                    _ = detenida => {}
                    _ = async {
                        loop {
                            if red.politica().comprobar(Servicio::Sincronizacion).is_err() { break; }
                            if cambios.changed().await.is_err() { break; }
                        }
                    } => {}
                }
                drop(anuncio);
            }).await.map_err(|_| ScError::NetworkFailure)
        });
        Ok(Self {
            direccion,
            detener: Some(detener),
            tarea: Some(tarea),
        })
    }

    pub fn direccion(&self) -> std::net::SocketAddr {
        self.direccion
    }

    /// Espera el cierre tras revocar el permiso, con cota para conexiones persistentes.
    /// # Errors
    /// [`ScError::NetworkFailure`] si la tarea falló.
    pub async fn esperar_cierre(&mut self) -> ScResult<()> {
        let Some(mut tarea) = self.tarea.take() else {
            return Ok(());
        };
        match tokio::time::timeout(Duration::from_secs(12), &mut tarea).await {
            Ok(Ok(resultado)) => resultado,
            Ok(Err(_)) => Err(ScError::NetworkFailure),
            Err(_) => {
                tarea.abort();
                Err(ScError::NetworkFailure)
            }
        }
    }
}
impl Drop for PuertaLocal {
    fn drop(&mut self) {
        if let Some(detener) = self.detener.take() {
            let _ = detener.send(());
        }
    }
}
struct AnuncioLocal(Option<(mdns_sd::ServiceDaemon, String)>);
impl Drop for AnuncioLocal {
    fn drop(&mut self) {
        if let Some((daemon, nombre)) = self.0.take() {
            let _ = daemon.unregister(&nombre);
            let _ = daemon.shutdown();
        }
    }
}

async fn vigilar_enlace(
    axum::extract::State(red): axum::extract::State<std::sync::Arc<Red>>,
    peticion: axum::extract::Request,
    siguiente: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse as _;
    if red.politica().comprobar(Servicio::Sincronizacion).is_err() {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    let resultado = tokio::time::timeout(Duration::from_secs(10), siguiente.run(peticion)).await;
    if red.politica().comprobar(Servicio::Sincronizacion).is_err() {
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }
    match resultado {
        Ok(respuesta) => respuesta,
        Err(_) => axum::http::StatusCode::REQUEST_TIMEOUT.into_response(),
    }
}
fn anunciar_enlace(direccion: std::net::SocketAddr) -> Option<(mdns_sd::ServiceDaemon, String)> {
    let info = mdns_sd::ServiceInfo::new(
        "_nb-selfcare._tcp.local.",
        &format!("SelfCare-{}", direccion.port()),
        &format!("nbselfcare-{}.local.", direccion.port()),
        direccion.ip(),
        direccion.port(),
        &[("version", "1")][..],
    )
    .ok()?;
    let daemon = mdns_sd::ServiceDaemon::new().ok()?;
    let nombre = info.get_fullname().to_owned();
    if daemon.register(info).is_err() {
        let _ = daemon.shutdown();
        return None;
    }
    Some((daemon, nombre))
}

#[cfg(test)]
mod pruebas_enlace_local {
    use super::*;
    use base64::Engine as _;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio_rustls::rustls::{
        self,
        pki_types::{CertificateDer, ServerName},
    };

    fn autorizada() -> Arc<Red> {
        Arc::new(Red::new(PoliticaDeRed {
            general: true,
            servicios: [Servicio::Sincronizacion].into_iter().collect(),
        }))
    }
    fn sonda() -> axum::Router {
        axum::Router::new().route("/sonda", axum::routing::get(|| async { "protocolo-1" }))
    }
    async fn consultar(
        puerta: &PuertaLocal,
        identidad: &crate::sincronizacion::identidad::IdentidadTls,
    ) -> String {
        let mut raices = rustls::RootCertStore::empty();
        let der = base64::engine::general_purpose::STANDARD
            .decode(identidad.certificado())
            .unwrap();
        raices.add(CertificateDer::from(der)).unwrap();
        let configuracion = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(raices)
        .with_no_client_auth();
        let tcp = tokio::net::TcpStream::connect(puerta.direccion())
            .await
            .unwrap();
        let mut tls = tokio_rustls::TlsConnector::from(Arc::new(configuracion))
            .connect(ServerName::try_from("nbselfcare.local").unwrap(), tcp)
            .await
            .unwrap();
        tls.write_all(
            b"GET /sonda HTTP/1.1\r\nHost: nbselfcare.local\r\nConnection: close\r\n\r\n",
        )
        .await
        .unwrap();
        let mut salida = Vec::new();
        tls.read_to_end(&mut salida).await.unwrap();
        String::from_utf8(salida).unwrap()
    }
    #[tokio::test]
    async fn no_se_abre_un_socket_sin_ambos_permisos_o_en_una_direccion_publica() {
        let identidad = crate::sincronizacion::identidad::IdentidadTls::efimera_de_prueba();
        for (politica, esperado) in [
            (PoliticaDeRed::default(), ScError::NetworkDisabled),
            (
                PoliticaDeRed {
                    general: true,
                    servicios: BTreeSet::new(),
                },
                ScError::ConsentRequired,
            ),
        ] {
            let resultado = PuertaLocal::abrir(
                Arc::new(Red::new(politica)),
                "127.0.0.1:0".parse().unwrap(),
                &identidad,
                sonda(),
                false,
            )
            .await;
            assert!(matches!(resultado, Err(error) if error == esperado));
        }
        assert!(matches!(
            PuertaLocal::abrir(
                autorizada(),
                "0.0.0.0:0".parse().unwrap(),
                &identidad,
                sonda(),
                false
            )
            .await,
            Err(ScError::PathNotAllowed)
        ));
    }
    #[tokio::test]
    async fn el_listener_solo_entrega_http_dentro_de_tls_y_cierra_al_revocar() {
        let identidad = crate::sincronizacion::identidad::IdentidadTls::efimera_de_prueba();
        let red = autorizada();
        let mut puerta = PuertaLocal::abrir(
            Arc::clone(&red),
            "127.0.0.1:0".parse().unwrap(),
            &identidad,
            sonda(),
            false,
        )
        .await
        .unwrap();
        // Un cliente que no inicia TLS no bloquea a otro que sí lo hace.
        let _lento = tokio::net::TcpStream::connect(puerta.direccion())
            .await
            .unwrap();
        let respuesta =
            tokio::time::timeout(Duration::from_secs(2), consultar(&puerta, &identidad))
                .await
                .unwrap();
        assert!(respuesta.starts_with("HTTP/1.1 200"));
        assert!(respuesta.ends_with("protocolo-1"));
        let mut plano = tokio::net::TcpStream::connect(puerta.direccion())
            .await
            .unwrap();
        plano
            .write_all(b"GET /sonda HTTP/1.1\r\n\r\n")
            .await
            .unwrap();
        let mut bytes = [0; 128];
        let lectura = tokio::time::timeout(Duration::from_secs(2), plano.read(&mut bytes))
            .await
            .unwrap();
        assert!(!matches!(lectura, Ok(n) if n > 0 && bytes[..n].starts_with(b"HTTP/")));
        red.cambiar_politica(PoliticaDeRed::default());
        puerta.esperar_cierre().await.unwrap();
        puerta.esperar_cierre().await.unwrap();
        assert!(tokio::net::TcpStream::connect(puerta.direccion())
            .await
            .is_err());
    }
}

/// Superficie de emparejamiento, sin rutas de lectura ni escritura sanitaria.
/// Solo se sirve mediante PuertaLocal, detrás del consentimiento y TLS.
pub fn router_de_pares(
    gestor: std::sync::Arc<crate::sincronizacion::pares::GestorDePares>,
) -> axum::Router {
    axum::Router::new()
        .route("/v1/emparejar", axum::routing::post(aceptar_par_local))
        .route("/v1/estado", axum::routing::get(estado_del_enlace))
        .layer(axum::extract::DefaultBodyLimit::max(4096))
        .with_state(gestor)
}
fn error_del_enlace(error: ScError) -> axum::response::Response {
    use axum::response::IntoResponse as _;
    let estado = match error {
        ScError::InvalidInput => axum::http::StatusCode::BAD_REQUEST,
        ScError::ConsentRequired => axum::http::StatusCode::UNAUTHORIZED,
        ScError::StorageFailure | ScError::ComponentUnavailable => {
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        }
        _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        estado,
        axum::Json(serde_json::json!({"code": error.code()})),
    )
        .into_response()
}
async fn aceptar_par_local(
    axum::extract::State(gestor): axum::extract::State<
        std::sync::Arc<crate::sincronizacion::pares::GestorDePares>,
    >,
    solicitud: Result<
        axum::Json<crate::sincronizacion::pares::SolicitudDeAlta>,
        axum::extract::rejection::JsonRejection,
    >,
) -> axum::response::Response {
    use axum::response::IntoResponse as _;
    let Ok(axum::Json(solicitud)) = solicitud else {
        return error_del_enlace(ScError::InvalidInput);
    };
    match tokio::task::spawn_blocking(move || gestor.aceptar(solicitud, std::time::Instant::now()))
        .await
    {
        Ok(Ok(respuesta)) => axum::Json(respuesta).into_response(),
        Ok(Err(error)) => error_del_enlace(error),
        Err(_) => error_del_enlace(ScError::Internal),
    }
}
fn autenticar_par(
    gestor: &crate::sincronizacion::pares::GestorDePares,
    cabeceras: &axum::http::HeaderMap,
) -> ScResult<crate::sincronizacion::contrato::Identificador> {
    if cabeceras.get_all("authorization").iter().count() != 1
        || cabeceras.get_all("x-selfcare-nodo").iter().count() != 1
    {
        return Err(ScError::ConsentRequired);
    }
    let nodo = cabeceras
        .get("x-selfcare-nodo")
        .and_then(|h| h.to_str().ok())
        .ok_or(ScError::ConsentRequired)?;
    if nodo.len() != 32 {
        return Err(ScError::ConsentRequired);
    }
    let nodo = crate::sincronizacion::contrato::Identificador::try_from(nodo.to_owned())
        .map_err(|_| ScError::ConsentRequired)?;
    let token = cabeceras
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(ScError::ConsentRequired)?;
    gestor.autenticar(&nodo, token)?;
    Ok(nodo)
}
async fn estado_del_enlace(
    axum::extract::State(gestor): axum::extract::State<
        std::sync::Arc<crate::sincronizacion::pares::GestorDePares>,
    >,
    cabeceras: axum::http::HeaderMap,
) -> axum::response::Response {
    use axum::response::IntoResponse as _;
    match autenticar_par(&gestor, &cabeceras) {
        Ok(_) => {
            axum::Json(serde_json::json!({"version": crate::sincronizacion::contrato::VERSION}))
                .into_response()
        }
        Err(error) => error_del_enlace(error),
    }
}

/// Lote máximo: 200 operaciones de hasta 64 KiB más el sobre.
const MAX_CUERPO_SINCRONIZACION: usize = 14 * 1024 * 1024;

#[derive(Clone)]
struct EstadoSincronizacion {
    gestor: std::sync::Arc<crate::sincronizacion::pares::GestorDePares>,
    db: crate::db::BaseDeDatos,
    avisar: crate::sincronizacion::enlace::Avisar,
}

/// Superficie completa del producto: emparejamiento y sincronización de registros.
/// Cada petición sanitaria exige un par autorizado; el dominio valida cada campo.
pub fn router_del_enlace(
    gestor: std::sync::Arc<crate::sincronizacion::pares::GestorDePares>,
    db: crate::db::BaseDeDatos,
    avisar: crate::sincronizacion::enlace::Avisar,
) -> axum::Router {
    let sincronizar = axum::Router::new()
        .route("/v1/sincronizar", axum::routing::post(sincronizar_con_par))
        .layer(axum::extract::DefaultBodyLimit::max(
            MAX_CUERPO_SINCRONIZACION,
        ))
        .with_state(EstadoSincronizacion {
            gestor: std::sync::Arc::clone(&gestor),
            db,
            avisar,
        });
    router_de_pares(gestor).merge(sincronizar)
}

async fn sincronizar_con_par(
    axum::extract::State(estado): axum::extract::State<EstadoSincronizacion>,
    cabeceras: axum::http::HeaderMap,
    peticion: Result<
        axum::Json<crate::sincronizacion::servicio::Peticion>,
        axum::extract::rejection::JsonRejection,
    >,
) -> axum::response::Response {
    use axum::response::IntoResponse as _;
    let nodo = match autenticar_par(&estado.gestor, &cabeceras) {
        Ok(n) => n,
        Err(e) => return error_del_enlace(e),
    };
    let Ok(axum::Json(peticion)) = peticion else {
        return error_del_enlace(ScError::InvalidInput);
    };
    let db = estado.db.clone();
    let con_lote = peticion.lote.is_some();
    match tokio::task::spawn_blocking(move || {
        let mut conn = db.conexion()?;
        crate::sincronizacion::servicio::atender(&mut conn, &nodo, peticion)
    })
    .await
    {
        Ok(Ok(respuesta)) => {
            if con_lote
                && respuesta
                    .resultados
                    .iter()
                    .any(|r| r.estado != crate::sincronizacion::servicio::Estado::Rechazado)
            {
                (estado.avisar)();
            }
            axum::Json(respuesta).into_response()
        }
        Ok(Err(error)) => error_del_enlace(error),
        Err(_) => error_del_enlace(ScError::Internal),
    }
}

#[cfg(test)]
mod proceso_dart {
    use super::*;
    use crate::sincronizacion::{
        contrato::Identificador,
        identidad::IdentidadTls,
        invitacion::SobreInvitacion,
        pares::{AlmacenDePares, GestorDePares},
    };
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _};

    #[derive(Default)]
    struct Memoria(Mutex<Option<String>>);
    impl AlmacenDePares for Memoria {
        fn leer(&self) -> ScResult<Option<String>> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn guardar(&self, valor: &str) -> ScResult<()> {
            *self.0.lock().unwrap() = Some(valor.into());
            Ok(())
        }
    }
    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    async fn esperar_linea(
        lineas: &mut tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>,
        esperada: &str,
    ) {
        tokio::time::timeout(Duration::from_secs(120), async {
            for _ in 0..20 {
                let linea = lineas
                    .next_line()
                    .await
                    .unwrap()
                    .expect("el cliente terminó sin confirmar el enlace");
                if linea == esperada {
                    return;
                }
            }
            panic!("el cliente no confirmó el estado esperado");
        })
        .await
        .expect("se agotó la espera del cliente Dart");
    }

    #[tokio::test]
    #[ignore = "requiere NB_SELFCARE_DART y NB_SELFCARE_MOBILE_DIR absolutos, con SDK y dependencias preparados"]
    async fn cliente_dart_real_empareja_fija_certificado_y_rechaza_una_revocacion() {
        probar_cliente("127.0.0.1:0").await;
        probar_cliente("[::1]:0").await;
    }
    async fn probar_cliente(direccion: &str) {
        let dart = std::path::PathBuf::from(
            std::env::var_os("NB_SELFCARE_DART").expect("SDK Dart explícito"),
        );
        let movil = std::path::PathBuf::from(
            std::env::var_os("NB_SELFCARE_MOBILE_DIR").expect("clon móvil explícito"),
        );
        assert!(dart.is_absolute() && dart.is_file());
        assert!(movil.is_absolute() && movil.join("tool/qa/enlace_local.dart").is_file());
        let identidad = IdentidadTls::efimera_de_prueba();
        let gestor = Arc::new(GestorDePares::cargar(Box::<Memoria>::default()).unwrap());
        let red = Arc::new(Red::new(PoliticaDeRed {
            general: true,
            servicios: [Servicio::Sincronizacion].into_iter().collect(),
        }));
        let mut puerta = PuertaLocal::abrir(
            Arc::clone(&red),
            direccion.parse().unwrap(),
            &identidad,
            router_de_pares(Arc::clone(&gestor)),
            false,
        )
        .await
        .unwrap();
        let token = gestor.invitar(std::time::Instant::now()).unwrap();
        let invitacion = SobreInvitacion::crear(
            puerta.direccion().ip(),
            puerta.direccion().port(),
            identidad.huella(),
            &token,
        )
        .unwrap();
        let nodo = Identificador::generar().unwrap();
        let fixture =
            Fixture(std::env::temp_dir().join(format!("nb-selfcare-qa-enlace-{}", nodo.exponer())));
        std::fs::create_dir(&fixture.0).unwrap();
        let archivo = fixture.0.join("invitacion.json");
        let mut opciones = std::fs::OpenOptions::new();
        opciones.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            opciones.mode(0o600);
        }
        {
            use std::io::Write as _;
            let mut f = opciones.open(&archivo).unwrap();
            f.write_all(
                &serde_json::to_vec(&serde_json::json!({"invitacion": invitacion, "nodo": nodo, "ahora": chrono::Utc::now().to_rfc3339()}))
                    .unwrap(),
            )
            .unwrap();
        }
        let mut cliente = tokio::process::Command::new(dart)
            .args(["run", "tool/qa/enlace_local.dart"])
            .arg(&archivo)
            .current_dir(movil)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut lineas = tokio::io::BufReader::new(cliente.stdout.take().unwrap()).lines();
        esperar_linea(&mut lineas, "ENLACE_LISTO").await;
        gestor.revocar(&nodo).unwrap();
        cliente
            .stdin
            .take()
            .unwrap()
            .write_all(b"REVOCADO\n")
            .await
            .unwrap();
        esperar_linea(&mut lineas, "REVOCACION_CORRECTA").await;
        assert!(
            tokio::time::timeout(Duration::from_secs(10), cliente.wait())
                .await
                .unwrap()
                .unwrap()
                .success()
        );
        red.cambiar_politica(PoliticaDeRed::default());
        puerta.esperar_cierre().await.unwrap();
    }
}

/// Direcciones candidatas para el QR, sin consultar DNS ni contactar un anfitrión.
/// No publica nombres, MAC ni identificadores permanentes de adaptadores.
/// # Errors
/// [`ScError::NetworkFailure`] si el sistema no puede enumerar interfaces.
pub fn interfaces_locales() -> ScResult<Vec<std::net::IpAddr>> {
    let interfaces = if_addrs::get_if_addrs().map_err(|_| ScError::NetworkFailure)?;
    Ok(filtrar_interfaces(
        interfaces
            .into_iter()
            .filter(|i| !interfaz_virtual(&i.name))
            .map(|i| (i.ip(), i.is_oper_up())),
    ))
}
/// Puentes de contenedores, máquinas virtuales y túneles: un teléfono de la red local no
/// llega a ellos. El nombre solo se usa aquí y nunca sale del proceso.
fn interfaz_virtual(nombre: &str) -> bool {
    const PREFIJOS: [&str; 14] = [
        "docker",
        "podman",
        "cni",
        "veth",
        "br-",
        "virbr",
        "vmnet",
        "vboxnet",
        "tun",
        "tap",
        "wg",
        "tailscale",
        "zt",
        "utun",
    ];
    let nombre = nombre.to_ascii_lowercase();
    PREFIJOS.iter().any(|p| nombre.starts_with(p))
}
#[cfg(test)]
mod interfaces_virtuales {
    #[test]
    fn los_puentes_y_tuneles_no_se_ofrecen_y_la_red_domestica_va_primero() {
        for n in [
            "docker0",
            "podman1",
            "virbr0",
            "veth12",
            "wg0",
            "tailscale0",
            "br-3f2a",
        ] {
            assert!(super::interfaz_virtual(n), "{n}");
        }
        for n in ["enp4s0", "wlan0", "eth0", "en0", "Wi-Fi"] {
            assert!(!super::interfaz_virtual(n), "{n}");
        }
        let ips = super::filtrar_interfaces([
            ("10.88.0.1".parse().unwrap(), true),
            ("192.168.1.20".parse().unwrap(), true),
        ]);
        assert_eq!(ips[0], "192.168.1.20".parse::<std::net::IpAddr>().unwrap());
    }
}
fn filtrar_interfaces(
    interfaces: impl IntoIterator<Item = (std::net::IpAddr, bool)>,
) -> Vec<std::net::IpAddr> {
    let mut direcciones: Vec<_> = interfaces
        .into_iter()
        .filter_map(|(ip, activa)| {
            // Una dirección IPv6 de enlace necesita una zona propia de cada dispositivo;
            // no se ofrece como QR sin esa información. IPv4 y ULA no tienen esa ambigüedad.
            let requiere_zona =
                matches!(ip, std::net::IpAddr::V6(ip) if ip.is_unicast_link_local());
            (activa
                && !ip.is_loopback()
                && !requiere_zona
                && crate::sincronizacion::invitacion::direccion_local(ip))
            .then_some(ip)
        })
        .collect();
    // Las redes domésticas habituales primero: es la dirección que se ofrece en el QR.
    let preferencia = |ip: &std::net::IpAddr| match ip {
        std::net::IpAddr::V4(v4) if v4.octets()[..2] == [192, 168] => 0,
        std::net::IpAddr::V4(v4) if v4.octets()[0] == 10 => 1,
        std::net::IpAddr::V4(_) => 2,
        std::net::IpAddr::V6(_) => 3,
    };
    direcciones.sort_by_key(|ip| (preferencia(ip), *ip));
    direcciones.dedup();
    direcciones.truncate(32);
    direcciones
}
#[cfg(test)]
mod interfaces_de_red {
    use super::*;
    #[test]
    fn el_qr_solo_ofrece_interfaces_activas_privadas_sin_zona_ambigua() {
        let entradas = [
            ("192.168.1.4", true),
            ("192.168.1.4", true),
            ("10.0.0.2", false),
            ("127.0.0.1", true),
            ("8.8.8.8", true),
            ("::1", true),
            ("fe80::1", true),
            ("fd12::4", true),
            ("0.0.0.0", true),
        ]
        .map(|(ip, activa)| (ip.parse().unwrap(), activa));
        let resultado = filtrar_interfaces(entradas);
        assert_eq!(
            resultado
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ["192.168.1.4", "fd12::4"]
        );
    }
    #[test]
    fn la_lista_no_excede_treinta_y_dos_direcciones() {
        let entradas = (1..=100).map(|n| {
            (
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, n)),
                true,
            )
        });
        assert_eq!(filtrar_interfaces(entradas).len(), 32);
    }
}

#[cfg(test)]
mod sincronizacion_con_movil {
    use super::*;
    use crate::sincronizacion::{
        contrato::Identificador,
        identidad::IdentidadTls,
        pares::{AlmacenDePares, GestorDePares, SolicitudDeAlta},
    };
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Memoria(Mutex<Option<String>>);
    impl AlmacenDePares for Memoria {
        fn leer(&self) -> ScResult<Option<String>> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn guardar(&self, valor: &str) -> ScResult<()> {
            *self.0.lock().unwrap() = Some(valor.into());
            Ok(())
        }
    }

    /// Recorrido real: dos motores Dart de SelfCare Mobile con SQLite sincronizan con este
    /// servidor por TLS fijado. El primero envía, el segundo recibe lo del primero.
    #[tokio::test]
    #[ignore = "requiere NB_SELFCARE_FLUTTER y NB_SELFCARE_MOBILE_DIR absolutos, con SDK y dependencias preparados"]
    async fn dos_telefonos_reales_sincronizan_a_traves_del_escritorio() {
        let flutter = std::path::PathBuf::from(
            std::env::var_os("NB_SELFCARE_FLUTTER").expect("SDK Flutter explícito"),
        );
        let movil = std::path::PathBuf::from(
            std::env::var_os("NB_SELFCARE_MOBILE_DIR").expect("clon móvil explícito"),
        );
        assert!(
            flutter.is_absolute() && movil.join("tool/qa/sincronizacion_e2e_test.dart").is_file()
        );
        let db = crate::db::BaseDeDatos::en_memoria().unwrap();
        crate::comida::sembrar(&db).unwrap();
        db.conexion()
            .unwrap()
            .execute(
                "INSERT INTO body_measurements (measured_at, date_key, source, weight_kg, trend_weight_kg, bmi)
                 VALUES ('2026-11-19T08:00:00.000', '2026-11-19', 'manual', 81.5, 81.5, 25.7)",
                [],
            )
            .unwrap();
        let identidad = IdentidadTls::efimera_de_prueba();
        let gestor = Arc::new(GestorDePares::cargar(Box::<Memoria>::default()).unwrap());
        let mut telefonos = Vec::new();
        for _ in 0..2 {
            let credencial = gestor.invitar(std::time::Instant::now()).unwrap();
            let nodo = Identificador::generar().unwrap();
            let alta = gestor
                .aceptar(
                    SolicitudDeAlta {
                        version: 1,
                        nodo: nodo.clone(),
                        credencial: credencial.exponer().into(),
                    },
                    std::time::Instant::now(),
                )
                .unwrap();
            telefonos.push(
                serde_json::json!({"nodo": nodo, "credencial": alta.credencial, "pc": alta.nodo}),
            );
        }
        let red = Arc::new(Red::new(PoliticaDeRed {
            general: true,
            servicios: [Servicio::Sincronizacion].into_iter().collect(),
        }));
        let mut puerta = PuertaLocal::abrir(
            Arc::clone(&red),
            "127.0.0.1:0".parse().unwrap(),
            &identidad,
            router_del_enlace(Arc::clone(&gestor), db.clone(), Arc::new(|| {})),
            false,
        )
        .await
        .unwrap();
        let carpeta = std::env::temp_dir().join(format!(
            "nb-selfcare-qa-sync-{}",
            Identificador::generar().unwrap().exponer()
        ));
        std::fs::create_dir(&carpeta).unwrap();
        let archivo = carpeta.join("enlace.json");
        std::fs::write(
            &archivo,
            serde_json::to_vec(&serde_json::json!({
                "host": puerta.direccion().ip().to_string(),
                "puerto": puerta.direccion().port(),
                "huella": identidad.huella(),
                "telefonos": telefonos,
            }))
            .unwrap(),
        )
        .unwrap();
        let estado = tokio::process::Command::new(flutter)
            .args(["test", "tool/qa/sincronizacion_e2e_test.dart"])
            .env("NB_SELFCARE_QA_ENLACE", &archivo)
            .current_dir(movil)
            .kill_on_drop(true)
            .status();
        let estado = tokio::time::timeout(Duration::from_secs(600), estado)
            .await
            .unwrap()
            .unwrap();
        let _ = std::fs::remove_dir_all(&carpeta);
        assert!(estado.success(), "el recorrido Dart falló");
        let conn = db.conexion().unwrap();
        let agua: i64 = conn
            .query_row(
                "SELECT coalesce(sum(volume_ml), 0) FROM hydration_logs",
                [],
                |f| f.get(0),
            )
            .unwrap();
        assert_eq!(
            agua, 600,
            "el escritorio conserva la edición del primer teléfono"
        );
        let comidas: i64 = conn
            .query_row("SELECT count(*) FROM food_logs", [], |f| f.get(0))
            .unwrap();
        assert_eq!(comidas, 1);
        drop(conn);
        red.cambiar_politica(PoliticaDeRed::default());
        puerta.esperar_cierre().await.unwrap();
    }
}
