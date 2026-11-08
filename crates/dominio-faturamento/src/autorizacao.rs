//! Autorização pedida pelo sistema (docs/22 E4, docs/25 § 3.3): solicitação de procedimento SP/SADT
//! (`SOLICITACAO_PROCEDIMENTOS`) no padrão TISS 4.03.00 e leitura da resposta `autorizacaoServico`
//! da operadora (autorizada, parcial, negada, em análise, pendente de documento, cancelada) ou da
//! mensagem de erro. Sem I/O (G5).
//!
//! A leitura não aceita DTD nem entidade (SEC-INJ-3: o leitor recusa o documento), limita tamanho e
//! número de nós e decide a situação pelas quantidades autorizadas (G1: "autorizado" com quantidade
//! menor que a pedida é parcial). O hash do epílogo é conferido e informado, sem bloquear (o cálculo
//! segue a mesma leitura do lote, a confirmar com a primeira operadora real).

use crate::guia::Profissional;
use crate::tiss::{
    conteudo, el, escrever, hex, latin1, tx, ErroTiss, No, CARATER_ELETIVO, NAMESPACE, TABELA_TUSS,
    VERSAO_TISS,
};
use chrono::{NaiveDate, NaiveDateTime};
use md5::Md5;
use sha2::Sha256;

/// Maior resposta aceita (SEC-DIS: limite de tamanho) e maior número de nós.
pub const MAX_RESPOSTA_BYTES: usize = 512 * 1024;
const MAX_NOS: u32 = 20_000;
/// Procedimentos por pedido (limite prático; o XSD não limita).
pub const MAX_PROCEDIMENTOS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcedimentoSolicitado {
    pub tuss: String,
    pub descricao: String,
    pub quantidade: u16,
}

/// Pedido de autorização prévia (etapa 1), montado a partir do agendamento.
#[derive(Debug, Clone)]
pub struct PedidoAutorizacao<'a> {
    /// Número no prestador (o código do pedido, ex.: `AUT-2026-000012`).
    pub numero_guia_prestador: &'a str,
    pub sequencial: i64,
    pub registro_ans: &'a str,
    pub codigo_prestador: &'a str,
    pub nome_prestador: &'a str,
    pub cnes: &'a str,
    pub carteirinha: &'a str,
    pub solicitante: &'a Profissional,
    pub data_solicitacao: NaiveDate,
    pub indicacao_clinica: Option<&'a str>,
    pub procedimentos: &'a [ProcedimentoSolicitado],
    pub gerado_em: NaiveDateTime,
}

/// Mensagem TISS pronta para enviar: bytes em ISO-8859-1, hash do epílogo e SHA-256.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MensagemTiss {
    pub xml: Vec<u8>,
    pub hash_tiss: String,
    pub sha256: String,
}

fn so_alfanumerico(s: &str) -> String {
    s.chars().filter(char::is_ascii_alphanumeric).collect()
}

fn mensagem(cabecalho: No, corpo: No) -> MensagemTiss {
    let mut campos = String::new();
    conteudo(&cabecalho, &mut campos);
    conteudo(&corpo, &mut campos);
    let hash_tiss = hex(&<Md5 as md5::Digest>::digest(latin1(&campos)));
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>");
    xml.push_str("<ans:mensagemTISS xmlns:ans=\"");
    xml.push_str(NAMESPACE);
    xml.push_str("\">");
    escrever(&cabecalho, &mut xml);
    escrever(&corpo, &mut xml);
    escrever(&el("epilogo", vec![tx("hash", &hash_tiss, 32)]), &mut xml);
    xml.push_str("</ans:mensagemTISS>");
    let bytes = latin1(&xml);
    let sha256 = hex(&<Sha256 as sha2::Digest>::digest(&bytes));
    MensagemTiss {
        xml: bytes,
        hash_tiss,
        sha256,
    }
}

fn cabecalho(tipo: &str, sequencial: i64, quando: NaiveDateTime, origem: No, destino: No) -> No {
    el(
        "cabecalho",
        vec![
            el(
                "identificacaoTransacao",
                vec![
                    tx("tipoTransacao", tipo, 40),
                    tx("sequencialTransacao", &sequencial.to_string(), 12),
                    tx("dataRegistroTransacao", &quando.date().to_string(), 10),
                    tx(
                        "horaRegistroTransacao",
                        &quando.format("%H:%M:%S").to_string(),
                        8,
                    ),
                ],
            ),
            el("origem", vec![origem]),
            el("destino", vec![destino]),
            tx("Padrao", VERSAO_TISS, 8),
        ],
    )
}

