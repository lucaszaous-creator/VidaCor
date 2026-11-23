//! Exportação do Visualizador com os exames sintéticos de `crates/visualizador/tests/dados`
//! (FC-VIS-02, FC-VIS-03, FC-VIS-05; VIS-11, VIS-12, VIS-S1). Sem Tauri: a lógica que os
//! comandos chamam, com arquivos e PDF de verdade.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use chrono::{Local, TimeZone};
use vidacor_visualizador::{abrir, Exames, Limites};
use vidacor_visualizador_app::{
    jpegs, legenda, montar_pdf, nome_livre, nome_pdf, resolver, ErroApp, Grade, ItemDto,
    Lancamento, Registro,
};

fn exames() -> Exames {
    let zip = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../crates/visualizador/tests/dados/exame.zip");
    abrir(&zip, &Limites::default()).unwrap()
}

fn indice(ex: &Exames, atendimento: &str) -> usize {
    ex.estudos
        .iter()
        .position(|e| e.numero_atendimento == atendimento)
        .unwrap()
}

fn item(
    estudo: usize,
    serie: usize,
    imagem: usize,
    quadro: u32,
    janela: Option<(f64, f64)>,
) -> serde_json::Value {
    serde_json::json!({
        "estudo": estudo, "serie": serie, "imagem": imagem, "quadro": quadro,
        "janela": janela.map(|(c, l)| serde_json::json!({ "centro": c, "largura": l })),
    })
}

/// A mesma forma JSON que a tela manda.
fn itens(v: Vec<serde_json::Value>) -> Vec<ItemDto> {
    serde_json::from_value(serde_json::Value::Array(v)).unwrap()
}

fn agora() -> chrono::DateTime<Local> {
    Local.with_ymd_and_hms(2026, 11, 23, 14, 5, 0).unwrap()
}

// FC-VIS-02 pelo programa: 4 imagens de um estudo viram PDF com os dados do próprio exame.
#[test]
fn pdf_de_um_estudo_sai_com_os_dados_do_exame() {
    let ex = exames();
    let tc = indice(&ex, "TESTE0001");
    let lista = itens(vec![
        item(tc, 0, 0, 0, Some((40.0, 400.0))),
        item(tc, 0, 1, 0, None),
        item(tc, 0, 2, 0, None),
        item(tc, 1, 0, 0, Some((500.0, 2000.0))),
    ]);
    let r = resolver(&ex, &lista).unwrap();
    let (pdf, estudo) = montar_pdf(&r, Grade::Quatro, agora()).unwrap();
    assert!(pdf.bytes.starts_with(b"%PDF"));
    assert_eq!(estudo.numero_atendimento, "TESTE0001");
    assert_eq!(nome_pdf(estudo), "imagens-exame-TESTE0001.pdf");
}

// FC-VIS-03 pelo programa: imagens de dois pacientes não viram PDF.
#[test]
fn pdf_com_dois_pacientes_e_bloqueado() {
    let ex = exames();
    let lista = itens(vec![
        item(indice(&ex, "TESTE0001"), 0, 0, 0, None),
        item(indice(&ex, "TESTE0006"), 0, 0, 0, None),
    ]);
    let r = resolver(&ex, &lista).unwrap();
    let erro = montar_pdf(&r, Grade::Uma, agora()).unwrap_err();
    assert!(matches!(
        erro,
        ErroApp::Visualizador(vidacor_visualizador::Erro::PacientesDiferentes)
    ));
    assert!(erro.to_string().contains("pacientes diferentes"));
}

#[test]
fn pdf_com_dois_exames_do_mesmo_paciente_e_bloqueado() {
    let ex = exames();
    let lista = itens(vec![
        item(indice(&ex, "TESTE0001"), 0, 0, 0, None),
        item(indice(&ex, "TESTE0002"), 0, 0, 0, None),
    ]);
    let r = resolver(&ex, &lista).unwrap();
    assert!(matches!(
        montar_pdf(&r, Grade::Uma, agora()),
        Err(ErroApp::Visualizador(
            vidacor_visualizador::Erro::EstudosDiferentes
        ))
    ));
}

#[test]
fn indice_fora_do_exame_e_recusado() {
    let ex = exames();
    for errado in [
        item(99, 0, 0, 0, None),
        item(0, 9, 0, 0, None),
        item(0, 0, 99, 0, None),
    ] {
        assert!(matches!(
            resolver(&ex, &itens(vec![errado])),
            Err(ErroApp::ItemInexistente)
        ));
    }
    let us = indice(&ex, "TESTE0005");
    assert!(matches!(
        resolver(&ex, &itens(vec![item(us, 0, 0, 4, None)])),
        Err(ErroApp::ItemInexistente)
    ));
    let muitos: Vec<_> = (0..601).map(|_| item(0, 0, 0, 0, None)).collect();
    assert!(matches!(
        resolver(&ex, &itens(muitos)),
        Err(ErroApp::MuitasImagens(600))
    ));
}

