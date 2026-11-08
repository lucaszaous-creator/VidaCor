//! Bloqueio de sala e horário (feriado, manutenção), sempre com motivo (docs/04 Agenda,
//! DEC-2026-11-08-82). Bloquear impede agendar e oferecer o período; retirar devolve o período à
//! agenda e oferece os horários à lista de espera. Nada é apagado: retirar é uma situação nova.
//! A exclusividade fica no banco (migration 0061): gatilho que trava a sala e exclusão GiST.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::Sala;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};

const ROTA_CRIAR: &str = "POST /v1/bloqueios";
/// Período máximo de um bloqueio (o mesmo limite da constraint).
const MAX_DIAS: i64 = 62;

#[derive(Deserialize, Serialize, ToSchema, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TipoBloqueio {
    Feriado,
    Manutencao,
    Outro,
}

impl TipoBloqueio {
    fn como_str(self) -> &'static str {
        match self {
            Self::Feriado => "feriado",
            Self::Manutencao => "manutencao",
            Self::Outro => "outro",
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoBloqueio {
    /// Salas bloqueadas (uma a vinte). Todas no mesmo período e com o mesmo motivo.
    pub sala_ids: Vec<Uuid>,
    pub tipo: TipoBloqueio,
    pub inicio: DateTime<Utc>,
    pub fim: DateTime<Utc>,
    pub motivo: String,
}

#[derive(Serialize, ToSchema)]
pub struct Bloqueio {
    pub id: Uuid,
    pub codigo: String,
    pub sala: Sala,
    /// feriado, manutencao ou outro.
    pub tipo: String,
    pub inicio: DateTime<Utc>,
    pub fim: DateTime<Utc>,
    pub motivo: String,
    /// ativo ou retirado.
    pub situacao: String,
    pub retirado_em: Option<DateTime<Utc>>,
    pub motivo_retirada: Option<String>,
}

fn motivo_valido(m: &str) -> bool {
    (5..=300).contains(&m.trim().chars().count())
}

/// Bloqueia salas num período (feriado, manutenção ou outro), com motivo.
#[utoipa::path(post, operation_id = "criar_bloqueios", path = "/v1/bloqueios", tag = "agenda",
    request_body = PedidoBloqueio, params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = Vec<Bloqueio>), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc, description = "sala-nao-encontrada"),
              (status = 409, body = crate::openapi::ProblemaDoc, description = "periodo-com-agendamentos (com agendamentos), bloqueio-sobreposto"),
              (status = 422, body = crate::openapi::ProblemaDoc, description = "bloqueio-invalido")))]
