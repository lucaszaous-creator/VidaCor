//! Início (Main.dc.html): o dia da unidade em números para a recepção e a gerência. Cada número vem
//! de onde o perfil já pode ler (RLS); laudos entram só como contagem agregada; o caixa do dia só para
//! a gerência. Sem número inventado: estacionamento e satisfação não aparecem enquanto não houver
//! dado (DEC-2026-11-08-90).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// Grade aberta por sala em minutos (07:00 às 19:00), a mesma da oferta de vagas.
const MINUTOS_ABERTOS: i64 = 12 * 60;

#[derive(Serialize, ToSchema)]
pub struct ProximoPaciente {
    pub agendamento_id: Uuid,
    pub inicio: DateTime<Utc>,
    pub nome: String,
    pub nascimento: NaiveDate,
    pub exame: String,
    pub sala: String,
    pub situacao: String,
}

#[derive(Serialize, ToSchema)]
pub struct OcupacaoDaSala {
    pub sala: String,
    pub minutos_marcados: i64,
    pub minutos_abertos: i64,
    pub percentual: i64,
}

#[derive(Serialize, ToSchema)]
pub struct CaixaDoDia {
    pub recebido_centavos: i64,
    pub pix_centavos: i64,
    pub cartao_centavos: i64,
    pub dinheiro_centavos: i64,
    pub aguardando_pagamento: i64,
}

#[derive(Serialize, ToSchema)]
pub struct InicioDoDia {
    pub data: NaiveDate,
    pub exames_agendados: i64,
    pub realizados: i64,
    pub aguardando_agora: i64,
    pub faltas: i64,
    pub laudos_a_liberar: i64,
    pub retificacoes_em_rascunho: i64,
    /// Reservas provisórias sem guia (travam o faturamento do convênio).
    pub reservas_sem_guia: i64,
    pub proximos: Vec<ProximoPaciente>,
    pub ocupacao: Vec<OcupacaoDaSala>,
    /// Só para a gerência.
    pub caixa: Option<CaixaDoDia>,
}

/// O dia da unidade (recepção e gerência).
#[utoipa::path(get, operation_id = "inicio_do_dia", path = "/v1/inicio", tag = "agenda",
    responses((status = 200, body = InicioDoDia), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn inicio(State(estado): State<Estado>, usuario: Usuario) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::AgendaAgendar)?;
    let dia = tempo::hoje_local();
    let (de, ate) =
        tempo::limites_do_dia(dia).ok_or_else(|| ErroApi::interno("dia sem limites"))?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let n = sqlx::query!(
        r#"SELECT count(*) FILTER (WHERE situacao NOT IN ('cancelado','expirada')) AS "agendados!",
                  count(*) FILTER (WHERE situacao = 'concluido') AS "realizados!",
                  count(*) FILTER (WHERE situacao = 'faltou') AS "faltas!"
             FROM agenda.agendamento WHERE inicio >= $1 AND inicio < $2"#,
        de,
        ate
    )
    .fetch_one(&mut *tx)
    .await?;
    let aguardando = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM recepcao.atendimento WHERE situacao = 'aguardando_sala' AND criado_em >= $1 AND criado_em < $2"#,
        de,
        ate
    )
    .fetch_one(&mut *tx)
    .await?;
    let reservas = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM agenda.agendamento WHERE situacao = 'reserva_provisoria' AND inicio >= $1"#,
        de
    )
    .fetch_one(&mut *tx)
    .await?;
    let laudos = sqlx::query!(
        r#"SELECT a_liberar AS "a!", em_retificacao AS "r!" FROM laudos.contagem_a_liberar()"#
    )
    .fetch_one(&mut *tx)
    .await?;
    let proximos = sqlx::query_as!(
        ProximoPaciente,
        r#"SELECT a.id AS agendamento_id, a.inicio, p.nome, p.nascimento, a.exame_nome AS exame, s.nome AS sala, a.situacao
             FROM agenda.agendamento a JOIN identidade.paciente p ON p.id = a.paciente_id JOIN agenda.sala s ON s.id = a.sala_id
            WHERE a.inicio >= $1 AND a.inicio < $2 AND a.situacao IN ('agendado','confirmado','reserva_provisoria')
            ORDER BY a.inicio LIMIT 6"#,
        tempo::agora(),
        ate
    )
    .fetch_all(&mut *tx)
    .await?;
    let ocupacao = sqlx::query!(
        r#"SELECT s.nome, coalesce(sum(extract(epoch FROM (a.fim - a.inicio)) / 60), 0)::bigint AS "marcados!"
             FROM agenda.sala s LEFT JOIN agenda.agendamento a ON a.sala_id = s.id AND a.inicio >= $1 AND a.inicio < $2
                  AND a.situacao NOT IN ('cancelado','expirada')
            WHERE s.unidade_id = $3 GROUP BY s.id, s.nome ORDER BY s.nome"#,
        de,
        ate,
        usuario.unidade_id
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|o| OcupacaoDaSala {
        sala: o.nome,
        minutos_marcados: o.marcados,
        minutos_abertos: MINUTOS_ABERTOS,
        percentual: (o.marcados * 100 / MINUTOS_ABERTOS).min(100),
    })
    .collect();
    let caixa = if usuario.tem_perfil("gerente") {
        let c = sqlx::query!(
            r#"SELECT coalesce(sum(total_centavos) FILTER (WHERE situacao = 'pago'), 0)::bigint AS "recebido!",
                      coalesce(sum(total_centavos) FILTER (WHERE situacao = 'pago' AND forma = 'pix'), 0)::bigint AS "pix!",
                      coalesce(sum(total_centavos) FILTER (WHERE situacao = 'pago' AND forma = 'cartao'), 0)::bigint AS "cartao!",
                      coalesce(sum(total_centavos) FILTER (WHERE situacao = 'pago' AND forma = 'dinheiro'), 0)::bigint AS "dinheiro!",
                      count(*) FILTER (WHERE situacao = 'aguardando_pagamento') AS "aguardando!"
                 FROM caixa.recebimento WHERE criado_em >= $1 AND criado_em < $2"#,
            de,
            ate
        )
        .fetch_one(&mut *tx)
        .await?;
        Some(CaixaDoDia {
            recebido_centavos: c.recebido,
            pix_centavos: c.pix,
            cartao_centavos: c.cartao,
            dinheiro_centavos: c.dinheiro,
            aguardando_pagamento: c.aguardando,
        })
    } else {
        None
    };
    tx.commit().await?;
    let corpo = InicioDoDia {
        data: dia,
        exames_agendados: n.agendados,
        realizados: n.realizados,
        aguardando_agora: aguardando,
        faltas: n.faltas,
        laudos_a_liberar: laudos.a,
        retificacoes_em_rascunho: laudos.r,
        reservas_sem_guia: reservas,
        proximos,
        ocupacao,
        caixa,
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(corpo)).into_response())
}
