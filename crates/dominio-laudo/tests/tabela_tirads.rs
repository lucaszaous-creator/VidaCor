//! Casos da tabela ACR TI-RADS 2017 (versão `TIRADS-ACR-2017.1`).
//!
//! AGUARDAM CONFERÊNCIA POR MÉDICO (docs/25 § 3.3): os valores esperados abaixo foram transcritos da
//! tabela oficial (Tessler et al., JACR 2017) por quem escreveu o código; nenhum médico os conferiu
//! ainda. Os pontos "a confirmar" estão no DEC-2026-11-08-64.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use vidacor_dominio_laudo::tirads::{
    self, Categoria, Composicao as C, Conduta, Ecogenicidade as E, FocoEcogenico as F, Forma,
    Margem as M, Nodulo,
};
use vidacor_dominio_laudo::{ErroClassificacao, Medida};

fn nodulo(c: C, e: E, forma: Forma, m: M, focos: &[F]) -> Nodulo {
    Nodulo {
        composicao: c,
        ecogenicidade: e,
        forma,
        margem: m,
        focos: focos.to_vec(),
    }
}

fn mm(v: f64) -> Medida {
    Medida::de_milimetros(v, tirads::LIMITE_MM).unwrap()
}

#[test]
fn pontos_de_cada_descritor_pela_tabela() {
    let base = || {
        nodulo(
            C::CisticaOuQuaseCistica,
            E::Anecoica,
            Forma::MaisLargoQueAlto,
            M::Lisa,
            &[],
        )
    };
    assert_eq!(tirads::pontos(&base()), 0);
    let casos: [(Nodulo, u8); 16] = [
        (
            Nodulo {
                composicao: C::MistaCisticaESolida,
                ecogenicidade: E::HiperOuIsoecoica,
                ..base()
            },
            1 + 1,
        ),
        (
            Nodulo {
                composicao: C::SolidaOuQuaseSolida,
                ecogenicidade: E::HiperOuIsoecoica,
                ..base()
            },
            2 + 1,
        ),
        (
            Nodulo {
                composicao: C::IndeterminadaPorCalcificacao,
                ecogenicidade: E::HiperOuIsoecoica,
                ..base()
            },
            2 + 1,
        ),
        (
            Nodulo {
                composicao: C::SolidaOuQuaseSolida,
                ecogenicidade: E::Hipoecoica,
                ..base()
            },
            2 + 2,
        ),
        (
            Nodulo {
                composicao: C::SolidaOuQuaseSolida,
                ecogenicidade: E::MuitoHipoecoica,
                ..base()
            },
            2 + 3,
        ),
        (
            Nodulo {
                composicao: C::SolidaOuQuaseSolida,
                ecogenicidade: E::Indeterminada,
                ..base()
            },
            2 + 1,
        ),
        (
            Nodulo {
                forma: Forma::MaisAltoQueLargo,
                ..base()
            },
            3,
        ),
        (
            Nodulo {
                margem: M::MalDefinida,
                ..base()
            },
            0,
        ),
        (
            Nodulo {
                margem: M::Indeterminada,
                ..base()
            },
            0,
        ),
        (
            Nodulo {
                margem: M::LobuladaOuIrregular,
                ..base()
            },
            2,
        ),
        (
            Nodulo {
                margem: M::ExtensaoExtratireoidiana,
                ..base()
            },
            3,
        ),
        (
            Nodulo {
                focos: vec![F::NenhumOuCaudaDeCometa],
                ..base()
            },
            0,
        ),
        (
            Nodulo {
                focos: vec![F::Macrocalcificacoes],
                ..base()
            },
            1,
        ),
        (
            Nodulo {
                focos: vec![F::CalcificacoesPerifericas],
                ..base()
            },
            2,
        ),
        (
            Nodulo {
                focos: vec![F::FocosPuntiformes],
                ..base()
            },
            3,
        ),
        // Focos somam entre si (a tabela manda escolher todos os que se aplicam).
        (
            Nodulo {
                focos: vec![F::Macrocalcificacoes, F::FocosPuntiformes],
                ..base()
            },
            4,
        ),
    ];
    for (n, esperado) in casos {
        assert_eq!(tirads::pontos(&n), esperado, "{n:?}");
    }
}

