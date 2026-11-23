//! Índice do exame: lê o cabeçalho de cada arquivo e agrupa em paciente, estudo e série
//! (docs/31 VIS-01, VIS-02). Os pixels não ficam na memória; cada imagem guarda a [`Origem`].

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::path::Path;

use dicom_dictionary_std::tags;
use dicom_object::file::ReadPreamble;
use dicom_object::{DefaultDicomObject, OpenFileOptions};

use crate::fonte::{listar, Limites, Origem};
use crate::texto::{data_br, nome_pessoa};
use crate::Erro;

/// UID da classe do DICOMDIR (Media Storage Directory Storage).
const CLASSE_DICOMDIR: &str = "1.2.840.10008.1.3.10";

/// Paciente como gravado no exame. O `Debug` não mostra os dados (VIS-S3).
#[derive(Clone, PartialEq, Eq)]
pub struct Paciente {
    /// PatientName como está no arquivo (formato DICOM, com `^`).
    pub nome_dicom: String,
    pub id: String,
    /// PatientBirthDate como está no arquivo (`AAAAMMDD`), vazio se ausente.
    pub nascimento_dicom: String,
    pub sexo: String,
}

impl Paciente {
    /// Nome para mostrar e imprimir, na ordem de leitura.
    pub fn nome(&self) -> String {
        nome_pessoa(&self.nome_dicom)
    }

    /// Nascimento em `DD/MM/AAAA`, ou `None` se ausente ou fora do formato.
    pub fn nascimento(&self) -> Option<String> {
        data_br(&self.nascimento_dicom)
    }

    /// Os dois identificadores existem (nome e nascimento). Sem eles o PDF não é gerado (E3).
    pub fn tem_dois_identificadores(&self) -> bool {
        !self.nome().is_empty() && self.nascimento().is_some()
    }
}

impl fmt::Debug for Paciente {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Paciente { .. }")
    }
}

#[derive(Debug, Clone)]
pub struct Estudo {
    pub uid: String,
    pub paciente: Paciente,
    pub descricao: String,
    /// StudyDate (`AAAAMMDD`); use [`data_br`] para mostrar.
    pub data_dicom: String,
    /// AccessionNumber (número do atendimento ou do pedido).
    pub numero_atendimento: String,
    pub medico_solicitante: String,
    pub instituicao: String,
    pub series: Vec<Serie>,
}

impl Estudo {
    pub fn quantidade_imagens(&self) -> usize {
        self.series.iter().map(|s| s.imagens.len()).sum()
    }

    /// Modalidades do estudo, sem repetição, na ordem das séries (ex.: `["CT"]`).
    pub fn modalidades(&self) -> Vec<String> {
        let mut vistas = Vec::new();
        for s in &self.series {
            if !s.modalidade.is_empty() && !vistas.contains(&s.modalidade) {
                vistas.push(s.modalidade.clone());
            }
        }
        vistas
    }
}

#[derive(Debug, Clone)]
pub struct Serie {
    pub uid: String,
    pub numero: Option<i32>,
    pub descricao: String,
    pub modalidade: String,
    /// Em ordem de exibição: posição ao longo da normal do corte, depois InstanceNumber.
    pub imagens: Vec<Imagem>,
}

#[derive(Debug, Clone)]
pub struct Imagem {
    pub sop_uid: String,
    pub numero: Option<i32>,
    /// Quantidade de quadros (ultrassom em cine, angiografia); 1 para imagem simples.
    pub quadros: u32,
    pub linhas: u32,
    pub colunas: u32,
    /// Posição do corte ao longo da normal (mm), quando o aparelho grava a geometria.
    pub posicao: Option<f64>,
    pub origem: Origem,
}

/// Por que um arquivo não entrou no exame. A tela lista os ignorados (VIS-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotivoIgnorado {
    NaoDicom,
    Danificado,
    /// DICOMDIR, laudo estruturado, estado de apresentação ou outro objeto sem pixels.
    SemImagem,
    /// Mesma imagem (SOPInstanceUID) já aberta por outro arquivo.
    Duplicado,
    GrandeDemais,
    /// O arquivo não pôde ser lido (permissão, disco).
    Ilegivel,
}

impl fmt::Display for MotivoIgnorado {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MotivoIgnorado::NaoDicom => "não é um arquivo DICOM",
            MotivoIgnorado::Danificado => "arquivo DICOM danificado ou incompleto",
            MotivoIgnorado::SemImagem => "arquivo DICOM sem imagem (índice, laudo ou outro objeto)",
            MotivoIgnorado::Duplicado => "imagem repetida, já aberta por outro arquivo",
            MotivoIgnorado::GrandeDemais => "arquivo maior que o limite",
            MotivoIgnorado::Ilegivel => "não foi possível ler o arquivo",
        })
    }
}

