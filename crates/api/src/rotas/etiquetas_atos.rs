//! Atos da tela de etiquetas (Etiquetas.dc.html, docs/20): invalidar a pulseira com motivo (sem
//! reimprimir; ex.: atendimento desistido) e resolver a divergência de pulseira bipada na sala errada
//! (encaminhar o paciente à sala certa). Os dois são operação atômica no banco (G2: `UPDATE ... WHERE`
//! conferindo as linhas afetadas), aceitam `Idempotency-Key` (G3) e entram no lastro (DEC-2026-11-09-129).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct PedidoInvalidacao {
    pub motivo: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct EtiquetaInvalidada {
    pub codigo: String,
    pub situacao: String,
    pub motivo_invalidacao: String,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoResolucao {
    /// encaminhado (paciente levado à sala certa) ou outro (com observação).
    pub resolucao: String,
    pub observacao: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DivergenciaResolvida {
    pub id: Uuid,
    pub codigo: String,
    pub situacao: String,
    pub descricao: String,
}

async fn repetida(
    tx: &mut sqlx::PgConnection,
    usuario: &Usuario,
    rota: &str,
    chave: Uuid,
) -> Result<Option<Response>, ErroApi> {
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(tx, usuario.id, rota, chave).await?
    {
        return Ok(Some(
            (
                StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
                Json(corpo),
            )
                .into_response(),
        ));
    }
    Ok(None)
}

/// Invalida a pulseira válida com motivo (recepção). Bipada depois, a sala bloqueia (docs/20).
#[utoipa::path(post, operation_id = "invalidar_etiqueta", path = "/v1/etiquetas/{codigo}/invalidacao", tag = "recepcao",
    params(("codigo" = String, Path), ("Idempotency-Key" = Uuid, Header)), request_body = PedidoInvalidacao,
    responses((status = 200, body = EtiquetaInvalidada), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc),
              (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn invalidar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(codigo): Path<String>,
    Corpo(p): Corpo<PedidoInvalidacao>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::EtiquetaReimprimir)?;
    let motivo = p.motivo.trim().to_owned();
    if !(3..=200).contains(&motivo.chars().count()) {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Informe o motivo da invalidação (3 a 200 letras). A etiqueta continua valendo.",
        ));
    }
    let codigo = codigo.trim().to_uppercase();
    let rota = format!("POST /v1/etiquetas/{codigo}/invalidacao");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Some(r) = repetida(&mut tx, &usuario, &rota, chave).await? {
        tx.commit().await?;
        return Ok(r);
    }
    // Atômico: só a primeira de duas invalidações simultâneas encontra a etiqueta válida (G2).
    let feita = sqlx::query!(
        "UPDATE recepcao.etiqueta SET situacao = 'invalidada', motivo_invalidacao = $2, invalidada_por = $3,
                invalidada_em = now(), versao = versao + 1
          WHERE codigo = $1 AND situacao = 'valida' RETURNING id, unidade_id",
        codigo,
        motivo,
        usuario.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(feita) = feita else {
        let existe =
            sqlx::query_scalar!("SELECT 1 FROM recepcao.etiqueta WHERE codigo = $1", codigo)
                .fetch_optional(&mut *tx)
                .await?
                .is_some();
        return Err(if existe {
            ErroApi::conflito(
                "etiqueta-ja-invalidada",
                "Esta etiqueta já não vale (invalidada ou substituída). Nada mudou.",
            )
        } else {
            ErroApi::nao_encontrado(
                "etiqueta-nao-encontrada",
                "Etiqueta não encontrada nesta unidade. Nada mudou.",
            )
        });
    };
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "recepcao",
            tipo: "etiqueta_invalidada",
            registro_id: feita.id,
            registro_codigo: Some(&codigo),
            dados: json!({ "motivo": motivo, "estacao": estacao }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: feita.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(EtiquetaInvalidada {
        codigo,
        situacao: "invalidada".into(),
        motivo_invalidacao: motivo,
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 200, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::OK, Json(corpo)).into_response())
}

/// Resolve a divergência de pulseira bipada na sala errada (recepção, técnico, coordenação).
#[utoipa::path(post, operation_id = "resolver_divergencia_de_sala", path = "/v1/divergencias-de-sala/{id}/resolucao", tag = "recepcao",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = PedidoResolucao,
    responses((status = 200, body = DivergenciaResolvida), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn resolver(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoResolucao>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::DivergenciaResolver)?;
    let observacao = p
        .observacao
        .as_deref()
        .map(str::trim)
        .filter(|o| !o.is_empty());
    match (p.resolucao.as_str(), observacao) {
        ("encaminhado", o) if o.is_none_or(|t| t.chars().count() <= 300) => {}
        ("outro", Some(o)) if (10..=300).contains(&o.chars().count()) => {}
        _ => {
            return Err(ErroApi::invalido(
                "resolucao-invalida",
                "Diga como foi resolvido: encaminhado à sala certa, ou outro com a explicação (10 a 300 letras). Nada mudou.",
            ))
        }
    }
    let rota = format!("POST /v1/divergencias-de-sala/{id}/resolucao");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Some(r) = repetida(&mut tx, &usuario, &rota, chave).await? {
        tx.commit().await?;
        return Ok(r);
    }
    let sala_correta = sqlx::query_scalar!(
        "SELECT s.nome FROM recepcao.divergencia d JOIN recepcao.atendimento t ON t.id = d.atendimento_id
           JOIN agenda.sala s ON s.id = t.sala_id WHERE d.id = $1 AND d.tipo = 'pulseira_sala_errada'",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::conflito(
            "divergencia-indisponivel",
            "Divergência não encontrada nesta unidade. Nada mudou.",
        )
    })?;
    let descricao = if p.resolucao == "encaminhado" {
        format!(
            "paciente encaminhado à {sala_correta}{}",
            observacao.map(|o| format!("; {o}")).unwrap_or_default()
        )
    } else {
        observacao.unwrap_or_default().to_owned()
    };
    // Atômico: duas resoluções ao mesmo tempo, só a primeira encontra a divergência aberta (G2).
    let d = sqlx::query!(
        "UPDATE recepcao.divergencia SET situacao = 'resolvida', versao = versao + 1,
                dados = dados || jsonb_build_object('resolucao', $2::text, 'descricao', $3::text, 'resolvida_por', $4::uuid, 'resolvida_em', now())
          WHERE id = $1 AND situacao = 'aberta' AND tipo = 'pulseira_sala_errada'
          RETURNING codigo, unidade_id, dados->>'etiqueta' AS etiqueta",
        id,
        p.resolucao,
        descricao,
        usuario.id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::conflito(
            "divergencia-indisponivel",
            "Esta divergência já foi resolvida por outra pessoa. Nada mudou; veja o histórico da etiqueta.",
        )
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "recepcao",
            tipo: "divergencia_resolvida",
            registro_id: id,
            registro_codigo: Some(&d.codigo),
            dados: json!({ "etiqueta": d.etiqueta, "resolucao": p.resolucao, "descricao": descricao, "estacao": estacao }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: d.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(DivergenciaResolvida {
        id,
        codigo: d.codigo,
        situacao: "resolvida".into(),
        descricao,
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 200, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::OK, Json(corpo)).into_response())
}
