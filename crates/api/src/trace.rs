//! `trace_id` por requisição: vai no cabeçalho `x-trace-id`, nos logs e no corpo dos erros (R8).

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument;

tokio::task_local! {
    static TRACE_ID: String;
}

/// `trace_id` da requisição em andamento, se houver.
#[must_use]
pub fn atual() -> Option<String> {
    TRACE_ID.try_with(Clone::clone).ok()
}

pub async fn middleware(req: Request, next: Next) -> Response {
    let id = uuid::Uuid::now_v7().simple().to_string();
    // Só método e caminho: nada de corpo, consulta ou cabeçalho (sem dado pessoal no log).
    let span = tracing::info_span!(
        "requisicao",
        trace_id = %id,
        metodo = %req.method(),
        caminho = %req.uri().path()
    );
    let mut resposta = TRACE_ID
        .scope(
            id.clone(),
            async move {
                let r = next.run(req).await;
                tracing::info!(status = r.status().as_u16(), "resposta");
                r
            }
            .instrument(span),
        )
        .await;
    if let Ok(valor) = HeaderValue::from_str(&id) {
        resposta.headers_mut().insert("x-trace-id", valor);
    }
    resposta
}
