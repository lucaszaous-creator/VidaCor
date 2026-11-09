//! Documentos do atendimento (Preparo-Exame.dc.html e Documentos.dc.html): comprovante do
//! agendamento com o preparo, declaração de comparecimento e recibo do caixa. Dados já autorizados e
//! montados pelo núcleo; aqui só a composição (G5).

use crate::motor::{self, Momento, Pdf};
use crate::{Clinica, ErroDocumento};
use serde_json::json;

/// Versões dos modelos; mudam a cada alteração do arquivo `.typ` correspondente.
pub const MODELO_COMPROVANTE: &str = "comprovante-preparo-1";
pub const MODELO_DECLARACAO: &str = "declaracao-comparecimento-1";
pub const MODELO_RECIBO: &str = "recibo-1";
const FONTE_COMPROVANTE: &str = include_str!("../modelos/comprovante.typ");
const FONTE_DECLARACAO: &str = include_str!("../modelos/declaracao.typ");
const FONTE_RECIBO: &str = include_str!("../modelos/recibo.typ");

/// Quem emitiu, quando e com qual código (comum aos três documentos).
#[derive(Debug, Clone)]
pub struct EmissaoImpressa {
    pub codigo: String,
    /// Ex.: "06/11/2026 às 07:58".
    pub em: String,
    /// Nome de quem emitiu.
    pub por: String,
    pub momento: Momento,
}