#[test]
fn faixas_de_pontos_para_categoria() {
    let esperado = [
        (0, Categoria::Tr1),
        (1, Categoria::Tr2), // a confirmar (DEC-2026-11-08-64): a tabela não lista 1 ponto
        (2, Categoria::Tr2),
        (3, Categoria::Tr3),
        (4, Categoria::Tr4),
        (5, Categoria::Tr4),
        (6, Categoria::Tr4),
        (7, Categoria::Tr5),
        (8, Categoria::Tr5),
        (14, Categoria::Tr5),
    ];
    for (p, c) in esperado {
        assert_eq!(tirads::categoria(p), c, "{p} pontos");
    }
}

#[test]
fn espongiforme_fica_em_zero_ponto_sem_somar_o_resto() {
    let n = nodulo(
        C::Espongiforme,
        E::Hipoecoica,
        Forma::MaisAltoQueLargo,
        M::LobuladaOuIrregular,
        &[F::FocosPuntiformes],
    );
    let (_, r) = tirads::classificar(&n, mm(30.0)).unwrap();
    assert_eq!((r.pontos, r.categoria), (0, Categoria::Tr1));
    assert_eq!(r.conduta, Conduta::SemPaafNemSeguimento);
}

/// N1 do mockup Laudo-Estruturado: sólido e hipoecoico, 18 mm, TR4 com PAAF (a partir de 15 mm).
#[test]
fn caso_do_mockup_n1_tr4_com_paaf() {
    let n = nodulo(
        C::SolidaOuQuaseSolida,
        E::Hipoecoica,
        Forma::MaisLargoQueAlto,
        M::Lisa,
        &[],
    );
    let (n, r) = tirads::classificar(&n, mm(18.0)).unwrap();
    assert_eq!((r.pontos, r.categoria), (4, Categoria::Tr4));
    assert_eq!(
        r.conduta,
        Conduta::Paaf {
            limite: Medida::de_decimos(150)
        }
    );
    assert_eq!(r.versao, "TIRADS-ACR-2017.1");
    assert_eq!(
        n.focos,
        vec![F::NenhumOuCaudaDeCometa],
        "vazio vira \"nenhum\""
    );
    let texto = tirads::texto_achado("N1", "Lobo direito, terço médio", mm(18.0), &n, &r);
    assert!(
        texto.starts_with("N1: no lobo direito, terço médio, nódulo de 18 mm"),
        "{texto}"
    );
    assert!(texto.ends_with("ACR TI-RADS TR4 (4 pontos)."), "{texto}");
    assert_eq!(
        tirads::texto_conclusao("N1", &r),
        "N1: ACR TI-RADS TR4, moderadamente suspeito; PAAF indicada (a partir de 15 mm)."
    );
}

/// N2 do mockup: cístico e anecoico, 6 mm, TR1 sem conduta.
#[test]
fn caso_do_mockup_n2_tr1() {
    let n = nodulo(
        C::CisticaOuQuaseCistica,
        E::Anecoica,
        Forma::MaisLargoQueAlto,
        M::Lisa,
        &[],
    );
    let (_, r) = tirads::classificar(&n, mm(6.0)).unwrap();
    assert_eq!((r.pontos, r.categoria), (0, Categoria::Tr1));
    assert_eq!(r.conduta, Conduta::SemPaafNemSeguimento);
}

#[test]
fn nodulo_altamente_suspeito_soma_ate_tr5() {
    // Sólido 2 + muito hipoecoico 3 + mais alto que largo 3 + extensão 3 + puntiformes 3 = 14.
    let n = nodulo(
        C::SolidaOuQuaseSolida,
        E::MuitoHipoecoica,
        Forma::MaisAltoQueLargo,
        M::ExtensaoExtratireoidiana,
        &[F::FocosPuntiformes],
    );
    let (_, r) = tirads::classificar(&n, mm(12.0)).unwrap();
    assert_eq!((r.pontos, r.categoria), (14, Categoria::Tr5));
    assert_eq!(
        r.conduta,
        Conduta::Paaf {
            limite: Medida::de_decimos(100)
        }
    );
}

