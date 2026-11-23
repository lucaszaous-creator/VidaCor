//! Imagens coloridas (ultrassom com Doppler, fotos clínicas, mapas coloridos) para RGB de 8 bits.
//! Cobre RGB, YBR_FULL e YBR_FULL_422 (já descomprimidos) e PALETTE COLOR (tabela de cores).

use dicom_dictionary_std::tags;
use dicom_object::DefaultDicomObject;
use dicom_pixeldata::{DecodedPixelData, PhotometricInterpretation, PlanarConfiguration};

use crate::Erro;

/// Converte o quadro 0 de `dec` para RGB intercalado (R, G, B por pixel).
pub(crate) fn para_rgb(
    dec: &DecodedPixelData<'_>,
    obj: &DefaultDicomObject,
) -> Result<Vec<u8>, Erro> {
    let pixels = dec.rows() as usize * dec.columns() as usize;
    match dec.photometric_interpretation() {
        PhotometricInterpretation::PaletteColor => paleta(dec, obj, pixels),
        PhotometricInterpretation::Rgb
        | PhotometricInterpretation::YbrFull
        | PhotometricInterpretation::YbrFull422 => {
            if dec.samples_per_pixel() != 3 || dec.bits_allocated() != 8 {
                return Err(Erro::FormatoNaoSuportado);
            }
            let dados = dec.frame_data(0).map_err(|_| Erro::Danificado)?;
            if dados.len() < pixels * 3 {
                return Err(Erro::Danificado);
            }
            let dados = &dados[..pixels * 3];
            let mut rgb = match dec.planar_configuration() {
                PlanarConfiguration::Standard => dados.to_vec(),
                PlanarConfiguration::PixelFirst => intercalar(dados),
            };
            if *dec.photometric_interpretation() != PhotometricInterpretation::Rgb {
                rgb.chunks_exact_mut(3).for_each(ybr_para_rgb);
            }
            Ok(rgb)
        }
        _ => Err(Erro::FormatoNaoSuportado),
    }
}

/// Planos separados (RRR…GGG…BBB…) para intercalado (RGBRGB…).
fn intercalar(dados: &[u8]) -> Vec<u8> {
    let n = dados.len() / 3;
    (0..n)
        .flat_map(|i| [dados[i], dados[n + i], dados[2 * n + i]])
        .collect()
}

/// YBR_FULL para RGB (DICOM PS3.3 C.7.6.3.1.2, mesma matriz do pydicom e do dicom-rs).
fn ybr_para_rgb(p: &mut [u8]) {
    let y = f32::from(p[0]);
    let cb = f32::from(p[1]) - 128.0;
    let cr = f32::from(p[2]) - 128.0;
    let r = y + 1.402 * cr;
    let g = y - (0.114 * 1.772 / 0.587) * cb - (0.299 * 1.402 / 0.587) * cr;
    let b = y + 1.772 * cb;
    for (destino, v) in p.iter_mut().zip([r, g, b]) {
        *destino = (v + 0.5).floor().clamp(0.0, 255.0) as u8;
    }
}

/// PALETTE COLOR: cada pixel é um índice nas tabelas vermelha, verde e azul do arquivo
/// (descritor: quantidade de entradas, primeiro valor mapeado, bits por entrada).
fn paleta(
    dec: &DecodedPixelData<'_>,
    obj: &DefaultDicomObject,
    pixels: usize,
) -> Result<Vec<u8>, Erro> {
    let tabelas = [
        (
            tags::RED_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR,
            tags::RED_PALETTE_COLOR_LOOKUP_TABLE_DATA,
        ),
        (
            tags::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR,
            tags::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA,
        ),
        (
            tags::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR,
            tags::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA,
        ),
    ]
    .map(|(d, t)| tabela(obj, d, t));
    let [Ok(r), Ok(g), Ok(b)] = tabelas else {
        return Err(Erro::FormatoNaoSuportado);
    };
    let indices: Vec<i64> = match dec.bits_allocated() {
        8 => dec
            .frame_data(0)
            .map_err(|_| Erro::Danificado)?
            .iter()
            .map(|v| i64::from(*v))
            .collect(),
        16 => dec
            .frame_data_ow(0)
            .map_err(|_| Erro::Danificado)?
            .iter()
            .map(|v| i64::from(*v))
            .collect(),
        _ => return Err(Erro::FormatoNaoSuportado),
    };
    if indices.len() < pixels {
        return Err(Erro::Danificado);
    }
    Ok(indices[..pixels]
        .iter()
        .flat_map(|i| [r.cor(*i), g.cor(*i), b.cor(*i)])
        .collect())
}

struct Tabela {
    primeiro: i64,
    valores: Vec<u16>,
    bits: u32,
}

impl Tabela {
    /// Índice abaixo do primeiro usa a primeira cor; acima do último, a última (PS3.3 C.7.6.3.1.5).
    fn cor(&self, indice: i64) -> u8 {
        let ultimo = self.valores.len() as i64 - 1;
        let pos = (indice - self.primeiro).clamp(0, ultimo.max(0)) as usize;
        let v = self.valores.get(pos).copied().unwrap_or(0);
        if self.bits > 8 {
            (v >> (self.bits - 8)) as u8
        } else {
            v as u8
        }
    }
}

fn tabela(
    obj: &DefaultDicomObject,
    descritor: dicom_core::Tag,
    dados: dicom_core::Tag,
) -> Result<Tabela, Erro> {
    let d: Vec<i64> = obj
        .get(descritor)
        .and_then(|e| e.to_multi_int::<i64>().ok())
        .ok_or(Erro::FormatoNaoSuportado)?;
    let [n, primeiro, bits] = d[..] else {
        return Err(Erro::FormatoNaoSuportado);
    };
    // O descritor é US ou SS; 0 entradas quer dizer 65536.
    let n = if n == 0 { 65_536 } else { n & 0xFFFF } as usize;
    let bits = u32::try_from(bits)
        .ok()
        .filter(|b| (1..=16).contains(b))
        .ok_or(Erro::FormatoNaoSuportado)?;
    let bruto = obj
        .get(dados)
        .and_then(|e| e.to_bytes().ok())
        .ok_or(Erro::FormatoNaoSuportado)?;
    let valores: Vec<u16> = if bits <= 8 && bruto.len() == n {
        bruto.iter().map(|v| u16::from(*v)).collect()
    } else {
        bruto
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect()
    };
    if valores.len() < n {
        return Err(Erro::Danificado);
    }
    Ok(Tabela {
        primeiro,
        valores: valores[..n].to_vec(),
        bits,
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ybr_cinza_continua_cinza_e_cores_primarias() {
        let mut p = [128, 128, 128];
        ybr_para_rgb(&mut p);
        assert_eq!(p, [128, 128, 128]);
        let mut vermelho = [76, 85, 255];
        ybr_para_rgb(&mut vermelho);
        assert!(vermelho[0] >= 250 && vermelho[1] <= 5 && vermelho[2] <= 5);
    }

    #[test]
    fn planos_separados_viram_intercalados() {
        assert_eq!(
            intercalar(&[1, 2, 10, 20, 100, 200]),
            vec![1, 10, 100, 2, 20, 200]
        );
    }

    #[test]
    fn indice_fora_da_tabela_usa_a_cor_da_ponta() {
        let t = Tabela {
            primeiro: 10,
            valores: vec![0x0000, 0x8000, 0xFFFF],
            bits: 16,
        };
        assert_eq!(t.cor(0), 0);
        assert_eq!(t.cor(11), 0x80);
        assert_eq!(t.cor(500), 0xFF);
    }
}
