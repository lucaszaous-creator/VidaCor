//! Erro no formato Problem Details (RFC 9457), com código estável e mensagem em pt-BR (docs/14 A4, docs/16 R8).

use serde::Serialize;

/// Corpo de erro devolvido pela API (`application/problem+json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problema {
    /// URI do tipo de erro.
    #[serde(rename = "type")]
    pub tipo: String,
    /// Título curto.
    pub title: String,
    /// Código HTTP.
    pub status: u16,
    /// Mensagem para a tela: o que aconteceu, o que foi feito e o que fazer agora (R8).
    pub detail: String,
    /// Código estável para a tela e para o suporte.
    pub codigo: String,
    /// Código de suporte para localizar o caso (R8).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl Problema {
    #[must_use]
    pub fn novo(status: u16, codigo: &str, title: &str, detail: &str) -> Self {
        Self {
            tipo: format!("https://vidacor.local/erros/{codigo}"),
            title: title.to_owned(),
            status,
            detail: detail.to_owned(),
            codigo: codigo.to_owned(),
            trace_id: None,
        }
    }

    #[must_use]
    pub fn com_trace(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }
}
