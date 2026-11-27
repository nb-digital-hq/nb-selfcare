//! Nibi, las monedas y la racha (gamificación positiva).
//!
//! Las reglas son las del teléfono (ADR-018 y ADR-022 de NB SelfCare Mobile):
//! cada acción da monedas una vez al día, la primera del día avanza la racha, un
//! hueco se cubre con escudos si los hay y, si no, la racha vuelve a empezar sin
//! perder los escudos. **Nibi nunca enferma, nunca se entristece por la persona y
//! nunca regaña** (Constitución, regla 5); los medicamentos no dan monedas.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use crate::db::{self, fallo, BaseDeDatos};
use crate::error::{ScError, ScResult};
use crate::fechas;

/// Una acción que da monedas. La clave es la del libro mayor del teléfono.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accion {
    Pesaje,
    MetaDeAgua,
    AyunoCompleto,
    Entrenamiento,
    Diario,
    Respiracion,
    SueloPelvico,
    PausaActiva,
    HabitosDelDia,
    GanasSuperadas,
    HitoLibre,
    Chequeo,
    HitoDePeso,
    RevisionSemanal,
    /// Desayuno, almuerzo o cena (los snacks no dan monedas).
    Comida(&'static str),
}

impl Accion {
    #[must_use]
    pub fn clave(self) -> String {
        match self {
            Self::Pesaje => "weigh_in".into(),
            Self::MetaDeAgua => "water_goal".into(),
            Self::AyunoCompleto => "fast_done".into(),
            Self::Entrenamiento => "workout_done".into(),
            Self::Diario => "journal_entry".into(),
            Self::Respiracion => "mindful_breath".into(),
            Self::SueloPelvico => "pelvic_floor".into(),
            Self::PausaActiva => "active_break".into(),
            Self::HabitosDelDia => "habits_done".into(),
            Self::GanasSuperadas => "urge_surfed".into(),
            Self::HitoLibre => "quit_milestone".into(),
            Self::Chequeo => "checkup_done".into(),
            Self::HitoDePeso => "milestone_reached".into(),
            Self::RevisionSemanal => "sunday_checkin".into(),
            Self::Comida(tipo) => format!("meal_log_{tipo}"),
        }
    }

    #[must_use]
    pub fn monedas(self) -> i64 {
        match self {
            Self::AyunoCompleto | Self::Entrenamiento | Self::Chequeo => 20,
            Self::Pesaje | Self::MetaDeAgua | Self::RevisionSemanal => 15,
            Self::HitoLibre | Self::HitoDePeso => 25,
            _ => 10,
        }
    }

    /// La recompensa de registrar una comida de ese tipo, si la tiene.
    #[must_use]
    pub fn de_comida(tipo: &str) -> Option<Self> {
        match tipo {
            "breakfast" => Some(Self::Comida("breakfast")),
            "lunch" => Some(Self::Comida("lunch")),
            "dinner" => Some(Self::Comida("dinner")),
            _ => None,
        }
    }
}

/// Multiplicador del día: ×1,5 si se durmió la meta o si sueño profundo y REM
/// suman al menos 90 minutos.
#[must_use]
pub fn multiplicador_de_sueno(minutos: i64, profundo: i64, rem: i64, meta_h: f64) -> f64 {
    let meta = minutos > 0 && minutos >= (meta_h * 60.0).round() as i64;
    if meta || profundo + rem >= 90 {
        1.5
    } else {
        1.0
    }
}

/// Días completos sin actividad entre dos días.
fn dias_perdidos(ultimo: chrono::NaiveDate, dia: chrono::NaiveDate) -> i64 {
    ((dia - ultimo).num_days() - 1).max(0)
}

/// La racha tras la primera acción de un día: `(racha, máxima, escudos)`.
#[must_use]
pub fn racha_tras(
    dia: chrono::NaiveDate,
    ultimo: Option<chrono::NaiveDate>,
    racha: i64,
    maxima: i64,
    escudos: i64,
) -> (i64, i64, i64) {
    let (nueva, quedan) = match ultimo {
        Some(u) if racha > 0 => {
            let hueco = dias_perdidos(u, dia);
            if hueco == 0 {
                (racha + 1, escudos)
            } else if hueco <= escudos {
                (racha + 1, escudos - hueco)
            } else {
                (1, escudos)
            }
        }
        _ => (racha + 1, escudos),
    };
    (nueva, maxima.max(nueva), quedan)
}

