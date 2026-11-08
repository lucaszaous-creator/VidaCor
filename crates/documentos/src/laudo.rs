//! Laudo impresso (Laudo-Impresso.dc.html): só a versão assinada, com dois identificadores do
//! paciente, médico com registro, código do laudo, hash da versão e código de verificação.

use crate::motor::{self, Momento, Pdf};
use crate::{hash_em_grupos, paragrafos, Clinica, ErroDocumento};
use serde::Serialize;
use serde_json::json;

/// Versão do modelo impresso; muda a cada alteração do arquivo `modelos/laudo.typ`.
pub const MODELO_LAUDO: &str = "laudo-impresso-1";
const FONTE: &str = include_str!("../modelos/laudo.typ");

#[derive(Debug, Clone, Serialize)]
pub struct MedicoImpresso {
    pub nome: String,
    /// Ex.: "CRM-SP 123456". Ausente: o PDF diz que o registro não está cadastrado.
    pub registro: Option<String>,
    pub rqe: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Assinatura {
    /// Ex.: "06/11/2026 às 10:42" (horário de Brasília).
    pub em: String,
    /// Identificação do certificado como gravada na assinatura.
    pub certificado: Option<String>,
    /// SHA-256 (hex) do documento canônico da versão assinada.
    pub hash_documento: String,
    /// Momento da assinatura (UTC), gravado como data de criação do PDF.
    #[serde(skip)]
    pub momento: Momento,
}

#[derive(Debug, Clone, Serialize)]
pub struct Retificacao {
    pub versao_anterior: i32,
    pub motivo: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AchadoImpresso {
    pub numero: i32,
    /// Ex.: "ACR TI-RADS (2017)".
    pub classificacao: String,
    pub categoria: String,
    pub localizacao: String,
    pub maior_diametro: Option<String>,
    pub descricao: String,
    pub conclusao: String,
}

#[derive(Debug, Clone)]
pub struct LaudoImpresso {
    pub clinica: Clinica,
    pub codigo: String,
    pub versao: i32,
    pub paciente_nome: String,
    /// "dd/mm/aaaa (NN anos)".
    pub paciente_nascimento: String,
    pub protocolo: String,
    pub exame: String,
    pub data_exame: String,
    pub texto: String,
    pub achados: Vec<AchadoImpresso>,
    pub retificacao: Option<Retificacao>,
    /// A versão impressa já foi substituída por uma retificação assinada (só no portal, enquanto a
    /// nova não é publicada).
    pub substituida: bool,
    pub medico: MedicoImpresso,
    pub assinatura: Assinatura,
}

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Código de verificação impresso: os primeiros 60 bits do hash da versão em base 32 de Crockford
/// (sem I, L, O e U, que confundem na leitura), em três grupos de quatro. Mesmo hash, mesmo código.
#[must_use]
pub fn codigo_verificacao(hash_hex: &str) -> String {
    let mut bits: u64 = 0;
    let mut usados = 0;
    for c in hash_hex.chars() {
        if usados == 15 {
            break;
        }
        if let Some(v) = c.to_digit(16) {
            bits = (bits << 4) | u64::from(v);
            usados += 1;
        }
    }
    bits <<= 4 * (15 - usados);
    let simbolos: Vec<char> = (0..12)
        .rev()
        .map(|i| {
            let indice = usize::try_from((bits >> (i * 5)) & 0x1f).unwrap_or(0);
            char::from(CROCKFORD[indice])
        })
        .collect();
    simbolos
        .chunks(4)
        .map(|g| g.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

fn dados(l: &LaudoImpresso) -> serde_json::Value {
    json!({
        "clinica": { "nome": l.clinica.nome, "linhas": l.clinica.linhas() },
        "laudo": { "codigo": l.codigo, "versao": l.versao.to_string(), "substituida": l.substituida },
        "paciente": { "nome": l.paciente_nome, "nascimento": l.paciente_nascimento },
        "atendimento": { "protocolo": l.protocolo, "data": l.data_exame },
        "exame": l.exame,
        "paragrafos": paragrafos(&l.texto),
        "achados": l.achados.iter().map(|a| json!({
            "numero": a.numero.to_string(),
            "classificacao": a.classificacao,
            "categoria": a.categoria,
            "localizacao": a.localizacao,
            "diametro": a.maior_diametro.clone().unwrap_or_else(|| "não medido".into()),
            "descricao": a.descricao,
            "conclusao": a.conclusao,
        })).collect::<Vec<_>>(),
        "retificacao": l.retificacao.as_ref().map(|r| json!({
            "anterior": r.versao_anterior.to_string(), "motivo": r.motivo,
        })),
        "medico": l.medico,
        "assinatura": {
            "em": l.assinatura.em,
            "certificado": l.assinatura.certificado,
            "hash": hash_em_grupos(&l.assinatura.hash_documento),
            "codigo_verificacao": codigo_verificacao(&l.assinatura.hash_documento),
        },
    })
}

/// Gera o PDF do laudo. Mesma versão assinada, mesmo PDF (bytes e SHA-256 iguais).
///
/// # Errors
/// Defeito do modelo ou dado impossível de serializar.
pub fn laudo_pdf(l: &LaudoImpresso) -> Result<Pdf, ErroDocumento> {
    let documento = motor::compilar_documento(FONTE, &dados(l))?;
    motor::exportar(
        &documento,
        MODELO_LAUDO,
        &format!("{}-v{}", l.codigo, l.versao),
        l.assinatura.momento,
    )
}

#[cfg(test)]
mod testes {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    pub(crate) fn exemplo(nome: &str) -> LaudoImpresso {
        LaudoImpresso {
            clinica: Clinica {
                nome: "VidaCor Imagem".into(),
                endereco: Some("Rua Fictícia, 100, Centro".into()),
                telefone: Some("(00) 0000-0000".into()),
                site: None,
                cnpj: Some("00.000.000/0001-00".into()),
                responsavel_tecnico: Some("Dra. Responsável Fictícia, CRM-XX 000000".into()),
            },
            codigo: "LAU-2026-000001".into(),
            versao: 2,
            paciente_nome: nome.into(),
            paciente_nascimento: "15/06/1954 (72 anos)".into(),
            protocolo: "ATD-2026-000001".into(),
            exame: "Ultrassonografia de tireoide".into(),
            data_exame: "06/11/2026".into(),
            texto:
                "Tireoide de dimensões normais.\nContornos regulares.\n\nSem linfonodos suspeitos."
                    .into(),
            achados: vec![AchadoImpresso {
                numero: 1,
                classificacao: "ACR TI-RADS (2017)".into(),
                categoria: "TR3".into(),
                localizacao: "Lobo direito, terço médio".into(),
                maior_diametro: Some("12 mm".into()),
                descricao: "Nódulo sólido isoecogênico.".into(),
                conclusao: "TR3: acompanhamento.".into(),
            }],
            retificacao: Some(Retificacao {
                versao_anterior: 1,
                motivo: "Medida do nódulo corrigida.".into(),
            }),
            substituida: false,
            medico: MedicoImpresso {
                nome: "Dr. Paulo Medeiros".into(),
                registro: Some("CRM-XX 000000".into()),
                rqe: None,
            },
            assinatura: Assinatura {
                em: "06/11/2026 às 10:42".into(),
                certificado: Some("ICP-Brasil de desenvolvimento (SEM VALOR LEGAL)".into()),
                hash_documento: "ab".repeat(32),
                momento: Momento {
                    ano: 2026,
                    mes: 11,
                    dia: 6,
                    hora: 13,
                    minuto: 42,
                    segundo: 0,
                },
            },
        }
    }

    #[test]
    fn gera_pdf_a_com_hash_e_e_deterministico() {
        let l = exemplo("Antônio Carlos Ribeiro");
        let a = laudo_pdf(&l).unwrap();
        let b = laudo_pdf(&l).unwrap();
        assert!(a.bytes.starts_with(b"%PDF"));
        assert_eq!(a.sha256.len(), 64);
        assert_eq!(a.sha256, b.sha256, "mesma versão, mesmo PDF");
        assert_eq!(a.modelo, MODELO_LAUDO);
    }

    #[test]
    fn texto_impresso_tem_os_dois_identificadores_e_o_codigo_de_verificacao() {
        let l = exemplo("Antônio Carlos Ribeiro");
        let doc = motor::compilar_documento(FONTE, &dados(&l)).unwrap();
        let texto = motor::texto(&doc);
        for esperado in [
            "Ribeiro",
            "15/06/1954",
            "LAU-2026-000001",
            "CRM-XX",
            &codigo_verificacao(&l.assinatura.hash_documento),
        ] {
            assert!(texto.contains(esperado), "{esperado} em {texto}");
        }
        assert!(texto.contains("substitui"), "retificação indicada");
    }

    /// SEC-INJ-4: marcação Typst no nome sai como texto e não executa nada.
    #[test]
    fn marcacao_no_dado_sai_como_texto() {
        let maligno = r#"Ana #panic("injetado") *negrito* $x^2$ [link] _i_ \ @ref <rotulo> // = Título #import "/etc/passwd""#;
        let mut l = exemplo(maligno);
        l.texto = maligno.into();
        l.medico.nome = maligno.into();
        let doc = motor::compilar_documento(FONTE, &dados(&l)).unwrap();
        let texto: String = motor::texto(&doc).split_whitespace().collect();
        for pedaco in [
            "#panic(\"injetado\")",
            "*negrito*",
            "$x^2$",
            "[link]",
            "<rotulo>",
            "#import",
        ] {
            let sem_espaco: String = pedaco.split_whitespace().collect();
            assert!(
                texto.contains(&sem_espaco),
                "{pedaco} impresso literalmente"
            );
        }
        assert!(laudo_pdf(&l).unwrap().bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn codigo_de_verificacao_estavel_e_legivel() {
        let c = codigo_verificacao(&"0".repeat(64));
        assert_eq!(c, "0000-0000-0000");
        let h = "f".repeat(64);
        assert_eq!(codigo_verificacao(&h), "ZZZZ-ZZZZ-ZZZZ");
        let x = codigo_verificacao("7d1c0a9e4b3f2a1908f7e6d5c4b3a291");
        assert_eq!(x.len(), 14);
        assert!(!x.contains(['I', 'L', 'O', 'U']));
    }
}
