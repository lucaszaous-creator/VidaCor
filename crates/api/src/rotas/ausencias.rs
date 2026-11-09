//! Ausência do médico (Agenda-Medico.dc.html, "Bloquear horário"; DEC-2026-11-09-141 e 142): o médico
//! pede o bloqueio da própria agenda com motivo; a coordenação (gerência) aprova ou recusa. Enquanto
//! aguarda, a agenda continua aberta. Aprovado, nenhum agendamento novo com o médico entra no período
//! (gatilho do banco com trava por médico). Nada é apagado: cancelar e recusar são situações com motivo.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::novo_id;

const MOTIVOS: [&str; 4] = ["congresso", "ferias", "licenca", "pessoal"];

#[derive(Serialize, ToSchema)]
pub struct AusenciaDoMedico {
    pub id: Uuid,
    pub codigo: String,
    pub medico_id: Uuid,
    pub medico: String,
    pub inicio: DateTime<Utc>,
    pub fim: DateTime<Utc>,
    /// congresso, ferias, licenca ou pessoal.
    pub motivo: String,
    pub detalhe: Option<String>,
    pub afetados_no_pedido: i32,
    /// aguardando, aprovada, recusada ou cancelada.
    pub situacao: String,
    pub decidido_por: Option<String>,
    pub decidido_em: Option<DateTime<Utc>>,
    pub motivo_decisao: Option<String>,
    pub afetados_na_decisao: Option<i32>,
    pub criado_em: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct NovaAusencia {
    pub inicio: DateTime<Utc>,
    pub fim: DateTime<Utc>,
    /// congresso, ferias, licenca ou pessoal.
    pub motivo: String,
    /// Opcional, 3 a 300 caracteres.
    pub detalhe: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct DecisaoDaAusencia {
    pub aprovar: bool,
    /// Obrigatório para recusar (5 a 300 caracteres).
    pub motivo: Option<String>,
}

#[derive(Deserialize, IntoParams)]
pub struct FiltroAusencias {
    /// aguardando, aprovada, recusada ou cancelada. Sem filtro: todas as dos últimos 90 dias.
    pub situacao: Option<String>,
}

/// Período e motivo válidos (puro, testado abaixo).
///
/// # Errors
/// Texto para a pessoa com o que corrigir.
pub fn conferir_pedido(n: &NovaAusencia, agora: DateTime<Utc>) -> Result<(), &'static str> {
    if !MOTIVOS.contains(&n.motivo.as_str()) {
        return Err("Escolha o motivo do bloqueio. Nada foi enviado.");
    }
    if n.fim <= n.inicio {
        return Err("O fim precisa ser depois do início. Nada foi enviado.");
    }
    if n.fim - n.inicio > Duration::days(62) {
        return Err("O bloqueio vai até 62 dias por pedido. Para mais tempo, faça outro pedido depois. Nada foi enviado.");
    }
    if n.fim <= agora {
        return Err("O período já passou. Escolha datas a partir de agora. Nada foi enviado.");
    }
    if n.detalhe
        .as_deref()
        .map(str::trim)
        .is_some_and(|d| !d.is_empty() && !(3..=300).contains(&d.chars().count()))
    {
        return Err("O detalhe tem de 3 a 300 caracteres. Nada foi enviado.");
    }
    Ok(())
}

async fn carregar(tx: &mut sqlx::PgConnection, id: Uuid) -> Result<AusenciaDoMedico, ErroApi> {
    sqlx::query_as!(
        AusenciaDoMedico,
        r#"SELECT x.id, x.codigo, x.medico_id, m.nome AS medico, x.inicio, x.fim, x.motivo, x.detalhe, x.afetados_no_pedido,
                  x.situacao, d.nome AS "decidido_por?", x.decidido_em, x.motivo_decisao, x.afetados_na_decisao, x.criado_em
             FROM agenda.ausencia_medico x
             JOIN identidade.usuario m ON m.id = x.medico_id
             LEFT JOIN identidade.usuario d ON d.id = x.decidido_por
            WHERE x.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(nao_encontrada)
}

fn nao_encontrada() -> ErroApi {
    ErroApi::nao_encontrado(
        "ausencia-nao-encontrada",
        "Este pedido de bloqueio não existe ou não é seu.",
    )
}

/// Pedido de bloqueio da própria agenda (médico; `Idempotency-Key`). O núcleo conta os pacientes já
/// agendados no período; a agenda só fecha depois da aprovação.
#[utoipa::path(post, operation_id = "pedir_ausencia", path = "/v1/minha-agenda/ausencias", tag = "agenda",
    params(("Idempotency-Key" = Uuid, Header)), request_body = NovaAusencia,
    responses((status = 201, body = AusenciaDoMedico), (status = 403, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn pedir(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovaAusencia>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::AusenciaPedir)?;
    conferir_pedido(&n, tempo::agora()).map_err(|m| ErroApi::invalido("ausencia-invalida", m))?;
    let detalhe = n
        .detalhe
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(str::to_owned);
    let rota = "POST /v1/minha-agenda/ausencias";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, rota, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let afetados = sqlx::query_scalar!(
        "SELECT agenda.afetados_pela_ausencia($1, $2, $3)",
        usuario.id,
        n.inicio,
        n.fim
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);
    let id = novo_id();
    let codigo = proximo(&mut tx, "AUS").await?;
    sqlx::query!(
        "INSERT INTO agenda.ausencia_medico (id, codigo, medico_id, inicio, fim, motivo, detalhe, afetados_no_pedido,
                criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $3, 'minha_agenda', $9, $10)",
        id, codigo.como_str(), usuario.id, n.inicio, n.fim, n.motivo, detalhe, afetados, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("ausencia_sem_sobreposicao") => ErroApi::conflito(
            "ausencia-sobreposta",
            "Você já tem um pedido aguardando ou aprovado neste período. Nada foi enviado. Confira os seus pedidos.",
        ),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "agenda",
            tipo: "ausencia_pedida",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "medico_id": usuario.id, "inicio": n.inicio, "fim": n.fim, "motivo": n.motivo, "afetados": afetados }),
            criado_por: Some(usuario.id),
            origem: "minha_agenda",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let a = carregar(&mut tx, id).await?;
    let corpo = serde_json::to_value(a).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Pedidos de bloqueio: o médico vê os seus; a gerência e a recepção, os da unidade.
#[utoipa::path(get, operation_id = "listar_ausencias", path = "/v1/ausencias", tag = "agenda", params(FiltroAusencias),
    responses((status = 200, body = Vec<AusenciaDoMedico>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroAusencias>,
) -> Result<Json<Vec<AusenciaDoMedico>>, ErroApi> {
    if !(usuario.pode(Ato::AusenciaPedir) || usuario.pode(Ato::AusenciaVer)) {
        return Err(ErroApi::proibido());
    }
    let so_meus = !usuario.pode(Ato::AusenciaVer);
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = sqlx::query_as!(
        AusenciaDoMedico,
        r#"SELECT x.id, x.codigo, x.medico_id, m.nome AS medico, x.inicio, x.fim, x.motivo, x.detalhe, x.afetados_no_pedido,
                  x.situacao, d.nome AS "decidido_por?", x.decidido_em, x.motivo_decisao, x.afetados_na_decisao, x.criado_em
             FROM agenda.ausencia_medico x
             JOIN identidade.usuario m ON m.id = x.medico_id
             LEFT JOIN identidade.usuario d ON d.id = x.decidido_por
            WHERE ($1::text IS NULL OR x.situacao = $1)
              AND (NOT $2 OR x.medico_id = $3)
              AND (x.situacao = 'aguardando' OR x.fim >= now() - interval '90 days')
            ORDER BY (x.situacao = 'aguardando') DESC, x.inicio
            LIMIT 200"#,
        f.situacao,
        so_meus,
        usuario.id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(r))
}

fn ja_decidida() -> ErroApi {
    ErroApi::conflito(
        "ausencia-ja-decidida",
        "Este pedido já foi decidido ou cancelado por outra pessoa. Nada foi alterado. Recarregue para ver a situação.",
    )
}

/// Aprova ou recusa o pedido (gerência, nunca o próprio médico; `Idempotency-Key`). A aprovação recontra
/// os pacientes agendados no período sob a mesma trava do agendamento.
#[utoipa::path(post, operation_id = "decidir_ausencia", path = "/v1/ausencias/{id}/decisao", tag = "agenda",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = DecisaoDaAusencia,
    responses((status = 200, body = AusenciaDoMedico), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn decidir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(d): Corpo<DecisaoDaAusencia>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::AusenciaDecidir)?;
    let motivo = d
        .motivo
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_owned);
    if motivo
        .as_ref()
        .is_some_and(|m| !(5..=300).contains(&m.chars().count()))
        || (!d.aprovar && motivo.is_none())
    {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Para recusar, diga o motivo ao médico (5 a 300 caracteres). Nada foi alterado.",
        ));
    }
    let rota = format!("POST /v1/ausencias/{id}/decisao");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let pedido = sqlx::query!(
        "SELECT codigo, medico_id, inicio, fim, situacao FROM agenda.ausencia_medico WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(nao_encontrada)?;
    if pedido.medico_id == usuario.id {
        return Err(ErroApi::proibido());
    }
    if pedido.situacao != "aguardando" {
        return Err(ja_decidida());
    }
    let afetados = if d.aprovar {
        sqlx::query!("SELECT agenda.travar_medico($1)", pedido.medico_id)
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query_scalar!(
            "SELECT agenda.afetados_pela_ausencia($1, $2, $3)",
            pedido.medico_id,
            pedido.inicio,
            pedido.fim
        )
        .fetch_one(&mut *tx)
        .await?
    } else {
        None
    };
    let situacao = if d.aprovar { "aprovada" } else { "recusada" };
    let feito = sqlx::query!(
        "UPDATE agenda.ausencia_medico
            SET situacao = $2, decidido_por = $3, decidido_em = now(), motivo_decisao = $4, afetados_na_decisao = $5,
                versao = versao + 1
          WHERE id = $1 AND situacao = 'aguardando'",
        id,
        situacao,
        usuario.id,
        motivo,
        afetados
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db) if db.constraint() == Some("ausencia_sem_sobreposicao") => ja_decidida(),
        _ => ErroApi::from(e),
    })?;
    if feito.rows_affected() != 1 {
        return Err(ja_decidida());
    }
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "agenda",
            tipo: if d.aprovar {
                "ausencia_aprovada"
            } else {
                "ausencia_recusada"
            },
            registro_id: id,
            registro_codigo: Some(&pedido.codigo),
            dados: json!({ "medico_id": pedido.medico_id, "afetados": afetados, "motivo": motivo }),
            criado_por: Some(usuario.id),
            origem: "minha_agenda",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let a = carregar(&mut tx, id).await?;
    let corpo = serde_json::to_value(a).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 200, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::OK, Json(corpo)).into_response())
}