#[derive(Debug, Clone)]
pub struct Ignorado {
    pub origem: Origem,
    pub motivo: MotivoIgnorado,
}

/// Resultado da abertura: estudos agrupados e arquivos que ficaram de fora com o motivo.
#[derive(Debug, Clone, Default)]
pub struct Exames {
    pub estudos: Vec<Estudo>,
    pub ignorados: Vec<Ignorado>,
}

/// Abre uma pasta, um `.zip` ou um arquivo e monta o índice.
/// Erro só quando a própria origem não abre (não existe, `.zip` inválido, limite de quantidade);
/// arquivo ruim lá dentro vai para [`Exames::ignorados`] e o resto abre normalmente.
pub fn abrir(caminho: &Path, limites: &Limites) -> Result<Exames, Erro> {
    let origens = listar(caminho, limites)?;
    let mut total: u64 = 0;
    let mut vistos = HashSet::new();
    let mut grupos: BTreeMap<ChaveEstudo, (Estudo, BTreeMap<String, Serie>)> = BTreeMap::new();
    let mut ignorados = Vec::new();

    for origem in origens {
        let bytes = match origem.ler(limites) {
            Ok(b) => b,
            Err(Erro::LimiteExcedido { .. }) => {
                ignorados.push(Ignorado {
                    origem,
                    motivo: MotivoIgnorado::GrandeDemais,
                });
                continue;
            }
            Err(Erro::ZipInvalido) => return Err(Erro::ZipInvalido),
            Err(_) => {
                ignorados.push(Ignorado {
                    origem,
                    motivo: MotivoIgnorado::Ilegivel,
                });
                continue;
            }
        };
        total = total.saturating_add(bytes.len() as u64);
        if total > limites.max_bytes_total {
            return Err(Erro::LimiteExcedido {
                limite: "tamanho total do exame",
            });
        }
        let obj = match ler_dicom(&bytes) {
            Ok(o) => o,
            Err(e) => {
                let motivo = match e {
                    Erro::NaoDicom => MotivoIgnorado::NaoDicom,
                    _ => MotivoIgnorado::Danificado,
                };
                ignorados.push(Ignorado { origem, motivo });
                continue;
            }
        };
        drop(bytes);
        if obj.meta().media_storage_sop_class_uid() == CLASSE_DICOMDIR
            || obj.get(tags::PIXEL_DATA).is_none()
        {
            ignorados.push(Ignorado {
                origem,
                motivo: MotivoIgnorado::SemImagem,
            });
            continue;
        }
        let sop_uid = texto(&obj, tags::SOP_INSTANCE_UID);
        if !sop_uid.is_empty() && !vistos.insert(sop_uid.clone()) {
            ignorados.push(Ignorado {
                origem,
                motivo: MotivoIgnorado::Duplicado,
            });
            continue;
        }
        acrescentar(&mut grupos, &obj, sop_uid, origem);
    }

    let mut estudos: Vec<Estudo> = grupos
        .into_values()
        .map(|(mut estudo, series)| {
            estudo.series = series.into_values().collect();
            for s in &mut estudo.series {
                ordenar(&mut s.imagens);
            }
            estudo
                .series
                .sort_by(|a, b| (a.numero, &a.uid).cmp(&(b.numero, &b.uid)));
            estudo
        })
        .collect();
    estudos.sort_by(|a, b| {
        (&a.paciente.nome_dicom, &a.data_dicom, &a.uid).cmp(&(
            &b.paciente.nome_dicom,
            &b.data_dicom,
            &b.uid,
        ))
    });
    Ok(Exames { estudos, ignorados })
}

/// Lê o objeto DICOM inteiro (inclusive os pixels, para notar arquivo cortado).
/// Aceita arquivo com o preâmbulo de 128 bytes (o normal) e sem ele.
pub(crate) fn ler_dicom(bytes: &[u8]) -> Result<DefaultDicomObject, Erro> {
    let preambulo = if bytes.len() >= 132 && &bytes[128..132] == b"DICM" {
        ReadPreamble::Always
    } else if bytes.starts_with(b"DICM") {
        ReadPreamble::Never
    } else {
        return Err(Erro::NaoDicom);
    };
    OpenFileOptions::new()
        .read_preamble(preambulo)
        .from_reader(bytes)
        .map_err(|_| Erro::Danificado)
}

