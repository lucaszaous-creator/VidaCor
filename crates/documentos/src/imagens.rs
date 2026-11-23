//! Imagens do exame em PDF para entregar ao paciente (docs/31 VIS-08 a VIS-10, FC-VIS-02).
//!
//! Arranjo das folhas que a clínica entrega hoje (Medilab, DEC-2026-11-23-16): todas as imagens, uma
//! série por página, grade escolhida pela quantidade (1, 2×2, 3×3 ou 4×5). Recebe as imagens já
//! renderizadas em JPEG (pelo visualizador, com a janela da série) e os dados já conferidos: um só
//! paciente e um só estudo (VIS-S1, conferido por quem chama). Sem nome e nascimento do paciente o
//! PDF não é gerado (dois identificadores).

use crate::motor::{self, Momento, Pdf};
use crate::{Clinica, ErroDocumento};
use serde_json::json;

/// Versão do modelo; muda a cada alteração de `modelos/imagens.typ`.
pub const MODELO_IMAGENS: &str = "imagens-exame-2";
const FONTE: &str = include_str!("../modelos/imagens.typ");
/// Logotipo da VidaCor (recorte em `design/assets/marca`, ver docs/10).
const LOGO: &[u8] = include_bytes!("../../../design/assets/marca/logo.png");

/// Limite de imagens num PDF (docs/17 disponibilidade): 30 páginas de 20.
pub const MAX_IMAGENS_PDF: usize = 600;

/// Grade da página (colunas, linhas) pela quantidade de imagens da série, como nas folhas da
/// clínica: 1 imagem ocupa a página; até 4, 2×2; até 9, 3×3; acima, 4×5 (20 por página).
pub fn grade_da_serie(imagens: usize) -> (u8, u8) {
    match imagens {
        0 | 1 => (1, 1),
        2..=4 => (2, 2),
        5..=9 => (3, 3),
        _ => (4, 5),
    }
}

/// Páginas que uma série com `imagens` imagens ocupa.
pub fn paginas_da_serie(imagens: usize) -> usize {
    if imagens == 0 {
        return 0;
    }
    let (c, l) = grade_da_serie(imagens);
    imagens.div_ceil(usize::from(c) * usize::from(l))
}

#[derive(Clone)]
pub struct ImagemDoExame {
    /// JPEG já renderizado (o mesmo que foi visto na tela).
    pub jpeg: Vec<u8>,
    /// Ex.: "Imagem 35, janela 40/400".
    pub legenda: String,
}

impl std::fmt::Debug for ImagemDoExame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ImagemDoExame {{ {} bytes }}", self.jpeg.len())
    }
}

/// Uma série do exame: começa sempre em página nova.
#[derive(Clone)]
pub struct SerieDoPdf {
    /// Ex.: "Série 2, PARTES MOLES Head 2.0, janela 35/100".
    pub titulo: String,
    pub imagens: Vec<ImagemDoExame>,
}

impl std::fmt::Debug for SerieDoPdf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SerieDoPdf {{ {} imagem(ns) }}", self.imagens.len())
    }
}

/// Dados impressos. Campo vazio sai como "não informado" (a página nunca inventa dado, R13).
#[derive(Clone)]
pub struct ImagensDoExame {
    pub clinica: Clinica,
    /// Razão social, impressa no cabeçalho (documento entregue ao paciente identifica a empresa).
    pub razao_social: Option<String>,
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
    pub series: Vec<SerieDoPdf>,
}

impl ImagensDoExame {
    pub fn quantidade_imagens(&self) -> usize {
        self.series.iter().map(|s| s.imagens.len()).sum()
    }
}

impl std::fmt::Debug for ImagensDoExame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ImagensDoExame {{ {} série(s), {} imagem(ns), .. }}",
            self.series.len(),
            self.quantidade_imagens()
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
    let total = x.quantidade_imagens();
    if total == 0 {
        return Err(ErroDocumento::Dados("nenhuma imagem foi escolhida".into()));
    }
    if total > MAX_IMAGENS_PDF {
        return Err(ErroDocumento::Dados(format!(
            "no máximo {MAX_IMAGENS_PDF} imagens por PDF"
        )));
    }
    if let Some(n) = x
        .series
        .iter()
        .flat_map(|s| &s.imagens)
        .position(|i| !i.jpeg.starts_with(&[0xFF, 0xD8, 0xFF]))
    {
        return Err(ErroDocumento::Dados(format!(
            "a imagem {} não é um JPEG",
            n + 1
        )));
    }
    Ok(())
}

