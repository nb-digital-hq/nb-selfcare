//! Los comandos del puente.
//!
//! Todos son `async` y devuelven [`ScResult`]: ni corren en el hilo de la ventana
//! ni devuelven cadenas como error. El trabajo con la base va a un hilo de
//! bloqueo con [`en_db`].

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::db::BaseDeDatos;
use crate::error::{ScError, ScResult};
use crate::Estado;

/// Ejecuta un trabajo con la base fuera del hilo de la ventana.
async fn en_db<T: Send + 'static>(
    estado: &tauri::State<'_, Estado>,
    trabajo: impl FnOnce(&BaseDeDatos) -> ScResult<T> + Send + 'static,
) -> ScResult<T> {
    let db = estado.db.clone();
    tauri::async_runtime::spawn_blocking(move || trabajo(&db))
        .await
        .map_err(|_| ScError::Internal)?
}

/// Lo que responde el runtime cuando se le pregunta si está vivo.
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct InformeDeSalud {
    pub version: String,
    pub gestionada_por_launcher: bool,
}

#[tauri::command]
pub async fn salud() -> ScResult<InformeDeSalud> {
    Ok(InformeDeSalud {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        gestionada_por_launcher: crate::launcher::gestionada(),
    })
}

/// Si la aplicación se abrió al iniciar sesión: entonces la interfaz no muestra
/// la ventana al terminar de pintarse.
#[tauri::command]
pub async fn arranque_en_segundo_plano() -> ScResult<bool> {
    Ok(crate::ventana::en_segundo_plano())
}

/// La interfaz no pudo mostrar la ventana: queda en el registro.
#[tauri::command]
pub async fn registrar_ventana_no_visible() -> ScResult<()> {
    tracing::error!("la interfaz no pudo mostrar la ventana principal");
    Ok(())
}

// --- Preferencias ------------------------------------------------------------

#[tauri::command]
pub async fn apariencia(
    estado: tauri::State<'_, Estado>,
) -> ScResult<crate::preferencias::Apariencia> {
    en_db(&estado, crate::preferencias::apariencia).await
}

#[tauri::command]
pub async fn guardar_preferencia(
    app: AppHandle,
    estado: tauri::State<'_, Estado>,
    clave: String,
    valor: serde_json::Value,
) -> ScResult<()> {
    let es_idioma = clave == "idioma";
    en_db(&estado, move |db| {
        crate::preferencias::guardar(db, &clave, &valor)
    })
    .await?;
    if es_idioma {
        crate::bandeja::actualizar(&app);
    }
    Ok(())
}

#[tauri::command]
pub async fn avisos_ajustes(
    estado: tauri::State<'_, Estado>,
) -> ScResult<crate::preferencias::AjustesAvisos> {
    en_db(&estado, crate::preferencias::avisos).await
}

#[tauri::command]
pub async fn guardar_avisos(
    estado: tauri::State<'_, Estado>,
    ajustes: crate::preferencias::AjustesAvisos,
) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::preferencias::guardar_avisos(db, &ajustes)
    })
    .await
}

#[tauri::command]
pub async fn avisos_posponer(estado: tauri::State<'_, Estado>, minutos: i64) -> ScResult<()> {
    estado.avisos.posponer(minutos);
    Ok(())
}

#[tauri::command]
pub async fn politica_red(estado: tauri::State<'_, Estado>) -> ScResult<crate::net::PoliticaDeRed> {
    Ok(estado.red.politica())
}

#[tauri::command]
pub async fn cambiar_politica_red(
    app: AppHandle,
    estado: tauri::State<'_, Estado>,
    politica: crate::net::PoliticaDeRed,
) -> ScResult<()> {
    let red = std::sync::Arc::clone(&estado.red);
    en_db(&estado, move |db| {
        crate::preferencias::cambiar_politica(db, &red, politica)
    })
    .await?;
    // El enlace sigue a la política: se cierra al retirarla y se reabre al concederla.
    if estado
        .red
        .politica()
        .comprobar(crate::net::Servicio::Sincronizacion)
        .is_ok()
    {
        let _ = abrir_enlace(&app, &estado).await;
    } else {
        estado.enlace.cerrar().await;
    }
    Ok(())
}

/// Una copia administrada por NB Launcher cambia de carpeta en cada versión: una entrada de
/// inicio apuntaría a un ejecutable retirado. Ahí solo se permite retirar la entrada.
fn autoinicio_permitido(gestionada: bool, activar: Option<bool>) -> bool {
    !(gestionada && activar == Some(true))
}

