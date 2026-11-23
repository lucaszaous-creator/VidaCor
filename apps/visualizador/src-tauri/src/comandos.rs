//! Comandos chamados pela tela. Cada um é fino: confere a abertura, chama o núcleo e devolve o
//! resultado. Os diálogos de arquivo são abertos aqui, então a tela nunca manda um caminho.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::{DialogExt, FilePath};
use vidacor_visualizador::{abrir, decodificar, Janela, Limites, Pixels, QuadroDecodificado};

use crate::dto::{self, AcaoDto, InfoQuadroDto, ItemDto, JanelaDto, ResultadoDto, ResumoDto};
use crate::estado::{ChaveQuadro, Estado};
use crate::exportar::{self, Grade};
use crate::registro::{sha256, Lancamento, Registro};
use crate::ErroApp;

fn caminho(f: FilePath) -> Option<PathBuf> {
    f.into_path().ok()
}

fn abrir_caminho(estado: &Estado, p: PathBuf) -> Result<ResumoDto, ErroApp> {
    let exames = abrir(&p, &Limites::default())?;
    let resumo_exames = dto::resumo(0, &exames);
    let id = estado.abrir(exames);
    Ok(ResumoDto {
        abertura: id,
        ..resumo_exames
    })
}

/// Escolher uma pasta (com subpastas, CD/DVD) e abrir. `None`: a pessoa cancelou.
#[tauri::command]
pub async fn abrir_pasta(
    app: AppHandle,
    estado: State<'_, Estado>,
) -> Result<Option<ResumoDto>, ErroApp> {
    let Some(p) = app
        .dialog()
        .file()
        .set_title("Abrir exame: escolha a pasta")
        .blocking_pick_folder()
        .and_then(caminho)
    else {
        return Ok(None);
    };
    abrir_caminho(&estado, p).map(Some)
}

/// Escolher um `.zip` ou um arquivo DICOM e abrir.
#[tauri::command]
pub async fn abrir_arquivo(
    app: AppHandle,
    estado: State<'_, Estado>,
) -> Result<Option<ResumoDto>, ErroApp> {
    let Some(p) = app
        .dialog()
        .file()
        .set_title("Abrir exame: escolha o .zip ou o arquivo DICOM")
        .add_filter("Exame (.zip, .dcm)", &["zip", "dcm", "dicom"])
        .add_filter("Todos os arquivos (DICOM sem extensão)", &["*"])
        .blocking_pick_file()
        .and_then(caminho)
    else {
        return Ok(None);
    };
    abrir_caminho(&estado, p).map(Some)
}

fn decodificado(
    estado: &Estado,
    chave: ChaveQuadro,
) -> Result<(Arc<QuadroDecodificado>, String), ErroApp> {
    let (img, modalidade) = estado.imagem(chave)?;
    if let Some(q) = estado.do_cache(chave) {
        return Ok((q, modalidade));
    }
    let q = Arc::new(decodificar(&img, chave.4, &Limites::default())?);
    estado.guardar(chave, Arc::clone(&q));
    Ok((q, modalidade))
}

/// Decodifica o quadro e devolve tamanho e janelas.
#[tauri::command]
pub async fn quadro(
    estado: State<'_, Estado>,
    abertura: u64,
    estudo: usize,
    serie: usize,
    imagem: usize,
    quadro: u32,
) -> Result<InfoQuadroDto, ErroApp> {
    let (q, modalidade) = decodificado(&estado, (abertura, estudo, serie, imagem, quadro))?;
    Ok(dto::info_quadro(&q, &modalidade))
}

/// Pixels do quadro com a janela pedida, em binário: largura (u32 LE), altura (u32 LE), canais (u8:
/// 1 cinza, 3 RGB) e os pixels linha a linha. Sem perda: a tela mostra exatamente o que o JPEG terá.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // um argumento por coordenada da imagem, como a tela chama
pub async fn pixels(
    estado: State<'_, Estado>,
    abertura: u64,
    estudo: usize,
    serie: usize,
    imagem: usize,
    quadro: u32,
    janela: Option<JanelaDto>,
) -> Result<tauri::ipc::Response, ErroApp> {
    let (q, _) = decodificado(&estado, (abertura, estudo, serie, imagem, quadro))?;
    let img = match q.as_ref() {
        QuadroDecodificado::Cinza(c) => {
            c.renderizar(janela.map_or_else(|| c.janela_padrao(), Janela::from))
        }
        QuadroDecodificado::Cor(c) => c.clone(),
    };
    let (canais, dados) = match img.pixels {
        Pixels::Cinza(p) => (1u8, p),
        Pixels::Rgb(p) => (3u8, p),
    };
    let mut saida = Vec::with_capacity(9 + dados.len());
    saida.extend_from_slice(&img.largura.to_le_bytes());
    saida.extend_from_slice(&img.altura.to_le_bytes());
    saida.push(canais);
    saida.extend_from_slice(&dados);
    Ok(tauri::ipc::Response::new(saida))
}

