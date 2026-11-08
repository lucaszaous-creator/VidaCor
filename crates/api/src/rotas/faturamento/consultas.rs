//! Consultas do faturamento (faturista e gerente): guias com pendências, detalhe com versões,
//! painel da auditoria e contratos. O faturista vê N1 e N2 e só os códigos TUSS (docs/18 § 4).

use super::contrato::{
    ConjuntoDeRegras, ContaDetalhe, ContaPendente, ContaResumo, Contrato, GrupoDePendencias,
    ItemDaConta, PacienteDaConta, PainelAuditoria, PendenciaDaConta, RegraDoConjunto,
    ResumoAuditoria, Solicitante, VersaoDaConta,
};
use super::{conta_nao_encontrada, glosas};
use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use sqlx::PgConnection;
use std::collections::BTreeMap;
use utoipa::IntoParams;
use uuid::Uuid;
use vidacor_dominio_faturamento::CONJUNTOS;

/// Guias por página (SEC-DIS: limite de página).
const LIMITE_LISTA: i64 = 300;

#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct FiltroContas {
    /// Situação da conta (ex.: `com_pendencia`).
    pub situacao: Option<String>,
    /// Contas de um lote.
    pub lote_id: Option<Uuid>,
}

/// Resumos de contas (por filtro opcional de situação, lote ou id).
pub(crate) async fn resumos(
    conexao: &mut PgConnection,
    situacao: Option<&str>,
    lote: Option<Uuid>,
    conta: Option<Uuid>,
) -> Result<Vec<ContaResumo>, sqlx::Error> {
    let linhas = sqlx::query!(
        r#"SELECT c.id, c.codigo, c.paciente_nome, c.paciente_nascimento, c.operadora, c.numero_guia_operadora,
                  c.valor_total_centavos, c.situacao, c.data_execucao, c.regra_versao,
                  (c.data_execucao + k.prazo_envio_dias) AS enviar_ate,
                  (SELECT coalesce(array_agg(i.tuss || ' ' || i.descricao ORDER BY i.sequencial), '{}')
                     FROM faturamento.conta_item i WHERE i.conta_id = c.id) AS "procedimentos!",
                  (SELECT count(*) FROM faturamento.pendencia p WHERE p.conta_id = c.id AND p.situacao = 'aberta'
                      AND p.efeito <> 'atencao') AS "impedem!",
                  (SELECT count(*) FROM faturamento.pendencia p WHERE p.conta_id = c.id AND p.situacao = 'aberta'
                      AND p.efeito = 'atencao') AS "atencao!",
                  l.codigo AS "lote_codigo?"
             FROM faturamento.conta c
             LEFT JOIN faturamento.contrato k ON k.id = c.contrato_id
             LEFT JOIN faturamento.lote l ON l.id = c.lote_id
            WHERE ($1::text IS NULL OR c.situacao = $1) AND ($2::uuid IS NULL OR c.lote_id = $2)
              AND ($3::uuid IS NULL OR c.id = $3)
            ORDER BY CASE c.situacao WHEN 'com_pendencia' THEN 0 WHEN 'pronta' THEN 1 ELSE 2 END,
                     c.data_execucao, c.codigo
            LIMIT $4"#,
        situacao,
        lote,
        conta,
        LIMITE_LISTA
    )
    .fetch_all(conexao)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| ContaResumo {
            id: l.id,
            codigo: l.codigo,
            paciente: PacienteDaConta {
                nome: l.paciente_nome,
                nascimento: l.paciente_nascimento,
            },
            operadora: l.operadora,
            numero_guia_operadora: l.numero_guia_operadora,
            procedimentos: l.procedimentos,
            valor_total_centavos: l.valor_total_centavos,
            situacao: l.situacao,
            data_execucao: l.data_execucao,
            enviar_ate: l.enviar_ate,
            pendencias_que_impedem: l.impedem,
            pendencias_de_atencao: l.atencao,
            regra_versao: l.regra_versao,
            lote_codigo: l.lote_codigo,
        })
        .collect())
}

