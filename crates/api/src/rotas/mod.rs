//! Rotas `/v1` (docs/29 § 6.2). Toda rota declara o ato exigido no próprio handler.

pub mod agenda;
pub mod atendimentos;
pub mod eventos;
pub mod saude;
pub mod sessao;

use crate::estado::Estado;
use axum::routing::{get, post};
use axum::Router;

pub fn rotas() -> Router<Estado> {
    Router::new()
        .route("/v1/saude", get(saude::saude))
        .route("/v1/sessao", get(sessao::atual))
        .route("/v1/sessao/entrar", get(sessao::entrar))
        .route("/v1/sessao/retorno", get(sessao::retorno))
        .route("/v1/sessao/sair", post(sessao::sair))
        .route("/v1/salas", get(agenda::salas))
        .route("/v1/agenda", get(agenda::agenda_do_dia))
        .route("/v1/atendimentos", post(atendimentos::checkin))
        .route("/v1/atendimentos/{id}", get(atendimentos::obter))
        .route("/v1/eventos", get(eventos::fluxo))
        .fallback(nao_existe)
}

async fn nao_existe() -> crate::erro::ErroApi {
    crate::erro::ErroApi::nao_encontrado(
        "rota-inexistente",
        "Este endereço não existe no VidaCor. Nada foi feito. Volte para a tela inicial.",
    )
}