/// Ações do menu de contexto da imagem (docs/19: as ações vêm do programa, não da tela).
#[tauri::command]
pub fn acoes_da_imagem(marcada: bool, total_marcadas: usize) -> Vec<AcaoDto> {
    let cheio = !marcada && total_marcadas >= exportar::MAX_ITENS;
    vec![
        AcaoDto {
            id: if marcada { "desmarcar" } else { "marcar" },
            rotulo: if marcada {
                "Tirar da exportação"
            } else {
                "Marcar para exportar"
            },
            atalho: Some("M"),
            estado: if cheio { "indisponivel" } else { "disponivel" },
            motivo: cheio.then_some("Limite de imagens por exportação atingido"),
        },
        AcaoDto {
            id: "salvar-jpeg",
            rotulo: "Salvar esta imagem em JPEG",
            atalho: None,
            estado: "disponivel",
            motivo: None,
        },
        AcaoDto {
            id: "janela-padrao",
            rotulo: "Voltar à janela do aparelho",
            atalho: Some("R"),
            estado: "disponivel",
            motivo: None,
        },
    ]
}

fn registro(app: &AppHandle) -> Result<Registro, ErroApp> {
    let pasta = app
        .path()
        .app_data_dir()
        .map_err(|e| ErroApp::Registro(e.to_string()))?;
    Registro::abrir(&pasta)
}

fn usuario() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "desconhecido".into())
}

fn lancar(reg: &Registro, l: &Lancamento) -> Result<(), ErroApp> {
    let quando = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let estacao = gethostname::gethostname().to_string_lossy().into_owned();
    reg.gravar(l, &quando, &usuario(), &estacao)
        .map_err(|e| match e {
            ErroApp::Registro(m) => ErroApp::RegistroAposSalvar(m),
            outro => outro,
        })
}

fn gravar(caminho: &std::path::Path, bytes: &[u8]) -> Result<(), ErroApp> {
    std::fs::write(caminho, bytes).map_err(|e| ErroApp::Gravacao(e.kind().to_string()))
}

/// Salva os JPEG numa pasta escolhida pela pessoa. Arquivo existente nunca é sobrescrito.
#[tauri::command]
pub async fn exportar_jpeg(
    app: AppHandle,
    estado: State<'_, Estado>,
    abertura: u64,
    itens: Vec<ItemDto>,
) -> Result<Option<ResultadoDto>, ErroApp> {
    let reg = registro(&app)?;
    let (arquivos, uid) = estado.com_exame(abertura, |ex| {
        let resolvidos = exportar::resolver(ex, &itens)?;
        let uid = resolvidos
            .first()
            .map(|i| i.estudo.uid.clone())
            .unwrap_or_default();
        Ok((exportar::jpegs(&resolvidos)?, uid))
    })?;
    let Some(pasta) = app
        .dialog()
        .file()
        .set_title("Salvar JPEG: escolha a pasta")
        .blocking_pick_folder()
        .and_then(caminho)
    else {
        return Ok(None);
    };
    let mut hashes = Vec::new();
    let mut falha = None;
    for (nome, bytes) in &arquivos {
        if let Err(e) = gravar(&exportar::nome_livre(&pasta, nome), bytes) {
            falha = Some(e);
            break;
        }
        hashes.push(sha256(bytes));
    }
    let salvas = hashes.len();
    if salvas > 0 {
        lancar(
            &reg,
            &Lancamento {
                ato: "jpeg",
                estudo_uid: uid,
                imagens: salvas,
                sha256: hashes,
                modelo: None,
            },
        )?;
    }
    match falha {
        Some(ErroApp::Gravacao(motivo)) if salvas > 0 => {
            return Err(ErroApp::GravacaoParcial {
                salvas,
                total: arquivos.len(),
                motivo,
            })
        }
        Some(e) => return Err(e),
        None => {}
    }
    Ok(Some(ResultadoDto {
        quantidade: arquivos.len(),
        destino: pasta.to_string_lossy().into_owned(),
    }))
}

/// Gera o PDF para o paciente e salva onde a pessoa escolher. A confirmação com nome e
/// nascimento é feita na tela antes desta chamada (VIS-S2).
#[tauri::command]
pub async fn gerar_pdf(
    app: AppHandle,
    estado: State<'_, Estado>,
    abertura: u64,
    itens: Vec<ItemDto>,
    por_pagina: u8,
) -> Result<Option<ResultadoDto>, ErroApp> {
    let grade = match por_pagina {
        1 => Grade::Uma,
        4 => Grade::Quatro,
        6 => Grade::Seis,
        12 => Grade::Doze,
        _ => return Err(ErroApp::GradeInvalida),
    };
    let reg = registro(&app)?;
    let (pdf, uid, sugerido) = estado.com_exame(abertura, |ex| {
        let resolvidos = exportar::resolver(ex, &itens)?;
        let (pdf, estudo) = exportar::montar_pdf(&resolvidos, grade, chrono::Local::now())?;
        Ok((pdf, estudo.uid.clone(), exportar::nome_pdf(estudo)))
    })?;
    let Some(destino) = app
        .dialog()
        .file()
        .set_title("Salvar PDF do paciente")
        .set_file_name(&sugerido)
        .add_filter("PDF", &["pdf"])
        .blocking_save_file()
        .and_then(caminho)
    else {
        return Ok(None);
    };
    gravar(&destino, &pdf.bytes)?;
    lancar(
        &reg,
        &Lancamento {
            ato: "pdf",
            estudo_uid: uid,
            imagens: itens.len(),
            sha256: vec![pdf.sha256.clone()],
            modelo: Some(pdf.modelo),
        },
    )?;
    Ok(Some(ResultadoDto {
        quantidade: itens.len(),
        destino: destino.to_string_lossy().into_owned(),
    }))
}
