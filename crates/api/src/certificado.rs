//! Leitura do certificado ICP-Brasil do médico (só a parte PÚBLICA: `.cer` em PEM ou DER). Nunca
//! aceita chave privada nem arquivo PKCS#12: a assinatura continua no token ou no provedor de
//! assinatura, fora do núcleo (SEC-CRI). Daqui sai o que a gerência confere: titular, CPF mascarado,
//! emissor, validade, tipo (A1 ou A3) e a impressão digital SHA-256. DEC-2026-11-09-121.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use x509_parser::extensions::{GeneralName, ParsedExtension};
use x509_parser::prelude::{FromDer, X509Certificate};

/// Tamanho máximo aceito (um certificado de usuário tem 1 a 3 KiB).
pub const TAMANHO_MAXIMO: usize = 16 * 1024;
/// Prefixo das políticas de certificado da ICP-Brasil (DOC-ICP-04).
const POLITICA_ICP: &str = "2.16.76.1.2.";
/// Campo do titular pessoa física: nascimento (8) + CPF (11) + ... (DOC-ICP-04, 2.16.76.1.3.1).
const OID_DADOS_PF: &str = "2.16.76.1.3.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificadoLido {
    /// Nome do titular (o CN sem o ":CPF" do fim, quando houver).
    pub titular: String,
    /// CPF do titular, já mascarado (só os dois últimos dígitos), quando o certificado traz.
    pub cpf_mascarado: Option<String>,
    pub emissor: String,
    pub serie: String,
    pub valido_de: DateTime<Utc>,
    pub valido_ate: DateTime<Utc>,
    /// SHA-256 do DER, em hexadecimal minúsculo.
    pub sha256: String,
    /// A1, A3 ou outro tipo da ICP-Brasil (A2, A4, S...), pela política.
    pub tipo: Option<String>,
    pub icp_brasil: bool,
}

fn pem_para_der(entrada: &[u8]) -> Result<Vec<u8>, &'static str> {
    let texto = std::str::from_utf8(entrada).map_err(|_| "arquivo ilegível")?;
    let inicio = "-----BEGIN CERTIFICATE-----";
    let fim = "-----END CERTIFICATE-----";
    let a = texto.find(inicio).ok_or("arquivo sem certificado")?;
    let b = texto[a..].find(fim).ok_or("certificado incompleto")?;
    let corpo: String = texto[a + inicio.len()..a + b]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(corpo)
        .map_err(|_| "certificado com base64 inválido")
}

fn mascarar(cpf: &str) -> String {
    format!("***.***.***-{}", &cpf[9..])
}

/// Troca qualquer sequência de 11 dígitos (CPF) pela forma mascarada; nunca guardamos o CPF inteiro.
fn sem_cpf(texto: &str) -> String {
    let mut saida = String::new();
    let mut digitos = String::new();
    let despejar = |d: &mut String, s: &mut String| {
        if d.len() == 11 {
            s.push_str(&mascarar(d));
        } else {
            s.push_str(d);
        }
        d.clear();
    };
    for c in texto.chars() {
        if c.is_ascii_digit() {
            digitos.push(c);
        } else {
            despejar(&mut digitos, &mut saida);
            saida.push(c);
        }
    }
    despejar(&mut digitos, &mut saida);
    saida
}

fn tipo_da_politica(oid: &str) -> Option<String> {
    let resto = oid.strip_prefix(POLITICA_ICP)?;
    let codigo = resto.split('.').next()?;
    Some(
        match codigo {
            "1" => "A1",
            "2" => "A2",
            "3" => "A3",
            "4" => "A4",
            _ => "outro",
        }
        .to_owned(),
    )
}

/// CPF do campo 2.16.76.1.3.1 (os 11 dígitos depois dos 8 da data de nascimento).
fn cpf_do_campo(valor: &[u8]) -> Option<String> {
    let digitos: String = valor
        .iter()
        .map(|b| char::from(*b))
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    (digitos.len() >= 19).then(|| digitos[8..19].to_owned())
}