/// Crea el estado de Nibi si no existe, como la semilla del teléfono.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la escritura falla.
pub fn asegurar(conn: &Connection) -> ScResult<()> {
    conn.execute(
        "INSERT INTO pet_status (pet_name, coins_balance, streak_shields_available)
         SELECT 'Nibi', 50, 1 WHERE NOT EXISTS (SELECT 1 FROM pet_status)",
        [],
    )
    .map_err(fallo)?;
    Ok(())
}

/// Lo que se ganó con una acción.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Recompensa {
    pub monedas: i64,
    pub racha: i64,
}

/// Da la recompensa de una acción del día, una sola vez. Se llama dentro de la
/// transacción de lo que la provoca. `None` si ya se había dado hoy.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la escritura falla.
pub fn premiar(conn: &Connection, accion: Accion) -> ScResult<Option<Recompensa>> {
    asegurar(conn)?;
    let dia = fechas::hoy();
    let clave_dia = fechas::clave(dia);
    let clave = accion.clave();
    let ya: bool = conn
        .query_row(
            "SELECT 1 FROM coin_ledger WHERE date_key = ?1 AND action_type = ?2",
            params![clave_dia, clave],
            |_| Ok(true),
        )
        .optional()
        .map_err(fallo)?
        .unwrap_or(false);
    if ya {
        return Ok(None);
    }
    let multiplicador = conn
        .query_row(
            "SELECT s.sleep_duration_minutes, s.sleep_deep_minutes, s.sleep_rem_minutes,
                    (SELECT daily_sleep_goal_hours FROM user_profiles LIMIT 1)
             FROM daily_summaries s WHERE s.date_key = ?1",
            [&clave_dia],
            |f| {
                Ok(multiplicador_de_sueno(
                    f.get(0)?,
                    f.get(1)?,
                    f.get(2)?,
                    f.get::<_, Option<f64>>(3)?.unwrap_or(7.5),
                ))
            },
        )
        .optional()
        .map_err(fallo)?
        .unwrap_or(1.0);
    let monedas = (accion.monedas() as f64 * multiplicador).ceil() as i64;
    let primera_del_dia: i64 = conn
        .query_row(
            "SELECT count(*) FROM coin_ledger WHERE date_key = ?1 AND amount > 0",
            [&clave_dia],
            |f| f.get(0),
        )
        .map_err(fallo)?;
    conn.execute(
        "INSERT INTO coin_ledger (created_at, date_key, action_type, amount, sleep_multiplier_applied)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![fechas::texto(fechas::ahora()), clave_dia, clave, monedas, multiplicador],
    )
    .map_err(fallo)?;
    let (id, racha, maxima, escudos): (i64, i64, i64, i64) = conn
        .query_row(
            "SELECT id, current_streak_days, highest_streak_days, streak_shields_available
             FROM pet_status ORDER BY id LIMIT 1",
            [],
            |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?, f.get(3)?)),
        )
        .map_err(fallo)?;
    let (racha, maxima, escudos) = if primera_del_dia == 0 {
        let ultimo: Option<String> = conn
            .query_row(
                "SELECT max(date_key) FROM coin_ledger WHERE date_key < ?1 AND amount > 0",
                [&clave_dia],
                |f| f.get(0),
            )
            .map_err(fallo)?;
        racha_tras(
            dia,
            ultimo.as_deref().and_then(fechas::de_clave),
            racha,
            maxima,
            escudos,
        )
    } else {
        (racha, maxima, escudos)
    };
    conn.execute(
        "UPDATE pet_status SET coins_balance = coins_balance + ?1, current_streak_days = ?2,
             highest_streak_days = ?3, streak_shields_available = ?4 WHERE id = ?5",
        params![monedas, racha, maxima, escudos, id],
    )
    .map_err(fallo)?;
    Ok(Some(Recompensa { monedas, racha }))
}

/// Un artículo de la tienda. Los nombres visibles están en los diccionarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub enum Tipo {
    Sombrero,
    Accesorio,
    Color,
    Escenario,
    Escudo,
}

