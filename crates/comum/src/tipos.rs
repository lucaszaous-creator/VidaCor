//! Tipos de valor usados nos dois identificadores do paciente (docs/16 C3 item 4, docs/14 A3).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErroTipo {
    #[error("nome completo vazio ou com uma palavra só")]
    NomeIncompleto,
    #[error("nome completo com mais de 200 caracteres")]
    NomeLongo,
    #[error("data de nascimento inválida")]
    DataInvalida,
}

/// Nome completo do paciente. Guarda o texto como digitado (aparado) e compara de forma
/// normalizada: sem diferença de maiúsculas, espaços repetidos ou acentos (DEC-2026-11-08-44).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NomeCompleto(String);

impl NomeCompleto {
    /// Valida um nome completo: pelo menos duas palavras e no máximo 200 caracteres.
    ///
    /// # Errors
    /// [`ErroTipo::NomeIncompleto`] ou [`ErroTipo::NomeLongo`].
    pub fn novo(texto: &str) -> Result<Self, ErroTipo> {
        let limpo = texto.split_whitespace().collect::<Vec<_>>().join(" ");
        if limpo.chars().count() > 200 {
            return Err(ErroTipo::NomeLongo);
        }
        if limpo.split(' ').filter(|p| !p.is_empty()).count() < 2 {
            return Err(ErroTipo::NomeIncompleto);
        }
        Ok(Self(limpo))
    }

    #[must_use]
    pub fn como_str(&self) -> &str {
        &self.0
    }

    /// Forma usada só para comparar: minúsculas, sem acentos, espaços simples.
    #[must_use]
    pub fn forma_de_comparacao(&self) -> String {
        self.0
            .nfd()
            .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
            .collect::<String>()
            .to_lowercase()
    }

    /// Mesmo paciente pelo nome?
    #[must_use]
    pub fn confere_com(&self, outro: &Self) -> bool {
        self.forma_de_comparacao() == outro.forma_de_comparacao()
    }
}

impl TryFrom<String> for NomeCompleto {
    type Error = ErroTipo;
    fn try_from(valor: String) -> Result<Self, Self::Error> {
        Self::novo(&valor)
    }
}

impl From<NomeCompleto> for String {
    fn from(valor: NomeCompleto) -> Self {
        valor.0
    }
}

/// Data de nascimento (sem hora, docs/14 A3). Não aceita data futura nem anterior a 1900.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataNascimento(NaiveDate);

impl DataNascimento {
    /// # Errors
    /// [`ErroTipo::DataInvalida`] se a data for futura em relação a `hoje` ou anterior a 1900.
    pub fn nova(data: NaiveDate, hoje: NaiveDate) -> Result<Self, ErroTipo> {
        let minimo = NaiveDate::from_ymd_opt(1900, 1, 1).ok_or(ErroTipo::DataInvalida)?;
        if data > hoje || data < minimo {
            return Err(ErroTipo::DataInvalida);
        }
        Ok(Self(data))
    }

    #[must_use]
    pub fn valor(&self) -> NaiveDate {
        self.0
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn nome(t: &str) -> NomeCompleto {
        NomeCompleto::novo(t).unwrap_or_else(|e| panic!("nome de teste inválido: {e}"))
    }

    #[test]
    fn nome_confere_sem_acento_maiuscula_ou_espaco_extra() {
        assert!(nome("Antônio Carlos Ribeiro").confere_com(&nome("  antonio   CARLOS ribeiro ")));
    }

    #[test]
    fn nome_diferente_nao_confere() {
        assert!(!nome("Maria Helena Duarte").confere_com(&nome("Maria Helena Dutra")));
    }

    #[test]
    fn nome_de_uma_palavra_e_recusado() {
        assert_eq!(
            NomeCompleto::novo("Maria").err(),
            Some(ErroTipo::NomeIncompleto)
        );
        assert_eq!(
            NomeCompleto::novo("   ").err(),
            Some(ErroTipo::NomeIncompleto)
        );
    }

    #[test]
    fn nome_longo_e_recusado() {
        let longo = format!("Ana {}", "a".repeat(200));
        assert_eq!(NomeCompleto::novo(&longo).err(), Some(ErroTipo::NomeLongo));
    }

    #[test]
    fn nascimento_futuro_e_recusado() {
        let hoje = NaiveDate::from_ymd_opt(2026, 11, 7).unwrap_or_default();
        let amanha = NaiveDate::from_ymd_opt(2026, 11, 8).unwrap_or_default();
        assert_eq!(
            DataNascimento::nova(amanha, hoje).err(),
            Some(ErroTipo::DataInvalida)
        );
        let ok = NaiveDate::from_ymd_opt(1954, 6, 15).unwrap_or_default();
        assert!(DataNascimento::nova(ok, hoje).is_ok());
    }
}
