use crate::erro::ErroApi;
use crate::estado::Estado;
use axum::extract::State;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct Saude {
    /// `ok` quando o núcleo responde e o banco aceita consultas.
    pub situacao: &'static str,
}

/// Verificação de vida e prontidão.
#[utoipa::path(get, path = "/v1/saude", tag = "sistema", responses((status = 200, body = Saude)))]
pub async fn saude(State(estado): State<Estado>) -> Result<Json<Saude>, ErroApi> {
    sqlx::query_scalar!("SELECT 1 AS \"ok!\"")
        .fetch_one(&estado.banco)
        .await?;
    Ok(Json(Saude { situacao: "ok" }))
}
