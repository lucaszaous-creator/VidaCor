//! Fluxo SSE para as telas (REQ-EXA-003): cada evento só chega a quem pode vê-lo (SEC-CLI-5).

use crate::estado::Estado;
use crate::sessao::Usuario;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::Stream;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// Eventos em tempo real (ex.: `estudo-recebido`).
#[utoipa::path(get, path = "/v1/eventos", tag = "eventos",
    responses((status = 200, content_type = "text/event-stream"), (status = 401, body = crate::openapi::ProblemaDoc)))]
pub async fn fluxo(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let unidade = usuario.unidade_id;
    let perfis = usuario.perfis;
    let fluxo = BroadcastStream::new(estado.eventos.subscribe()).filter_map(move |item| {
        let evento = item.ok()?;
        if !evento.pode_ver(unidade, &perfis) {
            return None;
        }
        Some(Ok(Event::default()
            .event(evento.tipo)
            .data(evento.dados.to_string())))
    });
    Sse::new(fluxo).keep_alive(KeepAlive::default())
}
