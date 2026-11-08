//! Regras de auditoria antes do faturamento (docs/22 E5). Casos montados a partir do mockup
//! Auditoria-Faturamento; AGUARDAM CONFERÊNCIA PELO ANALISTA TISS (DEC-2026-11-08-65).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use comum::{ctx, dia, guia_ok};
use vidacor_dominio_faturamento::auditoria::{conjunto, conjunto_padrao, pronta, CodigoRegra};
use vidacor_dominio_faturamento::{auditar, Centavos, Efeito, ItemExecutado, CONJUNTOS};

fn regras(versao: &str, g: &vidacor_dominio_faturamento::GuiaFaturavel) -> Vec<CodigoRegra> {
    auditar(g, &ctx(), conjunto(versao).unwrap())
        .into_iter()
        .map(|p| p.regra)
        .collect()
}

#[test]
fn guia_completa_passa_em_todas_as_operadoras() {
    for c in &CONJUNTOS {
        let mut g = guia_ok();
        g.cid = Some("R51".into());
        g.assinatura_beneficiario = true;
        assert_eq!(auditar(&g, &ctx(), c), vec![], "{}", c.versao);
    }
}

#[test]
fn versoes_sao_unicas_e_no_formato() {
    let mut vistas = std::collections::BTreeSet::new();
    for c in &CONJUNTOS {
        assert!(vistas.insert(c.versao), "versão repetida {}", c.versao);
        let (nome, numero) = c.versao.rsplit_once('.').unwrap();
        assert!(
            nome.starts_with("AUD-") && nome.ends_with("-2026"),
            "{}",
            c.versao
        );
        assert!(numero.parse::<u32>().is_ok(), "{}", c.versao);
    }
    assert_eq!(conjunto_padrao().versao, "AUD-TISS-2026.1");
    assert!(conjunto("AUD-INEXISTENTE-2026.1").is_none());
}

#[test]
fn quando_a_senha_vence_antes_do_exame_entao_bloqueia() {
    let mut g = guia_ok();
    g.validade_senha = dia(4, 11);
    let p = auditar(&g, &ctx(), conjunto("AUD-UNIMED-2026.1").unwrap());
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].regra, CodigoRegra::SenhaVencida);
    assert_eq!(p[0].efeito, Efeito::Bloqueia);
    assert_eq!(
        p[0].detalhe,
        "Senha venceu em 04/11/2026, exame em 05/11/2026."
    );
    assert!(!pronta(&p));
    // Vencer no próprio dia do exame ainda vale.
    g.validade_senha = dia(5, 11);
    assert_eq!(regras("AUD-UNIMED-2026.1", &g), vec![]);
}

#[test]
fn quando_o_executado_difere_do_autorizado_entao_bloqueia() {
    let mut g = guia_ok();
    g.executados[0].quantidade = 2;
    assert_eq!(
        regras("AUD-UNIMED-2026.1", &g),
        vec![CodigoRegra::ProcedimentoNaoAutorizado]
    );
    let mut g = guia_ok();
    g.executados.push(ItemExecutado {
        tuss: "40901300".into(),
        descricao: "Ecodoppler de carótidas".into(),
        quantidade: 1,
        valor_unitario: Some(Centavos(19_620)),
    });
    let p = auditar(&g, &ctx(), conjunto("AUD-UNIMED-2026.1").unwrap());
    assert_eq!(p[0].detalhe, "Executado sem autorização: 40901300 (1x).");
}

#[test]
fn quando_o_laudo_nao_esta_assinado_entao_bloqueia() {
    let mut g = guia_ok();
    g.laudo_assinado = false;
    let p = auditar(&g, &ctx(), conjunto("AUD-TISS-2026.1").unwrap());
    assert_eq!(p[0].regra, CodigoRegra::LaudoNaoAssinado);
    assert_eq!(p[0].origem, "regra da clínica");
    assert!(!pronta(&p));
}

