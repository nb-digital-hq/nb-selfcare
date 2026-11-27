//! El analizador de texto determinista (ADR-009): de una frase a alimentos con
//! sus gramos, y de una entrada del diario a ánimo, energía y estrés.
//!
//! Es el mismo vocabulario y las mismas reglas que NB SelfCare Mobile
//! (`text_assistant.dart`), en español, inglés y portugués. No hay modelo de
//! lenguaje: lo que no entiende lo deja como está, y la persona lo corrige.

use regex::{Regex, RegexBuilder};
use serde::Serialize;
use ts_rs::TS;

use crate::idioma::Idioma;

/// Un alimento leído de la frase, con macros de respaldo por 100 g si no está
/// en el catálogo.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Leido {
    pub termino: String,
    pub gramos: f64,
    pub kcal: f64,
    pub proteina: f64,
    pub carbohidratos: f64,
    pub grasa: f64,
}

struct Pieza {
    raices: &'static [&'static str],
    gramos: f64,
    nombre: &'static str,
}

struct Categoria {
    raices: &'static [&'static str],
    v: (f64, f64, f64, f64),
}

struct Lexico {
    conectores: &'static [&'static str],
    con: &'static str,
    modificadores: &'static [&'static str],
    unidades: &'static [&'static str],
    de: &'static [&'static str],
    articulos: &'static [&'static str],
    piezas: &'static [Pieza],
    categorias: &'static [Categoria],
    positivas: &'static [&'static str],
    negativas: &'static [&'static str],
    energia_alta: &'static [&'static str],
    energia_baja: &'static [&'static str],
    estres: &'static [&'static str],
    calma: &'static [&'static str],
}

const ES: Lexico = Lexico {
    conectores: &["y", "e"],
    con: "con",
    modificadores: &[
        "piel", "cáscara", "cascara", "hueso", "sal", "azúcar", "azucar", "agua",
    ],
    unidades: &["g", "gr", "gramos", "ml", "mililitros"],
    de: &["de"],
    articulos: &["un", "una", "unos", "unas", "el", "la", "los", "las"],
    piezas: &[
        Pieza {
            raices: &["huevo"],
            gramos: 55.0,
            nombre: "huevo cocido",
        },
        Pieza {
            raices: &["platano", "plátano", "banano"],
            gramos: 120.0,
            nombre: "platano banano",
        },
        Pieza {
            raices: &["manzana"],
            gramos: 150.0,
            nombre: "manzana fresca",
        },
        Pieza {
            raices: &["naranja"],
            gramos: 130.0,
            nombre: "naranja",
        },
        Pieza {
            raices: &["rebanada", "pan"],
            gramos: 35.0,
            nombre: "pan integral",
        },
        Pieza {
            raices: &["arepa"],
            gramos: 100.0,
            nombre: "arepa",
        },
    ],
    categorias: &[
        Categoria {
            raices: &["pollo", "pavo"],
            v: (165.0, 31.0, 0.0, 3.6),
        },
        Categoria {
            raices: &["carne", "res", "lomo"],
            v: (210.0, 26.0, 0.0, 11.0),
        },
        Categoria {
            raices: &["huevo"],
            v: (155.0, 13.0, 1.1, 11.0),
        },
        Categoria {
            raices: &["arroz"],
            v: (130.0, 2.7, 28.0, 0.3),
        },
        Categoria {
            raices: &["avena"],
            v: (375.0, 13.5, 68.0, 7.0),
        },
        Categoria {
            raices: &["atun", "atún", "salmon", "salmón", "pescado"],
            v: (180.0, 24.0, 0.0, 9.0),
        },
        Categoria {
            raices: &["aguacate"],
            v: (160.0, 2.0, 8.5, 14.7),
        },
        Categoria {
            raices: &["yogur", "leche"],
            v: (65.0, 4.5, 5.0, 3.2),
        },
    ],
    positivas: &[
        "feliz",
        "contento",
        "tranquilo",
        "paz",
        "motivado",
        "alegría",
        "logro",
        "orgullo",
        "bien",
        "excelente",
    ],
    negativas: &[
        "triste",
        "desanimado",
        "mal",
        "enojado",
        "frustrado",
        "tristeza",
        "difícil",
        "duda",
    ],
    energia_alta: &[
        "energía",
        "fuerza",
        "vitalidad",
        "activo",
        "ágil",
        "imparable",
        "entrené",
    ],
    energia_baja: &[
        "cansado",
        "agotado",
        "fatiga",
        "pesado",
        "sueño",
        "exhausto",
        "sin fuerza",
    ],
    estres: &[
        "estrés",
        "estresado",
        "agobio",
        "presión",
        "tensión",
        "urgente",
        "ansiedad",
        "sobrecarga",
    ],
    calma: &["relajado", "descanso", "tranquilidad"],
};

