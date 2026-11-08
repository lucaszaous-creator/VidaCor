//! Regras da lista de espera e da oferta de vaga (docs/04 Lista-Espera, docs/27 FC-AGD-08). Sem I/O
//! (G5): o núcleo busca os pedidos e grava; aqui só se decide a ordem, quem cabe na vaga e o prazo.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Prioridade clínica do pedido. Valor desconhecido é rejeitado (SEC-MSG-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Prioridade {
    // A ordem da declaração é a ordem da fila (derive Ord).
    Urgente,
    Oncologico,
    Gestante,
    Rotina,
}

impl Prioridade {
    #[must_use]
    pub fn como_str(self) -> &'static str {
        match self {
            Self::Urgente => "urgente",
            Self::Oncologico => "oncologico",
            Self::Gestante => "gestante",
            Self::Rotina => "rotina",
        }
    }

    /// # Errors
    /// Devolve o texto recebido quando não é uma prioridade conhecida.
    pub fn de_str(texto: &str) -> Result<Self, String> {
        match texto {
            "urgente" => Ok(Self::Urgente),
            "oncologico" => Ok(Self::Oncologico),
            "gestante" => Ok(Self::Gestante),
            "rotina" => Ok(Self::Rotina),
            outro => Err(outro.to_owned()),
        }
    }
}

/// Preferência de horário do paciente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preferencia {
    Qualquer,
    Manha,
    Tarde,
}

impl Preferencia {
    #[must_use]
    pub fn como_str(self) -> &'static str {
        match self {
            Self::Qualquer => "qualquer",
            Self::Manha => "manha",
            Self::Tarde => "tarde",
        }
    }

    /// # Errors
    /// Devolve o texto recebido quando não é uma preferência conhecida.
    pub fn de_str(texto: &str) -> Result<Self, String> {
        match texto {
            "qualquer" => Ok(Self::Qualquer),
            "manha" => Ok(Self::Manha),
            "tarde" => Ok(Self::Tarde),
            outro => Err(outro.to_owned()),
        }
    }

    /// Se o horário (hora local da clínica, 0 a 23) atende a preferência. Manhã é antes do meio-dia.
    #[must_use]
    pub fn aceita_hora(self, hora_local: u32) -> bool {
        match self {
            Self::Qualquer => true,
            Self::Manha => hora_local < 12,
            Self::Tarde => hora_local >= 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErroEspera {
    #[error("pedido urgente precisa de justificativa e do médico solicitante")]
    UrgenteSemJustificativa,
}

/// Confere o pedido de inclusão na lista.
///
/// # Errors
/// [`ErroEspera::UrgenteSemJustificativa`] quando a prioridade é urgente sem justificativa (5 ou mais
/// letras) e sem médico solicitante.
pub fn conferir_inclusao(
    prioridade: Prioridade,
    justificativa: Option<&str>,
    medico_solicitante: Option<&str>,
) -> Result<(), ErroEspera> {
    let preenchido =
        |t: Option<&str>, minimo: usize| t.is_some_and(|v| v.trim().chars().count() >= minimo);
    if prioridade == Prioridade::Urgente
        && !(preenchido(justificativa, 5) && preenchido(medico_solicitante, 3))
    {
        return Err(ErroEspera::UrgenteSemJustificativa);
    }
    Ok(())
}

/// O que a ordem da fila precisa saber de um pedido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PedidoNaFila<Id> {
    pub id: Id,
    pub prioridade: Prioridade,
    pub preferencia: Preferencia,
    /// Modalidade do exame pedido (nula: qualquer sala).
    pub modalidade: Option<String>,
    pub desde: DateTime<Utc>,
}

/// Ordena a fila: prioridade clínica, depois quem espera há mais tempo (empate pelo id, estável).
pub fn ordenar<Id: Ord>(fila: &mut [PedidoNaFila<Id>]) {
    fila.sort_by(|a, b| {
        a.prioridade
            .cmp(&b.prioridade)
            .then(a.desde.cmp(&b.desde))
            .then(a.id.cmp(&b.id))
    });
}

/// A vaga que abriu: modalidade da sala e hora local de início.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vaga<'a> {
    pub modalidade_sala: &'a str,
    pub hora_local: u32,
}

/// Se o pedido cabe na vaga: exame que a sala faz e horário que o paciente aceita.
#[must_use]
pub fn cabe_na_vaga<Id>(pedido: &PedidoNaFila<Id>, vaga: &Vaga<'_>) -> bool {
    let modalidade_ok = pedido
        .modalidade
        .as_deref()
        .is_none_or(|m| m == vaga.modalidade_sala);
    modalidade_ok && pedido.preferencia.aceita_hora(vaga.hora_local)
}

