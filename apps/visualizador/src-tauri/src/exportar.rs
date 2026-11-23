//! Exportação de PDF e JPEG, sem Tauri (testável). O PDF leva todas as imagens das séries pedidas de
//! um exame, com os dados do próprio exame (DEC-2026-11-23-16); o JPEG leva as imagens marcadas, cada
//! uma com a janela com que foi marcada.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Local, Timelike, Utc};
use vidacor_documentos::{
    imagens_pdf, Clinica, ImagemDoExame, ImagensDoExame, Momento, Pdf, SerieDoPdf,
};
use vidacor_visualizador::{
    codificar_jpeg, decodificar, nome_arquivo_jpeg, Estudo, Exames, Imagem, ImagemRenderizada,
    Janela, Limites, QuadroDecodificado, Serie, QUALIDADE_PADRAO,
};

use crate::dto::{ItemDto, SeriePedidaDto};
use crate::ErroApp;

/// Cabeçalho da clínica no PDF (dados informados pelo dono do produto em 23/11/2026, DEC-2026-11-23-14).
/// Telefone e responsável técnico ainda a confirmar (docs/31 P-06); campo ausente não é impresso.
pub const CLINICA: &str = "VidaCor Imagem";
pub const RAZAO_SOCIAL: &str = "VIDACOR SERVICOS MEDICOS MACAE LTDA";
pub const ENDERECO: &str = "Av. Nossa Senhora da Glória, 1225, Praia Campista, Macaé, RJ";
pub const CNPJ: &str = "07.000.379/0001-89";

/// Limite por exportação (o mesmo do PDF).
pub const MAX_ITENS: usize = vidacor_documentos::MAX_IMAGENS_PDF;

/// Item escolhido já localizado no exame aberto.
#[derive(Debug, Clone)]
pub struct ItemResolvido<'a> {
    pub estudo: &'a Estudo,
    pub serie: &'a Serie,
    pub imagem: &'a Imagem,
    pub quadro: u32,
    pub janela: Option<Janela>,
}

/// Localiza os itens pelos índices. Índice fora do exame é erro (a tela não inventa imagem).
pub fn resolver<'a>(
    exames: &'a Exames,
    itens: &[ItemDto],
) -> Result<Vec<ItemResolvido<'a>>, ErroApp> {
    if itens.len() > MAX_ITENS {
        return Err(ErroApp::MuitasImagens(MAX_ITENS));
    }
    itens
        .iter()
        .map(|i| {
            let estudo = exames
                .estudos
                .get(i.estudo)
                .ok_or(ErroApp::ItemInexistente)?;
            let serie = estudo.series.get(i.serie).ok_or(ErroApp::ItemInexistente)?;
            let imagem = serie
                .imagens
                .get(i.imagem)
                .ok_or(ErroApp::ItemInexistente)?;
            if i.quadro >= imagem.quadros {
                return Err(ErroApp::ItemInexistente);
            }
            Ok(ItemResolvido {
                estudo,
                serie,
                imagem,
                quadro: i.quadro,
                janela: i.janela.map(Janela::from),
            })
        })
        .collect()
}

/// Renderiza com a janela do item (ou a padrão do arquivo). Devolve a janela usada (cinza).
fn renderizar(item: &ItemResolvido<'_>) -> Result<(ImagemRenderizada, Option<Janela>), ErroApp> {
    Ok(
        match decodificar(item.imagem, item.quadro, &Limites::default())? {
            QuadroDecodificado::Cinza(q) => {
                let j = item.janela.unwrap_or_else(|| q.janela_padrao());
                (q.renderizar(j), Some(j))
            }
            QuadroDecodificado::Cor(c) => (c, None),
        },
    )
}

fn numero(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{n:.0}")
    } else {
        format!("{n:.1}")
    }
}