const EN: Lexico = Lexico {
    conectores: &["and"],
    con: "with",
    modificadores: &["skin", "peel", "bone", "salt", "sugar", "water"],
    unidades: &["g", "gr", "grams", "gram", "ml", "milliliters"],
    de: &["of"],
    articulos: &["a", "an", "the", "some"],
    piezas: &[
        Pieza {
            raices: &["egg"],
            gramos: 55.0,
            nombre: "boiled egg",
        },
        Pieza {
            raices: &["banana"],
            gramos: 120.0,
            nombre: "banana",
        },
        Pieza {
            raices: &["apple"],
            gramos: 150.0,
            nombre: "apple",
        },
        Pieza {
            raices: &["orange"],
            gramos: 130.0,
            nombre: "orange",
        },
        Pieza {
            raices: &["slice", "bread", "toast"],
            gramos: 35.0,
            nombre: "whole wheat bread",
        },
    ],
    categorias: &[
        Categoria {
            raices: &["chicken", "turkey"],
            v: (165.0, 31.0, 0.0, 3.6),
        },
        Categoria {
            raices: &["beef", "steak", "meat", "pork"],
            v: (210.0, 26.0, 0.0, 11.0),
        },
        Categoria {
            raices: &["egg"],
            v: (155.0, 13.0, 1.1, 11.0),
        },
        Categoria {
            raices: &["rice"],
            v: (130.0, 2.7, 28.0, 0.3),
        },
        Categoria {
            raices: &["oat"],
            v: (375.0, 13.5, 68.0, 7.0),
        },
        Categoria {
            raices: &["tuna", "salmon", "fish"],
            v: (180.0, 24.0, 0.0, 9.0),
        },
        Categoria {
            raices: &["avocado"],
            v: (160.0, 2.0, 8.5, 14.7),
        },
        Categoria {
            raices: &["yogurt", "yoghurt", "milk"],
            v: (65.0, 4.5, 5.0, 3.2),
        },
    ],
    positivas: &[
        "happy",
        "glad",
        "calm",
        "peace",
        "motivated",
        "joy",
        "proud",
        "good",
        "great",
        "excellent",
    ],
    negativas: &[
        "sad",
        "down",
        "bad",
        "angry",
        "frustrated",
        "sadness",
        "hard",
        "difficult",
        "doubt",
    ],
    energia_alta: &[
        "energy",
        "strong",
        "vitality",
        "active",
        "unstoppable",
        "trained",
        "workout",
    ],
    energia_baja: &[
        "tired",
        "exhausted",
        "fatigue",
        "heavy",
        "sleepy",
        "drained",
    ],
    estres: &[
        "stress",
        "stressed",
        "overwhelmed",
        "pressure",
        "tension",
        "urgent",
        "anxiety",
        "overload",
    ],
    calma: &["relaxed", "rest", "rested", "tranquil"],
};

