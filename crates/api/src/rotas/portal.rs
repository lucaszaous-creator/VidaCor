//! Portal do paciente e link compartilhado (docs/27 FC-LAU-03, FC-ACE-01, 02 e 08; ADR-0011).
//! O paciente só vê o que é dele (rota e RLS); "não é seu" e "não existe" respondem igual.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::portal::{self, Paciente};
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Serialize, ToSchema)]
pub struct LaudoDoPortal {
    pub id: Uuid,
    pub exame: String,
    pub texto: String,
    /// A versão vigente substituiu uma anterior (retificação).
    pub retificado: bool,
    pub publicado_em: DateTime<Utc>,
}

fn nao_encontrado() -> ErroApi {
    ErroApi::nao_encontrado(
        "nao-encontrado",
        "Não encontramos este documento. Confira o endereço ou volte à lista dos seus exames.",
    )
}

/// Laudos publicados do próprio paciente (versão vigente).
#[utoipa::path(get, path = "/v1/portal/laudos", tag = "portal", responses((status = 200, body = Vec<LaudoDoPortal>)))]
pub async fn meus_laudos(
    State(estado): State<Estado>,
    paciente: Paciente,
) -> Result<Json<Vec<LaudoDoPortal>>, ErroApi> {
    let mut tx = portal::transacao(&estado.banco, paciente.paciente_id).await?;
    let linhas = sqlx::query_as!(
        LaudoDoPortal,
        "SELECT laudo_id AS id, exame, texto, retificado, publicado_em FROM portal.publicacao WHERE vigente ORDER BY publicado_em DESC LIMIT 100"
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(linhas))
}

/// Um laudo do próprio paciente. De outro paciente ou inexistente: a mesma resposta 404, auditada.
#[utoipa::path(get, path = "/v1/portal/laudos/{id}", tag = "portal", params(("id" = Uuid, Path)),
    responses((status = 200, body = LaudoDoPortal), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn meu_laudo(
    State(estado): State<Estado>,
    paciente: Paciente,
    Path(id): Path<Uuid>,
) -> Result<Json<LaudoDoPortal>, ErroApi> {
    let mut tx = portal::transacao(&estado.banco, paciente.paciente_id).await?;
    let laudo = sqlx::query_as!(
        LaudoDoPortal,
        "SELECT laudo_id AS id, exame, texto, retificado, publicado_em FROM portal.publicacao WHERE laudo_id = $1 AND vigente",
        id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let resultado = if laudo.is_some() {
        "permitido"
    } else {
        "negado"
    };
    sqlx::query!(
        "INSERT INTO auditoria.acesso (id, quem, quem_tipo, recurso, recurso_id, resultado, correlacao) VALUES ($1, $2, 'paciente', 'portal.laudo', $3, $4, $5)",
        novo_id(), paciente.paciente_id, id, resultado, crate::trace::atual().unwrap_or_default()
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    laudo.map(Json).ok_or_else(nao_encontrado)
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoLink {
    /// Validade em horas (1 a 72).
    pub horas: i64,
}

#[derive(Serialize, ToSchema)]
pub struct LinkCriado {
    /// Mostrado uma única vez; o banco guarda só o hash.
    pub token: String,
    pub expira_em: DateTime<Utc>,
}

/// Cria um link de leitura do laudo publicado para médico externo, com validade curta (SEC-DAD-6).
#[utoipa::path(post, path = "/v1/laudos/{id}/links", tag = "laudos", params(("id" = Uuid, Path)), request_body = PedidoLink,
    responses((status = 201, body = LinkCriado), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn criar_link(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoLink>,
) -> Result<(StatusCode, Json<LinkCriado>), ErroApi> {
    usuario.exigir(Ato::LaudoLer)?;
    if !(1..=72).contains(&p.horas) {
        return Err(ErroApi::invalido(
            "validade-invalida",
            "O link vale de 1 a 72 horas. Nenhum link foi criado.",
        ));
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    // O médico precisa poder ler o laudo (vínculo); o RLS do laudo decide.
    let laudo = sqlx::query!("SELECT id, unidade_id FROM laudos.laudo WHERE id = $1", id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            ErroApi::nao_encontrado(
                "laudo-nao-encontrado",
                "Laudo não encontrado ou sem vínculo de cuidado.",
            )
        })?;
    tx.commit().await?;
    let mut tx = banco::transacao(
        &estado.banco,
        &crate::banco::ContextoBanco::sistema(laudo.unidade_id),
    )
    .await?;
    let publicacao = sqlx::query_scalar!(
        "SELECT id FROM portal.publicacao WHERE laudo_id = $1 AND vigente",
        laudo.id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::conflito(
            "laudo-nao-publicado",
            "Só laudo assinado e liberado pode ser compartilhado. Nenhum link foi criado.",
        )
    })?;
    let mut buf = [0_u8; 32];
    rand::rng().fill_bytes(&mut buf);
    let token = URL_SAFE_NO_PAD.encode(buf);
    let expira_em = Utc::now() + Duration::hours(p.horas);
    sqlx::query!(
        "INSERT INTO portal.link (id, token_hash, publicacao_id, expira_em, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, 'consultorio', $6, $7)",
        novo_id(), Sha256::digest(token.as_bytes()).to_vec(), publicacao, expira_em, usuario.id, laudo.unidade_id,
        crate::trace::atual().unwrap_or_default()
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(LinkCriado { token, expira_em })))
}

/// Abre o laudo por link compartilhado. Expirado: 410, sem exibir nada (FC-ACE-08).
#[utoipa::path(get, path = "/v1/links/{token}", tag = "portal", params(("token" = String, Path)),
    responses((status = 200, body = LaudoDoPortal), (status = 404, body = crate::openapi::ProblemaDoc), (status = 410, body = crate::openapi::ProblemaDoc)))]
pub async fn abrir_link(
    State(estado): State<Estado>,
    Path(token): Path<String>,
) -> Result<Json<serde_json::Value>, ErroApi> {
    if token.len() > 100 {
        return Err(nao_encontrado());
    }
    let l = sqlx::query!(
        r#"SELECT situacao AS "situacao!", exame, texto, retificado, publicado_em FROM portal.ler_link($1)"#,
        Sha256::digest(token.as_bytes()).to_vec()
    )
    .fetch_optional(&estado.banco)
    .await?
    .ok_or_else(nao_encontrado)?;
    if l.situacao != "valido" {
        return Err(ErroApi::novo(StatusCode::GONE, "link-expirado", "Link expirado",
            "Este link de laudo expirou e não mostra mais o documento. Peça um novo link a quem compartilhou."));
    }
    Ok(Json(
        serde_json::json!({ "exame": l.exame, "texto": l.texto, "retificado": l.retificado, "publicado_em": l.publicado_em }),
    ))
}
