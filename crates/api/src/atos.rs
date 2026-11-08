//! Catálogo de atos e quais perfis os concedem (docs/18 § 5). Negar por padrão (SEC-ACS-1): ato
//! que não está aqui não é concedido a ninguém. O contexto (sala, vínculo, caixa próprio) é
//! conferido em cada caso de uso, depois do ato.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ato {
    AgendaVer,
    AgendaAgendar,
    GuiaRegistrar,
    RecepcaoVer,
    RecepcaoCheckin,
    EtiquetaReimprimir,
    PacienteCorrigir,
    SalaVer,
    SalaBipar,
    TriagemRegistrar,
    TriagemConduta,
    LaudoEscrever,
    LaudoLer,
    CaixaOperar,
    DescontoAutorizar,
    EstornoPedir,
    EstornoAprovar,
    ContingenciaDeclarar,
    ContingenciaLancar,
    ContingenciaEncerrar,
    ContingenciaKit,
    EstudoConciliar,
    PortalConvidar,
    PacienteCadastrar,
    EstoqueVer,
    EstoqueMovimentar,
    ContrasteRegistrar,
    LoteRastrear,
    LastroExplorar,
    IndicadoresVer,
    ComunicacaoEnviar,
}

impl Ato {
    #[must_use]
    pub fn concedido_a(self, perfil: &str) -> bool {
        use Ato as A;
        let perfis: &[&str] = match self {
            // A agenda traz o exame pedido (N3): o caixa recebe pela rota própria (BUG-2026-001).
            A::AgendaVer => &["recepcao", "tecnico", "medico", "gerente", "faturista"],
            A::AgendaAgendar => &["recepcao", "gerente"],
            A::GuiaRegistrar => &["recepcao", "faturista"],
            A::RecepcaoVer => &["recepcao", "tecnico", "gerente", "caixa"],
            A::RecepcaoCheckin | A::EtiquetaReimprimir => &["recepcao"],
            A::PacienteCorrigir => &["recepcao", "gerente"],
            A::SalaVer => &["tecnico", "medico", "gerente"],
            A::SalaBipar | A::TriagemRegistrar => &["tecnico"],
            A::TriagemConduta | A::LaudoEscrever | A::LaudoLer => &["medico"],
            A::CaixaOperar => &["caixa"],
            A::DescontoAutorizar => &["gerente"],
            A::EstornoPedir => &["caixa", "gerente"],
            A::EstornoAprovar => &["financeiro", "gerente"],
            A::ContingenciaDeclarar | A::ContingenciaLancar => &["recepcao", "gerente"],
            A::ContingenciaEncerrar | A::ContingenciaKit => &["gerente"],
            // Vínculo manual de imagem: médico, com dois identificadores (docs/18 § 5).
            A::EstudoConciliar => &["medico"],
            // Convite do portal (docs/04 Cadastro): recepção e gerência.
            A::PortalConvidar => &["recepcao", "gerente"],
            // Cadastro de paciente (docs/18 § 5: recepção; gerência pela supervisão).
            A::PacienteCadastrar => &["recepcao", "gerente"],
            // Estoque (docs/18 § 3 e § 5): o perfil estoque vê itens e lotes sem paciente; contraste é
            // registrado por quem executa o exame; rastrear lote (com pacientes) é da gerência.
            A::EstoqueVer => &["estoque", "gerente", "tecnico", "enfermagem"],
            A::EstoqueMovimentar => &["estoque", "gerente"],
            A::ContrasteRegistrar => &["tecnico", "enfermagem"],
            A::LoteRastrear => &["gerente"],
            // Explorador de lastro: gerência e auditor/DPO (docs/18 § 4: metadados de acesso).
            A::LastroExplorar => &["gerente", "auditor"],
            // Indicadores agregados, sem paciente (docs/25 MVP).
            A::IndicadoresVer => &["gerente", "financeiro"],
            // Lembretes, preferência e histórico de mensagens do paciente (N0/N1).
            A::ComunicacaoEnviar => &["recepcao", "gerente"],
        };
        perfis.contains(&perfil)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const TODOS: [Ato; 31] = [
        Ato::AgendaVer,
        Ato::AgendaAgendar,
        Ato::GuiaRegistrar,
        Ato::RecepcaoVer,
        Ato::RecepcaoCheckin,
        Ato::EtiquetaReimprimir,
        Ato::PacienteCorrigir,
        Ato::SalaVer,
        Ato::SalaBipar,
        Ato::TriagemRegistrar,
        Ato::TriagemConduta,
        Ato::LaudoEscrever,
        Ato::LaudoLer,
        Ato::CaixaOperar,
        Ato::DescontoAutorizar,
        Ato::EstornoPedir,
        Ato::EstornoAprovar,
        Ato::ContingenciaDeclarar,
        Ato::ContingenciaLancar,
        Ato::ContingenciaEncerrar,
        Ato::ContingenciaKit,
        Ato::EstudoConciliar,
        Ato::PortalConvidar,
        Ato::PacienteCadastrar,
        Ato::EstoqueVer,
        Ato::EstoqueMovimentar,
        Ato::ContrasteRegistrar,
        Ato::LoteRastrear,
        Ato::LastroExplorar,
        Ato::IndicadoresVer,
        Ato::ComunicacaoEnviar,
    ];

    #[test]
    fn ti_e_sistema_nao_tem_nenhum_ato_de_tela() {
        for ato in TODOS {
            assert!(!ato.concedido_a("ti"), "{ato:?}");
            assert!(!ato.concedido_a("sistema"), "{ato:?}");
            assert!(!ato.concedido_a(""), "{ato:?}");
        }
    }

    #[test]
    fn so_recepcao_faz_checkin() {
        assert!(Ato::RecepcaoCheckin.concedido_a("recepcao"));
        for p in ["gerente", "medico", "caixa", "tecnico", "financeiro"] {
            assert!(!Ato::RecepcaoCheckin.concedido_a(p), "{p}");
        }
    }

    #[test]
    fn clinico_so_para_medico() {
        for ato in [Ato::TriagemConduta, Ato::LaudoEscrever, Ato::LaudoLer] {
            for p in [
                "recepcao",
                "tecnico",
                "caixa",
                "gerente",
                "financeiro",
                "ti",
            ] {
                assert!(!ato.concedido_a(p), "{ato:?} {p}");
            }
        }
    }
}
