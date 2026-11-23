//! Imagens do exame em PDF para entregar ao paciente (docs/31 VIS-08 a VIS-10, FC-VIS-02).
//!
//! Recebe as imagens já renderizadas em JPEG (pelo visualizador, com a janela escolhida na tela) e os
//! dados já conferidos: um só paciente e um só estudo (VIS-S1, conferido por quem chama). Sem nome e
//! nascimento do paciente o PDF não é gerado (dois identificadores).

use crate::motor::{self, Momento, Pdf};
use crate::{Clinica, ErroDocumento};
use serde_json::json;

/// Versão do modelo; muda a cada alteração de `modelos/imagens.typ`.
pub const MODELO_IMAGENS: &str = "imagens-exame-1";
const FONTE: &str = include_str!("../modelos/imagens.typ");
/// Logotipo da VidaCor (recorte em `design/assets/marca`, ver docs/10).
const LOGO: &[u8] = include_bytes!("../../../design/assets/marca/logo.png");

/// Limite de imagens num PDF (docs/17 disponibilidade): 50 páginas de 12.
pub const MAX_IMAGENS_PDF: usize = 600;

/// Imagens por página, em retrato: colunas × linhas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    /// 1 por página (1 × 1).
    Uma,
    /// 4 por página (2 × 2).
    Quatro,
    /// 6 por página (2 colunas × 3 linhas).
    Seis,
    /// 12 por página (3 colunas × 4 linhas).
    Doze,
}

impl Grade {
    pub fn colunas_e_linhas(self) -> (u8, u8) {
        match self {
            Grade::Uma => (1, 1),
            Grade::Quatro => (2, 2),
            Grade::Seis => (2, 3),
            Grade::Doze => (3, 4),
        }
    }

    pub fn por_pagina(self) -> usize {
        let (c, l) = self.colunas_e_linhas();
        usize::from(c) * usize::from(l)
    }
}

#[derive(Clone)]
pub struct ImagemDoExame {
    /// JPEG já renderizado (o mesmo que foi visto na tela).
    pub jpeg: Vec<u8>,
    /// Ex.: "Série 2 (PARTES MOLES), imagem 35, janela 40/400".
    pub legenda: String,
}

impl std::fmt::Debug for ImagemDoExame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ImagemDoExame {{ {} bytes }}", self.jpeg.len())
    }
}

/// Dados impressos. Campo vazio sai como "não informado" (a página nunca inventa dado, R13).
#[derive(Clone)]
pub struct ImagensDoExame {
    pub clinica: Clinica,
    pub paciente_nome: String,
    /// `dd/mm/aaaa`.
    pub paciente_nascimento: String,
    pub exame: String,
    /// `dd/mm/aaaa`.
    pub data_exame: String,
    pub numero_atendimento: String,
    pub medico_solicitante: String,
    /// Ex.: "23/11/2026 às 14:05".
    pub gerado_em: String,
    /// Momento da geração (UTC), gravado como data de criação do PDF.
    pub momento: Momento,
    pub grade: Grade,
    pub imagens: Vec<ImagemDoExame>,
}

impl std::fmt::Debug for ImagensDoExame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ImagensDoExame {{ {} imagem(ns), grade {:?}, .. }}",
            self.imagens.len(),
            self.grade
        )
    }
}

fn ou_nao_informado(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        "não informado".to_owned()
    } else {
        t.to_owned()
    }
}

fn conferir(x: &ImagensDoExame) -> Result<(), ErroDocumento> {
    if x.paciente_nome.trim().is_empty() || x.paciente_nascimento.trim().is_empty() {
        return Err(ErroDocumento::Dados(
            "o PDF exige o nome e a data de nascimento do paciente".into(),
        ));
    }
    if x.imagens.is_empty() {
        return Err(ErroDocumento::Dados("nenhuma imagem foi escolhida".into()));
    }
    if x.imagens.len() > MAX_IMAGENS_PDF {
        return Err(ErroDocumento::Dados(format!(
            "no máximo {MAX_IMAGENS_PDF} imagens por PDF"
        )));
    }
    if let Some(n) = x
        .imagens
        .iter()
        .position(|i| !i.jpeg.starts_with(&[0xFF, 0xD8, 0xFF]))
    {
        return Err(ErroDocumento::Dados(format!(
            "a imagem {} não é um JPEG",
            n + 1
        )));
    }
    Ok(())
}

fn dados(x: &ImagensDoExame) -> serde_json::Value {
    let (colunas, linhas) = x.grade.colunas_e_linhas();
    json!({
        "clinica": { "nome": x.clinica.nome, "linhas": x.clinica.linhas() },
        "paciente": { "nome": x.paciente_nome.trim(), "nascimento": x.paciente_nascimento.trim() },
        "exame": {
            "descricao": ou_nao_informado(&x.exame),
            "data": ou_nao_informado(&x.data_exame),
            "atendimento": ou_nao_informado(&x.numero_atendimento),
            "solicitante": ou_nao_informado(&x.medico_solicitante),
        },
        "gerado_em": x.gerado_em,
        "grade": { "colunas": colunas, "linhas": linhas },
        "imagens": x.imagens.iter().map(|i| json!({ "legenda": i.legenda })).collect::<Vec<_>>(),
    })
}

fn anexos(x: &ImagensDoExame) -> Vec<(String, Vec<u8>)> {
    let mut a = vec![("/logo.png".to_owned(), LOGO.to_vec())];
    a.extend(
        x.imagens
            .iter()
            .enumerate()
            .map(|(i, img)| (format!("/imagem/{i}.jpg"), img.jpeg.clone())),
    );
    a
}

