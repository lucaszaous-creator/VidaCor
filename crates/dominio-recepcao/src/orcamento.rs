//! Orçamento do particular (Orcamento.dc.html). Cálculo puro (G5): quem chama traz os preços da
//! tabela vigente e a regra de pacote; o cliente nunca manda valor (G1).

/// Limite de desconto sem autorização no orçamento, em pontos base (1000 = 10%, mockup). A confirmar
/// com a direção (docs/18 § 9), como o limite do caixa.
pub const LIMITE_DESCONTO_ORCAMENTO_PB: i32 = 1_000;
/// Validade do orçamento em dias (mockup: emitido em 08/11, válido até 22/11).
pub const VALIDADE_DIAS: i64 = 14;
/// Motivos aceitos para desconto (mockup); qualquer outro é recusado.
pub const MOTIVOS_DE_DESCONTO: [&str; 4] = [
    "paciente_frequente",
    "funcionario_ou_familiar",
    "condicao_social",
    "cobrir_outra_clinica",
];

/// Regra de combinação (pacote) com versão.
#[derive(Debug, Clone)]
pub struct Pacote {
    pub codigo: String,
    pub versao: i32,
    pub tuss: Vec<String>,
    pub valor_centavos: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Calculo {
    pub subtotal_centavos: i64,
    pub pacote_centavos: i64,
    pub desconto_centavos: i64,
    pub total_centavos: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErroDoCalculo {
    SemItens,
    ItemRepetido,
    PacoteNaoCabe,
    DescontoInvalido,
}

/// Soma os itens, aplica o pacote (se todos os exames dele estão no orçamento e ele sai mais barato)
/// e depois o desconto percentual sobre o que sobrou, arredondando o desconto para baixo.
///
/// # Errors
/// Orçamento vazio, exame repetido, pacote que não cabe ou desconto fora de 1 a 10000 pontos base.
pub fn calcular(
    itens: &[(String, i64)],
    pacote: Option<&Pacote>,
    desconto_pb: Option<i32>,
) -> Result<Calculo, ErroDoCalculo> {
    if itens.is_empty() {
        return Err(ErroDoCalculo::SemItens);
    }
    let mut vistos: Vec<&str> = itens.iter().map(|(t, _)| t.as_str()).collect();
    vistos.sort_unstable();
    if vistos.windows(2).any(|w| w[0] == w[1]) {
        return Err(ErroDoCalculo::ItemRepetido);
    }
    let subtotal: i64 = itens.iter().map(|(_, v)| *v).sum();
    let pacote_centavos = match pacote {
        None => 0,
        Some(p) => {
            let soma: Option<i64> = p
                .tuss
                .iter()
                .map(|t| itens.iter().find(|(x, _)| x == t).map(|(_, v)| *v))
                .sum();
            match soma {
                Some(s) if !p.tuss.is_empty() && s > p.valor_centavos => s - p.valor_centavos,
                _ => return Err(ErroDoCalculo::PacoteNaoCabe),
            }
        }
    };
    let base = subtotal - pacote_centavos;
    let desconto = match desconto_pb {
        None => 0,
        Some(pb) if (1..=10_000).contains(&pb) => base * i64::from(pb) / 10_000,
        Some(_) => return Err(ErroDoCalculo::DescontoInvalido),
    };
    Ok(Calculo {
        subtotal_centavos: subtotal,
        pacote_centavos,
        desconto_centavos: desconto,
        total_centavos: base - desconto,
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    fn eco_e_carotidas() -> Vec<(String, i64)> {
        vec![("40901114".into(), 30_000), ("40901300".into(), 30_000)]
    }

    fn cardio() -> Pacote {
        Pacote {
            codigo: "PAC-CARDIO".into(),
            versao: 3,
            tuss: vec!["40901114".into(), "40901300".into()],
            valor_centavos: 54_000,
        }
    }

    #[test]
    fn pacote_cardio_do_mockup_tira_60_reais() {
        let c = calcular(&eco_e_carotidas(), Some(&cardio()), None).unwrap();
        assert_eq!(
            (c.subtotal_centavos, c.pacote_centavos, c.total_centavos),
            (60_000, 6_000, 54_000)
        );
    }

    #[test]
    fn desconto_vem_depois_do_pacote_e_arredonda_para_baixo() {
        let c = calcular(&eco_e_carotidas(), Some(&cardio()), Some(333)).unwrap();
        // 3,33% de 540,00 = 17,982 → 17,98.
        assert_eq!((c.desconto_centavos, c.total_centavos), (1_798, 52_202));
    }

    #[test]
    fn pacote_sem_todos_os_exames_nao_vale() {
        let so_eco = vec![("40901114".to_owned(), 30_000)];
        assert_eq!(
            calcular(&so_eco, Some(&cardio()), None),
            Err(ErroDoCalculo::PacoteNaoCabe)
        );
    }

    #[test]
    fn limites() {
        assert_eq!(calcular(&[], None, None), Err(ErroDoCalculo::SemItens));
        let rep = vec![("40901114".to_owned(), 1), ("40901114".to_owned(), 1)];
        assert_eq!(calcular(&rep, None, None), Err(ErroDoCalculo::ItemRepetido));
        for pb in [0, -1, 10_001] {
            assert_eq!(
                calcular(&eco_e_carotidas(), None, Some(pb)),
                Err(ErroDoCalculo::DescontoInvalido)
            );
        }
        let todo = calcular(&eco_e_carotidas(), None, Some(10_000)).unwrap();
        assert_eq!(todo.total_centavos, 0);
    }
}
