//! Porta da consulta ao CFM (situação do CRM no conselho; provedor externo: único ponto simulado nos
//! testes, G4). O adaptador do webservice do CFM entra quando a clínica tiver o convênio e a chave de
//! acesso: o contrato do serviço ainda não foi recebido, então nada é inventado aqui. Sem adaptador, a
//! gerência registra a consulta feita no portal do CFM (fonte `portal_cfm`). DEC-2026-11-09-120.

use crate::assinatura::Futuro;
use std::collections::HashMap;
use std::sync::Mutex;

/// Situações do registro no conselho aceitas pelo núcleo (enum fechado; desconhecida é recusada).
pub const SITUACOES: [&str; 8] = [
    "regular",
    "suspenso",
    "cassado",
    "cancelado",
    "falecido",
    "transferido",
    "interdicao_cautelar",
    "nao_encontrado",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RespostaDoCfm {
    /// Uma das [`SITUACOES`].
    pub situacao: String,
    pub nome: Option<String>,
    /// Especialidades com RQE como o conselho devolve.
    pub especialidades: Vec<String>,
    /// SHA-256 (hex) da resposta bruta do serviço, guardado como comprovante.
    pub resposta_sha256: String,
}

pub trait Cfm: Send + Sync {
    fn consultar<'a>(
        &'a self,
        uf: &'a str,
        numero: &'a str,
    ) -> Futuro<'a, anyhow::Result<RespostaDoCfm>>;
}

/// Simulado em memória (testes e desenvolvimento): SEM VALOR DE CONFERÊNCIA. Registro não cadastrado
/// volta como `nao_encontrado`.
#[derive(Default)]
pub struct CfmEmMemoria {
    pub registros: Mutex<HashMap<(String, String), RespostaDoCfm>>,
    /// Em desenvolvimento, todo registro volta regular.
    pub tudo_regular: bool,
}

impl Cfm for CfmEmMemoria {
    fn consultar<'a>(
        &'a self,
        uf: &'a str,
        numero: &'a str,
    ) -> Futuro<'a, anyhow::Result<RespostaDoCfm>> {
        Box::pin(async move {
            let r = self
                .registros
                .lock()
                .map_err(|_| anyhow::anyhow!("CFM em memória indisponível"))?;
            let padrao = RespostaDoCfm {
                situacao: if self.tudo_regular {
                    "regular"
                } else {
                    "nao_encontrado"
                }
                .to_owned(),
                nome: None,
                especialidades: Vec::new(),
                resposta_sha256: "0".repeat(64),
            };
            Ok(r.get(&(uf.to_owned(), numero.to_owned()))
                .cloned()
                .unwrap_or(padrao))
        })
    }
}