const PT: Lexico = Lexico {
    conectores: &["e"],
    con: "com",
    modificadores: &[
        "pele", "casca", "osso", "sal", "açúcar", "acucar", "água", "agua",
    ],
    unidades: &["g", "gr", "gramas", "ml", "mililitros"],
    de: &["de", "do", "da"],
    articulos: &["um", "uma", "uns", "umas", "o", "a", "os", "as"],
    piezas: &[
        Pieza {
            raices: &["ovo"],
            gramos: 55.0,
            nombre: "ovo cozido",
        },
        Pieza {
            raices: &["banana"],
            gramos: 120.0,
            nombre: "banana",
        },
        Pieza {
            raices: &["maçã", "maca"],
            gramos: 150.0,
            nombre: "maçã",
        },
        Pieza {
            raices: &["laranja"],
            gramos: 130.0,
            nombre: "laranja",
        },
        Pieza {
            raices: &["fatia", "pão", "pao"],
            gramos: 35.0,
            nombre: "pão integral",
        },
    ],
    categorias: &[
        Categoria {
            raices: &["frango", "peru"],
            v: (165.0, 31.0, 0.0, 3.6),
        },
        Categoria {
            raices: &["carne", "bife", "lombo"],
            v: (210.0, 26.0, 0.0, 11.0),
        },
        Categoria {
            raices: &["ovo"],
            v: (155.0, 13.0, 1.1, 11.0),
        },
        Categoria {
            raices: &["arroz"],
            v: (130.0, 2.7, 28.0, 0.3),
        },
        Categoria {
            raices: &["aveia"],
            v: (375.0, 13.5, 68.0, 7.0),
        },
        Categoria {
            raices: &["atum", "salmão", "salmao", "peixe"],
            v: (180.0, 24.0, 0.0, 9.0),
        },
        Categoria {
            raices: &["abacate"],
            v: (160.0, 2.0, 8.5, 14.7),
        },
        Categoria {
            raices: &["iogurte", "leite"],
            v: (65.0, 4.5, 5.0, 3.2),
        },
    ],
    positivas: &[
        "feliz",
        "contente",
        "tranquilo",
        "paz",
        "motivado",
        "alegria",
        "conquista",
        "orgulho",
        "bem",
        "ótimo",
        "excelente",
    ],
    negativas: &[
        "triste",
        "desanimado",
        "mal",
        "irritado",
        "frustrado",
        "tristeza",
        "difícil",
        "dúvida",
    ],
    energia_alta: &[
        "energia",
        "força",
        "vitalidade",
        "ativo",
        "ágil",
        "imparável",
        "treinei",
    ],
    energia_baja: &[
        "cansado",
        "esgotado",
        "fadiga",
        "pesado",
        "sono",
        "exausto",
        "sem força",
    ],
    estres: &[
        "estresse",
        "stress",
        "estressado",
        "sobrecarregado",
        "pressão",
        "tensão",
        "urgente",
        "ansiedade",
    ],
    calma: &["relaxado", "descanso", "tranquilidade"],
};

fn lexico(i: Idioma) -> &'static Lexico {
    match i {
        Idioma::Es => &ES,
        Idioma::En => &EN,
        Idioma::Pt => &PT,
    }
}

fn alternativa(palabras: &[&str]) -> String {
    palabras
        .iter()
        .map(|p| regex::escape(p))
        .collect::<Vec<_>>()
        .join("|")
}

fn re(patron: &str) -> Regex {
    RegexBuilder::new(patron)
        .case_insensitive(true)
        .build()
        .unwrap_or_else(|_| Regex::new("$^").unwrap_or_else(|_| unreachable!()))
}

