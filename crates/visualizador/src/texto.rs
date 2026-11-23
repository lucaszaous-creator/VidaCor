//! Formatação de texto vindo do DICOM.

/// Converte data DICOM (`AAAAMMDD`) para `DD/MM/AAAA`. Valor fora do formato devolve `None`
/// (a tela mostra "não informado" em vez de inventar uma data, R13).
pub fn data_br(dicom: &str) -> Option<String> {
    let d = dicom.trim();
    if d.len() != 8 || !d.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (ano, mes, dia) = (&d[0..4], &d[4..6], &d[6..8]);
    let m: u32 = mes.parse().ok()?;
    let di: u32 = dia.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&di) {
        return None;
    }
    Some(format!("{dia}/{mes}/{ano}"))
}

/// Nome de pessoa DICOM (PN, `FAMILIA^NOMES^MEIO^PREFIXO^SUFIXO`) na ordem de leitura:
/// prefixo, nomes, meio, família, sufixo. Só o grupo alfabético (antes do primeiro `=`) é usado.
/// Nome gravado inteiro num componente só (comum em clínicas do Brasil) sai como está.
pub(crate) fn nome_pessoa(pn: &str) -> String {
    let alfabetico = pn.split('=').next().unwrap_or_default();
    let c: Vec<&str> = alfabetico.split('^').map(str::trim).collect();
    let pega = |i: usize| c.get(i).copied().unwrap_or_default();
    [pega(3), pega(1), pega(2), pega(0), pega(4)]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn data_dicom_vira_data_brasileira() {
        assert_eq!(data_br("19800101").as_deref(), Some("01/01/1980"));
        assert_eq!(data_br(" 20261123 ").as_deref(), Some("23/11/2026"));
        assert_eq!(data_br(""), None);
        assert_eq!(data_br("1980-01-01"), None);
        assert_eq!(data_br("19801301"), None);
    }

    #[test]
    fn nome_dicom_na_ordem_de_leitura() {
        assert_eq!(nome_pessoa("FICTICIO^ANA MARIA"), "ANA MARIA FICTICIO");
        assert_eq!(nome_pessoa("ANA MARIA FICTICIO"), "ANA MARIA FICTICIO");
        assert_eq!(nome_pessoa("SILVA^ANA^^DRA."), "DRA. ANA SILVA");
        assert_eq!(nome_pessoa("SILVA^ANA=シルバ"), "ANA SILVA");
        assert_eq!(nome_pessoa(""), "");
    }
}
