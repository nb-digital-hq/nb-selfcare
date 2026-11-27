//! Las fórmulas del cuerpo y de la energía, puras: sin base de datos ni reloj.
//!
//! Son las mismas que usa NB SelfCare Mobile (`biometric_formulas.dart`,
//! `daily_targets_calculator.dart` y `diet_phase_policy.dart`), con los mismos
//! redondeos: un mismo registro tiene que dar la misma meta en el teléfono y en
//! el escritorio. Las pruebas de este archivo fijan valores calculados con la
//! implementación del teléfono.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// kcal que equivalen a un kilo de grasa corporal.
pub const KCAL_POR_KG_GRASA: f64 = 7700.0;
/// Factor de la media móvil exponencial del peso tendencia.
pub const ALFA_TENDENCIA: f64 = 0.1;

/// Sexo biológico, que solo cambia las fórmulas (`male` / `female` en la base).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub enum Sexo {
    Hombre,
    Mujer,
}

impl Sexo {
    #[must_use]
    pub fn desde_db(v: &str) -> Self {
        if v == "female" {
            Self::Mujer
        } else {
            Self::Hombre
        }
    }

    #[must_use]
    pub fn a_db(self) -> &'static str {
        match self {
            Self::Hombre => "male",
            Self::Mujer => "female",
        }
    }

    fn es_hombre(self) -> bool {
        self == Self::Hombre
    }
}

/// Fase de la dieta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub enum FaseDieta {
    Deficit,
    Mantenimiento,
    Descanso,
    Inversa,
}

impl FaseDieta {
    #[must_use]
    pub fn desde_db(v: &str) -> Self {
        match v {
            "maintenance" => Self::Mantenimiento,
            "diet_break" => Self::Descanso,
            "reverse_diet" => Self::Inversa,
            _ => Self::Deficit,
        }
    }

    #[must_use]
    pub fn a_db(self) -> &'static str {
        match self {
            Self::Deficit => "deficit",
            Self::Mantenimiento => "maintenance",
            Self::Descanso => "diet_break",
            Self::Inversa => "reverse_diet",
        }
    }
}

/// Estilo de alimentación, con su reparto de macronutrientes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub enum Arquetipo {
    Flexible,
    Mediterranea,
    Keto,
    AltaProteina,
    Vegetal,
    Equilibrada,
}

impl Arquetipo {
    pub const TODOS: [Arquetipo; 6] = [
        Self::Flexible,
        Self::Mediterranea,
        Self::Keto,
        Self::AltaProteina,
        Self::Vegetal,
        Self::Equilibrada,
    ];

    #[must_use]
    pub fn desde_db(v: &str) -> Self {
        match v {
            "mediterranean" => Self::Mediterranea,
            "keto" => Self::Keto,
            "high_protein" => Self::AltaProteina,
            "plant_based" => Self::Vegetal,
            "balanced" => Self::Equilibrada,
            _ => Self::Flexible,
        }
    }

    #[must_use]
    pub fn a_db(self) -> &'static str {
        match self {
            Self::Flexible => "flexible",
            Self::Mediterranea => "mediterranean",
            Self::Keto => "keto",
            Self::AltaProteina => "high_protein",
            Self::Vegetal => "plant_based",
            Self::Equilibrada => "balanced",
        }
    }

    /// Porcentajes de energía de proteína, carbohidratos y grasa.
    #[must_use]
    pub fn reparto(self) -> (f64, f64, f64) {
        match self {
            Self::Mediterranea => (20.0, 50.0, 30.0),
            Self::Keto => (25.0, 5.0, 70.0),
            Self::AltaProteina => (35.0, 40.0, 25.0),
            Self::Vegetal => (20.0, 55.0, 25.0),
            Self::Equilibrada => (25.0, 45.0, 30.0),
            Self::Flexible => (25.0, 50.0, 25.0),
        }
    }
}

fn acotar(v: f64, min: f64, max: f64) -> f64 {
    v.clamp(min, max)
}

/// Redondeo a `n` decimales como `toStringAsFixed` de Dart (mitad hacia arriba
/// en valor absoluto).
#[must_use]
pub fn fijo(v: f64, n: i32) -> f64 {
    let f = 10f64.powi(n);
    (v * f).round() / f
}