#[test]
fn legenda_diz_serie_imagem_quadro_e_janela() {
    let ex = exames();
    let tc = indice(&ex, "TESTE0001");
    let us = indice(&ex, "TESTE0005");
    let r = resolver(
        &ex,
        &itens(vec![
            item(tc, 0, 0, 0, Some((40.0, 400.0))),
            item(us, 0, 0, 2, None),
        ]),
    )
    .unwrap();
    assert_eq!(
        legenda(&r[0], r[0].janela),
        "Série 2 (PARTES MOLES), imagem 3, janela 40/400"
    );
    assert_eq!(
        legenda(&r[1], None),
        "Série 1 (CINE), imagem 1, quadro 3 de 4"
    );
}

// FC-VIS-05 e VIS-11: JPEG com a janela marcada e nome sem dado pessoal.
#[test]
fn jpeg_com_nome_sem_dado_pessoal() {
    let ex = exames();
    let tc = indice(&ex, "TESTE0001");
    let us = indice(&ex, "TESTE0005");
    let r = resolver(
        &ex,
        &itens(vec![
            item(tc, 1, 0, 0, Some((500.0, 2000.0))),
            item(us, 0, 0, 1, None),
        ]),
    )
    .unwrap();
    let arquivos = jpegs(&r).unwrap();
    let nomes: Vec<_> = arquivos.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        nomes,
        vec![
            "exame-TESTE0001-serie3-img001.jpg",
            "exame-TESTE0005-serie1-img001-quadro002.jpg"
        ]
    );
    assert!(arquivos.iter().all(|(_, b)| b.starts_with(&[0xFF, 0xD8])));
    assert!(jpegs(&[]).is_err());
}

#[test]
fn arquivo_existente_nunca_e_sobrescrito() {
    let pasta = std::env::temp_dir().join(format!("vidacor-nome-livre-{}", std::process::id()));
    std::fs::create_dir_all(&pasta).unwrap();
    std::fs::write(pasta.join("a.jpg"), b"1").unwrap();
    std::fs::write(pasta.join("a-2.jpg"), b"2").unwrap();
    assert_eq!(nome_livre(&pasta, "a.jpg"), pasta.join("a-3.jpg"));
    assert_eq!(nome_livre(&pasta, "b.jpg"), pasta.join("b.jpg"));
    std::fs::remove_dir_all(&pasta).unwrap();
}

// VIS-12: registro encadeado, sem dado do paciente, e alteração descoberta.
#[test]
fn registro_de_exportacoes_encadeado_e_sem_dado_do_paciente() {
    let pasta = std::env::temp_dir().join(format!("vidacor-registro-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&pasta);
    let reg = Registro::abrir(&pasta).unwrap();
    for n in 1..=3 {
        reg.gravar(
            &Lancamento {
                ato: "pdf",
                estudo_uid: "1.2.826.0.1.3680043.10.1999.1".into(),
                imagens: n,
                sha256: vec!["ab".repeat(32)],
                modelo: Some("imagens-exame-1"),
            },
            "2026-11-23T17:05:00Z",
            "tecnico1",
            "SALA-TC",
        )
        .unwrap();
    }
    assert_eq!(reg.conferir().unwrap(), Ok(3));
    let arquivo = pasta.join("registro-exportacoes.jsonl");
    let texto = std::fs::read_to_string(&arquivo).unwrap();
    assert!(!texto.contains("ANA") && !texto.contains("FICTICIO"));

    // Alterar a quantidade de imagens da linha 2 quebra a corrente nessa linha.
    let alterado = texto.replacen("\"imagens\":2", "\"imagens\":9", 1);
    assert_ne!(alterado, texto);
    std::fs::write(&arquivo, alterado).unwrap();
    assert_eq!(reg.conferir().unwrap(), Err(2));

    // Apagar a linha 1 também.
    let sem_primeira: String = texto.lines().skip(1).map(|l| format!("{l}\n")).collect();
    std::fs::write(&arquivo, sem_primeira).unwrap();
    assert_eq!(reg.conferir().unwrap(), Err(1));
    std::fs::remove_dir_all(&pasta).unwrap();
}
