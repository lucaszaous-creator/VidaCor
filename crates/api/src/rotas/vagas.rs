//! Vaga que abre vai à lista de espera (docs/27 FC-AGD-08, DEC-2026-11-08-83): cancelamento,
//! bloqueio retirado, reserva provisória vencida e oferta sem aceite enfileiram a vaga no outbox; o
//! trabalhador oferece cada horário ao primeiro pedido compatível da fila, com prazo de resposta.
//! A oferta reserva o horário (gatilho da migration 0061) até ser aceita, recusada ou vencer.

use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::lastro::{self, NovoEvento};
use crate::tempo;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Timelike, Utc};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};
use vidacor_dominio_recepcao::espera::{
    cabe_na_vaga, ordenar, prazo_da_oferta, PedidoNaFila, Preferencia, Prioridade, Vaga,
};

/// Duração do horário da grade (a mesma do agendamento).
pub const DURACAO_MIN: i64 = 30;
/// Prazo de resposta da oferta automática (Lista-Espera.dc.html; a confirmar com a recepção).
pub const PRAZO_AUTOMATICO_MIN: i64 = 20;
/// Teto de horários oferecidos por vaga aberta (um bloqueio longo vira muitos horários).
const MAX_HORARIOS: usize = 400;

/// Origens de vaga aceitas (as mesmas da constraint).
pub const ORIGENS: [&str; 5] = [
    "cancelamento",
    "bloqueio_retirado",
    "reserva_expirada",
    "oferta_sem_aceite",
    "manual",
];

/// Enfileira a vaga para ser oferecida à lista de espera, na mesma transação do fato que a abriu.
/// Repetir com a mesma referência não enfileira de novo (G3).
///
/// # Errors
/// Falha de banco.
#[allow(clippy::too_many_arguments)]
pub async fn enfileirar_vaga(
    conexao: &mut PgConnection,
    unidade_id: Uuid,
    referencia: Uuid,
    sala_id: Uuid,
    inicio: DateTime<Utc>,
    fim: DateTime<Utc>,
    origem: &str,
    correlacao: &str,
) -> Result<(), ErroApi> {
    sqlx::query!(
        "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
         VALUES ($1, 'lista_espera.oferecer_vaga', $2, $3, $4, $5, $6)
         ON CONFLICT (tipo, referencia) WHERE referencia IS NOT NULL DO NOTHING",
        novo_id(),
        novo_id(),
        json!({ "sala_id": sala_id, "inicio": inicio, "fim": fim, "origem": origem }),
        unidade_id,
        correlacao,
        referencia
    )
    .execute(&mut *conexao)
    .await?;
    Ok(())
}

/// Oferta vencida devolvida pela rotina de vencimento.
pub struct OfertaVencida {
    pub id: Uuid,
    pub sala_id: Uuid,
    pub inicio: DateTime<Utc>,
    pub fim: DateTime<Utc>,
}

/// Vence as ofertas sem resposta no prazo (todas, ou só as de um horário): o pedido volta à fila na
/// mesma posição. Operação atômica: só muda o que ainda está aguardando.
///
/// # Errors
/// Falha de banco.
pub async fn vencer(
    conexao: &mut PgConnection,
    unidade_id: Uuid,
    horario: Option<(Uuid, DateTime<Utc>)>,
    correlacao: &str,
) -> Result<Vec<OfertaVencida>, ErroApi> {
    let (sala, inicio) = horario.map_or((None, None), |(s, i)| (Some(s), Some(i)));
    let vencidas = sqlx::query!(
        "UPDATE agenda.oferta SET situacao = 'expirada', motivo = 'sem resposta no prazo', versao = versao + 1
          WHERE unidade_id = $1 AND situacao = 'aguardando' AND expira_em <= now()
            AND ($2::uuid IS NULL OR sala_id = $2) AND ($3::timestamptz IS NULL OR inicio = $3)
          RETURNING id, codigo, espera_id, sala_id, inicio, fim",
        unidade_id,
        sala,
        inicio
    )
    .fetch_all(&mut *conexao)
    .await?;
    let mut saida = Vec::with_capacity(vencidas.len());
    for v in vencidas {
        sqlx::query!(
            "UPDATE agenda.espera SET situacao = 'aguardando', versao = versao + 1 WHERE id = $1 AND situacao = 'ofertado'",
            v.espera_id
        )
        .execute(&mut *conexao)
        .await?;
        lastro::registrar(
            &mut *conexao,
            NovoEvento {
                dominio: "agenda",
                tipo: "oferta_vencida",
                registro_id: v.id,
                registro_codigo: Some(&v.codigo),
                dados: json!({ "motivo": "sem resposta no prazo" }),
                criado_por: None,
                origem: "nucleo",
                unidade_id,
                correlacao,
            },
        )
        .await?;
        saida.push(OfertaVencida {
            id: v.id,
            sala_id: v.sala_id,
            inicio: v.inicio,
            fim: v.fim,
        });
    }
    Ok(saida)
}

