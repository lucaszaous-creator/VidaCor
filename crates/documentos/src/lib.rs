//! Documentos do VidaCor em PDF (docs/11 § 9.1 `documentos/`, ADR-0002): laudo impresso e dossiê do
//! lastro, com Typst embutido como biblioteca. Este crate não conhece HTTP, SQL nem provedores (G5):
//! recebe os dados já autorizados e montados pelo núcleo e devolve os bytes do PDF com o SHA-256.
//!
//! Regras (docs/17 SEC-INJ-4): o modelo é código fixo do binário, versionado pelo nome
//! ([`MODELO_LAUDO`], [`MODELO_DOSSIE`]); o dado do paciente entra como dado JSON e é colocado no
//! documento como texto, nunca interpretado como marcação Typst.

mod dossie;
mod laudo;
mod motor;

pub use dossie::{dossie_pdf, DossieLastro, EventoDoDossie, NoDoDossie, MODELO_DOSSIE};
pub use laudo::{
    codigo_verificacao, laudo_pdf, AchadoImpresso, Assinatura, LaudoImpresso, MedicoImpresso,
    Retificacao, MODELO_LAUDO,
};
pub use motor::{Momento, Pdf};

use serde::Serialize;

/// Cabeçalho da clínica (configuração versionada por unidade). Campo ausente não é impresso.
#[derive(Debug, Clone, Serialize)]
pub struct Clinica {
    pub nome: String,
    pub endereco: Option<String>,
    pub telefone: Option<String>,
    pub site: Option<String>,
    pub cnpj: Option<String>,
    /// Ex.: "Dra. Fulana de Tal, CRM-SP 000000".
    pub responsavel_tecnico: Option<String>,
}

impl Clinica {
    /// Linhas do cabeçalho, na ordem impressa (só o que existe).
    pub(crate) fn linhas(&self) -> Vec<String> {
        let mut linhas = Vec::new();
        if let Some(e) = &self.endereco {
            linhas.push(e.clone());
        }
        let contato: Vec<String> = [
            self.telefone.as_ref().map(|t| format!("Telefone {t}")),
            self.site.clone(),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !contato.is_empty() {
            linhas.push(contato.join("    "));
        }
        if let Some(c) = &self.cnpj {
            linhas.push(format!("CNPJ {c}"));
        }
        if let Some(r) = &self.responsavel_tecnico {
            linhas.push(format!("Responsável técnico: {r}"));
        }
        linhas
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ErroDocumento {
    /// O modelo não compilou com estes dados (defeito do modelo: nunca é culpa de quem pediu).
    #[error("modelo do documento: {0}")]
    Modelo(String),
    #[error("dados do documento: {0}")]
    Dados(String),
}

/// Quebra um texto livre em parágrafos (linha em branco) e linhas, para o modelo imprimir as quebras
/// como estão, sem interpretar nada.
pub(crate) fn paragrafos(texto: &str) -> Vec<Vec<String>> {
    let normalizado = texto.replace("\r\n", "\n");
    normalizado
        .split("\n\n")
        .map(|p| {
            p.lines()
                .map(|l| l.trim_end().to_owned())
                .collect::<Vec<_>>()
        })
        .filter(|p| p.iter().any(|l| !l.trim().is_empty()))
        .collect()
}

/// Hash longo em quatro grupos de 16, para quebrar a linha numa tabela estreita.
pub(crate) fn hash_em_grupos(hash: &str) -> String {
    hash.as_bytes()
        .chunks(16)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}
