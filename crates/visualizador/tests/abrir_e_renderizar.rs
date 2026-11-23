//! Testes do núcleo do visualizador com os exames sintéticos de `tests/dados`
//! (gerados por `tools/imagens-dicom/gerar_amostras_teste.py`; nenhum dado real, VIS-S5).
//! Frases: FC-VIS-01, FC-VIS-04 e FC-VIS-05 (docs/31 § 6).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use vidacor_visualizador::{
    abrir, codificar_jpeg, decodificar, nome_arquivo_jpeg, Erro, Estudo, Exames, Janela, Limites,
    MotivoIgnorado, Pixels, QuadroCinza, QuadroDecodificado, QUALIDADE_PADRAO,
};

fn dados(nome: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/dados")
        .join(nome)
}

fn estudo<'a>(exames: &'a Exames, atendimento: &str) -> &'a Estudo {
    exames
        .estudos
        .iter()
        .find(|e| e.numero_atendimento == atendimento)
        .unwrap_or_else(|| panic!("estudo {atendimento} não encontrado"))
}

fn cinza(atendimento: &str, serie: usize, imagem: usize) -> QuadroCinza {
    let exames = abrir(&dados("exame.zip"), &Limites::default()).unwrap();
    let img = &estudo(&exames, atendimento).series[serie].imagens[imagem];
    match decodificar(img, 0, &Limites::default()).unwrap() {
        QuadroDecodificado::Cinza(q) => q,
        QuadroDecodificado::Cor(_) => panic!("esperava imagem em tons de cinza"),
    }
}

fn valor(q: &QuadroCinza, linha: usize, coluna: usize) -> f32 {
    q.valores[linha * q.largura as usize + coluna]
}

// FC-VIS-01: o .zip abre com as séries agrupadas e os dois identificadores do paciente.
#[test]
fn zip_abre_agrupado_por_paciente_estudo_e_serie() {
    let exames = abrir(&dados("exame.zip"), &Limites::default()).unwrap();
    assert_eq!(exames.estudos.len(), 6);

    let tc = estudo(&exames, "TESTE0001");
    assert_eq!(tc.paciente.nome(), "ANA MARIA FICTICIO");
    assert_eq!(tc.paciente.nascimento().as_deref(), Some("01/01/1980"));
    assert!(tc.paciente.tem_dois_identificadores());
    assert_eq!(tc.descricao, "TC CRANIO");
    assert_eq!(tc.medico_solicitante, "MEDICO FICTICIO");
    assert_eq!(tc.modalidades(), vec!["CT"]);
    let series: Vec<_> = tc.series.iter().map(|s| s.numero).collect();
    assert_eq!(series, vec![Some(2), Some(3)]);
    assert_eq!(tc.quantidade_imagens(), 4);

    // Cortes pela posição ao longo da normal (z = 0, 10, 20), não pelo InstanceNumber.
    let ordem: Vec<_> = tc.series[0].imagens.iter().map(|i| i.numero).collect();
    assert_eq!(ordem, vec![Some(3), Some(2), Some(1)]);

    let mr = estudo(&exames, "TESTE0004");
    assert_eq!(mr.series.len(), 2);
    let us = estudo(&exames, "TESTE0005");
    assert_eq!(us.series[0].imagens[0].quadros, 4);
}

#[test]
fn pacientes_diferentes_nunca_no_mesmo_estudo_e_acento_lido() {
    let exames = abrir(&dados("exame.zip"), &Limites::default()).unwrap();
    let outro = estudo(&exames, "TESTE0006");
    assert_eq!(outro.paciente.nome(), "JOÃO PEDRO FICTICIO");
    assert_eq!(outro.paciente.nascimento().as_deref(), Some("15/06/1975"));
    let ana = exames
        .estudos
        .iter()
        .filter(|e| e.paciente.id == "TESTE001")
        .count();
    assert_eq!(ana, 5);
}

// FC-VIS-04: arquivos que não são DICOM ou estão danificados ficam de fora com o motivo.
#[test]
fn arquivo_ruim_fica_de_fora_com_motivo_e_o_resto_abre() {
    let exames = abrir(&dados("exame.zip"), &Limites::default()).unwrap();
    let mut motivos: Vec<_> = exames
        .ignorados
        .iter()
        .map(|i| (i.origem.nome(), i.motivo))
        .collect();
    motivos.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        motivos,
        vec![
            ("nao_dicom.txt".to_owned(), MotivoIgnorado::NaoDicom),
            ("truncado.dcm".to_owned(), MotivoIgnorado::Danificado),
        ]
    );
    assert_eq!(
        MotivoIgnorado::Danificado.to_string(),
        "arquivo DICOM danificado ou incompleto"
    );
}

