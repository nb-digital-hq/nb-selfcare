//! Rutas por plataforma.
//!
//! Ninguna ruta la decide el usuario salvo dónde guarda una copia de seguridad.
//! Todo vive en los directorios estándar del sistema operativo, y los
//! datos y la caché regenerable van separados: vaciar la caché nunca pierde nada.
//!
//! `NB_SELFCARE_TEST_DIR` desvía **todas** las rutas a un directorio temporal, para
//! que las pruebas no toquen la instalación de quien las ejecuta.

use std::path::{Path, PathBuf};

use crate::error::{ScError, ScResult};

/// Variable que aísla el entorno durante las pruebas.
pub const TEST_DIR_ENV: &str = "NB_SELFCARE_TEST_DIR";

/// Directorios del runtime.
#[derive(Debug, Clone)]
pub struct Rutas {
    /// Base de datos y copias automáticas. Se respalda.
    pub datos: PathBuf,
    /// Caché regenerable. No se respalda.
    pub cache: PathBuf,
    /// Registros de diagnóstico.
    pub registros: PathBuf,
}

impl Rutas {
    /// Resuelve los directorios y los crea si no existen.
    ///
    /// # Errors
    ///
    /// [`ScError::StorageFailure`] si el sistema no expone un directorio de datos
    /// o si no se pueden crear.
    pub fn resolver() -> ScResult<Self> {
        if let Some(base) = std::env::var_os(TEST_DIR_ENV) {
            return Self::bajo(&PathBuf::from(base));
        }
        let datos = plataforma::datos().ok_or(ScError::StorageFailure)?;
        let cache = plataforma::cache().ok_or(ScError::StorageFailure)?;
        Self {
            registros: datos.join("registros"),
            datos,
            cache,
        }
        .crear()
    }

    /// Todas las rutas bajo una sola carpeta. Lo usan las pruebas.
    ///
    /// # Errors
    ///
    /// [`ScError::StorageFailure`] si no se pueden crear.
    pub fn bajo(base: &Path) -> ScResult<Self> {
        Self {
            datos: base.join("datos"),
            cache: base.join("cache"),
            registros: base.join("registros"),
        }
        .crear()
    }

    fn crear(self) -> ScResult<Self> {
        for dir in [&self.datos, &self.cache] {
            std::fs::create_dir_all(dir).map_err(|_| ScError::StorageFailure)?;
        }
        // El diagnóstico no es un requisito para conservar los datos de salud.
        // Si el destino falla, telemetry::init utiliza un escritor vacío.
        let _ = std::fs::create_dir_all(&self.registros);
        Ok(self)
    }

    /// Archivo de la base de datos.
    pub fn base_de_datos(&self) -> PathBuf {
        self.datos.join("nb-selfcare.sqlite")
    }

    /// Copias de seguridad automáticas.
    pub fn copias(&self) -> PathBuf {
        self.datos.join("copias")
    }
}

// Resolución sin dependencias externas: son pocas llamadas por plataforma, como
// en el resto de la suite.
#[cfg(target_os = "linux")]
mod plataforma {
    use std::path::PathBuf;

    fn xdg(var: &str, respaldo: &str) -> Option<PathBuf> {
        match std::env::var_os(var) {
            Some(v) if !v.is_empty() => Some(PathBuf::from(v)),
            _ => Some(PathBuf::from(std::env::var_os("HOME")?).join(respaldo)),
        }
    }

    pub fn datos() -> Option<PathBuf> {
        Some(xdg("XDG_DATA_HOME", ".local/share")?.join("nb-selfcare"))
    }

    pub fn cache() -> Option<PathBuf> {
        Some(xdg("XDG_CACHE_HOME", ".cache")?.join("nb-selfcare"))
    }
}

#[cfg(target_os = "windows")]
mod plataforma {
    use std::path::PathBuf;

    fn var(nombre: &str) -> Option<PathBuf> {
        std::env::var_os(nombre).map(PathBuf::from)
    }

    pub fn datos() -> Option<PathBuf> {
        Some(var("APPDATA")?.join("NB SelfCare"))
    }

    pub fn cache() -> Option<PathBuf> {
        Some(var("LOCALAPPDATA")?.join("NB SelfCare").join("Cache"))
    }
}

#[cfg(target_os = "macos")]
mod plataforma {
    use std::path::PathBuf;

    fn home() -> Option<PathBuf> {
        std::env::var_os("HOME").map(PathBuf::from)
    }

    pub fn datos() -> Option<PathBuf> {
        Some(home()?.join("Library/Application Support/NB SelfCare"))
    }

    pub fn cache() -> Option<PathBuf> {
        Some(home()?.join("Library/Caches/NB SelfCare"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todas_las_rutas_quedan_bajo_la_base_de_pruebas() {
        let base = std::env::temp_dir().join(format!("sc-rutas-{}", std::process::id()));
        let rutas = Rutas::bajo(&base).expect("se crean");
        for ruta in [
            rutas.base_de_datos(),
            rutas.copias(),
            rutas.cache.clone(),
            rutas.registros.clone(),
        ] {
            assert!(
                ruta.starts_with(&base),
                "{} escapa de la base",
                ruta.display()
            );
        }
        let _ = std::fs::remove_dir_all(base);
    }
}

#[cfg(test)]
mod diagnostico_opcional {
    use super::*;
    #[test]
    fn un_destino_de_registros_invalido_no_bloquea_el_almacenamiento() {
        let base = std::env::temp_dir().join(format!("sc-rutas-registro-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("registros"), b"prueba").unwrap();
        let rutas = Rutas::bajo(&base).unwrap();
        assert!(rutas.datos.is_dir());
        assert!(rutas.cache.is_dir());
        assert!(rutas.registros.is_file());
        std::fs::remove_dir_all(base).unwrap();
    }
}
