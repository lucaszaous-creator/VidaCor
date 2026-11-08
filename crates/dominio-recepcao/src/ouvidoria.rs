//! Ouvidoria (Ouvidoria.dc.html): prazo de resposta calculado no núcleo conforme o tipo
//! (DEC-2026-11-08-99). Dias úteis contam de segunda a sexta; feriados ficam para quando houver o
//! calendário da unidade (a confirmar com a clínica).

use chrono::{Datelike, Duration, NaiveDate, Weekday};

pub const TIPOS: [&str; 5] = ["reclamacao", "elogio", "sugestao", "denuncia", "lgpd"];
pub const CANAIS: [&str; 4] = ["telefone", "presencial", "email", "portal"];

/// Prazo de resposta: 5 dias úteis (reclamação, elogio, sugestão), 10 dias úteis (denúncia) e
/// 15 dias corridos para o titular (LGPD, art. 19, II).
#[must_use]
pub fn prazo(tipo: &str, recebida: NaiveDate) -> Option<NaiveDate> {
    match tipo {
        "reclamacao" | "elogio" | "sugestao" => Some(somar_dias_uteis(recebida, 5)),
        "denuncia" => Some(somar_dias_uteis(recebida, 10)),
        "lgpd" => Some(recebida + Duration::days(15)),
        _ => None,
    }
}

fn somar_dias_uteis(mut d: NaiveDate, n: u32) -> NaiveDate {
    let mut faltam = n;
    while faltam > 0 {
        d += Duration::days(1);
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            faltam -= 1;
        }
    }
    d
}

#[cfg(test)]
mod testes {
    use super::*;

    fn dia(a: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, d).unwrap()
    }

    #[test]
    fn prazos_por_tipo() {
        // Sexta, 06/11/2026: 5 dias úteis caem na sexta seguinte.
        assert_eq!(
            prazo("reclamacao", dia(2026, 11, 6)),
            Some(dia(2026, 11, 13))
        );
        assert_eq!(prazo("denuncia", dia(2026, 11, 6)), Some(dia(2026, 11, 20)));
        assert_eq!(prazo("lgpd", dia(2026, 11, 6)), Some(dia(2026, 11, 21)));
        // Recebida no sábado: conta a partir da segunda.
        assert_eq!(prazo("elogio", dia(2026, 11, 7)), Some(dia(2026, 11, 13)));
        assert_eq!(prazo("outro", dia(2026, 11, 6)), None);
    }
}
