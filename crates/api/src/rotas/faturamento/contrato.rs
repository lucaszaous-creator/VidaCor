//! Contrato HTTP do faturamento TISS (docs/22 E5). Valores sempre em centavos; situação e efeito
//! como texto estável (os valores possíveis estão em cada campo).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Dois identificadores do paciente (N1), como em todo ato com paciente.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PacienteDaConta {
    pub nome: String,
    pub nascimento: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContaResumo {
    pub id: Uuid,
    /// Número da guia no prestador (ex.: `FAT-2026-000123`).
    pub codigo: String,
    pub paciente: PacienteDaConta,
    pub operadora: String,
    pub numero_guia_operadora: String,
    /// Código TUSS e descrição de cada procedimento executado.
    pub procedimentos: Vec<String>,
    pub valor_total_centavos: i64,
    /// em_conferencia, pronta, com_pendencia, em_lote, enviada, paga, paga_com_glosa ou glosada.
    pub situacao: String,
    pub data_execucao: NaiveDate,
    /// Último dia para enviar, pelo contrato (nulo sem contrato).
    pub enviar_ate: Option<NaiveDate>,
    /// Pendências abertas que deixam a guia fora do lote (bloqueia ou corrigir antes).
    pub pendencias_que_impedem: i64,
    pub pendencias_de_atencao: i64,
    /// Versão da regra da última conferência (ex.: `AUD-UNIMED-2026.1`).
    pub regra_versao: Option<String>,
    pub lote_codigo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendenciaDaConta {
    pub regra: String,
    pub regra_versao: String,
    /// bloqueia, corrigir_antes ou atencao.
    pub efeito: String,
    /// De onde veio a regra (padrão TISS, manual do prestador, contrato, regra da clínica).
    pub origem: String,
    pub titulo: String,
    pub detalhe: String,
    pub o_que_fazer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ItemDaConta {
    pub sequencial: i32,
    pub tuss: String,
    pub descricao: String,
    pub quantidade: i32,
    /// Nulo quando o código não está na tabela da operadora.
    pub valor_unitario_centavos: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct Solicitante {
    pub nome: String,
    /// Código do conselho no TISS (06 = CRM).
    pub conselho: Option<String>,
    pub numero: Option<String>,
    /// Código IBGE da UF (33 = RJ).
    pub uf: Option<String>,
    /// CBO-S de 6 dígitos.
    pub cbos: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VersaoDaConta {
    pub versao: i32,
    pub motivo: String,
    pub criado_em: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Recurso {
    pub id: Uuid,
    pub codigo: String,
    pub justificativa: String,
    /// aguardando_envio ou enviado.
    pub situacao: String,
    pub criado_em: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Glosa {
    pub id: Uuid,
    pub codigo: String,
    pub conta_id: Uuid,
    pub conta_codigo: String,
    pub paciente: PacienteDaConta,
    pub operadora: String,
    /// Código do motivo de glosa da tabela TISS (4 dígitos).
    pub codigo_glosa: String,
    pub motivo: String,
    pub valor_centavos: i64,
    pub prazo_recurso: NaiveDate,
    /// aberta ou em_recurso.
    pub situacao: String,
    pub recurso: Option<Recurso>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContaDetalhe {
    pub resumo: ContaResumo,
    pub carteirinha: String,
    pub senha: String,
    pub validade_senha: NaiveDate,
    pub solicitante: Solicitante,
    pub cid: Option<String>,
    pub assinatura_beneficiario: bool,
    pub itens: Vec<ItemDaConta>,
    pub pendencias: Vec<PendenciaDaConta>,
    /// Versões anteriores, da mais recente para a mais antiga.
    pub historico: Vec<VersaoDaConta>,
    pub versao: i32,
    pub glosas: Vec<Glosa>,
}

/// Resultado de "Conferir guias": contas novas apuradas e situação depois da auditoria.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResultadoConferencia {
    pub novas: i64,
    pub conferidas: i64,
    pub prontas: i64,
    pub com_pendencia: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResumoAuditoria {
    pub conferidas: i64,
    pub prontas: i64,
    pub corrigir_antes: i64,
    pub bloqueadas: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContaPendente {
    pub conta_id: Uuid,
    pub codigo: String,
    pub paciente: PacienteDaConta,
    pub operadora: String,
    pub numero_guia_operadora: String,
    pub detalhe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GrupoDePendencias {
    pub regra: String,
    pub titulo: String,
    pub efeito: String,
    pub origem: String,
    pub o_que_fazer: String,
    pub regra_versoes: Vec<String>,
    pub contas: Vec<ContaPendente>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegraDoConjunto {
    pub regra: String,
    pub titulo: String,
    pub efeito: String,
    pub origem: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConjuntoDeRegras {
    pub versao: String,
    pub nome: String,
    /// Operadoras com contrato nesta unidade que usam este conjunto.
    pub operadoras: Vec<String>,
    pub regras: Vec<RegraDoConjunto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PainelAuditoria {
    pub resumo: ResumoAuditoria,
    pub grupos: Vec<GrupoDePendencias>,
    pub conjuntos: Vec<ConjuntoDeRegras>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Contrato {
    pub id: Uuid,
    pub codigo: String,
    pub operadora: String,
    pub registro_ans: String,
    pub regra_versao: String,
    pub prazo_envio_dias: i32,
    pub prazo_recurso_dias: i32,
    pub contas_prontas: i64,
}

/// Envio à operadora ainda pendente no outbox (R7: visível, não se perde).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnvioPendente {
    pub tentativas: i32,
    pub ultimo_erro: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Lote {
    pub id: Uuid,
    pub codigo: String,
    pub numero: i64,
    pub contrato_id: Uuid,
    pub operadora: String,
    pub quantidade_guias: i32,
    pub valor_total_centavos: i64,
    /// gerado, aguardando_envio, enviado ou demonstrativo_recebido.
    pub situacao: String,
    pub versao_tiss: String,
    pub hash_tiss: String,
    pub sha256: String,
    pub protocolo: Option<String>,
    pub criado_em: DateTime<Utc>,
    pub enviado_em: Option<DateTime<Utc>>,
    pub envio_pendente: Option<EnvioPendente>,
    pub valor_pago_centavos: Option<i64>,
    pub valor_glosado_centavos: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoteDetalhe {
    pub lote: Lote,
    pub contas: Vec<ContaResumo>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PedidoLote {
    pub contrato_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SolicitanteCorrigido {
    pub conselho: String,
    pub numero: String,
    pub uf: String,
    pub cbos: String,
}

/// Correção da conta antes do lote: gera versão nova com motivo e confere de novo.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PedidoCorrecao {
    pub motivo: String,
    pub solicitante: Option<SolicitanteCorrigido>,
    /// CID-10 sem ponto (ex.: `R51`).
    pub cid: Option<String>,
    pub assinatura_beneficiario: Option<bool>,
    /// Reaplica a tabela de preços vigente na data do exame.
    #[serde(default)]
    pub reaplicar_tabela: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GlosaRecebida {
    pub codigo_glosa: String,
    pub motivo: String,
    pub valor_centavos: i64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RetornoDaConta {
    pub conta_id: Uuid,
    pub valor_pago_centavos: i64,
    #[serde(default)]
    pub glosas: Vec<GlosaRecebida>,
}

/// Demonstrativo de retorno da operadora, conta a conta.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PedidoDemonstrativo {
    pub recebido_em: NaiveDate,
    pub contas: Vec<RetornoDaConta>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PedidoRecurso {
    pub justificativa: String,
}