#[derive(Debug, Clone)]
pub struct PreparoImpresso {
    pub versao: i32,
    /// Ex.: "06/11/2026".
    pub vigente_desde: String,
    pub passos: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ComprovanteAgendamento {
    pub clinica: Clinica,
    pub emissao: EmissaoImpressa,
    pub agendamento_codigo: String,
    pub paciente_nome: String,
    pub paciente_nascimento: String,
    pub exame: String,
    /// Ex.: "Quinta-feira".
    pub dia_semana: String,
    /// Ex.: "12/11/2026".
    pub data: String,
    /// Ex.: "08:30".
    pub hora: String,
    /// Ex.: "08:15".
    pub chegada: String,
    pub antecedencia_min: i32,
    /// Ex.: "Cerca de 30 min, Sala 4".
    pub duracao: String,
    /// Ex.: "Unimed, guia autorizada" ou "Particular".
    pub convenio: String,
    /// Sem preparo cadastrado: o documento diz isso, sem inventar orientação.
    pub preparo: Option<PreparoImpresso>,
}

#[derive(Debug, Clone)]
pub struct DeclaracaoComparecimento {
    pub clinica: Clinica,
    pub emissao: EmissaoImpressa,
    pub paciente_nome: String,
    pub paciente_nascimento: String,
    /// CPF já mascarado (ex.: "***.456.789-**"); ausente não é impresso.
    pub paciente_cpf: Option<String>,
    /// Ex.: "06/11/2026".
    pub dia: String,
    /// Hora registrada no check-in (ex.: "07:52").
    pub chegada: String,
    pub exame: String,
}

#[derive(Debug, Clone)]
pub struct ItemDoRecibo {
    pub descricao: String,
    pub valor_centavos: i64,
}

#[derive(Debug, Clone)]
pub struct Recibo {
    pub clinica: Clinica,
    pub emissao: EmissaoImpressa,
    pub recebimento_codigo: String,
    pub pagador_nome: String,
    pub pagador_cpf: Option<String>,
    pub itens: Vec<ItemDoRecibo>,
    pub desconto_centavos: i64,
    pub total_centavos: i64,
    /// Ex.: "06/11/2026".
    pub pago_em: String,
    /// pix, dinheiro ou cartao.
    pub forma: String,
    pub nfse: Option<String>,
    pub recebido_por: String,
}

fn emissao(e: &EmissaoImpressa, modelo: &str) -> serde_json::Value {
    json!({ "codigo": e.codigo, "em": e.em, "por": e.por, "modelo": modelo })
}

fn clinica(c: &Clinica) -> serde_json::Value {
    json!({ "nome": c.nome, "linhas": c.linhas() })
}

/// Valor em reais no formato brasileiro (ex.: 18000 → "R$ 180,00"; 123456789 → "R$ 1.234.567,89").
#[must_use]
pub fn reais(centavos: i64) -> String {
    let sinal = if centavos < 0 { "-" } else { "" };
    let c = centavos.unsigned_abs();
    let inteiro = (c / 100).to_string();
    let mut agrupado = String::new();
    for (i, ch) in inteiro.chars().enumerate() {
        if i > 0 && (inteiro.len() - i).is_multiple_of(3) {
            agrupado.push('.');
        }
        agrupado.push(ch);
    }
    format!("{sinal}R$ {agrupado},{:02}", c % 100)
}

const UNIDADES: [&str; 20] = [
    "zero",
    "um",
    "dois",
    "três",
    "quatro",
    "cinco",
    "seis",
    "sete",
    "oito",
    "nove",
    "dez",
    "onze",
    "doze",
    "treze",
    "quatorze",
    "quinze",
    "dezesseis",
    "dezessete",
    "dezoito",
    "dezenove",
];
const DEZENAS: [&str; 10] = [
    "",
    "",
    "vinte",
    "trinta",
    "quarenta",
    "cinquenta",
    "sessenta",
    "setenta",
    "oitenta",
    "noventa",
];
const CENTENAS: [&str; 10] = [
    "",
    "cento",
    "duzentos",
    "trezentos",
    "quatrocentos",
    "quinhentos",
    "seiscentos",
    "setecentos",
    "oitocentos",
    "novecentos",
];

/// Número de 0 a 999 por extenso.
fn ate_mil(n: u64) -> String {
    let n = usize::try_from(n % 1000).unwrap_or(0);
    if n == 100 {
        return "cem".into();
    }
    let (c, resto) = (n / 100, n % 100);
    let mut partes: Vec<String> = Vec::new();
    if c > 0 {
        partes.push(CENTENAS[c].into());
    }
    if resto > 0 || c == 0 {
        if resto < 20 {
            partes.push(UNIDADES[resto].into());
        } else {
            let (d, u) = (resto / 10, resto % 10);
            partes.push(if u == 0 {
                DEZENAS[d].into()
            } else {
                format!("{} e {}", DEZENAS[d], UNIDADES[u])
            });
        }
    }
    partes.join(" e ")
}

/// Número inteiro (até 999.999.999) por extenso.
fn inteiro_por_extenso(n: u64) -> String {
    if n == 0 {
        return "zero".into();
    }
    let milhoes = n / 1_000_000;
    let milhares = (n / 1000) % 1000;
    let resto = n % 1000;
    let mut grupos: Vec<String> = Vec::new();
    if milhoes > 0 {
        grupos.push(if milhoes == 1 {
            "um milhão".into()
        } else {
            format!("{} milhões", ate_mil(milhoes))
        });
    }
    if milhares > 0 {
        grupos.push(if milhares == 1 {
            "mil".into()
        } else {
            format!("{} mil", ate_mil(milhares))
        });
    }
    if resto > 0 {
        grupos.push(ate_mil(resto));
    }
    // "e" antes do último grupo quando ele é menor que 100 ou centena redonda (ex.: mil e cem).
    if grupos.len() > 1 && resto > 0 && (resto < 100 || resto.is_multiple_of(100)) {
        let ultimo = grupos.pop().unwrap_or_default();
        return format!("{} e {ultimo}", grupos.join(" "));
    }
    grupos.join(" ")
}

/// Valor em reais por extenso (ex.: 18000 → "cento e oitenta reais"; 150 → "um real e cinquenta
/// centavos"). Só valores positivos e até 999.999.999,99; fora disso, `None`.
#[must_use]
pub fn por_extenso(centavos: i64) -> Option<String> {
    if !(1..=99_999_999_999).contains(&centavos) {
        return None;
    }
    let c = centavos.unsigned_abs();
    let (reais, cents) = (c / 100, c % 100);
    let mut partes: Vec<String> = Vec::new();
    if reais > 0 {
        let de = if reais >= 1_000_000 && reais % 1_000_000 == 0 {
            " de"
        } else {
            ""
        };
        let moeda = if reais == 1 { "real" } else { "reais" };
        partes.push(format!("{}{de} {moeda}", inteiro_por_extenso(reais)));
    }
    if cents > 0 {
        let nome = if cents == 1 { "centavo" } else { "centavos" };
        partes.push(format!("{} {nome}", ate_mil(cents)));
    }
    Some(partes.join(" e "))
}

fn forma(f: &str) -> &str {
    match f {
        "pix" => "PIX",
        "dinheiro" => "Dinheiro",
        "cartao" => "Cartão",
        outro => outro,
    }
}

fn dados_comprovante(c: &ComprovanteAgendamento) -> serde_json::Value {
    json!({
        "clinica": clinica(&c.clinica),
        "emissao": emissao(&c.emissao, MODELO_COMPROVANTE),
        "agendamento": c.agendamento_codigo,
        "paciente": { "nome": c.paciente_nome, "nascimento": c.paciente_nascimento },
        "exame": c.exame, "dia_semana": c.dia_semana, "data": c.data, "hora": c.hora,
        "chegada": c.chegada, "antecedencia": format!("{} min antes", c.antecedencia_min),
        "duracao": c.duracao, "convenio": c.convenio,
        "passos": c.preparo.as_ref().map(|p| p.passos.clone()).unwrap_or_default(),
        "preparo_versao": c.preparo.as_ref().map(|p| p.versao.to_string()).unwrap_or_default(),
        "preparo_desde": c.preparo.as_ref().map(|p| p.vigente_desde.clone()).unwrap_or_default(),
    })
}

fn dados_declaracao(d: &DeclaracaoComparecimento) -> serde_json::Value {
    json!({
        "clinica": clinica(&d.clinica),
        "emissao": emissao(&d.emissao, MODELO_DECLARACAO),
        "paciente": {
            "nome": d.paciente_nome, "nascimento": d.paciente_nascimento,
            "cpf": d.paciente_cpf.clone().unwrap_or_default(),
        },
        "dia": d.dia, "chegada": d.chegada, "exame": d.exame,
    })
}

fn dados_recibo(r: &Recibo) -> Result<serde_json::Value, ErroDocumento> {
    let extenso = por_extenso(r.total_centavos)
        .ok_or_else(|| ErroDocumento::Dados("valor do recibo fora do intervalo".into()))?;
    Ok(json!({
        "clinica": clinica(&r.clinica),
        "emissao": emissao(&r.emissao, MODELO_RECIBO),
        "recebimento": r.recebimento_codigo,
        "pagador": { "nome": r.pagador_nome, "cpf": r.pagador_cpf.clone().unwrap_or_default() },
        "itens": r.itens.iter().map(|i| json!({ "descricao": i.descricao, "valor": reais(i.valor_centavos) })).collect::<Vec<_>>(),
        "desconto": if r.desconto_centavos > 0 { format!("- {}", reais(r.desconto_centavos)) } else { String::new() },
        "valor": reais(r.total_centavos),
        "extenso": extenso,
        "pago_em": r.pago_em, "forma": forma(&r.forma),
        "nfse": r.nfse.clone().unwrap_or_else(|| "não emitida".into()),
        "recebido_por": r.recebido_por,
    }))
}

/// PDF do comprovante do agendamento com o preparo.
///
/// # Errors
/// Defeito do modelo ou dado impossível de serializar.
pub fn comprovante_pdf(c: &ComprovanteAgendamento) -> Result<Pdf, ErroDocumento> {
    let doc = motor::compilar_documento(FONTE_COMPROVANTE, &dados_comprovante(c))?;
    motor::exportar(
        &doc,
        MODELO_COMPROVANTE,
        &c.emissao.codigo,
        c.emissao.momento,
    )
}

/// PDF da declaração de comparecimento.
///
/// # Errors
/// Defeito do modelo ou dado impossível de serializar.
pub fn declaracao_pdf(d: &DeclaracaoComparecimento) -> Result<Pdf, ErroDocumento> {
    let doc = motor::compilar_documento(FONTE_DECLARACAO, &dados_declaracao(d))?;
    motor::exportar(
        &doc,
        MODELO_DECLARACAO,
        &d.emissao.codigo,
        d.emissao.momento,
    )
}

/// PDF do recibo.
///
/// # Errors
/// Defeito do modelo, valor fora do intervalo ou dado impossível de serializar.
pub fn recibo_pdf(r: &Recibo) -> Result<Pdf, ErroDocumento> {
    let doc = motor::compilar_documento(FONTE_RECIBO, &dados_recibo(r)?)?;
    motor::exportar(&doc, MODELO_RECIBO, &r.emissao.codigo, r.emissao.momento)
}

#[cfg(test)]
mod testes {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn clinica_teste() -> Clinica {
        Clinica {
            nome: "VidaCor Imagem".into(),
            endereco: Some("Rua Exemplo, 100".into()),
            telefone: None,
            site: None,
            cnpj: None,
            responsavel_tecnico: None,
        }
    }