/// Detalhe da conta com itens, pendências abertas, versões anteriores e glosas.
///
/// # Errors
/// Falha de banco.
pub(crate) async fn detalhe_da_conta(
    conexao: &mut PgConnection,
    id: Uuid,
) -> Result<Option<ContaDetalhe>, ErroApi> {
    let Some(resumo) = resumos(conexao, None, None, Some(id)).await?.pop() else {
        return Ok(None);
    };
    let c = sqlx::query!(
        "SELECT carteirinha, senha, validade_senha, solicitante_nome, solicitante_conselho, solicitante_numero,
                solicitante_uf, solicitante_cbos, cid, assinatura_beneficiario, versao
           FROM faturamento.conta WHERE id = $1",
        id
    )
    .fetch_one(&mut *conexao)
    .await?;
    let itens = sqlx::query_as!(
        ItemDaConta,
        "SELECT sequencial, tuss, descricao, quantidade, valor_unitario_centavos
           FROM faturamento.conta_item WHERE conta_id = $1 ORDER BY sequencial",
        id
    )
    .fetch_all(&mut *conexao)
    .await?;
    let pendencias = sqlx::query_as!(
        PendenciaDaConta,
        r#"SELECT regra, regra_versao, efeito, origem_regra AS origem, titulo, detalhe, o_que_fazer
             FROM faturamento.pendencia WHERE conta_id = $1 AND situacao = 'aberta'
            ORDER BY CASE efeito WHEN 'bloqueia' THEN 0 WHEN 'corrigir_antes' THEN 1 ELSE 2 END, criado_em"#,
        id
    )
    .fetch_all(&mut *conexao)
    .await?;
    let historico = sqlx::query_as!(
        VersaoDaConta,
        "SELECT versao, motivo, criado_em FROM faturamento.conta_historico WHERE conta_id = $1 ORDER BY versao DESC",
        id
    )
    .fetch_all(&mut *conexao)
    .await?;
    let glosas = glosas::carregar(conexao, Some(id), None).await?;
    Ok(Some(ContaDetalhe {
        resumo,
        carteirinha: c.carteirinha,
        senha: c.senha,
        validade_senha: c.validade_senha,
        solicitante: Solicitante {
            nome: c.solicitante_nome,
            conselho: c.solicitante_conselho,
            numero: c.solicitante_numero,
            uf: c.solicitante_uf,
            cbos: c.solicitante_cbos,
        },
        cid: c.cid,
        assinatura_beneficiario: c.assinatura_beneficiario,
        itens,
        pendencias,
        historico,
        versao: c.versao,
        glosas,
    }))
}