/// Páginas prontas para o modelo: título, grade e as imagens (índice global e legenda).
fn paginas(x: &ImagensDoExame) -> Vec<serde_json::Value> {
    let mut saida = Vec::new();
    let mut indice = 0usize;
    for serie in x.series.iter().filter(|s| !s.imagens.is_empty()) {
        let (colunas, linhas) = grade_da_serie(serie.imagens.len());
        let por_pagina = usize::from(colunas) * usize::from(linhas);
        let total = serie.imagens.len();
        for (n, bloco) in serie.imagens.chunks(por_pagina).enumerate() {
            let primeira = n * por_pagina + 1;
            let ultima = primeira + bloco.len() - 1;
            let titulo = if total > por_pagina {
                format!("{}, imagens {primeira} a {ultima} de {total}", serie.titulo)
            } else {
                serie.titulo.clone()
            };
            let imagens: Vec<_> = bloco
                .iter()
                .map(|img| {
                    let v = json!({ "indice": indice, "legenda": img.legenda });
                    indice += 1;
                    v
                })
                .collect();
            saida.push(json!({
                "titulo": titulo, "colunas": colunas, "linhas": linhas, "imagens": imagens,
            }));
        }
    }
    saida
}

fn dados(x: &ImagensDoExame) -> serde_json::Value {
    json!({
        "clinica": {
            "nome": x.clinica.nome,
            "linhas": x.razao_social.iter().cloned().chain(x.clinica.linhas()).collect::<Vec<_>>(),
        },
        "paciente": { "nome": x.paciente_nome.trim(), "nascimento": x.paciente_nascimento.trim() },
        "exame": {
            "descricao": ou_nao_informado(&x.exame),
            "data": ou_nao_informado(&x.data_exame),
            "atendimento": ou_nao_informado(&x.numero_atendimento),
            "solicitante": ou_nao_informado(&x.medico_solicitante),
        },
        "gerado_em": x.gerado_em,
        "paginas": paginas(x),
    })
}

