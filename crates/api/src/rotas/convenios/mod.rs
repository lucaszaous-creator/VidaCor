//! Convênios e tabelas (Convenios-Tabelas.dc.html; docs/04 Cadastros base; docs/21 § 3.9). O convênio
//! é o contrato do faturamento TISS (`faturamento.contrato`) e a tabela negociada é a mesma que a
//! conferência usa pela data do atendimento (`faturamento.tabela`), versionada. O faturamento cadastra
//! o contrato, as regras de autorização e propõe a tabela nova (reajuste calculado pelo servidor, G1);
//! a gerência aprova, e só então nasce a versão (segunda pessoa, também no banco). Nada é apagado.
//! DEC-2026-11-09-110 a 114.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::Json;
use chrono::{Duration, NaiveDate};
use uuid::Uuid;
use vidacor_dominio_faturamento::auditoria::{self, CONJUNTOS};

pub mod contratos;
pub mod decisao;
pub mod regras;
pub mod tabelas;
mod tipos;

pub use tipos::*;

/// Contrato que vence em até tantos dias aparece como "renovar" (a confirmar com o faturamento).
pub const DIAS_PARA_RENOVAR: i64 = 60;

pub(crate) fn situacao_do_convenio(ativo: bool, fim: Option<NaiveDate>, hoje: NaiveDate) -> String {
    match (ativo, fim) {
        (false, _) => "inativo",
        (true, None) => "sem_vigencia",
        (true, Some(f)) if f < hoje => "vencido",
        (true, Some(f)) if f <= hoje + Duration::days(DIAS_PARA_RENOVAR) => "renovar",
        _ => "vigente",
    }
    .to_owned()
}

pub(crate) fn nao_encontrado() -> ErroApi {
    ErroApi::nao_encontrado(
        "convenio-nao-encontrado",
        "Convênio não encontrado nesta unidade. Nada foi alterado.",
    )
}

pub(crate) fn auditoria_de(versao: &str) -> AuditoriaDoConvenio {
    let c = auditoria::conjunto(versao).unwrap_or_else(auditoria::conjunto_padrao);
    AuditoriaDoConvenio {
        versao: c.versao.to_owned(),
        nome: c.nome.to_owned(),
        regras: c
            .regras
            .iter()
            .map(|r| RegraDaAuditoriaDoConvenio {
                regra: r.codigo.codigo().to_owned(),
                titulo: r.codigo.titulo().to_owned(),
                efeito: r.efeito.codigo().to_owned(),
                origem: r.origem.to_owned(),
            })
            .collect(),
        disponiveis: CONJUNTOS
            .iter()
            .map(|c| ConjuntoDeAuditoriaDisponivel {
                versao: c.versao.to_owned(),
                nome: c.nome.to_owned(),
            })
            .collect(),
    }
}

/// Proposta de tabela com os totais e as maiores mudanças.
pub(crate) async fn proposta(
    tx: &mut sqlx::PgConnection,
    id: Uuid,
) -> Result<Option<PropostaDeTabela>, ErroApi> {
    let Some(p) = sqlx::query!(
        r#"SELECT p.id, p.codigo, p.contrato_id, p.tipo, p.reajuste_pontos_base, b.versao AS "base_versao?", p.vigente_desde,
                  p.descricao, p.documento, p.motivo, p.situacao, u.nome AS proposta_por, p.criado_por, p.criado_em,
                  d.nome AS "decidido_por?", p.decidido_em, p.motivo_decisao, t.versao AS "tabela_versao?",
                  (SELECT count(*) FROM faturamento.tabela_proposta_preco x WHERE x.proposta_id = p.id) AS "itens!",
                  (SELECT coalesce(sum(x.valor_anterior_centavos), 0)::bigint FROM faturamento.tabela_proposta_preco x
                    WHERE x.proposta_id = p.id AND x.valor_anterior_centavos IS NOT NULL) AS "antes!",
                  (SELECT coalesce(sum(x.valor_centavos), 0)::bigint FROM faturamento.tabela_proposta_preco x
                    WHERE x.proposta_id = p.id AND x.valor_anterior_centavos IS NOT NULL) AS "depois!"
             FROM faturamento.tabela_proposta p
             JOIN identidade.usuario u ON u.id = p.criado_por
             LEFT JOIN identidade.usuario d ON d.id = p.decidido_por
             LEFT JOIN faturamento.tabela b ON b.id = p.base_tabela_id
             LEFT JOIN faturamento.tabela t ON t.proposta_id = p.id
            WHERE p.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(None);
    };
    let maiores = sqlx::query_as!(
        MudancaDePrecoDoConvenio,
        r#"SELECT tuss, descricao, valor_anterior_centavos AS antes_centavos, valor_centavos AS depois_centavos
             FROM faturamento.tabela_proposta_preco WHERE proposta_id = $1
            ORDER BY (valor_anterior_centavos IS NULL), (valor_centavos - coalesce(valor_anterior_centavos, 0)) DESC, tuss
            LIMIT 3"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(Some(PropostaDeTabela {
        id: p.id,
        codigo: p.codigo,
        contrato_id: p.contrato_id,
        tipo: p.tipo,
        reajuste_pontos_base: p.reajuste_pontos_base,
        base_versao: p.base_versao,
        vigente_desde: p.vigente_desde,
        descricao: p.descricao,
        documento: p.documento,
        motivo: p.motivo,
        situacao: p.situacao,
        proposta_por: p.proposta_por,
        proposta_por_id: p.criado_por,
        proposta_em: p.criado_em,
        decidido_por: p.decidido_por,
        decidido_em: p.decidido_em,
        motivo_decisao: p.motivo_decisao,
        itens: p.itens,
        total_antes_centavos: p.antes,
        total_depois_centavos: p.depois,
        maiores,
        tabela_versao: p.tabela_versao,
    }))
}

