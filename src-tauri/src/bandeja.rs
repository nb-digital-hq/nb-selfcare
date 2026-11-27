//! El icono de la bandeja del sistema: volver a la ventana, empezar una pausa
//! activa, posponer los avisos una hora y salir.
//!
//! Con "Seguir en la bandeja" activado (de fábrica), cerrar la ventana la
//! esconde y la aplicación sigue aquí para que los avisos lleguen; salir de
//! verdad es "Salir" en este menú (ADR-007).

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::idioma::t;

const ID: &str = "nb-selfcare";

/// Trae la ventana principal al frente.
pub fn mostrar(app: &AppHandle) {
    if let Some(v) = app.get_webview_window("main") {
        let _ = v.unminimize();
        let _ = v.show();
        let _ = v.set_focus();
    }
}

/// El menú en el idioma en uso.
fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(
                app,
                "mostrar",
                t(
                    "Abrir NB SelfCare",
                    "Open NB SelfCare",
                    "Abrir o NB SelfCare",
                ),
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "pausa",
                t(
                    "Pausa activa ahora",
                    "Active break now",
                    "Pausa ativa agora",
                ),
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "posponer",
                t(
                    "Posponer avisos una hora",
                    "Snooze reminders for an hour",
                    "Adiar avisos por uma hora",
                ),
                true,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "salir", t("Salir", "Quit", "Sair"), true, None::<&str>)?,
        ],
    )
}

/// Vuelve a escribir el menú tras un cambio de idioma. Sin bandeja, nada.
pub fn actualizar(app: &AppHandle) {
    let Some(icono) = app.tray_by_id(ID) else {
        return;
    };
    if let Err(_error) = menu(app).and_then(|m| icono.set_menu(Some(m))) {
        tracing::info!("no se pudo traducir el menú de la bandeja");
    }
}

/// Pone el icono. Si el escritorio no tiene bandeja, no pasa nada.
pub fn poner(app: &AppHandle, avisos: crate::avisos::Servicio) {
    let resultado = (|| -> tauri::Result<()> {
        let menu = menu(app)?;
        let mut constructor = TrayIconBuilder::with_id(ID)
            .tooltip("NB SelfCare")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_tray_icon_event(|icono, evento| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = evento
                {
                    mostrar(icono.app_handle());
                }
            })
            .on_menu_event(move |app, evento| match evento.id().as_ref() {
                "mostrar" => mostrar(app),
                "pausa" => {
                    mostrar(app);
                    let _ = app.emit("avisos:empezar", "pausa");
                }
                "posponer" => avisos.posponer(60),
                "salir" => app.exit(0),
                _ => {}
            });
        if let Some(icono) = app.default_window_icon() {
            constructor = constructor.icon(icono.clone());
        }
        constructor.build(app)?;
        Ok(())
    })();
    if let Err(_error) = resultado {
        tracing::info!("sin icono en la bandeja");
    }
}
