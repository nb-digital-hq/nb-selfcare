//! La instalación administrada conserva una única autoridad sobre sus binarios.
use std::path::Path;

/// Una marca presente, incluso dañada, impide reemplazar archivos del gestor.
fn marca_presente(ejecutable: &Path) -> bool {
    ejecutable
        .parent()
        .is_some_and(|bin| std::fs::symlink_metadata(bin.join(".nb-launcher-managed")).is_ok())
}

#[must_use]
pub fn gestionada() -> bool {
    std::env::current_exe()
        .and_then(|e| e.canonicalize())
        .is_ok_and(|e| marca_presente(&e))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn la_marca_junto_al_binario_bloquea_incluso_si_esta_danada() {
        let carpeta =
            std::env::temp_dir().join(format!("nb-selfcare-launcher-{}", std::process::id()));
        std::fs::create_dir(&carpeta).unwrap();
        let ejecutable = carpeta.join("nb-selfcare");
        assert!(!marca_presente(&ejecutable));
        std::fs::write(
            carpeta.join(".nb-launcher-managed"),
            b"nb-launcher-managed-v1\n",
        )
        .unwrap();
        assert!(marca_presente(&ejecutable));
        std::fs::write(carpeta.join(".nb-launcher-managed"), b"").unwrap();
        assert!(marca_presente(&ejecutable));
        std::fs::remove_dir_all(carpeta).unwrap();
    }
}
