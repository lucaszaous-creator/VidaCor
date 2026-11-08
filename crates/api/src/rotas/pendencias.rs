//! Pendências da gerência no caixa (mockup Caixa, docs/27 FC-CXA): pedido de desconto acima do
//! limite numa fila (o caixa pede; a gerência autoriza ou recusa com motivo; quem pede só cancela) e
//! a lista de estornos pedidos, com recusa motivada. Tudo com lastro; nada é apagado.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::PacienteResumo;
use crate::rotas::caixa::{proximo, LIMITE_DESCONTO_CAIXA_PB};
use crate::sessao::Usuario;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Deserialize, ToSchema)]
pub struct NovoPedidoDesconto {
    pub paciente_id: Uuid,
    pub pontos_base: i32,
    pub motivo: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PedidoDesconto {
    pub id: Uuid,
    pub codigo: String,
    pub paciente: PacienteResumo,
    pub pontos_base: i32,
    pub motivo: String,
    pub solicitante: Pessoa,
    /// pedido, autorizado, recusado ou cancelado.
    pub situacao: String,
    /// Código da autorização (quando autorizado); o caixa a usa no recebimento.
    pub autorizacao_id: Option<Uuid>,
    pub decidido_por: Option<Pessoa>,
    pub motivo_decisao: Option<String>,
    pub criado_em: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct Pessoa {
    pub id: Uuid,
    pub nome: String,
}

#[derive(Deserialize, IntoParams)]
pub struct FiltroPendencias {
    /// Situação (padrão: pedido, as pendentes).
    pub situacao: Option<String>,
}

fn situacao_valida(s: Option<String>, validas: &[&str]) -> Result<String, ErroApi> {
    let s = s.unwrap_or_else(|| "pedido".into());
    if validas.contains(&s.as_str()) {
        Ok(s)
    } else {
        Err(ErroApi::invalido(
            "situacao-desconhecida",
            "Situação desconhecida no filtro.",
        ))
    }
}

async fn carregar_pedidos(
    tx: &mut sqlx::PgConnection,
    id: Option<Uuid>,
    situacao: &str,
) -> Result<Vec<PedidoDesconto>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT d.id, d.codigo, d.pontos_base, d.motivo, d.situacao, d.autorizacao_id, d.motivo_decisao, d.criado_em,
                  p.id AS paciente_id, p.nome AS paciente_nome, p.nascimento,
                  s.id AS solicitante_id, s.nome AS solicitante_nome,
                  g.id AS "decisor_id?", g.nome AS "decisor_nome?"
           FROM caixa.pedido_desconto d
           JOIN identidade.paciente p ON p.id = d.paciente_id
           JOIN identidade.usuario s ON s.id = d.solicitante_id
           LEFT JOIN identidade.usuario g ON g.id = d.decidido_por
           WHERE ($1::uuid IS NULL AND d.situacao = $2) OR d.id = $1
           ORDER BY d.criado_em LIMIT 200"#,
        id,
        situacao
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| PedidoDesconto {
            id: l.id,
            codigo: l.codigo,
            paciente: PacienteResumo {
                id: l.paciente_id,
                nome: l.paciente_nome,
                nascimento: l.nascimento,
            },
            pontos_base: l.pontos_base,
            motivo: l.motivo,
            solicitante: Pessoa {
                id: l.solicitante_id,
                nome: l.solicitante_nome,
            },
            situacao: l.situacao,
            autorizacao_id: l.autorizacao_id,
            decidido_por: l
                .decisor_id
                .zip(l.decisor_nome)
                .map(|(id, nome)| Pessoa { id, nome }),
            motivo_decisao: l.motivo_decisao,
            criado_em: l.criado_em,
        })
        .collect())
}

async fn um_pedido(tx: &mut sqlx::PgConnection, id: Uuid) -> Result<PedidoDesconto, ErroApi> {
    carregar_pedidos(tx, Some(id), "")
        .await?
        .pop()
        .ok_or_else(|| {
            ErroApi::nao_encontrado(
                "pedido-nao-encontrado",
                "Pedido de desconto não encontrado.",
            )
        })
}

