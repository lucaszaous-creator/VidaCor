//! Contrato HTTP de Convênios e tabelas. Valores em centavos; situações como texto estável (os valores
//! possíveis estão em cada campo).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Termos do contrato que o faturamento edita (o estado inteiro vai e volta, sem campo solto).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TermosDoConvenio {
    pub codigo_prestador: String,
    pub nome_prestador: String,
    pub cnes: String,
    /// Conjunto de regras da auditoria antes do lote (ex.: `AUD-UNIMED-2026.1`).
    pub regra_versao: String,
    pub prazo_envio_dias: i32,
    pub prazo_recurso_dias: i32,
    pub prazo_pagamento_dias: Option<i32>,
    pub vigencia_inicio: Option<NaiveDate>,
    pub vigencia_fim: Option<NaiveDate>,
    /// Índice e mês do reajuste (ex.: "IPCA em janeiro").
    pub reajuste: Option<String>,
    pub contato_nome: Option<String>,
    pub contato_telefone: Option<String>,
    pub contato_email: Option<String>,
    pub autorizacao_por_webservice: bool,
    pub ativo: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ConvenioResumo {
    pub id: Uuid,
    pub codigo: String,
    pub operadora: String,
    pub registro_ans: String,
    /// vigente, renovar, vencido, sem_vigencia ou inativo.
    pub situacao: String,
    pub vigencia_fim: Option<NaiveDate>,
    /// Versão da tabela vigente hoje (nula sem tabela).
    pub tabela_versao: Option<i32>,
    pub proposta_pendente: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct VersaoDaTabelaDoConvenio {
    pub id: Uuid,
    pub codigo: String,
    pub versao: i32,
    pub vigente_desde: NaiveDate,
    pub descricao: String,
    pub motivo: Option<String>,
    pub itens: i64,
    pub proposta_por: Option<String>,
    pub aprovada_por: Option<String>,
    pub criada_em: DateTime<Utc>,
    /// vigente, futura ou anterior.
    pub situacao: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MudancaDePrecoDoConvenio {
    pub tuss: String,
    pub descricao: String,
    /// Preço na versão de partida (nulo quando o exame entra agora).
    pub antes_centavos: Option<i64>,
    pub depois_centavos: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PropostaDeTabela {
    pub id: Uuid,
    pub codigo: String,
    pub contrato_id: Uuid,
    /// reajuste ou precos.
    pub tipo: String,
    pub reajuste_pontos_base: Option<i32>,
    pub base_versao: Option<i32>,
    pub vigente_desde: NaiveDate,
    pub descricao: String,
    pub documento: String,
    pub motivo: String,
    /// pendente, aprovada ou recusada.
    pub situacao: String,
    pub proposta_por: String,
    pub proposta_por_id: Uuid,
    pub proposta_em: DateTime<Utc>,
    pub decidido_por: Option<String>,
    pub decidido_em: Option<DateTime<Utc>>,
    pub motivo_decisao: Option<String>,
    pub itens: i64,
    /// Soma dos preços que já existiam, antes e depois (calculadas pelo servidor).
    pub total_antes_centavos: i64,
    pub total_depois_centavos: i64,
    /// As maiores mudanças em reais.
    pub maiores: Vec<MudancaDePrecoDoConvenio>,
    /// Versão da tabela criada pela aprovação.
    pub tabela_versao: Option<i32>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RegraDeAutorizacaoDoExame {
    pub tuss: String,
    pub nome: String,
    pub exige_senha: bool,
    pub validade_dias: Option<i32>,
    pub documentos: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RegrasDeAutorizacaoDoConvenio {
    pub codigo: String,
    pub versao_regras: i32,
    pub motivo: String,
    pub publicada_por: String,
    pub publicada_em: DateTime<Utc>,
    pub itens: Vec<RegraDeAutorizacaoDoExame>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RegraDaAuditoriaDoConvenio {
    pub regra: String,
    pub titulo: String,
    /// bloqueia, corrigir_antes ou atencao.
    pub efeito: String,
    pub origem: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ConjuntoDeAuditoriaDisponivel {
    pub versao: String,
    pub nome: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuditoriaDoConvenio {
    pub versao: String,
    pub nome: String,
    pub regras: Vec<RegraDaAuditoriaDoConvenio>,
    /// Conjuntos que o contrato pode usar (escolha no cadastro).
    pub disponiveis: Vec<ConjuntoDeAuditoriaDisponivel>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MudancaDoConvenio {
    pub codigo: String,
    /// cadastrar, alterar, desativar ou reativar.
    pub tipo: String,
    pub motivo: String,
    pub versao_do_contrato: i32,
    pub antes: Option<TermosDoConvenio>,
    pub depois: TermosDoConvenio,
    pub quem: String,
    pub em: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ConvenioDetalhe {
    pub id: Uuid,
    pub codigo: String,
    pub operadora: String,
    pub registro_ans: String,
    pub situacao: String,
    /// Versão do cadastro; a mudança informa a versão que viu (G2).
    pub versao: i32,
    pub termos: TermosDoConvenio,
    /// Versões da tabela, da mais nova para a mais antiga.
    pub tabelas: Vec<VersaoDaTabelaDoConvenio>,
    pub proposta_pendente: Option<PropostaDeTabela>,
    pub regras_autorizacao: Option<RegrasDeAutorizacaoDoConvenio>,
    pub auditoria: AuditoriaDoConvenio,
    pub historico: Vec<MudancaDoConvenio>,
}
