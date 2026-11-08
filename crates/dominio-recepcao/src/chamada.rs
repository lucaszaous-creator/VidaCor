//! Painel de chamada (Telao.dc.html; docs/18: o dispositivo TV vê só senha, nome abreviado e sala).
//! O nome abreviado é decidido aqui, no núcleo, e gravado na chamada: a TV nunca lê o cadastro.

/// Partículas que não viram inicial ("da", "de", "dos"...).
const PARTICULAS: [&str; 7] = ["da", "das", "de", "do", "dos", "e", "d'"];

/// Primeiro nome por extenso e as iniciais dos demais (no máximo três), como no mockup:
/// "Antônio Carlos Ribeiro" vira "Antônio C. R.".
#[must_use]
pub fn nome_abreviado(nome: &str) -> String {
    let mut partes = nome.split_whitespace();
    let Some(primeiro) = partes.next() else {
        return String::new();
    };
    let iniciais: Vec<String> = partes
        .filter(|p| !PARTICULAS.contains(&p.to_lowercase().as_str()))
        .filter_map(|p| p.chars().next())
        .take(3)
        .map(|c| format!("{}.", c.to_uppercase()))
        .collect();
    let primeiro: String = primeiro.chars().take(20).collect();
    if iniciais.is_empty() {
        primeiro
    } else {
        format!("{primeiro} {}", iniciais.join(" "))
    }
}

/// Destino da chamada: guichê da recepção (1 a 20) ou a sala do exame.
#[must_use]
pub fn guiche_valido(numero: i32) -> bool {
    (1..=20).contains(&numero)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn primeiro_nome_e_iniciais() {
        assert_eq!(nome_abreviado("Antônio Carlos Ribeiro"), "Antônio C. R.");
        assert_eq!(
            nome_abreviado("Maria Helena da Silva Duarte"),
            "Maria H. S. D."
        );
        assert_eq!(nome_abreviado("  vera   lúcia antunes "), "vera L. A.");
    }

    #[test]
    fn nunca_mostra_sobrenome_inteiro() {
        let a = nome_abreviado("João Pedro Albuquerque Cavalcanti Neto Filho");
        assert_eq!(a, "João P. A. C.");
        assert!(!a.contains("Albuquerque"));
    }

    #[test]
    fn nome_unico_e_vazio() {
        assert_eq!(nome_abreviado("Cher"), "Cher");
        assert_eq!(nome_abreviado("   "), "");
    }

    #[test]
    fn guiche_de_1_a_20() {
        assert!(guiche_valido(1) && guiche_valido(20));
        assert!(!guiche_valido(0) && !guiche_valido(21) && !guiche_valido(-1));
    }
}