fn procedimento(tuss: &str, descricao: &str) -> No {
    el(
        "procedimento",
        vec![
            tx("codigoTabela", TABELA_TUSS, 2),
            tx("codigoProcedimento", tuss, 10),
            tx("descricaoProcedimento", descricao, 150),
        ],
    )
}

/// Monta a solicitação de procedimento SP/SADT.
///
/// # Errors
/// Sem procedimento, acima do limite, solicitante incompleto ou carteirinha vazia.
pub fn montar_solicitacao(p: &PedidoAutorizacao<'_>) -> Result<MensagemTiss, ErroTiss> {
    let falta = |o: &'static str| ErroTiss::GuiaIncompleta(p.numero_guia_prestador.to_owned(), o);
    if p.procedimentos.is_empty() || p.procedimentos.len() > MAX_PROCEDIMENTOS {
        return Err(falta("procedimentos"));
    }
    if !p.solicitante.completo() {
        return Err(falta("dados do solicitante"));
    }
    let carteira = so_alfanumerico(p.carteirinha);
    if carteira.is_empty() || carteira.len() > 20 {
        return Err(falta("carteirinha"));
    }
    let s = p.solicitante;
    let campo = |v: &Option<String>| v.clone().unwrap_or_default();
    let mut filhos = vec![
        el(
            "cabecalhoSolicitacao",
            vec![
                tx("registroANS", p.registro_ans, 6),
                tx("numeroGuiaPrestador", p.numero_guia_prestador, 20),
            ],
        ),
        tx("tipoEtapaAutorizacao", "1", 1),
        el(
            "dadosBeneficiario",
            vec![
                tx("numeroCarteira", &carteira, 20),
                tx("atendimentoRN", "N", 1),
            ],
        ),
        el(
            "dadosSolicitante",
            vec![
                el(
                    "contratadoSolicitante",
                    vec![tx("codigoPrestadorNaOperadora", p.codigo_prestador, 14)],
                ),
                tx("nomeContratadoSolicitante", p.nome_prestador, 70),
                el(
                    "profissionalSolicitante",
                    vec![
                        tx("nomeProfissional", &s.nome, 70),
                        tx("conselhoProfissional", &campo(&s.conselho), 2),
                        tx("numeroConselhoProfissional", &campo(&s.numero), 15),
                        tx("UF", &campo(&s.uf), 2),
                        tx("CBOS", &campo(&s.cbos), 6),
                    ],
                ),
            ],
        ),
        tx("caraterAtendimento", CARATER_ELETIVO, 1),
        tx("dataSolicitacao", &p.data_solicitacao.to_string(), 10),
    ];
    if let Some(i) = p.indicacao_clinica.map(str::trim).filter(|i| !i.is_empty()) {
        filhos.push(tx("indicacaoClinica", i, 500));
    }
    for x in p.procedimentos {
        filhos.push(el(
            "procedimentosSolicitados",
            vec![
                procedimento(&x.tuss, &x.descricao),
                tx("quantidadeSolicitada", &x.quantidade.to_string(), 3),
            ],
        ));
    }
    filhos.push(el(
        "dadosExecutante",
        vec![
            tx("codigonaOperadora", p.codigo_prestador, 14),
            tx("CNES", p.cnes, 7),
        ],
    ));
    let cab = cabecalho(
        "SOLICITACAO_PROCEDIMENTOS",
        p.sequencial,
        p.gerado_em,
        el(
            "identificacaoPrestador",
            vec![tx("codigoPrestadorNaOperadora", p.codigo_prestador, 14)],
        ),
        tx("registroANS", p.registro_ans, 6),
    );
    let corpo = el(
        "prestadorParaOperadora",
        vec![el(
            "solicitacaoProcedimento",
            vec![el("solicitacaoSP-SADT", filhos)],
        )],
    );
    Ok(mensagem(cab, corpo))
}

/// Situação da solicitação na operadora (`dm_statusSolicitacao`) ou mensagem recusada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusAutorizacao {
    Autorizada,
    EmAnalise,
    Negada,
    PendenteDocumento,
    Cancelada,
    Parcial,
    /// A operadora recusou a mensagem (`mensagemErro`): o pedido precisa ser corrigido e refeito.
    Recusada,
}

