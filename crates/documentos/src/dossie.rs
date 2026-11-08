//! Dossiê do lastro em PDF (docs/04 Explorador de lastro, docs/21): a cadeia do atendimento com os
//! registros ligados, os eventos imutáveis, o hash completo de cada evento e a conferência.

use crate::motor::{self, Momento, Pdf};
use crate::{hash_em_grupos, Clinica, ErroDocumento};
use serde_json::json;
use sha2::{Digest, Sha256};

/// Versão do modelo; muda a cada alteração do arquivo `modelos/dossie.typ`.
pub const MODELO_DOSSIE: &str = "dossie-lastro-1";
const FONTE: &str = include_str!("../modelos/dossie.typ");

#[derive(Debug, Clone)]
pub struct NoDoDossie {
    pub tipo: String,
    pub codigo: String,
    pub situacao: Option<String>,
    pub criado_em: String,
}

#[derive(Debug, Clone)]
pub struct EventoDoDossie {
    pub sequencia: i64,
    pub dominio: String,
    pub tipo: String,
    pub registro_codigo: Option<String>,
    pub criado_em: String,
    pub origem: String,
    /// SHA-256 completo do evento (hex).
    pub hash: String,
    pub integro: bool,
}

#[derive(Debug, Clone)]
pub struct DossieLastro {
    pub clinica: Clinica,
    /// Código desta emissão (ex.: DOS-2026-000001).
    pub codigo_emissao: String,
    pub emitido_em: String,
    pub emitido_por: String,
    pub codigo_buscado: String,
    pub atendimento_codigo: String,
    pub paciente_nome: String,
    pub paciente_nascimento: String,
    pub nos: Vec<NoDoDossie>,
    pub eventos: Vec<EventoDoDossie>,
    pub momento: Momento,
}

/// Resumo da cadeia: SHA-256 dos hashes dos eventos, em ordem de sequência, separados por quebra de
/// linha. Quem tem o banco refaz a conta e confere que o dossiê mostra a mesma cadeia.
#[must_use]
pub fn resumo_da_cadeia(eventos: &[EventoDoDossie]) -> String {
    let mut h = Sha256::new();
    for e in eventos {
        h.update(e.hash.as_bytes());
        h.update(b"\n");
    }
    motor::hex(&h.finalize())
}

fn dados(d: &DossieLastro) -> serde_json::Value {
    let falhas = d.eventos.iter().filter(|e| !e.integro).count();
    json!({
        "clinica": { "nome": d.clinica.nome, "linhas": d.clinica.linhas() },
        "emissao": {
            "codigo": d.codigo_emissao, "em": d.emitido_em, "por": d.emitido_por,
            "buscado": d.codigo_buscado, "modelo": MODELO_DOSSIE,
        },
        "atendimento": d.atendimento_codigo,
        "paciente": { "nome": d.paciente_nome, "nascimento": d.paciente_nascimento },
        "integra": falhas == 0,
        "falhas": falhas.to_string(),
        "total": d.eventos.len().to_string(),
        "resumo": hash_em_grupos(&resumo_da_cadeia(&d.eventos)),
        "nos": d.nos.iter().map(|n| json!({
            "tipo": n.tipo, "codigo": n.codigo,
            "situacao": n.situacao.as_deref().unwrap_or("").replace('_', " "), "criado_em": n.criado_em,
        })).collect::<Vec<_>>(),
        "eventos": d.eventos.iter().map(|e| json!({
            "sequencia": e.sequencia.to_string(),
            "quando": e.criado_em,
            "evento": format!("{} / {}", e.dominio, e.tipo),
            "detalhe": match &e.registro_codigo {
                Some(r) => format!("{r}, origem {}", e.origem),
                None => format!("origem {}", e.origem),
            },
            "hash": hash_em_grupos(&e.hash),
            "confere": if e.integro { "Sim" } else { "NÃO" },
        })).collect::<Vec<_>>(),
    })
}

/// Gera o PDF do dossiê.
///
/// # Errors
/// Defeito do modelo ou dado impossível de serializar.
pub fn dossie_pdf(d: &DossieLastro) -> Result<Pdf, ErroDocumento> {
    let documento = motor::compilar_documento(FONTE, &dados(d))?;
    motor::exportar(&documento, MODELO_DOSSIE, &d.codigo_emissao, d.momento)
}

#[cfg(test)]
mod testes {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn exemplo(eventos: usize) -> DossieLastro {
        DossieLastro {
            clinica: Clinica {
                nome: "VidaCor Imagem".into(),
                endereco: None,
                telefone: None,
                site: None,
                cnpj: None,
                responsavel_tecnico: None,
            },
            codigo_emissao: "DOS-2026-000001".into(),
            emitido_em: "08/11/2026 às 14:00".into(),
            emitido_por: "Patrícia Lemos (gerente)".into(),
            codigo_buscado: "ATD-2026-000001".into(),
            atendimento_codigo: "ATD-2026-000001".into(),
            paciente_nome: "Antônio Carlos Ribeiro".into(),
            paciente_nascimento: "15/06/1954".into(),
            nos: vec![NoDoDossie {
                tipo: "atendimento".into(),
                codigo: "ATD-2026-000001".into(),
                situacao: Some("em_atendimento".into()),
                criado_em: "08/11/2026 08:00".into(),
            }],
            eventos: (0..eventos)
                .map(|i| EventoDoDossie {
                    sequencia: i64::try_from(i).unwrap() + 1,
                    dominio: "recepcao".into(),
                    tipo: "checkin".into(),
                    registro_codigo: Some("ATD-2026-000001".into()),
                    criado_em: "08/11/2026 08:00:00".into(),
                    origem: "recepcao".into(),
                    hash: format!("{i:064x}"),
                    integro: i != 3,
                })
                .collect(),
            momento: Momento {
                ano: 2026,
                mes: 11,
                dia: 8,
                hora: 17,
                minuto: 0,
                segundo: 0,
            },
        }
    }

    #[test]
    fn dossie_com_muitos_eventos_quebra_paginas_e_aponta_o_que_nao_confere() {
        let d = exemplo(120);
        let pdf = dossie_pdf(&d).unwrap();
        assert!(pdf.bytes.starts_with(b"%PDF"));
        let doc = motor::compilar_documento(FONTE, &dados(&d)).unwrap();
        assert!(doc.pages().len() > 1);
        let texto = motor::texto(&doc);
        assert!(texto.contains("NÃO"), "evento que não confere aparece");
        assert!(texto.contains("Ribeiro") && texto.contains("15/06/1954"));
    }

    #[test]
    fn resumo_muda_se_um_hash_muda() {
        let mut d = exemplo(5);
        let antes = resumo_da_cadeia(&d.eventos);
        d.eventos[2].hash = "f".repeat(64);
        assert_ne!(antes, resumo_da_cadeia(&d.eventos));
    }
}
