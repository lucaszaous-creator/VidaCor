//! Núcleo VidaCor: rotas HTTP, sessão, permissão, adaptadores e outbox (docs/11, docs/29).

#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]

pub mod assinatura;
pub mod atos;
pub mod banco;
pub mod config;
pub mod correio;
pub mod erro;
pub mod estado;
pub mod eventos;
pub mod extratores;
pub mod idempotencia;
pub mod lastro;
pub mod laudos;
pub mod oidc;
pub mod openapi;
pub mod operadora;
pub mod orthanc;
pub mod outbox;
pub mod pix;
pub mod portal;
pub mod rotas;
pub mod sessao;
pub mod tempo;
pub mod trace;
pub mod vinculo;

use axum::http::{header, HeaderValue};
use axum::Router;
use std::time::Duration;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;

pub use estado::Estado;

/// Limite do corpo das requisições (SEC-DIS-1).
pub const LIMITE_CORPO: usize = 64 * 1024;

/// Monta a aplicação com todas as rotas e as camadas transversais.
pub fn aplicacao(estado: Estado) -> Router {
    rotas::rotas()
        .with_state(estado)
        // Nenhuma resposta da API vai para cache (SEC-AUT-8, dado de saúde fora do cache do navegador).
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(RequestBodyLimitLayer::new(LIMITE_CORPO))
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(15),
        ))
        .layer(axum::middleware::from_fn(trace::middleware))
}