/// O caixa pede à gerência um desconto acima do seu limite. O total só muda depois da autorização.
#[utoipa::path(post, path = "/v1/pedidos-desconto", tag = "caixa", request_body = NovoPedidoDesconto,
    params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = PedidoDesconto), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn pedir_desconto(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<NovoPedidoDesconto>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::CaixaOperar)?;
    if p.pontos_base <= LIMITE_DESCONTO_CAIXA_PB || p.pontos_base > 10_000 {
        return Err(ErroApi::invalido("pedido-desnecessario",
            "Até o seu limite o desconto é aplicado direto no recebimento; acima de 100% não existe. Nada foi pedido.")
            .com_extra("limite_pontos_base", json!(LIMITE_DESCONTO_CAIXA_PB)));
    }
    if p.motivo.trim().chars().count() < 5 {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Informe o motivo do desconto (mínimo 5 caracteres). Nada foi pedido.",
        ));
    }
    const ROTA: &str = "POST /v1/pedidos-desconto";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, ROTA, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let codigo = proximo(&mut tx, "PDD").await?;
    let id = novo_id();
    sqlx::query!(
        "INSERT INTO caixa.pedido_desconto (id, codigo, paciente_id, pontos_base, motivo, solicitante_id, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $6, 'caixa', $7, $8)",
        id, codigo.como_str(), p.paciente_id, p.pontos_base, p.motivo.trim(), usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("pedido-ja-aberto",
            "Já há um pedido de desconto seu aguardando a gerência para este paciente. Nada foi pedido de novo."),
        sqlx::Error::Database(d) if d.is_foreign_key_violation() => ErroApi::nao_encontrado("paciente-nao-encontrado", "Paciente não encontrado. Nada foi pedido."),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "caixa", tipo: "desconto_pedido", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "pontos_base": p.pontos_base, "motivo": p.motivo.trim(), "paciente": p.paciente_id }),
        criado_por: Some(usuario.id), origem: "caixa", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let corpo = serde_json::to_value(um_pedido(&mut tx, id).await?).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Pedidos de desconto: a gerência vê os da unidade; o caixa, só os próprios (RLS).
