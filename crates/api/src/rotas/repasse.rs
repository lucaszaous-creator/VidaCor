//! Repasse médico com regra versionada (docs/25 § 3.3). Base: laudos assinados na competência ×
//! valor do procedimento na tabela de preços vigente na data; percentual pela versão da regra vigente
//! no início da competência. Gerar os títulos é idempotente: um por médico e competência (G2).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::rotas::financeiro::DUAS_APROVACOES_ACIMA_CENTAVOS;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::novo_id;

fn competencia(texto: &str) -> Result<(NaiveDate, NaiveDate), ErroApi> {
    let invalida = || {
        ErroApi::invalido(
            "competencia-invalida",
            "Use a competência no formato AAAA-MM.",
        )
    };
    let inicio =
        NaiveDate::parse_from_str(&format!("{texto}-01"), "%Y-%m-%d").map_err(|_| invalida())?;
    let fim = if inicio.month() == 12 {
        NaiveDate::from_ymd_opt(inicio.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(inicio.year(), inicio.month() + 1, 1)
    }
    .ok_or_else(invalida)?;
    Ok((inicio, fim))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RepasseDoMedico {
    pub medico_id: Uuid,
    pub medico: String,
    pub laudos: i64,
    pub base_centavos: i64,
    /// Nulo quando o médico não tem regra vigente (nada é calculado por suposição).
    pub percentual_pb: Option<i32>,
    pub regra_versao: Option<i32>,
    pub valor_centavos: Option<i64>,
    /// Laudos cujo procedimento não está na tabela de preços (não entram na base).
    pub sem_preco: i64,
}

#[derive(Deserialize, IntoParams)]
pub struct Competencia {
    /// AAAA-MM.
    pub competencia: String,
}

async fn calcular(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    inicio: NaiveDate,
    fim: NaiveDate,
) -> Result<Vec<RepasseDoMedico>, ErroApi> {
    let (de, _) = tempo::limites_do_dia(inicio).ok_or_else(|| ErroApi::interno("data"))?;
    let (ate, _) = tempo::limites_do_dia(fim).ok_or_else(|| ErroApi::interno("data"))?;
    let linhas = sqlx::query!(
        r#"WITH assinados AS (
             SELECT v.assinada_por AS medico_id, g.exame_tuss AS tuss, (v.assinada_em AT TIME ZONE 'America/Sao_Paulo')::date AS dia
               FROM laudos.versao v JOIN laudos.laudo l ON l.id = v.laudo_id JOIN recepcao.atendimento a ON a.id = l.atendimento_id
               JOIN agenda.agendamento g ON g.id = a.agendamento_id
              WHERE l.unidade_id = $1 AND v.numero = 1 AND v.assinada_em >= $2 AND v.assinada_em < $3
           ), precificados AS (
             SELECT s.medico_id, (SELECT p.valor_centavos FROM financeiro.preco p JOIN financeiro.tabela_preco t ON t.id = p.tabela_id
                                   WHERE t.unidade_id = $1 AND t.vigente_desde <= s.dia AND p.tuss = s.tuss ORDER BY t.versao DESC LIMIT 1) AS valor
               FROM assinados s
           )
           SELECT u.id AS "medico_id!", u.nome AS "medico!", count(*) AS "laudos!", coalesce(sum(p.valor), 0)::bigint AS "base!",
                  count(*) FILTER (WHERE p.valor IS NULL) AS "sem_preco!",
                  r.percentual_pb AS "percentual?", r.versao AS "versao?"
             FROM precificados p JOIN identidade.usuario u ON u.id = p.medico_id
             LEFT JOIN LATERAL (SELECT percentual_pb, versao FROM financeiro.regra_repasse rr WHERE rr.medico_id = u.id AND rr.vigente_desde <= $4
                                ORDER BY rr.versao DESC LIMIT 1) r ON true
            GROUP BY u.id, u.nome, r.percentual_pb, r.versao ORDER BY u.nome"#,
        unidade, de, ate, inicio
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| RepasseDoMedico {
            valor_centavos: l.percentual.map(|pb| l.base * i64::from(pb) / 10_000),
            medico_id: l.medico_id,
            medico: l.medico,
            laudos: l.laudos,
            base_centavos: l.base,
            percentual_pb: l.percentual,
            regra_versao: l.versao,
            sem_preco: l.sem_preco,
        })
        .collect())
}

