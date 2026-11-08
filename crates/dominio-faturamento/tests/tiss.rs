//! Lote TISS 4.03.00: o arquivo gerado passa no XSD oficial da ANS (validado com `xmllint`, que
//! precisa estar instalado: pacote `libxml2-utils`), o texto é escapado e os hashes conferem.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use chrono::NaiveDate;
use comum::guia_ok;
use std::path::PathBuf;
use std::process::Command;
use vidacor_dominio_faturamento::{montar_lote, Centavos, ErroTiss, GuiaFaturavel, LoteTiss};

fn gerado_em() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 11, 8)
        .unwrap()
        .and_hms_opt(10, 52, 0)
        .unwrap()
}

fn lote(guias: &[GuiaFaturavel]) -> LoteTiss<'_> {
    LoteTiss {
        numero: 149,
        registro_ans: "123456",
        codigo_prestador: "0004567",
        nome_prestador: "VidaCor Imagem Unidade Centro",
        gerado_em: gerado_em(),
        guias,
    }
}

/// Valida no XSD oficial (sem rede: `--nonet`). Devolve a saída do xmllint se reprovar.
fn validar_no_xsd(xml: &[u8], nome: &str) -> Result<(), String> {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let xsd = raiz.join("xsd/tiss-4.03.00/tissV4_03_00.xsd");
    let dir = std::env::temp_dir().join(format!("vidacor-tiss-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let arquivo = dir.join(format!("{nome}.xml"));
    std::fs::write(&arquivo, xml).unwrap();
    let saida = Command::new("xmllint")
        .args(["--noout", "--nonet", "--schema"])
        .arg(&xsd)
        .arg(&arquivo)
        .output()
        .expect("xmllint não encontrado: instale libxml2-utils para validar o TISS no XSD da ANS");
    let _ = std::fs::remove_file(&arquivo);
    if saida.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&saida.stderr).into_owned())
    }
}

fn texto(xml: &[u8]) -> String {
    xml.iter().map(|b| char::from(*b)).collect()
}

#[test]
fn lote_com_duas_guias_passa_no_xsd_oficial() {
    let mut g2 = guia_ok();
    g2.numero_guia_prestador = "FAT-2026-000002".into();
    g2.cid = Some("I10".into());
    g2.executados[0].quantidade = 2;
    g2.autorizados[0].quantidade = 2;
    let guias = [guia_ok(), g2];
    let a = montar_lote(&lote(&guias)).unwrap();
    validar_no_xsd(&a.xml, "lote-duas-guias").unwrap();
    assert_eq!(a.valor_total, Centavos(3 * 24_860));
    let t = texto(&a.xml);
    assert!(t.starts_with("<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>"));
    assert!(t.contains("<ans:Padrao>4.03.00</ans:Padrao>"));
    assert!(t.contains("<ans:numeroLote>149</ans:numeroLote>"));
    assert!(t.contains("<ans:numeroCarteira>01234567</ans:numeroCarteira>"));
    assert!(t.contains("<ans:valorTotal>497.20</ans:valorTotal>"));
    assert!(t.contains("<ans:observacao>CID-10 I10</ans:observacao>"));
    assert!(t.contains(&format!("<ans:hash>{}</ans:hash>", a.hash_tiss)));
    assert_eq!(a.hash_tiss.len(), 32);
    assert_eq!(a.sha256.len(), 64);
    // Mesmo lote, mesmo arquivo e mesmos hashes (o lastro confere o arquivo pelo SHA-256).
    assert_eq!(montar_lote(&lote(&guias)).unwrap(), a);
}

#[test]
fn texto_hostil_e_escapado_e_o_arquivo_continua_valido() {
    let mut g = guia_ok();
    g.solicitante.nome =
        "Dr. Júlio <!ENTITY x SYSTEM \"file:///etc/passwd\"> & Cia \u{1F600}\u{0007}".into();
    g.executados[0].descricao = "USG ]]> <ans:valorTotal>0</ans:valorTotal>".into();
    let guias = [g];
    let a = montar_lote(&lote(&guias)).unwrap();
    validar_no_xsd(&a.xml, "texto-hostil").unwrap();
    let t = texto(&a.xml);
    assert!(
        !t.contains("<!ENTITY"),
        "entidade não pode chegar ao arquivo"
    );
    assert!(!t.contains("<!DOCTYPE"));
    assert!(t.contains("&lt;!ENTITY x SYSTEM &quot;file:///etc/passwd&quot;&gt; &amp; Cia ?"));
    assert!(t.contains("USG ]]&gt; &lt;ans:valorTotal&gt;0&lt;/ans:valorTotal&gt;"));
    // Acento sai em ISO-8859-1 (um byte), como o padrão.
    assert!(
        a.xml.windows(2).any(|w| w == [b'J', 0xFA]),
        "Júlio em ISO-8859-1"
    );
}

#[test]
fn guia_que_o_xsd_recusaria_nao_entra_no_arquivo() {
    let mut sem_preco = guia_ok();
    sem_preco.executados[0].valor_unitario = None;
    let mut sem_crm = guia_ok();
    sem_crm.solicitante.numero = None;
    let mut outra = guia_ok();
    outra.registro_ans = Some("654321".into());
    for (g, esperado) in [
        (
            sem_preco,
            ErroTiss::GuiaIncompleta("FAT-2026-000001".into(), "preço da tabela"),
        ),
        (
            sem_crm,
            ErroTiss::GuiaIncompleta("FAT-2026-000001".into(), "dados do solicitante"),
        ),
        (
            outra,
            ErroTiss::OperadoraDiferente("FAT-2026-000001".into()),
        ),
    ] {
        assert_eq!(montar_lote(&lote(&[g])), Err(esperado));
    }
}

#[test]
fn limites_do_lote() {
    assert_eq!(montar_lote(&lote(&[])), Err(ErroTiss::LoteVazio));
    let cem: Vec<GuiaFaturavel> = (1..=100)
        .map(|i| {
            let mut g = guia_ok();
            g.numero_guia_prestador = format!("FAT-2026-{i:06}");
            g
        })
        .collect();
    let a = montar_lote(&lote(&cem)).unwrap();
    validar_no_xsd(&a.xml, "lote-cem").unwrap();
    let mut cento_e_uma = cem;
    cento_e_uma.push(guia_ok());
    assert_eq!(montar_lote(&lote(&cento_e_uma)), Err(ErroTiss::LoteGrande));
}
