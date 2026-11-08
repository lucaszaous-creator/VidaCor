//! Login pelo provedor de identidade (Keycloak, ADR-0010) com OIDC: código de autorização, PKCE
//! S256, `state` de uso único ligado ao navegador e `nonce` conferido no ID token (SEC-AUT-1, 2).
//! O VidaCor não guarda senha: o Keycloak autentica (e exige o 2FA); o núcleo decide os perfis.

use crate::erro::ErroApi;
use axum::http::StatusCode;
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct Oidc {
    // Chaves e endpoints do provedor. Trocam quando o Keycloak gira as chaves de assinatura.
    metadados: std::sync::RwLock<CoreProviderMetadata>,
    emissor: IssuerUrl,
    ultima_descoberta: std::sync::Mutex<std::time::Instant>,
    cliente_id: ClientId,
    cliente_segredo: Option<ClientSecret>,
    retorno: RedirectUrl,
    http: openidconnect::reqwest::Client,
}

#[must_use]
pub fn indisponivel() -> ErroApi {
    ErroApi::novo(
        StatusCode::SERVICE_UNAVAILABLE,
        "login-indisponivel",
        "Login indisponível",
        "O serviço de login não está configurado neste servidor. Nada foi alterado. Avise a TI.",
    )
}

#[must_use]
pub fn falhou() -> ErroApi {
    ErroApi::novo(
        StatusCode::UNAUTHORIZED,
        "login-recusado",
        "Login não concluído",
        "O login não foi concluído ou expirou. Nenhuma sessão foi aberta. Volte à tela inicial e entre de novo.",
    )
}

impl Oidc {
    /// Descobre o provedor (`.well-known/openid-configuration`).
    ///
    /// # Errors
    /// Provedor inacessível ou configuração inválida.
    pub async fn descobrir(cfg: &crate::config::ConfigOidc, url_api: &str) -> anyhow::Result<Self> {
        let http = openidconnect::reqwest::ClientBuilder::new()
            // Sem seguir redirecionamentos (recomendação da biblioteca contra SSRF).
            .redirect(openidconnect::reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let emissor = IssuerUrl::new(cfg.emissor.clone())?;
        let metadados = CoreProviderMetadata::discover_async(emissor.clone(), &http).await?;
        Ok(Self {
            metadados: std::sync::RwLock::new(metadados),
            emissor,
            ultima_descoberta: std::sync::Mutex::new(std::time::Instant::now()),
            cliente_id: ClientId::new(cfg.cliente_id.clone()),
            cliente_segredo: cfg.cliente_segredo.clone().map(ClientSecret::new),
            retorno: RedirectUrl::new(format!(
                "{}/v1/sessao/retorno",
                url_api.trim_end_matches('/')
            ))?,
            http,
        })
    }

    fn cliente(
        &self,
    ) -> CoreClient<
        openidconnect::EndpointSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointMaybeSet,
        openidconnect::EndpointMaybeSet,
    > {
        let metadados = match self.metadados.read() {
            Ok(m) => m.clone(),
            Err(envenenado) => envenenado.into_inner().clone(),
        };
        CoreClient::from_provider_metadata(
            metadados,
            self.cliente_id.clone(),
            self.cliente_segredo.clone(),
        )
        .set_redirect_uri(self.retorno.clone())
    }

    /// Gera a URL de login e guarda `state`, `nonce` e verificador PKCE por 10 minutos.
    ///
    /// # Errors
    /// Falha de banco.
    pub async fn iniciar(&self, banco: &PgPool) -> Result<(String, String), ErroApi> {
        let (desafio, verificador) = PkceCodeChallenge::new_random_sha256();
        let (url, state, nonce) = self
            .cliente()
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_owned()))
            .add_scope(Scope::new("profile".to_owned()))
            .set_pkce_challenge(desafio)
            .url();
        sqlx::query!(
            "INSERT INTO identidade.login_pendente (state, nonce, verificador_pkce, expira_em)
             VALUES ($1, $2, $3, now() + interval '10 minutes')",
            state.secret(),
            nonce.secret(),
            verificador.secret()
        )
        .execute(banco)
        .await?;
        Ok((url.to_string(), state.secret().clone()))
    }

    /// Troca o código pelo token, confere assinatura, emissor, audiência, validade e `nonce`, e
    /// devolve (usuário, unidade) do VidaCor. O `state` é consumido uma única vez (G2).
    ///
    /// # Errors
    /// [`falhou`] para qualquer divergência; falha de banco.
    pub async fn concluir(
        &self,
        banco: &PgPool,
        codigo: &str,
        state: &str,
    ) -> Result<(Uuid, Uuid), ErroApi> {
        let pendente = sqlx::query!(
            "UPDATE identidade.login_pendente SET usado_em = now()
              WHERE state = $1 AND usado_em IS NULL AND expira_em > now()
              RETURNING nonce, verificador_pkce",
            state
        )
        .fetch_optional(banco)
        .await?
        .ok_or_else(falhou)?;

        let cliente = self.cliente();
        let resposta = cliente
            .exchange_code(AuthorizationCode::new(codigo.to_owned()))
            .map_err(|e| {
                tracing::warn!(erro = %e, "provedor sem endpoint de token");
                falhou()
            })?
            .set_pkce_verifier(PkceCodeVerifier::new(pendente.verificador_pkce))
            .request_async(&self.http)
            .await
            .map_err(|e| {
                tracing::warn!(erro = %e, "troca de código recusada");
                falhou()
            })?;
        let id_token = resposta.id_token().ok_or_else(falhou)?;
        let nonce = Nonce::new(pendente.nonce);
        let declaracoes = match id_token.claims(&cliente.id_token_verifier(), &nonce) {
            Ok(d) => d.clone(),
            // Assinatura com chave desconhecida: o provedor pode ter girado as chaves. Busca as
            // chaves de novo (no máximo a cada 30 s) e confere uma única vez mais. Falha fechada.
            Err(openidconnect::ClaimsVerificationError::SignatureVerification(_))
                if self.redescobrir().await =>
            {
                let novo = self.cliente();
                let verificador = novo.id_token_verifier();
                let conferido = id_token.claims(&verificador, &nonce).cloned();
                conferido.map_err(|e| {
                    tracing::warn!(erro = %e, "ID token recusado após renovar as chaves");
                    falhou()
                })?
            }
            Err(e) => {
                tracing::warn!(erro = %e, "ID token recusado");
                return Err(falhou());
            }
        };
        let login = declaracoes
            .preferred_username()
            .map(|u| u.as_str().to_owned())
            .ok_or_else(falhou)?;

        let usuario = sqlx::query!(
            "SELECT id, unidade_id FROM identidade.usuario WHERE login = $1 AND ativo",
            login
        )
        .fetch_optional(banco)
        .await?
        .ok_or_else(|| {
            tracing::warn!("login sem usuário ativo no VidaCor");
            falhou()
        })?;
        Ok((usuario.id, usuario.unidade_id))
    }
}

