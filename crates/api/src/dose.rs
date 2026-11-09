//! Leitura da dose de radiação do RDSR (DICOM SR de dose) que o aparelho manda ao PACS (docs/21 § 3,
//! Anvisa RDC 611/2022; DEC-2026-11-09-144 a 146). Adaptador de borda: conhece DICOM; o resto do
//! núcleo só vê os números (CTDIvol, DLP, DAP) já convertidos e o SHA-256 das tags lidas.
//!
//! Códigos DCM do modelo TID 10011/10007 usados: 113830 (CTDIvol médio), 113838 (DLP do evento),
//! 113813 (DLP total), 113722 (DAP total), 113725 (DAP do evento).

use crate::banco::{self, ContextoBanco};
use crate::estado::Estado;
use crate::lastro::{self, NovoEvento};
use crate::orthanc::Orthanc;
use anyhow::Result;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Valores de dose de um RDSR, nas unidades da tela: mGy, mGy·cm e Gy·cm².
#[derive(Debug, Default, PartialEq)]
pub struct DoseLida {
    pub ctdivol_mgy: Option<f64>,
    pub dlp_mgycm: Option<f64>,
    pub dap_gycm2: Option<f64>,
}

impl DoseLida {
    #[must_use]
    pub fn vazia(&self) -> bool {
        self.ctdivol_mgy.is_none() && self.dlp_mgycm.is_none() && self.dap_gycm2.is_none()
    }
}

fn numero(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        Value::Array(a) => a.first().and_then(numero),
        _ => None,
    }
    .filter(|x: &f64| x.is_finite() && *x >= 0.0)
}

fn codigo(item: &Value) -> Option<&str> {
    let c = item["ConceptNameCodeSequence"].as_array()?.first()?;
    (c["CodingSchemeDesignator"].as_str() == Some("DCM"))
        .then(|| c["CodeValue"].as_str())
        .flatten()
}

/// Valor medido e unidade (UCUM) de um item de conteúdo.
fn medida(item: &Value) -> Option<(f64, String)> {
    let m = item["MeasuredValueSequence"].as_array()?.first()?;
    let v = numero(&m["NumericValue"])?;
    let u = m["MeasurementUnitsCodeSequence"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|u| u["CodeValue"].as_str())
        .unwrap_or("")
        .to_owned();
    Some((v, u))
}

/// DAP para Gy·cm² a partir da unidade informada (o RDSR usa Gy.m2 no total).
fn dap_em_gycm2(v: f64, unidade: &str) -> Option<f64> {
    match unidade {
        "Gy.m2" => Some(v * 10_000.0),
        "Gy.cm2" | "" => Some(v),
        "dGy.cm2" => Some(v / 10.0),
        "cGy.cm2" => Some(v / 100.0),
        "mGy.cm2" => Some(v / 1000.0),
        "uGy.m2" => Some(v / 100.0),
        _ => None,
    }
}

fn percorrer(item: &Value, acum: &mut Acumulado) {
    if let Some(c) = codigo(item) {
        if let Some((v, u)) = medida(item) {
            match c {
                "113830" => acum.ctdivol_max = Some(acum.ctdivol_max.map_or(v, |m: f64| m.max(v))),
                "113838" => acum.dlp_eventos += v,
                "113813" => acum.dlp_total = Some(v),
                "113722" => acum.dap_total = dap_em_gycm2(v, &u).or(acum.dap_total),
                "113725" => acum.dap_eventos += dap_em_gycm2(v, &u).unwrap_or(0.0),
                _ => {}
            }
        }
    }
    if let Some(filhos) = item["ContentSequence"].as_array() {
        for f in filhos.iter().take(5000) {
            percorrer(f, acum);
        }
    }
}

#[derive(Default)]
struct Acumulado {
    ctdivol_max: Option<f64>,
    dlp_total: Option<f64>,
    dlp_eventos: f64,
    dap_total: Option<f64>,
    dap_eventos: f64,
}

