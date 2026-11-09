//! Tabela de preços do convênio (Convenios-Tabelas.dc.html): reajuste em lote calculado pelo
//! servidor e a prévia que a segunda pessoa confere antes de aprovar. Sem I/O (G1, G5): quem chama
//! traz os preços da versão vigente; a tela nunca manda valor reajustado.

/// Menor e maior reajuste em lote aceitos, em pontos base (1 = 0,01%; 3000 = 30%). Redução de preço
/// ou mudança maior entra como tabela com preços digitados (DEC-2026-11-09-112, a confirmar).
pub const REAJUSTE_MINIMO_PB: i32 = 1;
pub const REAJUSTE_MAXIMO_PB: i32 = 3_000;
/// Maior preço aceito por procedimento (R$ 100.000,00), o mesmo do catálogo do particular.
pub const PRECO_MAXIMO_CENTAVOS: i64 = 10_000_000;
/// Quantas mudanças a prévia destaca (as maiores em reais).
pub const MAIORES_MUDANCAS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecoReajustado {
    pub tuss: String,
    pub antes_centavos: i64,
    pub depois_centavos: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviaDoReajuste {
    /// Todos os preços da versão nova, na ordem recebida.
    pub precos: Vec<PrecoReajustado>,
    pub total_antes_centavos: i64,
    pub total_depois_centavos: i64,
    /// As maiores mudanças em reais (até [`MAIORES_MUDANCAS`]), da maior para a menor.
    pub maiores: Vec<PrecoReajustado>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErroDoReajuste {
    /// Percentual fora de [`REAJUSTE_MINIMO_PB`] a [`REAJUSTE_MAXIMO_PB`].
    PercentualForaDoLimite,
    /// A versão de partida não tem preço nenhum.
    TabelaVazia,
    /// Um preço reajustado passaria do máximo aceito.
    PrecoAcimaDoMaximo(String),
}

/// Aplica o reajuste a um preço, arredondando ao centavo mais próximo (meio centavo sobe).
#[must_use]
pub fn reajustar(valor_centavos: i64, pontos_base: i32) -> i64 {
    let v = i128::from(valor_centavos) * i128::from(10_000 + pontos_base);
    let arredondado = (v + 5_000).div_euclid(10_000);
    i64::try_from(arredondado).unwrap_or(i64::MAX)
}

/// Prévia do reajuste em lote sobre todos os preços da versão de partida.
///
/// # Errors
/// Percentual fora do limite, tabela vazia ou preço reajustado acima do máximo.
pub fn previa(
    precos: &[(String, i64)],
    pontos_base: i32,
) -> Result<PreviaDoReajuste, ErroDoReajuste> {
    if !(REAJUSTE_MINIMO_PB..=REAJUSTE_MAXIMO_PB).contains(&pontos_base) {
        return Err(ErroDoReajuste::PercentualForaDoLimite);
    }
    if precos.is_empty() {
        return Err(ErroDoReajuste::TabelaVazia);
    }
    let mut novos = Vec::with_capacity(precos.len());
    for (tuss, antes) in precos {
        let depois = reajustar(*antes, pontos_base);
        if depois > PRECO_MAXIMO_CENTAVOS {
            return Err(ErroDoReajuste::PrecoAcimaDoMaximo(tuss.clone()));
        }
        novos.push(PrecoReajustado {
            tuss: tuss.clone(),
            antes_centavos: *antes,
            depois_centavos: depois,
        });
    }
    let total_antes_centavos = novos.iter().map(|p| p.antes_centavos).sum();
    let total_depois_centavos = novos.iter().map(|p| p.depois_centavos).sum();
    let mut maiores = novos.clone();
    maiores.sort_by(|a, b| {
        (b.depois_centavos - b.antes_centavos)
            .cmp(&(a.depois_centavos - a.antes_centavos))
            .then_with(|| a.tuss.cmp(&b.tuss))
    });
    maiores.truncate(MAIORES_MUDANCAS);
    Ok(PreviaDoReajuste {
        precos: novos,
        total_antes_centavos,
        total_depois_centavos,
        maiores,
    })
}

#[cfg(test)]
mod testes {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn tabela_unimed() -> Vec<(String, i64)> {
        vec![
            ("40901114".into(), 24_860),
            ("41001230".into(), 61_200),
            ("40901300".into(), 21_460),
            ("40805026".into(), 6_210),
        ]
    }

    #[test]
    fn reajuste_de_5_20_por_cento_do_mockup_arredonda_ao_centavo() {
        // 248,60 × 1,052 = 261,5272 → 261,53; 62,10 × 1,052 = 65,3292 → 65,33.
        assert_eq!(reajustar(24_860, 520), 26_153);
        assert_eq!(reajustar(6_210, 520), 6_533);
        // Meio centavo sobe: 0,50 × 1,01 = 0,505 → 0,51.
        assert_eq!(reajustar(50, 100), 51);
    }

    #[test]
    fn previa_soma_e_destaca_as_maiores_mudancas() {
        let p = previa(&tabela_unimed(), 520).unwrap();
        assert_eq!(p.precos.len(), 4);
        assert_eq!(p.total_antes_centavos, 113_730);
        assert_eq!(
            p.total_depois_centavos,
            p.precos.iter().map(|x| x.depois_centavos).sum::<i64>()
        );
        assert_eq!(
            p.maiores
                .iter()
                .map(|x| x.tuss.as_str())
                .collect::<Vec<_>>(),
            vec!["41001230", "40901114", "40901300"]
        );
    }

    #[test]
    fn limites_do_percentual_e_do_preco() {
        assert_eq!(
            previa(&tabela_unimed(), 0),
            Err(ErroDoReajuste::PercentualForaDoLimite)
        );
        assert_eq!(
            previa(&tabela_unimed(), -100),
            Err(ErroDoReajuste::PercentualForaDoLimite)
        );
        assert_eq!(
            previa(&tabela_unimed(), 3_001),
            Err(ErroDoReajuste::PercentualForaDoLimite)
        );
        assert!(previa(&tabela_unimed(), 3_000).is_ok());
        assert!(previa(&tabela_unimed(), 1).is_ok());
        assert_eq!(previa(&[], 500), Err(ErroDoReajuste::TabelaVazia));
        assert_eq!(
            previa(&[("41001230".into(), 9_900_000)], 200),
            Err(ErroDoReajuste::PrecoAcimaDoMaximo("41001230".into()))
        );
    }
}
