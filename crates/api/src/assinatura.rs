//! Porta de assinatura digital (ADR-0008): o serviço de assinatura ICP-Brasil (JVM) fica atrás desta
//! interface. Em desenvolvimento e nos testes usa-se o assinador de desenvolvimento, que não tem
//! valor legal e diz isso no próprio certificado. Provedor externo: o único ponto em que mock é
//! permitido (G4).

use chrono::{DateTime, Utc};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct Assinatura {
    pub certificado: String,
    pub carimbo_tempo: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum ErroAssinatura {
    #[error("PIN do certificado recusado")]
    PinRecusado,
    #[error("provedor de assinatura indisponível: {0}")]
    Indisponivel(String),
}

pub type Futuro<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Assinador: Send + Sync {
    /// Assina o hash do documento com o certificado do médico, liberado pelo PIN.
    fn assinar<'a>(
        &'a self,
        medico_id: uuid::Uuid,
        pin: &'a str,
        hash_documento: &'a str,
    ) -> Futuro<'a, Result<Assinatura, ErroAssinatura>>;
}

/// Assinador de DESENVOLVIMENTO: aceita PIN de 4 a 8 dígitos, recusa "0000". Sem valor legal.
pub struct AssinadorDesenvolvimento;

impl Assinador for AssinadorDesenvolvimento {
    fn assinar<'a>(
        &'a self,
        medico_id: uuid::Uuid,
        pin: &'a str,
        hash_documento: &'a str,
    ) -> Futuro<'a, Result<Assinatura, ErroAssinatura>> {
        Box::pin(async move {
            let valido = (4..=8).contains(&pin.len())
                && pin.chars().all(|c| c.is_ascii_digit())
                && pin != "0000";
            if !valido {
                return Err(ErroAssinatura::PinRecusado);
            }
            Ok(Assinatura {
                certificado: format!(
                    "DESENVOLVIMENTO SEM VALOR LEGAL; medico={medico_id}; documento={}",
                    &hash_documento[..16.min(hash_documento.len())]
                ),
                carimbo_tempo: Utc::now(),
            })
        })
    }
}