#[test]
fn pasta_com_zip_dentro_nao_repete_imagem() {
    let exames = abrir(&dados(""), &Limites::default()).unwrap();
    assert_eq!(exames.estudos.len(), 6);
    let total: usize = exames.estudos.iter().map(Estudo::quantidade_imagens).sum();
    assert_eq!(total, 12);
    let duplicados = exames
        .ignorados
        .iter()
        .filter(|i| i.motivo == MotivoIgnorado::Duplicado)
        .count();
    assert_eq!(duplicados, 12);
}

#[test]
fn arquivo_solto_abre() {
    let exames = abrir(&dados("mr_rle.dcm"), &Limites::default()).unwrap();
    assert_eq!(exames.estudos.len(), 1);
    assert!(exames.ignorados.is_empty());
}

#[test]
fn caminho_inexistente_e_erro_claro() {
    let r = abrir(&dados("nao-existe.zip"), &Limites::default());
    assert!(matches!(r, Err(Erro::NaoEncontrado)));
}

#[test]
fn limites_de_quantidade_e_tamanho() {
    let poucos = Limites {
        max_arquivos: 5,
        ..Limites::default()
    };
    assert!(matches!(
        abrir(&dados("exame.zip"), &poucos),
        Err(Erro::LimiteExcedido { .. })
    ));

    let pequenos = Limites {
        max_bytes_arquivo: 100,
        ..Limites::default()
    };
    let exames = abrir(&dados("exame.zip"), &pequenos).unwrap();
    assert!(exames.estudos.is_empty());
    assert!(exames
        .ignorados
        .iter()
        .filter(|i| i.origem.nome() != "nao_dicom.txt")
        .all(|i| i.motivo == MotivoIgnorado::GrandeDemais));

    let exames = abrir(&dados("exame.zip"), &Limites::default()).unwrap();
    let img = &estudo(&exames, "TESTE0001").series[0].imagens[0];
    let quadro_pequeno = Limites {
        max_pixels_quadro: 1000,
        ..Limites::default()
    };
    assert!(matches!(
        decodificar(img, 0, &quadro_pequeno),
        Err(Erro::LimiteExcedido { .. })
    ));
}

#[test]
fn tc_jpeg_lossless_decodifica_sem_perda_e_aplica_janela_do_aparelho() {
    // série 2, última na ordem = InstanceNumber 1, z = 20
    let q = cinza("TESTE0001", 0, 2);
    assert_eq!((q.largura, q.altura), (64, 64));
    assert_eq!(valor(&q, 0, 0), -1000.0);
    assert_eq!(valor(&q, 0, 63), 1000.0);
    assert_eq!(valor(&q, 32, 32), 20.0);
    assert_eq!(q.janela_padrao(), Janela::nova(35.0, 100.0));
    let img = q.renderizar(q.janela_padrao());
    let Pixels::Cinza(p) = &img.pixels else {
        panic!("esperava cinza")
    };
    assert_eq!(p[0], 0);
    assert_eq!(p[63], 255);
    assert_eq!(p[32 * 64 + 32], 90);
}

#[test]
fn tc_sem_compressao_aplica_reescala_e_le_as_duas_janelas() {
    let q = cinza("TESTE0001", 1, 0);
    assert_eq!(valor(&q, 0, 0), -1024.0);
    assert_eq!(valor(&q, 0, 63), 1976.0);
    assert_eq!(
        q.janelas_do_arquivo,
        vec![Janela::nova(500.0, 2000.0), Janela::nova(40.0, 80.0)]
    );
}

#[test]
fn raio_x_monochrome1_sai_invertido() {
    let q = cinza("TESTE0002", 0, 0);
    assert!(q.invertido);
    assert!(q.janelas_do_arquivo.is_empty());
    let Pixels::Cinza(p) = q.renderizar(q.janela_padrao()).pixels else {
        panic!("esperava cinza")
    };
    assert_eq!(p[0], 255, "valor mínimo é branco em MONOCHROME1");
    assert_eq!(p[63], 0);
}

#[test]
fn mamografia_jpeg2000_decodifica_sem_perda() {
    let q = cinza("TESTE0003", 0, 0);
    assert_eq!(valor(&q, 0, 0), 0.0);
    assert_eq!(valor(&q, 0, 63), 4095.0);
    assert_eq!(valor(&q, 32, 32), 2047.0);
}