#[test]
fn quando_o_solicitante_esta_incompleto_entao_corrigir_antes() {
    let mut g = guia_ok();
    g.solicitante.cbos = None;
    let p = auditar(&g, &ctx(), conjunto("AUD-UNIMED-2026.1").unwrap());
    assert_eq!(p[0].regra, CodigoRegra::SolicitanteIncompleto);
    assert_eq!(p[0].efeito, Efeito::CorrigirAntes);
    assert!(!pronta(&p));
    for (campo, valor) in [("uf", "RJ"), ("conselho", "CRM"), ("cbos", "22512")] {
        let mut g = guia_ok();
        match campo {
            "uf" => g.solicitante.uf = Some(valor.into()),
            "conselho" => g.solicitante.conselho = Some(valor.into()),
            _ => g.solicitante.cbos = Some(valor.into()),
        }
        assert_eq!(
            regras("AUD-UNIMED-2026.1", &g),
            vec![CodigoRegra::SolicitanteIncompleto],
            "{campo}={valor}"
        );
    }
}

#[test]
fn sem_preco_bloqueia_e_preco_antigo_pede_correcao() {
    let mut g = guia_ok();
    g.executados[0].valor_unitario = None;
    assert_eq!(
        regras("AUD-UNIMED-2026.1", &g),
        vec![CodigoRegra::ProcedimentoSemPreco]
    );
    let mut g = guia_ok();
    g.executados[0].valor_unitario = Some(Centavos(23_680));
    let p = auditar(&g, &ctx(), conjunto("AUD-UNIMED-2026.1").unwrap());
    assert_eq!(p[0].regra, CodigoRegra::ValorDiferenteDaTabela);
    assert_eq!(
        p[0].detalhe,
        "40901114: cobrado R$ 236,80, tabela atual R$ 248,60."
    );
    assert_eq!(p[0].efeito, Efeito::CorrigirAntes);
}

#[test]
fn regras_proprias_valem_so_na_operadora_delas() {
    let g = guia_ok();
    assert_eq!(
        regras("AUD-SULAMERICA-2026.1", &g),
        vec![CodigoRegra::CidAusente]
    );
    assert_eq!(
        regras("AUD-BRADESCO-2026.1", &g),
        vec![CodigoRegra::AssinaturaBeneficiarioAusente]
    );
    assert_eq!(regras("AUD-UNIMED-2026.1", &g), vec![]);
    let mut g = guia_ok();
    g.cid = Some("R5".into());
    assert_eq!(
        regras("AUD-SULAMERICA-2026.1", &g),
        vec![CodigoRegra::CidAusente],
        "CID com formato inválido"
    );
    g.cid = Some("I650".into());
    assert_eq!(regras("AUD-SULAMERICA-2026.1", &g), vec![]);
}

#[test]
fn prazo_de_envio_avisa_perto_e_bloqueia_vencido() {
    let mut c = ctx();
    let g = guia_ok();
    let conj = conjunto("AUD-UNIMED-2026.1").unwrap();
    // Exame em 05/11, prazo de 30 dias: envio até 05/12.
    c.hoje = dia(27, 11);
    assert_eq!(auditar(&g, &c, conj), vec![]);
    c.hoje = dia(28, 11);
    let p = auditar(&g, &c, conj);
    assert_eq!(p[0].regra, CodigoRegra::PrazoDeEnvioProximo);
    assert_eq!(p[0].efeito, Efeito::Atencao);
    assert!(pronta(&p), "atenção não impede o lote");
    c.hoje = dia(5, 12);
    assert_eq!(
        auditar(&g, &c, conj)[0].regra,
        CodigoRegra::PrazoDeEnvioProximo
    );
    c.hoje = dia(6, 12);
    let p = auditar(&g, &c, conj);
    assert_eq!(p[0].regra, CodigoRegra::PrazoDeEnvioVencido);
    assert!(!pronta(&p));
}

#[test]
fn sem_contrato_e_fora_do_padrao_bloqueiam() {
    let mut g = guia_ok();
    g.registro_ans = None;
    let mut c = ctx();
    c.prazo_envio_dias = None;
    let p = auditar(&g, &c, conjunto_padrao());
    assert_eq!(p[0].regra, CodigoRegra::OperadoraSemContrato);
    let mut g = guia_ok();
    g.senha = "X".repeat(21);
    g.carteirinha = " . - ".into();
    let p = auditar(&g, &ctx(), conjunto_padrao());
    assert_eq!(p[0].regra, CodigoRegra::GuiaForaDoPadrao);
    assert_eq!(p[0].detalhe, "Fora do padrão: senha, carteirinha.");
}
