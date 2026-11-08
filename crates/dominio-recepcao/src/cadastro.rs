//! Cadastro de paciente: conferência do CPF e regra de possível duplicado (Cadastro-Paciente.dc.html,
//! docs/04: "busca de possível duplicado antes de criar, nome + nascimento parecido"). Domínio puro:
//! sem banco, sem HTTP (G5).

use chrono::{Datelike, NaiveDate};
use vidacor_comum::NomeCompleto;

/// CPF só com dígitos e com os dois dígitos verificadores certos.
#[must_use]
pub fn cpf_valido(cpf: &str) -> bool {
    let d: Vec<u32> = cpf.chars().filter_map(|c| c.to_digit(10)).collect();
    if d.len() != 11
        || cpf
            .chars()
            .any(|c| !c.is_ascii_digit() && c != '.' && c != '-')
    {
        return false;
    }
    if d.iter().all(|x| *x == d[0]) {
        return false;
    }
    let dv = |n: usize| {
        let soma: u32 = (0..n)
            .map(|i| d[i] * (u32::try_from(n + 1 - i).unwrap_or(0)))
            .sum();
        let r = (soma * 10) % 11;
        if r == 10 {
            0
        } else {
            r
        }
    };
    dv(9) == d[9] && dv(10) == d[10]
}

/// Só os dígitos do CPF (forma gravada).
#[must_use]
pub fn cpf_digitos(cpf: &str) -> String {
    cpf.chars().filter(char::is_ascii_digit).collect()
}

/// Por que um cadastro existente parece a mesma pessoa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotivoDeDuplicado {
    MesmoCpf,
    MesmoNomeENascimento,
    MesmoNomeNascimentoParecido,
    NomeParecidoMesmoNascimento,
}

impl MotivoDeDuplicado {
    #[must_use]
    pub fn texto(&self) -> &'static str {
        match self {
            Self::MesmoCpf => "Mesmo CPF",
            Self::MesmoNomeENascimento => "Mesmo nome e mesmo nascimento",
            Self::MesmoNomeNascimentoParecido => {
                "Mesmo nome e nascimento parecido (dia e mês trocados ou um dígito diferente)"
            }
            Self::NomeParecidoMesmoNascimento => {
                "Mesmo nascimento e nome parecido (primeiro e último nome iguais)"
            }
        }
    }
}

/// Dia e mês trocados, um dígito diferente ou dois dígitos vizinhos invertidos (ddmmaaaa).
fn nascimento_parecido(a: NaiveDate, b: NaiveDate) -> bool {
    if a == b {
        return false;
    }
    let trocado = a.year() == b.year() && a.day() == b.month() && a.month() == b.day();
    let fa = a.format("%d%m%Y").to_string();
    let fb = b.format("%d%m%Y").to_string();
    let pos: Vec<usize> = fa
        .chars()
        .zip(fb.chars())
        .enumerate()
        .filter(|(_, (x, y))| x != y)
        .map(|(i, _)| i)
        .collect();
    // Dois dígitos vizinhos invertidos (21 no lugar de 12).
    let invertidos = pos.len() == 2
        && pos[1] == pos[0] + 1
        && fa.as_bytes()[pos[0]] == fb.as_bytes()[pos[1]]
        && fa.as_bytes()[pos[1]] == fb.as_bytes()[pos[0]];
    trocado || pos.len() == 1 || invertidos
}

fn primeiro_e_ultimo(n: &NomeCompleto) -> (String, String) {
    let f = n.forma_de_comparacao();
    let partes: Vec<&str> = f.split(' ').collect();
    (
        partes.first().copied().unwrap_or_default().to_owned(),
        partes.last().copied().unwrap_or_default().to_owned(),
    )
}

