//! Regras do check-in (docs/16 C3 item 4, docs/29 REQ-REC-001). Sem I/O (G5): quem chama busca o
//! agendamento e grava o resultado; este crate só decide.

#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]

pub mod cadastro;

use serde::{Deserialize, Serialize};
use vidacor_comum::{DataNascimento, NomeCompleto};

/// Situação do agendamento (docs/29 § 6.2). Valor desconhecido é rejeitado na leitura (SEC-MSG-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SituacaoAgendamento {
    Agendado,
    Confirmado,
    EmAtendimento,
    Concluido,
    Faltou,
    Cancelado,
}

impl SituacaoAgendamento {
    #[must_use]
    pub fn como_str(self) -> &'static str {
        match self {
            Self::Agendado => "agendado",
            Self::Confirmado => "confirmado",
            Self::EmAtendimento => "em_atendimento",
            Self::Concluido => "concluido",
            Self::Faltou => "faltou",
            Self::Cancelado => "cancelado",
        }
    }

    /// Converte o texto gravado no banco.
    ///
    /// # Errors
    /// Devolve o texto recebido quando não é uma situação conhecida.
    pub fn de_str(texto: &str) -> Result<Self, String> {
        match texto {
            "agendado" => Ok(Self::Agendado),
            "confirmado" => Ok(Self::Confirmado),
            "em_atendimento" => Ok(Self::EmAtendimento),
            "concluido" => Ok(Self::Concluido),
            "faltou" => Ok(Self::Faltou),
            "cancelado" => Ok(Self::Cancelado),
            outro => Err(outro.to_owned()),
        }
    }
}

/// O que o check-in precisa saber do agendamento e do cadastro.
#[derive(Debug, Clone)]
pub struct AgendamentoParaCheckin {
    pub situacao: SituacaoAgendamento,
    pub nome_cadastro: NomeCompleto,
    pub nascimento_cadastro: DataNascimento,
}

/// O que a recepção confirmou com o paciente à frente (dois identificadores).
#[derive(Debug, Clone)]
pub struct ConfirmacaoIdentidade {
    pub nome: NomeCompleto,
    pub nascimento: DataNascimento,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErroCheckin {
    #[error("nome ou nascimento não conferem com o cadastro")]
    IdentificacaoNaoConfere,
    #[error("a situação do agendamento ({0}) não permite check-in")]
    SituacaoNaoPermite(&'static str),
}

/// Decide se o check-in pode ser feito. Não diz qual dos dois identificadores divergiu, para não
/// virar oráculo de dados do cadastro (SEC-DAD-4).
///
/// # Errors
/// [`ErroCheckin::SituacaoNaoPermite`] se o agendamento não está agendado ou confirmado;
/// [`ErroCheckin::IdentificacaoNaoConfere`] se nome ou nascimento divergem.
pub fn conferir_checkin(
    agendamento: &AgendamentoParaCheckin,
    confirmacao: &ConfirmacaoIdentidade,
) -> Result<(), ErroCheckin> {
    match agendamento.situacao {
        SituacaoAgendamento::Agendado | SituacaoAgendamento::Confirmado => {}
        outra => return Err(ErroCheckin::SituacaoNaoPermite(outra.como_str())),
    }
    let nome_ok = agendamento.nome_cadastro.confere_com(&confirmacao.nome);
    let nascimento_ok = agendamento.nascimento_cadastro == confirmacao.nascimento;
    if nome_ok && nascimento_ok {
        Ok(())
    } else {
        Err(ErroCheckin::IdentificacaoNaoConfere)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use chrono_teste::data;

    mod chrono_teste {
        use vidacor_comum::DataNascimento;
        pub fn data(a: i32, m: u32, d: u32) -> DataNascimento {
            let hoje = chrono::NaiveDate::from_ymd_opt(2026, 11, 7).unwrap_or_default();
            let valor = chrono::NaiveDate::from_ymd_opt(a, m, d).unwrap_or_default();
            DataNascimento::nova(valor, hoje).unwrap_or_else(|e| panic!("data de teste: {e}"))
        }
    }

    fn nome(t: &str) -> NomeCompleto {
        NomeCompleto::novo(t).unwrap_or_else(|e| panic!("nome de teste: {e}"))
    }

    fn antonio(situacao: SituacaoAgendamento) -> AgendamentoParaCheckin {
        AgendamentoParaCheckin {
            situacao,
            nome_cadastro: nome("Antônio Carlos Ribeiro"),
            nascimento_cadastro: data(1954, 6, 15),
        }
    }

    #[test]
    fn checkin_com_dois_identificadores_corretos() {
        let c = ConfirmacaoIdentidade {
            nome: nome("antonio carlos ribeiro"),
            nascimento: data(1954, 6, 15),
        };
        assert_eq!(
            conferir_checkin(&antonio(SituacaoAgendamento::Confirmado), &c),
            Ok(())
        );
    }

    #[test]
    fn nascimento_errado_bloqueia() {
        let c = ConfirmacaoIdentidade {
            nome: nome("Antônio Carlos Ribeiro"),
            nascimento: data(1954, 6, 16),
        };
        assert_eq!(
            conferir_checkin(&antonio(SituacaoAgendamento::Agendado), &c),
            Err(ErroCheckin::IdentificacaoNaoConfere)
        );
    }

    #[test]
    fn homonimo_com_nascimento_proximo_bloqueia() {
        // Maria Helena Duarte 12/03/1968 e o duplicado 21/03/1968 (design/dados-ficticios.md)
        let ag = AgendamentoParaCheckin {
            situacao: SituacaoAgendamento::Agendado,
            nome_cadastro: nome("Maria Helena Duarte"),
            nascimento_cadastro: data(1968, 3, 12),
        };
        let c = ConfirmacaoIdentidade {
            nome: nome("Maria Helena Duarte"),
            nascimento: data(1968, 3, 21),
        };
        assert_eq!(
            conferir_checkin(&ag, &c),
            Err(ErroCheckin::IdentificacaoNaoConfere)
        );
    }

    #[test]
    fn situacao_que_nao_permite_checkin() {
        let c = ConfirmacaoIdentidade {
            nome: nome("Antônio Carlos Ribeiro"),
            nascimento: data(1954, 6, 15),
        };
        for s in [
            SituacaoAgendamento::EmAtendimento,
            SituacaoAgendamento::Concluido,
            SituacaoAgendamento::Faltou,
            SituacaoAgendamento::Cancelado,
        ] {
            assert_eq!(
                conferir_checkin(&antonio(s), &c),
                Err(ErroCheckin::SituacaoNaoPermite(s.como_str()))
            );
        }
    }

    #[test]
    fn situacao_desconhecida_e_rejeitada() {
        assert_eq!(
            SituacaoAgendamento::de_str("pendente"),
            Err("pendente".to_owned())
        );
    }
}