#[utoipa::path(get, path = "/v1/pedidos-desconto", tag = "caixa", params(FiltroPendencias),
    responses((status = 200, body = Vec<PedidoDesconto>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar_pedidos(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroPendencias>,
) -> Result<Json<Vec<PedidoDesconto>>, ErroApi> {
    if !(usuario.pode(Ato::CaixaOperar) || usuario.pode(Ato::DescontoAutorizar)) {
        return Err(ErroApi::proibido());
    }
    let situacao = situacao_valida(
        f.situacao,
        &["pedido", "autorizado", "recusado", "cancelado"],
    )?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = carregar_pedidos(&mut tx, None, &situacao).await?;
    tx.commit().await?;
    Ok(Json(r))
}

/// Um pedido de desconto (o caixa acompanha; a gerência abre para decidir).
#[utoipa::path(get, path = "/v1/pedidos-desconto/{id}", tag = "caixa", params(("id" = Uuid, Path)),
    responses((status = 200, body = PedidoDesconto), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn obter_pedido(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<PedidoDesconto>, ErroApi> {
    if !(usuario.pode(Ato::CaixaOperar) || usuario.pode(Ato::DescontoAutorizar)) {
        return Err(ErroApi::proibido());
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = um_pedido(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(r))
}

#[derive(Deserialize, ToSchema)]
pub struct Decisao {
    pub motivo: String,
}

/// Fecha um pedido aberto de forma atômica (G2): só uma decisão vale.
async fn decidir(
    estado: &Estado,
    usuario: &Usuario,
    id: Uuid,
    situacao: &str,
    motivo: Option<&str>,
) -> Result<PedidoDesconto, ErroApi> {
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let atual = um_pedido(&mut tx, id).await?;
    let quem_pediu = atual.solicitante.id == usuario.id;
    if situacao == "cancelado" && !quem_pediu {
        return Err(ErroApi::novo(StatusCode::FORBIDDEN, "so-quem-pediu-cancela", "Só quem pediu cancela",
            "Só o caixa que fez o pedido pode cancelá-lo. Para não autorizar, recuse com o motivo. Nada foi alterado."));
    }
    if situacao == "recusado" && quem_pediu {
        return Err(ErroApi::novo(StatusCode::FORBIDDEN, "separacao-de-funcoes", "Quem pede não decide",
            "Você fez este pedido e não pode recusá-lo; cancele se não precisar mais. Nada foi alterado."));
    }
    let alterado = sqlx::query_scalar!(
        "UPDATE caixa.pedido_desconto SET situacao = $2, decidido_por = $3, decidido_em = now(), motivo_decisao = $4, versao = versao + 1
         WHERE id = $1 AND situacao = 'pedido' RETURNING id",
        id, situacao, usuario.id, motivo
    )
    .fetch_optional(&mut *tx)
    .await?;
    if alterado.is_none() {
        return Err(ErroApi::conflito(
            "pedido-ja-decidido",
            "Este pedido já foi decidido. Nada foi alterado.",
        )
        .com_extra("situacao", json!(atual.situacao)));
    }
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "caixa",
            tipo: if situacao == "cancelado" {
                "desconto_pedido_cancelado"
            } else {
                "desconto_recusado"
            },
            registro_id: id,
            registro_codigo: Some(&atual.codigo),
            dados: json!({ "motivo": motivo, "por": usuario.id }),
            criado_por: Some(usuario.id),
            origem: if quem_pediu { "caixa" } else { "gerencia" },
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let r = um_pedido(&mut tx, id).await?;
    tx.commit().await?;
    Ok(r)
}

/// A gerência recusa o pedido de desconto, com motivo que o caixa lê.
#[utoipa::path(post, path = "/v1/pedidos-desconto/{id}/recusa", tag = "caixa", params(("id" = Uuid, Path)), request_body = Decisao,
    responses((status = 200, body = PedidoDesconto), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn recusar_desconto(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(d): Corpo<Decisao>,
) -> Result<Json<PedidoDesconto>, ErroApi> {
    usuario.exigir(Ato::DescontoAutorizar)?;
    if d.motivo.trim().chars().count() < 5 {
        return Err(ErroApi::invalido("motivo-obrigatorio", "Diga ao caixa por que o desconto foi recusado (mínimo 5 caracteres). Nada foi alterado."));
    }
    Ok(Json(
        decidir(&estado, &usuario, id, "recusado", Some(d.motivo.trim())).await?,
    ))
}

/// Quem pediu cancela (vai cobrar o valor cheio ou mudar o percentual).
#[utoipa::path(post, path = "/v1/pedidos-desconto/{id}/cancelamento", tag = "caixa", params(("id" = Uuid, Path)),
    responses((status = 200, body = PedidoDesconto), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn cancelar_desconto(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<PedidoDesconto>, ErroApi> {
    usuario.exigir(Ato::CaixaOperar)?;
    Ok(Json(
        decidir(&estado, &usuario, id, "cancelado", None).await?,
    ))
}

/// Liga a autorização ao pedido aberto, na mesma transação da autorização (G2). O pedido precisa
/// bater com o que está sendo autorizado: paciente, percentual e caixa.
pub(crate) async fn marcar_autorizado(
    tx: &mut sqlx::PgConnection,
    pedido_id: Uuid,
    autorizacao_id: Uuid,
    usuario: &Usuario,
    paciente_id: Uuid,
    pontos_base: i32,
    solicitante_id: Uuid,
) -> Result<(), ErroApi> {
    let feito = sqlx::query_scalar!(
        "UPDATE caixa.pedido_desconto SET situacao = 'autorizado', autorizacao_id = $2, decidido_por = $3, decidido_em = now(), versao = versao + 1
         WHERE id = $1 AND situacao = 'pedido' AND paciente_id = $4 AND pontos_base = $5 AND solicitante_id = $6 RETURNING id",
        pedido_id, autorizacao_id, usuario.id, paciente_id, pontos_base, solicitante_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if feito.is_none() {
        return Err(ErroApi::conflito("pedido-ja-decidido",
            "O pedido de desconto já foi decidido, cancelado ou não bate com o que está sendo autorizado. Nada foi autorizado."));
    }
    Ok(())
}