#[test]
fn ressonancia_rle_e_jpeg_ls_dao_os_mesmos_valores() {
    let rle = cinza("TESTE0004", 0, 0);
    let jls = cinza("TESTE0004", 1, 0);
    assert_eq!(valor(&rle, 0, 63), 1200.0);
    assert_eq!(rle.valores, jls.valores);
}

#[test]
fn ultrassom_colorido_com_quadros() {
    let exames = abrir(&dados("exame.zip"), &Limites::default()).unwrap();
    let img = &estudo(&exames, "TESTE0005").series[0].imagens[0];
    let QuadroDecodificado::Cor(q) = decodificar(img, 2, &Limites::default()).unwrap() else {
        panic!("esperava imagem colorida")
    };
    let Pixels::Rgb(p) = &q.pixels else {
        panic!("esperava RGB")
    };
    let perto = |a: u8, b: u8| a.abs_diff(b) <= 10;
    assert!(perto(p[0], 200) && perto(p[1], 120) && perto(p[2], 0));
    let centro = (32 * 64 + 32) * 3;
    assert!(perto(p[centro + 2], 255));
    assert!(matches!(
        decodificar(img, 4, &Limites::default()),
        Err(Erro::QuadroInexistente)
    ));
}

fn cor(atendimento: &str, serie: usize) -> Vec<u8> {
    let exames = abrir(&dados("exame.zip"), &Limites::default()).unwrap();
    let img = &estudo(&exames, atendimento).series[serie].imagens[0];
    let QuadroDecodificado::Cor(q) = decodificar(img, 0, &Limites::default()).unwrap() else {
        panic!("esperava imagem colorida")
    };
    let Pixels::Rgb(p) = q.pixels else {
        panic!("esperava RGB")
    };
    assert_eq!(p.len(), 64 * 64 * 3);
    p
}

#[test]
fn ultrassom_com_paleta_de_cores() {
    let p = cor("TESTE0005", 1);
    // coluna 0: índice 0 -> (0, 255, 128); coluna 63: índice 252 -> (252, 3, 128)
    assert_eq!(&p[0..3], &[0, 255, 128]);
    assert_eq!(&p[63 * 3..64 * 3], &[252, 3, 128]);
}

#[test]
fn ultrassom_ybr_com_planos_separados_vira_rgb() {
    let p = cor("TESTE0005", 2);
    assert!(
        p[0] >= 250 && p[1] <= 5 && p[2] <= 5,
        "esperava vermelho: {:?}",
        &p[0..3]
    );
    assert_eq!(&p[0..3], &p[p.len() - 3..]);
}

// FC-VIS-05: o JPEG salvo é igual ao mostrado na tela (diferença só da compressão).
#[test]
fn jpeg_e_igual_ao_mostrado_na_tela() {
    let q = cinza("TESTE0001", 1, 0);
    let tela = q.renderizar(Janela::nova(500.0, 2000.0));
    let bytes = codificar_jpeg(&tela, QUALIDADE_PADRAO).unwrap();
    assert_eq!(&bytes[..2], &[0xFF, 0xD8]);

    let salvo = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)
        .unwrap()
        .to_luma8();
    assert_eq!(salvo.dimensions(), (64, 64));
    let salvo = salvo.into_raw();
    let Pixels::Cinza(mostrado) = &tela.pixels else {
        panic!("esperava cinza")
    };
    let diferenca_media = salvo
        .iter()
        .zip(mostrado)
        .map(|(a, b)| f64::from(a.abs_diff(*b)))
        .sum::<f64>()
        / mostrado.len() as f64;
    assert!(diferenca_media < 2.0, "diferença média {diferenca_media}");
}

#[test]
fn nada_do_paciente_no_debug_nem_no_nome_do_arquivo() {
    let exames = abrir(&dados("exame.zip"), &Limites::default()).unwrap();
    let texto = format!("{exames:?}");
    for proibido in ["ANA MARIA", "FICTICIO^", "19800101", "TESTE001\"", "JOÃO"] {
        assert!(!texto.contains(proibido), "Debug expôs {proibido}");
    }
    let tc = estudo(&exames, "TESTE0001");
    let img = &tc.series[0].imagens[0];
    let nome = nome_arquivo_jpeg(
        &tc.numero_atendimento,
        tc.series[0].numero,
        img.numero,
        None,
    );
    assert_eq!(nome, "exame-TESTE0001-serie2-img003.jpg");
}
