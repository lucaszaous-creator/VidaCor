//! Casos do BI-RADS 5ª edição, ultrassom de mama (versão `BIRADS-ACR-2013-US.1`).
//!
//! AGUARDAM CONFERÊNCIA POR MÉDICO (docs/25 § 3.3): as regras de coerência e os textos de conduta
//! foram transcritos do ACR BI-RADS Atlas (2013) por quem escreveu o código; nenhum médico os
//! conferiu ainda. Os pontos "a confirmar" estão no DEC-2026-11-08-64.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use vidacor_dominio_laudo::birads::{
    self, Achado, Categoria as K, Forma, Lado, Margem as G, Massa, Orientacao, PadraoEcogenico,
    Posterior,
};
use vidacor_dominio_laudo::{ErroClassificacao as Erro, Medida};

fn mm(v: f64) -> Medida {
    Medida::de_milimetros(v, birads::LIMITE_MM).unwrap()
}

/// Massa de aspecto provavelmente benigno (oval, paralela, circunscrita, hipoecoica).
fn benigna() -> Massa {
    Massa {
        forma: Forma::Oval,
        orientacao: Orientacao::Paralela,
        margens: vec![G::Circunscrita],
        padrao_ecogenico: PadraoEcogenico::Hipoecoico,
        posterior: Posterior::Nenhum,
    }
}

/// Massa suspeita (irregular, não paralela, espiculada, com sombra).
fn suspeita() -> Massa {
    Massa {
        forma: Forma::Irregular,
        orientacao: Orientacao::NaoParalela,
        margens: vec![G::Espiculada, G::Angular],
        padrao_ecogenico: PadraoEcogenico::Hipoecoico,
        posterior: Posterior::Sombra,
    }
}

fn achado(categoria: K, massa: Option<Massa>, biopsia: bool) -> Achado {
    Achado {
        lado: Lado::Esquerda,
        categoria_atribuida: categoria,
        biopsia_maligna_confirmada: biopsia,
        massa,
    }
}

#[test]
fn categoria_3_com_massa_benigna_e_aceita_com_conduta_da_tabela() {
    let (_, r) =
        birads::classificar(&achado(K::Tres, Some(benigna()), false), Some(mm(12.0))).unwrap();
    assert_eq!(r.categoria, K::Tres);
    assert!(r.suspeitas.is_empty());
    assert_eq!(r.versao, "BIRADS-ACR-2013-US.1");
    assert_eq!(
        r.categoria.conduta(),
        "seguimento de curto intervalo (6 meses) ou vigilância"
    );
    assert_eq!(r.categoria.probabilidade(), "acima de 0% e até 2%");
}

#[test]
fn seis_exige_biopsia_confirmada() {
    let sem = birads::classificar(&achado(K::Seis, Some(suspeita()), false), Some(mm(20.0)));
    assert_eq!(sem.unwrap_err(), Erro::Birads6SemBiopsia);
    let (_, r) =
        birads::classificar(&achado(K::Seis, Some(suspeita()), true), Some(mm(20.0))).unwrap();
    assert_eq!(
        r.categoria.conduta(),
        "excisão cirúrgica quando clinicamente apropriada"
    );
}

#[test]
fn massa_suspeita_nao_pode_ser_2_nem_3() {
    for k in [K::Dois, K::Tres] {
        let e =
            birads::classificar(&achado(k, Some(suspeita()), false), Some(mm(15.0))).unwrap_err();
        assert_eq!(e.codigo(), "categoria-incoerente", "{k:?}");
        assert!(
            e.to_string()
                .contains("forma irregular, orientação não paralela, margem não circunscrita"),
            "{e}"
        );
    }
    // Uma só característica suspeita já basta (margem indistinta).
    let indistinta = Massa {
        margens: vec![G::Indistinta],
        ..benigna()
    };
    assert!(birads::classificar(&achado(K::Tres, Some(indistinta), false), Some(mm(9.0))).is_err());
}