impl StatusAutorizacao {
    #[must_use]
    pub fn codigo(self) -> &'static str {
        match self {
            Self::Autorizada => "autorizada",
            Self::EmAnalise => "em_analise",
            Self::Negada => "negada",
            Self::PendenteDocumento => "pendente_documento",
            Self::Cancelada => "cancelada",
            Self::Parcial => "parcial",
            Self::Recusada => "recusada",
        }
    }

    /// Resposta final: a operadora não vai mudar sem pedido novo.
    #[must_use]
    pub fn final_(self) -> bool {
        !matches!(self, Self::EmAnalise | Self::PendenteDocumento)
    }

    fn do_tiss(codigo: &str) -> Option<Self> {
        Some(match codigo {
            "1" => Self::Autorizada,
            "2" => Self::EmAnalise,
            "3" => Self::Negada,
            "4" | "5" => Self::PendenteDocumento,
            "6" => Self::Cancelada,
            "7" => Self::Parcial,
            _ => return None,
        })
    }

    fn para_tiss(self) -> &'static str {
        match self {
            Self::Autorizada => "1",
            Self::EmAnalise => "2",
            Self::Negada | Self::Recusada => "3",
            Self::PendenteDocumento => "5",
            Self::Cancelada => "6",
            Self::Parcial => "7",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcedimentoAutorizado {
    pub tuss: String,
    pub solicitado: u16,
    pub autorizado: u16,
}

/// Resposta lida da operadora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RespostaAutorizacao {
    pub numero_guia_prestador: Option<String>,
    pub status: StatusAutorizacao,
    pub numero_guia_operadora: Option<String>,
    pub senha: Option<String>,
    pub data_autorizacao: Option<NaiveDate>,
    pub validade: Option<NaiveDate>,
    pub procedimentos: Vec<ProcedimentoAutorizado>,
    /// Códigos de glosa/negativa da tabela TISS, ou o código da mensagem de erro.
    pub motivos: Vec<String>,
    pub observacao: Option<String>,
    /// O hash do epílogo confere com o conteúdo.
    pub hash_confere: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ErroResposta {
    #[error("A resposta da operadora passa do tamanho aceito.")]
    Grande,
    #[error("A resposta da operadora não é um XML TISS legível: {0}")]
    Ilegivel(String),
    #[error("A resposta da operadora não é uma resposta de autorização.")]
    NaoEhAutorizacao,
    #[error("A resposta diz autorizado, mas falta {0}.")]
    Incompleta(&'static str),
}

fn texto_de(bytes: &[u8]) -> Result<String, ErroResposta> {
    let inicio: String = bytes
        .iter()
        .take(120)
        .map(|b| char::from(*b))
        .collect::<String>()
        .to_ascii_lowercase();
    if inicio.contains("iso-8859-1") || inicio.contains("latin1") {
        Ok(bytes.iter().map(|b| char::from(*b)).collect())
    } else {
        String::from_utf8(bytes.to_vec())
            .map_err(|_| ErroResposta::Ilegivel("codificação desconhecida".into()))
    }
}

type Nodo<'a, 'i> = roxmltree::Node<'a, 'i>;

fn filho<'a, 'i>(n: Nodo<'a, 'i>, nome: &str) -> Option<Nodo<'a, 'i>> {
    n.children().find(|c| {
        c.is_element() && c.tag_name().name() == nome && c.tag_name().namespace() == Some(NAMESPACE)
    })
}

fn caminho<'a, 'i>(n: Nodo<'a, 'i>, nomes: &[&str]) -> Option<Nodo<'a, 'i>> {
    nomes.iter().try_fold(n, |atual, nome| filho(atual, nome))
}

fn valor(n: Nodo<'_, '_>, nomes: &[&str]) -> Option<String> {
    caminho(n, nomes)
        .and_then(|x| x.text())
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
}

fn data(n: Nodo<'_, '_>, nomes: &[&str]) -> Option<NaiveDate> {
    valor(n, nomes).and_then(|v| NaiveDate::parse_from_str(&v, "%Y-%m-%d").ok())
}

fn todo_texto(n: Nodo<'_, '_>, saida: &mut String) {
    for d in n.descendants().filter(roxmltree::Node::is_text) {
        let t = d.text().unwrap_or_default();
        // Espaço entre marcações (indentação) não é conteúdo de campo.
        if !t.trim().is_empty() {
            saida.push_str(t.trim());
        }
    }
}

/// Lê a resposta de autorização da operadora.
///
/// # Errors
/// Tamanho, XML ilegível (inclusive com DTD/entidade), mensagem de outro tipo ou autorização sem
/// senha, validade ou procedimento.
pub fn ler_resposta(bytes: &[u8]) -> Result<RespostaAutorizacao, ErroResposta> {
    if bytes.len() > MAX_RESPOSTA_BYTES {
        return Err(ErroResposta::Grande);
    }
    let texto = texto_de(bytes)?;
    let opcoes = roxmltree::ParsingOptions {
        allow_dtd: false,
        nodes_limit: MAX_NOS,
    };
    let doc = roxmltree::Document::parse_with_options(&texto, opcoes)
        .map_err(|e| ErroResposta::Ilegivel(e.to_string()))?;
    let raiz = doc.root_element();
    if raiz.tag_name().name() != "mensagemTISS" || raiz.tag_name().namespace() != Some(NAMESPACE) {
        return Err(ErroResposta::NaoEhAutorizacao);
    }
    let cab = filho(raiz, "cabecalho").ok_or(ErroResposta::NaoEhAutorizacao)?;
    let corpo = filho(raiz, "operadoraParaPrestador").ok_or(ErroResposta::NaoEhAutorizacao)?;
    let situacao = filho(corpo, "autorizacaoServicos")
        .or_else(|| filho(corpo, "situacaoAutorizacao"))
        .ok_or(ErroResposta::NaoEhAutorizacao)?;
    let mut campos = String::new();
    todo_texto(cab, &mut campos);
    todo_texto(corpo, &mut campos);
    let calculado = hex(&<Md5 as md5::Digest>::digest(latin1(&campos)));
    let hash_confere =
        valor(raiz, &["epilogo", "hash"]).is_some_and(|h| h.eq_ignore_ascii_case(&calculado));
    if let Some(erro) = filho(situacao, "mensagemErro") {
        let codigo = valor(erro, &["codigoGlosa"]).unwrap_or_default();
        return Ok(RespostaAutorizacao {
            numero_guia_prestador: None,
            status: StatusAutorizacao::Recusada,
            numero_guia_operadora: None,
            senha: None,
            data_autorizacao: None,
            validade: None,
            procedimentos: Vec::new(),
            motivos: vec![codigo],
            observacao: valor(erro, &["descricaoGlosa"]),
            hash_confere,
        });
    }
    let a = filho(situacao, "autorizacaoServico").ok_or(ErroResposta::NaoEhAutorizacao)?;
    let mut status = valor(a, &["statusSolicitacao"])
        .and_then(|s| StatusAutorizacao::do_tiss(&s))
        .ok_or(ErroResposta::NaoEhAutorizacao)?;
    let mut procedimentos = Vec::new();
    if let Some(lista) = filho(a, "servicosAutorizados") {
        for s in lista
            .children()
            .filter(|c| c.is_element() && c.tag_name().name() == "servicoAutorizado")
        {
            let qtd = |nome: &str| {
                valor(s, &[nome])
                    .and_then(|v| v.parse::<u16>().ok())
                    .unwrap_or(0)
            };
            let tuss = valor(s, &["procedimento", "codigoProcedimento"]).unwrap_or_default();
            if tuss.len() != 8 || !tuss.chars().all(|c| c.is_ascii_digit()) {
                return Err(ErroResposta::Ilegivel(format!(
                    "código TUSS inválido: {tuss}"
                )));
            }
            procedimentos.push(ProcedimentoAutorizado {
                tuss,
                solicitado: qtd("quantidadeSolicitada"),
                autorizado: qtd("quantidadeAutorizada"),
            });
        }
    }
    let motivos: Vec<String> = filho(a, "motivosNegativa")
        .map(|m| {
            m.children()
                .filter(|c| c.is_element())
                .filter_map(|c| c.text().map(|t| t.trim().to_owned()))
                .collect()
        })
        .unwrap_or_default();
    let senha = valor(a, &["dadosAutorizacao", "senha"]);
    let validade = data(a, &["dadosAutorizacao", "dataValidadeSenha"]);
    if matches!(
        status,
        StatusAutorizacao::Autorizada | StatusAutorizacao::Parcial
    ) {
        if senha.is_none() {
            return Err(ErroResposta::Incompleta("a senha"));
        }
        if validade.is_none() {
            return Err(ErroResposta::Incompleta("a validade da senha"));
        }
        if !procedimentos.iter().any(|p| p.autorizado > 0) {
            return Err(ErroResposta::Incompleta("procedimento autorizado"));
        }
        // O servidor decide pela quantidade: autorizado a menos é parcial (G1).
        if procedimentos.iter().any(|p| p.autorizado < p.solicitado) {
            status = StatusAutorizacao::Parcial;
        }
    }
    Ok(RespostaAutorizacao {
        numero_guia_prestador: valor(a, &["dadosAutorizacao", "numeroGuiaPrestador"]),
        status,
        numero_guia_operadora: valor(a, &["dadosAutorizacao", "numeroGuiaOperadora"]),
        senha,
        data_autorizacao: data(a, &["dadosAutorizacao", "dataAutorizacao"]),
        validade,
        procedimentos,
        motivos,
        observacao: valor(a, &["observacao"]),
        hash_confere,
    })
}

/// Dados de uma resposta simulada (operadora de desenvolvimento e testes).
#[derive(Debug, Clone)]
pub struct RespostaSimulada<'a> {
    pub numero_guia_prestador: &'a str,
    pub registro_ans: &'a str,
    pub codigo_prestador: &'a str,
    pub carteirinha: &'a str,
    pub status: StatusAutorizacao,
    pub numero_guia_operadora: Option<&'a str>,
    pub senha: Option<&'a str>,
    pub data_autorizacao: Option<NaiveDate>,
    pub validade: Option<NaiveDate>,
    /// TUSS, quantidade pedida e autorizada.
    pub procedimentos: &'a [(&'a str, u16, u16)],
    pub motivos: &'a [&'a str],
    pub quando: NaiveDateTime,
}