/// Dados de uma oferta nova.
pub struct NovaOferta<'a> {
    pub unidade_id: Uuid,
    pub espera_id: Uuid,
    pub sala_id: Uuid,
    pub inicio: DateTime<Utc>,
    pub expira_em: DateTime<Utc>,
    pub origem_vaga: &'a str,
    pub criado_por: Option<Uuid>,
    pub origem: &'a str,
    pub estacao: Option<String>,
    pub correlacao: &'a str,
}

/// Cria a oferta para o pedido de espera. Devolve `None` quando o pedido não está mais aguardando
/// (outra oferta o pegou, ou saiu da lista). O horário é conferido pelo gatilho do banco.
///
/// # Errors
/// `vaga-ja-ofertada` e `horario-ocupado` (409) quando o horário não está livre; falha de banco.
pub async fn criar_oferta(
    conexao: &mut PgConnection,
    n: NovaOferta<'_>,
) -> Result<Option<Uuid>, ErroApi> {
    // Operação atômica conferindo a linha: o mesmo pedido não recebe duas ofertas (G2).
    let pegou = sqlx::query_scalar!(
        "UPDATE agenda.espera SET situacao = 'ofertado', versao = versao + 1
          WHERE id = $1 AND situacao = 'aguardando' AND unidade_id = $2 RETURNING codigo",
        n.espera_id,
        n.unidade_id
    )
    .fetch_optional(&mut *conexao)
    .await?;
    let Some(espera_codigo) = pegou else {
        return Ok(None);
    };
    let ano = tempo::ano_local();
    let numero = sqlx::query_scalar!("SELECT comum.proximo_numero('OFE', $1)", ano)
        .fetch_one(&mut *conexao)
        .await?
        .ok_or_else(|| ErroApi::interno("contador OFE sem valor"))?;
    let codigo = CodigoLegivel::novo("OFE", ano, numero);
    let id = novo_id();
    sqlx::query!(
        "INSERT INTO agenda.oferta (id, codigo, espera_id, sala_id, inicio, fim, expira_em, origem_vaga,
                                    criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        id,
        codigo.como_str(),
        n.espera_id,
        n.sala_id,
        n.inicio,
        n.inicio + Duration::minutes(DURACAO_MIN),
        n.expira_em,
        n.origem_vaga,
        n.criado_por,
        n.origem,
        n.estacao,
        n.unidade_id,
        n.correlacao
    )
    .execute(&mut *conexao)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("oferta_uma_por_horario") => ErroApi::conflito(
            "vaga-ja-ofertada",
            "Este horário acabou de ser oferecido a outro paciente da lista. Nada foi oferecido. Escolha outro horário.",
        ),
        sqlx::Error::Database(d) if d.constraint() == Some("oferta_horario_ocupado") => ErroApi::conflito(
            "horario-ocupado",
            "Este horário não está mais livre (agendamento ou bloqueio). Nada foi oferecido. Escolha outro horário.",
        ),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(&mut *conexao, NovoEvento {
        dominio: "agenda", tipo: "oferta_criada", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "espera": espera_codigo, "sala_id": n.sala_id, "inicio": n.inicio, "expira_em": n.expira_em,
                       "origem_vaga": n.origem_vaga, "estacao": n.estacao }),
        criado_por: n.criado_por, origem: n.origem, unidade_id: n.unidade_id, correlacao: n.correlacao,
    }).await?;
    // Aviso ao paciente pelo WhatsApp, pelo outbox (só primeiro nome, dia, hora e prazo).
    sqlx::query!(
        "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
         VALUES ($1, 'aviso.oferta_vaga', $2, $3, $4, $5, $6) ON CONFLICT (tipo, referencia) WHERE referencia IS NOT NULL DO NOTHING",
        novo_id(), novo_id(), json!({ "oferta_id": id }), n.unidade_id, n.correlacao, id
    )
    .execute(&mut *conexao)
    .await?;
    Ok(Some(id))
}

