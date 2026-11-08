//! Extratores que devolvem erro em Problem Details em vez de texto solto.

use crate::erro::ErroApi;
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;
use uuid::Uuid;

/// Corpo JSON validado; erro de formato vira 422 com mensagem para a tela.
pub struct Corpo<T>(pub T);

impl<T, S> FromRequest<S> for Corpo<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ErroApi;

    async fn from_request(req: Request, estado: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, estado).await {
            Ok(axum::Json(valor)) => Ok(Self(valor)),
            Err(_) => Err(ErroApi::invalido(
                "corpo-invalido",
                "Os dados enviados estão incompletos ou em formato inesperado. Nada foi gravado. Recarregue a tela e tente de novo.",
            )),
        }
    }
}

/// Cabeçalho `Idempotency-Key` (G3): UUID gerado pela tela uma vez por operação.
pub struct ChaveIdempotencia(pub Uuid);

impl<S: Send + Sync> FromRequestParts<S> for ChaveIdempotencia {
    type Rejection = ErroApi;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let valor = parts.headers.get("idempotency-key").ok_or_else(|| {
            ErroApi::requisicao(
                "idempotency-key-ausente",
                "O pedido chegou sem a chave que evita duplicidade. Nada foi gravado. Recarregue a tela e tente de novo.",
            )
        })?;
        valor
            .to_str()
            .ok()
            .and_then(|v| Uuid::parse_str(v).ok())
            .map(Self)
            .ok_or_else(|| {
                ErroApi::requisicao(
                    "idempotency-key-invalida",
                    "A chave que evita duplicidade veio em formato inválido. Nada foi gravado. Recarregue a tela e tente de novo.",
                )
            })
    }
}
