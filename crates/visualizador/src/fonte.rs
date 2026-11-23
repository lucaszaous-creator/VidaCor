//! De onde vêm os arquivos: pasta (com subpastas, como CD/DVD com DICOMDIR), `.zip` ou arquivo
//! solto. Nada é extraído para o disco: o `.zip` é lido em memória, entrada por entrada.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::Erro;

/// Limites contra arquivo malformado, gigante ou `.zip` bomba (docs/31 VIS-S6, docs/17
/// disponibilidade). Os padrões cobrem com folga um exame grande (TC de milhares de cortes).
#[derive(Debug, Clone)]
pub struct Limites {
    /// Arquivos e entradas de `.zip` examinados por abertura.
    pub max_arquivos: usize,
    /// Tamanho de um arquivo ou de uma entrada de `.zip` depois de descomprimida.
    pub max_bytes_arquivo: u64,
    /// Soma de tudo o que é lido numa abertura.
    pub max_bytes_total: u64,
    /// Pixels decodificados de um quadro (linhas × colunas × amostras).
    pub max_pixels_quadro: u64,
}

impl Default for Limites {
    fn default() -> Self {
        Limites {
            max_arquivos: 50_000,
            max_bytes_arquivo: 2 * 1024 * 1024 * 1024,
            max_bytes_total: 16 * 1024 * 1024 * 1024,
            max_pixels_quadro: 16_384 * 16_384 * 3,
        }
    }
}

/// Onde está um arquivo DICOM. Guardado no índice para ler os pixels só quando a imagem for
/// mostrada, sem manter o exame inteiro na memória.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origem {
    Arquivo(PathBuf),
    /// `indice` é a posição da entrada no `.zip`; `entrada` é o nome, só para mostrar.
    Zip {
        zip: PathBuf,
        indice: usize,
        entrada: String,
    },
}

impl Origem {
    /// Nome curto para a tela (nome do arquivo ou da entrada do `.zip`).
    pub fn nome(&self) -> String {
        let caminho = match self {
            Origem::Arquivo(p) => p.to_string_lossy().into_owned(),
            Origem::Zip { entrada, .. } => entrada.clone(),
        };
        caminho
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
            .to_owned()
    }

    /// Lê os bytes do arquivo respeitando o limite de tamanho.
    pub fn ler(&self, limites: &Limites) -> Result<Vec<u8>, Erro> {
        match self {
            Origem::Arquivo(p) => ler_limitado(File::open(p).map_err(io)?, limites),
            Origem::Zip { zip, indice, .. } => {
                let mut arquivo = abrir_zip(zip)?;
                let e = arquivo.by_index(*indice).map_err(|_| Erro::ZipInvalido)?;
                ler_limitado(e, limites)
            }
        }
    }
}

fn io(e: std::io::Error) -> Erro {
    if e.kind() == std::io::ErrorKind::NotFound {
        Erro::NaoEncontrado
    } else {
        Erro::Leitura(e)
    }
}

/// Lê no máximo `max_bytes_arquivo`; se houver mais, é limite excedido (não confia no tamanho
/// declarado no `.zip`, que pode mentir).
fn ler_limitado(leitor: impl Read, limites: &Limites) -> Result<Vec<u8>, Erro> {
    let mut bytes = Vec::new();
    leitor
        .take(limites.max_bytes_arquivo.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() as u64 > limites.max_bytes_arquivo {
        return Err(Erro::LimiteExcedido {
            limite: "tamanho por arquivo",
        });
    }
    Ok(bytes)
}

fn abrir_zip(zip: &Path) -> Result<zip::ZipArchive<File>, Erro> {
    zip::ZipArchive::new(File::open(zip).map_err(io)?).map_err(|_| Erro::ZipInvalido)
}

fn parece_zip(p: &Path) -> bool {
    let mut cabeca = [0u8; 4];
    File::open(p)
        .and_then(|mut f| f.read_exact(&mut cabeca))
        .is_ok()
        && cabeca == *b"PK\x03\x04"
}

/// Lista as origens a examinar. Pasta: recursiva, sem seguir atalhos (links simbólicos), com os
/// `.zip` encontrados abertos também. `.zip` dentro de `.zip` não é aberto.
pub(crate) fn listar(caminho: &Path, limites: &Limites) -> Result<Vec<Origem>, Erro> {
    let meta = std::fs::metadata(caminho).map_err(io)?;
    let mut saida = Vec::new();
    if meta.is_dir() {
        listar_pasta(caminho, limites, &mut saida)?;
    } else if parece_zip(caminho) {
        listar_zip(caminho, limites, &mut saida)?;
    } else {
        saida.push(Origem::Arquivo(caminho.to_path_buf()));
    }
    Ok(saida)
}

fn empurrar(saida: &mut Vec<Origem>, o: Origem, limites: &Limites) -> Result<(), Erro> {
    if saida.len() >= limites.max_arquivos {
        return Err(Erro::LimiteExcedido {
            limite: "quantidade de arquivos",
        });
    }
    saida.push(o);
    Ok(())
}

fn listar_pasta(pasta: &Path, limites: &Limites, saida: &mut Vec<Origem>) -> Result<(), Erro> {
    let mut entradas: Vec<_> = std::fs::read_dir(pasta)
        .map_err(io)?
        .filter_map(Result::ok)
        .collect();
    entradas.sort_by_key(|e| e.file_name());
    for e in entradas {
        let tipo = e.file_type().map_err(io)?;
        let p = e.path();
        if tipo.is_symlink() {
            continue;
        }
        if tipo.is_dir() {
            listar_pasta(&p, limites, saida)?;
        } else if tipo.is_file() {
            if parece_zip(&p) {
                listar_zip(&p, limites, saida)?;
            } else {
                empurrar(saida, Origem::Arquivo(p), limites)?;
            }
        }
    }
    Ok(())
}

fn listar_zip(zip: &Path, limites: &Limites, saida: &mut Vec<Origem>) -> Result<(), Erro> {
    let mut arquivo = abrir_zip(zip)?;
    if arquivo.len() > limites.max_arquivos {
        return Err(Erro::LimiteExcedido {
            limite: "quantidade de arquivos",
        });
    }
    let mut nomes = Vec::new();
    for indice in 0..arquivo.len() {
        let e = arquivo.by_index(indice).map_err(|_| Erro::ZipInvalido)?;
        if e.is_file() {
            let nome = String::from_utf8_lossy(e.name_raw()).into_owned();
            nomes.push((nome, indice));
        }
    }
    nomes.sort();
    for (entrada, indice) in nomes {
        let o = Origem::Zip {
            zip: zip.to_path_buf(),
            indice,
            entrada,
        };
        empurrar(saida, o, limites)?;
    }
    Ok(())
}
