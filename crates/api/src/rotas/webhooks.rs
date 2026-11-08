//! Webhook do PSP do PIX (docs/27 FC-PIX, CLAUDE.md G3): assinatura conferida no CORPO BRUTO antes de
//! qualquer processamento; ID do evento gravado com chave única ANTES de agir; repetido responde 200
//! e para; processamento pelo outbox, consultando o estado atual no PSP (não depende da ordem).

use crate::erro::ErroApi;
use crate::estado::Estado;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::Sha256;
use vidacor_comum::novo_id;

#[derive(Deserialize)]
struct EventoPix {
    id: String,
    txid: String,
}

fn recusado() -> ErroApi {
    ErroApi::novo(
        StatusCode::UNAUTHORIZED,
        "assinatura-invalida",
        "Assinatura inválida",
        "A assinatura do webhook não confere. Nada foi processado.",
    )
}

/// Recebe o webhook do PSP. Responde 2xx rápido; o efeito acontece pelo outbox.
#[utoipa::path(post, path = "/v1/webhooks/pix", tag = "integracoes", params(("X-Assinatura" = String, Header)),
    request_body(content = String, content_type = "application/json", description = "Evento do PSP: id e txid"),
    responses((status = 200), (status = 401, body = crate::openapi::ProblemaDoc)))]
pub async fn pix(
    State(estado): State<Estado>,
    cabecalhos: HeaderMap,
    corpo: Bytes,
) -> Result<(StatusCode, Json<Value>), ErroApi> {
    // 1. Assinatura HMAC-SHA256 do corpo bruto, comparada em tempo constante (SEC-CRI-2).
    let segredo = estado
        .config
        .pix_webhook_segredo
        .as_deref()
        .ok_or_else(recusado)?;
    let enviada = cabecalhos
        .get("x-assinatura")
        .and_then(|v| v.to_str().ok())
        .and_then(|h| {
            (0..h.len())
                .step_by(2)
                .map(|i| h.get(i..i + 2).and_then(|b| u8::from_str_radix(b, 16).ok()))
                .collect::<Option<Vec<u8>>>()
        })
        .ok_or_else(recusado)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(segredo.as_bytes()).map_err(ErroApi::interno)?;
    mac.update(&corpo);
    mac.verify_slice(&enviada).map_err(|_| recusado())?;

    // 2. Só depois de conferir, interpreta o corpo (limite de tamanho já aplicado na aplicação).
    let evento: EventoPix = serde_json::from_slice(&corpo).map_err(|_| {
        ErroApi::invalido(
            "evento-invalido",
            "Evento do PSP em formato inesperado. Nada foi processado.",
        )
    })?;
    if evento.id.is_empty()
        || evento.id.len() > 100
        || evento.txid.is_empty()
        || evento.txid.len() > 64
    {
        return Err(ErroApi::invalido(
            "evento-invalido",
            "Evento do PSP em formato inesperado. Nada foi processado.",
        ));
    }
    let unidade = estado
        .config
        .unidade_servidor
        .ok_or_else(|| ErroApi::interno("servidor sem unidade configurada"))?;

    // 3. Grava o ID do evento ANTES de agir; se já existe, responde 200 e para (FC-PIX-02, 05).
    let mut tx = estado.banco.begin().await?;
    let novo = sqlx::query_scalar!(
        "INSERT INTO integracao.evento_recebido (provedor, evento_id) VALUES ('pix', $1) ON CONFLICT DO NOTHING RETURNING 1",
        evento.id
    )
    .fetch_optional(&mut *tx)
    .await?
    .is_some();
    if !novo {
        tx.commit().await?;
        return Ok((StatusCode::OK, Json(json!({ "situacao": "ja_processado" }))));
    }
    sqlx::query!(
        "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao)
         VALUES ($1, 'pix.processar_evento', $2, $3, $4, $5)",
        novo_id(), novo_id(), json!({ "evento": evento.id, "txid": evento.txid }), unidade, format!("pix:{}", evento.id)
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok((StatusCode::OK, Json(json!({ "situacao": "recebido" }))))
}