/// Si la aplicación se abre al iniciar sesión.
#[tauri::command]
pub async fn inicio_con_sistema(app: AppHandle, activar: Option<bool>) -> ScResult<bool> {
    use tauri_plugin_autostart::ManagerExt;
    if !autoinicio_permitido(crate::launcher::gestionada(), activar) {
        return Err(ScError::ComponentUnavailable);
    }
    let gestor = app.autolaunch();
    if let Some(a) = activar {
        let r = if a { gestor.enable() } else { gestor.disable() };
        r.map_err(|_| ScError::ComponentUnavailable)?;
    }
    gestor
        .is_enabled()
        .map_err(|_| ScError::ComponentUnavailable)
}

/// La autorización se cambia exclusivamente desde la política explícita de red.
/// Consultar o instalar no vuelve a activar servicios revocados.
fn red_de_actualizaciones<R: tauri::Runtime>(
    app: &AppHandle<R>,
) -> std::sync::Arc<crate::net::Red> {
    std::sync::Arc::clone(&app.state::<Estado>().red)
}

#[tauri::command]
pub async fn actualizacion_buscar(app: AppHandle) -> ScResult<Option<crate::net::Actualizacion>> {
    let red = red_de_actualizaciones(&app);
    crate::net::buscar_actualizacion(&app, &red).await
}

/// Instala la versión más nueva y reinicia la aplicación con ella.
#[tauri::command]
pub async fn actualizacion_instalar(app: AppHandle) -> ScResult<()> {
    let red = red_de_actualizaciones(&app);
    if crate::net::instalar_actualizacion(&app, &red).await? {
        app.restart();
    }
    Ok(())
}

// --- Perfil y Hoy -----------------------------------------------------------

#[tauri::command]
pub async fn hoy(estado: tauri::State<'_, Estado>) -> ScResult<crate::hoy::Hoy> {
    en_db(&estado, crate::hoy::hoy).await
}

#[tauri::command]
pub async fn perfil_bienvenida(
    estado: tauri::State<'_, Estado>,
    respuestas: crate::perfil::Bienvenida,
) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::comida::sembrar(db)?;
        crate::perfil::completar_bienvenida(db, &respuestas)
    })
    .await
}

#[tauri::command]
pub async fn perfil_cambiar(
    estado: tauri::State<'_, Estado>,
    cambio: crate::perfil::CambioPerfil,
) -> ScResult<()> {
    en_db(&estado, move |db| crate::perfil::cambiar(db, &cambio)).await
}

// --- Comida ----------------------------------------------------------------

#[tauri::command]
pub async fn comida_dia(
    estado: tauri::State<'_, Estado>,
    dia: String,
) -> ScResult<crate::comida::DiaComida> {
    en_db(&estado, move |db| crate::comida::dia(db, &dia)).await
}

#[tauri::command]
pub async fn comida_buscar(
    estado: tauri::State<'_, Estado>,
    consulta: String,
    limit: u32,
) -> ScResult<Vec<crate::comida::Alimento>> {
    en_db(&estado, move |db| {
        crate::comida::buscar(&*db.conexion()?, &consulta, i64::from(limit))
    })
    .await
}

#[tauri::command]
pub async fn comida_alimento(
    estado: tauri::State<'_, Estado>,
    id: i64,
) -> ScResult<crate::comida::Alimento> {
    en_db(&estado, move |db| crate::comida::alimento(db, id)).await
}

#[tauri::command]
pub async fn comida_registrar(
    estado: tauri::State<'_, Estado>,
    alimento: i64,
    gramos: f64,
    comida: String,
) -> ScResult<crate::comida::Registrado> {
    en_db(&estado, move |db| {
        crate::comida::registrar(db, alimento, gramos, &comida)
    })
    .await
}

/// Cota: veinte partes por frase (`comida::proponer`).
#[tauri::command]
pub async fn comida_proponer(
    estado: tauri::State<'_, Estado>,
    frase: String,
) -> ScResult<Vec<crate::comida::Propuesta>> {
    en_db(&estado, move |db| crate::comida::proponer(db, &frase)).await
}

#[tauri::command]
pub async fn comida_registrar_frase(
    estado: tauri::State<'_, Estado>,
    frase: String,
    comida: String,
) -> ScResult<crate::comida::Registrado> {
    en_db(&estado, move |db| {
        crate::comida::registrar_frase(db, &frase, &comida)
    })
    .await
}

