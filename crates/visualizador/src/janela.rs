//! Janela (centro e largura): escolhe quais valores do exame viram os 256 tons de cinza da tela
//! e do JPEG (docs/31 VIS-04). Fórmula linear do DICOM PS3.3 C.11.2.1.2.1.

/// Janela em unidades do exame depois da reescala (Hounsfield na tomografia).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Janela {
    pub centro: f64,
    /// Sempre ≥ 1 (o DICOM não admite largura menor).
    pub largura: f64,
}

impl Janela {
    /// Largura abaixo de 1 ou valor não finito é corrigido (largura 1, centro 0) em vez de falhar:
    /// a tela nunca fica sem imagem por causa de um ajuste do mouse.
    pub fn nova(centro: f64, largura: f64) -> Janela {
        Janela {
            centro: if centro.is_finite() { centro } else { 0.0 },
            largura: if largura.is_finite() {
                largura.max(1.0)
            } else {
                1.0
            },
        }
    }

    /// Janela que cobre do menor ao maior valor (usada quando o arquivo não traz janela).
    pub fn automatica(minimo: f64, maximo: f64) -> Janela {
        Janela::nova((minimo + maximo) / 2.0 + 0.5, (maximo - minimo) + 1.0)
    }

    /// Valor do exame para tom de cinza 0 a 255.
    pub fn aplicar(&self, x: f64) -> u8 {
        let c = self.centro - 0.5;
        let w = self.largura - 1.0;
        if x <= c - w / 2.0 {
            0
        } else if x > c + w / 2.0 || w <= 0.0 {
            255
        } else {
            (((x - c) / w + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8
        }
    }
}

/// Janelas prontas da tomografia (valores usuais em RIS/PACS; a confirmar com os médicos da clínica).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JanelaPronta {
    PartesMoles,
    Cerebro,
    Osso,
    Pulmao,
}

impl JanelaPronta {
    pub fn nome(&self) -> &'static str {
        match self {
            JanelaPronta::PartesMoles => "Partes moles",
            JanelaPronta::Cerebro => "Cérebro",
            JanelaPronta::Osso => "Osso",
            JanelaPronta::Pulmao => "Pulmão",
        }
    }

    pub fn janela(&self) -> Janela {
        match self {
            JanelaPronta::PartesMoles => Janela::nova(40.0, 400.0),
            JanelaPronta::Cerebro => Janela::nova(40.0, 80.0),
            JanelaPronta::Osso => Janela::nova(500.0, 2000.0),
            JanelaPronta::Pulmao => Janela::nova(-600.0, 1500.0),
        }
    }
}

/// Janelas prontas por modalidade. Fora da tomografia os valores não têm unidade fixa, então
/// só há a janela do arquivo, a automática e o ajuste com o mouse.
pub fn janelas_prontas(modalidade: &str) -> &'static [JanelaPronta] {
    match modalidade {
        "CT" => &[
            JanelaPronta::PartesMoles,
            JanelaPronta::Cerebro,
            JanelaPronta::Osso,
            JanelaPronta::Pulmao,
        ],
        _ => &[],
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn formula_linear_do_dicom() {
        let j = Janela::nova(35.0, 100.0);
        // limite inferior c - 0,5 - (w - 1)/2 = -15; superior = 84
        assert_eq!(j.aplicar(-1000.0), 0);
        assert_eq!(j.aplicar(-15.0), 0);
        assert_eq!(j.aplicar(84.1), 255);
        assert_eq!(j.aplicar(34.5), 128);
        assert_eq!(j.aplicar(20.0), 90);
    }

    #[test]
    fn largura_invalida_nao_quebra() {
        assert_eq!(Janela::nova(10.0, 0.0).largura, 1.0);
        assert_eq!(Janela::nova(10.0, -50.0).largura, 1.0);
        assert_eq!(
            Janela::nova(f64::NAN, f64::INFINITY),
            Janela::nova(0.0, 1.0)
        );
        let j = Janela::nova(10.0, 1.0);
        assert_eq!(j.aplicar(9.0), 0);
        assert_eq!(j.aplicar(10.0), 255);
    }

    #[test]
    fn automatica_cobre_minimo_e_maximo() {
        let j = Janela::automatica(0.0, 255.0);
        assert_eq!(j.aplicar(0.0), 0);
        assert_eq!(j.aplicar(255.0), 255);
    }

    #[test]
    fn prontas_so_na_tomografia() {
        assert_eq!(janelas_prontas("CT").len(), 4);
        assert!(janelas_prontas("US").is_empty());
        assert_eq!(JanelaPronta::Osso.janela(), Janela::nova(500.0, 2000.0));
    }
}
