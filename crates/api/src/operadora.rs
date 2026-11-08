//! Porta do webservice TISS da operadora (provedor externo: único ponto simulado nos testes, G4).
//! O lote é enviado com a chave de idempotência gerada uma vez na criação do lote e reaproveitada
//! nas retentativas (G3). Sem operadora configurada, o envio fica pendente no outbox, visível na
//! tela do lote (R7).

use crate::assinatura::Futuro;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

/// Protocolo de recebimento devolvido pela operadora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protocolo {
    pub numero: String,
}

pub trait Operadora: Send + Sync {
    /// Envia o arquivo do lote (`tissLoteGuias`) à operadora de registro ANS `registro_ans`.
    fn enviar_lote<'a>(
        &'a self,
        chave: Uuid,
        registro_ans: &'a str,
        arquivo: &'a [u8],
    ) -> Futuro<'a, anyhow::Result<Protocolo>>;
}

/// Operadora de DESENVOLVIMENTO em memória: idempotente pela chave, sem envio de verdade.
#[derive(Default)]
pub struct OperadoraDesenvolvimento {
    protocolos: Mutex<HashMap<Uuid, Protocolo>>,
    /// Quantas vezes o envio foi chamado (inclusive repetições com a mesma chave).
    pub chamadas: std::sync::atomic::AtomicUsize,
}

impl Operadora for OperadoraDesenvolvimento {
    fn enviar_lote<'a>(
        &'a self,
        chave: Uuid,
        _registro_ans: &'a str,
        arquivo: &'a [u8],
    ) -> Futuro<'a, anyhow::Result<Protocolo>> {
        Box::pin(async move {
            self.chamadas
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            anyhow::ensure!(!arquivo.is_empty(), "arquivo vazio");
            let mut mapa = self
                .protocolos
                .lock()
                .map_err(|_| anyhow::anyhow!("operadora indisponível"))?;
            let n = mapa.len() + 1;
            Ok(mapa
                .entry(chave)
                .or_insert_with(|| Protocolo {
                    numero: format!("DEV-{n:08}"),
                })
                .clone())
        })
    }
}

impl OperadoraDesenvolvimento {
    /// Protocolos distintos emitidos (um por chave).
    #[must_use]
    pub fn protocolos_emitidos(&self) -> usize {
        self.protocolos.lock().map(|m| m.len()).unwrap_or(0)
    }
}
