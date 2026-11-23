//! Exportação JPEG (docs/31 VIS-11).

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder};

use crate::{Erro, ImagemRenderizada, Pixels};

/// Qualidade padrão: alta, porque o JPEG perde tons (o aviso de "imagem para referência" do PDF
/// e do VIS-S7 continua valendo).
pub const QUALIDADE_PADRAO: u8 = 95;

/// Codifica a imagem renderizada em JPEG. Qualidade de 1 a 100.
pub fn codificar_jpeg(imagem: &ImagemRenderizada, qualidade: u8) -> Result<Vec<u8>, Erro> {
    // O formato JPEG limita largura e altura a 65535.
    if imagem.largura > u32::from(u16::MAX) || imagem.altura > u32::from(u16::MAX) {
        return Err(Erro::ImagemGrandeDemais);
    }
    let (dados, cor, canais) = match &imagem.pixels {
        Pixels::Cinza(p) => (p, ExtendedColorType::L8, 1u64),
        Pixels::Rgb(p) => (p, ExtendedColorType::Rgb8, 3u64),
    };
    if dados.len() as u64 != u64::from(imagem.largura) * u64::from(imagem.altura) * canais {
        return Err(Erro::Jpeg);
    }
    let mut saida = Vec::new();
    JpegEncoder::new_with_quality(&mut saida, qualidade.clamp(1, 100))
        .write_image(dados, imagem.largura, imagem.altura, cor)
        .map_err(|_| Erro::Jpeg)?;
    Ok(saida)
}

/// Nome do arquivo JPEG sem dado pessoal: número do atendimento (só letras, números e hífen),
/// série, imagem e quadro. Ex.: `exame-TESTE0001-serie2-img003.jpg`.
pub fn nome_arquivo_jpeg(
    numero_atendimento: &str,
    numero_serie: Option<i32>,
    numero_imagem: Option<i32>,
    quadro: Option<u32>,
) -> String {
    let limpo: String = numero_atendimento
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(32)
        .collect();
    let mut nome = String::from("exame");
    if !limpo.is_empty() {
        nome.push('-');
        nome.push_str(&limpo);
    }
    if let Some(s) = numero_serie {
        nome.push_str(&format!("-serie{s}"));
    }
    if let Some(i) = numero_imagem {
        nome.push_str(&format!("-img{i:03}"));
    }
    if let Some(q) = quadro {
        nome.push_str(&format!("-quadro{:03}", q + 1));
    }
    nome.push_str(".jpg");
    nome
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn nome_sem_dado_pessoal_e_sem_caractere_de_caminho() {
        assert_eq!(
            nome_arquivo_jpeg("TESTE0001", Some(2), Some(3), None),
            "exame-TESTE0001-serie2-img003.jpg"
        );
        assert_eq!(
            nome_arquivo_jpeg("../2026/0001 a", None, Some(1), Some(0)),
            "exame-20260001a-img001-quadro001.jpg"
        );
        assert_eq!(nome_arquivo_jpeg("", None, None, None), "exame.jpg");
    }

    #[test]
    fn tamanho_dos_pixels_conferido() {
        let img = ImagemRenderizada {
            largura: 2,
            altura: 2,
            pixels: Pixels::Cinza(vec![0; 3]),
        };
        assert!(matches!(codificar_jpeg(&img, 95), Err(Erro::Jpeg)));
    }
}
