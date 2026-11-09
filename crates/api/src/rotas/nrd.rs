//! NRD da clínica (nível de referência de diagnóstico) por exame de TC e raio X, versionado com
//! motivo (Gestao-Dose.dc.html, aba Protocolos; DEC-2026-11-09-146). A gerência publica; médicos e
//! gerência consultam. Doses já lidas continuam comparadas à versão aplicada na leitura.

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
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use vidacor_comum::novo_id;

#[derive(Serialize, ToSchema)]
pub struct NrdDoExame {
    pub tuss: String,
    pub exame: String,
    pub modalidade: Option<String>,
    pub numero: Option<i32>,
    pub grandeza: Option<String>,
    pub valor: Option<f64>,
    pub fator_dose_efetiva: Option<f64>,
    pub motivo: Option<String>,
    pub publicado_em: Option<DateTime<Utc>>,
}

#[derive(Deserialize, ToSchema)]
pub struct NovoNrd {
    /// Número da versão vigente que a pessoa viu (ausente quando não há nenhuma).
    pub versao_vista: Option<i32>,
    /// ctdivol ou dlp (TC); dap (raio X).
    pub grandeza: String,
    pub valor: f64,
    /// mSv por mGy·cm (TC) ou por Gy·cm² (raio X). Opcional.
    pub fator_dose_efetiva: Option<f64>,
    /// Obrigatório a partir da segunda versão (5 a 300 caracteres).
    pub motivo: Option<String>,
}

/// NRD vigente de cada exame de TC e raio X do catálogo (médicos e gerência).
#[utoipa::path(get, operation_id = "listar_nrd", path = "/v1/nrd", tag = "exames",
    responses((status = 200, body = Vec<NrdDoExame>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn nrd(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<NrdDoExame>>, ErroApi> {
    usuario.exigir(Ato::DoseAgregadoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = sqlx::query_as!(
        NrdDoExame,
        r#"SELECT p.tuss, p.nome AS exame, p.modalidade, n.numero AS "numero?", n.grandeza AS "grandeza?", n.valor AS "valor?",
                  n.fator_dose_efetiva, n.motivo, n.criado_em AS "publicado_em?"
             FROM agenda.procedimento p
             LEFT JOIN exames.nrd n ON n.exame_tuss = p.tuss AND n.vigente
            WHERE p.ativo AND p.modalidade IN ('CT','DX','CR','MG','XA','RF')
            ORDER BY p.modalidade, p.nome"#
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(r))
}

fn nrd_mudou() -> ErroApi {
    ErroApi::conflito(
        "nrd-mudou",
        "Outra pessoa publicou um NRD para este exame enquanto você editava. Nada foi publicado. Recarregue e confira.",
    )
}

/// Publica versão nova do NRD de um exame (gerência; `Idempotency-Key`). Doses já lidas continuam
/// comparadas à versão aplicada na leitura.
#[utoipa::path(post, operation_id = "publicar_nrd", path = "/v1/nrd/{tuss}", tag = "exames",
    params(("tuss" = String, Path), ("Idempotency-Key" = Uuid, Header)), request_body = NovoNrd,
    responses((status = 201, body = NrdDoExame), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn publicar_nrd(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(tuss): Path<String>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoNrd>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::NrdAdministrar)?;
    let motivo = n
        .motivo
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_owned);
    let fator_ok = n
        .fator_dose_efetiva
        .is_none_or(|k| k.is_finite() && k > 0.0 && k <= 10.0);
    if !["ctdivol", "dlp", "dap"].contains(&n.grandeza.as_str())
        || !n.valor.is_finite()
        || n.valor <= 0.0
        || n.valor > 100_000.0
        || !fator_ok
    {
        return Err(ErroApi::invalido(
            "nrd-invalido",
            "Informe a grandeza (CTDIvol, DLP ou DAP) e um valor maior que zero; o fator, se houver, vai até 10. Nada foi publicado.",
        ));
    }
    if motivo
        .as_ref()
        .is_some_and(|m| !(5..=300).contains(&m.chars().count()))
        || (n.versao_vista.is_some() && motivo.is_none())
    {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Diga por que o NRD mudou (5 a 300 caracteres). Nada foi publicado.",
        ));
    }
    let rota = format!("POST /v1/nrd/{tuss}");
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
    let exame = sqlx::query!(
        "SELECT nome, modalidade FROM agenda.procedimento WHERE tuss = $1 AND ativo",
        tuss
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado("exame-nao-encontrado", "Este exame não está no catálogo.")
    })?;
    let tc = exame.modalidade.as_deref() == Some("CT");
    if tc == (n.grandeza == "dap") {
        return Err(ErroApi::invalido(
            "nrd-invalido",
            "Na tomografia o NRD é em CTDIvol ou DLP; no raio X, em DAP. Nada foi publicado.",
        ));
    }
    let atual = sqlx::query_scalar!(
        "SELECT numero FROM exames.nrd WHERE exame_tuss = $1 AND vigente FOR UPDATE",
        tuss
    )
    .fetch_optional(&mut *tx)
    .await?;
    if atual != n.versao_vista {
        return Err(nrd_mudou());
    }
    if let Some(numero) = atual {
        sqlx::query!(
            "UPDATE exames.nrd SET vigente = false WHERE exame_tuss = $1 AND numero = $2 AND vigente",
            tuss,
            numero
        )
        .execute(&mut *tx)
        .await?;
    }
    let numero = atual.unwrap_or(0) + 1;
    let id = novo_id();
    let codigo = proximo(&mut tx, "NRD").await?;
    sqlx::query!(
        "INSERT INTO exames.nrd (id, codigo, exame_tuss, numero, grandeza, valor, fator_dose_efetiva, motivo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'gestao_de_dose', $10, $11)",
        id, codigo.como_str(), tuss, numero, n.grandeza, n.valor, n.fator_dose_efetiva, motivo, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => nrd_mudou(),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "exames",
            tipo: "nrd_publicado",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "tuss": tuss, "numero": numero, "grandeza": n.grandeza, "valor": n.valor, "motivo": motivo }),
            criado_por: Some(usuario.id),
            origem: "gestao_de_dose",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(NrdDoExame {
        tuss,
        exame: exame.nome,
        modalidade: exame.modalidade,
        numero: Some(numero),
        grandeza: Some(n.grandeza),
        valor: Some(n.valor),
        fator_dose_efetiva: n.fator_dose_efetiva,
        motivo,
        publicado_em: Some(tempo::agora()),
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}