/// Índice de masa corporal, con dos decimales.
#[must_use]
pub fn imc(peso_kg: f64, altura_cm: f64) -> f64 {
    if altura_cm <= 0.0 {
        return 0.0;
    }
    let m = altura_cm / 100.0;
    fijo(peso_kg / (m * m), 2)
}

/// Las métricas que se derivan de un pesaje.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Metricas {
    pub imc: f64,
    pub grasa_pct: f64,
    pub musculo_kg: f64,
    pub agua_pct: f64,
    pub grasa_visceral: i64,
    pub hueso_kg: f64,
    pub proteina_pct: f64,
    pub metabolismo_basal: i64,
    pub edad_metabolica: i64,
    pub masa_magra_kg: f64,
    pub puntuacion: i64,
    /// Uno de los nueve somatotipos (`balanced`, `skinny_muscular`…).
    pub somatotipo: String,
}

/// Las métricas de un pesaje sin bioimpedancia: modelo antropométrico de
/// Deurenberg. En el escritorio es siempre este, porque no hay báscula.
#[must_use]
pub fn metricas_estimadas(peso_kg: f64, altura_cm: f64, edad: f64, sexo: Sexo) -> Metricas {
    let hombre = sexo.es_hombre();
    let imc = imc(peso_kg, altura_cm);
    let grasa = acotar(
        1.20 * imc + 0.23 * edad - 10.8 * if hombre { 1.0 } else { 0.0 } - 5.4,
        5.0,
        65.0,
    );
    let hueso = acotar(peso_kg * if hombre { 0.045 } else { 0.040 }, 1.0, 5.2);
    let musculo_bruto = peso_kg - peso_kg * (grasa / 100.0) - hueso;
    let musculo = fijo(
        if musculo_bruto > 0.0 {
            musculo_bruto
        } else {
            peso_kg * 0.5
        },
        2,
    );
    let agua = acotar((100.0 - grasa) * 0.72, 35.0, 75.0);
    let proteina = acotar((musculo / peso_kg) * 100.0 - agua, 10.0, 26.0);
    let visceral = grasa_visceral(peso_kg, altura_cm, edad, hombre);
    let magra = fijo(peso_kg * (1.0 - grasa / 100.0), 1);
    let basal = katch_mcardle(magra);
    let edad_metabolica = edad_metabolica(basal, edad, hombre, peso_kg, altura_cm);
    let puntuacion = puntuacion(imc, grasa, visceral, agua, hombre);
    let somatotipo = somatotipo(grasa, musculo, peso_kg, hombre).to_owned();
    Metricas {
        imc,
        grasa_pct: fijo(grasa, 1),
        musculo_kg: musculo,
        agua_pct: fijo(agua, 1),
        grasa_visceral: visceral,
        hueso_kg: fijo(hueso, 2),
        proteina_pct: fijo(proteina, 1),
        metabolismo_basal: basal,
        edad_metabolica,
        masa_magra_kg: magra,
        puntuacion,
        somatotipo,
    }
}

/// Metabolismo basal de Katch-McArdle: `370 + 21,6 × masa magra`.
#[must_use]
pub fn katch_mcardle(masa_magra_kg: f64) -> i64 {
    (370.0 + 21.6 * masa_magra_kg).round() as i64
}

fn grasa_visceral(peso: f64, altura: f64, edad: f64, hombre: bool) -> i64 {
    let visceral = if altura < peso * 1.6 {
        let sub = -(altura * 0.4 - altura * altura * 0.0826);
        peso * 305.0 / (sub + 48.0) - if hombre { 2.9 } else { 4.9 } + edad * 0.15
    } else {
        let sub = 0.765 + altura * -0.0015;
        -(altura * 0.143 - peso * sub) + edad * 0.15 - if hombre { 5.0 } else { 7.0 }
    };
    (visceral.round() as i64).clamp(1, 30)
}

fn edad_metabolica(basal: i64, edad: f64, hombre: bool, peso: f64, altura: f64) -> i64 {
    let esperado = if hombre {
        88.362 + 13.397 * peso + 4.799 * altura - 5.677 * edad
    } else {
        447.593 + 9.247 * peso + 3.098 * altura - 4.330 * edad
    };
    let delta = (1.0 - basal as f64 / esperado) * 15.0;
    let estimada = (edad + delta).round() as i64;
    let min = 12.max((edad - 10.0).round() as i64);
    let max = (edad + 15.0).round() as i64;
    estimada.clamp(min, max.max(min))
}

