fn main() {
    // En Windows, los diálogos nativos enlazan con Common Controls 6. Tauri incrusta su manifiesto solo en el
    // ejecutable de la aplicación, y los binarios de prueba sin él terminan con STATUS_ENTRYPOINT_NOT_FOUND. El
    // manifiesto se pasa al enlazador para todos los objetivos en lugar de mediante el recurso de Tauri.
    let mut atributos = tauri_build::Attributes::new();
    let objetivo_windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let enlazador_msvc = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if objetivo_windows && enlazador_msvc {
        atributos = atributos
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let manifiesto =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifiesto.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
            manifiesto.display()
        );
    }
    tauri_build::try_build(atributos).expect("no se pudo preparar la compilación de Tauri");
}