    fn emissao_teste(codigo: &str) -> EmissaoImpressa {
        EmissaoImpressa {
            codigo: codigo.into(),
            em: "06/11/2026 às 07:58".into(),
            por: "Ana Souza".into(),
            momento: Momento {
                ano: 2026,
                mes: 11,
                dia: 6,
                hora: 10,
                minuto: 58,
                segundo: 0,
            },
        }
    }

    #[test]
    fn valores_por_extenso() {
        assert_eq!(por_extenso(18000).unwrap(), "cento e oitenta reais");
        assert_eq!(por_extenso(100).unwrap(), "um real");
        assert_eq!(por_extenso(1).unwrap(), "um centavo");
        assert_eq!(por_extenso(150).unwrap(), "um real e cinquenta centavos");
        assert_eq!(por_extenso(10000).unwrap(), "cem reais");
        assert_eq!(por_extenso(110_000).unwrap(), "mil e cem reais");
        assert_eq!(
            por_extenso(123_456).unwrap(),
            "mil duzentos e trinta e quatro reais e cinquenta e seis centavos"
        );
        assert_eq!(por_extenso(200_000_000).unwrap(), "dois milhões de reais");
        assert_eq!(por_extenso(0), None);
        assert_eq!(por_extenso(-5), None);
        assert_eq!(reais(18000), "R$ 180,00");
        assert_eq!(reais(123_456_789), "R$ 1.234.567,89");
        assert_eq!(reais(5), "R$ 0,05");
    }