/// Lee los alimentos de una frase ("200 g de pollo y una manzana").
#[must_use]
pub fn leer_comida(texto: &str, idioma: Idioma) -> Vec<Leido> {
    let texto = texto.trim();
    if texto.is_empty() {
        return Vec::new();
    }
    let lx = lexico(idioma);
    // Separa por conectores y por "con", salvo cuando "con" introduce un
    // modificador culinario ("con piel") que es parte del alimento. El motor de
    // `regex` no tiene búsqueda hacia delante: se protege el modificador antes.
    let sin_conectores = re(&format!(r"\s+(?:{})\s+", alternativa(lx.conectores)))
        .replace_all(texto, ", ")
        .into_owned();
    let modificador = re(&format!(
        r"\s+{}\s+((?:{})\b)",
        regex::escape(lx.con),
        alternativa(lx.modificadores)
    ));
    let protegido = modificador
        .replace_all(&sin_conectores, "\u{1}$1")
        .into_owned();
    let separado = re(&format!(r"\s+{}\s+", regex::escape(lx.con)))
        .replace_all(&protegido, ", ")
        .replace('\u{1}', &format!(" {} ", lx.con))
        .replace('\n', ", ");

    let gramos_re = re(&format!(
        r"(\d+(?:[\.,]\d+)?)\s*(?:{})\s+(?:(?:{})\s+)?(.+)",
        alternativa(lx.unidades),
        alternativa(lx.de)
    ));
    let cantidad_re = re(r"(\d+)\s*(?:x\s*)?([^\d\s]\S*)(?:\s+(.*))?");
    let articulo = re(&format!(r"^(?:{})\s+", alternativa(lx.articulos)));
    let limpiar = Regex::new(r"[^\w\sáéíóúüñãõâêôçàÁÉÍÓÚÜÑÃÕÂÊÔÇÀ]").ok();

    let mut v = Vec::new();
    for crudo in separado.split(',') {
        let seg = crudo.trim();
        if seg.is_empty() {
            continue;
        }
        let mut gramos = 100.0;
        let mut nombre = seg.to_owned();
        if let Some(m) = gramos_re.captures(seg) {
            gramos = m[1].replace(',', ".").parse().unwrap_or(100.0);
            nombre = m[2].trim().to_owned();
        } else if let Some(m) = cantidad_re.captures(seg) {
            let n: f64 = m[1].parse().unwrap_or(1.0);
            let sustantivo = m[2].to_lowercase();
            if let Some(p) = lx
                .piezas
                .iter()
                .find(|p| p.raices.iter().any(|r| sustantivo.starts_with(r)))
            {
                gramos = n * p.gramos;
                nombre = p.nombre.to_owned();
            } else {
                gramos = n * 100.0;
                nombre = match m.get(3) {
                    Some(resto) => format!("{sustantivo} {}", resto.as_str()),
                    None => sustantivo,
                };
            }
        }
        if !gramos.is_finite() || gramos <= 0.0 {
            continue;
        }
        let gramos = gramos.clamp(1.0, 5000.0);
        let mut nombre = articulo.replace(&nombre, "").into_owned();
        if let Some(l) = &limpiar {
            nombre = l.replace_all(&nombre, "").into_owned();
        }
        let nombre = nombre.trim().to_owned();
        if nombre.is_empty() {
            continue;
        }
        let bajo = nombre.to_lowercase();
        let (kcal, proteina, carbohidratos, grasa) = lx
            .categorias
            .iter()
            .find(|c| c.raices.iter().any(|r| bajo.contains(r)))
            .map_or((130.0, 4.0, 20.0, 3.0), |c| c.v);
        v.push(Leido {
            termino: nombre,
            gramos,
            kcal,
            proteina,
            carbohidratos,
            grasa,
        });
    }
    v
}

/// Lo que se sugiere para una entrada del diario.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct Sugerencia {
    pub animo: i64,
    pub energia: i64,
    pub estres: i64,
    /// Códigos estables (`calma_interior`…); la interfaz los traduce.
    pub etiquetas: Vec<String>,
    /// `great`, `stress`, `tired`, `default` o `empty`.
    pub lectura: String,
}

