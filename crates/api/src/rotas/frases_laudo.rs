//! Frases rápidas do laudo (Modelos-Laudo.dc.html, aba Frases rápidas; DEC-2026-11-08-100). Cada médico
//! cria as suas; as da clínica, a coordenação médica. Atalho começa com ponto. Editar é nova frase que
//! substitui a anterior (que fica desativada com motivo); nada é apagado. Sem dado de paciente.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::rotas::modelos_laudo::{
    exigir_coordenacao_medica, parece_dado_de_paciente, repetida, texto_valido, PedidoDeDesativacao,
};
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct FraseDeLaudo {
    pub id: Uuid,
    pub codigo: String,
    /// clinica ou pessoal.
    pub tipo: String,
    pub atalho: String,
    pub texto: String,
    /// "Clínica" ou o nome do médico dono.
    pub dono: String,
    pub substitui_codigo: Option<String>,
    pub motivo: Option<String>,
    pub criado_em: DateTime<Utc>,
    pub pode_editar: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct NovaFraseDeLaudo {
    /// clinica ou pessoal.
    pub tipo: String,
    pub atalho: String,
    pub texto: String,
    /// Frase que esta nova versão substitui (com motivo).
    pub substitui_id: Option<Uuid>,
    pub motivo: Option<String>,
}

async fn carregar(
    tx: &mut PgConnection,
    usuario: &Usuario,
    id: Option<Uuid>,
) -> Result<Vec<FraseDeLaudo>, ErroApi> {
    let coordenacao = exigir_coordenacao_medica(usuario).is_ok();
    let linhas = sqlx::query!(
        r#"SELECT f.id, f.codigo, f.tipo, f.atalho, f.texto, f.dono_id, d.nome AS "dono_nome?", s.codigo AS "substitui_codigo?", f.motivo, f.criado_em
             FROM laudos.frase f
             LEFT JOIN identidade.usuario d ON d.id = f.dono_id
             LEFT JOIN laudos.frase s ON s.id = f.substitui_id
            WHERE f.ativo AND ($1::uuid IS NULL OR f.id = $1)
            ORDER BY f.atalho, f.tipo LIMIT 500"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| FraseDeLaudo {
            pode_editar: if l.tipo == "clinica" {
                coordenacao
            } else {
                l.dono_id == Some(usuario.id)
            },
            id: l.id,
            codigo: l.codigo,
            tipo: l.tipo,
            atalho: l.atalho,
            texto: l.texto,
            dono: l.dono_nome.unwrap_or_else(|| "Clínica".to_owned()),
            substitui_codigo: l.substitui_codigo,
            motivo: l.motivo,
            criado_em: l.criado_em,
        })
        .collect())
}

fn nao_encontrada() -> ErroApi {
    ErroApi::nao_encontrado(
        "frase-nao-encontrada",
        "Esta frase não existe, já foi substituída ou não é sua. Nada foi alterado.",
    )
}

