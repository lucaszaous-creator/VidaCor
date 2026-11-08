//! Porta do PSP do PIX (provedor externo: único ponto simulado nos testes, G4). A cobrança é criada
//! com chave de idempotência gerada uma vez por recebimento e reaproveitada nas retentativas (G3).

use crate::assinatura::Futuro;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Cobranca {
    pub txid: String,
    pub copia_e_cola: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstadoPix {
    pub pago: bool,
    pub end_to_end: Option<String>,
}

pub trait Psp: Send + Sync {
    fn criar_cobranca<'a>(
        &'a self,
        chave: Uuid,
        valor_centavos: i64,
        descricao: &'a str,
    ) -> Futuro<'a, anyhow::Result<Cobranca>>;
    /// Estado atual da cobrança no PSP (fonte da verdade para webhooks fora de ordem, FC-PIX-04).
    fn consultar<'a>(&'a self, txid: &'a str) -> Futuro<'a, anyhow::Result<EstadoPix>>;
    fn estornar<'a>(&'a self, chave: Uuid, txid: &'a str) -> Futuro<'a, anyhow::Result<()>>;
}

/// PSP de DESENVOLVIMENTO em memória: idempotente pela chave, sem dinheiro de verdade.
#[derive(Default)]
pub struct PspDesenvolvimento {
    cobrancas: Mutex<HashMap<Uuid, Cobranca>>,
    pagos: Mutex<HashMap<String, String>>,
    pub chamadas_de_criacao: std::sync::atomic::AtomicUsize,
}

impl PspDesenvolvimento {
    /// Simula o pagamento do pagador (usado em testes e na demonstração).
    pub fn marcar_pago(&self, txid: &str) -> String {
        let e2e = format!("E{}", &Uuid::now_v7().simple().to_string()[..20]);
        if let Ok(mut p) = self.pagos.lock() {
            p.insert(txid.to_owned(), e2e.clone());
        }
        e2e
    }
}

impl Psp for PspDesenvolvimento {
    fn criar_cobranca<'a>(
        &'a self,
        chave: Uuid,
        valor_centavos: i64,
        _descricao: &'a str,
    ) -> Futuro<'a, anyhow::Result<Cobranca>> {
        Box::pin(async move {
            self.chamadas_de_criacao
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut mapa = self
                .cobrancas
                .lock()
                .map_err(|_| anyhow::anyhow!("PSP indisponível"))?;
            let c = mapa.entry(chave).or_insert_with(|| {
                let txid = format!("VC{}", chave.simple());
                Cobranca {
                    copia_e_cola: format!(
                        "00020126DESENVOLVIMENTO-SEM-VALOR{txid}5204000053039865406{:.2}",
                        valor_centavos as f64 / 100.0
                    ),
                    txid,
                }
            });
            Ok(c.clone())
        })
    }

    fn consultar<'a>(&'a self, txid: &'a str) -> Futuro<'a, anyhow::Result<EstadoPix>> {
        Box::pin(async move {
            let pagos = self
                .pagos
                .lock()
                .map_err(|_| anyhow::anyhow!("PSP indisponível"))?;
            Ok(EstadoPix {
                pago: pagos.contains_key(txid),
                end_to_end: pagos.get(txid).cloned(),
            })
        })
    }

    fn estornar<'a>(&'a self, _chave: Uuid, _txid: &'a str) -> Futuro<'a, anyhow::Result<()>> {
        Box::pin(async move { Ok(()) })
    }
}