/// Sugiere ánimo, energía, estrés y etiquetas para una entrada del diario.
#[must_use]
pub fn leer_diario(texto: &str, idioma: Idioma) -> Sugerencia {
    let texto = texto.trim();
    if texto.is_empty() {
        return Sugerencia {
            animo: 3,
            energia: 3,
            estres: 2,
            etiquetas: vec!["calma".into()],
            lectura: "empty".into(),
        };
    }
    let lx = lexico(idioma);
    let bajo = texto.to_lowercase();
    let contiene = |p: &&&str| {
        Regex::new(&format!(r"(^|[^\p{{L}}]){}", regex::escape(p))).is_ok_and(|r| r.is_match(&bajo))
    };
    let (mut animo, mut energia, mut estres) = (3i64, 3i64, 2i64);
    let mut etiquetas = Vec::new();
    let pos = lx.positivas.iter().filter(contiene).count() as i64;
    let neg = lx.negativas.iter().filter(contiene).count() as i64;
    if pos > neg {
        animo = (3 + pos).clamp(1, 5);
        etiquetas.push("calma_interior");
    } else if neg > pos {
        animo = (3 - neg).clamp(1, 5);
        etiquetas.push("gestion_emocional");
    }
    if lx.energia_alta.iter().any(|p| contiene(&p)) {
        energia = 5;
        etiquetas.push("alta_energia");
    } else if lx.energia_baja.iter().any(|p| contiene(&p)) {
        energia = 2;
        etiquetas.push("recuperacion_fisica");
    }
    if lx.estres.iter().any(|p| contiene(&p)) {
        estres = 4;
        etiquetas.push("sobrecarga_mental");
    } else if lx.calma.iter().any(|p| contiene(&p)) {
        estres = 1;
        etiquetas.push("descanso_reparador");
    }
    if etiquetas.is_empty() {
        etiquetas.push("reflexion_personal");
    }
    let lectura = if animo >= 4 && energia >= 4 {
        "great"
    } else if estres >= 4 {
        "stress"
    } else if energia <= 2 {
        "tired"
    } else {
        "default"
    };
    Sugerencia {
        animo,
        energia,
        estres,
        etiquetas: etiquetas.into_iter().take(3).map(str::to_owned).collect(),
        lectura: lectura.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terminos(v: &[Leido]) -> Vec<(&str, f64)> {
        v.iter().map(|l| (l.termino.as_str(), l.gramos)).collect()
    }

    #[test]
    fn lee_gramos_piezas_y_conectores() {
        let v = leer_comida("200 g de pollo y 2 huevos con una manzana", Idioma::Es);
        assert_eq!(
            terminos(&v),
            vec![
                ("pollo", 200.0),
                ("huevo cocido", 110.0),
                ("manzana", 100.0)
            ]
        );
        assert_eq!(v[0].kcal, 165.0, "el pollo trae su categoría de respaldo");
    }

    #[test]
    fn con_piel_es_parte_del_alimento() {
        let v = leer_comida("150 g de papa con piel", Idioma::Es);
        assert_eq!(terminos(&v), vec![("papa con piel", 150.0)]);
    }

    #[test]
    fn entiende_ingles_y_portugues() {
        let en = leer_comida("2 eggs and 150 g of rice", Idioma::En);
        assert_eq!(terminos(&en), vec![("boiled egg", 110.0), ("rice", 150.0)]);
        let pt = leer_comida("100 g de frango e 1 banana", Idioma::Pt);
        assert_eq!(terminos(&pt), vec![("frango", 100.0), ("banana", 120.0)]);
    }

    #[test]
    fn cero_gramos_no_es_una_comida_y_lo_enorme_se_acota() {
        assert!(leer_comida("0 g de arroz", Idioma::Es).is_empty());
        assert_eq!(
            leer_comida("90000 g de arroz", Idioma::Es)[0].gramos,
            5000.0
        );
        assert!(leer_comida("   ", Idioma::Es).is_empty());
    }

    #[test]
    fn el_diario_sugiere_sin_juzgar() {
        let s = leer_diario("Hoy me sentí feliz y con mucha energía", Idioma::Es);
        assert_eq!((s.animo, s.energia, s.lectura.as_str()), (4, 5, "great"));
        let s = leer_diario("Mucho estrés en el trabajo, estoy cansado", Idioma::Es);
        assert_eq!((s.estres, s.energia), (4, 2));
        assert_eq!(leer_diario("", Idioma::Es).lectura, "empty");
    }

    #[test]
    fn una_palabra_dentro_de_otra_no_cuenta() {
        // "mal" no está en "animal": la palabra tiene que empezar ahí.
        assert_eq!(leer_diario("vi un animal", Idioma::Es).animo, 3);
    }
}