/// Legenda da imagem no PDF: série, imagem, quadro e janela.
pub fn legenda(item: &ItemResolvido<'_>, janela: Option<Janela>) -> String {
    let mut partes = Vec::new();
    let serie = match (item.serie.numero, item.serie.descricao.is_empty()) {
        (Some(n), false) => format!("Série {n} ({})", item.serie.descricao),
        (Some(n), true) => format!("Série {n}"),
        (None, false) => format!("Série {}", item.serie.descricao),
        (None, true) => "Série sem número".to_owned(),
    };
    partes.push(serie);
    if let Some(n) = item.imagem.numero {
        partes.push(format!("imagem {n}"));
    }
    if item.imagem.quadros > 1 {
        partes.push(format!(
            "quadro {} de {}",
            item.quadro + 1,
            item.imagem.quadros
        ));
    }
    if let Some(j) = janela {
        partes.push(format!("janela {}/{}", numero(j.centro), numero(j.largura)));
    }
    partes.join(", ")
}

fn momento(agora: DateTime<Local>) -> Momento {
    let u = agora.with_timezone(&Utc);
    Momento {
        ano: u.year(),
        mes: u.month() as u8,
        dia: u.day() as u8,
        hora: u.hour() as u8,
        minuto: u.minute() as u8,
        segundo: u.second() as u8,
    }
}

/// Título da série na página: número, descrição e a janela, quando foi escolhida uma para a série.
pub fn titulo_da_serie(serie: &Serie, janela: Option<Janela>) -> String {
    let mut t = match (serie.numero, serie.descricao.is_empty()) {
        (Some(n), false) => format!("Série {n}, {}", serie.descricao),
        (Some(n), true) => format!("Série {n}"),
        (None, false) => format!("Série {}", serie.descricao),
        (None, true) => "Série sem número".to_owned(),
    };
    if let Some(j) = janela {
        t.push_str(&format!(
            ", janela {}/{}",
            numero(j.centro),
            numero(j.largura)
        ));
    }
    t
}

/// Legenda curta de cada imagem no PDF (as imagens ficam pequenas em 4×5).
fn legenda_curta(imagem: &Imagem, posicao: usize, janela: Option<Janela>) -> String {
    let mut partes = vec![format!(
        "Imagem {}",
        imagem
            .numero
            .map_or(posicao, |n| usize::try_from(n).unwrap_or(posicao))
    )];
    if imagem.quadros > 1 {
        partes.push(format!("vídeo de {} quadros, quadro 1", imagem.quadros));
    }
    if let Some(j) = janela {
        partes.push(format!("janela {}/{}", numero(j.centro), numero(j.largura)));
    }
    partes.join(", ")
}

/// Séries pedidas, sem repetição e na ordem do exame. Índice fora do exame é erro.
pub fn series_pedidas<'a>(
    estudo: &'a Estudo,
    pedidas: &[SeriePedidaDto],
) -> Result<Vec<(&'a Serie, Option<Janela>)>, ErroApp> {
    let mut indices: Vec<&SeriePedidaDto> = Vec::new();
    for p in pedidas {
        if estudo.series.get(p.serie).is_none() {
            return Err(ErroApp::ItemInexistente);
        }
        if !indices.iter().any(|x| x.serie == p.serie) {
            indices.push(p);
        }
    }
    indices.sort_by_key(|p| p.serie);
    if indices.is_empty() {
        return Err(vidacor_visualizador::Erro::NadaEscolhido.into());
    }
    let escolhidas: Vec<_> = indices
        .iter()
        .filter_map(|p| {
            estudo
                .series
                .get(p.serie)
                .map(|s| (s, p.janela.map(Janela::from)))
        })
        .collect();
    let total: usize = escolhidas.iter().map(|(s, _)| s.imagens.len()).sum();
    if total > MAX_ITENS {
        return Err(ErroApp::MuitasImagens(MAX_ITENS));
    }
    Ok(escolhidas)
}

