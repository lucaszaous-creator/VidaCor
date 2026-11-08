//! Indicadores da gerência (Indicadores.dc.html; docs/25 MVP: produção, tempo de laudo, faltas e
//! receita). Só números agregados: nenhum paciente identificado sai daqui (docs/18 § 6 item 5).
//! A glosa entra quando o faturamento TISS estiver no ar.

use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Query, State};
use axum::Json;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, IntoParams)]
pub struct Periodo {
    /// Início (AAAA-MM-DD), incluído.
    pub de: NaiveDate,
    /// Fim (AAAA-MM-DD), incluído. No máximo 366 dias.
    pub ate: NaiveDate,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProducaoPorExame {
    pub exame: String,
    pub atendimentos: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LaudoPorMedico {
    pub medico: String,
    pub laudos: i64,
    /// Horas entre o estudo recebido e a assinatura (média). Nulo sem laudo assinado.
    pub horas_medias: Option<f64>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct OcupacaoPorHora {
    /// 0 = domingo.
    pub dia_da_semana: i32,
    pub hora: i32,
    pub agendamentos: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Indicadores {
    pub de: NaiveDate,
    pub ate: NaiveDate,
    pub agendamentos: i64,
    pub atendimentos: i64,
    pub faltas: i64,
    /// Faltas sobre agendamentos que já passaram (0 a 1).
    pub taxa_de_faltas: Option<f64>,
    /// Recebimentos pagos menos estornados, em centavos.
    pub receita_centavos: i64,
    pub recebimentos: i64,
    pub ticket_medio_centavos: Option<i64>,
    pub tempo_medio_de_laudo_horas: Option<f64>,
    pub producao: Vec<ProducaoPorExame>,
    pub por_medico: Vec<LaudoPorMedico>,
    pub ocupacao: Vec<OcupacaoPorHora>,
}

/// Indicadores do período (gerência).
#[utoipa::path(get, path = "/v1/indicadores", tag = "indicadores", params(Periodo),
    responses((status = 200, body = Indicadores), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn indicadores(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(p): Query<Periodo>,
) -> Result<Json<Indicadores>, ErroApi> {
    usuario.exigir(Ato::IndicadoresVer)?;
    if p.ate < p.de || (p.ate - p.de).num_days() > 366 {
        return Err(ErroApi::invalido(
            "periodo-invalido",
            "Escolha um período de até um ano, com o início antes do fim.",
        ));
    }
    let (inicio, _) = tempo::limites_do_dia(p.de)
        .ok_or_else(|| ErroApi::invalido("periodo-invalido", "Data inválida."))?;
    let (_, fim) = tempo::limites_do_dia(p.ate)
        .ok_or_else(|| ErroApi::invalido("periodo-invalido", "Data inválida."))?;
    // Agregados de vários domínios (agenda, laudo, caixa) em contexto de sistema da mesma unidade:
    // a resposta só tem contagens e médias.
    let sistema = ContextoBanco {
        usuario_id: Some(usuario.id),
        unidade_id: usuario.unidade_id,
        perfis: vec!["sistema".into()],
    };
    let mut tx = banco::transacao(&estado.banco, &sistema).await?;
    let u = usuario.unidade_id;
    let ag = sqlx::query!(
        r#"SELECT count(*) AS "total!",
                  count(*) FILTER (WHERE situacao = 'faltou') AS "faltas!",
                  count(*) FILTER (WHERE inicio < now() AND situacao NOT IN ('cancelado','expirada','reserva_provisoria')) AS "passados!"
           FROM agenda.agendamento WHERE unidade_id = $1 AND inicio >= $2 AND inicio < $3 AND situacao NOT IN ('cancelado','expirada')"#,
        u, inicio, fim
    )
    .fetch_one(&mut *tx)
    .await?;
    let producao = sqlx::query_as!(
        ProducaoPorExame,
        r#"SELECT g.exame_nome AS "exame!", count(*) AS "atendimentos!" FROM recepcao.atendimento a JOIN agenda.agendamento g ON g.id = a.agendamento_id
           WHERE a.unidade_id = $1 AND g.inicio >= $2 AND g.inicio < $3 AND a.situacao <> 'cancelado'
           GROUP BY g.exame_nome ORDER BY 2 DESC, 1 LIMIT 30"#,
        u, inicio, fim
    )
    .fetch_all(&mut *tx)
    .await?;
    let atendimentos: i64 = producao.iter().map(|p| p.atendimentos).sum();
    let caixa = sqlx::query!(
        r#"SELECT coalesce(sum(total_centavos) FILTER (WHERE situacao = 'pago'), 0)::bigint AS "receita!",
                  count(*) FILTER (WHERE situacao = 'pago') AS "pagos!"
           FROM caixa.recebimento WHERE unidade_id = $1 AND pago_em >= $2 AND pago_em < $3"#,
        u, inicio, fim
    )
    .fetch_one(&mut *tx)
    .await?;
    let por_medico = sqlx::query!(
        r#"SELECT m.nome AS "medico!", count(*) AS "laudos!",
                  avg(extract(epoch FROM (v.assinada_em - s.recebido_em)) / 3600.0)::float8 AS horas
           FROM laudos.versao v JOIN laudos.laudo l ON l.id = v.laudo_id JOIN exames.estudo s ON s.id = l.estudo_id
           JOIN identidade.usuario m ON m.id = v.assinada_por
           WHERE l.unidade_id = $1 AND v.numero = 1 AND v.assinada_em >= $2 AND v.assinada_em < $3
           GROUP BY m.nome ORDER BY 2 DESC"#,
        u, inicio, fim
    )
    .fetch_all(&mut *tx)
    .await?;
    let ocupacao = sqlx::query_as!(
        OcupacaoPorHora,
        r#"SELECT extract(dow FROM inicio AT TIME ZONE 'America/Sao_Paulo')::int AS "dia_da_semana!",
                  extract(hour FROM inicio AT TIME ZONE 'America/Sao_Paulo')::int AS "hora!", count(*) AS "agendamentos!"
           FROM agenda.agendamento WHERE unidade_id = $1 AND inicio >= $2 AND inicio < $3 AND situacao NOT IN ('cancelado','expirada')
           GROUP BY 1, 2 ORDER BY 1, 2"#,
        u, inicio, fim
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let laudos: i64 = por_medico.iter().map(|m| m.laudos).sum();
    let soma_horas: f64 = por_medico
        .iter()
        .filter_map(|m| m.horas.map(|h| h * m.laudos as f64))
        .sum();
    Ok(Json(Indicadores {
        de: p.de,
        ate: p.ate,
        agendamentos: ag.total,
        atendimentos,
        faltas: ag.faltas,
        taxa_de_faltas: (ag.passados > 0).then(|| ag.faltas as f64 / ag.passados as f64),
        receita_centavos: caixa.receita,
        recebimentos: caixa.pagos,
        ticket_medio_centavos: (caixa.pagos > 0).then(|| caixa.receita / caixa.pagos),
        tempo_medio_de_laudo_horas: (laudos > 0).then(|| soma_horas / laudos as f64),
        producao,
        por_medico: por_medico
            .into_iter()
            .map(|m| LaudoPorMedico {
                medico: m.medico,
                laudos: m.laudos,
                horas_medias: m.horas,
            })
            .collect(),
        ocupacao,
    }))
}
