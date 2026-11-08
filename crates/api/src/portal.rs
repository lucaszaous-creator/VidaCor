//! Sessão do portal do paciente, separada da sessão da equipe (ADR-0011). O portal só vê o que foi
//! publicado e só do próprio paciente: rota e RLS por paciente (CLAUDE.md G1).

use crate::erro::ErroApi;
use crate::estado::Estado;
use axum::extract::FromRequestParts;
use axum::http::header;
use axum::http::request::Parts;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub const COOKIE: &str = "vc_portal";

pub struct Paciente {
    pub conta_id: Uuid,
    pub paciente_id: Uuid,
}

fn hash(s: &str) -> Vec<u8> {
    Sha256::digest(s.as_bytes()).to_vec()
}

/// Cria a sessão do paciente (usada pelo login do portal; o convite fica para a próxima entrega).
///
/// # Errors
/// Falha de banco.
pub async fn criar_sessao(banco: &PgPool, conta_id: Uuid) -> Result<String, sqlx::Error> {
    let mut buf = [0_u8; 32];
    rand::rng().fill_bytes(&mut buf);
    let segredo = URL_SAFE_NO_PAD.encode(buf);
    sqlx::query!(
        "INSERT INTO portal.sessao (id, conta_id, segredo_hash, csrf, expira_em) VALUES ($1, $2, $3, $4, now() + interval '30 minutes')",
        vidacor_comum::novo_id(), conta_id, hash(&segredo), URL_SAFE_NO_PAD.encode(&buf[..12])
    )
    .execute(banco)
    .await?;
    Ok(segredo)
}

/// Transação com o contexto do paciente para o RLS do portal.
///
/// # Errors
/// Falha de banco.
pub async fn transacao(
    banco: &PgPool,
    paciente_id: Uuid,
) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
    let mut tx = banco.begin().await?;
    sqlx::query("SELECT set_config('app.paciente_id', $1, true)")
        .bind(paciente_id.to_string())
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

impl FromRequestParts<Estado> for Paciente {
    type Rejection = ErroApi;

    async fn from_request_parts(
        parts: &mut Parts,
        estado: &Estado,
    ) -> Result<Self, Self::Rejection> {
        let segredo = parts
            .headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .map(str::trim)
            .find_map(|p| p.strip_prefix("vc_portal=").map(str::to_owned))
            .filter(|v| !v.is_empty() && v.len() <= 128)
            .ok_or_else(ErroApi::nao_autenticado)?;
        let l = sqlx::query!(
            "SELECT c.id, c.paciente_id FROM portal.sessao s JOIN portal.conta c ON c.id = s.conta_id
              WHERE s.segredo_hash = $1 AND s.revogada_em IS NULL AND s.expira_em > now() AND c.ativa",
            hash(&segredo)
        )
        .fetch_optional(&estado.banco)
        .await?
        .ok_or_else(ErroApi::nao_autenticado)?;
        Ok(Self {
            conta_id: l.id,
            paciente_id: l.paciente_id,
        })
    }
}
