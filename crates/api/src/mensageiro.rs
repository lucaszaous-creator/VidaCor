//! Porta de WhatsApp (provedor externo: único ponto simulado nos testes, G4). Só modelos aprovados,
//! sem dado de saúde; a chave de idempotência da operação vai junto (G3). O adaptador real (WhatsApp
//! Cloud API) entra quando a clínica tiver a conta comercial: sem ele, os avisos ficam no outbox (R7).

use crate::assinatura::Futuro;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MensagemWhatsapp {
    /// Número no formato internacional, só dígitos (ex.: 5511987654321).
    pub para: String,
    pub modelo: String,
    pub parametros: Vec<String>,
}

pub trait Mensageiro: Send + Sync {
    /// Devolve o ID da mensagem no provedor.
    fn enviar<'a>(
        &'a self,
        chave: Uuid,
        m: &'a MensagemWhatsapp,
    ) -> Futuro<'a, anyhow::Result<String>>;
}

/// Simulado em memória (testes e desenvolvimento): nada sai da máquina; mesma chave, mesma mensagem.
#[derive(Default)]
pub struct MensageiroEmMemoria {
    pub enviadas: Mutex<Vec<(Uuid, MensagemWhatsapp)>>,
}

impl Mensageiro for MensageiroEmMemoria {
    fn enviar<'a>(
        &'a self,
        chave: Uuid,
        m: &'a MensagemWhatsapp,
    ) -> Futuro<'a, anyhow::Result<String>> {
        Box::pin(async move {
            let mut v = self
                .enviadas
                .lock()
                .map_err(|_| anyhow::anyhow!("mensageiro em memória indisponível"))?;
            if !v.iter().any(|(c, _)| *c == chave) {
                v.push((chave, m.clone()));
            }
            Ok(format!("wamid.dev.{chave}"))
        })
    }
}

/// WhatsApp Cloud API da Meta (mensagem de modelo aprovado). Configurado por ambiente:
/// `VIDACOR_WHATSAPP_API_URL` (padrão `https://graph.facebook.com/v21.0`),
/// `VIDACOR_WHATSAPP_NUMERO_ID` (ID do número de telefone na Meta) e `VIDACOR_WHATSAPP_TOKEN`
/// (token de sistema; segredo, só no ambiente do servidor, nunca no código nem no navegador).
/// A Cloud API não tem chave de idempotência: a repetição é barrada antes, pela mensagem única por
/// modelo e referência e pelo outbox (G3).
pub struct MensageiroCloudApi {
    http: reqwest::Client,
    url: String,
    token: String,
    idioma: String,
}

impl MensageiroCloudApi {
    /// # Errors
    /// Endereço inválido ou cliente HTTP indisponível.
    pub fn novo(api_url: &str, numero_id: &str, token: &str) -> anyhow::Result<Self> {
        if numero_id.is_empty() || !numero_id.bytes().all(|b| b.is_ascii_digit()) {
            anyhow::bail!("VIDACOR_WHATSAPP_NUMERO_ID deve ter só dígitos");
        }
        let base = reqwest::Url::parse(api_url)?;
        if base.scheme() != "https" && base.host_str() != Some("127.0.0.1") {
            anyhow::bail!("a API do WhatsApp precisa de https");
        }
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()?,
            url: format!("{}/{numero_id}/messages", api_url.trim_end_matches('/')),
            token: token.to_owned(),
            idioma: "pt_BR".to_owned(),
        })
    }

    /// Corpo da requisição de mensagem de modelo (público para teste).
    #[must_use]
    pub fn corpo(&self, m: &MensagemWhatsapp) -> serde_json::Value {
        let parametros: Vec<serde_json::Value> = m
            .parametros
            .iter()
            .map(|p| serde_json::json!({ "type": "text", "text": p }))
            .collect();
        serde_json::json!({
            "messaging_product": "whatsapp",
            "to": m.para,
            "type": "template",
            "template": {
                "name": m.modelo,
                "language": { "code": self.idioma },
                "components": [{ "type": "body", "parameters": parametros }],
            },
        })
    }
}

impl Mensageiro for MensageiroCloudApi {
    fn enviar<'a>(
        &'a self,
        _chave: Uuid,
        m: &'a MensagemWhatsapp,
    ) -> Futuro<'a, anyhow::Result<String>> {
        Box::pin(async move {
            let resposta = self
                .http
                .post(&self.url)
                .bearer_auth(&self.token)
                .json(&self.corpo(m))
                .send()
                .await?;
            let status = resposta.status();
            let corpo: serde_json::Value = resposta.json().await.unwrap_or_default();
            if !status.is_success() {
                // Só o código de erro do provedor; o corpo pode ecoar o número do paciente.
                let codigo = corpo["error"]["code"].as_i64().unwrap_or_default();
                anyhow::bail!("WhatsApp recusou o envio (HTTP {status}, código {codigo})");
            }
            corpo["messages"][0]["id"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow::anyhow!("resposta do WhatsApp sem id da mensagem"))
        })
    }
}