/// Monta o PDF do exame com todas as imagens das séries pedidas (uma série por página, grade pela
/// quantidade). Um só exame por PDF (VIS-S1) e os dois identificadores conferidos antes de tudo; os
/// dados impressos vêm do exame, nunca da tela (G1). Vídeo (vários quadros) entra pelo primeiro quadro.
pub fn montar_pdf<'a>(
    exames: &'a Exames,
    estudo: usize,
    pedidas: &[SeriePedidaDto],
    agora: DateTime<Local>,
) -> Result<(Pdf, &'a Estudo, usize), ErroApp> {
    let estudo = exames.estudos.get(estudo).ok_or(ErroApp::ItemInexistente)?;
    let p = &estudo.paciente;
    let Some(nascimento) = p.nascimento().filter(|_| !p.nome().is_empty()) else {
        return Err(ErroApp::SemIdentificadores);
    };
    let escolhidas = series_pedidas(estudo, pedidas)?;
    let mut series = Vec::new();
    let mut quantidade = 0;
    for (serie, janela) in escolhidas {
        let imagens = serie
            .imagens
            .iter()
            .enumerate()
            .map(|(i, imagem)| {
                let item = ItemResolvido {
                    estudo,
                    serie,
                    imagem,
                    quadro: 0,
                    janela,
                };
                let (img, usada) = renderizar(&item)?;
                Ok(ImagemDoExame {
                    jpeg: codificar_jpeg(&img, QUALIDADE_PADRAO)?,
                    legenda: legenda_curta(imagem, i + 1, usada),
                })
            })
            .collect::<Result<Vec<_>, ErroApp>>()?;
        quantidade += imagens.len();
        series.push(SerieDoPdf {
            titulo: titulo_da_serie(serie, janela),
            imagens,
        });
    }
    let dados = ImagensDoExame {
        clinica: Clinica {
            nome: CLINICA.into(),
            endereco: Some(ENDERECO.into()),
            telefone: None,
            site: None,
            cnpj: Some(CNPJ.into()),
            responsavel_tecnico: None,
        },
        razao_social: Some(RAZAO_SOCIAL.into()),
        paciente_nome: p.nome(),
        paciente_nascimento: nascimento,
        exame: estudo.descricao.clone(),
        data_exame: vidacor_visualizador::data_br(&estudo.data_dicom).unwrap_or_default(),
        numero_atendimento: estudo.numero_atendimento.clone(),
        medico_solicitante: estudo.medico_solicitante.clone(),
        gerado_em: agora.format("%d/%m/%Y às %H:%M").to_string(),
        momento: momento(agora),
        series,
    };
    let pdf = imagens_pdf(&dados).map_err(|e| ErroApp::Documento(e.to_string()))?;
    Ok((pdf, estudo, quantidade))
}

/// JPEG de cada item com o nome sem dado pessoal.
pub fn jpegs(itens: &[ItemResolvido<'_>]) -> Result<Vec<(String, Vec<u8>)>, ErroApp> {
    if itens.is_empty() {
        return Err(vidacor_visualizador::Erro::NadaEscolhido.into());
    }
    itens
        .iter()
        .map(|item| {
            let (img, _) = renderizar(item)?;
            let quadro = (item.imagem.quadros > 1).then_some(item.quadro);
            let nome = nome_arquivo_jpeg(
                &item.estudo.numero_atendimento,
                item.serie.numero,
                item.imagem.numero,
                quadro,
            );
            Ok((nome, codificar_jpeg(&img, QUALIDADE_PADRAO)?))
        })
        .collect()
}

/// Nome sugerido para o PDF, sem dado pessoal.
pub fn nome_pdf(estudo: &Estudo) -> String {
    let base = nome_arquivo_jpeg(&estudo.numero_atendimento, None, None, None);
    base.replacen("exame", "imagens-exame", 1)
        .replace(".jpg", ".pdf")
}

/// Caminho que não sobrescreve arquivo existente: acrescenta -2, -3… antes da extensão (nada se perde).
pub fn nome_livre(pasta: &Path, nome: &str) -> PathBuf {
    let caminho = pasta.join(nome);
    if !caminho.exists() {
        return caminho;
    }
    let (base, ext) = nome.rsplit_once('.').unwrap_or((nome, ""));
    (2..)
        .map(|n| {
            if ext.is_empty() {
                pasta.join(format!("{base}-{n}"))
            } else {
                pasta.join(format!("{base}-{n}.{ext}"))
            }
        })
        .find(|p| !p.exists())
        .unwrap_or(caminho)
}
