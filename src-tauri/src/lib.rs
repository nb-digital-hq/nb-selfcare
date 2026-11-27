//! Runtime de NB SelfCare.
//!
//! Contratos: docs/adr/README.md. Cada dominio vive en su módulo, y los comandos
//! del puente están en [`comandos`]: ninguno es síncrono, porque en Tauri un
//! comando sin `async` corre en el hilo de la ventana y la congela mientras dura
//! (lo vigila `scripts/check-comandos.mjs`).

pub mod avisos;
pub mod ayuno;
pub mod bandeja;
pub mod comandos;
pub mod comida;
pub mod copia;
pub mod cuerpo;
pub mod cuidado;
pub mod db;
pub mod diario;
pub mod error;
pub mod evolucion;
pub mod fechas;
pub mod formulas;
pub mod hoy;
pub mod idioma;
pub mod launcher;
pub mod net;
pub mod nibi;
pub mod paths;
pub mod perfil;
pub mod preferencias;
pub mod recetas;
pub mod sincronizacion;
pub mod telemetry;
pub mod texto;
pub mod ventana;

use std::sync::{Arc, Mutex};

use crate::db::BaseDeDatos;
use crate::net::Red;
use crate::paths::Rutas;

/// Estado compartido del runtime.
pub struct Estado {
    pub rutas: Rutas,
    pub red: Arc<Red>,
    pub db: BaseDeDatos,
    pub avisos: avisos::Servicio,
    /// La copia elegida para restaurar: la ruta se queda en el proceso y no
    /// cruza el puente (ADR-004).
    pub copia_elegida: Mutex<Option<std::path::PathBuf>>,
    /// Conexión local con SelfCare Mobile (ADR-014).
    pub enlace: Arc<sincronizacion::enlace::Enlace>,
}

/// Muestra un aviso del sistema. Si el escritorio no tiene servicio de
/// notificaciones, queda en el registro sin contenido.
fn notificar(app: &tauri::AppHandle, titulo: &str, cuerpo: &str) {
    use tauri_plugin_notification::NotificationExt;
    if app
        .notification()
        .builder()
        .title(titulo)
        .body(cuerpo)
        .show()
        .is_err()
    {
        tracing::info!("el sistema no mostró el aviso");
    }
}

