#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::NaiveDate;
use std::collections::BTreeMap;
use vidacor_dominio_faturamento::{
    Centavos, ContextoAuditoria, GuiaFaturavel, ItemAutorizado, ItemExecutado, Profissional,
};

pub fn dia(d: u32, m: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, d).unwrap()
}

/// Guia fictícia que passa em todas as regras (dados de design/dados-ficticios.md).
pub fn guia_ok() -> GuiaFaturavel {
    GuiaFaturavel {
        numero_guia_prestador: "FAT-2026-000001".into(),
        registro_ans: Some("123456".into()),
        codigo_prestador: Some("0004567".into()),
        cnes: Some("1234567".into()),
        numero_guia_operadora: "UN-55120".into(),
        senha: "UN-88231-X".into(),
        data_autorizacao: dia(3, 11),
        validade_senha: dia(30, 11),
        carteirinha: "0 123 4567".into(),
        solicitante: Profissional {
            nome: "Dr. Júlio Prado".into(),
            conselho: Some("06".into()),
            numero: Some("52123456".into()),
            uf: Some("33".into()),
            cbos: Some("225125".into()),
        },
        data_execucao: dia(5, 11),
        executados: vec![ItemExecutado {
            tuss: "40901114".into(),
            descricao: "Ecocardiograma transtorácico".into(),
            quantidade: 1,
            valor_unitario: Some(Centavos(24_860)),
        }],
        autorizados: vec![ItemAutorizado {
            tuss: "40901114".into(),
            quantidade: 1,
        }],
        laudo_assinado: true,
        cid: None,
        assinatura_beneficiario: false,
    }
}

pub fn ctx() -> ContextoAuditoria {
    ContextoAuditoria {
        hoje: dia(6, 11),
        prazo_envio_dias: Some(30),
        precos_vigentes: BTreeMap::from([("40901114".to_owned(), Centavos(24_860))]),
    }
}
