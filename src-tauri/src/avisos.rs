//! Los avisos del escritorio: pausas activas, descanso visual 20-20-20, tomas
//! de medicamentos y agua (ADR-007).
//!
//! Un hilo despierta cada medio minuto, mira la hora y lo registrado, y decide
//! con [`que_avisar`], que es pura y se prueba sin reloj ni base. Las pausas y el
//! descanso visual solo avisan **dentro de la jornada** que la persona fijó, y
//! cuentan desde la última sesión hecha o el último aviso: quien acaba de hacer
//! una pausa no recibe otra al minuto. Las tomas avisan una vez, a su hora, con
//! texto discreto si así se pidió (sin el nombre del medicamento). Nada de esto
//! sale del equipo: son notificaciones del sistema.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime};
use serde::Serialize;
use ts_rs::TS;

use crate::db::BaseDeDatos;
use crate::fechas;
use crate::idioma::t;
use crate::preferencias::{self, AjustesAvisos};

/// Lo que hay que avisar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(
    export,
    rename = "AvisoEscritorio",
    export_to = "../../src/lib/bindings/"
)]
#[serde(rename_all = "camelCase", tag = "tipo")]
pub enum Aviso {
    Pausa,
    Vista,
    Agua,
    /// Una toma a su hora.
    Toma {
        tratamiento: i64,
        hora: i64,
        nombre: String,
    },
}

/// Lo que el hilo recuerda entre una vuelta y otra. Vive en memoria: al
/// reiniciar la aplicación, la jornada vuelve a contar desde ese momento.
#[derive(Debug, Default)]
pub struct Memoria {
    pub ultima_pausa: Option<NaiveDateTime>,
    pub ultima_vista: Option<NaiveDateTime>,
    pub ultimo_agua: Option<NaiveDateTime>,
    pub pospuesto_hasta: Option<NaiveDateTime>,
    pub tomas_avisadas: HashSet<(NaiveDate, i64, i64)>,
}

/// Lo registrado que pesa en los avisos.
#[derive(Debug, Clone, Default)]
pub struct Datos {
    pub ultima_pausa_hecha: Option<NaiveDateTime>,
    pub ultima_vista_hecha: Option<NaiveDateTime>,
    pub agua_ml: i64,
    pub agua_meta_ml: i64,
    /// Tomas pendientes de hoy con aviso: `(tratamiento, minuto, nombre)`.
    pub tomas: Vec<(i64, i64, String)>,
}

/// Si `ahora` cae dentro de la jornada.
#[must_use]
pub fn en_jornada(a: &AjustesAvisos, ahora: NaiveDateTime) -> bool {
    let minuto = fechas::minutos_del_dia(ahora);
    a.jornada_dias & fechas::bit_del_dia(ahora.date()) != 0
        && minuto >= a.jornada_desde
        && minuto < a.jornada_hasta
}

fn vencido(
    cada: u32,
    desde: &[Option<NaiveDateTime>],
    inicio_jornada: NaiveDateTime,
    ahora: NaiveDateTime,
) -> bool {
    if cada == 0 {
        return false;
    }
    let referencia = desde
        .iter()
        .flatten()
        .copied()
        .chain(std::iter::once(inicio_jornada))
        .max()
        .unwrap_or(inicio_jornada);
    (ahora - referencia).num_minutes() >= i64::from(cada)
}

/// Decide qué avisar ahora y lo apunta en la memoria.
pub fn que_avisar(
    a: &AjustesAvisos,
    m: &mut Memoria,
    d: &Datos,
    ahora: NaiveDateTime,
    abierta_desde: NaiveDateTime,
) -> Vec<Aviso> {
    let mut v = Vec::new();
    let pospuesto = m.pospuesto_hasta.is_some_and(|p| ahora < p);
    if en_jornada(a, ahora) && !pospuesto {
        let inicio = (fechas::inicio(ahora.date())
            + chrono::Duration::minutes(i64::from(a.jornada_desde)))
        .max(abierta_desde);
        if vencido(
            a.pausa_cada_min,
            &[d.ultima_pausa_hecha, m.ultima_pausa],
            inicio,
            ahora,
        ) {
            m.ultima_pausa = Some(ahora);
            // La pausa ya incluye mirar lejos: el descanso visual vuelve a contar.
            m.ultima_vista = Some(ahora);
            v.push(Aviso::Pausa);
        } else if vencido(
            a.vista_cada_min,
            &[d.ultima_vista_hecha, d.ultima_pausa_hecha, m.ultima_vista],
            inicio,
            ahora,
        ) {
            m.ultima_vista = Some(ahora);
            v.push(Aviso::Vista);
        }
        if d.agua_meta_ml > 0
            && d.agua_ml < d.agua_meta_ml
            && vencido(a.agua_cada_min, &[m.ultimo_agua], inicio, ahora)
        {
            m.ultimo_agua = Some(ahora);
            v.push(Aviso::Agua);
        }
    }
    if a.tomas {
        let minuto = i64::from(fechas::minutos_del_dia(ahora));
        let hoy = ahora.date();
        for (id, hora, nombre) in &d.tomas {
            // A su hora y hasta 30 minutos después; una sola vez.
            if minuto >= *hora && minuto - hora <= 30 && m.tomas_avisadas.insert((hoy, *id, *hora))
            {
                v.push(Aviso::Toma {
                    tratamiento: *id,
                    hora: *hora,
                    nombre: nombre.clone(),
                });
            }
        }
        m.tomas_avisadas.retain(|(d, _, _)| *d == hoy);
    }
    v
}

