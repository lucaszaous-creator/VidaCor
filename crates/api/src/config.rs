//! Configuração por ambiente (12-Factor, docs/14 A1). Segredos só por variável de ambiente ou
//! cofre, nunca no repositório (G1).

use anyhow::{Context, Result};
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ConfigOidc {
    /// Emissor OIDC (realm do Keycloak), ex.: `http://127.0.0.1:8081/realms/vidacor`.
    pub emissor: String,
    pub cliente_id: String,
    pub cliente_segredo: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub endereco: SocketAddr,
    /// Conexão do papel `vidacor_app` (sem BYPASSRLS).
    pub banco_url: String,
    /// Conexão administrativa, só para rodar migrations em dev e homologação.
    pub banco_admin_url: Option<String>,
    /// Endereço público da aplicação (tela), para onde o login volta.
    pub url_aplicacao: String,
    /// Endereço público do núcleo, base do `redirect_uri` do OIDC.
    pub url_api: String,
    /// Cookie com `Secure`. Só fica desligado em `dev` sem TLS.
    pub cookie_seguro: bool,
    pub orthanc_url: Option<String>,
    /// Unidade a que o servidor local pertence (estudos recebidos do PACS).
    pub unidade_servidor: Option<Uuid>,
    pub oidc: Option<ConfigOidc>,
    /// Segredo HMAC do webhook do PSP (G3). Sem ele, todo webhook é recusado (falha fechada).
    pub pix_webhook_segredo: Option<String>,
    /// Segredo do convite do portal: o token é HMAC(segredo, id do convite) e não fica guardado.
    /// Sem ele, convidar responde 503 (falha fechada).
    pub convite_segredo: Option<String>,
    /// Segredo do app do WhatsApp para conferir o webhook (X-Hub-Signature-256). Sem ele, recusa.
    pub whatsapp_segredo: Option<String>,
}

fn var(nome: &str) -> Option<String> {
    std::env::var(nome).ok().filter(|v| !v.trim().is_empty())
}

impl Config {
    /// Lê a configuração das variáveis `VIDACOR_*`.
    ///
    /// # Errors
    /// Variável obrigatória ausente ou com formato inválido.
    pub fn do_ambiente() -> Result<Self> {
        let endereco = var("VIDACOR_ENDERECO")
            .unwrap_or_else(|| "127.0.0.1:8080".to_owned())
            .parse()
            .context("VIDACOR_ENDERECO inválido")?;
        let oidc = match (var("VIDACOR_OIDC_EMISSOR"), var("VIDACOR_OIDC_CLIENTE")) {
            (Some(emissor), Some(cliente_id)) => Some(ConfigOidc {
                emissor,
                cliente_id,
                cliente_segredo: var("VIDACOR_OIDC_SEGREDO"),
            }),
            _ => None,
        };
        let unidade_servidor = match var("VIDACOR_UNIDADE") {
            Some(v) => Some(v.parse().context("VIDACOR_UNIDADE não é UUID")?),
            None => None,
        };
        Ok(Self {
            endereco,
            banco_url: var("VIDACOR_BANCO_URL").context("VIDACOR_BANCO_URL ausente")?,
            banco_admin_url: var("VIDACOR_BANCO_ADMIN_URL"),
            url_aplicacao: var("VIDACOR_URL_APLICACAO")
                .unwrap_or_else(|| "http://127.0.0.1:5173".to_owned()),
            url_api: var("VIDACOR_URL_API").unwrap_or_else(|| "http://127.0.0.1:5173".to_owned()),
            cookie_seguro: var("VIDACOR_COOKIE_INSEGURO_DEV").is_none(),
            orthanc_url: var("VIDACOR_ORTHANC_URL"),
            unidade_servidor,
            oidc,
            pix_webhook_segredo: var("VIDACOR_PIX_WEBHOOK_SEGREDO"),
            convite_segredo: var("VIDACOR_CONVITE_SEGREDO").filter(|s| s.len() >= 32),
            whatsapp_segredo: var("VIDACOR_WHATSAPP_SEGREDO"),
        })
    }
}