/// Versões da tabela do contrato (mais nova primeiro), com a situação de hoje.
async fn versoes(
    tx: &mut sqlx::PgConnection,
    contrato: Uuid,
) -> Result<Vec<VersaoDaTabelaDoConvenio>, ErroApi> {
    let hoje = tempo::hoje_local();
    let linhas = sqlx::query!(
        r#"SELECT t.id, t.codigo, t.versao, t.vigente_desde, t.descricao, t.motivo, t.criado_em,
                  u.nome AS "proposta_por?", a.nome AS "aprovada_por?",
                  (SELECT count(*) FROM faturamento.preco x WHERE x.tabela_id = t.id) AS "itens!"
             FROM faturamento.tabela t
             LEFT JOIN faturamento.tabela_proposta p ON p.id = t.proposta_id
             LEFT JOIN identidade.usuario u ON u.id = p.criado_por
             LEFT JOIN identidade.usuario a ON a.id = p.decidido_por
            WHERE t.contrato_id = $1 ORDER BY t.vigente_desde DESC, t.versao DESC LIMIT 100"#,
        contrato
    )
    .fetch_all(&mut *tx)
    .await?;
    // Mesma regra da conferência: vale a de início mais recente já iniciada.
    let vigente = linhas
        .iter()
        .find(|t| t.vigente_desde <= hoje)
        .map(|t| t.id);
    Ok(linhas
        .into_iter()
        .map(|t| VersaoDaTabelaDoConvenio {
            situacao: if Some(t.id) == vigente {
                "vigente"
            } else if t.vigente_desde > hoje {
                "futura"
            } else {
                "anterior"
            }
            .to_owned(),
            id: t.id,
            codigo: t.codigo,
            versao: t.versao,
            vigente_desde: t.vigente_desde,
            descricao: t.descricao,
            motivo: t.motivo,
            itens: t.itens,
            proposta_por: t.proposta_por,
            aprovada_por: t.aprovada_por,
            criada_em: t.criado_em,
        })
        .collect())
}

