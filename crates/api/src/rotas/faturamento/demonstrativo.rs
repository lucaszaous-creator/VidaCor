//! Demonstrativo de retorno da operadora (docs/21 § 3.9): valor pago e glosas por guia, com o prazo
//! de recurso do contrato. Pago mais glosado fecha com o apresentado em cada guia (G1).

use super::contrato::{LoteDetalhe, PedidoDemonstrativo};
use super::lotes::{detalhe, lote_nao_encontrado, repetida};
use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;
use vidacor_comum::novo_id;

fn invalido(codigo: &str, detalhe: String) -> ErroApi {
    ErroApi::invalido(codigo, &format!("{detalhe} Nada foi gravado."))
}

/// Registra o demonstrativo de retorno do lote, conta a conta (só o faturista): valor pago e glosas
/// com código e motivo. Pago mais glosado fecha com o apresentado em cada guia (G1: o servidor
/// confere a conta).
#[utoipa::path(post, operation_id = "registrar_demonstrativo", path = "/v1/faturamento/lotes/{id}/demonstrativo", tag = "faturamento",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = PedidoDemonstrativo,
    responses((status = 201, body = LoteDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn demonstrativo(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoDemonstrativo>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::DemonstrativoRegistrar)?;
    if p.recebido_em > tempo::hoje_local() {
        return Err(invalido(
            "data-futura",
            "A data de recebimento do demonstrativo está no futuro.".into(),
        ));
    }
    let rota = format!("POST /v1/faturamento/lotes/{id}/demonstrativo");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let l = sqlx::query!(
        "SELECT l.codigo, l.situacao, l.valor_total_centavos, k.prazo_recurso_dias FROM faturamento.lote l
           JOIN faturamento.contrato k ON k.id = l.contrato_id WHERE l.id = $1 FOR UPDATE OF l",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(lote_nao_encontrado)?;
    match l.situacao.as_str() {
        "enviado" => {}
        "demonstrativo_recebido" => {
            return Err(ErroApi::conflito(
                "demonstrativo-ja-registrado",
                "O demonstrativo deste lote já foi registrado. Nada foi alterado.",
            ))
        }
        _ => {
            return Err(ErroApi::conflito(
                "lote-nao-enviado",
                "Este lote ainda não foi recebido pela operadora. Nada foi gravado. Registre o demonstrativo depois do protocolo.",
            ))
        }
    }
    let contas: BTreeMap<Uuid, (String, i64)> = sqlx::query!(
        "SELECT id, codigo, valor_total_centavos FROM faturamento.conta WHERE lote_id = $1",
        id
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|c| (c.id, (c.codigo, c.valor_total_centavos)))
    .collect();
    let mut vistas = std::collections::BTreeSet::new();
    for r in &p.contas {
        let Some((codigo, total)) = contas.get(&r.conta_id) else {
            return Err(invalido(
                "conta-fora-do-lote",
                "Uma das guias do demonstrativo não é deste lote.".into(),
            ));
        };
        if !vistas.insert(r.conta_id) {
            return Err(invalido(
                "conta-repetida",
                format!("A guia {codigo} aparece duas vezes no demonstrativo."),
            ));
        }
        if r.glosas.len() > 20 || r.valor_pago_centavos < 0 {
            return Err(invalido(
                "retorno-invalido",
                format!("O retorno da guia {codigo} tem valor negativo ou mais de 20 glosas."),
            ));
        }
        for g in &r.glosas {
            let motivo = g.motivo.trim().chars().count();
            if g.codigo_glosa.len() != 4
                || !g.codigo_glosa.chars().all(|c| c.is_ascii_digit())
                || !(3..=500).contains(&motivo)
                || g.valor_centavos <= 0
            {
                return Err(invalido("glosa-invalida", format!("Uma glosa da guia {codigo} está incompleta: código TISS com 4 dígitos, motivo e valor acima de zero.")));
            }
        }
        let glosado: i64 = r.glosas.iter().map(|g| g.valor_centavos).sum();
        if r.valor_pago_centavos + glosado != *total {
            return Err(invalido(
                "valores-nao-fecham",
                format!(
                    "Na guia {codigo}, pago mais glosado não fecha com o apresentado ({}).",
                    vidacor_dominio_faturamento::Centavos(*total).reais()
                ),
            ));
        }
    }
    if vistas.len() != contas.len() {
        return Err(invalido(
            "demonstrativo-incompleto",
            format!(
                "O demonstrativo traz {} de {} guias do lote.",
                vistas.len(),
                contas.len()
            ),
        ));
    }
    let pago: i64 = p.contas.iter().map(|r| r.valor_pago_centavos).sum();
    let demonstrativo = novo_id();
    let codigo = crate::rotas::caixa::proximo(&mut tx, "DMT").await?;
    sqlx::query!(
        "INSERT INTO faturamento.demonstrativo (id, codigo, lote_id, recebido_em, valor_apresentado_centavos, valor_pago_centavos,
                valor_glosado_centavos, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'faturamento', $9, $10)",
        demonstrativo, codigo.como_str(), id, p.recebido_em, l.valor_total_centavos, pago, l.valor_total_centavos - pago,
        usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    let prazo = p.recebido_em + chrono::Duration::days(i64::from(l.prazo_recurso_dias));
    for r in &p.contas {
        for g in &r.glosas {
            let gcodigo = crate::rotas::caixa::proximo(&mut tx, "GLO").await?;
            sqlx::query!(
                "INSERT INTO faturamento.glosa (id, codigo, conta_id, demonstrativo_id, codigo_glosa, motivo, valor_centavos,
                        prazo_recurso, criado_por, origem, unidade_id, correlacao)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'faturamento', $10, $11)",
                novo_id(), gcodigo.como_str(), r.conta_id, demonstrativo, g.codigo_glosa, g.motivo.trim(), g.valor_centavos,
                prazo, usuario.id, usuario.unidade_id, correlacao
            )
            .execute(&mut *tx)
            .await?;
        }
        let situacao = match (r.valor_pago_centavos, r.glosas.is_empty()) {
            (_, true) => "paga",
            (0, false) => "glosada",
            _ => "paga_com_glosa",
        };
        sqlx::query!(
            "UPDATE faturamento.conta SET situacao = $2, valor_pago_centavos = $3, versao = versao + 1 WHERE id = $1",
            r.conta_id,
            situacao,
            r.valor_pago_centavos
        )
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query!(
        "UPDATE faturamento.lote SET situacao = 'demonstrativo_recebido', versao = versao + 1 WHERE id = $1",
        id
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "faturamento",
            tipo: "demonstrativo_registrado",
            registro_id: demonstrativo,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "lote": id, "lote_codigo": l.codigo, "pago_centavos": pago,
                           "glosado_centavos": l.valor_total_centavos - pago,
                           "glosas": p.contas.iter().map(|r| r.glosas.len()).sum::<usize>() }),
            criado_por: Some(usuario.id),
            origem: "faturamento",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let d = detalhe(&mut tx, id).await?;
    let corpo = serde_json::to_value(&d).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}