pub(crate) fn texto(obj: &DefaultDicomObject, tag: dicom_core::Tag) -> String {
    obj.get(tag)
        .and_then(|e| e.to_str().ok())
        .map(|s| {
            s.trim_matches(|c: char| c.is_whitespace() || c == '\0')
                .to_owned()
        })
        .unwrap_or_default()
}

fn inteiro(obj: &DefaultDicomObject, tag: dicom_core::Tag) -> Option<i32> {
    obj.get(tag).and_then(|e| e.to_int::<i32>().ok())
}

fn numeros(obj: &DefaultDicomObject, tag: dicom_core::Tag) -> Option<Vec<f64>> {
    obj.get(tag).and_then(|e| e.to_multi_float64().ok())
}

/// Posição do corte projetada na normal do plano (produto vetorial das direções de linha e
/// coluna), como fazem os visualizadores de RIS/PACS para ordenar cortes.
fn posicao(obj: &DefaultDicomObject) -> Option<f64> {
    let p = numeros(obj, tags::IMAGE_POSITION_PATIENT)?;
    let o = numeros(obj, tags::IMAGE_ORIENTATION_PATIENT)?;
    if p.len() != 3 || o.len() != 6 {
        return None;
    }
    let n = [
        o[1] * o[5] - o[2] * o[4],
        o[2] * o[3] - o[0] * o[5],
        o[0] * o[4] - o[1] * o[3],
    ];
    let v = n[0] * p[0] + n[1] * p[1] + n[2] * p[2];
    v.is_finite().then_some(v)
}

fn ordenar(imagens: &mut [Imagem]) {
    imagens.sort_by(|a, b| {
        let pos = match (a.posicao, b.posicao) {
            (Some(x), Some(y)) => x.total_cmp(&y),
            _ => std::cmp::Ordering::Equal,
        };
        pos.then_with(|| a.numero.cmp(&b.numero))
            .then_with(|| a.sop_uid.cmp(&b.sop_uid))
    });
}

/// Paciente e estudo juntos: o mesmo StudyInstanceUID com paciente diferente nunca é unido.
type ChaveEstudo = (String, String, String, String);

fn acrescentar(
    grupos: &mut BTreeMap<ChaveEstudo, (Estudo, BTreeMap<String, Serie>)>,
    obj: &DefaultDicomObject,
    sop_uid: String,
    origem: Origem,
) {
    let paciente = Paciente {
        nome_dicom: texto(obj, tags::PATIENT_NAME),
        id: texto(obj, tags::PATIENT_ID),
        nascimento_dicom: texto(obj, tags::PATIENT_BIRTH_DATE),
        sexo: texto(obj, tags::PATIENT_SEX),
    };
    let uid_estudo = texto(obj, tags::STUDY_INSTANCE_UID);
    let chave = (
        paciente.id.clone(),
        paciente.nome_dicom.clone(),
        paciente.nascimento_dicom.clone(),
        uid_estudo.clone(),
    );
    let (_, series) = grupos.entry(chave).or_insert_with(|| {
        (
            Estudo {
                uid: uid_estudo,
                paciente,
                descricao: texto(obj, tags::STUDY_DESCRIPTION),
                data_dicom: texto(obj, tags::STUDY_DATE),
                numero_atendimento: texto(obj, tags::ACCESSION_NUMBER),
                medico_solicitante: nome_pessoa(&texto(obj, tags::REFERRING_PHYSICIAN_NAME)),
                instituicao: texto(obj, tags::INSTITUTION_NAME),
                series: Vec::new(),
            },
            BTreeMap::new(),
        )
    });
    let uid_serie = texto(obj, tags::SERIES_INSTANCE_UID);
    let serie = series.entry(uid_serie.clone()).or_insert_with(|| Serie {
        uid: uid_serie,
        numero: inteiro(obj, tags::SERIES_NUMBER),
        descricao: texto(obj, tags::SERIES_DESCRIPTION),
        modalidade: texto(obj, tags::MODALITY),
        imagens: Vec::new(),
    });
    serie.imagens.push(Imagem {
        sop_uid,
        numero: inteiro(obj, tags::INSTANCE_NUMBER),
        quadros: inteiro(obj, tags::NUMBER_OF_FRAMES)
            .and_then(|q| u32::try_from(q).ok())
            .filter(|q| *q > 0)
            .unwrap_or(1),
        linhas: inteiro(obj, tags::ROWS)
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0),
        colunas: inteiro(obj, tags::COLUMNS)
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(0),
        posicao: posicao(obj),
        origem,
    });
}