/// Guias do faturamento com situação, prazo e pendências (faturista e gerente).
#[utoipa::path(get, operation_id = "listar_contas_faturamento", path = "/v1/faturamento/contas", tag = "faturamento",
    params(FiltroContas),
    responses((status = 200, body = Vec<ContaResumo>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroContas>,
) -> Result<Json<Vec<ContaResumo>>, ErroApi> {
    usuario.exigir(Ato::FaturamentoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let lista = resumos(&mut tx, f.situacao.as_deref(), f.lote_id, None).await?;
    tx.commit().await?;
    Ok(Json(lista))
}

/// Detalhe de uma guia do faturamento.
#[utoipa::path(get, operation_id = "obter_conta_faturamento", path = "/v1/faturamento/contas/{id}", tag = "faturamento",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = ContaDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn obter(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<ContaDetalhe>, ErroApi> {
    usuario.exigir(Ato::FaturamentoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let d = detalhe_da_conta(&mut tx, id)
        .await?
        .ok_or_else(conta_nao_encontrada)?;
    tx.commit().await?;
    Ok(Json(d))
}

/// Painel da auditoria: contas fora de lote por situação, pendências agrupadas por regra e os
/// conjuntos de regras com as operadoras que os usam.
#[utoipa::path(get, operation_id = "painel_auditoria_faturamento", path = "/v1/faturamento/auditoria", tag = "faturamento",
    responses((status = 200, body = PainelAuditoria), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn auditoria(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<PainelAuditoria>, ErroApi> {
    usuario.exigir(Ato::FaturamentoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = sqlx::query!(
        r#"SELECT count(*) FILTER (WHERE situacao IN ('pronta','com_pendencia')) AS "conferidas!",
                  count(*) FILTER (WHERE situacao = 'pronta') AS "prontas!",
                  count(*) FILTER (WHERE situacao = 'com_pendencia' AND NOT EXISTS (
                      SELECT 1 FROM faturamento.pendencia p WHERE p.conta_id = c.id AND p.situacao = 'aberta' AND p.efeito = 'bloqueia'))
                    AS "corrigir!",
                  count(*) FILTER (WHERE situacao = 'com_pendencia' AND EXISTS (
                      SELECT 1 FROM faturamento.pendencia p WHERE p.conta_id = c.id AND p.situacao = 'aberta' AND p.efeito = 'bloqueia'))
                    AS "bloqueadas!"
             FROM faturamento.conta c WHERE c.lote_id IS NULL"#
    )
    .fetch_one(&mut *tx)
    .await?;
    let linhas = sqlx::query!(
        r#"SELECT p.regra, p.regra_versao, p.efeito, p.origem_regra, p.titulo, p.detalhe, p.o_que_fazer,
                  c.id AS conta_id, c.codigo, c.paciente_nome, c.paciente_nascimento, c.operadora, c.numero_guia_operadora
             FROM faturamento.pendencia p JOIN faturamento.conta c ON c.id = p.conta_id
            WHERE p.situacao = 'aberta' AND c.lote_id IS NULL
            ORDER BY CASE p.efeito WHEN 'bloqueia' THEN 0 WHEN 'corrigir_antes' THEN 1 ELSE 2 END, p.regra, c.codigo
            LIMIT 2000"#
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut grupos: Vec<GrupoDePendencias> = Vec::new();
    for l in linhas {
        let pos = match grupos.iter().position(|g| g.regra == l.regra) {
            Some(i) => i,
            None => {
                grupos.push(GrupoDePendencias {
                    regra: l.regra.clone(),
                    titulo: l.titulo.clone(),
                    efeito: l.efeito.clone(),
                    origem: l.origem_regra.clone(),
                    o_que_fazer: l.o_que_fazer.clone(),
                    regra_versoes: Vec::new(),
                    contas: Vec::new(),
                });
                grupos.len() - 1
            }
        };
        let g = &mut grupos[pos];
        if !g.regra_versoes.contains(&l.regra_versao) {
            g.regra_versoes.push(l.regra_versao);
        }
        g.contas.push(ContaPendente {
            conta_id: l.conta_id,
            codigo: l.codigo,
            paciente: PacienteDaConta {
                nome: l.paciente_nome,
                nascimento: l.paciente_nascimento,
            },
            operadora: l.operadora,
            numero_guia_operadora: l.numero_guia_operadora,
            detalhe: l.detalhe,
        });
    }
    let mut por_versao: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for k in sqlx::query!(
        "SELECT operadora, regra_versao FROM faturamento.contrato WHERE ativo ORDER BY operadora"
    )
    .fetch_all(&mut *tx)
    .await?
    {
        por_versao
            .entry(k.regra_versao)
            .or_default()
            .push(k.operadora);
    }
    tx.commit().await?;
    let conjuntos = CONJUNTOS
        .iter()
        .map(|c| ConjuntoDeRegras {
            versao: c.versao.to_owned(),
            nome: c.nome.to_owned(),
            operadoras: por_versao.remove(c.versao).unwrap_or_default(),
            regras: c
                .regras
                .iter()
                .map(|r| RegraDoConjunto {
                    regra: r.codigo.codigo().to_owned(),
                    titulo: r.codigo.titulo().to_owned(),
                    efeito: r.efeito.codigo().to_owned(),
                    origem: r.origem.to_owned(),
                })
                .collect(),
        })
        .collect();
    Ok(Json(PainelAuditoria {
        resumo: ResumoAuditoria {
            conferidas: r.conferidas,
            prontas: r.prontas,
            corrigir_antes: r.corrigir,
            bloqueadas: r.bloqueadas,
        },
        grupos,
        conjuntos,
    }))
}

/// Contratos ativos da unidade, com as guias prontas para lote de cada um.
#[utoipa::path(get, operation_id = "listar_contratos_faturamento", path = "/v1/faturamento/contratos", tag = "faturamento",
    responses((status = 200, body = Vec<Contrato>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn contratos(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<Contrato>>, ErroApi> {
    usuario.exigir(Ato::FaturamentoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let lista = sqlx::query_as!(
        Contrato,
        r#"SELECT k.id, k.codigo, k.operadora, k.registro_ans, k.regra_versao, k.prazo_envio_dias, k.prazo_recurso_dias,
                  (SELECT count(*) FROM faturamento.conta c WHERE c.contrato_id = k.id AND c.situacao = 'pronta') AS "contas_prontas!"
             FROM faturamento.contrato k WHERE k.ativo ORDER BY k.operadora"#
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(lista))
}
