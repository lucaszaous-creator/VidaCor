//! Contrato HTTP da autorização pedida pelo sistema (docs/22 E4). Situação como texto estável.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SolicitanteDoPedido {
    pub nome: String,
    /// Código do conselho no TISS (06 = CRM).
    pub conselho: String,
    pub numero: String,
    /// Código IBGE da UF (33 = RJ).
    pub uf: String,
    /// CBO-S de 6 dígitos.
    pub cbos: String,
}

/// Pedido de autorização a partir do agendamento (ou da reserva provisória). Os procedimentos vêm do
/// agendamento; o canal (webservice ou portal) vem do contrato da operadora.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NovoPedidoAutorizacao {
    pub agendamento_id: Uuid,
    pub operadora: String,
    pub carteirinha: String,
    pub solicitante: SolicitanteDoPedido,
    pub indicacao_clinica: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PedidoProtocolo {
    pub protocolo: String,
}

/// Resposta copiada do portal da operadora (canal portal): fica gravada como recebida, com hash.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RespostaDoPortal {
    /// autorizada, parcial, negada, em_analise ou pendente_documento.
    pub situacao: String,
    pub numero_guia_operadora: Option<String>,
    pub senha: Option<String>,
    pub data_autorizacao: Option<NaiveDate>,
    pub validade: Option<NaiveDate>,
    #[serde(default)]
    pub procedimentos: Vec<ProcedimentoRespondidoSerial>,
    pub motivo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcedimentoRespondidoSerial {
    pub tuss: String,
    pub autorizado: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PacienteDoPedido {
    pub nome: String,
    pub nascimento: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutorizacaoResumo {
    pub id: Uuid,
    pub codigo: String,
    pub paciente: PacienteDoPedido,
    pub operadora: String,
    /// webservice ou portal.
    pub canal: String,
    /// aguardando_envio, aguardando_portal, enviada, em_analise, pendente_documento, autorizada,
    /// parcial, negada, cancelada ou recusada.
    pub situacao: String,
    pub procedimentos: Vec<String>,
    pub protocolo: Option<String>,
    pub motivo: Option<String>,
    pub agendamento_id: Uuid,
    pub inicio_do_exame: DateTime<Utc>,
    pub criado_em: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcedimentoDoPedido {
    pub tuss: String,
    pub descricao: String,
    pub solicitado: i32,
    /// Nulo até a resposta.
    pub autorizado: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RespostaRecebida {
    /// webservice ou portal.
    pub origem: String,
    pub sha256: String,
    pub situacao_lida: Option<String>,
    pub erro_leitura: Option<String>,
    pub hash_confere: Option<bool>,
    /// A resposta mudou o pedido (respostas depois da final só ficam registradas).
    pub aplicada: bool,
    pub recebida_em: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GuiaPreenchida {
    pub id: Uuid,
    pub codigo: String,
    pub numero: String,
    pub senha: String,
    pub validade: NaiveDate,
    /// disponivel ou consumida (consumida quando confirmou a reserva).
    pub situacao: String,
}

/// Consulta automática da operadora pendente no outbox (espera crescente; R7).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsultaPendente {
    pub tentativas: i32,
    pub proxima_em: DateTime<Utc>,
    pub ultimo_erro: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutorizacaoDetalhe {
    pub resumo: AutorizacaoResumo,
    pub carteirinha: String,
    pub solicitante: String,
    pub indicacao_clinica: Option<String>,
    pub enviada_em: Option<DateTime<Utc>>,
    pub respondida_em: Option<DateTime<Utc>>,
    pub itens: Vec<ProcedimentoDoPedido>,
    pub respostas: Vec<RespostaRecebida>,
    pub guia: Option<GuiaPreenchida>,
    /// Envio ou consulta à operadora ainda pendente no outbox.
    pub pendente_no_outbox: Option<ConsultaPendente>,
    /// Situação atual do agendamento (reserva_provisoria, confirmado...).
    pub situacao_do_agendamento: String,
}