/// Limites exatos de PAAF e seguimento: "a partir de" inclui o próprio valor.
#[test]
fn limites_de_paaf_e_seguimento_por_categoria() {
    let sem = Conduta::SemPaafNemSeguimento;
    let casos = [
        // TR3: PAAF a partir de 25 mm; seguimento a partir de 15 mm.
        (Categoria::Tr3, 25.0, Conduta::Paaf { limite: mm(25.0) }),
        (
            Categoria::Tr3,
            24.9,
            Conduta::Seguimento { limite: mm(15.0) },
        ),
        (
            Categoria::Tr3,
            15.0,
            Conduta::Seguimento { limite: mm(15.0) },
        ),
        (Categoria::Tr3, 14.9, sem),
        // TR4: PAAF a partir de 15 mm; seguimento a partir de 10 mm.
        (Categoria::Tr4, 15.0, Conduta::Paaf { limite: mm(15.0) }),
        (
            Categoria::Tr4,
            14.9,
            Conduta::Seguimento { limite: mm(10.0) },
        ),
        (
            Categoria::Tr4,
            10.0,
            Conduta::Seguimento { limite: mm(10.0) },
        ),
        (Categoria::Tr4, 9.9, sem),
        // TR5: PAAF a partir de 10 mm; seguimento a partir de 5 mm.
        (Categoria::Tr5, 10.0, Conduta::Paaf { limite: mm(10.0) }),
        (Categoria::Tr5, 9.9, Conduta::Seguimento { limite: mm(5.0) }),
        (Categoria::Tr5, 5.0, Conduta::Seguimento { limite: mm(5.0) }),
        (Categoria::Tr5, 4.9, sem),
        // TR1 e TR2 nunca indicam PAAF nem seguimento, mesmo grandes.
        (Categoria::Tr1, 100.0, sem),
        (Categoria::Tr2, 100.0, sem),
    ];
    for (c, medida, esperado) in casos {
        assert_eq!(
            tirads::conduta(c, mm(medida)),
            esperado,
            "{c:?} {medida} mm"
        );
    }
}

#[test]
fn medida_fora_dos_limites_e_recusada() {
    for v in [0.0, -1.0, 0.04, 100.1, f64::NAN, f64::INFINITY] {
        assert_eq!(
            Medida::de_milimetros(v, tirads::LIMITE_MM),
            Err(ErroClassificacao::MedidaInvalida { limite_mm: 100 }),
            "{v}"
        );
    }
    assert_eq!(mm(0.1).decimos(), 1);
    assert_eq!(mm(100.0).decimos(), 1000);
    assert_eq!(mm(8.54).to_string(), "8,5 mm");
}

#[test]
fn descritores_incoerentes_sao_recusados() {
    let nenhum_e_outro = nodulo(
        C::SolidaOuQuaseSolida,
        E::Hipoecoica,
        Forma::MaisLargoQueAlto,
        M::Lisa,
        &[F::NenhumOuCaudaDeCometa, F::Macrocalcificacoes],
    );
    assert_eq!(
        tirads::classificar(&nenhum_e_outro, mm(10.0)).unwrap_err(),
        ErroClassificacao::FocosIncoerentes
    );
    let anecoico_solido = nodulo(
        C::SolidaOuQuaseSolida,
        E::Anecoica,
        Forma::MaisLargoQueAlto,
        M::Lisa,
        &[],
    );
    assert_eq!(
        tirads::classificar(&anecoico_solido, mm(10.0))
            .unwrap_err()
            .codigo(),
        "anecoico-nao-cistico"
    );
}

#[test]
fn focos_repetidos_contam_uma_vez() {
    let n = nodulo(
        C::SolidaOuQuaseSolida,
        E::Hipoecoica,
        Forma::MaisLargoQueAlto,
        M::Lisa,
        &[F::FocosPuntiformes, F::FocosPuntiformes],
    );
    let (n, r) = tirads::classificar(&n, mm(10.0)).unwrap();
    assert_eq!(n.focos, vec![F::FocosPuntiformes]);
    assert_eq!(r.pontos, 7);
}

#[test]
fn codigos_dos_descritores_sao_estaveis() {
    // Os códigos vão para o banco (descritores do achado) e para o contrato da API.
    let n = nodulo(
        C::MistaCisticaESolida,
        E::HiperOuIsoecoica,
        Forma::MaisAltoQueLargo,
        M::ExtensaoExtratireoidiana,
        &[F::CalcificacoesPerifericas],
    );
    assert_eq!(
        serde_json::to_value(&n).unwrap(),
        serde_json::json!({
            "composicao": "mista_cistica_e_solida",
            "ecogenicidade": "hiper_ou_isoecoica",
            "forma": "mais_alto_que_largo",
            "margem": "extensao_extratireoidiana",
            "focos": ["calcificacoes_perifericas"]
        })
    );
    assert!(serde_json::from_value::<Nodulo>(serde_json::json!({
        "composicao": "desconhecida", "ecogenicidade": "anecoica", "forma": "mais_largo_que_alto", "margem": "lisa", "focos": []
    })).is_err(), "valor desconhecido é recusado");
}
