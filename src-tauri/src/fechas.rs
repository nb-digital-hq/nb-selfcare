//! La hora y las fechas: de dónde salen y cómo se guardan.
//!
//! **La hora sale de un solo sitio** (Constitución, regla 4): ninguna regla llama
//! a `Local::now()` por su cuenta, sino a [`ahora`], que en las pruebas se puede
//! fijar por hilo. Así una prueba de "a medianoche cambia el día" no depende de
//! cuándo se ejecute.
//!
//! **El formato es el de NB SelfCare Mobile** (ADR-004): las fechas se guardan
//! como texto ISO-8601 con la diferencia horaria local detrás de un espacio
//! (`2026-01-02T08:30:05.123 -05:00`), que es lo que escribe Drift en el
//! teléfono, y los días como `YYYY-MM-DD`. Al leer se aceptan además las
//! variantes que puede traer una copia: con `Z`, sin diferencia horaria y el
//! `CURRENT_TIMESTAMP` de SQLite (que es UTC).

use chrono::{DateTime, Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};

#[cfg(test)]
thread_local! {
    static FIJADA: std::cell::Cell<Option<NaiveDateTime>> = const { std::cell::Cell::new(None) };
}

/// La hora local de ahora, sin zona: todas las reglas razonan en la hora de
/// quien usa la aplicación.
#[must_use]
pub fn ahora() -> NaiveDateTime {
    #[cfg(test)]
    if let Some(f) = FIJADA.with(std::cell::Cell::get) {
        return f;
    }
    Local::now().naive_local()
}

/// Fija la hora de este hilo. Solo en las pruebas.
#[cfg(test)]
pub fn fijar(hora: NaiveDateTime) {
    FIJADA.with(|c| c.set(Some(hora)));
}

/// El día de hoy.
#[must_use]
pub fn hoy() -> NaiveDate {
    ahora().date()
}

/// La clave de un día, como la guarda el teléfono: `YYYY-MM-DD`.
#[must_use]
pub fn clave(dia: NaiveDate) -> String {
    dia.format("%Y-%m-%d").to_string()
}

/// El día de una clave `YYYY-MM-DD`.
#[must_use]
pub fn de_clave(clave: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(clave.get(..10)?, "%Y-%m-%d").ok()
}

/// Una hora local en el formato de Drift, con la diferencia horaria que tenía
/// ese momento en la zona del equipo.
#[must_use]
pub fn texto(hora: NaiveDateTime) -> String {
    let desfase = Local
        .from_local_datetime(&hora)
        .earliest()
        .map_or(0, |h| h.offset().local_minus_utc());
    let signo = if desfase < 0 { '-' } else { '+' };
    let minutos = desfase.abs() / 60;
    format!(
        "{} {signo}{:02}:{:02}",
        hora.format("%Y-%m-%dT%H:%M:%S%.3f"),
        minutos / 60,
        minutos % 60
    )
}

/// Lee una fecha guardada y la devuelve en hora local. `None` si no se entiende.
#[must_use]
pub fn leer(texto: &str) -> Option<NaiveDateTime> {
    let t = texto.trim();
    // `2026-01-02T08:30:05.123 -05:00`: el formato de Drift para horas locales.
    if let Some((base, desfase)) = t.rsplit_once(' ') {
        if desfase.starts_with(['+', '-']) && desfase.len() == 6 {
            let conjunto = format!("{base}{desfase}");
            if let Ok(h) =
                DateTime::<FixedOffset>::parse_from_str(&conjunto, "%Y-%m-%dT%H:%M:%S%.f%:z")
            {
                return Some(h.with_timezone(&Local).naive_local());
            }
        }
    }
    // Con `Z` o con la diferencia pegada: RFC 3339.
    if let Ok(h) = DateTime::parse_from_rfc3339(t) {
        return Some(h.with_timezone(&Local).naive_local());
    }
    // Sin zona: hora local tal cual (Drift con horas locales antiguas).
    for formato in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M"] {
        if let Ok(h) = NaiveDateTime::parse_from_str(t, formato) {
            return Some(h);
        }
    }
    // `CURRENT_TIMESTAMP` de SQLite: UTC con un espacio en medio.
    if let Ok(h) = NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S") {
        return Some(
            Utc.from_utc_datetime(&h)
                .with_timezone(&Local)
                .naive_local(),
        );
    }
    // Solo el día.
    de_clave(t).and_then(|d| d.and_hms_opt(0, 0, 0))
}

