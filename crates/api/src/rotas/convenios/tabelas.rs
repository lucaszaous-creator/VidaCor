//! Tabela do convênio: preços de uma versão, prévia do reajuste, proposta (faturamento) e decisão
//! (gerência, outra pessoa, com reautenticação). Só a aprovação cria a versão nova.

use super::contratos::motivo_valido;
use super::{nao_encontrado, proposta, MudancaDePrecoDoConvenio, PropostaDeTabela};
use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, HashSet};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::novo_id;
use vidacor_dominio_faturamento::tabela::{self as regra, ErroDoReajuste, PRECO_MAXIMO_CENTAVOS};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PrecoDoConvenio {
    pub tuss: String,
    pub descricao: String,
    pub valor_centavos: i64,
    /// Preço na versão anterior (nulo quando o exame entrou nesta versão).
    pub valor_anterior_centavos: Option<i64>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PrecosDoConvenio {
    pub versao: i32,
    pub vigente_desde: NaiveDate,
    pub descricao: String,
    pub itens: Vec<PrecoDoConvenio>,
}

#[derive(Deserialize, IntoParams)]
pub struct FiltroDePrecos {
    /// Versão da tabela (sem ela, a vigente hoje; sem vigente, a mais nova).
    pub versao: Option<i32>,
}

#[derive(Deserialize, IntoParams)]
pub struct PedidoDePrevia {
    /// Reajuste em pontos base (520 = 5,20%).
    pub pontos_base: i32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PreviaDeReajuste {
    pub base_versao: i32,
    pub pontos_base: i32,
    pub afetados: i64,
    pub total_antes_centavos: i64,
    pub total_depois_centavos: i64,
    pub maiores: Vec<MudancaDePrecoDoConvenio>,
}

#[derive(Deserialize, ToSchema)]
pub struct PrecoPropostoDoConvenio {
    pub tuss: String,
    pub valor_centavos: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct NovaPropostaDeTabela {
    /// Primeiro dia da versão nova: depois de hoje e depois do início da última versão.
    pub vigente_desde: NaiveDate,
    pub descricao: String,
    /// Documento que sustenta a mudança (ex.: "Carta de reajuste 2027").
    pub documento: String,
    pub motivo: String,
    /// Reajuste em lote sobre a última versão, em pontos base; ou...
    pub reajuste_pontos_base: Option<i32>,
    /// ...a tabela inteira com os preços negociados.
    pub precos: Option<Vec<PrecoPropostoDoConvenio>>,
}

struct Base {
    id: Uuid,
    versao: i32,
    precos: Vec<(String, String, i64)>,
}

/// Última versão da tabela do contrato (a de início mais recente), com os preços.
async fn ultima(tx: &mut sqlx::PgConnection, contrato: Uuid) -> Result<Option<Base>, ErroApi> {
    let Some(t) = sqlx::query!(
        "SELECT id, versao FROM faturamento.tabela WHERE contrato_id = $1 ORDER BY vigente_desde DESC, versao DESC LIMIT 1",
        contrato
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(None);
    };
    let precos = sqlx::query!(
        "SELECT tuss, descricao, valor_centavos FROM faturamento.preco WHERE tabela_id = $1 ORDER BY tuss",
        t.id
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|p| (p.tuss, p.descricao, p.valor_centavos))
    .collect();
    Ok(Some(Base {
        id: t.id,
        versao: t.versao,
        precos,
    }))
}

fn erro_do_reajuste(e: &ErroDoReajuste, nada: &str) -> ErroApi {
    match e {
        ErroDoReajuste::PercentualForaDoLimite => ErroApi::invalido(
            "reajuste-invalido",
            &format!("O reajuste em lote vai de 0,01% a 30%. {nada} Para reduzir preço ou mudar mais, proponha a tabela com os preços."),
        ),
        ErroDoReajuste::TabelaVazia => ErroApi::invalido(
            "sem-tabela",
            &format!("O convênio ainda não tem tabela para reajustar. {nada} Proponha a primeira tabela com os preços."),
        ),
        ErroDoReajuste::PrecoAcimaDoMaximo(t) => ErroApi::invalido(
            "preco-invalido",
            &format!("O preço do exame {t} passaria de R$ 100.000,00. {nada} Confira a tabela."),
        ),
    }
}

async fn contrato_ativo(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    id: Uuid,
) -> Result<bool, ErroApi> {
    sqlx::query_scalar!(
        "SELECT ativo FROM faturamento.contrato WHERE id = $1 AND unidade_id = $2",
        id,
        unidade
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(nao_encontrado)
}

/// Preços de uma versão da tabela do convênio, com o preço da versão anterior (faturamento e gerência).
#[utoipa::path(get, operation_id = "precos_do_convenio", path = "/v1/convenios/{id}/precos", tag = "convenios",
    params(("id" = Uuid, Path), FiltroDePrecos),
    responses((status = 200, body = PrecosDoConvenio), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn precos(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Query(f): Query<FiltroDePrecos>,
) -> Result<Json<PrecosDoConvenio>, ErroApi> {
    usuario.exigir(Ato::ConveniosVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    contrato_ativo(&mut tx, usuario.unidade_id, id).await?;
    let t = sqlx::query!(
        "SELECT id, versao, vigente_desde, descricao FROM faturamento.tabela
          WHERE contrato_id = $1 AND ($2::int IS NULL OR versao = $2)
          ORDER BY ($2::int IS NULL AND vigente_desde > $3), vigente_desde DESC, versao DESC LIMIT 1",
        id,
        f.versao,
        tempo::hoje_local()
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "tabela-nao-encontrada",
            "Esta versão da tabela não existe neste convênio.",
        )
    })?;
    let itens = sqlx::query_as!(
        PrecoDoConvenio,
        r#"SELECT p.tuss, p.descricao, p.valor_centavos,
                  (SELECT a.valor_centavos FROM faturamento.preco a JOIN faturamento.tabela ta ON ta.id = a.tabela_id
                    WHERE ta.contrato_id = $2 AND ta.vigente_desde < $3 AND a.tuss = p.tuss
                    ORDER BY ta.vigente_desde DESC LIMIT 1) AS valor_anterior_centavos
             FROM faturamento.preco p WHERE p.tabela_id = $1 ORDER BY p.descricao, p.tuss LIMIT 2000"#,
        t.id,
        id,
        t.vigente_desde
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(PrecosDoConvenio {
        versao: t.versao,
        vigente_desde: t.vigente_desde,
        descricao: t.descricao,
        itens,
    }))
}

/// Prévia do reajuste em lote sobre a última versão, calculada pelo servidor (nada é gravado).
#[utoipa::path(get, operation_id = "previa_de_reajuste_do_convenio", path = "/v1/convenios/{id}/previa-de-reajuste", tag = "convenios",
    params(("id" = Uuid, Path), PedidoDePrevia),
    responses((status = 200, body = PreviaDeReajuste), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn previa(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Query(q): Query<PedidoDePrevia>,
) -> Result<Json<PreviaDeReajuste>, ErroApi> {
    usuario.exigir(Ato::ConveniosVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    contrato_ativo(&mut tx, usuario.unidade_id, id).await?;
    let nada = "Nada foi calculado.";
    let base = ultima(&mut tx, id)
        .await?
        .ok_or_else(|| erro_do_reajuste(&ErroDoReajuste::TabelaVazia, nada))?;
    tx.commit().await?;
    let pares: Vec<(String, i64)> = base
        .precos
        .iter()
        .map(|(t, _, v)| (t.clone(), *v))
        .collect();
    let p = regra::previa(&pares, q.pontos_base).map_err(|e| erro_do_reajuste(&e, nada))?;
    let nomes: BTreeMap<&str, &str> = base
        .precos
        .iter()
        .map(|(t, d, _)| (t.as_str(), d.as_str()))
        .collect();
    Ok(Json(PreviaDeReajuste {
        base_versao: base.versao,
        pontos_base: q.pontos_base,
        afetados: i64::try_from(p.precos.len()).unwrap_or(i64::MAX),
        total_antes_centavos: p.total_antes_centavos,
        total_depois_centavos: p.total_depois_centavos,
        maiores: p
            .maiores
            .iter()
            .map(|m| MudancaDePrecoDoConvenio {
                descricao: nomes
                    .get(m.tuss.as_str())
                    .copied()
                    .unwrap_or_default()
                    .to_owned(),
                tuss: m.tuss.clone(),
                antes_centavos: Some(m.antes_centavos),
                depois_centavos: m.depois_centavos,
            })
            .collect(),
    }))
}

fn texto_valido(
    t: &str,
    min: usize,
    max: usize,
    campo: &str,
    nada: &str,
) -> Result<String, ErroApi> {
    let t = t.trim();
    if (min..=max).contains(&t.chars().count()) {
        Ok(t.to_owned())
    } else {
        Err(ErroApi::invalido(
            "proposta-invalida",
            &format!("{campo}: de {min} a {max} letras. {nada}"),
        ))
    }
}

/// Propõe a tabela nova (faturamento; `Idempotency-Key`): reajuste em lote sobre a última versão ou a
/// tabela inteira com preços. O servidor calcula e grava os preços; a versão só nasce na aprovação.
/// Uma proposta pendente por convênio (índice único, G2).
#[utoipa::path(post, operation_id = "propor_tabela_do_convenio", path = "/v1/convenios/{id}/propostas", tag = "convenios",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = NovaPropostaDeTabela,
    responses((status = 201, body = PropostaDeTabela), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn propor(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovaPropostaDeTabela>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::ConveniosAdministrar)?;
    let nada = "Nada foi proposto.";
    let motivo = motivo_valido(&n.motivo)?;
    let descricao = texto_valido(&n.descricao, 3, 120, "Nome da versão", nada)?;
    let documento = texto_valido(&n.documento, 3, 120, "Documento", nada)?;
    let hoje = tempo::hoje_local();
    if n.vigente_desde <= hoje || n.vigente_desde > hoje + Duration::days(366) {
        return Err(ErroApi::invalido(
            "vigencia-invalida",
            &format!("A versão nova começa a partir de amanhã e em até um ano: a tabela vigente não muda no meio do dia. {nada}"),
        ));
    }
    if n.reajuste_pontos_base.is_some() == n.precos.is_some() {
        return Err(ErroApi::invalido(
            "proposta-invalida",
            &format!("Escolha reajuste em lote ou tabela com preços, não os dois. {nada}"),
        ));
    }
    if let Some(precos) = &n.precos {
        if precos.is_empty() || precos.len() > 1000 {
            return Err(ErroApi::invalido(
                "proposta-invalida",
                &format!("A tabela tem de 1 a 1000 preços. {nada}"),
            ));
        }
        let mut vistos = HashSet::new();
        for p in precos {
            if p.tuss.len() != 8
                || !p.tuss.bytes().all(|b| b.is_ascii_digit())
                || !vistos.insert(p.tuss.as_str())
            {
                return Err(ErroApi::invalido(
                    "proposta-invalida",
                    &format!("Cada exame aparece uma vez, com código TUSS de 8 algarismos. {nada}"),
                ));
            }
            if !(1..=PRECO_MAXIMO_CENTAVOS).contains(&p.valor_centavos) {
                return Err(ErroApi::invalido(
                    "preco-invalido",
                    &format!(
                        "Preço do exame {} fora do aceito (de R$ 0,01 a R$ 100.000,00). {nada}",
                        p.tuss
                    ),
                ));
            }
        }
    }
    let rota = format!("POST /v1/convenios/{id}/propostas");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    if !contrato_ativo(&mut tx, usuario.unidade_id, id).await? {
        return Err(ErroApi::conflito(
            "convenio-inativo",
            &format!(
                "Convênio desativado não recebe tabela nova. {nada} Reative o convênio antes."
            ),
        ));
    }
    let ultima_vigencia = sqlx::query_scalar!(
        "SELECT max(vigente_desde) FROM faturamento.tabela WHERE contrato_id = $1",
        id
    )
    .fetch_one(&mut *tx)
    .await?;
    if let Some(d) = ultima_vigencia.filter(|d| n.vigente_desde <= *d) {
        return Err(ErroApi::invalido(
            "vigencia-invalida",
            &format!(
                "A versão nova precisa começar depois de {} (início da última versão). {nada}",
                d.format("%d/%m/%Y")
            ),
        ));
    }
    let base = ultima(&mut tx, id).await?;
    let anteriores: BTreeMap<String, i64> = base
        .as_ref()
        .map(|b| b.precos.iter().map(|(t, _, v)| (t.clone(), *v)).collect())
        .unwrap_or_default();
    // (tuss, descrição, valor, anterior)
    let itens: Vec<(String, String, i64, Option<i64>)> = match (&n.reajuste_pontos_base, &n.precos)
    {
        (Some(pb), _) => {
            let b = base
                .as_ref()
                .ok_or_else(|| erro_do_reajuste(&ErroDoReajuste::TabelaVazia, nada))?;
            let pares: Vec<(String, i64)> =
                b.precos.iter().map(|(t, _, v)| (t.clone(), *v)).collect();
            let p = regra::previa(&pares, *pb).map_err(|e| erro_do_reajuste(&e, nada))?;
            p.precos
                .into_iter()
                .zip(b.precos.iter())
                .map(|(r, (_, d, _))| {
                    (r.tuss, d.clone(), r.depois_centavos, Some(r.antes_centavos))
                })
                .collect()
        }
        (None, Some(precos)) => {
            let tuss: Vec<String> = precos.iter().map(|p| p.tuss.clone()).collect();
            let nomes: BTreeMap<String, String> = sqlx::query!(
                "SELECT tuss, nome FROM agenda.procedimento WHERE tuss = ANY($1) AND ativo",
                &tuss
            )
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|p| (p.tuss, p.nome))
            .collect();
            let mut v = Vec::with_capacity(precos.len());
            for p in precos {
                let nome = nomes.get(&p.tuss).ok_or_else(|| {
                    ErroApi::invalido(
                        "procedimento-fora-do-catalogo",
                        &format!("O exame {} não está ativo no catálogo da clínica. {nada} Confira a lista.", p.tuss),
                    )
                })?;
                v.push((
                    p.tuss.clone(),
                    nome.clone(),
                    p.valor_centavos,
                    anteriores.get(&p.tuss).copied(),
                ));
            }
            v
        }
        (None, None) => return Err(ErroApi::interno("proposta sem tipo")),
    };
    let pid = novo_id();
    let codigo = proximo(&mut tx, "PTB").await?;
    let tipo = if n.reajuste_pontos_base.is_some() {
        "reajuste"
    } else {
        "precos"
    };
    sqlx::query!(
        "INSERT INTO faturamento.tabela_proposta (id, codigo, contrato_id, tipo, reajuste_pontos_base, base_tabela_id, vigente_desde,
                descricao, documento, motivo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'convenios', $12, $13)",
        pid, codigo.como_str(), id, tipo, n.reajuste_pontos_base,
        if n.reajuste_pontos_base.is_some() { base.as_ref().map(|b| b.id) } else { None },
        n.vigente_desde, descricao, documento, motivo, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito(
            "proposta-pendente",
            &format!("Este convênio já tem proposta de tabela aguardando aprovação. {nada} Peça a decisão da gerência antes de propor outra."),
        ),
        _ => ErroApi::from(e),
    })?;
    let (tuss, descricoes, valores, anteriores): (
        Vec<String>,
        Vec<String>,
        Vec<i64>,
        Vec<Option<i64>>,
    ) = itens
        .into_iter()
        .fold((vec![], vec![], vec![], vec![]), |mut a, (t, d, v, x)| {
            a.0.push(t);
            a.1.push(d);
            a.2.push(v);
            a.3.push(x);
            a
        });
    sqlx::query!(
        "INSERT INTO faturamento.tabela_proposta_preco (proposta_id, tuss, descricao, valor_centavos, valor_anterior_centavos, unidade_id)
         SELECT $1, t, d, v, x, $6 FROM unnest($2::text[], $3::text[], $4::bigint[], $5::bigint[]) AS u(t, d, v, x)",
        pid, &tuss, &descricoes, &valores, &anteriores as &[Option<i64>], usuario.unidade_id
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "convenios", tipo: "tabela_proposta", registro_id: pid, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "contrato_id": id, "tipo": tipo, "reajuste_pontos_base": n.reajuste_pontos_base, "vigente_desde": n.vigente_desde,
                       "itens": tuss.len(), "motivo": motivo, "documento": documento }),
        criado_por: Some(usuario.id), origem: "convenios", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let corpo = serde_json::to_value(
        proposta(&mut tx, pid)
            .await?
            .ok_or_else(|| ErroApi::interno("proposta sumiu"))?,
    )
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}