/// O médico desiste do próprio pedido enquanto aguarda (`Idempotency-Key`).
#[utoipa::path(post, operation_id = "cancelar_ausencia", path = "/v1/ausencias/{id}/cancelamento", tag = "agenda",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)),
    responses((status = 200, body = AusenciaDoMedico), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn cancelar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::AusenciaPedir)?;
    let rota = format!("POST /v1/ausencias/{id}/cancelamento");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let pedido = sqlx::query!(
        "SELECT codigo, medico_id, situacao FROM agenda.ausencia_medico WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(nao_encontrada)?;
    // Pedido de outro médico: mesma resposta de inexistente (sem oráculo de enumeração).
    if pedido.medico_id != usuario.id {
        return Err(nao_encontrada());
    }
    let feito = sqlx::query!(
        "UPDATE agenda.ausencia_medico SET situacao = 'cancelada', versao = versao + 1 WHERE id = $1 AND situacao = 'aguardando'",
        id
    )
    .execute(&mut *tx)
    .await?;
    if feito.rows_affected() != 1 {
        return Err(ja_decidida());
    }
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "agenda",
            tipo: "ausencia_cancelada",
            registro_id: id,
            registro_codigo: Some(&pedido.codigo),
            dados: json!({ "medico_id": pedido.medico_id }),
            criado_por: Some(usuario.id),
            origem: "minha_agenda",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let a = carregar(&mut tx, id).await?;
    let corpo = serde_json::to_value(a).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 200, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::OK, Json(corpo)).into_response())
}

#[cfg(test)]
mod testes {
    use super::*;

    fn pedido(motivo: &str, horas: i64, dias_frente: i64) -> NovaAusencia {
        let inicio = Utc::now() + Duration::days(dias_frente);
        NovaAusencia {
            inicio,
            fim: inicio + Duration::hours(horas),
            motivo: motivo.into(),
            detalhe: None,
        }
    }

    #[test]
    fn limites_do_pedido() {
        let agora = Utc::now();
        assert!(conferir_pedido(&pedido("congresso", 48, 3), agora).is_ok());
        assert!(conferir_pedido(&pedido("viagem", 48, 3), agora).is_err());
        assert!(conferir_pedido(&pedido("ferias", 0, 3), agora).is_err());
        assert!(conferir_pedido(&pedido("ferias", 63 * 24, 3), agora).is_err());
        assert!(conferir_pedido(&pedido("ferias", 24, -5), agora).is_err());
        let mut p = pedido("pessoal", 4, 1);
        p.detalhe = Some("ab".into());
        assert!(conferir_pedido(&p, agora).is_err());
    }
}
