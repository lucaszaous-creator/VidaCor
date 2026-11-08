//! ACR TI-RADS 2017 (Tessler et al., J Am Coll Radiol 2017;14:587-595): cada nódulo de tireoide
//! soma pontos por composição, ecogenicidade, forma, margem e focos ecogênicos; a soma dá a
//! categoria TR1 a TR5 e, com o maior diâmetro, a conduta (PAAF, seguimento ou nenhum).
//!
//! Versão `TIRADS-ACR-2017.1`. Pontos marcados "a confirmar" (DEC-2026-11-08-60):
//! - soma de 1 ponto: a tabela oficial lista TR1 = 0 e TR2 = 2 pontos; aqui 1 ponto é TR2, como no
//!   mockup aprovado (a conduta de TR1 e TR2 é a mesma: sem PAAF e sem seguimento);
//! - nódulo espongiforme: 0 ponto no total, sem somar as demais características (nota da tabela);
//! - ecogenicidade anecoica só com composição cística ou quase toda cística (nota da tabela).
//!
//! Os casos dos testes aguardam conferência por médico.

use crate::{minuscula_inicial, ErroClassificacao, Medida};
use serde::{Deserialize, Serialize};

pub const VERSAO: &str = "TIRADS-ACR-2017.1";
/// Maior nódulo de tireoide aceito (mm).
pub const LIMITE_MM: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Composicao {
    CisticaOuQuaseCistica,
    Espongiforme,
    MistaCisticaESolida,
    SolidaOuQuaseSolida,
    /// Não determinável por calcificação (a tabela manda pontuar 2).
    IndeterminadaPorCalcificacao,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ecogenicidade {
    Anecoica,
    HiperOuIsoecoica,
    Hipoecoica,
    MuitoHipoecoica,
    /// Não determinável (a tabela manda pontuar 1).
    Indeterminada,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Forma {
    MaisLargoQueAlto,
    MaisAltoQueLargo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Margem {
    Lisa,
    MalDefinida,
    LobuladaOuIrregular,
    ExtensaoExtratireoidiana,
    /// Não determinável (a tabela manda pontuar 0).
    Indeterminada,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocoEcogenico {
    NenhumOuCaudaDeCometa,
    Macrocalcificacoes,
    CalcificacoesPerifericas,
    FocosPuntiformes,
}

/// Descritores de um nódulo, como o médico marcou.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nodulo {
    pub composicao: Composicao,
    pub ecogenicidade: Ecogenicidade,
    pub forma: Forma,
    pub margem: Margem,
    /// Pode ter mais de um (os pontos somam); vazio vale "nenhum".
    pub focos: Vec<FocoEcogenico>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Categoria {
    Tr1,
    Tr2,
    Tr3,
    Tr4,
    Tr5,
}

impl Categoria {
    #[must_use]
    pub const fn sigla(self) -> &'static str {
        match self {
            Self::Tr1 => "TR1",
            Self::Tr2 => "TR2",
            Self::Tr3 => "TR3",
            Self::Tr4 => "TR4",
            Self::Tr5 => "TR5",
        }
    }

    #[must_use]
    pub const fn nome(self) -> &'static str {
        match self {
            Self::Tr1 => "benigno",
            Self::Tr2 => "não suspeito",
            Self::Tr3 => "levemente suspeito",
            Self::Tr4 => "moderadamente suspeito",
            Self::Tr5 => "altamente suspeito",
        }
    }

    /// Limites de PAAF e de seguimento (décimos de mm) da tabela: TR3 25 e 15 mm, TR4 15 e 10 mm,
    /// TR5 10 e 5 mm. TR1 e TR2 não têm.
    #[must_use]
    pub const fn limites(self) -> Option<(Medida, Medida)> {
        match self {
            Self::Tr1 | Self::Tr2 => None,
            Self::Tr3 => Some((Medida::de_decimos(250), Medida::de_decimos(150))),
            Self::Tr4 => Some((Medida::de_decimos(150), Medida::de_decimos(100))),
            Self::Tr5 => Some((Medida::de_decimos(100), Medida::de_decimos(50))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conduta {
    /// PAAF indicada (maior diâmetro a partir do limite).
    Paaf {
        limite: Medida,
    },
    /// Seguimento ecográfico (entre o limite de seguimento e o de PAAF).
    Seguimento {
        limite: Medida,
    },
    SemPaafNemSeguimento,
}

impl Conduta {
    #[must_use]
    pub fn texto(self) -> String {
        match self {
            Self::Paaf { limite } => format!("PAAF indicada (a partir de {limite})"),
            Self::Seguimento { limite } => {
                format!("seguimento ecográfico (a partir de {limite})")
            }
            Self::SemPaafNemSeguimento => "sem indicação de PAAF ou de seguimento".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resultado {
    pub pontos: u8,
    pub categoria: Categoria,
    pub conduta: Conduta,
    pub versao: &'static str,
}

/// Normaliza e confere os descritores (focos sem repetição e em ordem).
///
/// # Errors
/// "Nenhum" junto com outro foco; anecoico em nódulo não cístico.
pub fn conferir(n: &Nodulo) -> Result<Nodulo, ErroClassificacao> {
    let mut focos = n.focos.clone();
    focos.sort();
    focos.dedup();
    if focos.len() > 1 && focos.contains(&FocoEcogenico::NenhumOuCaudaDeCometa) {
        return Err(ErroClassificacao::FocosIncoerentes);
    }
    if focos.is_empty() {
        focos.push(FocoEcogenico::NenhumOuCaudaDeCometa);
    }
    if n.ecogenicidade == Ecogenicidade::Anecoica
        && n.composicao != Composicao::CisticaOuQuaseCistica
    {
        return Err(ErroClassificacao::AnecoicoNaoCistico);
    }
    Ok(Nodulo { focos, ..n.clone() })
}

/// Pontos do nódulo pela tabela.
#[must_use]
pub fn pontos(n: &Nodulo) -> u8 {
    if n.composicao == Composicao::Espongiforme {
        return 0;
    }
    let composicao = match n.composicao {
        Composicao::CisticaOuQuaseCistica | Composicao::Espongiforme => 0,
        Composicao::MistaCisticaESolida => 1,
        Composicao::SolidaOuQuaseSolida | Composicao::IndeterminadaPorCalcificacao => 2,
    };
    let ecogenicidade = match n.ecogenicidade {
        Ecogenicidade::Anecoica => 0,
        Ecogenicidade::HiperOuIsoecoica | Ecogenicidade::Indeterminada => 1,
        Ecogenicidade::Hipoecoica => 2,
        Ecogenicidade::MuitoHipoecoica => 3,
    };
    let forma = match n.forma {
        Forma::MaisLargoQueAlto => 0,
        Forma::MaisAltoQueLargo => 3,
    };
    let margem = match n.margem {
        Margem::Lisa | Margem::MalDefinida | Margem::Indeterminada => 0,
        Margem::LobuladaOuIrregular => 2,
        Margem::ExtensaoExtratireoidiana => 3,
    };
    let focos: u8 = n
        .focos
        .iter()
        .map(|f| match f {
            FocoEcogenico::NenhumOuCaudaDeCometa => 0,
            FocoEcogenico::Macrocalcificacoes => 1,
            FocoEcogenico::CalcificacoesPerifericas => 2,
            FocoEcogenico::FocosPuntiformes => 3,
        })
        .sum();
    composicao + ecogenicidade + forma + margem + focos
}

#[must_use]
pub const fn categoria(pontos: u8) -> Categoria {
    match pontos {
        0 => Categoria::Tr1,
        1 | 2 => Categoria::Tr2,
        3 => Categoria::Tr3,
        4..=6 => Categoria::Tr4,
        _ => Categoria::Tr5,
    }
}

#[must_use]
pub fn conduta(categoria: Categoria, maior: Medida) -> Conduta {
    match categoria.limites() {
        Some((paaf, _)) if maior >= paaf => Conduta::Paaf { limite: paaf },
        Some((_, seguimento)) if maior >= seguimento => Conduta::Seguimento { limite: seguimento },
        _ => Conduta::SemPaafNemSeguimento,
    }
}

/// Classifica o nódulo: confere, soma, categoriza e indica a conduta pelo maior diâmetro.
///
/// # Errors
/// Descritores incoerentes (ver [`conferir`]).
pub fn classificar(n: &Nodulo, maior: Medida) -> Result<(Nodulo, Resultado), ErroClassificacao> {
    let n = conferir(n)?;
    let p = pontos(&n);
    let c = categoria(p);
    Ok((
        n,
        Resultado {
            pontos: p,
            categoria: c,
            conduta: conduta(c, maior),
            versao: VERSAO,
        },
    ))
}

/// Frase do achado para o texto do laudo (o médico revisa e edita).
#[must_use]
pub fn texto_achado(rotulo: &str, local: &str, maior: Medida, n: &Nodulo, r: &Resultado) -> String {
    let composicao = match n.composicao {
        Composicao::CisticaOuQuaseCistica => "cístico ou quase todo cístico",
        Composicao::Espongiforme => "espongiforme",
        Composicao::MistaCisticaESolida => "misto cístico e sólido",
        Composicao::SolidaOuQuaseSolida => "sólido ou quase todo sólido",
        Composicao::IndeterminadaPorCalcificacao => "de composição indeterminada por calcificação",
    };
    let eco = match n.ecogenicidade {
        Ecogenicidade::Anecoica => "anecoico",
        Ecogenicidade::HiperOuIsoecoica => "hiper ou isoecoico",
        Ecogenicidade::Hipoecoica => "hipoecoico",
        Ecogenicidade::MuitoHipoecoica => "muito hipoecoico",
        Ecogenicidade::Indeterminada => "de ecogenicidade indeterminada",
    };
    let forma = match n.forma {
        Forma::MaisLargoQueAlto => "mais largo que alto",
        Forma::MaisAltoQueLargo => "mais alto que largo",
    };
    let margem = match n.margem {
        Margem::Lisa => "margens lisas",
        Margem::MalDefinida => "margens mal definidas",
        Margem::LobuladaOuIrregular => "margens lobuladas ou irregulares",
        Margem::ExtensaoExtratireoidiana => "extensão extratireoidiana",
        Margem::Indeterminada => "margens indeterminadas",
    };
    let focos: Vec<&str> = n
        .focos
        .iter()
        .map(|f| match f {
            FocoEcogenico::NenhumOuCaudaDeCometa => "sem focos ecogênicos",
            FocoEcogenico::Macrocalcificacoes => "macrocalcificações",
            FocoEcogenico::CalcificacoesPerifericas => "calcificações periféricas",
            FocoEcogenico::FocosPuntiformes => "focos ecogênicos puntiformes",
        })
        .collect();
    format!(
        "{rotulo}: no {}, nódulo de {maior} no maior eixo, {composicao}, {eco}, {forma}, {margem}, {}. ACR TI-RADS {} ({} pontos).",
        minuscula_inicial(local),
        focos.join(" e "),
        r.categoria.sigla(),
        r.pontos
    )
}

/// Linha da conclusão.
#[must_use]
pub fn texto_conclusao(rotulo: &str, r: &Resultado) -> String {
    format!(
        "{rotulo}: ACR TI-RADS {}, {}; {}.",
        r.categoria.sigla(),
        r.categoria.nome(),
        r.conduta.texto()
    )
}