pub async fn criar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<PedidoBloqueio>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::BloqueioGerir)?;
    let mut salas = p.sala_ids.clone();
    salas.sort_unstable();
    salas.dedup();
    if salas.is_empty() || salas.len() > 20 {
        return Err(ErroApi::invalido(
            "bloqueio-invalido",
            "Escolha de 1 a 20 salas para bloquear. Nada foi bloqueado.",
        ));
    }
    if p.fim <= p.inicio || p.fim - p.inicio > Duration::days(MAX_DIAS) {
        return Err(ErroApi::invalido("bloqueio-invalido", "O fim do bloqueio precisa ser depois do início, num período de até 62 dias. Nada foi bloqueado."));
    }
    if p.fim <= tempo::agora() {
        return Err(ErroApi::invalido(
            "bloqueio-invalido",
            "Este período já passou. Nada foi bloqueado. Escolha um período a partir de agora.",
        ));
    }
    if !motivo_valido(&p.motivo) {
        return Err(ErroApi::invalido("bloqueio-invalido", "Escreva o motivo do bloqueio (de 5 a 300 letras), por exemplo \"Feriado de Finados\" ou \"Manutenção preventiva do aparelho\". Nada foi bloqueado."));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, ROTA_CRIAR, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    // Trava as salas sempre na mesma ordem (por id): dois bloqueios de várias salas não se travam
    // um ao outro. Quem agenda ou oferece na mesma sala espera este bloqueio terminar (G2).
    let travadas = sqlx::query_scalar!(
        "SELECT id FROM agenda.sala WHERE id = ANY($1) AND unidade_id = $2 ORDER BY id FOR UPDATE",
        &salas,
        usuario.unidade_id
    )
    .fetch_all(&mut *tx)
    .await?;
    if travadas.len() != salas.len() {
        return Err(ErroApi::nao_encontrado(
            "sala-nao-encontrada",
            "Alguma sala escolhida não existe nesta unidade. Nada foi bloqueado.",
        ));
    }
    // Conferência com a lista do que impede (a mensagem diz o que remarcar); o gatilho garante.
    let ocupados = sqlx::query!(
        r#"SELECT a.codigo AS "codigo!", a.inicio AS "inicio!", s.nome AS "sala!" FROM agenda.agendamento a JOIN agenda.sala s ON s.id = a.sala_id
            WHERE a.sala_id = ANY($1) AND a.situacao NOT IN ('cancelado','faltou','expirada')
              AND tstzrange(a.inicio, a.fim) && tstzrange($2, $3)
           UNION ALL
           SELECT o.codigo, o.inicio, s.nome FROM agenda.oferta o JOIN agenda.sala s ON s.id = o.sala_id
            WHERE o.sala_id = ANY($1) AND o.situacao = 'aguardando' AND o.expira_em > now()
              AND tstzrange(o.inicio, o.fim) && tstzrange($2, $3)
           ORDER BY 2 LIMIT 50"#,
        &salas,
        p.inicio,
        p.fim
    )
    .fetch_all(&mut *tx)
    .await?;
    if !ocupados.is_empty() {
        let lista: Vec<_> = ocupados
            .iter()
            .map(|o| json!({ "codigo": o.codigo, "inicio": o.inicio, "sala": o.sala }))
            .collect();
        return Err(
            periodo_com_agendamentos(ocupados.len()).com_extra("agendamentos", json!(lista))
        );
    }
    let ano = tempo::ano_local();
    let mut criados = Vec::with_capacity(salas.len());
    for sala in &salas {
        let n = sqlx::query_scalar!("SELECT comum.proximo_numero('BLQ', $1)", ano)
            .fetch_one(&mut *tx)
            .await?
            .ok_or_else(|| ErroApi::interno("contador BLQ sem valor"))?;
        let codigo = CodigoLegivel::novo("BLQ", ano, n);
        let id = novo_id();
        sqlx::query!(
            "INSERT INTO agenda.bloqueio (id, codigo, sala_id, tipo, inicio, fim, motivo, criado_por, origem, estacao, unidade_id, correlacao)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'balcao', $9, $10, $11)",
            id, codigo.como_str(), sala, p.tipo.como_str(), p.inicio, p.fim, p.motivo.trim(),
            usuario.id, estacao, usuario.unidade_id, correlacao
        )
        .execute(&mut *tx)
        .await
        .map_err(erro_de_gravacao)?;
        lastro::registrar(&mut tx, NovoEvento {
            dominio: "agenda", tipo: "bloqueio_criado", registro_id: id, registro_codigo: Some(codigo.como_str()),
            dados: json!({ "sala_id": sala, "tipo": p.tipo.como_str(), "inicio": p.inicio, "fim": p.fim, "motivo": p.motivo.trim(), "estacao": estacao }),
            criado_por: Some(usuario.id), origem: "balcao", unidade_id: usuario.unidade_id, correlacao: &correlacao,
        }).await?;
        criados.push(id);
    }
    let mut saida = Vec::with_capacity(criados.len());
    for id in criados {
        saida.push(carregar(&mut tx, id).await?);
    }
    let corpo = serde_json::to_value(&saida).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA_CRIAR, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

fn periodo_com_agendamentos(n: usize) -> ErroApi {
    ErroApi::conflito(
        "periodo-com-agendamentos",
        &format!("Há {n} agendamento(s) ou oferta(s) da lista de espera neste período. Nada foi bloqueado. Remarque ou cancele cada um com motivo e bloqueie de novo."),
    )
}

fn erro_de_gravacao(e: sqlx::Error) -> ErroApi {
    match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("bloqueio_sem_sobreposicao") => ErroApi::conflito(
            "bloqueio-sobreposto",
            "Esta sala já tem um bloqueio ativo que cobre parte deste período. Nada foi bloqueado. Confira os bloqueios da sala.",
        ),
        sqlx::Error::Database(d) if d.constraint() == Some("bloqueio_com_agendamentos") => periodo_com_agendamentos(1),
        _ => ErroApi::from(e),
    }
}

async fn carregar(conexao: &mut PgConnection, id: Uuid) -> Result<Bloqueio, ErroApi> {
    let b = sqlx::query!(
        "SELECT b.id, b.codigo, b.tipo, b.inicio, b.fim, b.motivo, b.situacao, b.retirado_em, b.motivo_retirada,
                s.id AS sala_id, s.nome AS sala_nome
           FROM agenda.bloqueio b JOIN agenda.sala s ON s.id = b.sala_id WHERE b.id = $1",
        id
    )
    .fetch_optional(&mut *conexao)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("bloqueio-nao-encontrado", "Este bloqueio não existe nesta unidade. Nada foi alterado."))?;
    Ok(Bloqueio {
        id: b.id,
        codigo: b.codigo,
        sala: Sala {
            id: b.sala_id,
            nome: b.sala_nome,
        },
        tipo: b.tipo,
        inicio: b.inicio,
        fim: b.fim,
        motivo: b.motivo,
        situacao: b.situacao,
        retirado_em: b.retirado_em,
        motivo_retirada: b.motivo_retirada,
    })
}

