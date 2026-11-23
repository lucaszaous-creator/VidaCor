//! Decodificação dos pixels e aplicação da janela.
//!
//! A tela decodifica um quadro uma vez ([`decodificar`]) e reaplica a janela a cada ajuste do
//! mouse ([`QuadroCinza::renderizar`]) sem ler o arquivo de novo. O JPEG usa a mesma
//! renderização, então o que o paciente recebe é o que foi visto (VIS-11, FC-VIS-05).

use dicom_dictionary_std::tags;
use dicom_encoding::TransferSyntaxIndex;
use dicom_pixeldata::{PhotometricInterpretation, PixelDecoder};
use dicom_transfer_syntax_registry::TransferSyntaxRegistry;

use crate::exame::{ler_dicom, Imagem};
use crate::fonte::Limites;
use crate::{Erro, Janela};

/// Pixels prontos para a tela ou o JPEG, linha a linha a partir do canto superior esquerdo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pixels {
    /// Um byte por pixel (0 = preto).
    Cinza(Vec<u8>),
    /// Três bytes por pixel (R, G, B).
    Rgb(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagemRenderizada {
    pub largura: u32,
    pub altura: u32,
    pub pixels: Pixels,
}

/// Quadro em tons de cinza com os valores do exame já reescalados (Hounsfield na TC).
#[derive(Debug, Clone)]
pub struct QuadroCinza {
    pub largura: u32,
    pub altura: u32,
    /// Valores depois da reescala (RescaleSlope e RescaleIntercept), linha a linha.
    pub valores: Vec<f32>,
    /// MONOCHROME1: o menor valor é branco (raio-X de alguns aparelhos); a inversão é automática.
    pub invertido: bool,
    /// Janelas gravadas pelo aparelho, na ordem do arquivo (a primeira é a padrão).
    pub janelas_do_arquivo: Vec<Janela>,
    pub minimo: f32,
    pub maximo: f32,
}

impl QuadroCinza {
    /// Janela ao abrir: a primeira do arquivo; sem ela, do menor ao maior valor.
    pub fn janela_padrao(&self) -> Janela {
        self.janelas_do_arquivo
            .first()
            .copied()
            .unwrap_or_else(|| Janela::automatica(f64::from(self.minimo), f64::from(self.maximo)))
    }

    pub fn renderizar(&self, janela: Janela) -> ImagemRenderizada {
        let pixels = self
            .valores
            .iter()
            .map(|v| {
                let t = janela.aplicar(f64::from(*v));
                if self.invertido {
                    255 - t
                } else {
                    t
                }
            })
            .collect();
        ImagemRenderizada {
            largura: self.largura,
            altura: self.altura,
            pixels: Pixels::Cinza(pixels),
        }
    }
}

/// Quadro decodificado. Imagem colorida (ultrassom com Doppler, fotos) não tem janela.
#[derive(Debug, Clone)]
pub enum QuadroDecodificado {
    Cinza(QuadroCinza),
    Cor(ImagemRenderizada),
}

impl QuadroDecodificado {
    /// Renderização com a janela padrão (a cor sai como está).
    pub fn renderizar_padrao(&self) -> ImagemRenderizada {
        match self {
            QuadroDecodificado::Cinza(q) => q.renderizar(q.janela_padrao()),
            QuadroDecodificado::Cor(i) => i.clone(),
        }
    }
}

/// WindowCenter e WindowWidth podem ter vários valores (ex.: osso e cérebro); todos viram opção.
/// Par incompleto ou largura menor que 1 é descartado.
fn janelas_do_arquivo(obj: &dicom_object::DefaultDicomObject) -> Vec<Janela> {
    let multi = |tag| {
        obj.get(tag)
            .and_then(|e| e.to_multi_float64().ok())
            .unwrap_or_default()
    };
    let centros = multi(tags::WINDOW_CENTER);
    let larguras = multi(tags::WINDOW_WIDTH);
    centros
        .iter()
        .zip(&larguras)
        .filter(|(c, w)| c.is_finite() && w.is_finite() && **w >= 1.0)
        .map(|(c, w)| Janela::nova(*c, *w))
        .collect()
}

/// Lê o arquivo da imagem e decodifica o quadro pedido (0 para imagem simples).
pub fn decodificar(
    imagem: &Imagem,
    quadro: u32,
    limites: &Limites,
) -> Result<QuadroDecodificado, Erro> {
    if quadro >= imagem.quadros {
        return Err(Erro::QuadroInexistente);
    }
    let bytes = imagem.origem.ler(limites)?;
    let obj = ler_dicom(&bytes)?;
    drop(bytes);

    let ts = obj.meta().transfer_syntax();
    let suportada = TransferSyntaxRegistry.get(ts).is_some_and(|t| {
        !t.is_unsupported() && !t.is_unsupported_pixel_encapsulation() && t.can_decode_all()
    });
    if !suportada {
        return Err(Erro::FormatoNaoSuportado);
    }

    let pixels = u64::from(imagem.linhas) * u64::from(imagem.colunas) * 3;
    if pixels > limites.max_pixels_quadro {
        return Err(Erro::LimiteExcedido {
            limite: "tamanho da imagem",
        });
    }

    let dec = obj
        .decode_pixel_data_frame(quadro)
        .map_err(|_| Erro::Danificado)?;
    let (largura, altura) = (dec.columns(), dec.rows());
    let fotometria = dec.photometric_interpretation().clone();

    if fotometria.is_monochrome() {
        let valores: Vec<f32> = dec.to_vec_frame(0).map_err(|_| Erro::FormatoNaoSuportado)?;
        if valores.len() as u64 != u64::from(largura) * u64::from(altura) {
            return Err(Erro::Danificado);
        }
        let (minimo, maximo) = valores
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), v| {
                (a.min(*v), b.max(*v))
            });
        let janelas_do_arquivo = janelas_do_arquivo(&obj);
        Ok(QuadroDecodificado::Cinza(QuadroCinza {
            largura,
            altura,
            valores,
            invertido: fotometria == PhotometricInterpretation::Monochrome1,
            janelas_do_arquivo,
            minimo: if minimo.is_finite() { minimo } else { 0.0 },
            maximo: if maximo.is_finite() { maximo } else { 0.0 },
        }))
    } else {
        Ok(QuadroDecodificado::Cor(ImagemRenderizada {
            largura,
            altura,
            pixels: Pixels::Rgb(crate::cor::para_rgb(&dec, &obj)?),
        }))
    }
}