/// Horários da grade (07:00 às 19:00, de 30 em 30 minutos, horário de Brasília) que tocam o período
/// e ainda não começaram.
#[must_use]
pub fn horarios_da_grade(
    inicio: DateTime<Utc>,
    fim: DateTime<Utc>,
    agora: DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    let mut saida = Vec::new();
    let primeiro: NaiveDate = inicio.with_timezone(&tempo::FUSO).date_naive();
    let ultimo: NaiveDate = fim.with_timezone(&tempo::FUSO).date_naive();
    let mut dia = primeiro;
    while dia <= ultimo && saida.len() < MAX_HORARIOS {
        if let Some(abre) = dia
            .and_hms_opt(7, 0, 0)
            .and_then(|d| tempo::FUSO.from_local_datetime(&d).earliest())
        {
            let mut h = abre.with_timezone(&Utc);
            for _ in 0..24 {
                let h_fim = h + Duration::minutes(DURACAO_MIN);
                if h < fim && h_fim > inicio && h > agora && saida.len() < MAX_HORARIOS {
                    saida.push(h);
                }
                h = h_fim;
            }
        }
        let Some(seguinte) = dia.succ_opt() else {
            break;
        };
        dia = seguinte;
    }
    saida
}

/// Trabalhador do outbox: oferece cada horário da vaga ao primeiro pedido compatível da fila.
///
/// # Errors
/// Carga inválida ou falha de banco (o item fica no outbox e tenta de novo).
pub async fn oferecer_vaga(
    estado: &Estado,
    unidade_id: Uuid,
    carga: &serde_json::Value,
) -> anyhow::Result<()> {
    let sala_id: Uuid = carga["sala_id"]
        .as_str()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("carga sem sala_id"))?;
    let inicio: DateTime<Utc> = carga["inicio"]
        .as_str()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("carga sem inicio"))?;
    let fim: DateTime<Utc> = carga["fim"]
        .as_str()
        .and_then(|v| v.parse().ok())
        .unwrap_or(inicio + Duration::minutes(DURACAO_MIN));
    let origem = carga["origem"].as_str().unwrap_or("reserva_expirada");
    if !ORIGENS.contains(&origem) {
        anyhow::bail!("origem de vaga desconhecida: {origem}");
    }
    let mut ofereceu = false;
    for h in horarios_da_grade(inicio, fim, tempo::agora()) {
        ofereceu |= oferecer_horario(estado, unidade_id, sala_id, h, origem)
            .await
            .map_err(|e| anyhow::anyhow!("oferta da vaga: {}", e.codigo()))?
            .is_some();
    }
    if ofereceu {
        estado.outbox_aviso.notify_one();
    }
    Ok(())
}

