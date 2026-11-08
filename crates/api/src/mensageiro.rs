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