#[tauri::command]
pub async fn comida_repetir_ayer(
    estado: tauri::State<'_, Estado>,
) -> ScResult<crate::comida::Registrado> {
    en_db(&estado, crate::comida::repetir_ayer).await
}

#[tauri::command]
pub async fn comida_borrar(estado: tauri::State<'_, Estado>, ids: Vec<i64>) -> ScResult<()> {
    en_db(&estado, move |db| crate::comida::borrar(db, &ids)).await
}

#[tauri::command]
pub async fn comida_favorito(
    estado: tauri::State<'_, Estado>,
    id: i64,
    favorito: bool,
) -> ScResult<()> {
    en_db(&estado, move |db| crate::comida::favorito(db, id, favorito)).await
}

#[tauri::command]
pub async fn comida_crear(
    estado: tauri::State<'_, Estado>,
    alimento: crate::comida::NuevoAlimento,
) -> ScResult<crate::comida::Alimento> {
    en_db(&estado, move |db| crate::comida::crear(db, &alimento)).await
}

#[tauri::command]
pub async fn comida_beber(estado: tauri::State<'_, Estado>, ml: i64) -> ScResult<i64> {
    en_db(&estado, move |db| crate::comida::beber(db, ml)).await
}

#[tauri::command]
pub async fn comida_borrar_vaso(estado: tauri::State<'_, Estado>, id: i64) -> ScResult<()> {
    en_db(&estado, move |db| crate::comida::borrar_vaso(db, id)).await
}

/// Cota: cien combos (`comida::combos`).
#[tauri::command]
pub async fn comida_combos(
    estado: tauri::State<'_, Estado>,
) -> ScResult<Vec<crate::comida::Combo>> {
    en_db(&estado, crate::comida::combos).await
}

#[tauri::command]
pub async fn comida_guardar_combo(
    estado: tauri::State<'_, Estado>,
    nombre: String,
    dia: String,
    comida: String,
) -> ScResult<i64> {
    en_db(&estado, move |db| {
        crate::comida::guardar_combo(db, &nombre, &dia, &comida)
    })
    .await
}

#[tauri::command]
pub async fn comida_registrar_combo(
    estado: tauri::State<'_, Estado>,
    combo: i64,
    escala: f64,
    comida: String,
) -> ScResult<crate::comida::Registrado> {
    en_db(&estado, move |db| {
        crate::comida::registrar_combo(db, combo, escala, &comida)
    })
    .await
}

#[tauri::command]
pub async fn comida_borrar_combo(estado: tauri::State<'_, Estado>, id: i64) -> ScResult<()> {
    en_db(&estado, move |db| crate::comida::borrar_combo(db, id)).await
}

/// Busca un código de barras en el catálogo y, con consentimiento, en Open Food
/// Facts (que solo recibe el código).
#[tauri::command]
pub async fn comida_codigo(
    estado: tauri::State<'_, Estado>,
    codigo: String,
) -> ScResult<crate::comida::Alimento> {
    let red = std::sync::Arc::clone(&estado.red);
    en_db(&estado, move |db| {
        crate::comida::por_codigo_de_barras(db, &red, &codigo)
    })
    .await
}

// --- Cuerpo ----------------------------------------------------------------

#[tauri::command]
pub async fn cuerpo_resumen(
    estado: tauri::State<'_, Estado>,
) -> ScResult<crate::cuerpo::ResumenCuerpo> {
    en_db(&estado, crate::cuerpo::resumen).await
}

#[tauri::command]
pub async fn cuerpo_pesar(
    estado: tauri::State<'_, Estado>,
    peso: f64,
) -> ScResult<crate::cuerpo::ResultadoPesaje> {
    en_db(&estado, move |db| crate::cuerpo::pesar(db, peso)).await
}

#[tauri::command]
pub async fn cuerpo_parece_de_otra_persona(
    estado: tauri::State<'_, Estado>,
    peso: f64,
) -> ScResult<Option<f64>> {
    en_db(&estado, move |db| {
        crate::cuerpo::parece_de_otra_persona(db, peso)
    })
    .await
}

#[tauri::command]
pub async fn cuerpo_cinta(
    estado: tauri::State<'_, Estado>,
    cinta: crate::cuerpo::Cinta,
) -> ScResult<f64> {
    en_db(&estado, move |db| crate::cuerpo::medir_cinta(db, cinta)).await
}

#[tauri::command]
pub async fn cuerpo_borrar(estado: tauri::State<'_, Estado>, id: i64) -> ScResult<()> {
    en_db(&estado, move |db| crate::cuerpo::borrar(db, id)).await
}