/// El catálogo de la tienda, el del teléfono: nada se compra con dinero real.
pub const CATALOGO: &[(&str, Tipo, i64, i64)] = &[
    ("shield_daily", Tipo::Escudo, 25, 1),
    ("shield_weekend", Tipo::Escudo, 65, 3),
    ("hat_party", Tipo::Sombrero, 25, 0),
    ("hat_chef", Tipo::Sombrero, 30, 0),
    ("hat_beanie", Tipo::Sombrero, 30, 0),
    ("hat_cap", Tipo::Sombrero, 35, 0),
    ("hat_headband", Tipo::Sombrero, 35, 0),
    ("hat_flowers", Tipo::Sombrero, 40, 0),
    ("hat_zen_halo", Tipo::Sombrero, 45, 0),
    ("hat_crown", Tipo::Sombrero, 60, 0),
    ("hat_wizard", Tipo::Sombrero, 70, 0),
    ("acc_bowtie", Tipo::Accesorio, 25, 0),
    ("acc_scarf", Tipo::Accesorio, 30, 0),
    ("acc_bell", Tipo::Accesorio, 30, 0),
    ("acc_glasses", Tipo::Accesorio, 35, 0),
    ("acc_sunglasses", Tipo::Accesorio, 45, 0),
    ("color_jade", Tipo::Color, 0, 0),
    ("color_ocean", Tipo::Color, 50, 0),
    ("color_lilac", Tipo::Color, 50, 0),
    ("color_coral", Tipo::Color, 55, 0),
    ("color_ember", Tipo::Color, 60, 0),
    ("color_midnight", Tipo::Color, 80, 0),
    ("color_gold", Tipo::Color, 90, 0),
    ("bg_default_room", Tipo::Escenario, 0, 0),
    ("bg_zen_garden", Tipo::Escenario, 40, 0),
    ("bg_beach", Tipo::Escenario, 45, 0),
    ("bg_forest", Tipo::Escenario, 45, 0),
    ("bg_mountain_dawn", Tipo::Escenario, 50, 0),
    ("bg_snow", Tipo::Escenario, 50, 0),
    ("bg_starry_dojo", Tipo::Escenario, 55, 0),
    ("bg_sakura", Tipo::Escenario, 60, 0),
    ("bg_space", Tipo::Escenario, 75, 0),
];

fn articulo(id: &str) -> Option<(&'static str, Tipo, i64, i64)> {
    CATALOGO.iter().copied().find(|a| a.0 == id)
}

/// Un artículo con lo que la persona tiene de él.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Articulo {
    pub id: String,
    pub tipo: Tipo,
    pub precio: i64,
    pub tenido: bool,
    pub puesto: bool,
}

/// El estado de Nibi para su pantalla.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct EstadoNibi {
    pub nombre: String,
    pub monedas: i64,
    pub racha: i64,
    pub racha_maxima: i64,
    pub escudos: i64,
    pub sombrero: Option<String>,
    pub accesorio: Option<String>,
    pub color: String,
    pub escenario: String,
    pub articulos: Vec<Articulo>,
    /// Lo que está haciendo según el día: `sleeping`, `zen`, `happy`…
    pub animo: String,
    /// Aura de la cetosis (ayuno de 12 h o más).
    pub aura: bool,
    /// Monedas ganadas hoy.
    pub monedas_hoy: i64,
}

/// Lo que pesa en el ánimo de Nibi.
#[derive(Debug, Clone, Copy, Default)]
pub struct DatosDelDia {
    pub hora: u32,
    pub agua: f64,
    pub durmio_bien: bool,
    pub horas_de_ayuno: Option<f64>,
    pub entreno: bool,
    pub racha: i64,
    pub comidas: i64,
}

/// El ánimo de base: nunca uno negativo; como mucho duerme o tiene curiosidad.
#[must_use]
pub fn animo(d: DatosDelDia) -> (&'static str, bool) {
    let aura = d.horas_de_ayuno.unwrap_or(0.0) >= 12.0;
    let a = if d.hora >= 23 || d.hora < 6 {
        "sleeping"
    } else if aura {
        "zen"
    } else if d.agua >= 1.0 && d.durmio_bien {
        "celebrating"
    } else if d.agua >= 1.0 || d.durmio_bien || d.entreno || d.racha >= 7 {
        "happy"
    } else if d.comidas == 0 && d.hora >= 11 {
        "curious"
    } else {
        "idle"
    };
    (a, aura)
}

fn desbloqueados(texto: &str) -> Vec<String> {
    let mut v: Vec<String> = serde_json::from_str(texto).unwrap_or_default();
    for base in ["bg_default_room", "color_jade"] {
        if !v.iter().any(|x| x == base) {
            v.push(base.into());
        }
    }
    v
}

