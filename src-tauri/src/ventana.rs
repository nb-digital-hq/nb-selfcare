//! Red de seguridad para que un fallo del webview no deje la aplicación abierta
//! sin ninguna ventana, y el arranque en segundo plano (al iniciar sesión), en
//! el que la ventana no se muestra hasta que se pide desde la bandeja.

use std::time::Duration;

use tauri::{AppHandle, Manager};

const ESPERA_RED_SEGURIDAD: Duration = Duration::from_secs(4);

static EN_SEGUNDO_PLANO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// La aplicación se abrió al iniciar sesión: la ventana espera en la bandeja.
pub fn no_mostrar_al_arrancar() {
    EN_SEGUNDO_PLANO.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Si la ventana debe quedarse oculta al arrancar.
#[must_use]
pub fn en_segundo_plano() -> bool {
    EN_SEGUNDO_PLANO.load(std::sync::atomic::Ordering::Relaxed)
}

/// La red solo actúa si la ventana todavía no está visible, también cuando ni
/// siquiera se puede consultar su estado.
fn hay_que_mostrar(visible: Option<bool>) -> bool {
    visible != Some(true)
}

/// Programa la comprobación sin bloquear el `setup` ni el hilo de la ventana.
pub(crate) fn programar_red_de_seguridad(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(ESPERA_RED_SEGURIDAD).await;
        if en_segundo_plano() {
            return;
        }
        let Some(ventana) = app.get_webview_window("main") else {
            tracing::error!("la red de seguridad no encontró la ventana principal");
            return;
        };
        if !hay_que_mostrar(ventana.is_visible().ok()) {
            return;
        }
        if ventana.show().is_err() {
            tracing::error!("la red de seguridad no pudo mostrar la ventana principal");
        } else {
            tracing::warn!(
                "la ventana principal seguía oculta y se mostró por la red de seguridad"
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_se_muestra_lo_que_no_esta_visible() {
        assert!(hay_que_mostrar(Some(false)));
        assert!(hay_que_mostrar(None));
        assert!(!hay_que_mostrar(Some(true)));
    }
}
