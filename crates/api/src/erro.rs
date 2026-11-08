//! Erros da API em Problem Details (RFC 9457) com mensagem no formato R8: o que aconteceu, o que
//! já foi feito e o que fazer agora. Detalhe interno vai só para o log, nunca para a tela.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use vidacor_comum::Problema;

#[derive(Debug)]
pub struct ErroApi {
    // Em caixa: o erro viaja em `Result` por toda rota e não deve pesar no caminho feliz.
    problema: Box<Problema>,
}

impl ErroApi {
    #[must_use]
    pub fn novo(status: StatusCode, codigo: &str, titulo: &str, detalhe: &str) -> Self {
        Self {
            problema: Box::new(Problema::novo(status.as_u16(), codigo, titulo, detalhe)),
        }
    }

    #[must_use]
    pub fn status(&self) -> u16 {
        self.problema.status
    }

    #[must_use]
    pub fn codigo(&self) -> &str {
        &self.problema.codigo
    }

    #[must_use]
    pub fn nao_autenticado() -> Self {
        Self::novo(
            StatusCode::UNAUTHORIZED,
            "sessao-ausente",
            "Sessão encerrada",
            "Sua sessão terminou ou não foi iniciada. Nada foi alterado. Entre novamente para continuar.",
        )
    }

    #[must_use]
    pub fn proibido() -> Self {
        Self::novo(
            StatusCode::FORBIDDEN,
            "sem-permissao",
            "Sem permissão",
            "Seu perfil não permite esta ação. Nada foi alterado. Se precisar dela, fale com a coordenação do setor.",
        )
    }

    #[must_use]
    pub fn csrf() -> Self {
        Self::novo(
            StatusCode::FORBIDDEN,
            "csrf-invalido",
            "Pedido não confirmado",
            "O pedido não veio da tela do VidaCor ou a página está desatualizada. Nada foi alterado. Recarregue a página e tente de novo.",
        )
    }

    #[must_use]
    pub fn nao_encontrado(codigo: &str, detalhe: &str) -> Self {
        Self::novo(StatusCode::NOT_FOUND, codigo, "Não encontrado", detalhe)
    }

    #[must_use]
    pub fn conflito(codigo: &str, detalhe: &str) -> Self {
        Self::novo(StatusCode::CONFLICT, codigo, "Conflito", detalhe)
    }

    #[must_use]
    pub fn invalido(codigo: &str, detalhe: &str) -> Self {
        Self::novo(
            StatusCode::UNPROCESSABLE_ENTITY,
            codigo,
            "Dados não aceitos",
            detalhe,
        )
    }

    #[must_use]
    pub fn requisicao(codigo: &str, detalhe: &str) -> Self {
        Self::novo(
            StatusCode::BAD_REQUEST,
            codigo,
            "Pedido incompleto",
            detalhe,
        )
    }

    /// Falha interna: registra o detalhe no log com o `trace_id` e devolve mensagem genérica.
    pub fn interno(erro: impl std::fmt::Display) -> Self {
        tracing::error!(erro = %erro, "falha interna");
        Self::novo(
            StatusCode::INTERNAL_SERVER_ERROR,
            "falha-interna",
            "Falha no servidor",
            "O servidor não conseguiu concluir o pedido. Nada foi gravado. Tente de novo; se continuar, informe o código de suporte à TI.",
        )
    }
}

impl From<sqlx::Error> for ErroApi {
    fn from(erro: sqlx::Error) -> Self {
        Self::interno(erro)
    }
}

impl IntoResponse for ErroApi {
    fn into_response(self) -> Response {
        let mut problema = *self.problema;
        if let Some(id) = crate::trace::atual() {
            problema = problema.com_trace(id);
        }
        let status =
            StatusCode::from_u16(problema.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let corpo = serde_json::to_vec(&problema).unwrap_or_default();
        let mut resposta = (status, corpo).into_response();
        resposta.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        resposta
    }
}
