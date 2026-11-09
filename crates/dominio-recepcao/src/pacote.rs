//! Check-up e pacotes (Checkup-Pacotes.dc.html): composição e preço do pacote conferidos pelo
//! servidor (G1). Sem I/O (G5): quem chama traz o preço avulso de cada exame, tirado da tabela do
//! particular vigente no primeiro dia da versão; a tela nunca manda o avulso nem a economia.
//!
//! O pacote vale no orçamento quando sai mais barato que a soma avulsa ([`crate::orcamento::calcular`]);
//! por isso a versão nova só é aceita se o preço for menor que o avulso e o desconto não passar do
//! limite.

/// Exames por pacote (a mesma faixa da coluna `tuss` de `financeiro.pacote`).
pub const EXAMES_MINIMO: usize = 2;
pub const EXAMES_MAXIMO: usize = 10;
/// Maior desconto do pacote sobre a soma avulsa, em pontos base (4000 = 40%). A confirmar com a
/// direção (DEC-2026-11-09-116): o pacote ocupacional do mockup dá 38%.
pub const DESCONTO_MAXIMO_PB: i64 = 4_000;
/// Maior preço de pacote aceito (R$ 100.000,00), contra erro de digitação.
pub const PRECO_MAXIMO_CENTAVOS: i64 = 10_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecoDoPacote {
    pub avulso_centavos: i64,
    pub preco_centavos: i64,
    pub economia_centavos: i64,
    /// Desconto sobre o avulso, em pontos base, arredondado para baixo.
    pub desconto_pb: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErroDoPacote {
    /// Menos de 2 ou mais de 10 exames.
    QuantidadeDeExames,
    ExameRepetido,
    /// O exame não tem preço na tabela do particular da data (não dá para calcular o avulso).
    ExameSemPreco(String),
    /// Preço zero, negativo ou acima do máximo.
    PrecoInvalido,
    /// O pacote não sai mais barato que a soma avulsa (o orçamento não o aplicaria).
    NaoSaiMaisBarato,
    /// O desconto passa do limite.
    DescontoAcimaDoLimite,
}

/// Confere a composição e o preço do pacote contra o avulso de cada exame.
///
/// # Errors
/// Quantidade de exames, exame repetido, exame sem preço avulso, preço inválido, pacote que não sai
/// mais barato ou desconto acima do limite.
pub fn conferir(
    itens: &[(String, Option<i64>)],
    preco_centavos: i64,
) -> Result<PrecoDoPacote, ErroDoPacote> {
    if !(EXAMES_MINIMO..=EXAMES_MAXIMO).contains(&itens.len()) {
        return Err(ErroDoPacote::QuantidadeDeExames);
    }
    let mut vistos: Vec<&str> = itens.iter().map(|(t, _)| t.as_str()).collect();
    vistos.sort_unstable();
    if vistos.windows(2).any(|w| w[0] == w[1]) {
        return Err(ErroDoPacote::ExameRepetido);
    }
    if !(1..=PRECO_MAXIMO_CENTAVOS).contains(&preco_centavos) {
        return Err(ErroDoPacote::PrecoInvalido);
    }
    let mut avulso: i64 = 0;
    for (tuss, valor) in itens {
        match valor {
            Some(v) if *v > 0 => avulso += v,
            _ => return Err(ErroDoPacote::ExameSemPreco(tuss.clone())),
        }
    }
    let r = avaliar(avulso, preco_centavos).ok_or(ErroDoPacote::NaoSaiMaisBarato)?;
    if r.economia_centavos * 10_000 > avulso * DESCONTO_MAXIMO_PB {
        return Err(ErroDoPacote::DescontoAcimaDoLimite);
    }
    Ok(r)
}

/// Economia do pacote para mostrar (sem limite): `None` quando ele não sai mais barato que o avulso,
/// e então o orçamento não o aplica.
#[must_use]
pub fn avaliar(avulso_centavos: i64, preco_centavos: i64) -> Option<PrecoDoPacote> {
    if avulso_centavos <= 0 || preco_centavos <= 0 || preco_centavos >= avulso_centavos {
        return None;
    }
    let economia = avulso_centavos - preco_centavos;
    Some(PrecoDoPacote {
        avulso_centavos,
        preco_centavos,
        economia_centavos: economia,
        desconto_pb: economia * 10_000 / avulso_centavos,
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cardio() -> Vec<(String, Option<i64>)> {
        vec![
            ("40901300".into(), Some(32_000)),
            ("40901114".into(), Some(38_000)),
            ("40101045".into(), Some(19_000)),
        ]
    }

    #[test]
    fn check_up_cardiologico_do_mockup() {
        // Avulso R$ 890,00, pacote R$ 690,00: economia de R$ 200,00 (22,47%).
        let p = conferir(&cardio(), 69_000).unwrap();
        assert_eq!(
            (p.avulso_centavos, p.economia_centavos, p.desconto_pb),
            (89_000, 20_000, 2_247)
        );
    }

    #[test]
    fn pacote_que_nao_sai_mais_barato_ou_passa_do_limite() {
        assert_eq!(
            conferir(&cardio(), 89_000),
            Err(ErroDoPacote::NaoSaiMaisBarato)
        );
        assert_eq!(
            conferir(&cardio(), 90_000),
            Err(ErroDoPacote::NaoSaiMaisBarato)
        );
        // 40% de 890,00 = 356,00: R$ 534,00 é o menor preço aceito.
        assert!(conferir(&cardio(), 53_400).is_ok());
        assert_eq!(
            conferir(&cardio(), 53_399),
            Err(ErroDoPacote::DescontoAcimaDoLimite)
        );
        assert_eq!(conferir(&cardio(), 0), Err(ErroDoPacote::PrecoInvalido));
        assert_eq!(conferir(&cardio(), -100), Err(ErroDoPacote::PrecoInvalido));
    }

    #[test]
    fn composicao_invalida() {
        assert_eq!(
            conferir(&cardio()[..1], 10_000),
            Err(ErroDoPacote::QuantidadeDeExames)
        );
        let onze: Vec<(String, Option<i64>)> = (0..11)
            .map(|k| (format!("409011{k:02}"), Some(10_000)))
            .collect();
        assert_eq!(
            conferir(&onze, 50_000),
            Err(ErroDoPacote::QuantidadeDeExames)
        );
        let mut repetido = cardio();
        repetido.push(("40901114".into(), Some(38_000)));
        assert_eq!(
            conferir(&repetido, 60_000),
            Err(ErroDoPacote::ExameRepetido)
        );
        let mut sem_preco = cardio();
        sem_preco[2].1 = None;
        assert_eq!(
            conferir(&sem_preco, 60_000),
            Err(ErroDoPacote::ExameSemPreco("40101045".into()))
        );
    }

    #[test]
    fn avaliar_sem_vantagem_nao_tem_economia() {
        assert_eq!(avaliar(50_000, 50_000), None);
        assert_eq!(avaliar(0, 1), None);
        assert_eq!(avaliar(50_000, 40_000).map(|p| p.desconto_pb), Some(2_000));
    }
}