#[tauri::command]
pub async fn cuerpo_entrenar(
    estado: tauri::State<'_, Estado>,
    categoria: crate::cuerpo::Categoria,
    intensidad: crate::cuerpo::Intensidad,
    minutos: i64,
) -> ScResult<i64> {
    en_db(&estado, move |db| {
        crate::cuerpo::entrenar(db, categoria, intensidad, minutos).map(|r| r.0)
    })
    .await
}

#[tauri::command]
pub async fn cuerpo_borrar_entreno(estado: tauri::State<'_, Estado>, id: i64) -> ScResult<()> {
    en_db(&estado, move |db| crate::cuerpo::borrar_entreno(db, id)).await
}

#[tauri::command]
pub async fn cuerpo_calibrar(
    estado: tauri::State<'_, Estado>,
) -> ScResult<crate::cuerpo::IntentoCalibracion> {
    en_db(&estado, crate::cuerpo::calibrar_ahora).await
}

// --- Ayuno -----------------------------------------------------------------

#[tauri::command]
pub async fn ayuno_estado(estado: tauri::State<'_, Estado>) -> ScResult<crate::ayuno::EstadoAyuno> {
    en_db(&estado, crate::ayuno::estado).await
}

#[tauri::command]
pub async fn ayuno_comprobar(
    estado: tauri::State<'_, Estado>,
    horas: i64,
) -> ScResult<Option<crate::ayuno::Aviso>> {
    en_db(&estado, move |db| crate::ayuno::comprobar(db, horas)).await
}

#[tauri::command]
pub async fn ayuno_empezar(
    estado: tauri::State<'_, Estado>,
    protocolo: String,
    horas: i64,
    hace_minutos: i64,
) -> ScResult<i64> {
    en_db(&estado, move |db| {
        crate::ayuno::empezar(db, &protocolo, horas, hace_minutos)
    })
    .await
}

#[tauri::command]
pub async fn ayuno_terminar(estado: tauri::State<'_, Estado>) -> ScResult<crate::ayuno::Terminado> {
    en_db(&estado, crate::ayuno::terminar).await
}

#[tauri::command]
pub async fn ayuno_ajustar(estado: tauri::State<'_, Estado>, hace_minutos: i64) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::ayuno::ajustar_inicio(db, hace_minutos)
    })
    .await
}

#[tauri::command]
pub async fn ayuno_borrar(estado: tauri::State<'_, Estado>, id: i64) -> ScResult<()> {
    en_db(&estado, move |db| crate::ayuno::borrar(db, id)).await
}

// --- Cuidado ---------------------------------------------------------------

#[tauri::command]
pub async fn cuidado_resumen(
    estado: tauri::State<'_, Estado>,
) -> ScResult<crate::cuidado::ResumenCuidado> {
    en_db(&estado, crate::cuidado::resumen).await
}

#[tauri::command]
pub async fn tratamiento_guardar(
    estado: tauri::State<'_, Estado>,
    id: Option<i64>,
    tratamiento: crate::cuidado::BorradorTratamiento,
) -> ScResult<i64> {
    en_db(&estado, move |db| {
        crate::cuidado::guardar_tratamiento(db, id, &tratamiento)
    })
    .await
}

/// Pausa o reanuda (`activo`) o borra (`activo = None`) un tratamiento.
#[tauri::command]
pub async fn tratamiento_estado(
    estado: tauri::State<'_, Estado>,
    id: i64,
    activo: Option<bool>,
) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::cuidado::tratamiento_activo(db, id, activo)
    })
    .await
}

#[tauri::command]
pub async fn tratamiento_reponer(
    estado: tauri::State<'_, Estado>,
    id: i64,
    unidades: f64,
) -> ScResult<()> {
    en_db(&estado, move |db| crate::cuidado::reponer(db, id, unidades)).await
}

#[tauri::command]
pub async fn toma_marcar(
    estado: tauri::State<'_, Estado>,
    tratamiento: i64,
    hora: Option<i64>,
    tomada: Option<bool>,
) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::cuidado::marcar_toma(db, tratamiento, hora, tomada)
    })
    .await
}

#[tauri::command]
pub async fn habito_crear(
    estado: tauri::State<'_, Estado>,
    nombre: String,
    icono: String,
    dias: i64,
) -> ScResult<i64> {
    en_db(&estado, move |db| {
        crate::cuidado::crear_habito(db, &nombre, &icono, dias)
    })
    .await
}