fn datos(db: &BaseDeDatos) -> crate::error::ScResult<(Datos, bool)> {
    let conn = db.conexion()?;
    let hoy = fechas::hoy();
    let dia = fechas::clave(hoy);
    let ultima = |filtro: &str| -> Option<NaiveDateTime> {
        conn.query_row(
            &format!("SELECT max(completed_at) FROM care_sessions WHERE kind = 'active_break' AND {filtro}"),
            [],
            |f| f.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|t| fechas::leer(&t))
    };
    let perfil = crate::perfil::leer(&conn)?;
    let discretos = perfil.as_ref().is_none_or(|p| p.perfil.avisos_discretos);
    let con_aviso: HashSet<i64> = crate::cuidado::tratamientos(&conn)?
        .into_iter()
        .filter(|t| t.avisos && t.activo)
        .map(|t| t.id)
        .collect();
    let tomas = crate::cuidado::tomas_del_dia(&conn, hoy)?
        .into_iter()
        .filter(|t| t.estado == "pending" && con_aviso.contains(&t.tratamiento))
        .filter_map(|t| Some((t.tratamiento, t.hora?, t.nombre)))
        .collect();
    Ok((
        Datos {
            ultima_pausa_hecha: ultima("variant != 'eyes'"),
            ultima_vista_hecha: ultima("variant = 'eyes'"),
            agua_ml: crate::comida::agua_del_dia(&conn, &dia)?,
            agua_meta_ml: perfil.map_or(0, |p| p.perfil.agua_meta_ml),
            tomas,
        },
        discretos,
    ))
}

/// El texto de un aviso, en el idioma de la interfaz.
#[must_use]
pub fn texto(aviso: &Aviso, discretos: bool) -> (String, String) {
    match aviso {
        Aviso::Pausa => (
            t(
                "Hora de una pausa activa",
                "Time for an active break",
                "Hora de uma pausa ativa",
            )
            .into(),
            t(
                "Dos minutos para estirar cuello, espalda y muñecas.",
                "Two minutes to stretch your neck, back and wrists.",
                "Dois minutos para alongar pescoço, costas e punhos.",
            )
            .into(),
        ),
        Aviso::Vista => (
            t("Descanso visual", "Eye rest", "Descanso visual").into(),
            t(
                "Mira algo a seis metros durante veinte segundos.",
                "Look at something twenty feet away for twenty seconds.",
                "Olhe para algo a seis metros por vinte segundos.",
            )
            .into(),
        ),
        Aviso::Agua => (
            t("Un vaso de agua", "A glass of water", "Um copo de água").into(),
            t(
                "Aún no llegas a tu meta de hoy.",
                "You haven't reached today's goal yet.",
                "Você ainda não chegou à meta de hoje.",
            )
            .into(),
        ),
        Aviso::Toma { nombre, .. } => {
            let titulo = t(
                "Es la hora de tu toma",
                "It's time for your dose",
                "É a hora da sua dose",
            );
            let cuerpo = if discretos {
                t(
                    "Ábrela en NB SelfCare para marcarla.",
                    "Open NB SelfCare to mark it.",
                    "Abra o NB SelfCare para marcá-la.",
                )
                .to_owned()
            } else {
                nombre.clone()
            };
            (titulo.into(), cuerpo)
        }
    }
}

/// El servicio: el hilo y lo que la interfaz le pide (posponer, reiniciar).
#[derive(Clone, Default)]
pub struct Servicio {
    memoria: Arc<Mutex<Memoria>>,
}

impl Servicio {
    /// Pospone las pausas y el descanso visual.
    pub fn posponer(&self, minutos: i64) {
        if let Ok(mut m) = self.memoria.lock() {
            m.pospuesto_hasta =
                Some(fechas::ahora() + chrono::Duration::minutes(minutos.clamp(1, 240)));
        }
    }