fn puntuacion(imc: f64, grasa: f64, visceral: i64, agua: f64, hombre: bool) -> i64 {
    let mut p = 100.0;
    if imc < 18.5 {
        p -= (18.5 - imc) * 3.0;
    } else if imc > 24.9 {
        p -= (imc - 24.9) * 2.5;
    }
    let (min, max) = if hombre { (10.0, 20.0) } else { (18.0, 28.0) };
    if grasa > max {
        p -= (grasa - max) * 1.5;
    } else if grasa < min {
        p -= min - grasa;
    }
    if visceral > 9 {
        p -= (visceral - 9) as f64 * 2.0;
    }
    let agua_min = if hombre { 50.0 } else { 45.0 };
    if agua < agua_min {
        p -= (agua_min - agua) * 0.8;
    }
    (p.round() as i64).clamp(10, 100)
}

fn somatotipo(grasa: f64, musculo: f64, peso: f64, hombre: bool) -> &'static str {
    let musculo_pct = musculo / peso * 100.0;
    let (g_bajo, g_alto) = if hombre { (12.0, 22.0) } else { (20.0, 30.0) };
    let (m_bajo, m_alto) = if hombre { (40.0, 52.0) } else { (32.0, 42.0) };
    let g = if grasa < g_bajo {
        0
    } else if grasa <= g_alto {
        1
    } else {
        2
    };
    let m = if musculo_pct < m_bajo {
        0
    } else if musculo_pct <= m_alto {
        1
    } else {
        2
    };
    match (g, m) {
        (2, 0) => "obese",
        (2, 1) => "overweight",
        (2, _) => "thick_set",
        (1, 0) => "lack_exercise",
        (1, 1) => "balanced",
        (1, _) => "balanced_muscular",
        (_, 0) => "skinny",
        (_, 1) => "balanced_skinny",
        _ => "skinny_muscular",
    }
}

/// Grasa corporal por la ecuación de la Marina de EE. UU. (cinta métrica).
#[must_use]
pub fn grasa_marina(
    altura_cm: f64,
    cintura_cm: f64,
    cuello_cm: f64,
    sexo: Sexo,
    cadera_cm: Option<f64>,
) -> f64 {
    if sexo.es_hombre() {
        let diff = cintura_cm - cuello_cm;
        if diff <= 0.0 {
            return 10.0;
        }
        let d = 1.0324 - 0.19077 * diff.log10() + 0.15456 * altura_cm.log10();
        acotar(495.0 / d - 450.0, 4.0, 60.0)
    } else {
        let cadera = cadera_cm.unwrap_or(cintura_cm * 1.1);
        let suma = cintura_cm + cadera - cuello_cm;
        if suma <= 0.0 {
            return 18.0;
        }
        let d = 1.29579 - 0.35004 * suma.log10() + 0.22100 * altura_cm.log10();
        acotar(495.0 / d - 450.0, 8.0, 65.0)
    }
}

/// Grasa validada: 60 % la estimación del pesaje y 40 % la cinta (ADR-012 del
/// teléfono).
#[must_use]
pub fn grasa_validada(estimada: f64, marina: f64) -> f64 {
    fijo(0.60 * estimada + 0.40 * marina, 1)
}

/// El peso tendencia: media móvil exponencial con `alfa = 0,1`.
#[must_use]
pub fn tendencia(peso_kg: f64, anterior: Option<f64>) -> f64 {
    match anterior {
        Some(a) if a > 0.0 => fijo(a + ALFA_TENDENCIA * (peso_kg - a), 2),
        _ => peso_kg,
    }
}

/// Recalibra el gasto diario tras 14 días: lo comido menos lo que explica el
/// cambio de peso, acotado a ±300 kcal y nunca por debajo del metabolismo basal.
#[must_use]
pub fn calibrar_gasto(
    media_kcal: f64,
    tendencia_inicio: f64,
    tendencia_fin: f64,
    gasto_anterior: i64,
    basal: i64,
) -> (i64, i64, bool) {
    let delta = tendencia_fin - tendencia_inicio;
    let bruto = media_kcal - delta * KCAL_POR_KG_GRASA / 14.0;
    let max = gasto_anterior as f64 + 300.0;
    let min = (gasto_anterior as f64 - 300.0).max(basal as f64);
    let acotado = acotar(bruto, min, max.max(min));
    let r = acotado.round() as i64;
    (r, bruto.round() as i64, r != bruto.round() as i64)
}