/// Gera o PDF (PDF/A-2b) com o SHA-256. O identificador do PDF não leva dado do paciente.
pub fn imagens_pdf(x: &ImagensDoExame) -> Result<Pdf, ErroDocumento> {
    conferir(x)?;
    let documento = motor::compilar_documento_com_anexos(FONTE, &dados(x), anexos(x))?;
    let m = x.momento;
    motor::exportar(
        &documento,
        MODELO_IMAGENS,
        &format!(
            "imagens-{:04}{:02}{:02}{:02}{:02}{:02}",
            m.ano, m.mes, m.dia, m.hora, m.minuto, m.segundo
        ),
        m,
    )
}

#[cfg(test)]
mod testes {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    const QUADRO: &[u8] = include_bytes!("../tests/dados/quadro.jpg");

    fn exemplo(n: usize, grade: Grade) -> ImagensDoExame {
        ImagensDoExame {
            clinica: Clinica {
                nome: "VidaCor Imagem".into(),
                endereco: Some("Rua Fictícia, 100, Centro".into()),
                telefone: Some("(00) 0000-0000".into()),
                site: None,
                cnpj: None,
                responsavel_tecnico: None,
            },
            paciente_nome: "ANA MARIA FICTICIO".into(),
            paciente_nascimento: "01/01/1980".into(),
            exame: "TC CRANIO".into(),
            data_exame: "01/01/2026".into(),
            numero_atendimento: "TESTE0001".into(),
            medico_solicitante: String::new(),
            gerado_em: "23/11/2026 às 14:05".into(),
            momento: Momento {
                ano: 2026,
                mes: 11,
                dia: 23,
                hora: 17,
                minuto: 5,
                segundo: 0,
            },
            grade,
            imagens: (0..n)
                .map(|i| ImagemDoExame {
                    jpeg: QUADRO.to_vec(),
                    legenda: format!("Série 2, imagem {}", i + 1),
                })
                .collect(),
        }
    }

    fn texto_por_pagina(x: &ImagensDoExame) -> Vec<String> {
        let doc = motor::compilar_documento_com_anexos(FONTE, &dados(x), anexos(x)).unwrap();
        motor::texto(&doc).lines().map(str::to_owned).collect()
    }

    // FC-VIS-02: 12 imagens em 6 por página = 2 páginas com os identificadores em todas.
    #[test]
    fn doze_imagens_em_seis_por_pagina_dao_duas_paginas_com_os_identificadores() {
        let x = exemplo(12, Grade::Seis);
        let paginas = texto_por_pagina(&x);
        assert_eq!(paginas.len(), 2);
        for p in &paginas {
            for esperado in [
                "ANA MARIA FICTICIO",
                "01/01/1980",
                "TC CRANIO",
                "01/01/2026",
            ] {
                assert!(p.contains(esperado), "{esperado} em {p}");
            }
            assert!(p.contains("laudo assinado"), "aviso VIS-10 em {p}");
        }
        assert!(paginas[1].contains("imagem 12"));
        assert!(paginas[0].contains("não informado"), "solicitante vazio");
        let pdf = imagens_pdf(&x).unwrap();
        assert!(pdf.bytes.starts_with(b"%PDF"));
        assert_eq!(pdf.sha256.len(), 64);
        assert_eq!(pdf.modelo, MODELO_IMAGENS);
        assert_eq!(
            imagens_pdf(&x).unwrap().sha256,
            pdf.sha256,
            "determinístico"
        );
    }

    #[test]
    fn paginas_por_grade() {
        for (grade, n, paginas) in [
            (Grade::Uma, 3, 3),
            (Grade::Quatro, 5, 2),
            (Grade::Doze, 12, 1),
            (Grade::Doze, 13, 2),
        ] {
            assert_eq!(
                texto_por_pagina(&exemplo(n, grade)).len(),
                paginas,
                "{grade:?}"
            );
        }
    }

    #[test]
    fn sem_os_dois_identificadores_nao_gera() {
        let mut x = exemplo(1, Grade::Uma);
        x.paciente_nascimento = "  ".into();
        assert!(matches!(imagens_pdf(&x), Err(ErroDocumento::Dados(_))));
        let mut x = exemplo(1, Grade::Uma);
        x.paciente_nome = String::new();
        assert!(matches!(imagens_pdf(&x), Err(ErroDocumento::Dados(_))));
    }

    #[test]
    fn limites_de_imagens_e_formato() {
        assert!(imagens_pdf(&exemplo(0, Grade::Uma)).is_err());
        assert!(imagens_pdf(&exemplo(MAX_IMAGENS_PDF + 1, Grade::Doze)).is_err());
        let mut x = exemplo(2, Grade::Uma);
        x.imagens[1].jpeg = b"%PDF-1.7 nao e imagem".to_vec();
        let Err(ErroDocumento::Dados(m)) = imagens_pdf(&x) else {
            unreachable!("esperava erro de dados")
        };
        assert!(m.contains("imagem 2"));
    }

    /// SEC-INJ-4: marcação Typst no nome e na legenda sai como texto.
    #[test]
    fn marcacao_no_dado_sai_como_texto() {
        let maligno = r#"Ana #image("/logo.png") #panic("x") *negrito* $x$"#;
        let mut x = exemplo(1, Grade::Uma);
        x.paciente_nome = maligno.into();
        x.imagens[0].legenda = maligno.into();
        let texto: String = texto_por_pagina(&x).concat().split_whitespace().collect();
        for pedaco in ["#image(\"/logo.png\")", "#panic(\"x\")", "*negrito*", "$x$"] {
            assert!(texto.contains(pedaco), "{pedaco} impresso literalmente");
        }
    }

    #[test]
    fn debug_nao_mostra_o_paciente() {
        let x = exemplo(2, Grade::Quatro);
        let d = format!("{x:?}");
        assert!(!d.contains("ANA") && !d.contains("1980"));
    }
}