#[tauri::command]
pub async fn habito_quitar(
    estado: tauri::State<'_, Estado>,
    id: i64,
    borrar: bool,
) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::cuidado::quitar_habito(db, id, borrar)
    })
    .await
}

#[tauri::command]
pub async fn habito_marcar(
    estado: tauri::State<'_, Estado>,
    id: i64,
    hecho: bool,
) -> ScResult<Option<crate::nibi::Recompensa>> {
    en_db(&estado, move |db| {
        crate::cuidado::marcar_habito(db, id, hecho)
    })
    .await
}

#[tauri::command]
pub async fn sesion_completar(
    estado: tauri::State<'_, Estado>,
    tipo: String,
    variante: String,
    segundos: i64,
) -> ScResult<Option<crate::nibi::Recompensa>> {
    en_db(&estado, move |db| {
        crate::cuidado::completar_sesion(db, &tipo, &variante, segundos)
    })
    .await
}

#[tauri::command]
pub async fn vital_medir(
    estado: tauri::State<'_, Estado>,
    tipo: String,
    principal: f64,
    secundario: Option<f64>,
    contexto: Option<String>,
) -> ScResult<String> {
    en_db(&estado, move |db| {
        crate::cuidado::medir(db, &tipo, principal, secundario, contexto.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn sintoma_registrar(
    estado: tauri::State<'_, Estado>,
    clave: String,
    intensidad: i64,
    notas: Option<String>,
) -> ScResult<i64> {
    en_db(&estado, move |db| {
        crate::cuidado::sentir(db, &clave, intensidad, notas.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn chequeo_hecho(
    estado: tauri::State<'_, Estado>,
    id: i64,
    dia: String,
) -> ScResult<Option<crate::nibi::Recompensa>> {
    en_db(&estado, move |db| {
        crate::cuidado::chequeo_hecho(db, id, &dia)
    })
    .await
}

#[tauri::command]
pub async fn chequeo_ajustar(
    estado: tauri::State<'_, Estado>,
    id: Option<i64>,
    nombre: Option<String>,
    meses: i64,
    activo: bool,
) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::cuidado::ajustar_chequeo(db, id, nombre.as_deref(), meses, activo)
    })
    .await
}

#[tauri::command]
pub async fn sustancia_consumir(
    estado: tauri::State<'_, Estado>,
    clave: String,
    unidades: i64,
) -> ScResult<i64> {
    en_db(&estado, move |db| {
        crate::cuidado::consumir(db, &clave, unidades)
    })
    .await
}

#[tauri::command]
pub async fn sustancia_borrar(estado: tauri::State<'_, Estado>, id: i64) -> ScResult<()> {
    en_db(&estado, move |db| crate::cuidado::borrar_consumo(db, id)).await
}

#[tauri::command]
pub async fn libre_crear(
    estado: tauri::State<'_, Estado>,
    contador: crate::cuidado::NuevoContador,
) -> ScResult<i64> {
    en_db(&estado, move |db| {
        crate::cuidado::crear_contador(db, &contador)
    })
    .await
}

#[tauri::command]
pub async fn libre_ganas(
    estado: tauri::State<'_, Estado>,
    id: i64,
) -> ScResult<Option<crate::nibi::Recompensa>> {
    en_db(&estado, move |db| crate::cuidado::ganas_superadas(db, id)).await
}

#[tauri::command]
pub async fn libre_reiniciar(estado: tauri::State<'_, Estado>, id: i64) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::cuidado::reiniciar_contador(db, id)
    })
    .await
}

#[tauri::command]
pub async fn libre_celebrar(estado: tauri::State<'_, Estado>, id: i64) -> ScResult<Option<i64>> {
    en_db(&estado, move |db| crate::cuidado::celebrar_si_toca(db, id)).await
}

#[tauri::command]
pub async fn libre_quitar(estado: tauri::State<'_, Estado>, id: i64, borrar: bool) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::cuidado::quitar_contador(db, id, borrar)
    })
    .await
}

#[tauri::command]
pub async fn bienestar_guardar(
    estado: tauri::State<'_, Estado>,
    respuestas: Vec<i64>,
) -> ScResult<i64> {
    en_db(&estado, move |db| {
        crate::cuidado::bienestar(db, &respuestas)
    })
    .await
}

// --- Diario ----------------------------------------------------------------

#[tauri::command]
pub async fn diario_entradas(
    estado: tauri::State<'_, Estado>,
    limit: u32,
) -> ScResult<Vec<crate::diario::Entrada>> {
    en_db(&estado, move |db| {
        crate::diario::entradas(&*db.conexion()?, i64::from(limit))
    })
    .await
}

#[tauri::command]
pub async fn diario_escribir(
    estado: tauri::State<'_, Estado>,
    entrada: crate::diario::NuevaEntrada,
) -> ScResult<Option<crate::nibi::Recompensa>> {
    en_db(&estado, move |db| crate::diario::escribir(db, &entrada)).await
}

#[tauri::command]
pub async fn diario_borrar(estado: tauri::State<'_, Estado>, id: i64) -> ScResult<()> {
    en_db(&estado, move |db| crate::diario::borrar(db, id)).await
}

#[tauri::command]
pub async fn diario_sugerir(texto: String) -> ScResult<crate::texto::Sugerencia> {
    Ok(crate::texto::leer_diario(&texto, crate::idioma::actual()))
}

#[tauri::command]
pub async fn noche_anotar(
    estado: tauri::State<'_, Estado>,
    noche: crate::diario::Noche,
) -> ScResult<()> {
    en_db(&estado, move |db| crate::diario::anotar_noche(db, noche)).await
}

#[tauri::command]
pub async fn cafeina_tomar(
    estado: tauri::State<'_, Estado>,
    fuente: String,
    mg: f64,
) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::diario::tomar_cafeina(db, &fuente, mg)
    })
    .await
}