pub(crate) async fn detalhe(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    id: Uuid,
) -> Result<ConvenioDetalhe, ErroApi> {
    let k = sqlx::query!(
        r#"SELECT id, codigo, operadora, registro_ans, versao, codigo_prestador, nome_prestador, cnes, regra_versao,
                  prazo_envio_dias, prazo_recurso_dias, prazo_pagamento_dias, vigencia_inicio, vigencia_fim, reajuste,
                  contato_nome, contato_telefone, contato_email, autorizacao_por_webservice, ativo
             FROM faturamento.contrato WHERE id = $1 AND unidade_id = $2"#,
        id,
        unidade
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(nao_encontrado)?;
    let termos = TermosDoConvenio {
        codigo_prestador: k.codigo_prestador,
        nome_prestador: k.nome_prestador,
        cnes: k.cnes,
        regra_versao: k.regra_versao,
        prazo_envio_dias: k.prazo_envio_dias,
        prazo_recurso_dias: k.prazo_recurso_dias,
        prazo_pagamento_dias: k.prazo_pagamento_dias,
        vigencia_inicio: k.vigencia_inicio,
        vigencia_fim: k.vigencia_fim,
        reajuste: k.reajuste,
        contato_nome: k.contato_nome,
        contato_telefone: k.contato_telefone,
        contato_email: k.contato_email,
        autorizacao_por_webservice: k.autorizacao_por_webservice,
        ativo: k.ativo,
    };
    let pendente = sqlx::query_scalar!(
        "SELECT id FROM faturamento.tabela_proposta WHERE contrato_id = $1 AND situacao = 'pendente'",
        id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let proposta_pendente = match pendente {
        Some(p) => proposta(tx, p).await?,
        None => None,
    };
    let historico = sqlx::query!(
        r#"SELECT m.codigo, m.tipo, m.motivo, m.versao_do_contrato, m.antes, m.depois, u.nome AS quem, m.criado_em
             FROM faturamento.contrato_mudanca m JOIN identidade.usuario u ON u.id = m.criado_por
            WHERE m.contrato_id = $1 ORDER BY m.versao_do_contrato DESC LIMIT 100"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|m| {
        Ok(MudancaDoConvenio {
            codigo: m.codigo,
            tipo: m.tipo,
            motivo: m.motivo,
            versao_do_contrato: m.versao_do_contrato,
            antes: m
                .antes
                .map(serde_json::from_value)
                .transpose()
                .map_err(ErroApi::interno)?,
            depois: serde_json::from_value(m.depois).map_err(ErroApi::interno)?,
            quem: m.quem,
            em: m.criado_em,
        })
    })
    .collect::<Result<Vec<_>, ErroApi>>()?;
    Ok(ConvenioDetalhe {
        situacao: situacao_do_convenio(k.ativo, termos.vigencia_fim, tempo::hoje_local()),
        id: k.id,
        codigo: k.codigo,
        operadora: k.operadora,
        registro_ans: k.registro_ans,
        versao: k.versao,
        auditoria: auditoria_de(&termos.regra_versao),
        termos,
        tabelas: versoes(tx, id).await?,
        proposta_pendente,
        regras_autorizacao: regras::vigentes(tx, id).await?,
        historico,
    })
}

/// Convênios da unidade (ativos e inativos), com a situação da vigência e a tabela de hoje.
#[utoipa::path(get, operation_id = "listar_convenios", path = "/v1/convenios", tag = "convenios",
    responses((status = 200, body = Vec<ConvenioResumo>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<ConvenioResumo>>, ErroApi> {
    usuario.exigir(Ato::ConveniosVer)?;
    let hoje = tempo::hoje_local();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let linhas = sqlx::query!(
        r#"SELECT k.id, k.codigo, k.operadora, k.registro_ans, k.ativo, k.vigencia_fim,
                  (SELECT t.versao FROM faturamento.tabela t WHERE t.contrato_id = k.id AND t.vigente_desde <= $2
                    ORDER BY t.vigente_desde DESC, t.versao DESC LIMIT 1) AS tabela_versao,
                  EXISTS (SELECT 1 FROM faturamento.tabela_proposta p WHERE p.contrato_id = k.id AND p.situacao = 'pendente') AS "pendente!"
             FROM faturamento.contrato k WHERE k.unidade_id = $1
            ORDER BY k.ativo DESC, k.operadora LIMIT 200"#,
        usuario.unidade_id,
        hoje
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        linhas
            .into_iter()
            .map(|k| ConvenioResumo {
                situacao: situacao_do_convenio(k.ativo, k.vigencia_fim, hoje),
                id: k.id,
                codigo: k.codigo,
                operadora: k.operadora,
                registro_ans: k.registro_ans,
                vigencia_fim: k.vigencia_fim,
                tabela_versao: k.tabela_versao,
                proposta_pendente: k.pendente,
            })
            .collect(),
    ))
}

/// Convênio com termos, versões da tabela, proposta pendente, regras de autorização, conjunto da
/// auditoria e histórico (faturamento e gerência).
#[utoipa::path(get, operation_id = "obter_convenio", path = "/v1/convenios/{id}", tag = "convenios",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = ConvenioDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn obter(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<ConvenioDetalhe>, ErroApi> {
    usuario.exigir(Ato::ConveniosVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = detalhe(&mut tx, usuario.unidade_id, id).await?;
    tx.commit().await?;
    Ok(Json(r))
}