/// Gasto inicial: Mifflin-St Jeor por un factor según la meta de pasos.
#[must_use]
pub fn gasto_inicial(peso: f64, altura: f64, edad: f64, sexo: Sexo, pasos: i64) -> i64 {
    let basal = mifflin(peso, altura, edad, sexo);
    let factor = if pasos >= 12_000 {
        1.55
    } else if pasos >= 8_000 {
        1.42
    } else if pasos >= 5_000 {
        1.32
    } else {
        1.22
    };
    ((basal * factor).round() as i64).clamp(1200, 4500)
}

/// Metabolismo basal de Mifflin-St Jeor.
#[must_use]
pub fn mifflin(peso: f64, altura: f64, edad: f64, sexo: Sexo) -> f64 {
    10.0 * peso + 6.25 * altura - 5.0 * edad + if sexo.es_hombre() { 5.0 } else { -161.0 }
}

/// Basal de referencia del perfil, acotado como en el teléfono.
#[must_use]
pub fn basal_estimado(peso: f64, altura: f64, edad: f64, sexo: Sexo) -> i64 {
    (mifflin(peso, altura, edad, sexo).round() as i64).clamp(1000, 3200)
}

/// Ajuste diario sobre el gasto según la fase de la dieta.
#[must_use]
pub fn ajuste_por_fase(
    fase: FaseDieta,
    ritmo_semanal_kg: f64,
    dias_en_la_fase: Option<i64>,
    antecedentes_tca: bool,
) -> i64 {
    let suelo = if antecedentes_tca { 275 } else { 1100 };
    let previsto =
        ((ritmo_semanal_kg * KCAL_POR_KG_GRASA / 7.0).round() as i64).clamp(-suelo, suelo);
    match fase {
        FaseDieta::Deficit => previsto,
        FaseDieta::Mantenimiento | FaseDieta::Descanso => 0,
        FaseDieta::Inversa => {
            let inicio = previsto.min(0);
            let semanas = dias_en_la_fase.unwrap_or(0) / 7;
            (inicio + 75 * semanas).min(0)
        }
    }
}

/// Las metas de un día.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Metas {
    pub gasto_base: i64,
    pub ajuste: i64,
    pub ejercicio: i64,
    pub kcal: i64,
    pub proteina_g: f64,
    pub carbohidratos_g: f64,
    pub grasa_g: f64,
    pub fibra_g: f64,
    pub sodio_mg: f64,
    pub potasio_mg: f64,
    pub magnesio_mg: f64,
}

/// Calorías y objetivos opcionales de proteína, carbohidratos y grasa.
pub type MacrosManuales = (i64, Option<i64>, Option<i64>, Option<i64>);

/// Lo que hace falta del perfil para calcular las metas.
#[derive(Debug, Clone)]
pub struct DatosMetas {
    pub sexo: Sexo,
    pub edad: f64,
    pub altura_cm: f64,
    pub peso_meta_kg: f64,
    pub peso_actual_kg: Option<f64>,
    pub gasto_adaptativo: i64,
    pub fase: FaseDieta,
    pub ritmo_semanal_kg: f64,
    pub dias_en_la_fase: Option<i64>,
    pub antecedentes_tca: bool,
    pub arquetipo: Arquetipo,
    /// Metas a mano (kcal, proteína, carbohidratos, grasa), si están activadas.
    pub manual: Option<MacrosManuales>,
}