/// El estado de Nibi.
///
/// # Errors
///
/// [`ScError::StorageFailure`] si la consulta falla.
pub fn estado(db: &BaseDeDatos, dia: DatosDelDia) -> ScResult<EstadoNibi> {
    let conn = db.conexion()?;
    asegurar(&conn)?;
    let hoy = fechas::clave(fechas::hoy());
    let monedas_hoy: i64 = conn
        .query_row(
            "SELECT coalesce(sum(amount), 0) FROM coin_ledger WHERE date_key = ?1 AND amount > 0",
            [&hoy],
            |f| f.get(0),
        )
        .map_err(fallo)?;
    conn.query_row(
        "SELECT pet_name, coins_balance, current_streak_days, highest_streak_days,
                streak_shields_available, equipped_hat_id, equipped_accessory_id,
                equipped_color_id, equipped_background_id, unlocked_items_json
         FROM pet_status ORDER BY id LIMIT 1",
        [],
        |f| {
            let sombrero: Option<String> = f.get(5)?;
            let accesorio: Option<String> = f.get(6)?;
            let color: Option<String> = f.get(7)?;
            let escenario: String = f.get(8)?;
            let tenidos = desbloqueados(&f.get::<_, String>(9)?);
            let racha: i64 = f.get(2)?;
            let valido =
                |id: Option<String>, t: Tipo| id.filter(|i| articulo(i).is_some_and(|a| a.1 == t));
            let sombrero = valido(sombrero, Tipo::Sombrero);
            let accesorio = valido(accesorio, Tipo::Accesorio);
            let color = valido(color, Tipo::Color).unwrap_or_else(|| "color_jade".into());
            let escenario = valido(Some(escenario), Tipo::Escenario)
                .unwrap_or_else(|| "bg_default_room".into());
            let articulos = CATALOGO
                .iter()
                .map(|(id, tipo, precio, _)| Articulo {
                    id: (*id).into(),
                    tipo: *tipo,
                    precio: *precio,
                    tenido: *tipo != Tipo::Escudo && tenidos.iter().any(|t| t == id),
                    puesto: [&sombrero, &accesorio]
                        .iter()
                        .any(|p| p.as_deref() == Some(*id))
                        || color == *id
                        || escenario == *id,
                })
                .collect();
            let (animo, aura) = animo(DatosDelDia { racha, ..dia });
            Ok(EstadoNibi {
                nombre: f.get(0)?,
                monedas: f.get(1)?,
                racha,
                racha_maxima: f.get(3)?,
                escudos: f.get(4)?,
                sombrero,
                accesorio,
                color,
                escenario,
                articulos,
                animo: animo.into(),
                aura,
                monedas_hoy,
            })
        },
    )
    .map_err(fallo)
}

/// Compra un artículo. Un escudo suma escudos; un cosmético queda para siempre.
///
/// # Errors
///
/// [`ScError::NotFound`] si el artículo no existe; [`ScError::InvalidInput`] si
/// no alcanzan las monedas o ya se tiene.
pub fn comprar(db: &BaseDeDatos, id: &str) -> ScResult<()> {
    let (id, tipo, precio, escudos) = articulo(id).ok_or(ScError::NotFound)?;
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    asegurar(&tx)?;
    let (pid, monedas, tenidos): (i64, i64, String) = tx
        .query_row(
            "SELECT id, coins_balance, unlocked_items_json FROM pet_status ORDER BY id LIMIT 1",
            [],
            |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)),
        )
        .map_err(fallo)?;
    let mut tenidos = desbloqueados(&tenidos);
    let ya = tipo != Tipo::Escudo && tenidos.iter().any(|t| t == id);
    if monedas < precio || ya {
        return Err(ScError::InvalidInput);
    }
    tx.execute(
        "INSERT INTO coin_ledger (created_at, date_key, action_type, amount) VALUES (?1, ?2, 'shop_buy', ?3)",
        params![fechas::texto(fechas::ahora()), fechas::clave(fechas::hoy()), -precio],
    )
    .map_err(fallo)?;
    if tipo != Tipo::Escudo {
        tenidos.push(id.into());
    }
    tx.execute(
        "UPDATE pet_status SET coins_balance = coins_balance - ?1,
             streak_shields_available = streak_shields_available + ?2,
             unlocked_items_json = ?3 WHERE id = ?4",
        params![
            precio,
            escudos,
            serde_json::to_string(&tenidos).unwrap_or_default(),
            pid
        ],
    )
    .map_err(fallo)?;
    tx.commit().map_err(fallo)?;
    Ok(())
}

