//! O que passa entre o Rust e a tela. A tela recebe só o necessário para mostrar; os índices
//! apontam para o exame aberto (conferidos pelo número da abertura).

use serde::{Deserialize, Serialize};
use vidacor_visualizador::{data_br, janelas_prontas, Estudo, Exames, Janela, QuadroDecodificado};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumoDto {
    pub abertura: u64,
    pub estudos: Vec<EstudoDto>,
    pub ignorados: Vec<IgnoradoDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PacienteDto {
    pub nome: String,
    pub nascimento: Option<String>,
    pub sexo: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EstudoDto {
    pub paciente: PacienteDto,
    pub descricao: String,
    pub data: Option<String>,
    pub atendimento: String,
    pub solicitante: String,
    pub modalidades: Vec<String>,
    pub series: Vec<SerieDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerieDto {
    pub numero: Option<i32>,
    /// Páginas que a série ocupa no PDF do exame (mesma regra do modelo, decidida no Rust).
    pub paginas_no_pdf: usize,
    pub descricao: String,
    pub modalidade: String,
    pub imagens: Vec<ImagemDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagemDto {
    pub numero: Option<i32>,
    pub quadros: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoradoDto {
    pub nome: String,
    pub motivo: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JanelaDto {
    pub centro: f64,
    pub largura: f64,
}

impl From<Janela> for JanelaDto {
    fn from(j: Janela) -> Self {
        JanelaDto {
            centro: j.centro,
            largura: j.largura,
        }
    }
}

impl From<JanelaDto> for Janela {
    /// Valor vindo da tela passa pela correção de [`Janela::nova`] (largura mínima 1, finito).
    fn from(j: JanelaDto) -> Self {
        Janela::nova(j.centro, j.largura)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JanelaProntaDto {
    pub nome: &'static str,
    pub centro: f64,
    pub largura: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoQuadroDto {
    pub largura: u32,
    pub altura: u32,
    pub cor: bool,
    pub janela_padrao: Option<JanelaDto>,
    pub janelas_do_arquivo: Vec<JanelaDto>,
    pub prontas: Vec<JanelaProntaDto>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    pub estudo: usize,
    pub serie: usize,
    pub imagem: usize,
    pub quadro: u32,
    /// Janela com que a imagem foi marcada na tela (ausente na imagem colorida).
    pub janela: Option<JanelaDto>,
}

/// Série pedida para o PDF do exame, com a janela escolhida para ela (nula = a do aparelho).
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriePedidaDto {
    pub serie: usize,
    pub janela: Option<JanelaDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoDto {
    pub quantidade: usize,
    /// Pasta ou arquivo escolhido pela pessoa, para a mensagem "salvo em".
    pub destino: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcaoDto {
    pub id: &'static str,
    pub rotulo: &'static str,
    pub atalho: Option<&'static str>,
    pub estado: &'static str,
    pub motivo: Option<&'static str>,
}

fn estudo_dto(e: &Estudo) -> EstudoDto {
    EstudoDto {
        paciente: PacienteDto {
            nome: e.paciente.nome(),
            nascimento: e.paciente.nascimento(),
            sexo: e.paciente.sexo.clone(),
        },
        descricao: e.descricao.clone(),
        data: data_br(&e.data_dicom),
        atendimento: e.numero_atendimento.clone(),
        solicitante: e.medico_solicitante.clone(),
        modalidades: e.modalidades(),
        series: e
            .series
            .iter()
            .map(|s| SerieDto {
                numero: s.numero,
                paginas_no_pdf: vidacor_documentos::paginas_da_serie(s.imagens.len()),
                descricao: s.descricao.clone(),
                modalidade: s.modalidade.clone(),
                imagens: s
                    .imagens
                    .iter()
                    .map(|i| ImagemDto {
                        numero: i.numero,
                        quadros: i.quadros,
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub fn resumo(abertura: u64, exames: &Exames) -> ResumoDto {
    ResumoDto {
        abertura,
        estudos: exames.estudos.iter().map(estudo_dto).collect(),
        ignorados: exames
            .ignorados
            .iter()
            .map(|i| IgnoradoDto {
                nome: i.origem.nome(),
                motivo: i.motivo.to_string(),
            })
            .collect(),
    }
}

pub fn info_quadro(q: &QuadroDecodificado, modalidade: &str) -> InfoQuadroDto {
    let prontas = janelas_prontas(modalidade)
        .iter()
        .map(|p| {
            let j = p.janela();
            JanelaProntaDto {
                nome: p.nome(),
                centro: j.centro,
                largura: j.largura,
            }
        })
        .collect();
    match q {
        QuadroDecodificado::Cinza(c) => InfoQuadroDto {
            largura: c.largura,
            altura: c.altura,
            cor: false,
            janela_padrao: Some(c.janela_padrao().into()),
            janelas_do_arquivo: c.janelas_do_arquivo.iter().map(|j| (*j).into()).collect(),
            prontas,
        },
        QuadroDecodificado::Cor(i) => InfoQuadroDto {
            largura: i.largura,
            altura: i.altura,
            cor: true,
            janela_padrao: None,
            janelas_do_arquivo: Vec::new(),
            prontas: Vec::new(),
        },
    }
}
