//! Adaptador do Orthanc (ADR-0006): cria item de worklist e lê estudos recebidos pela API REST.
//! Rede com tempo limite (docs/14 A5); o Orthanc fica na VLAN dos servidores.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone)]
pub struct Orthanc {
    base: String,
    http: reqwest::Client,
}

pub struct ItemWorklist<'a> {
    pub accession: &'a str,
    pub paciente_codigo: &'a str,
    /// Nome no formato DICOM (`SOBRENOME^NOMES`).
    pub paciente_nome_dicom: &'a str,
    /// AAAAMMDD.
    pub nascimento: &'a str,
    pub ae_title: &'a str,
    /// Modalidade DICOM (US, DX, CT...).
    pub modalidade: &'a str,
    pub data_agendada: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct Mudanca {
    #[serde(rename = "ChangeType")]
    pub tipo: String,
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Seq")]
    pub seq: i64,
}

#[derive(Debug, Deserialize)]
pub struct Mudancas {
    #[serde(rename = "Changes")]
    pub itens: Vec<Mudanca>,
    #[serde(rename = "Last")]
    pub ultima: i64,
    #[serde(rename = "Done")]
    pub fim: bool,
}

pub struct EstudoOrthanc {
    pub orthanc_id: String,
    pub study_uid: String,
    pub accession: Option<String>,
    pub imagens: i32,
}

impl Orthanc {
    /// # Errors
    /// Falha ao montar o cliente HTTP.
    pub fn novo(base: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            base: base.trim_end_matches('/').to_owned(),
            http,
        })
    }

    /// Cria o item de worklist, sem duplicar: se o accession já está na worklist, não cria outro (G3).
    ///
    /// # Errors
    /// Orthanc fora do ar ou recusando o pedido.
    pub async fn criar_worklist(&self, item: &ItemWorklist<'_>) -> Result<()> {
        let existentes: Vec<Value> = self
            .http
            .get(format!("{}/worklists?format=Simplify", self.base))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let ja_existe = existentes
            .iter()
            .any(|w| w["Tags"]["AccessionNumber"].as_str() == Some(item.accession));
        if ja_existe {
            return Ok(());
        }
        let corpo = json!({ "Tags": {
            "AccessionNumber": item.accession,
            "PatientID": item.paciente_codigo,
            "PatientName": item.paciente_nome_dicom,
            "PatientBirthDate": item.nascimento,
            "ScheduledProcedureStepSequence": [{
                "Modality": item.modalidade,
                "ScheduledStationAETitle": item.ae_title,
                "ScheduledProcedureStepStartDate": item.data_agendada
            }]
        }});
        let resposta = self
            .http
            .post(format!("{}/worklists/create", self.base))
            .json(&corpo)
            .send()
            .await?;
        if !resposta.status().is_success() {
            bail!("Orthanc recusou a worklist: HTTP {}", resposta.status());
        }
        Ok(())
    }

    /// # Errors
    /// Orthanc fora do ar.
    pub async fn mudancas(&self, desde: i64) -> Result<Mudancas> {
        Ok(self
            .http
            .get(format!("{}/changes?since={desde}&limit=100", self.base))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// # Errors
    /// Orthanc fora do ar ou estudo sem StudyInstanceUID.
    pub async fn estudo(&self, orthanc_id: &str) -> Result<EstudoOrthanc> {
        // O id vem do próprio Orthanc; ainda assim só aceita o formato dele (SEC-INJ-5).
        if !orthanc_id
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-')
        {
            bail!("id de estudo do Orthanc em formato inesperado");
        }
        let estudo: Value = self
            .http
            .get(format!("{}/studies/{orthanc_id}", self.base))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let estatisticas: Value = self
            .http
            .get(format!("{}/studies/{orthanc_id}/statistics", self.base))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let tags = &estudo["MainDicomTags"];
        let imagens = estatisticas["CountInstances"]
            .as_i64()
            .or_else(|| {
                estatisticas["CountInstances"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
            })
            .unwrap_or(0);
        Ok(EstudoOrthanc {
            orthanc_id: orthanc_id.to_owned(),
            study_uid: tags["StudyInstanceUID"]
                .as_str()
                .context("estudo sem StudyInstanceUID")?
                .to_owned(),
            accession: tags["AccessionNumber"]
                .as_str()
                .map(str::to_owned)
                .filter(|a| !a.is_empty()),
            imagens: i32::try_from(imagens).unwrap_or(i32::MAX),
        })
    }
}

/// Converte "Antônio Carlos Ribeiro" em "RIBEIRO^ANTONIO CARLOS" (PN do DICOM, sem acento).
#[must_use]
pub fn nome_dicom(nome: &str) -> String {
    use unicode_normalization_shim::sem_acento;
    let limpo = sem_acento(nome).to_uppercase();
    let partes: Vec<&str> = limpo.split_whitespace().collect();
    match partes.split_last() {
        Some((ultimo, resto)) if !resto.is_empty() => format!("{ultimo}^{}", resto.join(" ")),
        _ => limpo,
    }
}

mod unicode_normalization_shim {
    pub fn sem_acento(texto: &str) -> String {
        vidacor_comum::NomeCompleto::novo(texto)
            .map(|n| n.forma_de_comparacao())
            .unwrap_or_else(|_| texto.to_owned())
    }
}

#[cfg(test)]
mod testes {
    #[test]
    fn nome_no_formato_dicom() {
        assert_eq!(
            super::nome_dicom("Antônio Carlos Ribeiro"),
            "RIBEIRO^ANTONIO CARLOS"
        );
    }
}