/// Resposta TISS de autorização válida no XSD, como uma operadora mandaria. Só para a operadora de
/// DESENVOLVIMENTO e para os testes: não tem valor de autorização.
#[must_use]
pub fn resposta_simulada(r: &RespostaSimulada<'_>) -> Vec<u8> {
    let mut dados = vec![tx("numeroGuiaPrestador", r.numero_guia_prestador, 20)];
    if let Some(n) = r.numero_guia_operadora {
        dados.push(tx("numeroGuiaOperadora", n, 20));
    }
    if let Some(d) = r.data_autorizacao {
        dados.push(tx("dataAutorizacao", &d.to_string(), 10));
    }
    if let Some(s) = r.senha {
        dados.push(tx("senha", s, 20));
    }
    if let Some(v) = r.validade {
        dados.push(tx("dataValidadeSenha", &v.to_string(), 10));
    }
    let mut a = vec![
        el("dadosAutorizacao", dados),
        tx("tipoEtapaAutorizacao", "1", 1),
        el(
            "dadosBeneficiario",
            vec![
                tx("numeroCarteira", &so_alfanumerico(r.carteirinha), 20),
                tx("atendimentoRN", "N", 1),
            ],
        ),
        tx("nomeBeneficiario", "BENEFICIARIO DE TESTE", 70),
        tx("statusSolicitacao", r.status.para_tiss(), 1),
    ];
    if !r.motivos.is_empty() {
        a.push(el(
            "motivosNegativa",
            r.motivos.iter().map(|m| tx("codigoGlosa", m, 4)).collect(),
        ));
    }
    if !r.procedimentos.is_empty() {
        a.push(el(
            "servicosAutorizados",
            r.procedimentos
                .iter()
                .enumerate()
                .map(|(i, (tuss, pedida, autorizada))| {
                    el(
                        "servicoAutorizado",
                        vec![
                            tx("sequencialItem", &(i + 1).to_string(), 4),
                            procedimento(tuss, "Procedimento"),
                            tx("quantidadeSolicitada", &pedida.to_string(), 3),
                            tx("quantidadeAutorizada", &autorizada.to_string(), 3),
                        ],
                    )
                })
                .collect(),
        ));
    }
    let cab = cabecalho(
        "RESPOSTA_SOLICITACAO",
        1,
        r.quando,
        tx("registroANS", r.registro_ans, 6),
        el(
            "identificacaoPrestador",
            vec![tx("codigoPrestadorNaOperadora", r.codigo_prestador, 14)],
        ),
    );
    let corpo = el(
        "operadoraParaPrestador",
        vec![el("autorizacaoServicos", vec![el("autorizacaoServico", a)])],
    );
    mensagem(cab, corpo).xml
}
