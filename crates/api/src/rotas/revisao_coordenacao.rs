//! Revisão por pares, lado da coordenação médica (Revisao-Pares.dc.html, visão Resultados;
//! DEC-2026-11-08-101 e 103). Sorteio ou escolha de laudos assinados para segunda leitura, nunca pelo
//! autor (CHECK e gatilho no banco), um revisor por laudo de cada vez (índice único) e o painel com o
//! resultado por médico e as discrepâncias. Só metadados: a coordenação não lê o texto do laudo nem
//! o comentário do revisor.

use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::rotas::modelos_laudo::{exigir_coordenacao_medica, repetida};
use crate::rotas::revisao_pares::SIGNIFICATIVAS;
use crate::sessao::Usuario;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Serialize, ToSchema)]
pub struct RevisaoDesignada {
    pub id: Uuid,
    pub codigo: String,
    pub laudo_id: Uuid,
    pub laudo_codigo: String,
    pub exame: String,
    pub revisor_id: Uuid,
    pub revisor: String,
    /// sorteio ou selecao.
    pub modo: String,
    pub situacao: String,
    pub designada_em: DateTime<Utc>,
    pub motivo_conflito: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoSorteio {
    /// Quantos laudos sortear (1 a 50).
    pub quantidade: i32,
    /// Laudos assinados nos últimos N dias (1 a 90).
    pub dias: i32,
}

#[derive(Serialize, ToSchema)]
pub struct ResultadoDoSorteio {
    pub candidatos: i64,
    pub designadas: Vec<RevisaoDesignada>,
    /// Laudos sorteados sem outro médico disponível para revisar.
    pub sem_revisor: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoDesignacao {
    pub laudo_id: Uuid,
    pub revisor_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct ResultadoDoMedico {
    pub medico_id: Uuid,
    pub medico: String,
    pub revisados: i64,
    pub concordantes: i64,
    pub concordancia_percentual: Option<i64>,
    pub discrepancias: i64,
    pub pendentes: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DiscrepanciaDeRevisao {
    pub id: Uuid,
    pub codigo: String,
    pub laudo_codigo: String,
    pub exame: String,
    pub classe: String,
    pub autor: String,
    pub pendencia: Option<String>,
    pub concluida_em: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct MedicoDaUnidade {
    pub id: Uuid,
    pub nome: String,
}

#[derive(Serialize, ToSchema)]
pub struct PainelDeRevisao {
    pub dias: i32,
    pub por_medico: Vec<ResultadoDoMedico>,
    pub discrepancias: Vec<DiscrepanciaDeRevisao>,
    pub abertas: Vec<RevisaoDesignada>,
    pub conflitos: Vec<RevisaoDesignada>,
    pub medicos: Vec<MedicoDaUnidade>,
}

async fn designadas(
    tx: &mut PgConnection,
    situacao: &str,
    ids: Option<&[Uuid]>,
) -> Result<Vec<RevisaoDesignada>, sqlx::Error> {
    sqlx::query_as!(
        RevisaoDesignada,
        r#"SELECT r.id, r.codigo, r.laudo_id, r.laudo_codigo, r.exame, r.revisor_id, u.nome AS revisor, r.modo, r.situacao,
                  r.criado_em AS designada_em, r.motivo_conflito
             FROM laudos.revisao r JOIN identidade.usuario u ON u.id = r.revisor_id
            WHERE r.situacao = $1 AND ($2::uuid[] IS NULL OR r.id = ANY($2))
              AND (r.situacao <> 'conflito_declarado' OR NOT EXISTS (
                    SELECT 1 FROM laudos.revisao o WHERE o.laudo_id = r.laudo_id AND o.situacao IN ('designada','concluida')))
            ORDER BY r.criado_em DESC LIMIT 200"#,
        situacao,
        ids
    )
    .fetch_all(&mut *tx)
    .await
}

async fn designar_um(
    tx: &mut PgConnection,
    usuario: &Usuario,
    laudo_id: Uuid,
    revisor_id: Uuid,
    modo: &str,
    correlacao: &str,
) -> Result<Uuid, ErroApi> {
    let id = novo_id();
    let codigo = proximo(tx, "REV").await?;
    // Versão, autor, paciente, código do laudo e exame vêm do próprio laudo, pelo gatilho do banco.
    sqlx::query!(
        "INSERT INTO laudos.revisao (id, codigo, laudo_id, revisor_id, modo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'coordenacao', $7, $8)",
        id, codigo.como_str(), laudo_id, revisor_id, modo, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("revisao_revisor_nao_e_autor") => ErroApi::invalido(
            "revisor-e-autor",
            "Quem laudou não revisa o próprio laudo. Nada foi designado. Escolha outro médico.",
        ),
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito(
            "revisao-em-andamento",
            "Este laudo já tem revisor designado ou já foi revisado nesta versão. Nada foi designado.",
        ),
        sqlx::Error::Database(d) if d.code().as_deref() == Some("VCR01") => ErroApi::nao_encontrado(
            "laudo-nao-encontrado",
            "Este laudo não existe nesta unidade. Nada foi designado.",
        ),
        sqlx::Error::Database(d) if d.code().as_deref() == Some("VCR02") => ErroApi::conflito(
            "laudo-nao-assinado",
            "Só laudo assinado vai para revisão por par. Nada foi designado.",
        ),
        sqlx::Error::Database(d) if d.code().as_deref() == Some("VCR03") => ErroApi::invalido(
            "revisor-invalido",
            "O revisor precisa ser médico ativo desta unidade. Nada foi designado.",
        ),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        tx,
        NovoEvento {
            dominio: "laudos",
            tipo: "revisao_designada",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "laudo": laudo_id, "revisor": revisor_id, "modo": modo }),
            criado_por: Some(usuario.id),
            origem: "coordenacao",
            unidade_id: usuario.unidade_id,
            correlacao,
        },
    )
    .await?;
    Ok(id)
}

async fn medicos(
    tx: &mut PgConnection,
    unidade: Uuid,
) -> Result<Vec<MedicoDaUnidade>, sqlx::Error> {
    sqlx::query_as!(
        MedicoDaUnidade,
        "SELECT id, nome FROM identidade.usuario WHERE unidade_id = $1 AND ativo AND 'medico' = ANY(perfis) ORDER BY nome",
        unidade
    )
    .fetch_all(&mut *tx)
    .await
}

/// Sorteia laudos assinados no período e designa a cada um o médico com menos revisões abertas,
/// nunca o autor nem quem já declarou conflito naquele laudo (coordenação médica; Idempotency-Key).
#[utoipa::path(post, operation_id = "sortear_revisoes_por_pares", path = "/v1/revisoes-por-pares/sorteio", tag = "revisao-por-pares",
    params(("Idempotency-Key" = Uuid, Header)), request_body = PedidoSorteio,
    responses((status = 201, body = ResultadoDoSorteio), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn sortear(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<PedidoSorteio>,
) -> Result<Response, ErroApi> {
    exigir_coordenacao_medica(&usuario)?;
    if !(1..=50).contains(&p.quantidade) || !(1..=90).contains(&p.dias) {
        return Err(ErroApi::invalido(
            "sorteio-invalido",
            "Sorteie de 1 a 50 laudos, assinados nos últimos 1 a 90 dias. Nada foi sorteado.",
        ));
    }
    let rota = "POST /v1/revisoes-por-pares/sorteio";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let candidatos = sqlx::query!(
        r#"SELECT laudo_id AS "laudo_id!", autor_id AS "autor_id!" FROM laudos.candidatos_a_revisao($1, $2)"#,
        Utc::now() - Duration::days(i64::from(p.dias)),
        p.quantidade
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut lista = medicos(&mut tx, usuario.unidade_id).await?;
    lista.shuffle(&mut rand::rng());
    let mut carga: HashMap<Uuid, i64> = sqlx::query!(
        r#"SELECT revisor_id, count(*) AS "n!" FROM laudos.revisao WHERE situacao = 'designada' GROUP BY revisor_id"#
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|c| (c.revisor_id, c.n))
    .collect();
    let laudos: Vec<Uuid> = candidatos.iter().map(|c| c.laudo_id).collect();
    let conflitos: HashSet<(Uuid, Uuid)> = sqlx::query!(
        "SELECT laudo_id, revisor_id FROM laudos.revisao WHERE situacao = 'conflito_declarado' AND laudo_id = ANY($1)",
        &laudos
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|c| (c.laudo_id, c.revisor_id))
    .collect();
    let mut feitas = Vec::new();
    let mut sem_revisor = 0;
    for c in &candidatos {
        let escolhido = lista
            .iter()
            .filter(|m| m.id != c.autor_id && !conflitos.contains(&(c.laudo_id, m.id)))
            .min_by_key(|m| carga.get(&m.id).copied().unwrap_or(0))
            .map(|m| m.id);
        let Some(revisor) = escolhido else {
            sem_revisor += 1;
            continue;
        };
        // Outro sorteio ou designação pode ter levado o laudo agora: pula sem perder os demais.
        sqlx::query("SAVEPOINT designacao")
            .execute(&mut *tx)
            .await?;
        match designar_um(
            &mut tx,
            &usuario,
            c.laudo_id,
            revisor,
            "sorteio",
            &correlacao,
        )
        .await
        {
            Ok(id) => {
                sqlx::query("RELEASE SAVEPOINT designacao")
                    .execute(&mut *tx)
                    .await?;
                feitas.push(id);
                *carga.entry(revisor).or_insert(0) += 1;
            }
            Err(e) if e.codigo() == "revisao-em-andamento" => {
                sqlx::query("ROLLBACK TO SAVEPOINT designacao")
                    .execute(&mut *tx)
                    .await?;
            }
            Err(e) => return Err(e),
        }
    }
    let resultado = ResultadoDoSorteio {
        candidatos: candidatos.len() as i64,
        designadas: designadas(&mut tx, "designada", Some(feitas.as_slice())).await?,
        sem_revisor,
    };
    let corpo = serde_json::to_value(&resultado).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Designa um laudo assinado a um revisor escolhido (coordenação médica; Idempotency-Key). O banco
/// recusa o autor como revisor e um segundo revisor ao mesmo tempo no mesmo laudo.
#[utoipa::path(post, operation_id = "designar_revisao_por_par", path = "/v1/revisoes-por-pares", tag = "revisao-por-pares",
    params(("Idempotency-Key" = Uuid, Header)), request_body = PedidoDesignacao,
    responses((status = 201, body = RevisaoDesignada), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn designar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<PedidoDesignacao>,
) -> Result<Response, ErroApi> {
    exigir_coordenacao_medica(&usuario)?;
    let rota = "POST /v1/revisoes-por-pares";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let declarou = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM laudos.revisao WHERE laudo_id = $1 AND revisor_id = $2 AND situacao = 'conflito_declarado') AS "d!""#,
        p.laudo_id,
        p.revisor_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if declarou {
        return Err(ErroApi::conflito(
            "revisor-declarou-conflito",
            "Este médico declarou conflito neste laudo. Nada foi designado. Escolha outro revisor.",
        ));
    }
    let id = designar_um(
        &mut tx,
        &usuario,
        p.laudo_id,
        p.revisor_id,
        "selecao",
        &correlacao,
    )
    .await?;
    let r = designadas(&mut tx, "designada", Some(std::slice::from_ref(&id)))
        .await?
        .pop()
        .ok_or_else(|| ErroApi::interno("revisão sumiu"))?;
    let corpo = serde_json::to_value(&r).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Painel da coordenação médica: resultado por médico (laudos revisados, concordância e
/// discrepâncias), discrepâncias com a situação da pendência, revisões abertas e conflitos a
/// redesignar. Sem texto de laudo e sem comentário do revisor.
#[utoipa::path(get, operation_id = "painel_de_revisao_por_pares", path = "/v1/revisoes-por-pares/painel", tag = "revisao-por-pares",
    responses((status = 200, body = PainelDeRevisao), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn painel(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<PainelDeRevisao>, ErroApi> {
    exigir_coordenacao_medica(&usuario)?;
    const DIAS: i32 = 90;
    let desde = Utc::now() - Duration::days(i64::from(DIAS));
    let significativas: Vec<String> = SIGNIFICATIVAS.iter().map(|s| (*s).to_owned()).collect();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let por_medico = sqlx::query!(
        r#"SELECT r.autor_id, u.nome, count(*) AS "revisados!",
                  count(*) FILTER (WHERE r.classe = '1') AS "concordantes!",
                  count(*) FILTER (WHERE r.pendencia = 'aguarda_autor') AS "pendentes!"
             FROM laudos.revisao r JOIN identidade.usuario u ON u.id = r.autor_id
            WHERE r.situacao = 'concluida' AND r.concluida_em >= $1
            GROUP BY r.autor_id, u.nome ORDER BY u.nome"#,
        desde
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|l| ResultadoDoMedico {
        medico_id: l.autor_id,
        medico: l.nome,
        revisados: l.revisados,
        concordantes: l.concordantes,
        concordancia_percentual: (l.revisados > 0).then(|| l.concordantes * 100 / l.revisados),
        discrepancias: l.revisados - l.concordantes,
        pendentes: l.pendentes,
    })
    .collect();
    let discrepancias = sqlx::query_as!(
        DiscrepanciaDeRevisao,
        r#"SELECT r.id, r.codigo, r.laudo_codigo, r.exame, r.classe AS "classe!", u.nome AS autor, r.pendencia, r.concluida_em AS "concluida_em!"
             FROM laudos.revisao r JOIN identidade.usuario u ON u.id = r.autor_id
            WHERE r.situacao = 'concluida' AND r.classe <> '1' AND r.concluida_em >= $1
            ORDER BY (r.pendencia = 'aguarda_autor') DESC NULLS LAST, (r.classe = ANY($2)) DESC, r.concluida_em DESC LIMIT 100"#,
        desde,
        &significativas[..]
    )
    .fetch_all(&mut *tx)
    .await?;
    let abertas = designadas(&mut tx, "designada", None).await?;
    let conflitos = designadas(&mut tx, "conflito_declarado", None).await?;
    let medicos = medicos(&mut tx, usuario.unidade_id).await?;
    tx.commit().await?;
    Ok(Json(PainelDeRevisao {
        dias: DIAS,
        por_medico,
        discrepancias,
        abertas,
        conflitos,
        medicos,
    }))
}
