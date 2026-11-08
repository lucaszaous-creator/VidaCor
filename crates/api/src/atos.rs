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
    FinanceiroOperar,
    FinanceiroAprovar,
    UsuarioAdministrar,
    PerfilConferir,
    ExameExternoImportar,
    FaturamentoVer,
    FaturamentoAuditar,
    LoteGerar,
    DemonstrativoRegistrar,
    GlosaRecorrer,
    ChamadaFazer,
    PainelVer,
    OrcamentoEmitir,
    AparelhoAdministrar,
    AutorizacaoPedir,
    AutorizacaoVer,
    BloqueioGerir,
    EsperaGerir,
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
            // Financeiro (docs/18 § 5: contas, conciliação, repasse): financeiro e gerência.
            A::FinanceiroOperar | A::FinanceiroAprovar => &["financeiro", "gerente"],
            // Contas e perfis (docs/18 § 6): TI e gerência administram; a conferência de perfil
            // clínico (segunda pessoa) é da gerência.
            A::UsuarioAdministrar => &["ti", "gerente"],
            A::PerfilConferir => &["gerente"],
            // Exame externo (docs/22 E3): a recepção importa com conferência de identidade.
            A::ExameExternoImportar => &["recepcao", "gerente"],
            // Faturamento TISS (docs/18 § 5: faturista gera guia, lote e recurso; gerente acompanha).
            A::FaturamentoVer => &["faturista", "gerente"],
            A::FaturamentoAuditar | A::LoteGerar | A::DemonstrativoRegistrar | A::GlosaRecorrer => {
                &["faturista"]
            }
            // TV de chamada (docs/18 § 3): a recepção chama ao guichê; o técnico, à sala. O painel é
            // do dispositivo TV; recepção e gerência podem abri-lo para conferir.
            A::ChamadaFazer => &["recepcao", "tecnico"],
            A::PainelVer => &["tv", "recepcao", "gerente"],
            // Orçamento do particular (docs/04 Agenda): recepção emite; gerência acompanha.
            A::OrcamentoEmitir => &["recepcao", "gerente"],
            // Aparelhos e integrações (docs/18 § 3: TI administra aparelhos, sem dado clínico).
            A::AparelhoAdministrar => &["ti", "gerente"],
            // Bloqueio de sala e horário, lista de espera e oferta de vaga (docs/04 Agenda e
            // Lista-Espera): recepção e gerência (DEC-2026-11-08-82).
            A::BloqueioGerir | A::EsperaGerir => &["recepcao", "gerente"],
            // Autorização pedida pelo sistema (docs/22 E4): recepção e faturista pedem; gerente acompanha.
            A::AutorizacaoPedir => &["recepcao", "faturista"],
            A::AutorizacaoVer => &["recepcao", "faturista", "gerente"],
        };
        perfis.contains(&perfil)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const TODOS: [Ato; 49] = [
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
        Ato::FinanceiroOperar,
        Ato::FinanceiroAprovar,
        Ato::UsuarioAdministrar,
        Ato::PerfilConferir,
        Ato::ExameExternoImportar,
        Ato::FaturamentoVer,
        Ato::FaturamentoAuditar,
        Ato::LoteGerar,
        Ato::DemonstrativoRegistrar,
        Ato::GlosaRecorrer,
        Ato::ChamadaFazer,
        Ato::PainelVer,
        Ato::OrcamentoEmitir,
        Ato::AparelhoAdministrar,
        Ato::AutorizacaoPedir,
        Ato::AutorizacaoVer,
        Ato::BloqueioGerir,
        Ato::EsperaGerir,
    ];

    #[test]
    fn ti_so_administra_contas_e_sistema_nao_tem_ato_de_tela() {
        // docs/18: TI administra usuários, aparelhos e integrações, sem dado clínico nem financeiro.
        // Os atos de tela da TI são administrar contas e aparelhos (DEC-2026-11-08-69 e 75).
        for ato in TODOS {
            assert_eq!(
                ato.concedido_a("ti"),
                matches!(ato, Ato::UsuarioAdministrar | Ato::AparelhoAdministrar),
                "{ato:?}"
            );
            assert!(!ato.concedido_a("sistema"), "{ato:?}");
            assert!(!ato.concedido_a(""), "{ato:?}");
        }
    }

    #[test]
    fn tv_so_ve_o_painel() {
        for ato in TODOS {
            assert_eq!(ato.concedido_a("tv"), ato == Ato::PainelVer, "{ato:?}");
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
    fn faturamento_so_para_faturista_e_gerente_so_le() {
        for ato in [
            Ato::FaturamentoAuditar,
            Ato::LoteGerar,
            Ato::DemonstrativoRegistrar,
            Ato::GlosaRecorrer,
        ] {
            assert!(ato.concedido_a("faturista"), "{ato:?}");
            for p in [
                "recepcao",
                "caixa",
                "medico",
                "gerente",
                "financeiro",
                "tecnico",
            ] {
                assert!(!ato.concedido_a(p), "{ato:?} {p}");
            }
        }
        assert!(Ato::FaturamentoVer.concedido_a("gerente"));
        assert!(!Ato::FaturamentoVer.concedido_a("financeiro"));
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
