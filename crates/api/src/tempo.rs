//! Tempo: grava em UTC, exibe e calcula o dia em `America/Sao_Paulo` (docs/14 A3).

use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use chrono_tz::America::Sao_Paulo;

#[must_use]
pub fn agora() -> DateTime<Utc> {
    Utc::now()
}

#[must_use]
pub fn hoje_local() -> NaiveDate {
    agora().with_timezone(&Sao_Paulo).date_naive()
}

#[must_use]
pub fn ano_local() -> i32 {
    agora().with_timezone(&Sao_Paulo).year()
}

/// Início e fim (exclusivo) de um dia local, em UTC.
#[must_use]
pub fn limites_do_dia(dia: NaiveDate) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let inicio = Sao_Paulo
        .from_local_datetime(&dia.and_hms_opt(0, 0, 0)?)
        .earliest()?;
    let seguinte = dia.succ_opt()?;
    let fim = Sao_Paulo
        .from_local_datetime(&seguinte.and_hms_opt(0, 0, 0)?)
        .earliest()?;
    Some((inicio.with_timezone(&Utc), fim.with_timezone(&Utc)))
}
