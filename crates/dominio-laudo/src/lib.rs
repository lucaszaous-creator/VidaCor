//! Laudo estruturado com classificações (docs/22 E2, docs/25 § 3.3). Sem I/O (G5): recebe os
//! descritores escolhidos pelo médico e devolve categoria, conduta e texto sugerido, sempre com o
//! identificador da versão da regra aplicada. O núcleo grava essa versão junto do achado; mudança de
//! versão não altera achados antigos (docs/22, "Regras novas").
//!
//! Regras desta versão:
//! - [`tirads`]: ACR TI-RADS 2017 (`TIRADS-ACR-2017.1`), pontos somados e categoria calculada aqui;
//! - [`birads`]: BI-RADS 5ª edição, ultrassom de mama (`BIRADS-ACR-2013-US.1`), categoria escolhida
//!   pelo médico e conferida por regras de coerência.
//!
//! **Os casos de teste destas tabelas aguardam conferência por médico** (docs/25 § 3.3); nada aqui
//! foi aprovado pelo responsável clínico de segurança. Pontos marcados "a confirmar" estão no
//! DEC-2026-11-08-60.

#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]

pub mod birads;
pub mod tirads;

use std::fmt;

/// Medida em décimos de milímetro (sem ponto flutuante na regra: 1,5 cm = 150).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Medida(u32);

impl Medida {
    /// Converte milímetros (com uma casa decimal) para a medida, recusando zero, negativo, valor
    /// não numérico e acima do limite do órgão.
    ///
    /// # Errors
    /// [`ErroClassificacao::MedidaInvalida`] fora de 0,1 mm até `limite_mm`.
    pub fn de_milimetros(mm: f64, limite_mm: u32) -> Result<Self, ErroClassificacao> {
        if !mm.is_finite() || mm < 0.05 || mm > f64::from(limite_mm) {
            return Err(ErroClassificacao::MedidaInvalida { limite_mm });
        }
        // Positivo, finito e limitado acima: cabe em u32 depois do arredondamento.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let decimos = (mm * 10.0).round() as u32;
        Ok(Self(decimos))
    }

    #[must_use]
    pub const fn de_decimos(decimos: u32) -> Self {
        Self(decimos)
    }

    #[must_use]
    pub const fn decimos(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn milimetros(self) -> f64 {
        f64::from(self.0) / 10.0
    }
}

/// "18 mm" ou "8,5 mm" (vírgula decimal, como no laudo).
impl fmt::Display for Medida {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inteiro = self.0 / 10;
        let decimo = self.0 % 10;
        if decimo == 0 {
            write!(f, "{inteiro} mm")
        } else {
            write!(f, "{inteiro},{decimo} mm")
        }
    }
}

/// Erros de preenchimento ou de coerência. O código é estável (vai para o Problem Details) e a
/// mensagem diz o que aconteceu e o que fazer (docs/16 R8).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErroClassificacao {
    #[error("A medida precisa estar entre 0,1 mm e {limite_mm} mm. Confira o maior diâmetro.")]
    MedidaInvalida { limite_mm: u32 },
    #[error("A localização do achado é obrigatória e tem até 120 caracteres.")]
    LocalizacaoInvalida,
    #[error("Focos ecogênicos: \"nenhum\" não pode ser marcado junto com outro foco.")]
    FocosIncoerentes,
    #[error("Ecogenicidade anecoica só se aplica a nódulo cístico ou quase todo cístico. Confira a composição ou a ecogenicidade.")]
    AnecoicoNaoCistico,
    #[error("Margem circunscrita não pode ser marcada junto com margem não circunscrita. Marque uma ou outra.")]
    MargensIncoerentes,
    #[error("Descreva pelo menos uma margem da massa.")]
    MargemAusente,
    #[error("BI-RADS 6 exige malignidade confirmada por biópsia. Confirme a biópsia ou escolha outra categoria.")]
    Birads6SemBiopsia,
    #[error("BI-RADS 1 (negativo) não admite massa descrita. Retire a massa ou escolha outra categoria.")]
    NegativoComMassa,
    #[error("A categoria {categoria} exige a massa descrita (forma, orientação, margens, padrão e achados posteriores) e o maior diâmetro.")]
    MassaAusente { categoria: &'static str },
    #[error("Sem massa descrita não há maior diâmetro. Retire a medida ou descreva a massa.")]
    MedidaSemMassa,
    #[error("A massa tem característica suspeita ({suspeitas}); a categoria {categoria} não combina com ela. Use 4A, 4B, 4C, 5 ou 0, ou revise os descritores.")]
    CategoriaBaixaComSuspeita {
        categoria: &'static str,
        suspeitas: String,
    },
    #[error("BI-RADS 5 pede ao menos uma característica suspeita na massa (forma irregular, orientação não paralela ou margem não circunscrita). Revise os descritores ou a categoria.")]
    Categoria5SemSuspeita,
}

impl ErroClassificacao {
    /// Código estável para o Problem Details.
    #[must_use]
    pub const fn codigo(&self) -> &'static str {
        match self {
            Self::MedidaInvalida { .. } => "medida-invalida",
            Self::LocalizacaoInvalida => "localizacao-invalida",
            Self::FocosIncoerentes => "focos-incoerentes",
            Self::AnecoicoNaoCistico => "anecoico-nao-cistico",
            Self::MargensIncoerentes => "margens-incoerentes",
            Self::MargemAusente => "margem-ausente",
            Self::Birads6SemBiopsia => "birads-6-sem-biopsia",
            Self::NegativoComMassa => "birads-1-com-massa",
            Self::MassaAusente { .. } => "massa-ausente",
            Self::MedidaSemMassa => "medida-sem-massa",
            Self::CategoriaBaixaComSuspeita { .. } => "categoria-incoerente",
            Self::Categoria5SemSuspeita => "birads-5-sem-suspeita",
        }
    }
}

/// Localização em texto livre curto ("Lobo direito, terço médio"), sem quebras de linha.
///
/// # Errors
/// [`ErroClassificacao::LocalizacaoInvalida`] vazia, longa ou com caractere de controle.
pub fn localizacao(texto: &str) -> Result<String, ErroClassificacao> {
    let t = texto.trim();
    if t.is_empty() || t.chars().count() > 120 || t.chars().any(char::is_control) {
        return Err(ErroClassificacao::LocalizacaoInvalida);
    }
    Ok(t.to_owned())
}

/// Primeira letra minúscula, para encaixar a localização no meio da frase.
fn minuscula_inicial(texto: &str) -> String {
    let mut c = texto.chars();
    match c.next() {
        Some(p) => p.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}