fn anexos(x: &ImagensDoExame) -> Vec<(String, Vec<u8>)> {
    let mut a = vec![("/logo.png".to_owned(), LOGO.to_vec())];
    a.extend(
        x.series
            .iter()
            .flat_map(|s| &s.imagens)
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

    fn serie(titulo: &str, n: usize) -> SerieDoPdf {
        SerieDoPdf {
            titulo: titulo.into(),
            imagens: (0..n)
                .map(|i| ImagemDoExame {
                    jpeg: QUADRO.to_vec(),
                    legenda: format!("Imagem {}", i + 1),
                })
                .collect(),
        }
    }

    fn exemplo(series: Vec<SerieDoPdf>) -> ImagensDoExame {
        ImagensDoExame {
            clinica: Clinica {
                nome: "VidaCor Imagem".into(),
                endereco: Some("Rua Fictícia, 100, Centro".into()),
                telefone: None,
                site: None,
                cnpj: Some("00.000.000/0001-00".into()),
                responsavel_tecnico: None,
            },
            razao_social: Some("CLINICA FICTICIA LTDA".into()),
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
            series,
        }
    }

    fn texto_por_pagina(x: &ImagensDoExame) -> Vec<String> {
        let doc = motor::compilar_documento_com_anexos(FONTE, &dados(x), anexos(x)).unwrap();
        motor::texto(&doc).lines().map(str::to_owned).collect()
    }

    #[test]
    fn grade_pela_quantidade_como_nas_folhas_da_clinica() {
        assert_eq!(grade_da_serie(1), (1, 1));
        assert_eq!(grade_da_serie(4), (2, 2));
        assert_eq!(grade_da_serie(5), (3, 3));
        assert_eq!(grade_da_serie(9), (3, 3));
        assert_eq!(grade_da_serie(10), (4, 5));
        assert_eq!(paginas_da_serie(0), 0);
        assert_eq!(paginas_da_serie(4), 1);
        assert_eq!(paginas_da_serie(20), 1);
        assert_eq!(paginas_da_serie(35), 2);
        assert_eq!(paginas_da_serie(70), 4);
    }

    // FC-VIS-02 (v2): todas as imagens, uma série por página, identificadores em todas as páginas.
    #[test]
    fn cada_serie_comeca_em_pagina_nova_com_os_identificadores() {
        let x = exemplo(vec![
            serie("Série 1, 3D", 4),
            serie("Série 2, PARTES MOLES", 35),
        ]);
        let paginas = texto_por_pagina(&x);
        assert_eq!(paginas.len(), 1 + 2);
        for p in &paginas {
            for esperado in [
                "ANA MARIA FICTICIO",
                "01/01/1980",
                "TC CRANIO",
                "01/01/2026",
                "TESTE0001",
            ] {
                assert!(p.contains(esperado), "{esperado} em {p}");
            }
            assert!(p.contains("laudo assinado"), "aviso VIS-10 em {p}");
            assert!(p.contains("CLINICA FICTICIA LTDA"), "razão social em {p}");
            assert!(p.contains("00.000.000/0001-00"), "CNPJ em {p}");
        }
        assert!(paginas[0].contains("Série 1, 3D") && !paginas[0].contains("PARTES MOLES"));
        assert!(paginas[1].contains("Série 2, PARTES MOLES, imagens 1 a 20 de 35"));
        assert!(paginas[2].contains("imagens 21 a 35 de 35") && paginas[2].contains("Imagem 35"));
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
    fn serie_vazia_nao_gera_pagina() {
        let x = exemplo(vec![serie("Vazia", 0), serie("Série 3", 1)]);
        assert_eq!(texto_por_pagina(&x).len(), 1);
    }

    #[test]
    fn sem_os_dois_identificadores_nao_gera() {
        let mut x = exemplo(vec![serie("S", 1)]);
        x.paciente_nascimento = "  ".into();
        assert!(matches!(imagens_pdf(&x), Err(ErroDocumento::Dados(_))));
        let mut x = exemplo(vec![serie("S", 1)]);
        x.paciente_nome = String::new();
        assert!(matches!(imagens_pdf(&x), Err(ErroDocumento::Dados(_))));
    }

    #[test]
    fn limites_de_imagens_e_formato() {
        assert!(imagens_pdf(&exemplo(vec![])).is_err());
        assert!(imagens_pdf(&exemplo(vec![serie("S", 0)])).is_err());
        assert!(imagens_pdf(&exemplo(vec![serie("S", 300), serie("T", 301)])).is_err());
        let mut x = exemplo(vec![serie("S", 1), serie("T", 1)]);
        x.series[1].imagens[0].jpeg = b"%PDF-1.7 nao e imagem".to_vec();
        let Err(ErroDocumento::Dados(m)) = imagens_pdf(&x) else {
            unreachable!("esperava erro de dados")
        };
        assert!(m.contains("imagem 2"));
    }

    /// SEC-INJ-4: marcação Typst no nome, no título e na legenda sai como texto.
    #[test]
    fn marcacao_no_dado_sai_como_texto() {
        let maligno = r#"Ana #image("/logo.png") #panic("x") *negrito* $x$"#;
        let mut x = exemplo(vec![serie(maligno, 1)]);
        x.paciente_nome = maligno.into();
        x.series[0].imagens[0].legenda = maligno.into();
        let texto: String = texto_por_pagina(&x).concat().split_whitespace().collect();
        for pedaco in ["#image(\"/logo.png\")", "#panic(\"x\")", "*negrito*", "$x$"] {
            assert!(texto.contains(pedaco), "{pedaco} impresso literalmente");
        }
    }

    #[test]
    fn debug_nao_mostra_o_paciente() {
        let d = format!("{:?}", exemplo(vec![serie("S", 2)]));
        assert!(!d.contains("ANA") && !d.contains("1980"));
    }
}
