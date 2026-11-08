//! Porta de e-mail (provedor externo: único ponto simulado nos testes, G4). A mensagem não leva dado
//! de saúde (docs/08). O `Message-ID` sai da chave de idempotência da operação (G3): reenvio pelo
//! outbox depois de uma falha usa o mesmo ID, e o servidor de e-mail ou o leitor reconhecem a cópia.

use crate::assinatura::Futuro;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mensagem {
    pub para: String,
    pub assunto: String,
    pub texto: String,
}

pub trait Correio: Send + Sync {
    fn enviar<'a>(&'a self, chave: Uuid, m: &'a Mensagem) -> Futuro<'a, anyhow::Result<()>>;
}

/// Correio de DESENVOLVIMENTO e testes: guarda as mensagens em memória (nada sai da máquina).
#[derive(Default)]
pub struct CorreioEmMemoria {
    pub enviadas: Mutex<Vec<(Uuid, Mensagem)>>,
}

impl Correio for CorreioEmMemoria {
    fn enviar<'a>(&'a self, chave: Uuid, m: &'a Mensagem) -> Futuro<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let mut v = self
                .enviadas
                .lock()
                .map_err(|_| anyhow::anyhow!("correio em memória indisponível"))?;
            // Mesma chave: o provedor não duplica.
            if !v.iter().any(|(c, _)| *c == chave) {
                v.push((chave, m.clone()));
            }
            Ok(())
        })
    }
}

/// SMTP (em desenvolvimento, o Mailpit do compose; em produção, o relay da clínica com TLS).
pub struct CorreioSmtp {
    transporte: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
    de: lettre::message::Mailbox,
}

impl CorreioSmtp {
    /// `url`: `smtp://host:porta` (sem TLS, só local) ou `smtps://usuario:senha@host:465`.
    ///
    /// # Errors
    /// URL ou remetente inválidos.
    pub fn novo(url: &str, de: &str) -> anyhow::Result<Self> {
        Ok(Self {
            transporte: lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::from_url(url)?
                .build(),
            de: de.parse()?,
        })
    }
}

impl Correio for CorreioSmtp {
    fn enviar<'a>(&'a self, chave: Uuid, m: &'a Mensagem) -> Futuro<'a, anyhow::Result<()>> {
        use lettre::AsyncTransport;
        Box::pin(async move {
            let email = lettre::Message::builder()
                .from(self.de.clone())
                .to(m.para.parse()?)
                .subject(m.assunto.clone())
                .message_id(Some(format!("<{chave}@vidacor.local>")))
                .header(lettre::message::header::ContentType::TEXT_PLAIN)
                .body(m.texto.clone())?;
            self.transporte.send(email).await?;
            Ok(())
        })
    }
}