/// Las metas del día: gasto adaptativo, ajuste de la fase y ejercicio neto, o
/// las metas a mano si la persona las fijó.
#[must_use]
pub fn metas(d: &DatosMetas, ejercicio: i64) -> Metas {
    let hombre = d.sexo.es_hombre();
    let magnesio = if hombre { 420.0 } else { 320.0 };
    if let Some((kcal, p, c, g)) = d.manual {
        let fibra = acotar(kcal as f64 / 1000.0 * 14.0, 20.0, 45.0);
        return Metas {
            gasto_base: kcal,
            ajuste: 0,
            ejercicio,
            kcal: kcal + ejercicio,
            proteina_g: p.map_or(d.peso_meta_kg * 2.0, |v| v as f64),
            carbohidratos_g: c.map_or(150.0, |v| v as f64),
            grasa_g: g.map_or(50.0, |v| v as f64),
            fibra_g: fijo(fibra, 0),
            sodio_mg: 2300.0,
            potasio_mg: 3500.0,
            magnesio_mg: magnesio,
        };
    }
    let peso = d.peso_actual_kg.unwrap_or(d.peso_meta_kg);
    let basal_seguro = acotar(mifflin(peso, d.altura_cm, d.edad, d.sexo), 1100.0, 2600.0);
    let ajuste = ajuste_por_fase(
        d.fase,
        d.ritmo_semanal_kg,
        d.dias_en_la_fase,
        d.antecedentes_tca,
    );
    let kcal = (basal_seguro.round() as i64).max(d.gasto_adaptativo + ajuste + ejercicio);
    let (p, c, g) = reparto(d.arquetipo, d.peso_meta_kg, kcal);
    let fibra = acotar(kcal as f64 / 1000.0 * 14.0, 22.0, 45.0);
    Metas {
        gasto_base: d.gasto_adaptativo,
        ajuste,
        ejercicio,
        kcal,
        proteina_g: p.round(),
        carbohidratos_g: c.round(),
        grasa_g: g.round(),
        fibra_g: fibra.round(),
        sodio_mg: 2300.0,
        potasio_mg: 3500.0,
        magnesio_mg: magnesio,
    }
}

fn reparto(arquetipo: Arquetipo, peso_meta: f64, kcal: i64) -> (f64, f64, f64) {
    let kcal = kcal as f64;
    if arquetipo != Arquetipo::Flexible {
        let (p, c, g) = arquetipo.reparto();
        return (
            kcal * p / 100.0 / 4.0,
            kcal * c / 100.0 / 4.0,
            kcal * g / 100.0 / 9.0,
        );
    }
    let proteina = acotar(peso_meta * 2.0, 60.0, 230.0);
    let grasa = acotar(kcal * 0.27 / 9.0, 40.0, 130.0);
    let resto = (kcal - proteina * 4.0 - grasa * 9.0).max(280.0);
    let carbohidratos = acotar(resto / 4.0, 70.0, 480.0);
    (proteina, carbohidratos, grasa)
}