#[tauri::command]
pub async fn ciclo(estado: tauri::State<'_, Estado>) -> ScResult<Option<crate::diario::Ciclo>> {
    en_db(&estado, |db| crate::diario::ciclo(&*db.conexion()?)).await
}

#[tauri::command]
pub async fn ciclo_empezar(
    estado: tauri::State<'_, Estado>,
    dia: String,
    largo: i64,
) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::diario::empezar_ciclo(db, &dia, largo)
    })
    .await
}

// --- Nibi ------------------------------------------------------------------

#[tauri::command]
pub async fn nibi_estado(estado: tauri::State<'_, Estado>) -> ScResult<crate::nibi::EstadoNibi> {
    en_db(&estado, |db| {
        crate::hoy::hoy(db)?.nibi.ok_or(ScError::NotFound)
    })
    .await
}

#[tauri::command]
pub async fn nibi_comprar(estado: tauri::State<'_, Estado>, id: String) -> ScResult<()> {
    en_db(&estado, move |db| crate::nibi::comprar(db, &id)).await
}

#[tauri::command]
pub async fn nibi_poner(estado: tauri::State<'_, Estado>, id: String) -> ScResult<()> {
    en_db(&estado, move |db| crate::nibi::poner(db, &id)).await
}

#[tauri::command]
pub async fn nibi_renombrar(estado: tauri::State<'_, Estado>, nombre: String) -> ScResult<()> {
    en_db(&estado, move |db| crate::nibi::renombrar(db, &nombre)).await
}

// --- Copias ----------------------------------------------------------------

/// Guarda una copia donde la persona elija. `false` si canceló.
#[tauri::command]
pub async fn copia_exportar(app: AppHandle) -> ScResult<bool> {
    use tauri_plugin_dialog::DialogExt;
    let db = app.state::<Estado>().db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(destino) = app
            .dialog()
            .file()
            .set_title(crate::idioma::t(
                "Guardar una copia de seguridad",
                "Save a backup",
                "Salvar um backup",
            ))
            .add_filter("SQLite", &["sqlite"])
            .set_file_name(crate::copia::nombre_de_copia())
            .blocking_save_file()
            .and_then(|f| f.into_path().ok())
        else {
            return Ok(false);
        };
        crate::copia::exportar(&db, &destino).map(|()| true)
    })
    .await
    .map_err(|_| ScError::Internal)?
}

/// Elige una copia y la comprueba sin tocar nada. La ruta se queda en el
/// proceso: no cruza el puente.
#[tauri::command]
pub async fn copia_elegir(app: AppHandle) -> ScResult<Option<crate::copia::Inspeccion>> {
    use tauri_plugin_dialog::DialogExt;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(origen) = app
            .dialog()
            .file()
            .set_title(crate::idioma::t(
                "Restaurar una copia de seguridad",
                "Restore a backup",
                "Restaurar um backup",
            ))
            .add_filter("SQLite", &["sqlite", "db"])
            .blocking_pick_file()
            .and_then(|f| f.into_path().ok())
        else {
            return Ok(None);
        };
        let i = crate::copia::inspeccionar(&origen)?;
        if let Ok(mut elegida) = app.state::<Estado>().copia_elegida.lock() {
            *elegida = Some(origen);
        }
        Ok(Some(i))
    })
    .await
    .map_err(|_| ScError::Internal)?
}