    #[test]
    fn comprovante_com_e_sem_preparo() {
        let mut c = ComprovanteAgendamento {
            clinica: clinica_teste(),
            emissao: emissao_teste("CPV-2026-000001"),
            agendamento_codigo: "AGD-2026-000123".into(),
            paciente_nome: "Maria Helena Duarte".into(),
            paciente_nascimento: "12/03/1968".into(),
            exame: "Teste ergométrico".into(),
            dia_semana: "Quinta-feira".into(),
            data: "12/11/2026".into(),
            hora: "08:30".into(),
            chegada: "08:15".into(),
            antecedencia_min: 15,
            duracao: "Cerca de 60 min, Sala 4".into(),
            convenio: "Unimed, guia autorizada".into(),
            preparo: Some(PreparoImpresso {
                versao: 2,
                vigente_desde: "01/11/2026".into(),
                passos: vec![
                    "Venha com roupa leve e tênis.".into(),
                    "#set text(red) [x]".into(),
                ],
            }),
        };
        let pdf = comprovante_pdf(&c).unwrap();
        assert!(pdf.bytes.starts_with(b"%PDF"));
        let texto = motor::texto(
            &motor::compilar_documento(FONTE_COMPROVANTE, &dados_comprovante(&c)).unwrap(),
        );
        assert!(texto.contains("Maria Helena Duarte") && texto.contains("12/03/1968"));
        assert!(
            texto.contains("#set text(red)"),
            "passo entra como texto, nunca como marcação"
        );
        c.preparo = None;
        let texto = motor::texto(
            &motor::compilar_documento(FONTE_COMPROVANTE, &dados_comprovante(&c)).unwrap(),
        );
        assert!(texto.contains("Não há orientação de preparo cadastrada"));
    }

    #[test]
    fn declaracao_e_recibo() {
        let d = DeclaracaoComparecimento {
            clinica: clinica_teste(),
            emissao: emissao_teste("DCL-2026-000001"),
            paciente_nome: "Maria Helena Duarte".into(),
            paciente_nascimento: "12/03/1968".into(),
            paciente_cpf: Some("***.456.789-**".into()),
            dia: "06/11/2026".into(),
            chegada: "07:52".into(),
            exame: "Ecocardiograma transtorácico".into(),
        };
        let texto = motor::texto(
            &motor::compilar_documento(FONTE_DECLARACAO, &dados_declaracao(&d)).unwrap(),
        );
        assert!(texto.contains("07:52") && texto.contains("não substitui atestado médico"));
        assert!(declaracao_pdf(&d).unwrap().bytes.starts_with(b"%PDF"));
        let r = Recibo {
            clinica: clinica_teste(),
            emissao: emissao_teste("RCB-2026-000001"),
            recebimento_codigo: "REC-2026-004281".into(),
            pagador_nome: "Rafael Monteiro Lima".into(),
            pagador_cpf: None,
            itens: vec![ItemDoRecibo {
                descricao: "Raio X de tórax".into(),
                valor_centavos: 20000,
            }],
            desconto_centavos: 2000,
            total_centavos: 18000,
            pago_em: "06/11/2026".into(),
            forma: "pix".into(),
            nfse: None,
            recebido_por: "Carla Mendes".into(),
        };
        let texto = motor::texto(
            &motor::compilar_documento(FONTE_RECIBO, &dados_recibo(&r).unwrap()).unwrap(),
        );
        assert!(texto.contains("cento e oitenta reais") && texto.contains("R$ 180,00"));
        assert!(texto.contains("não emitida"));
        assert!(recibo_pdf(&r).unwrap().bytes.starts_with(b"%PDF"));
    }
}
