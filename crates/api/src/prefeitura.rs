//! Porta da NFS-e (prefeitura; provedor externo: único ponto simulado nos testes, G4). Emissão e
//! cancelamento levam a chave de idempotência da operação (G3). O adaptador da prefeitura da clínica
//! entra quando o município e o certificado forem definidos: sem ele, a emissão fica no outbox (R7).

use crate::assinatura::Futuro;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NotaParaEmitir {
    pub tomador_nome: String,
    pub tomador_cpf: Option<String>,
    pub valor_centavos: i64,
    /// Genérica (sem nome de exame).
    pub descricao: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotaEmitida {
    pub numero: String,
    pub codigo_verificacao: String,
}

pub trait Prefeitura: Send + Sync {
    fn emitir<'a>(
        &'a self,
        chave: Uuid,
        nota: &'a NotaParaEmitir,
    ) -> Futuro<'a, anyhow::Result<NotaEmitida>>;
    fn cancelar<'a>(
        &'a self,
        chave: Uuid,
        numero: &'a str,
        motivo: &'a str,
    ) -> Futuro<'a, anyhow::Result<()>>;
}

/// Simulado em memória (testes e desenvolvimento): mesma chave, mesma nota; SEM VALOR FISCAL.
#[derive(Default)]
pub struct PrefeituraEmMemoria {
    notas: Mutex<HashMap<Uuid, NotaEmitida>>,
    pub canceladas: Mutex<Vec<String>>,
}

impl Prefeitura for PrefeituraEmMemoria {
    fn emitir<'a>(
        &'a self,
        chave: Uuid,
        _nota: &'a NotaParaEmitir,
    ) -> Futuro<'a, anyhow::Result<NotaEmitida>> {
        Box::pin(async move {
            let mut n = self
                .notas
                .lock()
                .map_err(|_| anyhow::anyhow!("prefeitura em memória indisponível"))?;
            let proximo = n.len() + 1;
            Ok(n.entry(chave)
                .or_insert_with(|| NotaEmitida {
                    numero: format!("DEV-{proximo:06}"),
                    codigo_verificacao: chave.simple().to_string()[..8].to_uppercase(),
                })
                .clone())
        })
    }
    fn cancelar<'a>(
        &'a self,
        _chave: Uuid,
        numero: &'a str,
        _motivo: &'a str,
    ) -> Futuro<'a, anyhow::Result<()>> {
        Box::pin(async move {
            let mut c = self
                .canceladas
                .lock()
                .map_err(|_| anyhow::anyhow!("prefeitura em memória indisponível"))?;
            if !c.iter().any(|x| x == numero) {
                c.push(numero.to_owned());
            }
            Ok(())
        })
    }
}
