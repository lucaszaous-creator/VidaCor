//! Triagem de segurança para exame com contraste iodado (docs/22 E1, docs/27 FC-TRI). Sem I/O (G5).
//!
//! Regra `TRI-2026.1` (a confirmar pelo responsável clínico de segurança, DEC-2026-11-08-50):
//! - reação prévia a contraste: precisa de aprovação médica;
//! - gestação: precisa de aprovação médica;
//! - TFG abaixo de 45 mL/min/1,73 m²: precisa de aprovação médica;
//! - TFG abaixo de 45 com metformina: motivo adicional (suspender metformina);
//! - TFG de 45 a 59: liberado com ressalvas (hidratação);
//! - amamentação: liberado com ressalvas (orientação);
//! - sem creatinina recente (até 90 dias) e idade a partir de 60 anos: precisa de aprovação médica.
//!
//! A TFG é sempre calculada aqui (CKD-EPI 2021, sem raça), nunca aceita da tela (FC-TRI-07).

#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]

use serde::Serialize;

pub const REGRA_VERSAO: &str = "TRI-2026.1";
const LIMITE_APROVACAO: u32 = 45;
const LIMITE_RESSALVA: u32 = 60;
const VALIDADE_CREATININA_DIAS: i64 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sexo {
    Feminino,
    Masculino,
}

/// TFG estimada pela CKD-EPI 2021 em mL/min/1,73 m², arredondada para baixo.
#[must_use]
pub fn tfg_ckd_epi_2021(creatinina_mg_dl: f64, idade_anos: u32, sexo: Sexo) -> u32 {
    let (kappa, alfa, fator_sexo) = match sexo {
        Sexo::Feminino => (0.7, -0.241, 1.012),
        Sexo::Masculino => (0.9, -0.302, 1.0),
    };
    let razao = creatinina_mg_dl / kappa;
    let tfg = 142.0
        * razao.min(1.0).powf(alfa)
        * razao.max(1.0).powf(-1.200)
        * 0.9938_f64.powi(i32::try_from(idade_anos).unwrap_or(i32::MAX))
        * fator_sexo;
    if tfg.is_finite() && tfg > 0.0 {
        // Valores positivos e finitos cabem em u32 depois do piso.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let piso = tfg.floor() as u32;
        piso
    } else {
        0
    }
}

#[derive(Debug, Clone)]
pub struct DadosTriagem {
    pub reacao_previa_contraste: bool,
    pub gestante: bool,
    pub amamentando: bool,
    pub usa_metformina: bool,
    /// Creatinina e idade do exame em dias.
    pub creatinina: Option<(f64, i64)>,
    pub idade_anos: u32,
    pub sexo: Sexo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Resultado {
    Liberado,
    LiberadoComRessalvas,
    PrecisaAprovacaoMedica,
}

impl Resultado {
    #[must_use]
    pub fn como_str(self) -> &'static str {
        match self {
            Self::Liberado => "liberado",
            Self::LiberadoComRessalvas => "liberado_com_ressalvas",
            Self::PrecisaAprovacaoMedica => "precisa_aprovacao_medica",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Avaliacao {
    pub resultado: Resultado,
    pub tfg: Option<u32>,
    pub motivos: Vec<String>,
}

/// Avalia a triagem pela regra vigente.
#[must_use]
pub fn avaliar(d: &DadosTriagem) -> Avaliacao {
    let mut aprovacao = Vec::new();
    let mut ressalvas = Vec::new();
    if d.reacao_previa_contraste {
        aprovacao.push("reação prévia a contraste".to_owned());
    }
    if d.gestante {
        aprovacao.push("gestação".to_owned());
    }
    let tfg = match d.creatinina {
        Some((valor, dias)) if dias <= VALIDADE_CREATININA_DIAS && valor > 0.0 => {
            Some(tfg_ckd_epi_2021(valor, d.idade_anos, d.sexo))
        }
        _ => None,
    };
    match tfg {
        Some(t) if t < LIMITE_APROVACAO => {
            aprovacao.push(format!("TFG {t} abaixo de {LIMITE_APROVACAO}"));
            if d.usa_metformina {
                aprovacao.push("metformina com TFG abaixo de 45: avaliar suspensão".to_owned());
            }
        }
        Some(t) if t < LIMITE_RESSALVA => {
            ressalvas.push(format!("TFG {t} entre 45 e 59: hidratação"))
        }
        Some(_) => {}
        None if d.idade_anos >= 60 => {
            aprovacao.push("sem creatinina dos últimos 90 dias aos 60 anos ou mais".to_owned());
        }
        None => {}
    }
    if d.amamentando {
        ressalvas.push("amamentação: orientar a paciente".to_owned());
    }
    let resultado = if !aprovacao.is_empty() {
        Resultado::PrecisaAprovacaoMedica
    } else if !ressalvas.is_empty() {
        Resultado::LiberadoComRessalvas
    } else {
        Resultado::Liberado
    };
    aprovacao.extend(ressalvas);
    Avaliacao {
        resultado,
        tfg,
        motivos: aprovacao,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn base() -> DadosTriagem {
        DadosTriagem {
            reacao_previa_contraste: false,
            gestante: false,
            amamentando: false,
            usa_metformina: false,
            creatinina: Some((0.8, 10)),
            idade_anos: 69,
            sexo: Sexo::Feminino,
        }
    }

    #[test]
    fn tfg_da_frase_fc_tri_02() {
        // Vera Lúcia Antunes, 69 anos, feminino, creatinina 1,4 mg/dL: TFG 40 (docs/27).
        assert_eq!(tfg_ckd_epi_2021(1.4, 69, Sexo::Feminino), 40);
    }

    #[test]
    fn tfg_valores_de_referencia() {
        assert_eq!(tfg_ckd_epi_2021(0.7, 40, Sexo::Feminino), 112);
        assert_eq!(tfg_ckd_epi_2021(1.0, 50, Sexo::Masculino), 91);
    }

    #[test]
    fn sem_risco_libera() {
        let a = avaliar(&base());
        assert_eq!(a.resultado, Resultado::Liberado);
        assert!(a.motivos.is_empty());
    }

    #[test]
    fn tfg_baixa_exige_aprovacao() {
        let a = avaliar(&DadosTriagem {
            creatinina: Some((1.4, 11)),
            ..base()
        });
        assert_eq!(a.resultado, Resultado::PrecisaAprovacaoMedica);
        assert_eq!(a.tfg, Some(40));
    }

    #[test]
    fn reacao_previa_exige_aprovacao() {
        let a = avaliar(&DadosTriagem {
            reacao_previa_contraste: true,
            ..base()
        });
        assert_eq!(a.resultado, Resultado::PrecisaAprovacaoMedica);
    }

    #[test]
    fn creatinina_velha_aos_60_exige_aprovacao() {
        let a = avaliar(&DadosTriagem {
            creatinina: Some((0.8, 200)),
            ..base()
        });
        assert_eq!(a.resultado, Resultado::PrecisaAprovacaoMedica);
        assert_eq!(a.tfg, None);
    }

    #[test]
    fn amamentando_libera_com_ressalvas() {
        let a = avaliar(&DadosTriagem {
            amamentando: true,
            idade_anos: 30,
            ..base()
        });
        assert_eq!(a.resultado, Resultado::LiberadoComRessalvas);
    }
}