/// Restaura la copia elegida. Avisa a la interfaz para que recargue todo.
#[tauri::command]
pub async fn copia_restaurar(app: AppHandle) -> ScResult<crate::copia::Inspeccion> {
    let (db, rutas, red, origen) = {
        let e = app.state::<Estado>();
        let origen = e.copia_elegida.lock().ok().and_then(|mut c| c.take());
        (
            e.db.clone(),
            e.rutas.clone(),
            std::sync::Arc::clone(&e.red),
            origen,
        )
    };
    let origen = origen.ok_or(ScError::NotFound)?;
    let i = tauri::async_runtime::spawn_blocking(move || {
        let i = crate::copia::restaurar(&db, &rutas, &origen)?;
        red.cambiar_politica(crate::preferencias::politica(&db));
        let _ = crate::preferencias::cargar_idioma(&db);
        Ok::<_, ScError>(i)
    })
    .await
    .map_err(|_| ScError::Internal)??;
    let _ = app.emit("datos:cambio", ());
    crate::bandeja::actualizar(&app);
    Ok(i)
}

/// Recetas propias y del teléfono. Cota: 200 (`recetas::recetas`).
#[tauri::command]
pub async fn recetas(estado: tauri::State<'_, Estado>) -> ScResult<Vec<crate::recetas::Receta>> {
    en_db(&estado, crate::recetas::recetas).await
}

#[tauri::command]
pub async fn receta_crear(
    estado: tauri::State<'_, Estado>,
    nombre: String,
    raciones: i64,
    dia: String,
    comida: String,
) -> ScResult<i64> {
    en_db(&estado, move |db| {
        crate::recetas::crear(db, &nombre, raciones, &dia, &comida)
    })
    .await
}

#[tauri::command]
pub async fn receta_registrar(
    app: AppHandle,
    estado: tauri::State<'_, Estado>,
    id: i64,
    raciones: f64,
    comida: String,
) -> ScResult<crate::comida::Registrado> {
    let r = en_db(&estado, move |db| {
        crate::recetas::registrar(db, id, raciones, &comida)
    })
    .await?;
    let _ = app.emit("datos:cambio", ());
    Ok(r)
}

#[tauri::command]
pub async fn receta_borrar(estado: tauri::State<'_, Estado>, id: i64) -> ScResult<()> {
    en_db(&estado, move |db| crate::recetas::borrar(db, id)).await
}

/// Días agregados, semana frente a la anterior y relaciones entre hábitos.
#[tauri::command]
pub async fn evolucion(
    estado: tauri::State<'_, Estado>,
    dias: u32,
) -> ScResult<crate::evolucion::Evolucion> {
    en_db(&estado, move |db| crate::evolucion::evolucion(db, dias)).await
}

/// Registros de un rango de días, de cualquier origen, para revisarlos y corregirlos.
/// Cota: un año como máximo y 500 registros (`evolucion::historial`).
#[tauri::command]
pub async fn historial(
    estado: tauri::State<'_, Estado>,
    desde: String,
    hasta: String,
) -> ScResult<Vec<crate::evolucion::Movimiento>> {
    en_db(&estado, move |db| {
        crate::evolucion::historial(db, &desde, &hasta)
    })
    .await
}

#[tauri::command]
pub async fn historial_borrar(
    app: AppHandle,
    estado: tauri::State<'_, Estado>,
    tipo: crate::evolucion::TipoDeMovimiento,
    id: i64,
) -> ScResult<()> {
    en_db(&estado, move |db| crate::evolucion::borrar(db, tipo, id)).await?;
    let _ = app.emit("datos:cambio", ());
    Ok(())
}

/// Abre el enlace local si la política lo permite; no concede permisos por sí solo.
pub(crate) async fn abrir_enlace(app: &AppHandle, estado: &Estado) -> ScResult<()> {
    let avisar_app = app.clone();
    let avisar: crate::sincronizacion::enlace::Avisar = std::sync::Arc::new(move || {
        let _ = avisar_app.emit("datos:cambio", ());
    });
    estado
        .enlace
        .abrir(
            std::sync::Arc::clone(&estado.red),
            estado.db.clone(),
            avisar,
        )
        .await
}

