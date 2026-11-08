//! Motor Typst embutido (DEC-2026-11-08-86): um "mundo" fechado com o modelo do documento, um único
//! arquivo de dados em JSON e as fontes embutidas no binário. Nada é lido do disco nem baixado da
//! rede em tempo de execução: qualquer outro arquivo pedido pelo modelo é recusado.
//!
//! O dado entra SEMPRE como dado (SEC-INJ-4): o modelo lê `/dados.json` com `json(...)` e coloca os
//! textos no documento como cadeias; nada vindo do banco é concatenado ao código do modelo.

use crate::ErroDocumento;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use typst::diag::{FileError, FileResult, Severity};
use typst::foundations::{Bytes, Datetime, Duration, Smart};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;
use typst_pdf::{PdfOptions, PdfStandard, PdfStandards, Timestamp};

/// Fontes e biblioteca padrão, montadas uma vez por processo.
struct Base {
    biblioteca: LazyHash<Library>,
    livro: LazyHash<FontBook>,
    fontes: Vec<Font>,
}

fn base() -> &'static Base {
    static BASE: OnceLock<Base> = OnceLock::new();
    BASE.get_or_init(|| {
        let fontes: Vec<Font> = typst_assets::fonts()
            .flat_map(|dados| Font::iter(Bytes::new(dados)))
            .collect();
        Base {
            biblioteca: LazyHash::new(Library::default()),
            livro: LazyHash::new(FontBook::from_fonts(&fontes)),
            fontes,
        }
    })
}

fn arquivo(caminho: &str) -> Result<FileId, ErroDocumento> {
    let virtual_ = VirtualPath::new(caminho)
        .map_err(|_| ErroDocumento::Modelo(format!("caminho inválido: {caminho}")))?;
    Ok(RootedPath::new(VirtualRoot::Project, virtual_).intern())
}

/// O mundo de uma compilação: o modelo (fixo, do binário) e os dados deste documento.
struct Mundo {
    principal: Source,
    dados_id: FileId,
    dados: Bytes,
}

impl World for Mundo {
    fn library(&self) -> &LazyHash<Library> {
        &base().biblioteca
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &base().livro
    }

    fn main(&self) -> FileId {
        self.principal.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.principal.id() {
            Ok(self.principal.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.dados_id {
            Ok(self.dados.clone())
        } else if id == self.principal.id() {
            Ok(Bytes::from_string(self.principal.text().to_owned()))
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        base().fontes.get(index).cloned()
    }

    // O modelo não usa "hoje": a data impressa vem dos dados (determinístico).
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

/// PDF gerado com o SHA-256 dos bytes e a versão do modelo usado.
#[derive(Debug, Clone)]
pub struct Pdf {
    pub bytes: Vec<u8>,
    /// SHA-256 do PDF em hexadecimal minúsculo.
    pub sha256: String,
    pub modelo: &'static str,
}

/// Data e hora (UTC) gravadas como data de criação do PDF.
#[derive(Debug, Clone, Copy)]
pub struct Momento {
    pub ano: i32,
    pub mes: u8,
    pub dia: u8,
    pub hora: u8,
    pub minuto: u8,
    pub segundo: u8,
}

pub(crate) fn compilar_documento(
    modelo: &'static str,
    dados: &serde_json::Value,
) -> Result<PagedDocument, ErroDocumento> {
    let principal_id = arquivo("/principal.typ")?;
    let dados_id = arquivo("/dados.json")?;
    let json = serde_json::to_vec(dados).map_err(|e| ErroDocumento::Dados(e.to_string()))?;
    let mundo = Mundo {
        principal: Source::new(principal_id, modelo.to_owned()),
        dados_id,
        dados: Bytes::new(json),
    };
    let resultado = typst::compile::<PagedDocument>(&mundo).output;
    // Libera o cache de memoização das compilações antigas (servidor de longa duração).
    typst::comemo::evict(10);
    resultado.map_err(|erros| {
        ErroDocumento::Modelo(
            erros
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .map(|d| d.message.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })
}

pub(crate) fn exportar(
    documento: &PagedDocument,
    modelo: &'static str,
    identificador: &str,
    momento: Momento,
) -> Result<Pdf, ErroDocumento> {
    let data = Datetime::from_ymd_hms(
        momento.ano,
        momento.mes,
        momento.dia,
        momento.hora,
        momento.minuto,
        momento.segundo,
    )
    .ok_or_else(|| ErroDocumento::Dados("data do documento inválida".into()))?;
    // PDF/A-2b: arquivamento de longo prazo do prontuário (fontes embutidas, sem dependência externa).
    let padroes = PdfStandards::new(&[PdfStandard::A_2b])
        .map_err(|e| ErroDocumento::Modelo(e.message().to_string()))?;
    let opcoes = PdfOptions {
        ident: Smart::Custom(identificador.to_owned()),
        creator: Smart::Custom(Some("VidaCor".to_owned())),
        timestamp: Some(Timestamp::new_utc(data)),
        standards: padroes,
        ..PdfOptions::default()
    };
    let bytes = typst_pdf::pdf(documento, &opcoes).map_err(|erros| {
        ErroDocumento::Modelo(
            erros
                .iter()
                .map(|d| d.message.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;
    let sha256 = hex(&Sha256::digest(&bytes));
    Ok(Pdf {
        bytes,
        sha256,
        modelo,
    })
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Texto plano do documento (para os testes conferirem que o dado saiu como texto).
#[cfg(test)]
pub(crate) fn texto(documento: &PagedDocument) -> String {
    use typst::layout::{Frame, FrameItem};
    fn percorrer(frame: &Frame, saida: &mut String) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(g) => percorrer(&g.frame, saida),
                FrameItem::Text(t) => {
                    saida.push_str(&t.text);
                    saida.push(' ');
                }
                _ => {}
            }
        }
    }
    let mut saida = String::new();
    for pagina in documento.pages() {
        percorrer(&pagina.frame, &mut saida);
        saida.push('\n');
    }
    saida
}
