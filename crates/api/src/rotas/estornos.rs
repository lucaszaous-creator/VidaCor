//! Lista e recusa de estornos (gerência e financeiro). A aprovação continua em `caixa::aprovar_estorno`
//! (reautenticação e separação de funções). Recusa exige motivo e fica no lastro; depois dela o caixa
//! pode pedir de novo.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::PacienteResumo;
use crate::rotas::pendencias::{Decisao, FiltroPendencias, Pessoa};
use crate::sessao::Usuario;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct EstornoDetalhado {
    pub id: Uuid,
    pub codigo: String,
    /// pedido, aprovado ou recusado.
    pub situacao: String,
    pub motivo: String,
    pub recebimento_id: Uuid,
    pub recebimento_codigo: String,
    pub paciente: PacienteResumo,
    pub total_centavos: i64,
    /// pix, dinheiro ou cartao.
    pub forma: String,
    pub pago_em: Option<DateTime<Utc>>,
    pub pedido_por: Pessoa,
    pub pedido_em: DateTime<Utc>,
    pub decidido_por: Option<Pessoa>,
    pub motivo_recusa: Option<String>,
}

async fn carregar(
    tx: &mut sqlx::PgConnection,
    id: Option<Uuid>,
    situacao: &str,
) -> Result<Vec<EstornoDetalhado>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT e.id, e.codigo, e.situacao, e.motivo, e.criado_em, e.motivo_recusa,
                  r.id AS recebimento_id, r.codigo AS recebimento_codigo, r.total_centavos, r.forma, r.pago_em,
                  p.id AS paciente_id, p.nome AS paciente_nome, p.nascimento,
                  u.id AS pedido_por_id, u.nome AS pedido_por_nome,
                  d.id AS "decisor_id?", d.nome AS "decisor_nome?"
           FROM caixa.estorno e
           JOIN caixa.recebimento r ON r.id = e.recebimento_id
           JOIN identidade.paciente p ON p.id = r.paciente_id
           JOIN identidade.usuario u ON u.id = e.pedido_por
           LEFT JOIN identidade.usuario d ON d.id = coalesce(e.aprovado_por, e.recusado_por)
           WHERE ($1::uuid IS NULL AND e.situacao = $2) OR e.id = $1
           ORDER BY e.criado_em LIMIT 200"#,
        id,
        situacao
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| EstornoDetalhado {
            id: l.id,
            codigo: l.codigo,
            situacao: l.situacao,
            motivo: l.motivo,
            recebimento_id: l.recebimento_id,
            recebimento_codigo: l.recebimento_codigo,
            paciente: PacienteResumo {
                id: l.paciente_id,
                nome: l.paciente_nome,
                nascimento: l.nascimento,
            },
            total_centavos: l.total_centavos,
            forma: l.forma,
            pago_em: l.pago_em,
            pedido_por: Pessoa {
                id: l.pedido_por_id,
                nome: l.pedido_por_nome,
            },
            pedido_em: l.criado_em,
            decidido_por: l
                .decisor_id
                .zip(l.decisor_nome)
                .map(|(id, nome)| Pessoa { id, nome }),
            motivo_recusa: l.motivo_recusa,
        })
        .collect())
}

fn pode_ver(usuario: &Usuario) -> Result<(), ErroApi> {
    if usuario.pode(Ato::EstornoAprovar) || usuario.pode(Ato::EstornoPedir) {
        Ok(())
    } else {
        Err(ErroApi::proibido())
    }
}

/// Estornos por situação (padrão: os pedidos que aguardam aprovação).
#[utoipa::path(get, operation_id = "listar_estornos", path = "/v1/estornos", tag = "caixa", params(FiltroPendencias),
    responses((status = 200, body = Vec<EstornoDetalhado>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroPendencias>,
) -> Result<Json<Vec<EstornoDetalhado>>, ErroApi> {
    pode_ver(&usuario)?;
    let situacao = f.situacao.unwrap_or_else(|| "pedido".into());
    if !["pedido", "aprovado", "recusado"].contains(&situacao.as_str()) {
        return Err(ErroApi::invalido(
            "situacao-desconhecida",
            "Situação desconhecida no filtro.",
        ));
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = carregar(&mut tx, None, &situacao).await?;
    tx.commit().await?;
    Ok(Json(r))
}

/// Um pedido de estorno, com o recebimento e o paciente (dois identificadores).
#[utoipa::path(get, operation_id = "obter_estorno", path = "/v1/estornos/{id}", tag = "caixa", params(("id" = Uuid, Path)),
    responses((status = 200, body = EstornoDetalhado), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn obter(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<EstornoDetalhado>, ErroApi> {
    pode_ver(&usuario)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = carregar(&mut tx, Some(id), "")
        .await?
        .pop()
        .ok_or_else(|| {
            ErroApi::nao_encontrado(
                "estorno-nao-encontrado",
                "Pedido de estorno não encontrado.",
            )
        })?;
    tx.commit().await?;
    Ok(Json(r))
}

/// Recusa o estorno com motivo (quem pediu não recusa). Atômico: só uma decisão vale (G2).
#[utoipa::path(post, operation_id = "recusar_estorno", path = "/v1/estornos/{id}/recusa", tag = "caixa", params(("id" = Uuid, Path)), request_body = Decisao,
    responses((status = 200, body = EstornoDetalhado), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn recusar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(d): Corpo<Decisao>,
) -> Result<Json<EstornoDetalhado>, ErroApi> {
    usuario.exigir(Ato::EstornoAprovar)?;
    if d.motivo.trim().chars().count() < 5 {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Diga ao caixa por que o estorno foi recusado (mínimo 5 caracteres). Nada foi alterado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let atual = carregar(&mut tx, Some(id), "")
        .await?
        .pop()
        .ok_or_else(|| {
            ErroApi::nao_encontrado(
                "estorno-nao-encontrado",
                "Pedido de estorno não encontrado.",
            )
        })?;
    if atual.pedido_por.id == usuario.id {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "separacao-de-funcoes",
            "Quem pede não decide",
            "Você pediu este estorno e não pode decidi-lo. Nada foi alterado.",
        ));
    }
    let feito = sqlx::query_scalar!(
        "UPDATE caixa.estorno SET situacao = 'recusado', recusado_por = $2, recusado_em = now(), motivo_recusa = $3, versao = versao + 1
         WHERE id = $1 AND situacao = 'pedido' RETURNING id",
        id,
        usuario.id,
        d.motivo.trim()
    )
    .fetch_optional(&mut *tx)
    .await?;
    if feito.is_none() {
        return Err(ErroApi::conflito(
            "estorno-ja-decidido",
            "Este estorno já foi decidido. Nada foi alterado.",
        )
        .com_extra("situacao", json!(atual.situacao)));
    }
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "caixa",
            tipo: "estorno_recusado",
            registro_id: id,
            registro_codigo: Some(&atual.codigo),
            dados: json!({ "motivo": d.motivo.trim(), "pedido_por": atual.pedido_por.id, "recusado_por": usuario.id }),
            criado_por: Some(usuario.id),
            origem: "gerencia",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let r = carregar(&mut tx, Some(id), "")
        .await?
        .pop()
        .ok_or_else(|| ErroApi::interno("estorno sumiu"))?;
    tx.commit().await?;
    Ok(Json(r))
}