/// Gasto activo por pasos: 0,04 kcal por paso.
#[must_use]
pub fn kcal_de_pasos(pasos: i64) -> i64 {
    (pasos.clamp(0, 200_000) as f64 * 0.04).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_imc_lleva_dos_decimales() {
        assert_eq!(imc(80.0, 180.0), 24.69);
        assert_eq!(imc(80.0, 0.0), 0.0);
    }

    #[test]
    fn la_tendencia_avanza_un_decimo_hacia_el_peso_nuevo() {
        assert_eq!(tendencia(80.0, None), 80.0);
        assert_eq!(tendencia(81.0, Some(80.0)), 80.1);
        assert_eq!(tendencia(79.0, Some(80.0)), 79.9);
    }

    /// Valores calculados con `BiometricFormulas.deriveAllMetricsEstimated` del
    /// teléfono para el mismo pesaje.
    #[test]
    fn las_metricas_estimadas_coinciden_con_el_telefono() {
        let m = metricas_estimadas(80.0, 180.0, 35.0, Sexo::Hombre);
        assert_eq!(m.imc, 24.69);
        assert_eq!(m.grasa_pct, 21.5);
        assert_eq!(m.somatotipo, "balanced_muscular");
        assert_eq!(m.metabolismo_basal, katch_mcardle(m.masa_magra_kg));
    }

    #[test]
    fn la_marina_da_valores_razonables() {
        let h = grasa_marina(180.0, 85.0, 38.0, Sexo::Hombre, None);
        assert!((15.0..20.0).contains(&h), "{h}");
        let m = grasa_marina(165.0, 72.0, 32.0, Sexo::Mujer, Some(98.0));
        assert!((25.0..35.0).contains(&m), "{m}");
        assert_eq!(grasa_marina(180.0, 30.0, 38.0, Sexo::Hombre, None), 10.0);
    }

    #[test]
    fn la_calibracion_no_se_mueve_mas_de_300_ni_baja_del_basal() {
        // Bajar 1 kg comiendo 2000 explica 550 kcal más de gasto: 2550, que se
        // queda en 2500 porque el anterior era 2200.
        let (g, bruto, acotado) = calibrar_gasto(2000.0, 80.0, 79.0, 2200, 1700);
        assert!(acotado);
        assert_eq!((g, bruto), (2500, 2550));
        let (g, _, acotado) = calibrar_gasto(2000.0, 80.0, 79.0, 2400, 1700);
        assert!(!acotado);
        assert_eq!(g, 2550);
        let (g, bruto, acotado) = calibrar_gasto(1000.0, 80.0, 80.0, 2200, 1700);
        assert!(acotado);
        assert_eq!((g, bruto), (1900, 1000));
        let (g, _, _) = calibrar_gasto(1000.0, 80.0, 80.0, 1800, 1700);
        assert_eq!(g, 1700, "nunca por debajo del basal");
    }

    #[test]
    fn el_deficit_se_acota_y_con_antecedentes_mucho_mas() {
        assert_eq!(ajuste_por_fase(FaseDieta::Deficit, -0.5, None, false), -550);
        assert_eq!(
            ajuste_por_fase(FaseDieta::Deficit, -2.0, None, false),
            -1100
        );
        assert_eq!(ajuste_por_fase(FaseDieta::Deficit, -0.5, None, true), -275);
        assert_eq!(ajuste_por_fase(FaseDieta::Descanso, -0.5, None, false), 0);
    }

    #[test]
    fn la_dieta_inversa_sube_75_por_semana_hasta_el_mantenimiento() {
        assert_eq!(
            ajuste_por_fase(FaseDieta::Inversa, -0.5, Some(0), false),
            -550
        );
        assert_eq!(
            ajuste_por_fase(FaseDieta::Inversa, -0.5, Some(14), false),
            -400
        );
        assert_eq!(
            ajuste_por_fase(FaseDieta::Inversa, -0.5, Some(400), false),
            0
        );
    }

    fn datos() -> DatosMetas {
        DatosMetas {
            sexo: Sexo::Hombre,
            edad: 35.0,
            altura_cm: 180.0,
            peso_meta_kg: 75.0,
            peso_actual_kg: Some(80.0),
            gasto_adaptativo: 2500,
            fase: FaseDieta::Deficit,
            ritmo_semanal_kg: -0.5,
            dias_en_la_fase: Some(3),
            antecedentes_tca: false,
            arquetipo: Arquetipo::Flexible,
            manual: None,
        }
    }

    #[test]
    fn las_metas_suman_gasto_ajuste_y_ejercicio() {
        let m = metas(&datos(), 200);
        assert_eq!(m.kcal, 2500 - 550 + 200);
        assert_eq!(m.proteina_g, 150.0);
        assert_eq!(m.grasa_g, (2150.0f64 * 0.27 / 9.0).round());
    }

    #[test]
    fn las_metas_nunca_bajan_del_basal_seguro() {
        let mut d = datos();
        d.gasto_adaptativo = 1300;
        let m = metas(&d, 0);
        assert_eq!(
            m.kcal,
            mifflin(80.0, 180.0, 35.0, Sexo::Hombre).round() as i64
        );
    }

    #[test]
    fn las_metas_a_mano_mandan() {
        let mut d = datos();
        d.manual = Some((1800, Some(140), None, None));
        let m = metas(&d, 100);
        assert_eq!(
            (m.kcal, m.proteina_g, m.carbohidratos_g),
            (1900, 140.0, 150.0)
        );
    }

    #[test]
    fn un_arquetipo_reparte_por_porcentajes() {
        let mut d = datos();
        d.arquetipo = Arquetipo::Keto;
        let m = metas(&d, 0);
        assert_eq!(m.carbohidratos_g, (1950.0f64 * 0.05 / 4.0).round());
    }

    #[test]
    fn el_gasto_inicial_sigue_la_meta_de_pasos() {
        let poco = gasto_inicial(80.0, 180.0, 35.0, Sexo::Hombre, 3000);
        let mucho = gasto_inicial(80.0, 180.0, 35.0, Sexo::Hombre, 12000);
        assert!(mucho > poco);
    }
}