    /// Arranca el hilo. `avisar` recibe cada aviso con su texto; se llama desde
    /// el hilo de los avisos, nunca desde el de la ventana.
    pub fn arrancar(
        &self,
        db: BaseDeDatos,
        avisar: impl Fn(&Aviso, String, String) + Send + 'static,
    ) {
        let memoria = Arc::clone(&self.memoria);
        let abierta = fechas::ahora();
        let lanzado = std::thread::Builder::new()
            .name("avisos".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(30));
                let ajustes = preferencias::avisos(&db).unwrap_or_default();
                let (d, discretos) = match datos(&db) {
                    Ok(x) => x,
                    Err(error) => {
                        tracing::warn!(
                            codigo = error.code(),
                            "no se pudieron leer los datos de los avisos"
                        );
                        continue;
                    }
                };
                let ahora = fechas::ahora();
                let avisos = match memoria.lock() {
                    Ok(mut m) => que_avisar(&ajustes, &mut m, &d, ahora, abierta),
                    Err(_) => continue,
                };
                for a in avisos {
                    let (titulo, cuerpo) = texto(&a, discretos);
                    avisar(&a, titulo, cuerpo);
                }
            });
        if lanzado.is_err() {
            tracing::error!("no se pudo lanzar el hilo de los avisos");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(t: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(&format!("2026-11-18 {t}"), "%Y-%m-%d %H:%M").unwrap()
    }

    fn ajustes() -> AjustesAvisos {
        AjustesAvisos::default()
    }

    #[test]
    fn fuera_de_la_jornada_no_se_avisa_de_pausas() {
        let mut m = Memoria::default();
        let d = Datos::default();
        assert!(que_avisar(&ajustes(), &mut m, &d, h("07:00"), h("06:00")).is_empty());
        assert!(que_avisar(&ajustes(), &mut m, &d, h("20:00"), h("06:00")).is_empty());
        // El 22 de noviembre de 2026 es domingo: fuera de una jornada de lunes a viernes.
        let domingo = NaiveDateTime::parse_from_str("2026-11-22 11:00", "%Y-%m-%d %H:%M").unwrap();
        assert!(que_avisar(&ajustes(), &mut m, &d, domingo, h("06:00")).is_empty());
    }

    #[test]
    fn la_vista_cada_20_y_la_pausa_cada_60_minutos() {
        let mut m = Memoria::default();
        let d = Datos::default();
        let a = ajustes();
        assert!(que_avisar(&a, &mut m, &d, h("09:10"), h("08:00")).is_empty());
        assert_eq!(
            que_avisar(&a, &mut m, &d, h("09:20"), h("08:00")),
            vec![Aviso::Vista]
        );
        assert!(que_avisar(&a, &mut m, &d, h("09:30"), h("08:00")).is_empty());
        assert_eq!(
            que_avisar(&a, &mut m, &d, h("09:40"), h("08:00")),
            vec![Aviso::Vista]
        );
        assert_eq!(
            que_avisar(&a, &mut m, &d, h("10:00"), h("08:00")),
            vec![Aviso::Pausa]
        );
        assert!(
            que_avisar(&a, &mut m, &d, h("10:05"), h("08:00")).is_empty(),
            "la pausa reinicia la vista"
        );
    }

    #[test]
    fn una_pausa_hecha_reinicia_la_cuenta() {
        let mut m = Memoria::default();
        let d = Datos {
            ultima_pausa_hecha: Some(h("09:50")),
            ..Default::default()
        };
        let mut a = ajustes();
        a.vista_cada_min = 0;
        assert!(que_avisar(&a, &mut m, &d, h("10:00"), h("08:00")).is_empty());
        assert_eq!(
            que_avisar(&a, &mut m, &d, h("10:50"), h("08:00")),
            vec![Aviso::Pausa]
        );
    }

    #[test]
    fn posponer_calla_las_pausas_pero_no_las_tomas() {
        let mut m = Memoria {
            pospuesto_hasta: Some(h("11:00")),
            ..Default::default()
        };
        let d = Datos {
            tomas: vec![(1, 600, "Vitamina D".into())],
            ..Default::default()
        };
        let v = que_avisar(&ajustes(), &mut m, &d, h("10:05"), h("08:00"));
        assert_eq!(
            v,
            vec![Aviso::Toma {
                tratamiento: 1,
                hora: 600,
                nombre: "Vitamina D".into()
            }]
        );
        assert!(
            que_avisar(&ajustes(), &mut m, &d, h("10:06"), h("08:00")).is_empty(),
            "una sola vez"
        );
    }

    #[test]
    fn una_toma_de_hace_mas_de_media_hora_no_se_avisa() {
        let mut m = Memoria::default();
        let d = Datos {
            tomas: vec![(1, 480, "x".into())],
            ..Default::default()
        };
        let mut a = ajustes();
        a.pausa_cada_min = 0;
        a.vista_cada_min = 0;
        assert!(que_avisar(&a, &mut m, &d, h("09:00"), h("06:00")).is_empty());
    }

    #[test]
    fn el_aviso_discreto_no_nombra_el_medicamento() {
        crate::idioma::fijar(crate::idioma::Idioma::Es);
        let a = Aviso::Toma {
            tratamiento: 1,
            hora: 480,
            nombre: "Losartán".into(),
        };
        assert!(!texto(&a, true).1.contains("Losartán"));
        assert_eq!(texto(&a, false).1, "Losartán");
    }
}

#[cfg(test)]
mod contrato_de_tipos {
    use super::Aviso;
    use ts_rs::TS;

    #[test]
    fn los_avisos_de_escritorio_y_ayuno_no_sobrescriben_el_mismo_binding() {
        let config = ts_rs::Config::default();
        assert_eq!(Aviso::name(&config), "AvisoEscritorio");
        assert_ne!(Aviso::name(&config), crate::ayuno::Aviso::name(&config));
    }
}
