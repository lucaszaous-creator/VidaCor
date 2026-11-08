use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::{self, Usuario};
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct UsuarioSessao {
    pub id: Uuid,
    pub nome: String,
    pub perfis: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct UnidadeSessao {
    pub id: Uuid,
    pub nome: String,
}

#[derive(Serialize, ToSchema)]
pub struct SessaoAtual {
    pub usuario: UsuarioSessao,
    pub unidade: UnidadeSessao,
    /// Valor a enviar no cabeçalho `X-CSRF-Token` em toda escrita.
    pub csrf: String,
}

/// Quem está conectado.
#[utoipa::path(get, path = "/v1/sessao", tag = "sessao",
    responses((status = 200, body = SessaoAtual), (status = 401, body = crate::openapi::ProblemaDoc)))]
pub async fn atual(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<SessaoAtual>, ErroApi> {
    let unidade = sqlx::query!(
        "SELECT nome FROM identidade.unidade WHERE id = $1",
        usuario.unidade_id
    )
    .fetch_one(&estado.banco)
    .await?;
    Ok(Json(SessaoAtual {
        usuario: UsuarioSessao {
            id: usuario.id,
            nome: usuario.nome,
            perfis: usuario.perfis,
        },
        unidade: UnidadeSessao {
            id: usuario.unidade_id,
            nome: unidade.nome,
        },
        csrf: usuario.csrf,
    }))
}

/// Encerra a sessão.
#[utoipa::path(post, path = "/v1/sessao/sair", tag = "sessao",
    params(("X-CSRF-Token" = String, Header)),
    responses((status = 204), (status = 401, body = crate::openapi::ProblemaDoc)))]
pub async fn sair(State(estado): State<Estado>, usuario: Usuario) -> Result<Response, ErroApi> {
    sessao::revogar(&estado.banco, usuario.sessao_id).await?;
    Ok((
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            sessao::cookie_apagado(estado.config.cookie_seguro),
        )],
    )
        .into_response())
}

/// Inicia o login no provedor de identidade (OIDC com PKCE, state e nonce).
#[utoipa::path(get, path = "/v1/sessao/entrar", tag = "sessao", responses((status = 303)))]
pub async fn entrar(State(estado): State<Estado>) -> Result<Response, ErroApi> {
    let oidc = estado.oidc.clone().ok_or_else(crate::oidc::indisponivel)?;
    let (url, state) = oidc.iniciar(&estado.banco).await?;
    // O `state` também fica num cookie curto deste navegador: o retorno só é aceito no mesmo
    // navegador que pediu o login (contra CSRF de login).
    Ok((
        [(
            header::SET_COOKIE,
            crate::oidc::cookie_login(&state, estado.config.cookie_seguro),
        )],
        Redirect::to(&url),
    )
        .into_response())
}

#[derive(Deserialize)]
pub struct Retorno {
    pub code: Option<String>,
    pub state: Option<String>,
}

/// Retorno do provedor de identidade: confere state, nonce e token e cria a sessão.
#[utoipa::path(get, path = "/v1/sessao/retorno", tag = "sessao", responses((status = 303)))]
pub async fn retorno(
    State(estado): State<Estado>,
    cabecalhos: axum::http::HeaderMap,
    Query(q): Query<Retorno>,
) -> Result<Response, ErroApi> {
    let oidc = estado.oidc.clone().ok_or_else(crate::oidc::indisponivel)?;
    let (code, state) = q.code.zip(q.state).ok_or_else(crate::oidc::falhou)?;
    // O retorno só vale no mesmo navegador que pediu o login (contra CSRF de login).
    if crate::oidc::ler_cookie_login(&cabecalhos).as_deref() != Some(state.as_str()) {
        return Err(crate::oidc::falhou());
    }
    let usuario = oidc.concluir(&estado.banco, &code, &state).await?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let nova = sessao::criar(&estado.banco, usuario.0, usuario.1, &correlacao).await?;
    let seguro = estado.config.cookie_seguro;
    Ok((
        axum::response::AppendHeaders([
            (header::SET_COOKIE, sessao::cookie(&nova.segredo, seguro)),
            (
                header::SET_COOKIE,
                crate::oidc::cookie_login_apagado(seguro),
            ),
        ]),
        Redirect::to(&estado.config.url_aplicacao),
    )
        .into_response())
}
