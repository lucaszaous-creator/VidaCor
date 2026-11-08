//! Lote de guias SP/SADT no padrão TISS 4.03.00 (componente de comunicação de novembro de 2025,
//! XSD oficial da ANS em `xsd/tiss-4.03.00`). O XML é escrito por este módulo, elemento por elemento,
//! na ordem do XSD, com texto escapado (SEC-INJ-3): nunca há DOCTYPE, entidade nem dado do usuário
//! fora de nó de texto. O arquivo sai em ISO-8859-1, como o próprio padrão.
//!
//! O hash do epílogo é o MD5 do conteúdo de todos os campos da mensagem (sem as marcações), em
//! ISO-8859-1, como descreve o componente de comunicação. **A confirmar com o analista TISS e com a
//! primeira operadora real** (DEC-2026-11-08-65). O lastro guarda também o SHA-256 do arquivo.

use crate::guia::{Centavos, GuiaFaturavel};
use chrono::NaiveDateTime;
use md5::Md5;
use sha2::Sha256;

pub const VERSAO_TISS: &str = "4.03.00";
/// Limite do XSD (`guiaSP-SADT maxOccurs="100"`).
pub const MAX_GUIAS_POR_LOTE: usize = 100;
const NAMESPACE: &str = "http://www.ans.gov.br/padroes/tiss/schemas";

/// Códigos fixos desta entrega (a confirmar com o analista TISS):
/// tabela 22 (TUSS procedimentos), atendimento eletivo (1), tipo de atendimento 23 (exame, que
/// substituiu o 05 na 4.00.00), sem acidente (9), regime ambulatorial (01), redução/acréscimo 1,00.
const TABELA_TUSS: &str = "22";
const CARATER_ELETIVO: &str = "1";
const TIPO_ATENDIMENTO_EXAME: &str = "23";
const SEM_ACIDENTE: &str = "9";
const REGIME_AMBULATORIAL: &str = "01";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ErroTiss {
    #[error("O lote não tem guias.")]
    LoteVazio,
    #[error("O lote passa de {MAX_GUIAS_POR_LOTE} guias, o limite do TISS.")]
    LoteGrande,
    #[error("A guia {0} não está pronta para o TISS (falta {1}).")]
    GuiaIncompleta(String, &'static str),
    #[error("A guia {0} é de outra operadora.")]
    OperadoraDiferente(String),
}

/// Cabeçalho do lote: quem envia, para quem e o número atômico por operadora.
#[derive(Debug, Clone)]
pub struct LoteTiss<'a> {
    pub numero: i64,
    pub registro_ans: &'a str,
    pub codigo_prestador: &'a str,
    /// Nome do prestador (contratado solicitante, `st_texto70`).
    pub nome_prestador: &'a str,
    /// Data e hora local da geração.
    pub gerado_em: NaiveDateTime,
    pub guias: &'a [GuiaFaturavel],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArquivoTiss {
    /// Bytes do arquivo em ISO-8859-1.
    pub xml: Vec<u8>,
    /// Hash do epílogo (MD5, hexadecimal minúsculo).
    pub hash_tiss: String,
    /// SHA-256 do arquivo inteiro (lastro, docs/21).
    pub sha256: String,
    pub valor_total: Centavos,
}