/// Oferece um horário ao primeiro pedido compatível que ainda não recebeu este horário.
///
/// # Errors
/// Falha de banco.
pub async fn oferecer_horario(
    estado: &Estado,
    unidade_id: Uuid,
    sala_id: Uuid,
    inicio: DateTime<Utc>,
    origem_vaga: &str,
) -> Result<Option<Uuid>, ErroApi> {
    let correlacao = format!("vaga-{}", inicio.timestamp());
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    vencer(&mut tx, unidade_id, Some((sala_id, inicio)), &correlacao).await?;
    // Trava a sala: agendar, bloquear e oferecer neste horário esperam esta decisão (G2).
    let Some(modalidade) = sqlx::query_scalar!(
        "SELECT modalidade FROM agenda.sala WHERE id = $1 AND unidade_id = $2 FOR UPDATE",
        sala_id,
        unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        tx.commit().await?;
        return Ok(None);
    };
    let fim = inicio + Duration::minutes(DURACAO_MIN);
    let ocupado = sqlx::query_scalar!(
        r#"SELECT (EXISTS (SELECT 1 FROM agenda.agendamento WHERE sala_id = $1 AND situacao NOT IN ('cancelado','faltou','expirada')
                             AND tstzrange(inicio, fim) && tstzrange($2, $3))
                OR EXISTS (SELECT 1 FROM agenda.bloqueio WHERE sala_id = $1 AND situacao = 'ativo'
                             AND tstzrange(inicio, fim) && tstzrange($2, $3))
                OR EXISTS (SELECT 1 FROM agenda.oferta WHERE sala_id = $1 AND situacao = 'aguardando'
                             AND tstzrange(inicio, fim) && tstzrange($2, $3))) AS "ocupado!""#,
        sala_id,
        inicio,
        fim
    )
    .fetch_one(&mut *tx)
    .await?;
    let Some(expira_em) = prazo_da_oferta(tempo::agora(), inicio, PRAZO_AUTOMATICO_MIN) else {
        tx.commit().await?;
        return Ok(None);
    };
    if ocupado {
        tx.commit().await?;
        return Ok(None);
    }
    let candidatos = sqlx::query!(
        "SELECT e.id, e.prioridade, e.preferencia, p.modalidade, e.criado_em
           FROM agenda.espera e JOIN agenda.procedimento p ON p.tuss = e.exame_tuss
          WHERE e.unidade_id = $1 AND e.situacao = 'aguardando'
            AND NOT EXISTS (SELECT 1 FROM agenda.oferta o WHERE o.espera_id = e.id AND o.sala_id = $2 AND o.inicio = $3)
          LIMIT 1000",
        unidade_id,
        sala_id,
        inicio
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut fila = Vec::with_capacity(candidatos.len());
    for c in candidatos {
        fila.push(PedidoNaFila {
            id: c.id,
            prioridade: Prioridade::de_str(&c.prioridade).map_err(ErroApi::interno)?,
            preferencia: Preferencia::de_str(&c.preferencia).map_err(ErroApi::interno)?,
            modalidade: c.modalidade,
            desde: c.criado_em,
        });
    }
    ordenar(&mut fila);
    let vaga = Vaga {
        modalidade_sala: &modalidade,
        hora_local: inicio.with_timezone(&tempo::FUSO).hour(),
    };
    for pedido in fila.iter().filter(|p| cabe_na_vaga(p, &vaga)) {
        let criada = criar_oferta(
            &mut tx,
            NovaOferta {
                unidade_id,
                espera_id: pedido.id,
                sala_id,
                inicio,
                expira_em,
                origem_vaga,
                criado_por: None,
                origem: "nucleo",
                estacao: None,
                correlacao: &correlacao,
            },
        )
        .await?;
        if criada.is_some() {
            tx.commit().await?;
            return Ok(criada);
        }
    }
    tx.commit().await?;
    Ok(None)
}

/// Vence as ofertas sem resposta e enfileira o horário para o próximo da fila.
///
/// # Errors
/// Falha de banco.
pub async fn expirar_ofertas(estado: &Estado, unidade_id: Uuid) -> Result<u64, ErroApi> {
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    let vencidas = vencer(&mut tx, unidade_id, None, "vencimento-de-oferta").await?;
    for v in &vencidas {
        if v.inicio > tempo::agora() {
            enfileirar_vaga(
                &mut tx,
                unidade_id,
                v.id,
                v.sala_id,
                v.inicio,
                v.fim,
                "oferta_sem_aceite",
                "vencimento-de-oferta",
            )
            .await?;
        }
    }
    tx.commit().await?;
    if !vencidas.is_empty() {
        estado.outbox_aviso.notify_one();
    }
    Ok(vencidas.len() as u64)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn grade_so_tem_horarios_futuros_dentro_do_expediente() {
        // 09/11/2026, 10:15 às 11:45 em Brasília (13:15 a 14:45 UTC).
        let ini = Utc.with_ymd_and_hms(2026, 11, 9, 13, 15, 0).unwrap();
        let fim = Utc.with_ymd_and_hms(2026, 11, 9, 14, 45, 0).unwrap();
        let antes = Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).unwrap();
        let h: Vec<String> = horarios_da_grade(ini, fim, antes)
            .iter()
            .map(|d| d.with_timezone(&tempo::FUSO).format("%H:%M").to_string())
            .collect();
        assert_eq!(h, vec!["10:00", "10:30", "11:00", "11:30"]);
        // Depois das 10:40, só os que não começaram.
        let agora = Utc.with_ymd_and_hms(2026, 11, 9, 13, 40, 0).unwrap();
        assert_eq!(horarios_da_grade(ini, fim, agora).len(), 2);
        // Um dia inteiro tem 24 horários.
        let d0 = Utc.with_ymd_and_hms(2026, 11, 10, 3, 0, 0).unwrap();
        assert_eq!(
            horarios_da_grade(d0, d0 + Duration::days(1), antes).len(),
            24
        );
    }
}