/// Lê o certificado público (PEM ou DER).
///
/// # Errors
/// Arquivo grande demais, com chave privada, PKCS#12 ou que não é um certificado X.509.
pub fn ler(entrada: &[u8]) -> Result<CertificadoLido, &'static str> {
    if entrada.is_empty() || entrada.len() > TAMANHO_MAXIMO {
        return Err("o arquivo do certificado tem até 16 KB");
    }
    if entrada.windows(11).any(|j| j == b"PRIVATE KEY") {
        return Err("o arquivo traz chave privada; envie só o certificado público (.cer)");
    }
    let der = if entrada.starts_with(b"-----") || entrada.starts_with(b"\xef\xbb\xbf-----") {
        pem_para_der(entrada)?
    } else {
        entrada.to_vec()
    };
    let (resto, cert) = X509Certificate::from_der(&der)
        .map_err(|_| "não é um certificado X.509 (arquivos .pfx ou .p12 não são aceitos)")?;
    if !resto.is_empty() {
        return Err("arquivo com dados além do certificado");
    }
    let cn = cert
        .subject()
        .iter_common_name()
        .next()
        .and_then(|a| a.as_str().ok())
        .unwrap_or_default()
        .to_owned();
    let mut cpf = None;
    let mut tipo = None;
    for ext in cert.extensions() {
        match ext.parsed_extension() {
            ParsedExtension::CertificatePolicies(politicas) => {
                for p in politicas.iter() {
                    if let Some(t) = tipo_da_politica(&p.policy_id.to_id_string()) {
                        tipo = Some(t);
                    }
                }
            }
            ParsedExtension::SubjectAlternativeName(san) => {
                for nome in &san.general_names {
                    if let GeneralName::OtherName(oid, valor) = nome {
                        if oid.to_id_string() == OID_DADOS_PF {
                            cpf = cpf_do_campo(valor);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    // No e-CPF o CN é "NOME:CPF"; sem campo próprio, o CPF do CN vale.
    let (titular, cpf_cn) = match cn.rsplit_once(':') {
        Some((nome, n)) if n.len() == 11 && n.bytes().all(|b| b.is_ascii_digit()) => {
            (nome.trim().to_owned(), Some(n.to_owned()))
        }
        _ => (cn.trim().to_owned(), None),
    };
    let cpf = cpf.or(cpf_cn);
    let nome_do_emissor: Option<String> = cert
        .issuer()
        .iter_common_name()
        .next()
        .and_then(|a| a.as_str().ok())
        .map(str::to_owned);
    let emissor = sem_cpf(&nome_do_emissor.unwrap_or_else(|| cert.issuer().to_string()));
    let validade = cert.validity();
    let data = |t: x509_parser::time::ASN1Time| {
        DateTime::<Utc>::from_timestamp(t.timestamp(), 0).ok_or("data inválida no certificado")
    };
    Ok(CertificadoLido {
        titular,
        cpf_mascarado: cpf.as_deref().map(mascarar),
        emissor,
        serie: cert.raw_serial_as_string(),
        valido_de: data(validade.not_before)?,
        valido_ate: data(validade.not_after)?,
        sha256: Sha256::digest(&der)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        icp_brasil: tipo.is_some(),
        tipo,
    })
}

/// O nome do titular confere com o nome do cadastro? Compara sem acento, sem caixa e sem título
/// ("Dr.", "Dra."), pelas palavras do titular contidas no cadastro.
#[must_use]
pub fn nome_confere(titular: &str, cadastro: &str) -> bool {
    let normal = |s: &str| -> Vec<String> {
        s.chars()
            .map(|c| match c {
                'á' | 'à' | 'â' | 'ã' | 'Á' | 'À' | 'Â' | 'Ã' => 'a',
                'é' | 'ê' | 'É' | 'Ê' => 'e',
                'í' | 'Í' => 'i',
                'ó' | 'ô' | 'õ' | 'Ó' | 'Ô' | 'Õ' => 'o',
                'ú' | 'ü' | 'Ú' | 'Ü' => 'u',
                'ç' | 'Ç' => 'c',
                outro => outro.to_ascii_lowercase(),
            })
            .collect::<String>()
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|p| {
                !p.is_empty()
                    && !matches!(*p, "dr" | "dra" | "de" | "da" | "do" | "dos" | "das" | "e")
            })
            .map(str::to_owned)
            .collect()
    };
    let t = normal(titular);
    let c = normal(cadastro);
    !c.is_empty() && c.iter().all(|p| t.contains(p))
}

#[cfg(test)]
mod testes {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const ICP: &[u8] = include_bytes!("../tests/dados/certificado-icp-ficticio.pem");
    const COMUM: &[u8] = include_bytes!("../tests/dados/certificado-comum-ficticio.pem");

    #[test]
    fn le_certificado_icp_ficticio() {
        let c = ler(ICP).unwrap();
        assert_eq!(c.titular, "PAULO MEDEIROS");
        assert_eq!(c.cpf_mascarado.as_deref(), Some("***.***.***-35"));
        assert_eq!(c.tipo.as_deref(), Some("A3"));
        assert!(c.icp_brasil);
        assert_eq!(c.sha256.len(), 64);
        assert!(c.valido_ate > c.valido_de);
        assert!(!c.emissor.contains("11144477735"));
        assert_eq!(sem_cpf("AC X:11144477735"), "AC X:***.***.***-35");
    }

    #[test]
    fn recusa_o_que_nao_e_certificado_publico() {
        assert!(!ler(COMUM).unwrap().icp_brasil);
        assert!(ler(b"-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----").is_err());
        assert!(ler(b"lixo").is_err());
        assert!(ler(&[]).is_err());
        assert!(ler(&vec![b'a'; TAMANHO_MAXIMO + 1]).is_err());
    }

    #[test]
    fn nome_do_titular() {
        assert!(nome_confere("PAULO MEDEIROS", "Dr. Paulo Medeiros"));
        assert!(nome_confere(
            "CECILIA SOUTO DE ALMEIDA",
            "Dra. Cecília Souto"
        ));
        assert!(!nome_confere("MARCOS FREIRE", "Dr. Paulo Medeiros"));
    }
}