/// Compara o cadastro novo com um existente. `None`: não parece a mesma pessoa.
#[must_use]
pub fn possivel_duplicado(
    nome: &NomeCompleto,
    nascimento: NaiveDate,
    cpf: Option<&str>,
    existente_nome: &NomeCompleto,
    existente_nascimento: NaiveDate,
    existente_cpf: Option<&str>,
) -> Option<MotivoDeDuplicado> {
    if let (Some(a), Some(b)) = (cpf, existente_cpf) {
        if !a.is_empty() && cpf_digitos(a) == cpf_digitos(b) {
            return Some(MotivoDeDuplicado::MesmoCpf);
        }
    }
    let mesmo_nome = nome.confere_com(existente_nome);
    if mesmo_nome && nascimento == existente_nascimento {
        return Some(MotivoDeDuplicado::MesmoNomeENascimento);
    }
    if mesmo_nome && nascimento_parecido(nascimento, existente_nascimento) {
        return Some(MotivoDeDuplicado::MesmoNomeNascimentoParecido);
    }
    if nascimento == existente_nascimento
        && primeiro_e_ultimo(nome) == primeiro_e_ultimo(existente_nome)
    {
        return Some(MotivoDeDuplicado::NomeParecidoMesmoNascimento);
    }
    None
}

#[cfg(test)]
mod testes {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn n(s: &str) -> NomeCompleto {
        NomeCompleto::novo(s).unwrap()
    }
    fn d(a: i32, m: u32, dia: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, dia).unwrap()
    }

    #[test]
    fn cpf_confere_os_digitos_verificadores() {
        assert!(cpf_valido("529.982.247-25"));
        assert!(cpf_valido("52998224725"));
        assert!(!cpf_valido("529.982.247-26"));
        assert!(!cpf_valido("111.111.111-11"));
        assert!(!cpf_valido("1234"));
        assert!(!cpf_valido("529x982x247x25"));
    }

    #[test]
    fn maria_helena_com_dia_e_mes_trocados_e_possivel_duplicado() {
        // Fictícios do design: Maria Helena Duarte 12/03/1968, 21/03/1968 (dígitos invertidos) e 03/12/1968 (dia e mês trocados).
        let m = n("Maria Helena Duarte");
        assert_eq!(
            possivel_duplicado(&m, d(1968, 3, 21), None, &m, d(1968, 3, 12), None),
            Some(MotivoDeDuplicado::MesmoNomeNascimentoParecido)
        );
        assert_eq!(
            possivel_duplicado(&m, d(1968, 12, 3), None, &m, d(1968, 3, 12), None),
            Some(MotivoDeDuplicado::MesmoNomeNascimentoParecido)
        );
        assert_eq!(
            possivel_duplicado(
                &n("maria helena  DUARTE"),
                d(1968, 3, 12),
                None,
                &m,
                d(1968, 3, 12),
                None
            ),
            Some(MotivoDeDuplicado::MesmoNomeENascimento)
        );
    }

    #[test]
    fn nome_do_meio_diferente_com_mesmo_nascimento_e_possivel_duplicado() {
        assert_eq!(
            possivel_duplicado(
                &n("Sônia Castro"),
                d(1962, 2, 3),
                None,
                &n("Sônia Maria Castro"),
                d(1962, 2, 3),
                None
            ),
            Some(MotivoDeDuplicado::NomeParecidoMesmoNascimento)
        );
    }

    #[test]
    fn mesmo_cpf_e_duplicado_e_pessoas_diferentes_nao() {
        assert_eq!(
            possivel_duplicado(
                &n("Ana Lima"),
                d(1990, 1, 1),
                Some("529.982.247-25"),
                &n("Rita Souza"),
                d(1970, 5, 5),
                Some("52998224725")
            ),
            Some(MotivoDeDuplicado::MesmoCpf)
        );
        assert_eq!(
            possivel_duplicado(
                &n("Rafael Monteiro Lima"),
                d(1988, 11, 9),
                None,
                &n("Rita Costa Barbosa"),
                d(1966, 9, 9),
                None
            ),
            None
        );
        assert_eq!(
            possivel_duplicado(
                &n("Maria Helena Duarte"),
                d(1990, 7, 7),
                None,
                &n("Maria Helena Duarte"),
                d(1968, 3, 12),
                None
            ),
            None
        );
    }
}