impl Oidc {
    /// Busca de novo a configuração e as chaves do provedor. Devolve `true` se renovou.
    async fn redescobrir(&self) -> bool {
        {
            let Ok(mut ultima) = self.ultima_descoberta.lock() else {
                return false;
            };
            if ultima.elapsed() < std::time::Duration::from_secs(30) {
                return false;
            }
            *ultima = std::time::Instant::now();
        }
        match CoreProviderMetadata::discover_async(self.emissor.clone(), &self.http).await {
            Ok(novos) => {
                if let Ok(mut m) = self.metadados.write() {
                    *m = novos;
                    tracing::info!("chaves do provedor de identidade renovadas");
                    return true;
                }
                false
            }
            Err(e) => {
                tracing::warn!(erro = %e, "não foi possível renovar as chaves do provedor");
                false
            }
        }
    }
}

const COOKIE_LOGIN: &str = "vc_login";

/// Cookie curto com o `state` do login em andamento (Lax: precisa voltar no redirecionamento do provedor).
#[must_use]
pub fn cookie_login(state: &str, seguro: bool) -> String {
    let seguro = if seguro { "; Secure" } else { "" };
    format!("{COOKIE_LOGIN}={state}; Path=/v1/sessao; HttpOnly; SameSite=Lax; Max-Age=600{seguro}")
}

#[must_use]
pub fn cookie_login_apagado(seguro: bool) -> String {
    let seguro = if seguro { "; Secure" } else { "" };
    format!("{COOKIE_LOGIN}=; Path=/v1/sessao; HttpOnly; SameSite=Lax; Max-Age=0{seguro}")
}

#[must_use]
pub fn ler_cookie_login(cabecalhos: &axum::http::HeaderMap) -> Option<String> {
    cabecalhos
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .map(str::trim)
        .find_map(|par| par.strip_prefix("vc_login=").map(str::to_owned))
        .filter(|v| !v.is_empty() && v.len() <= 128)
}