/// Nó do documento: elemento com filhos ou elemento só com texto.
enum No {
    El(&'static str, Vec<No>),
    Tx(&'static str, String),
}

fn el(nome: &'static str, filhos: Vec<No>) -> No {
    No::El(nome, filhos)
}

/// Texto limpo para o padrão: sem caracteres de controle, só ISO-8859-1 (o resto vira `?`), cortado
/// no tamanho do campo.
fn tx(nome: &'static str, valor: &str, max: usize) -> No {
    let limpo: String = valor
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if u32::from(c) <= 0xFF { c } else { '?' })
        .take(max)
        .collect();
    No::Tx(nome, limpo.trim().to_owned())
}

fn escapar(s: &str, saida: &mut String) {
    for c in s.chars() {
        match c {
            '&' => saida.push_str("&amp;"),
            '<' => saida.push_str("&lt;"),
            '>' => saida.push_str("&gt;"),
            '"' => saida.push_str("&quot;"),
            '\'' => saida.push_str("&apos;"),
            outro => saida.push(outro),
        }
    }
}

fn escrever(no: &No, saida: &mut String) {
    match no {
        No::El(nome, filhos) => {
            saida.push_str("<ans:");
            saida.push_str(nome);
            saida.push('>');
            for f in filhos {
                escrever(f, saida);
            }
            saida.push_str("</ans:");
            saida.push_str(nome);
            saida.push('>');
        }
        No::Tx(nome, valor) => {
            saida.push_str("<ans:");
            saida.push_str(nome);
            saida.push('>');
            escapar(valor, saida);
            saida.push_str("</ans:");
            saida.push_str(nome);
            saida.push('>');
        }
    }
}

fn conteudo(no: &No, saida: &mut String) {
    match no {
        No::El(_, filhos) => filhos.iter().for_each(|f| conteudo(f, saida)),
        No::Tx(_, valor) => saida.push_str(valor),
    }
}

fn latin1(s: &str) -> Vec<u8> {
    // Todos os textos passaram por `tx`; marcações e escapes são ASCII.
    s.chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn guia_sp_sadt(g: &GuiaFaturavel, lote: &LoteTiss<'_>) -> Result<No, ErroTiss> {
    let falta = |o: &'static str| ErroTiss::GuiaIncompleta(g.numero_guia_prestador.clone(), o);
    if g.registro_ans.as_deref() != Some(lote.registro_ans) {
        return Err(ErroTiss::OperadoraDiferente(
            g.numero_guia_prestador.clone(),
        ));
    }
    let cnes = g.cnes.as_deref().ok_or_else(|| falta("CNES"))?;
    let s = &g.solicitante;
    if !s.completo() {
        return Err(falta("dados do solicitante"));
    }
    let campo = |v: &Option<String>| v.clone().unwrap_or_default();
    if g.executados.is_empty() {
        return Err(falta("procedimentos"));
    }
    let mut itens = Vec::new();
    for (i, e) in g.executados.iter().enumerate() {
        let unitario = e.valor_unitario.ok_or_else(|| falta("preço da tabela"))?;
        itens.push(el(
            "procedimentoExecutado",
            vec![
                tx("sequencialItem", &(i + 1).to_string(), 4),
                tx("dataExecucao", &g.data_execucao.to_string(), 10),
                el(
                    "procedimento",
                    vec![
                        tx("codigoTabela", TABELA_TUSS, 2),
                        tx("codigoProcedimento", &e.tuss, 10),
                        tx("descricaoProcedimento", &e.descricao, 150),
                    ],
                ),
                tx("quantidadeExecutada", &e.quantidade.to_string(), 3),
                tx("reducaoAcrescimo", "1.00", 4),
                tx("valorUnitario", &unitario.tiss(), 13),
                tx("valorTotal", &unitario.vezes(e.quantidade).tiss(), 13),
            ],
        ));
    }
    let total = g.valor_total().tiss();
    let mut filhos = vec![
        el(
            "cabecalhoGuia",
            vec![
                tx("registroANS", lote.registro_ans, 6),
                tx("numeroGuiaPrestador", &g.numero_guia_prestador, 20),
            ],
        ),
        el(
            "dadosAutorizacao",
            vec![
                tx("numeroGuiaOperadora", &g.numero_guia_operadora, 20),
                tx("dataAutorizacao", &g.data_autorizacao.to_string(), 10),
                tx("senha", &g.senha, 20),
                tx("dataValidadeSenha", &g.validade_senha.to_string(), 10),
            ],
        ),
        el(
            "dadosBeneficiario",
            vec![
                tx("numeroCarteira", &g.carteirinha_tiss(), 20),
                tx("atendimentoRN", "N", 1),
            ],
        ),
        el(
            "dadosSolicitante",
            vec![
                el(
                    "contratadoSolicitante",
                    vec![tx("codigoPrestadorNaOperadora", lote.codigo_prestador, 14)],
                ),
                tx("nomeContratadoSolicitante", lote.nome_prestador, 70),
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
        el(
            "dadosSolicitacao",
            vec![tx("caraterAtendimento", CARATER_ELETIVO, 1)],
        ),
        el(
            "dadosExecutante",
            vec![
                el(
                    "contratadoExecutante",
                    vec![tx("codigoPrestadorNaOperadora", lote.codigo_prestador, 14)],
                ),
                tx("CNES", cnes, 7),
            ],
        ),
        el(
            "dadosAtendimento",
            vec![
                tx("tipoAtendimento", TIPO_ATENDIMENTO_EXAME, 2),
                tx("indicacaoAcidente", SEM_ACIDENTE, 1),
                tx("regimeAtendimento", REGIME_AMBULATORIAL, 2),
            ],
        ),
        el("procedimentosExecutados", itens),
    ];
    // A SP/SADT de execução não tem campo de diagnóstico no 4.03.00: o CID exigido por algumas
    // operadoras vai na observação (a confirmar com cada manual).
    if let Some(cid) = g.cid.as_deref() {
        filhos.push(tx("observacao", &format!("CID-10 {cid}"), 500));
    }
    filhos.push(el(
        "valorTotal",
        vec![
            tx("valorProcedimentos", &total, 13),
            tx("valorTotalGeral", &total, 13),
        ],
    ));
    Ok(el("guiaSP-SADT", filhos))
}

/// Monta o arquivo do lote. Confere de novo o que o XSD exige (defesa em profundidade: a auditoria
/// já deixou de fora as guias com pendência).
///
/// # Errors
/// Lote vazio, acima do limite, guia de outra operadora ou incompleta.
pub fn montar_lote(lote: &LoteTiss<'_>) -> Result<ArquivoTiss, ErroTiss> {
    if lote.guias.is_empty() {
        return Err(ErroTiss::LoteVazio);
    }
    if lote.guias.len() > MAX_GUIAS_POR_LOTE {
        return Err(ErroTiss::LoteGrande);
    }
    let guias = lote
        .guias
        .iter()
        .map(|g| guia_sp_sadt(g, lote))
        .collect::<Result<Vec<_>, _>>()?;
    let numero = lote.numero.to_string();
    let cabecalho = el(
        "cabecalho",
        vec![
            el(
                "identificacaoTransacao",
                vec![
                    tx("tipoTransacao", "ENVIO_LOTE_GUIAS", 30),
                    tx("sequencialTransacao", &numero, 12),
                    tx(
                        "dataRegistroTransacao",
                        &lote.gerado_em.date().to_string(),
                        10,
                    ),
                    tx(
                        "horaRegistroTransacao",
                        &lote.gerado_em.format("%H:%M:%S").to_string(),
                        8,
                    ),
                ],
            ),
            el(
                "origem",
                vec![el(
                    "identificacaoPrestador",
                    vec![tx("codigoPrestadorNaOperadora", lote.codigo_prestador, 14)],
                )],
            ),
            el("destino", vec![tx("registroANS", lote.registro_ans, 6)]),
            tx("Padrao", VERSAO_TISS, 8),
        ],
    );
    let corpo = el(
        "prestadorParaOperadora",
        vec![el(
            "loteGuias",
            vec![tx("numeroLote", &numero, 12), el("guiasTISS", guias)],
        )],
    );
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
    let valor_total = Centavos(lote.guias.iter().map(|g| g.valor_total().0).sum());
    Ok(ArquivoTiss {
        xml: bytes,
        hash_tiss,
        sha256,
        valor_total,
    })
}