/// Arranca la aplicación.
///
/// # Panics
///
/// Si el sistema no ofrece un directorio de datos donde escribir, o si la base de
/// datos no se puede abrir: sin ellos no hay nada que mostrar, y seguir sería
/// peor que detenerse con un mensaje.
pub fn run() {
    let rutas = Rutas::resolver().expect("el sistema no ofrece un directorio de datos utilizable");
    let _registro = telemetry::init(&rutas.registros);
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "arranca NB SelfCare");

    let db = BaseDeDatos::abrir(&rutas.base_de_datos()).expect("no se pudo abrir la base de datos");
    // Antes de crear la bandeja o sembrar nada: lo que el runtime nombra sale ya
    // en el idioma de la interfaz.
    if let Err(error) = preferencias::cargar_idioma(&db) {
        tracing::warn!(
            codigo = error.code(),
            "no se pudo leer el idioma; queda el de fábrica"
        );
    }
    if let Err(error) = comida::sembrar(&db) {
        tracing::error!(
            codigo = error.code(),
            "no se pudo sembrar el catálogo de alimentos"
        );
    }
    let red = Arc::new(Red::new(preferencias::politica(&db)));
    let servicio_avisos = avisos::Servicio::default();
    let estado = Estado {
        rutas: rutas.clone(),
        red,
        db: db.clone(),
        avisos: servicio_avisos.clone(),
        copia_elegida: Mutex::new(None),
        enlace: Arc::new(sincronizacion::enlace::Enlace::default()),
    };

    let aplicacion = tauri::Builder::default()
        // Primero: abrir la aplicación otra vez trae al frente la que ya está.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            bandeja::mostrar(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--en-segundo-plano"]),
        ))
        .plugin(net::plugin_de_actualizaciones())
        .manage(estado)
        .setup(move |app| {
            crate::ventana::programar_red_de_seguridad(app.handle().clone());
            bandeja::poner(app.handle(), servicio_avisos.clone());
            // Abierta al iniciar sesión: se queda en la bandeja sin ventana.
            if std::env::args().any(|a| a == "--en-segundo-plano") {
                crate::ventana::no_mostrar_al_arrancar();
            }
            let app_avisos = app.handle().clone();
            servicio_avisos.arrancar(db.clone(), move |aviso, titulo, cuerpo| {
                use tauri::Emitter;
                notificar(&app_avisos, &titulo, &cuerpo);
                let _ = app_avisos.emit("avisos:aviso", aviso);
            });
            // El enlace con el teléfono se reabre solo si la persona ya lo permitió.
            {
                use tauri::Manager;
                let app_enlace = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let e = app_enlace.state::<Estado>();
                    let _ = comandos::abrir_enlace(&app_enlace, &e).await;
                });
            }
            let (d, r) = (db.clone(), rutas.clone());
            let _ = std::thread::Builder::new()
                .name("copia-automatica".into())
                .spawn(move || {
                    // Al abrir, la aplicación ya lee: la copia espera a que eso pase.
                    std::thread::sleep(std::time::Duration::from_secs(90));
                    loop {
                        if let Err(error) = copia::automatica_si_toca(&d, &r) {
                            tracing::warn!(
                                codigo = error.code(),
                                "no se pudo hacer la copia automática"
                            );
                        }
                        std::thread::sleep(std::time::Duration::from_secs(6 * 3600));
                    }
                });
            Ok(())
        })
        // Con "Seguir en la bandeja", cerrar la ventana la esconde: los avisos
        // siguen llegando. Si no, cerrar es salir.
        .on_window_event(move |ventana, evento| {
            use tauri::Manager;
            if ventana.label() != "main" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = evento {
                let seguir = ventana
                    .app_handle()
                    .try_state::<Estado>()
                    .and_then(|e| preferencias::avisos(&e.db).ok())
                    .is_some_and(|a| a.seguir_en_bandeja);
                if seguir && ventana.app_handle().tray_by_id("nb-selfcare").is_some() {
                    api.prevent_close();
                    let _ = ventana.hide();
                } else {
                    ventana.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            comandos::salud,
            comandos::registrar_ventana_no_visible,
            comandos::arranque_en_segundo_plano,
            comandos::apariencia,
            comandos::guardar_preferencia,
            comandos::avisos_ajustes,
            comandos::guardar_avisos,
            comandos::avisos_posponer,
            comandos::politica_red,
            comandos::cambiar_politica_red,
            comandos::inicio_con_sistema,
            comandos::actualizacion_buscar,
            comandos::actualizacion_instalar,
            comandos::hoy,
            comandos::perfil_bienvenida,
            comandos::perfil_cambiar,
            comandos::comida_dia,
            comandos::comida_buscar,
            comandos::comida_alimento,
            comandos::comida_registrar,
            comandos::comida_proponer,
            comandos::comida_registrar_frase,
            comandos::comida_repetir_ayer,
            comandos::comida_borrar,
            comandos::comida_favorito,
            comandos::comida_crear,
            comandos::comida_beber,
            comandos::comida_borrar_vaso,
            comandos::comida_combos,
            comandos::comida_guardar_combo,
            comandos::comida_registrar_combo,
            comandos::comida_borrar_combo,
            comandos::comida_codigo,
            comandos::cuerpo_resumen,
            comandos::cuerpo_pesar,
            comandos::cuerpo_parece_de_otra_persona,
            comandos::cuerpo_cinta,
            comandos::cuerpo_borrar,
            comandos::cuerpo_entrenar,
            comandos::cuerpo_borrar_entreno,
            comandos::cuerpo_calibrar,
            comandos::ayuno_estado,
            comandos::ayuno_comprobar,
            comandos::ayuno_empezar,
            comandos::ayuno_terminar,
            comandos::ayuno_ajustar,
            comandos::ayuno_borrar,
            comandos::cuidado_resumen,
            comandos::tratamiento_guardar,
            comandos::tratamiento_estado,
            comandos::tratamiento_reponer,
            comandos::toma_marcar,
            comandos::habito_crear,
            comandos::habito_quitar,
            comandos::habito_marcar,
            comandos::sesion_completar,
            comandos::vital_medir,
            comandos::sintoma_registrar,
            comandos::chequeo_hecho,
            comandos::chequeo_ajustar,
            comandos::sustancia_consumir,
            comandos::sustancia_borrar,
            comandos::libre_crear,
            comandos::libre_ganas,
            comandos::libre_reiniciar,
            comandos::libre_celebrar,
            comandos::libre_quitar,
            comandos::bienestar_guardar,
            comandos::diario_entradas,
            comandos::diario_escribir,
            comandos::diario_borrar,
            comandos::diario_sugerir,
            comandos::noche_anotar,
            comandos::cafeina_tomar,
            comandos::ciclo,
            comandos::ciclo_empezar,
            comandos::nibi_estado,
            comandos::nibi_comprar,
            comandos::nibi_poner,
            comandos::nibi_renombrar,
            comandos::copia_exportar,
            comandos::copia_elegir,
            comandos::copia_restaurar,
            comandos::recetas,
            comandos::receta_crear,
            comandos::receta_registrar,
            comandos::receta_borrar,
            comandos::evolucion,
            comandos::historial,
            comandos::historial_borrar,
            comandos::enlace_estado,
            comandos::enlace_activar,
            comandos::enlace_invitar,
            comandos::enlace_revocar,
            comandos::sincronizacion_conflictos,
            comandos::sincronizacion_resolver
        ])
        .build(tauri::generate_context!())
        .expect("no se pudo arrancar la ventana de NB SelfCare");

    aplicacion.run(|_, _| {});
}
