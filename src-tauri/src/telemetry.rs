//! Observabilidad local con redacción obligatoria.
//!
//! En un producto local-first no tenemos acceso a la máquina del usuario: los
//! registros son la única herramienta de diagnóstico en producción. Y a la vez,
//! los registros de salud son datos íntimos.
//!
//! De esa tensión sale la regla de este módulo: **se registra el comportamiento
//! del sistema, nunca el contenido del usuario**: ni rutas, ni títulos, ni
//! artistas, ni claves (Constitución, regla 5). Portado de NB Books.

use std::path::Path;

// Solo lo usa el escritor de consola de depuración, que en release se compila
// fuera: sin esta condición, el perfil de release avisa de un import sin usar. No
// lo veía `pnpm verify` porque **la verificación nunca compila en release**, y lo
// destapó la puerta de empaquetado la primera vez que se cruzó.
#[cfg(debug_assertions)]
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Guarda que mantiene viva la escritura asíncrona a archivo.
///
/// Debe conservarse durante toda la vida del proceso: al soltarlo se vacía el
/// búfer y se deja de escribir.
pub struct TelemetryGuard(#[allow(dead_code)] tracing_appender::non_blocking::WorkerGuard);

/// Inicializa el registro estructurado.
///
/// Escribe a un archivo rotado por día en `logs_dir` y, en compilaciones de
/// depuración, también a la salida estándar.
///
/// El nivel se controla con la variable `NB_SELFCARE_LOG` siguiendo la sintaxis de
/// filtros de `tracing`. Por defecto es `info`.
///
/// # Errors
///
/// No falla: si el archivo de registro no se puede abrir, la aplicación debe
/// seguir arrancando. Perder el diagnóstico es malo; impedir que el usuario
/// use sus registros por ello es peor.
#[must_use]
pub fn init(logs_dir: &Path) -> TelemetryGuard {
    let appender: Box<dyn std::io::Write + Send> =
        tracing_appender::rolling::RollingFileAppender::builder()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("nb-selfcare.log")
            .build(logs_dir)
            .map(|archivo| Box::new(archivo) as Box<dyn std::io::Write + Send>)
            .unwrap_or_else(|_| Box::new(std::io::sink()));
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let filter =
        EnvFilter::try_from_env("NB_SELFCARE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true)
        .with_file(false)
        .with_line_number(false);

    let registry = tracing_subscriber::registry().with(filter).with(file_layer);

    #[cfg(debug_assertions)]
    let registry = registry.with(
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout.with_max_level(tracing::Level::DEBUG)),
    );

    // `try_init` y no `init`: un segundo arranque en el mismo proceso -el de una
    // prueba que comprueba qué ve la sesión siguiente- no puede tumbar el
    // runtime por un registro que ya está puesto. Sigue escribiendo el primero.
    let _ = registry.try_init();

    TelemetryGuard(guard)
}

/// Redacta una ruta del sistema de archivos para que pueda registrarse.
///
/// Devuelve únicamente la extensión y la longitud, que es lo que sirve para
/// diagnosticar, sin revelar el nombre del archivo, la estructura de carpetas ni
/// el nombre de usuario del sistema operativo.
///
/// ```
/// use nb_selfcare_lib::telemetry::redact_path;
///
/// let redacted = redact_path(std::path::Path::new("/ejemplo/copia.sqlite"));
///
/// assert!(redacted.contains("sqlite"));    // la extensión sirve para diagnosticar
/// assert!(!redacted.contains("ejemplo"));   // el nombre de usuario, no
/// assert!(!redacted.contains("copia"));
/// ```
#[must_use]
pub fn redact_path(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    // Una extensión arbitraria también puede ser contenido privado.
    let ext = match ext.as_str() {
        "sqlite" => "sqlite",
        "sqlite3" => "sqlite3",
        "db" => "db",
        "log" => "log",
        "json" => "json",
        _ => "otra",
    };
    let len = path.as_os_str().len();
    format!("<ruta ext={ext} len={len}>")
}

/// Redacta un texto libre que podría contener contenido del usuario.
///
/// Se usa cuando hay que registrar que *algo* tenía un valor sin registrar cuál:
/// títulos, artistas, consultas de búsqueda o letras.
#[must_use]
pub fn redact_text(text: &str) -> String {
    format!("<texto len={}>", text.chars().count())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Si esta prueba falla, los registros pueden estar filtrando la biblioteca
    /// del usuario y su nombre de cuenta del sistema operativo.
    #[test]
    fn la_redaccion_de_rutas_no_revela_nada_del_usuario() {
        let redacted = redact_path(Path::new("/ejemplo/datos/copia.sqlite"));

        assert!(!redacted.contains("ejemplo"));
        assert!(!redacted.contains("copia"));
        assert!(!redacted.contains("datos"));
        assert!(!redacted.contains('/'));
        assert!(
            redacted.contains("sqlite"),
            "la extensión sí es útil: {redacted}"
        );
    }

    #[test]
    fn la_redaccion_de_rutas_funciona_con_rutas_de_windows() {
        let redacted = redact_path(Path::new(r"C:\Datos\Privados\copia.db"));

        assert!(!redacted.contains("Privados"));
        assert!(!redacted.contains("copia"));
        assert!(redacted.contains("db"));
    }

    #[test]
    fn la_redaccion_de_rutas_sin_extension_no_entra_en_panico() {
        assert_eq!(
            redact_path(Path::new("/tmp/sin_extension")),
            "<ruta ext=otra len=18>"
        );
    }

    #[test]
    fn la_redaccion_de_texto_conserva_solo_la_longitud() {
        let redacted = redact_text("Canción de cuna");

        assert!(!redacted.contains("cuna"));
        assert_eq!(redacted, "<texto len=15>");
    }
}

#[cfg(test)]
mod privacidad_adicional {
    use super::*;

    #[test]
    fn una_extension_arbitraria_no_se_copia_al_registro() {
        let redactada = redact_path(Path::new("/ejemplo/copia.detalle_medico_privado"));
        assert!(!redactada.contains("detalle_medico_privado"));
        assert!(redactada.contains("ext=otra"));
    }

    #[test]
    fn un_destino_de_registro_no_escribible_no_impide_iniciar() {
        let ruta = std::env::temp_dir().join(format!("sc-registro-archivo-{}", std::process::id()));
        std::fs::write(&ruta, b"prueba").unwrap();
        let guarda = init(&ruta);
        drop(guarda);
        std::fs::remove_file(ruta).unwrap();
    }
}