/// Frases ativas: as da clínica e as do próprio médico.
#[utoipa::path(get, operation_id = "listar_frases_de_laudo", path = "/v1/frases-de-laudo", tag = "modelos-de-laudo",
    responses((status = 200, body = Vec<FraseDeLaudo>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<FraseDeLaudo>>, ErroApi> {
    usuario.exigir(Ato::ModeloLaudoUsar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let frases = carregar(&mut tx, &usuario, None).await?;
    tx.commit().await?;
    Ok(Json(frases))
}

/// Cria uma frase (ou nova versão de uma frase, que substitui a anterior com motivo). Pessoal: o
/// médico; da clínica: a coordenação médica. Idempotency-Key.
#[utoipa::path(post, operation_id = "criar_frase_de_laudo", path = "/v1/frases-de-laudo", tag = "modelos-de-laudo",
    params(("Idempotency-Key" = Uuid, Header)), request_body = NovaFraseDeLaudo,
    responses((status = 201, body = FraseDeLaudo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn criar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovaFraseDeLaudo>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::ModeloLaudoUsar)?;
    let clinica = match n.tipo.as_str() {
        "clinica" => {
            exigir_coordenacao_medica(&usuario)?;
            true
        }
        "pessoal" => false,
        _ => {
            return Err(ErroApi::invalido(
                "tipo-invalido",
                "A frase é da clínica ou pessoal. Nada foi criado.",
            ))
        }
    };
    let atalho = n.atalho.trim().to_lowercase();
    let atalho_ok = atalho.starts_with('.')
        && (3..=21).contains(&atalho.len())
        && atalho[1..]
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
    if !atalho_ok {
        return Err(ErroApi::invalido(
            "atalho-invalido",
            "O atalho começa com ponto e tem 2 a 20 letras sem acento, números, _ ou - (ex.: .normal). Nada foi criado.",
        ));
    }
    let texto = texto_valido(
        &n.texto,
        3,
        1000,
        "texto-invalido",
        "A frase tem 3 a 1000 caracteres. Nada foi criado.",
    )?;
    if parece_dado_de_paciente(&texto) {
        return Err(ErroApi::invalido(
            "modelo-com-dado-de-paciente",
            "O texto tem o que parece CPF ou data completa. Frase não leva dado de paciente. Nada foi criado.",
        ));
    }
    let motivo = match (n.substitui_id, n.motivo.as_deref()) {
        (Some(_), Some(m)) => Some(texto_valido(
            m,
            5,
            500,
            "motivo-obrigatorio",
            "Diga o que mudou na frase (5 a 500 letras). Nada foi criado.",
        )?),
        (Some(_), None) => {
            return Err(ErroApi::invalido(
                "motivo-obrigatorio",
                "Diga o que mudou na frase (5 a 500 letras). Nada foi criado.",
            ));
        }
        (None, _) => None,
    };
    let rota = "POST /v1/frases-de-laudo";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let id = novo_id();
    let codigo = proximo(&mut tx, "FRA").await?;
    if let Some(anterior) = n.substitui_id {
        carregar(&mut tx, &usuario, Some(anterior))
            .await?
            .pop()
            .filter(|f| f.pode_editar && f.tipo == n.tipo)
            .ok_or_else(nao_encontrada)?;
        let desativou = sqlx::query!(
            "UPDATE laudos.frase SET ativo = false, desativado_em = now(), desativado_por = $2, motivo_desativacao = $3, versao = versao + 1
              WHERE id = $1 AND ativo",
            anterior, usuario.id, format!("Substituída por {}: {}", codigo.como_str(), motivo.as_deref().unwrap_or_default())
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if desativou != 1 {
            return Err(ErroApi::conflito("frase-ja-substituida", "Esta frase acabou de ser alterada por outra pessoa. Nada foi criado. Recarregue as frases."));
        }
    }
    sqlx::query!(
        "INSERT INTO laudos.frase (id, codigo, tipo, dono_id, atalho, texto, substitui_id, motivo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'consultorio', $10, $11)",
        id, codigo.como_str(), n.tipo, (!clinica).then_some(usuario.id), atalho, texto, n.substitui_id, motivo, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("atalho-em-uso", "Este atalho já está em uso. Nada foi criado. Escolha outro ou edite a frase existente."),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "laudos",
            tipo: "frase_criada",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "tipo": n.tipo, "atalho": atalho, "substitui": n.substitui_id, "motivo": motivo }),
            criado_por: Some(usuario.id),
            origem: "consultorio",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let frase = carregar(&mut tx, &usuario, Some(id))
        .await?
        .pop()
        .ok_or_else(|| ErroApi::interno("frase sumiu"))?;
    let corpo = serde_json::to_value(frase).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Desativa a frase com motivo (pessoal: o dono; da clínica: a coordenação médica). Nada é apagado.
#[utoipa::path(post, operation_id = "desativar_frase_de_laudo", path = "/v1/frases-de-laudo/{id}/desativacao", tag = "modelos-de-laudo",
    params(("id" = Uuid, Path)), request_body = PedidoDeDesativacao,
    responses((status = 200, body = FraseDeLaudo), (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn desativar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoDeDesativacao>,
) -> Result<Json<FraseDeLaudo>, ErroApi> {
    usuario.exigir(Ato::ModeloLaudoUsar)?;
    let motivo = texto_valido(
        &p.motivo,
        5,
        500,
        "motivo-obrigatorio",
        "Diga por que a frase sai de uso (5 a 500 letras). Nada foi alterado.",
    )?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let mut frase = carregar(&mut tx, &usuario, Some(id))
        .await?
        .pop()
        .filter(|f| f.pode_editar)
        .ok_or_else(nao_encontrada)?;
    let n = sqlx::query!(
        "UPDATE laudos.frase SET ativo = false, desativado_em = now(), desativado_por = $2, motivo_desativacao = $3, versao = versao + 1
          WHERE id = $1 AND ativo",
        id, usuario.id, motivo
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if n != 1 {
        return Err(nao_encontrada());
    }
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "laudos",
            tipo: "frase_desativada",
            registro_id: id,
            registro_codigo: Some(&frase.codigo),
            dados: json!({ "motivo": motivo }),
            criado_por: Some(usuario.id),
            origem: "consultorio",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    frase.pode_editar = false;
    Ok(Json(frase))
}
