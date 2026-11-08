//! BI-RADS 5ª edição (ACR, 2013), ultrassom de mama. A categoria final é do médico; o servidor
//! confere a coerência com os descritores e devolve a conduta e a probabilidade de malignidade da
//! tabela.
//!
//! Versão `BIRADS-ACR-2013-US.1`. Regras de coerência (a confirmar, DEC-2026-11-08-60):
//! - 6 exige malignidade confirmada por biópsia;
//! - 1 (negativo) não admite massa descrita;
//! - 2, 3, 4A, 4B, 4C, 5 e 6 exigem a massa descrita e o maior diâmetro; 0 admite sem massa;
//! - massa com característica suspeita (forma irregular, orientação não paralela ou margem não
//!   circunscrita) não pode ser 2 nem 3;
//! - 5 exige ao menos uma característica suspeita;
//! - margem circunscrita não combina com margem não circunscrita.
//!
//! Os casos dos testes aguardam conferência por médico.

use crate::{minuscula_inicial, ErroClassificacao, Medida};
use serde::{Deserialize, Serialize};

pub const VERSAO: &str = "BIRADS-ACR-2013-US.1";
/// Maior massa de mama aceita (mm).
pub const LIMITE_MM: u32 = 150;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lado {
    Direita,
    Esquerda,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Forma {
    Oval,
    Redonda,
    Irregular,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientacao {
    Paralela,
    NaoParalela,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Margem {
    Circunscrita,
    Indistinta,
    Angular,
    Microlobulada,
    Espiculada,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PadraoEcogenico {
    Anecoico,
    Hiperecoico,
    ComplexoCisticoESolido,
    Hipoecoico,
    Isoecoico,
    Heterogeneo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Posterior {
    Nenhum,
    Reforco,
    Sombra,
    Combinado,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Massa {
    pub forma: Forma,
    pub orientacao: Orientacao,
    pub margens: Vec<Margem>,
    pub padrao_ecogenico: PadraoEcogenico,
    pub posterior: Posterior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Categoria {
    #[serde(rename = "0")]
    Zero,
    #[serde(rename = "1")]
    Um,
    #[serde(rename = "2")]
    Dois,
    #[serde(rename = "3")]
    Tres,
    #[serde(rename = "4A")]
    QuatroA,
    #[serde(rename = "4B")]
    QuatroB,
    #[serde(rename = "4C")]
    QuatroC,
    #[serde(rename = "5")]
    Cinco,
    #[serde(rename = "6")]
    Seis,
}

impl Categoria {
    #[must_use]
    pub const fn sigla(self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::Um => "1",
            Self::Dois => "2",
            Self::Tres => "3",
            Self::QuatroA => "4A",
            Self::QuatroB => "4B",
            Self::QuatroC => "4C",
            Self::Cinco => "5",
            Self::Seis => "6",
        }
    }

    #[must_use]
    pub const fn nome(self) -> &'static str {
        match self {
            Self::Zero => "incompleto, precisa de avaliação adicional de imagem",
            Self::Um => "negativo",
            Self::Dois => "achado benigno",
            Self::Tres => "provavelmente benigno",
            Self::QuatroA => "baixa suspeita de malignidade",
            Self::QuatroB => "moderada suspeita de malignidade",
            Self::QuatroC => "alta suspeita de malignidade",
            Self::Cinco => "altamente sugestivo de malignidade",
            Self::Seis => "malignidade comprovada por biópsia",
        }
    }

    /// Conduta da tabela.
    #[must_use]
    pub const fn conduta(self) -> &'static str {
        match self {
            Self::Zero => "avaliação adicional de imagem ou comparação com exames anteriores",
            Self::Um | Self::Dois => "rastreamento de rotina",
            Self::Tres => "seguimento de curto intervalo (6 meses) ou vigilância",
            Self::QuatroA | Self::QuatroB | Self::QuatroC | Self::Cinco => {
                "diagnóstico tecidual (biópsia)"
            }
            Self::Seis => "excisão cirúrgica quando clinicamente apropriada",
        }
    }

    /// Probabilidade de malignidade da tabela.
    #[must_use]
    pub const fn probabilidade(self) -> &'static str {
        match self {
            Self::Zero => "não se aplica",
            Self::Um | Self::Dois => "essencialmente 0%",
            Self::Tres => "acima de 0% e até 2%",
            Self::QuatroA => "acima de 2% e até 10%",
            Self::QuatroB => "acima de 10% e até 50%",
            Self::QuatroC => "acima de 50% e abaixo de 95%",
            Self::Cinco => "95% ou mais",
            Self::Seis => "não se aplica",
        }
    }
}

/// O achado como o médico registrou.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Achado {
    pub lado: Lado,
    pub categoria_atribuida: Categoria,
    pub biopsia_maligna_confirmada: bool,
    pub massa: Option<Massa>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resultado {
    pub categoria: Categoria,
    pub suspeitas: Vec<&'static str>,
    pub versao: &'static str,
}

/// Características suspeitas da massa, na ordem do léxico.
#[must_use]
pub fn suspeitas(m: &Massa) -> Vec<&'static str> {
    let mut s = Vec::new();
    if m.forma == Forma::Irregular {
        s.push("forma irregular");
    }
    if m.orientacao == Orientacao::NaoParalela {
        s.push("orientação não paralela");
    }
    if m.margens.iter().any(|g| *g != Margem::Circunscrita) {
        s.push("margem não circunscrita");
    }
    s
}

/// Confere a coerência e devolve o achado normalizado (margens sem repetição e em ordem).
///
/// # Errors
/// Regras da versão (ver o cabeçalho do módulo).
pub fn classificar(
    a: &Achado,
    maior: Option<Medida>,
) -> Result<(Achado, Resultado), ErroClassificacao> {
    use Categoria as C;
    let cat = a.categoria_atribuida;
    if cat == C::Seis && !a.biopsia_maligna_confirmada {
        return Err(ErroClassificacao::Birads6SemBiopsia);
    }
    let massa = match &a.massa {
        Some(m) => {
            let mut margens = m.margens.clone();
            margens.sort();
            margens.dedup();
            if margens.is_empty() {
                return Err(ErroClassificacao::MargemAusente);
            }
            if margens.len() > 1 && margens.contains(&Margem::Circunscrita) {
                return Err(ErroClassificacao::MargensIncoerentes);
            }
            Some(Massa {
                margens,
                ..m.clone()
            })
        }
        None => None,
    };
    if cat == C::Um && massa.is_some() {
        return Err(ErroClassificacao::NegativoComMassa);
    }
    if massa.is_none() && maior.is_some() {
        return Err(ErroClassificacao::MedidaSemMassa);
    }
    if !matches!(cat, C::Zero | C::Um) && (massa.is_none() || maior.is_none()) {
        return Err(ErroClassificacao::MassaAusente {
            categoria: cat.sigla(),
        });
    }
    let sus = massa.as_ref().map(suspeitas).unwrap_or_default();
    if massa.is_some() {
        if matches!(cat, C::Dois | C::Tres) && !sus.is_empty() {
            return Err(ErroClassificacao::CategoriaBaixaComSuspeita {
                categoria: cat.sigla(),
                suspeitas: sus.join(", "),
            });
        }
        if cat == C::Cinco && sus.is_empty() {
            return Err(ErroClassificacao::Categoria5SemSuspeita);
        }
    }
    Ok((
        Achado { massa, ..a.clone() },
        Resultado {
            categoria: cat,
            suspeitas: sus,
            versao: VERSAO,
        },
    ))
}

/// Frase do achado para o texto do laudo (o médico revisa e edita).
#[must_use]
pub fn texto_achado(
    rotulo: &str,
    local: &str,
    maior: Option<Medida>,
    a: &Achado,
    r: &Resultado,
) -> String {
    let lado = match a.lado {
        Lado::Direita => "mama direita",
        Lado::Esquerda => "mama esquerda",
    };
    let descricao = match (&a.massa, maior) {
        (Some(m), Some(maior)) => {
            let forma = match m.forma {
                Forma::Oval => "oval",
                Forma::Redonda => "redonda",
                Forma::Irregular => "irregular",
            };
            let orientacao = match m.orientacao {
                Orientacao::Paralela => "paralela à pele",
                Orientacao::NaoParalela => "não paralela à pele",
            };
            let margens: Vec<&str> = m
                .margens
                .iter()
                .map(|g| match g {
                    Margem::Circunscrita => "circunscritas",
                    Margem::Indistinta => "indistintas",
                    Margem::Angular => "angulares",
                    Margem::Microlobulada => "microlobuladas",
                    Margem::Espiculada => "espiculadas",
                })
                .collect();
            let padrao = match m.padrao_ecogenico {
                PadraoEcogenico::Anecoico => "anecoica",
                PadraoEcogenico::Hiperecoico => "hiperecoica",
                PadraoEcogenico::ComplexoCisticoESolido => "complexa cística e sólida",
                PadraoEcogenico::Hipoecoico => "hipoecoica",
                PadraoEcogenico::Isoecoico => "isoecoica",
                PadraoEcogenico::Heterogeneo => "heterogênea",
            };
            let posterior = match m.posterior {
                Posterior::Nenhum => "sem achados acústicos posteriores",
                Posterior::Reforco => "com reforço acústico posterior",
                Posterior::Sombra => "com sombra acústica posterior",
                Posterior::Combinado => "com padrão acústico posterior combinado",
            };
            format!(
                "massa {padrao}, {forma}, {orientacao}, de margens {}, {posterior}, medindo {maior} no maior eixo",
                margens.join(" e ")
            )
        }
        _ => "sem massa descrita".to_owned(),
    };
    let biopsia = if a.biopsia_maligna_confirmada {
        " Malignidade confirmada por biópsia."
    } else {
        ""
    };
    format!(
        "{rotulo}: {lado}, {}, {descricao}. BI-RADS {}.{biopsia}",
        minuscula_inicial(local),
        r.categoria.sigla()
    )
}

/// Linha da conclusão.
#[must_use]
pub fn texto_conclusao(rotulo: &str, r: &Resultado) -> String {
    format!(
        "{rotulo}: BI-RADS {}, {}; conduta: {}.",
        r.categoria.sigla(),
        r.categoria.nome(),
        r.categoria.conduta()
    )
}