#[test]
fn massa_suspeita_aceita_4_e_5() {
    for k in [K::QuatroA, K::QuatroB, K::QuatroC, K::Cinco] {
        let (_, r) =
            birads::classificar(&achado(k, Some(suspeita()), false), Some(mm(15.0))).unwrap();
        assert_eq!(
            r.categoria.conduta(),
            "diagnóstico tecidual (biópsia)",
            "{k:?}"
        );
    }
}

#[test]
fn cinco_exige_caracteristica_suspeita() {
    let e =
        birads::classificar(&achado(K::Cinco, Some(benigna()), false), Some(mm(15.0))).unwrap_err();
    assert_eq!(e, Erro::Categoria5SemSuspeita);
    // 4A com massa circunscrita é decisão do médico (por exemplo, crescimento): aceita.
    assert!(
        birads::classificar(&achado(K::QuatroA, Some(benigna()), false), Some(mm(15.0))).is_ok()
    );
}

#[test]
fn negativo_nao_admite_massa_e_sem_massa_nao_ha_medida() {
    assert_eq!(
        birads::classificar(&achado(K::Um, Some(benigna()), false), Some(mm(5.0))).unwrap_err(),
        Erro::NegativoComMassa
    );
    assert!(birads::classificar(&achado(K::Um, None, false), None).is_ok());
    assert_eq!(
        birads::classificar(&achado(K::Um, None, false), Some(mm(5.0))).unwrap_err(),
        Erro::MedidaSemMassa
    );
    // Zero (incompleto) admite registro sem massa.
    assert!(birads::classificar(&achado(K::Zero, None, false), None).is_ok());
}

#[test]
fn categorias_de_achado_exigem_massa_e_medida() {
    for k in [
        K::Dois,
        K::Tres,
        K::QuatroA,
        K::QuatroB,
        K::QuatroC,
        K::Cinco,
    ] {
        let e = birads::classificar(&achado(k, None, false), None).unwrap_err();
        assert_eq!(e.codigo(), "massa-ausente", "{k:?}");
        let e = birads::classificar(&achado(k, Some(suspeita()), false), None).unwrap_err();
        assert_eq!(e.codigo(), "massa-ausente", "{k:?} sem medida");
    }
}

#[test]
fn margens_incoerentes_ou_vazias_sao_recusadas() {
    let mistas = Massa {
        margens: vec![G::Circunscrita, G::Espiculada],
        ..benigna()
    };
    assert_eq!(
        birads::classificar(&achado(K::QuatroB, Some(mistas), false), Some(mm(10.0))).unwrap_err(),
        Erro::MargensIncoerentes
    );
    let vazias = Massa {
        margens: vec![],
        ..benigna()
    };
    assert_eq!(
        birads::classificar(&achado(K::QuatroB, Some(vazias), false), Some(mm(10.0))).unwrap_err(),
        Erro::MargemAusente
    );
}

#[test]
fn limite_de_medida_da_mama() {
    assert!(Medida::de_milimetros(150.0, birads::LIMITE_MM).is_ok());
    assert!(Medida::de_milimetros(150.1, birads::LIMITE_MM).is_err());
}

#[test]
fn texto_sugerido_traz_lado_descritores_e_categoria() {
    let a = achado(K::QuatroC, Some(suspeita()), false);
    let (a, r) = birads::classificar(&a, Some(mm(14.0))).unwrap();
    let t = birads::texto_achado(
        "N1",
        "Quadrante superolateral, 2 horas",
        Some(mm(14.0)),
        &a,
        &r,
    );
    assert_eq!(
        t,
        "N1: mama esquerda, quadrante superolateral, 2 horas, massa hipoecoica, irregular, não paralela à pele, de margens angulares e espiculadas, com sombra acústica posterior, medindo 14 mm no maior eixo. BI-RADS 4C."
    );
    assert_eq!(
        birads::texto_conclusao("N1", &r),
        "N1: BI-RADS 4C, alta suspeita de malignidade; conduta: diagnóstico tecidual (biópsia)."
    );
}

#[test]
fn codigos_da_categoria_sao_os_do_atlas() {
    let v = serde_json::to_value([K::Zero, K::QuatroA, K::Seis]).unwrap();
    assert_eq!(v, serde_json::json!(["0", "4A", "6"]));
    assert!(serde_json::from_value::<K>(serde_json::json!("7")).is_err());
}
