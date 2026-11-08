//! Reimpressão de etiqueta com motivo invalida a anterior (docs/20, docs/27 FC-ETQ).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::atendimentos::{emitir_pulseira, Etiqueta};
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct PedidoReimpressao {
    pub motivo: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct Reimpressao {
    pub nova: Etiqueta,
    pub invalidada: Etiqueta,
}

/// Reimprime a pulseira com motivo: cria código novo e invalida a anterior.
#[utoipa::path(post, path = "/v1/etiquetas/{codigo}/reimpressao", tag = "recepcao",
    params(("codigo" = String, Path), ("Idempotency-Key" = uuid::Uuid, Header)),
    request_body = PedidoReimpressao,
    responses((status = 201, body = Reimpressao), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc),
              (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn reimprimir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(codigo): Path<String>,
    Corpo(p): Corpo<PedidoReimpressao>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::EtiquetaReimprimir)?;
    let motivo = p
        .motivo
        .map(|m| m.trim().to_owned())
        .filter(|m| m.chars().count() >= 3)
        .ok_or_else(|| {
            ErroApi::invalido(
                "motivo-obrigatorio",
                "Informe o motivo da reimpressão (por exemplo, \"rasgou\"). Nada foi impresso.",
            )
        })?;
    let rota = format!("POST /v1/etiquetas/{codigo}/reimpressao");
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
    let atual = sqlx::query!(
        "SELECT id, codigo, situacao, atendimento_id, paciente_id, unidade_id
           FROM recepcao.etiqueta WHERE codigo = $1 FOR UPDATE",
        codigo
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "etiqueta-nao-encontrada",
            "Esta etiqueta não existe nesta unidade. Nada foi impresso.",
        )
    })?;
    if atual.situacao != "valida" {
        return Err(ErroApi::conflito(
            "etiqueta-ja-invalidada",
            "Esta etiqueta já foi substituída. Nada foi impresso. Reimprima a partir da etiqueta válida do atendimento.",
        ));
    }
    // Invalida primeiro (o índice único só permite uma válida por atendimento), depois emite a nova.
    sqlx::query!(
        "UPDATE recepcao.etiqueta SET situacao = 'invalidada', motivo_invalidacao = $2, invalidada_por = $3,
                invalidada_em = now(), versao = versao + 1
          WHERE id = $1 AND situacao = 'valida'",
        atual.id,
        motivo,
        usuario.id
    )
    .execute(&mut *tx)
    .await?;
    let nova = emitir_pulseira(
        &mut tx,
        atual.atendimento_id,
        atual.paciente_id,
        atual.unidade_id,
        usuario.id,
        estacao.as_deref(),
        &correlacao,
    )
    .await?;
    sqlx::query!(
        "UPDATE recepcao.etiqueta SET substituida_por = $2 WHERE id = $1",
        atual.id,
        nova.codigo
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "recepcao",
            tipo: "etiqueta_reimpressa",
            registro_id: atual.id,
            registro_codigo: Some(&atual.codigo),
            dados: json!({ "motivo": motivo, "nova": nova.codigo, "estacao": estacao }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: atual.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let resposta = Reimpressao {
        invalidada: Etiqueta {
            codigo: atual.codigo,
            tipo: "pulseira".into(),
            situacao: "invalidada".into(),
            motivo_invalidacao: Some(motivo),
            substituida_por: Some(nova.codigo.clone()),
        },
        nova,
    };
    let corpo: Value = serde_json::to_value(&resposta).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoNovaPulseira {
    pub motivo: Option<String>,
}

/// Emite pulseira nova quando o atendimento ficou sem pulseira válida (ex.: correção de cadastro
/// invalidou as etiquetas, FC-ETQ-04). Com pulseira válida, use a reimpressão.
#[utoipa::path(post, path = "/v1/atendimentos/{id}/pulseira", tag = "recepcao",
    params(("id" = uuid::Uuid, Path), ("Idempotency-Key" = uuid::Uuid, Header)),
    request_body = PedidoNovaPulseira,
    responses((status = 201, body = Etiqueta), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn nova_pulseira(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<uuid::Uuid>,
    Corpo(p): Corpo<PedidoNovaPulseira>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::EtiquetaReimprimir)?;
    let motivo = p
        .motivo
        .map(|m| m.trim().to_owned())
        .filter(|m| m.chars().count() >= 3)
        .ok_or_else(|| {
            ErroApi::invalido(
                "motivo-obrigatorio",
                "Informe o motivo da nova pulseira. Nada foi impresso.",
            )
        })?;
    let rota = format!("POST /v1/atendimentos/{id}/pulseira");
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
    let a = sqlx::query!("SELECT id, codigo, paciente_id, unidade_id FROM recepcao.atendimento WHERE id = $1 FOR UPDATE", id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::nao_encontrado("atendimento-nao-encontrado", "Atendimento não encontrado nesta unidade. Nada foi impresso."))?;
    let valida = sqlx::query_scalar!(
        "SELECT codigo FROM recepcao.etiqueta WHERE atendimento_id = $1 AND situacao = 'valida'",
        a.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(c) = valida {
        return Err(ErroApi::conflito("pulseira-valida-existente", &format!("O atendimento já tem a pulseira válida {c}. Nada foi impresso. Para trocar, use a reimpressão com motivo.")));
    }
    let nova = emitir_pulseira(
        &mut tx,
        a.id,
        a.paciente_id,
        a.unidade_id,
        usuario.id,
        estacao.as_deref(),
        &correlacao,
    )
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "recepcao",
            tipo: "pulseira_nova",
            registro_id: a.id,
            registro_codigo: Some(&a.codigo),
            dados: json!({ "motivo": motivo, "nova": nova.codigo, "estacao": estacao }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: a.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo: Value = serde_json::to_value(&nova).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}