/// Se pone (o se quita, si ya lo llevaba) un artículo que se tiene.
///
/// # Errors
///
/// [`ScError::NotFound`] si no existe; [`ScError::InvalidInput`] si no se tiene.
pub fn poner(db: &BaseDeDatos, id: &str) -> ScResult<()> {
    let (id, tipo, _, _) = articulo(id).ok_or(ScError::NotFound)?;
    let mut conn = db.conexion()?;
    let tx = db::transaccion_de_escritura(&mut conn)?;
    asegurar(&tx)?;
    let (tenidos, sombrero, accesorio): (String, Option<String>, Option<String>) = tx
        .query_row(
            "SELECT unlocked_items_json, equipped_hat_id, equipped_accessory_id FROM pet_status ORDER BY id LIMIT 1",
            [],
            |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)),
        )
        .map_err(fallo)?;
    if !desbloqueados(&tenidos).iter().any(|t| t == id) {
        return Err(ScError::InvalidInput);
    }
    let alternar = |actual: Option<String>| -> Option<&str> {
        if actual.as_deref() == Some(id) {
            None
        } else {
            Some(id)
        }
    };
    let (columna, valor) = match tipo {
        Tipo::Sombrero => ("equipped_hat_id", alternar(sombrero)),
        Tipo::Accesorio => ("equipped_accessory_id", alternar(accesorio)),
        Tipo::Color => ("equipped_color_id", Some(id)),
        Tipo::Escenario => ("equipped_background_id", Some(id)),
        Tipo::Escudo => return Err(ScError::InvalidInput),
    };
    tx.execute(&format!("UPDATE pet_status SET {columna} = ?1"), [valor])
        .map_err(fallo)?;
    tx.commit().map_err(fallo)?;
    Ok(())
}

/// Cambia el nombre de Nibi.
///
/// # Errors
///
/// [`ScError::InvalidInput`] si queda vacío o pasa de 24 caracteres.
pub fn renombrar(db: &BaseDeDatos, nombre: &str) -> ScResult<()> {
    let n = nombre.trim();
    if n.is_empty() || n.chars().count() > 24 {
        return Err(ScError::InvalidInput);
    }
    let conn = db.conexion()?;
    asegurar(&conn)?;
    conn.execute("UPDATE pet_status SET pet_name = ?1", [n])
        .map_err(fallo)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn d(t: &str) -> NaiveDate {
        fechas::de_clave(t).unwrap()
    }

    #[test]
    fn un_dia_seguido_suma_uno() {
        assert_eq!(
            racha_tras(d("2026-01-02"), Some(d("2026-01-01")), 4, 4, 1),
            (5, 5, 1)
        );
    }

    #[test]
    fn un_hueco_se_cubre_con_escudos() {
        assert_eq!(
            racha_tras(d("2026-01-04"), Some(d("2026-01-01")), 4, 9, 2),
            (5, 9, 0)
        );
    }

    #[test]
    fn sin_escudos_suficientes_se_empieza_de_nuevo_sin_perderlos() {
        assert_eq!(
            racha_tras(d("2026-01-05"), Some(d("2026-01-01")), 4, 9, 2),
            (1, 9, 2)
        );
    }

    #[test]
    fn dormir_bien_multiplica() {
        assert_eq!(multiplicador_de_sueno(450, 0, 0, 7.5), 1.5);
        assert_eq!(multiplicador_de_sueno(300, 50, 40, 7.5), 1.5);
        assert_eq!(multiplicador_de_sueno(300, 10, 10, 7.5), 1.0);
    }

    #[test]
    fn nibi_nunca_esta_mal() {
        for hora in 0..24 {
            let (a, _) = animo(DatosDelDia {
                hora,
                ..Default::default()
            });
            assert!(["sleeping", "idle", "curious"].contains(&a), "{hora}: {a}");
        }
        let (a, aura) = animo(DatosDelDia {
            hora: 14,
            horas_de_ayuno: Some(13.0),
            ..Default::default()
        });
        assert_eq!((a, aura), ("zen", true));
    }

    #[test]
    fn cada_accion_se_premia_una_vez_al_dia() {
        let db = crate::perfil::pruebas::base_con_perfil();
        let conn = db.conexion().unwrap();
        let r = premiar(&conn, Accion::Diario).unwrap();
        assert!(r.is_some());
        assert_eq!(premiar(&conn, Accion::Diario).unwrap(), None);
    }

    #[test]
    fn la_tienda_cobra_y_no_vende_dos_veces_un_cosmetico() {
        let db = crate::perfil::pruebas::base_con_perfil();
        comprar(&db, "hat_party").unwrap();
        assert_eq!(comprar(&db, "hat_party"), Err(ScError::InvalidInput));
        poner(&db, "hat_party").unwrap();
        let e = estado(&db, DatosDelDia::default()).unwrap();
        assert_eq!(e.sombrero.as_deref(), Some("hat_party"));
        assert_eq!(
            e.monedas,
            50 + 15 - 25,
            "50 de inicio + el primer pesaje - el sombrero"
        );
        assert_eq!(
            comprar(&db, "bg_space"),
            Err(ScError::InvalidInput),
            "no alcanza"
        );
    }
}