/// Prévia do repasse da competência (financeiro e gerência).
#[utoipa::path(get, path = "/v1/financeiro/repasse", tag = "financeiro", params(Competencia),
    responses((status = 200, body = Vec<RepasseDoMedico>), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn previa(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(c): Query<Competencia>,
) -> Result<Json<Vec<RepasseDoMedico>>, ErroApi> {
    usuario.exigir(Ato::FinanceiroOperar)?;
    let (inicio, fim) = competencia(&c.competencia)?;
    // Laudos são de outro domínio: leitura agregada em contexto de sistema da mesma unidade.
    let sistema = crate::banco::ContextoBanco {
        usuario_id: Some(usuario.id),
        unidade_id: usuario.unidade_id,
        perfis: vec!["sistema".into()],
    };
    let mut tx = banco::transacao(&estado.banco, &sistema).await?;
    let r = calcular(&mut tx, usuario.unidade_id, inicio, fim).await?;
    tx.commit().await?;
    Ok(Json(r))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RepasseGerado {
    pub titulos_criados: i64,
    pub ja_existiam: i64,
    /// Médicos sem regra vigente: nada foi gerado para eles.
    pub sem_regra: Vec<String>,
}

/// Gera os títulos a pagar do repasse (um por médico e competência; repetir não duplica).
#[utoipa::path(post, path = "/v1/financeiro/repasse/{competencia}/titulos", tag = "financeiro", params(("competencia" = String, Path)),
    responses((status = 200, body = RepasseGerado), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn gerar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(comp): Path<String>,
) -> Result<Json<RepasseGerado>, ErroApi> {
    usuario.exigir(Ato::FinanceiroOperar)?;
    let (inicio, fim) = competencia(&comp)?;
    if fim > tempo::hoje_local() {
        return Err(ErroApi::invalido("competencia-aberta", "A competência ainda não terminou. Gere o repasse depois do último dia do mês. Nada foi gerado."));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let sistema = crate::banco::ContextoBanco {
        usuario_id: Some(usuario.id),
        unidade_id: usuario.unidade_id,
        perfis: vec!["sistema".into()],
    };
    let mut tx = banco::transacao(&estado.banco, &sistema).await?;
    let linhas = calcular(&mut tx, usuario.unidade_id, inicio, fim).await?;
    let mut criados = 0_i64;
    let mut ja = 0_i64;
    let mut sem_regra = Vec::new();
    let vencimento = fim.with_day(10).unwrap_or(fim);
    for l in linhas {
        let (Some(valor), Some(versao)) = (l.valor_centavos, l.regra_versao) else {
            sem_regra.push(l.medico);
            continue;
        };
        if valor <= 0 {
            continue;
        }
        let codigo = proximo(&mut tx, "TIT").await?;
        let exigidas = if valor > DUAS_APROVACOES_ACIMA_CENTAVOS {
            2
        } else {
            1
        };
        let id = novo_id();
        let r = sqlx::query!(
            "INSERT INTO financeiro.titulo (id, codigo, tipo, descricao, contraparte, categoria, valor_centavos, vencimento, aprovacoes_exigidas, referencia, criado_por, origem, unidade_id, correlacao)
             VALUES ($1, $2, 'pagar', $3, $4, 'repasse', $5, $6, $7, $8, $9, 'repasse', $10, $11)
             ON CONFLICT (unidade_id, referencia) WHERE referencia IS NOT NULL DO NOTHING",
            id, codigo.como_str(), format!("Repasse {comp}: {} laudos, regra v{versao}", l.laudos), l.medico, valor, vencimento, exigidas,
            format!("repasse:{}:{comp}", l.medico_id), usuario.id, usuario.unidade_id, correlacao
        )
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 1 {
            criados += 1;
            lastro::registrar(&mut tx, NovoEvento {
                dominio: "financeiro", tipo: "repasse_gerado", registro_id: id, registro_codigo: Some(codigo.como_str()),
                dados: json!({ "medico": l.medico_id, "competencia": comp, "laudos": l.laudos, "base": l.base_centavos, "regra_versao": versao, "percentual_pb": l.percentual_pb }),
                criado_por: Some(usuario.id), origem: "financeiro", unidade_id: usuario.unidade_id, correlacao: &correlacao,
            }).await?;
        } else {
            ja += 1;
        }
    }
    tx.commit().await?;
    Ok(Json(RepasseGerado {
        titulos_criados: criados,
        ja_existiam: ja,
        sem_regra,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct NovaRegraRepasse {
    pub medico_id: Uuid,
    pub percentual_pb: i32,
    pub vigente_desde: NaiveDate,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RegraRepasse {
    pub id: Uuid,
    pub versao: i32,
    pub percentual_pb: i32,
    pub vigente_desde: NaiveDate,
}

/// Nova versão da regra de repasse do médico (a anterior fica no histórico; reautenticação).
#[utoipa::path(post, path = "/v1/financeiro/regras-repasse", tag = "financeiro", request_body = NovaRegraRepasse,
    responses((status = 201, body = RegraRepasse), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn nova_regra(
    State(estado): State<Estado>,
    usuario: Usuario,
    Corpo(n): Corpo<NovaRegraRepasse>,
) -> Result<(StatusCode, Json<RegraRepasse>), ErroApi> {
    usuario.exigir(Ato::FinanceiroAprovar)?;
    usuario.exigir_reautenticacao()?;
    if !(1..=10_000).contains(&n.percentual_pb) {
        return Err(ErroApi::invalido(
            "percentual-invalido",
            "O percentual vai de 0,01% a 100%. Nada foi gravado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    // Versão seguinte de forma atômica: a constraint (medico, versao) recusa a corrida (G2).
    let id = novo_id();
    let versao = sqlx::query_scalar!(
        "INSERT INTO financeiro.regra_repasse (id, medico_id, versao, percentual_pb, vigente_desde, criado_por, origem, unidade_id, correlacao)
         SELECT $1, $2, coalesce(max(versao), 0) + 1, $3, $4, $5, 'financeiro', $6, $7 FROM financeiro.regra_repasse WHERE medico_id = $2
         RETURNING versao",
        id, n.medico_id, n.percentual_pb, n.vigente_desde, usuario.id, usuario.unidade_id, correlacao
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("regra-alterada-agora", "Outra pessoa acabou de mudar esta regra. Confira e tente de novo."),
        sqlx::Error::Database(d) if d.is_foreign_key_violation() => ErroApi::nao_encontrado("medico-nao-encontrado", "Médico não encontrado."),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "financeiro", tipo: "regra_repasse_versionada", registro_id: id, registro_codigo: None,
        dados: json!({ "medico": n.medico_id, "versao": versao, "percentual_pb": n.percentual_pb, "vigente_desde": n.vigente_desde, "reautenticado": true }),
        criado_por: Some(usuario.id), origem: "financeiro", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(RegraRepasse {
            id,
            versao,
            percentual_pb: n.percentual_pb,
            vigente_desde: n.vigente_desde,
        }),
    ))
}