/// Minutos desde la medianoche de una hora local.
#[must_use]
pub fn minutos_del_dia(hora: NaiveDateTime) -> u32 {
    use chrono::Timelike as _;
    hora.hour() * 60 + hora.minute()
}

/// El día de la semana como bit del teléfono: lunes es el bit 0 y domingo el 6
/// (`127` son todos los días).
#[must_use]
pub fn bit_del_dia(dia: NaiveDate) -> u8 {
    use chrono::Datelike as _;
    1 << dia.weekday().num_days_from_monday()
}

/// Días completos entre dos horas, como `Duration::inDays` de Dart (trunca).
#[must_use]
pub fn dias_entre(desde: NaiveDateTime, hasta: NaiveDateTime) -> i64 {
    (hasta - desde).num_days()
}

/// Edad en años con decimales, como `calculateDynamicAge` del teléfono.
#[must_use]
pub fn edad(nacimiento: NaiveDateTime, referencia: NaiveDateTime) -> f64 {
    let dias = (referencia - nacimiento).num_days();
    if dias > 0 {
        dias as f64 / 365.25
    } else {
        0.0
    }
}

/// Medianoche de un día.
#[must_use]
pub fn inicio(dia: NaiveDate) -> NaiveDateTime {
    dia.and_hms_opt(0, 0, 0).unwrap_or_default()
}

/// Suma días a una fecha.
#[must_use]
pub fn mas_dias(dia: NaiveDate, n: i64) -> NaiveDate {
    dia + Duration::days(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(t: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn lo_que_se_escribe_se_lee_igual() {
        let hora = h("2026-03-14 21:07:09");
        assert_eq!(leer(&texto(hora)), Some(hora));
    }

    #[test]
    fn se_escribe_como_drift_con_la_diferencia_horaria_detras_de_un_espacio() {
        let t = texto(h("2026-01-02 08:30:05"));
        assert!(t.starts_with("2026-01-02T08:30:05.000 "), "{t}");
        let desfase = t.rsplit_once(' ').unwrap().1;
        assert!(desfase.starts_with(['+', '-']) && desfase.len() == 6, "{t}");
    }

    #[test]
    fn se_leen_todas_las_variantes_que_puede_traer_una_copia() {
        for t in [
            "2026-01-02T08:30:05.123 -05:00",
            "2026-01-02T08:30:00.000Z",
            "2026-01-02T08:30:05.123",
            "2026-01-02 08:30:00",
            "2026-01-02",
        ] {
            assert!(leer(t).is_some(), "{t}");
        }
        assert_eq!(leer("ayer"), None);
    }

    #[test]
    fn la_hora_fijada_es_la_que_ven_las_reglas() {
        fijar(h("2026-05-01 10:00:00"));
        assert_eq!(clave(hoy()), "2026-05-01");
        assert_eq!(minutos_del_dia(ahora()), 600);
    }

    #[test]
    fn el_lunes_es_el_primer_bit_y_el_domingo_el_ultimo() {
        assert_eq!(bit_del_dia(de_clave("2026-11-16").unwrap()), 1);
        assert_eq!(bit_del_dia(de_clave("2026-11-22").unwrap()), 64);
    }

    #[test]
    fn la_edad_cuenta_anios_de_365_25_dias() {
        let e = edad(h("1990-01-01 00:00:00"), h("2026-01-01 00:00:00"));
        assert!((e - 36.0).abs() < 0.001, "{e}");
    }
}