/// Lê a dose das tags simplificadas de um RDSR (puro, testado abaixo). O total do relatório tem
/// preferência; sem ele, soma os eventos.
#[must_use]
pub fn ler_rdsr(tags: &Value) -> DoseLida {
    let mut a = Acumulado::default();
    percorrer(tags, &mut a);
    let positivo = |x: f64| (x > 0.0).then_some(x);
    DoseLida {
        ctdivol_mgy: a.ctdivol_max,
        dlp_mgycm: a.dlp_total.or_else(|| positivo(a.dlp_eventos)),
        dap_gycm2: a.dap_total.or_else(|| positivo(a.dap_eventos)),
    }
}

/// Valor comparado ao NRD pela grandeza do protocolo.
#[must_use]
pub fn valor_da_grandeza(d: &DoseLida, grandeza: &str) -> Option<f64> {
    match grandeza {
        "ctdivol" => d.ctdivol_mgy,
        "dlp" => d.dlp_mgycm,
        "dap" => d.dap_gycm2,
        _ => None,
    }
}

fn sha256_hex(v: &Value) -> String {
    let bytes = serde_json::to_vec(v).unwrap_or_default();
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

async fn tentativa(
    estado: &Estado,
    unidade_id: Uuid,
    estudo_id: Uuid,
    resultado: &str,
) -> Result<()> {
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    sqlx::query!(
        "INSERT INTO exames.dose_tentativa (estudo_id, resultado, unidade_id) VALUES ($1, $2, $3)
         ON CONFLICT (estudo_id) DO UPDATE SET tentativas = exames.dose_tentativa.tentativas + 1,
             ultima_em = now(), resultado = EXCLUDED.resultado",
        estudo_id,
        resultado,
        unidade_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Procura o RDSR dos estudos de TC e raio X conciliados sem dose e grava a dose com a comparação ao
/// NRD vigente. Repetir não duplica (uma dose por estudo, G3). Devolve quantas doses gravou.
///
/// # Errors
/// Falha de banco. Falha do Orthanc num estudo vira tentativa registrada e segue para o próximo.
pub async fn ler_pendentes(estado: &Estado, orthanc: &Orthanc, unidade_id: Uuid) -> Result<usize> {
    let contexto = ContextoBanco::sistema(unidade_id);
    let mut tx = banco::transacao(&estado.banco, &contexto).await?;
    let pendentes = sqlx::query!(
        r#"SELECT estudo_id AS "estudo_id!", orthanc_id AS "orthanc_id!", atendimento_id AS "atendimento_id!",
                  paciente_id AS "paciente_id!", sala_id AS "sala_id!", modalidade AS "modalidade!", aparelho, exame_tuss AS "exame_tuss!"
             FROM exames.estudos_sem_dose(20)"#
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut gravadas = 0;
    for p in pendentes {
        let objetos = match orthanc.objetos_sr(&p.orthanc_id).await {
            Ok(o) => o,
            Err(e) => {
                tracing::warn!(erro = %e, estudo = %p.estudo_id, "leitura do RDSR");
                tentativa(estado, unidade_id, p.estudo_id, "erro").await?;
                continue;
            }
        };
        let achado = objetos
            .iter()
            .map(|o| (o, ler_rdsr(&o.tags)))
            .find(|(_, d)| !d.vazia());
        let Some((objeto, d)) = achado else {
            let r = if objetos.is_empty() {
                "sem_rdsr"
            } else {
                "rdsr_sem_dose"
            };
            tentativa(estado, unidade_id, p.estudo_id, r).await?;
            continue;
        };
        if !objeto
            .sop_uid
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.')
            || objeto.sop_uid.len() > 64
        {
            tentativa(estado, unidade_id, p.estudo_id, "erro").await?;
            continue;
        }
        let mut tx = banco::transacao(&estado.banco, &contexto).await?;
        let nrd = sqlx::query!(
            "SELECT id, grandeza, valor FROM exames.nrd WHERE exame_tuss = $1 AND vigente",
            p.exame_tuss
        )
        .fetch_optional(&mut *tx)
        .await?;
        let comparado = nrd
            .as_ref()
            .and_then(|n| valor_da_grandeza(&d, &n.grandeza));
        let acima = match (&nrd, comparado) {
            (Some(n), Some(v)) => v > n.valor,
            _ => false,
        };
        let id = vidacor_comum::novo_id();
        let codigo = crate::rotas::caixa::proximo(&mut tx, "DOS")
            .await
            .map_err(|e| anyhow::anyhow!("código da dose: {}", e.codigo()))?;
        let correlacao = format!("dose:{}", p.estudo_id);
        let inserida = sqlx::query!(
            "INSERT INTO exames.dose (id, codigo, estudo_id, atendimento_id, paciente_id, sala_id, aparelho, modalidade, exame_tuss,
                    ctdivol_mgy, dlp_mgycm, dap_gycm2, rdsr_sop_uid, rdsr_sha256, nrd_id, valor_comparado, acima_do_nrd,
                    origem, unidade_id, correlacao)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,'rdsr',$18,$19)
             ON CONFLICT (estudo_id) DO NOTHING",
            id, codigo.como_str(), p.estudo_id, p.atendimento_id, p.paciente_id, p.sala_id, p.aparelho, p.modalidade,
            p.exame_tuss, d.ctdivol_mgy, d.dlp_mgycm, d.dap_gycm2, objeto.sop_uid, sha256_hex(&objeto.tags),
            nrd.as_ref().map(|n| n.id), comparado, acima, unidade_id, correlacao
        )
        .execute(&mut *tx)
        .await?;
        if inserida.rows_affected() == 1 {
            lastro::registrar(
                &mut tx,
                NovoEvento {
                    dominio: "exames",
                    tipo: "dose_lida",
                    registro_id: id,
                    registro_codigo: Some(codigo.como_str()),
                    dados: json!({ "estudo_id": p.estudo_id, "rdsr": objeto.sop_uid, "acima_do_nrd": acima,
                                   "nrd_id": nrd.as_ref().map(|n| n.id) }),
                    criado_por: None,
                    origem: "rdsr",
                    unidade_id,
                    correlacao: &correlacao,
                },
            )
            .await?;
            gravadas += 1;
        }
        tx.commit().await?;
    }
    Ok(gravadas)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn item(codigo: &str, valor: &str, unidade: &str) -> Value {
        json!({
            "ConceptNameCodeSequence": [{ "CodeValue": codigo, "CodingSchemeDesignator": "DCM" }],
            "MeasuredValueSequence": [{ "NumericValue": valor,
                "MeasurementUnitsCodeSequence": [{ "CodeValue": unidade, "CodingSchemeDesignator": "UCUM" }] }]
        })
    }

    #[test]
    fn tc_com_dois_eventos_usa_o_total_e_o_maior_ctdivol() {
        let tags = json!({ "ContentSequence": [
            { "ContentSequence": [item("113813", "520.5", "mGy.cm")] },
            { "ContentSequence": [{ "ContentSequence": [item("113830", "14.2", "mGy"), item("113838", "300", "mGy.cm")] }] },
            { "ContentSequence": [{ "ContentSequence": [item("113830", "9.8", "mGy"), item("113838", "220.5", "mGy.cm")] }] }
        ]});
        let d = ler_rdsr(&tags);
        assert_eq!(d.ctdivol_mgy, Some(14.2));
        assert_eq!(d.dlp_mgycm, Some(520.5));
        assert_eq!(d.dap_gycm2, None);
    }

    #[test]
    fn sem_total_soma_os_eventos_e_dap_converte_unidade() {
        let tags = json!({ "ContentSequence": [
            item("113838", "100", "mGy.cm"), item("113838", "50", "mGy.cm"),
            item("113722", "0.00009", "Gy.m2")
        ]});
        let d = ler_rdsr(&tags);
        assert_eq!(d.dlp_mgycm, Some(150.0));
        assert!((d.dap_gycm2.unwrap_or_default() - 0.9).abs() < 1e-9);
    }

    #[test]
    fn valor_invalido_ou_outro_esquema_e_ignorado() {
        let mut outro = item("113830", "14", "mGy");
        outro["ConceptNameCodeSequence"][0]["CodingSchemeDesignator"] = json!("99X");
        let tags = json!({ "ContentSequence": [outro, item("113830", "-3", "mGy"), item("113838", "abc", "mGy.cm")] });
        assert!(ler_rdsr(&tags).vazia());
        assert_eq!(
            valor_da_grandeza(
                &DoseLida {
                    ctdivol_mgy: Some(1.0),
                    ..DoseLida::default()
                },
                "ctdivol"
            ),
            Some(1.0)
        );
        assert_eq!(valor_da_grandeza(&DoseLida::default(), "x"), None);
    }
}