/// Antecedência mínima entre o fim do prazo de resposta e o início do exame (preparo e chegada).
pub const ANTECEDENCIA_MIN: i64 = 30;
/// Prazo mínimo útil para o paciente responder.
pub const PRAZO_UTIL_MIN: i64 = 5;
/// Prazos de resposta aceitos (minutos), como na tela (Lista-Espera.dc.html).
pub const PRAZOS_MIN: [i64; 5] = [15, 20, 30, 60, 120];

/// Fim do prazo de resposta de uma oferta: o prazo pedido, mas nunca depois de 30 minutos antes do
/// exame. `None` quando não sobra prazo útil (a vaga está perto demais para oferecer).
#[must_use]
pub fn prazo_da_oferta(
    agora: DateTime<Utc>,
    inicio_da_vaga: DateTime<Utc>,
    prazo_min: i64,
) -> Option<DateTime<Utc>> {
    let limite = inicio_da_vaga - Duration::minutes(ANTECEDENCIA_MIN);
    let expira = (agora + Duration::minutes(prazo_min)).min(limite);
    (expira >= agora + Duration::minutes(PRAZO_UTIL_MIN)).then_some(expira)
}

#[cfg(test)]
mod testes {
    use super::*;
    use chrono::TimeZone;

    fn t(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 9, h, m, 0).unwrap()
    }

    fn pedido(id: u32, p: Prioridade, desde: DateTime<Utc>) -> PedidoNaFila<u32> {
        PedidoNaFila {
            id,
            prioridade: p,
            preferencia: Preferencia::Qualquer,
            modalidade: None,
            desde,
        }
    }

    #[test]
    fn prioridade_vem_antes_do_tempo_de_espera() {
        let mut fila = vec![
            pedido(1, Prioridade::Rotina, t(8, 0)),
            pedido(2, Prioridade::Gestante, t(9, 0)),
            pedido(3, Prioridade::Urgente, t(10, 0)),
            pedido(4, Prioridade::Rotina, t(7, 0)),
            pedido(5, Prioridade::Oncologico, t(11, 0)),
        ];
        ordenar(&mut fila);
        let ids: Vec<u32> = fila.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![3, 5, 2, 4, 1]);
    }

    #[test]
    fn urgente_sem_justificativa_e_recusado() {
        assert_eq!(
            conferir_inclusao(Prioridade::Urgente, Some("dor"), Some("Dr. Paulo")),
            Err(ErroEspera::UrgenteSemJustificativa)
        );
        assert_eq!(
            conferir_inclusao(Prioridade::Urgente, Some("suspeita de dissecção"), None),
            Err(ErroEspera::UrgenteSemJustificativa)
        );
        assert!(conferir_inclusao(
            Prioridade::Urgente,
            Some("suspeita de dissecção"),
            Some("Dr. Paulo Medeiros")
        )
        .is_ok());
        assert!(conferir_inclusao(Prioridade::Rotina, None, None).is_ok());
    }

    #[test]
    fn vaga_respeita_modalidade_e_preferencia() {
        let mut p = pedido(1, Prioridade::Rotina, t(8, 0));
        p.modalidade = Some("US".into());
        p.preferencia = Preferencia::Tarde;
        let manha_us = Vaga {
            modalidade_sala: "US",
            hora_local: 9,
        };
        let tarde_us = Vaga {
            modalidade_sala: "US",
            hora_local: 14,
        };
        let tarde_ct = Vaga {
            modalidade_sala: "CT",
            hora_local: 14,
        };
        assert!(!cabe_na_vaga(&p, &manha_us));
        assert!(cabe_na_vaga(&p, &tarde_us));
        assert!(!cabe_na_vaga(&p, &tarde_ct));
        p.modalidade = None;
        assert!(cabe_na_vaga(&p, &tarde_ct));
    }

    #[test]
    fn prazo_nunca_passa_de_meia_hora_antes_do_exame() {
        // Pedido de 20 min, exame daqui a 3 h: vale o pedido.
        assert_eq!(prazo_da_oferta(t(10, 0), t(13, 0), 20), Some(t(10, 20)));
        // Exame daqui a 40 min: o prazo encolhe para 10 min.
        assert_eq!(prazo_da_oferta(t(10, 0), t(10, 40), 20), Some(t(10, 10)));
        // Exame daqui a 33 min: sobram 3 min, menos que o útil. Não oferece.
        assert_eq!(prazo_da_oferta(t(10, 0), t(10, 33), 20), None);
        // Vaga que já passou.
        assert_eq!(prazo_da_oferta(t(10, 0), t(9, 0), 20), None);
    }

    #[test]
    fn texto_desconhecido_e_rejeitado() {
        assert!(Prioridade::de_str("vip").is_err());
        assert!(Preferencia::de_str("noite").is_err());
        assert_eq!(Prioridade::de_str("gestante"), Ok(Prioridade::Gestante));
        assert_eq!(Preferencia::Manha.como_str(), "manha");
    }
}
