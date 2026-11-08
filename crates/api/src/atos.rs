//! Catálogo de atos e quais perfis os concedem (docs/18 § 5). Negar por padrão (SEC-ACS-1): ato
//! que não está aqui não é concedido a ninguém.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ato {
    AgendaVer,
    RecepcaoVer,
    RecepcaoCheckin,
}

impl Ato {
    #[must_use]
    pub fn concedido_a(self, perfil: &str) -> bool {
        matches!(
            (self, perfil),
            (
                Self::AgendaVer,
                "recepcao" | "tecnico" | "medico" | "gerente"
            ) | (Self::RecepcaoVer, "recepcao" | "tecnico" | "gerente")
                | (Self::RecepcaoCheckin, "recepcao")
        )
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ti_nao_tem_atos_com_dado_de_paciente() {
        for ato in [Ato::AgendaVer, Ato::RecepcaoVer, Ato::RecepcaoCheckin] {
            assert!(!ato.concedido_a("ti"));
            assert!(!ato.concedido_a("financeiro"));
            assert!(!ato.concedido_a("sistema"));
        }
    }

    #[test]
    fn so_recepcao_faz_checkin() {
        assert!(Ato::RecepcaoCheckin.concedido_a("recepcao"));
        assert!(!Ato::RecepcaoCheckin.concedido_a("gerente"));
        assert!(!Ato::RecepcaoCheckin.concedido_a("medico"));
    }
}