/// Estado de la conexión con el teléfono.
#[tauri::command]
pub async fn enlace_estado(
    estado: tauri::State<'_, Estado>,
) -> ScResult<crate::sincronizacion::enlace::EstadoEnlace> {
    estado.enlace.estado(&estado.red, estado.db.clone()).await
}

/// Activar concede el consentimiento general y el del servicio de sincronización
/// (una acción explícita de la persona) y abre el enlace; desactivar lo retira.
#[tauri::command]
pub async fn enlace_activar(
    app: AppHandle,
    estado: tauri::State<'_, Estado>,
    activar: bool,
) -> ScResult<crate::sincronizacion::enlace::EstadoEnlace> {
    let mut politica = estado.red.politica();
    if activar {
        politica.general = true;
        politica
            .servicios
            .insert(crate::net::Servicio::Sincronizacion);
    } else {
        politica
            .servicios
            .remove(&crate::net::Servicio::Sincronizacion);
    }
    let red = std::sync::Arc::clone(&estado.red);
    en_db(&estado, move |db| {
        crate::preferencias::cambiar_politica(db, &red, politica)
    })
    .await?;
    if activar {
        abrir_enlace(&app, &estado).await?;
    } else {
        estado.enlace.cerrar().await;
    }
    estado.enlace.estado(&estado.red, estado.db.clone()).await
}

/// Invitación de un solo uso para emparejar un teléfono, válida cinco minutos.
#[tauri::command]
pub async fn enlace_invitar(
    estado: tauri::State<'_, Estado>,
) -> ScResult<crate::sincronizacion::enlace::InvitacionEnlace> {
    estado.enlace.invitar().await
}

/// Retira la autorización de un teléfono. Sus datos ya sincronizados se conservan.
#[tauri::command]
pub async fn enlace_revocar(estado: tauri::State<'_, Estado>, nodo: String) -> ScResult<()> {
    estado.enlace.revocar(nodo).await
}

/// Cambios simultáneos pendientes de decidir.
/// Cota: los 100 más antiguos; al resolverlos aparecen los siguientes.
#[tauri::command]
pub async fn sincronizacion_conflictos(
    estado: tauri::State<'_, Estado>,
) -> ScResult<Vec<crate::sincronizacion::servicio::Conflicto>> {
    en_db(&estado, |db| {
        crate::sincronizacion::servicio::conflictos(&mut *db.conexion()?, 100)
    })
    .await
}

/// Conserva la versión de este equipo o la del teléfono.
#[tauri::command]
pub async fn sincronizacion_resolver(
    app: AppHandle,
    estado: tauri::State<'_, Estado>,
    nodo: String,
    operacion: String,
    usar_telefono: bool,
) -> ScResult<()> {
    en_db(&estado, move |db| {
        crate::sincronizacion::servicio::resolver(
            &mut *db.conexion()?,
            &nodo,
            &operacion,
            usar_telefono,
        )
    })
    .await?;
    let _ = app.emit("datos:cambio", ());
    Ok(())
}

#[cfg(test)]
mod contrato_de_consentimiento {
    use super::*;
    use crate::{
        avisos,
        net::{PoliticaDeRed, Red},
        paths::Rutas,
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn una_copia_administrada_no_crea_entradas_de_inicio_pero_puede_retirarlas() {
        assert!(!autoinicio_permitido(true, Some(true)));
        assert!(autoinicio_permitido(true, Some(false)));
        assert!(autoinicio_permitido(true, None));
        assert!(autoinicio_permitido(false, Some(true)));
    }

    #[test]
    fn pedir_la_red_del_actualizador_no_activa_ni_concede_servicios() {
        let db = BaseDeDatos::en_memoria().unwrap();
        let red = Arc::new(Red::new(PoliticaDeRed::default()));
        let rutas = Rutas {
            datos: Default::default(),
            cache: Default::default(),
            registros: Default::default(),
        };
        let app = tauri::test::mock_builder()
            .manage(Estado {
                rutas,
                red: Arc::clone(&red),
                db: db.clone(),
                avisos: avisos::Servicio::default(),
                copia_elegida: Mutex::new(None),
                enlace: Arc::default(),
            })
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let obtenida = red_de_actualizaciones(app.handle());
        assert!(Arc::ptr_eq(&red, &obtenida));
        assert_eq!(obtenida.politica(), PoliticaDeRed::default());
        assert_eq!(crate::preferencias::politica(&db), PoliticaDeRed::default());
    }
}
