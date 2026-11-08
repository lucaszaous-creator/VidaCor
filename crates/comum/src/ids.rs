//! Identificadores: UUIDv7 (ordenável no tempo) como chave e código legível à parte (docs/21 § 1).

use std::fmt;
use uuid::Uuid;

/// Gera um identificador UUIDv7.
#[must_use]
pub fn novo_id() -> Uuid {
    Uuid::now_v7()
}

/// Código legível de um registro, no formato `PREFIXO-AAAA-NNNNNN` (ex.: `ATD-2026-000001`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodigoLegivel(String);

impl CodigoLegivel {
    /// Monta o código a partir do prefixo do domínio, do ano e do número sequencial do ano.
    #[must_use]
    pub fn novo(prefixo: &str, ano: i32, numero: i64) -> Self {
        Self(format!("{prefixo}-{ano:04}-{numero:06}"))
    }

    #[must_use]
    pub fn como_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CodigoLegivel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn codigo_tem_seis_digitos_e_ano() {
        assert_eq!(
            CodigoLegivel::novo("ATD", 2026, 1).como_str(),
            "ATD-2026-000001"
        );
        assert_eq!(
            CodigoLegivel::novo("ACC", 2026, 118_790).como_str(),
            "ACC-2026-118790"
        );
    }

    #[test]
    fn ids_sao_v7_e_crescentes() {
        let a = novo_id();
        let b = novo_id();
        assert_eq!(a.get_version_num(), 7);
        assert!(a < b);
    }
}
