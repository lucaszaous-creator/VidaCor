//! Equipe da unidade (nome e perfis), para escolher responsável e quem atendeu sem digitar
//! identificadores. Só dados de trabalho, sem contato nem documento (docs/18 § 6 item 5).

use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use axum::extract::State;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct Membro {
    pub id: Uuid,
    pub nome: String,
    pub perfis: Vec<String>,
}

/// Membros ativos da equipe da unidade do usuário.
#[utoipa::path(get, path = "/v1/equipe", tag = "sessao", responses((status = 200, body = Vec<Membro>), (status = 401, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<Membro>>, ErroApi> {
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let membros = sqlx::query_as!(
        Membro,
        "SELECT id, nome, perfis FROM identidade.usuario WHERE unidade_id = $1 AND ativo ORDER BY nome LIMIT 500",
        usuario.unidade_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(membros))
}