#[derive(Deserialize, IntoParams)]
pub struct PeriodoQuery {
    /// Primeiro dia (AAAA-MM-DD, horário de Brasília).
    pub de: NaiveDate,
    /// Último dia, incluído (até 62 dias depois de `de`).
    pub ate: NaiveDate,
}

/// Bloqueios (ativos e retirados) que tocam o período.
#[utoipa::path(get, operation_id = "listar_bloqueios", path = "/v1/bloqueios", tag = "agenda", params(PeriodoQuery),
    responses((status = 200, body = Vec<Bloqueio>), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 422, body = crate::openapi::ProblemaDoc, description = "periodo-invalido")))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(q): Query<PeriodoQuery>,
) -> Result<Json<Vec<Bloqueio>>, ErroApi> {
    usuario.exigir(Ato::AgendaVer)?;
    if q.ate < q.de || (q.ate - q.de).num_days() > MAX_DIAS {
        return Err(ErroApi::invalido(
            "periodo-invalido",
            "Escolha um período de até 62 dias, com o fim depois do início.",
        ));
    }
    let (ini, _) = tempo::limites_do_dia(q.de).ok_or_else(|| ErroApi::interno("dia inválido"))?;
    let (_, fim) = tempo::limites_do_dia(q.ate).ok_or_else(|| ErroApi::interno("dia inválido"))?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let ids = sqlx::query_scalar!(
        "SELECT id FROM agenda.bloqueio WHERE unidade_id = $1 AND tstzrange(inicio, fim) && tstzrange($2, $3)
          ORDER BY inicio, codigo LIMIT 500",
        usuario.unidade_id,
        ini,
        fim
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut saida = Vec::with_capacity(ids.len());
    for id in ids {
        saida.push(carregar(&mut tx, id).await?);
    }
    tx.commit().await?;
    Ok(Json(saida))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoRetirada {
    pub motivo: String,
}

/// Retira um bloqueio (com motivo): o período volta à agenda e os horários vão à lista de espera.
#[utoipa::path(post, operation_id = "retirar_bloqueio", path = "/v1/bloqueios/{id}/retirada", tag = "agenda",
    params(("id" = Uuid, Path)), request_body = PedidoRetirada,
    responses((status = 200, body = Bloqueio), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc, description = "bloqueio-ja-retirado"),
              (status = 422, body = crate::openapi::ProblemaDoc, description = "motivo-invalido")))]
pub async fn retirar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoRetirada>,
) -> Result<Json<Bloqueio>, ErroApi> {
    usuario.exigir(Ato::BloqueioGerir)?;
    if !motivo_valido(&p.motivo) {
        return Err(ErroApi::invalido(
            "motivo-invalido",
            "Escreva o motivo da retirada (de 5 a 300 letras). Nada foi alterado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    // Operação atômica conferindo as linhas: duas retiradas ao mesmo tempo, só uma vale (G2).
    let retirado = sqlx::query!(
        "UPDATE agenda.bloqueio SET situacao = 'retirado', retirado_em = now(), retirado_por = $2,
                motivo_retirada = $3, versao = versao + 1
          WHERE id = $1 AND situacao = 'ativo' AND unidade_id = $4
          RETURNING codigo, sala_id, inicio, fim",
        id,
        usuario.id,
        p.motivo.trim(),
        usuario.unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(r) = retirado else {
        let existe = carregar(&mut tx, id).await?;
        return Err(ErroApi::conflito(
            "bloqueio-ja-retirado",
            &format!(
                "O bloqueio {} já foi retirado. Nada foi alterado.",
                existe.codigo
            ),
        ));
    };
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "agenda",
            tipo: "bloqueio_retirado",
            registro_id: id,
            registro_codigo: Some(&r.codigo),
            dados: json!({ "motivo": p.motivo.trim() }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    if r.fim > tempo::agora() {
        crate::rotas::vagas::enfileirar_vaga(
            &mut tx,
            usuario.unidade_id,
            id,
            r.sala_id,
            r.inicio,
            r.fim,
            "bloqueio_retirado",
            &correlacao,
        )
        .await?;
    }
    let b = carregar(&mut tx, id).await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok(Json(b))
}
