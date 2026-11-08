use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct Sala {
    pub id: Uuid,
    pub nome: String,
}

/// Salas da unidade do usuário.
#[utoipa::path(get, path = "/v1/salas", tag = "agenda",
    responses((status = 200, body = Vec<Sala>), (status = 401, body = crate::openapi::ProblemaDoc),
              (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn salas(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<Sala>>, ErroApi> {
    usuario.exigir(Ato::AgendaVer)?;
    let linhas = sqlx::query_as!(
        Sala,
        "SELECT id, nome FROM agenda.sala WHERE unidade_id = $1 ORDER BY codigo",
        usuario.unidade_id
    )
    .fetch_all(&estado.banco)
    .await?;
    Ok(Json(linhas))
}

#[derive(Deserialize, IntoParams)]
pub struct FiltroAgenda {
    /// Dia no formato AAAA-MM-DD (horário de Brasília).
    pub data: NaiveDate,
    pub sala_id: Option<Uuid>,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct PacienteResumo {
    pub id: Uuid,
    pub nome: String,
    pub nascimento: NaiveDate,
}

#[derive(Serialize, ToSchema)]
pub struct Exame {
    pub codigo_tuss: String,
    pub nome: String,
}

#[derive(Serialize, ToSchema)]
pub struct ItemAgenda {
    pub id: Uuid,
    pub codigo: String,
    pub inicio: DateTime<Utc>,
    pub fim: DateTime<Utc>,
    pub sala: Sala,
    pub paciente: PacienteResumo,
    pub exame: Exame,
    /// agendado, confirmado, em_atendimento, concluido, faltou ou cancelado.
    pub situacao: String,
    pub atendimento_id: Option<Uuid>,
    /// Outros pacientes do dia com o mesmo nome e outro nascimento (alerta de atenção, FC-CHK-04).
    pub homonimos: Vec<PacienteResumo>,
    pub requer_contraste: bool,
    /// Situação do laudo, sem conteúdo.
    pub laudo_situacao: Option<String>,
    /// Fim da reserva provisória, quando houver.
    pub expira_em: Option<DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct AgendaDoDia {
    pub data: NaiveDate,
    pub itens: Vec<ItemAgenda>,
}

/// Agenda do dia por sala, com os dois identificadores do paciente (REQ-AGD-001).
#[utoipa::path(get, path = "/v1/agenda", tag = "agenda", params(FiltroAgenda),
    responses((status = 200, body = AgendaDoDia), (status = 401, body = crate::openapi::ProblemaDoc),
              (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn agenda_do_dia(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(filtro): Query<FiltroAgenda>,
) -> Result<Json<AgendaDoDia>, ErroApi> {
    usuario.exigir(Ato::AgendaVer)?;
    let (inicio, fim) = crate::tempo::limites_do_dia(filtro.data).ok_or_else(|| {
        ErroApi::invalido(
            "data-invalida",
            "A data pedida não é válida. Nada foi alterado. Escolha outro dia no calendário.",
        )
    })?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let linhas = sqlx::query!(
        r#"SELECT a.id, a.codigo, a.inicio, a.fim, a.situacao, a.exame_tuss, a.exame_nome,
                  s.id AS sala_id, s.nome AS sala_nome,
                  p.id AS paciente_id, p.nome AS paciente_nome, p.nascimento,
                  t.id AS "atendimento_id?", a.requer_contraste, a.expira_em,
                  laudos.situacao_do_atendimento(t.id) AS laudo_situacao
             FROM agenda.agendamento a
             JOIN agenda.sala s ON s.id = a.sala_id
             JOIN identidade.paciente p ON p.id = a.paciente_id
             LEFT JOIN recepcao.atendimento t ON t.agendamento_id = a.id
            WHERE a.inicio >= $1 AND a.inicio < $2
              AND ($3::uuid IS NULL OR a.sala_id = $3)
              AND a.situacao <> 'expirada'
            ORDER BY a.inicio, s.codigo
            LIMIT 500"#,
        inicio,
        fim,
        filtro.sala_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    // Homônimos: mesmo nome (sem acento e maiúsculas) e outro paciente no mesmo dia.
    let chave = |nome: &str| {
        vidacor_comum::NomeCompleto::novo(nome)
            .map(|n| n.forma_de_comparacao())
            .unwrap_or_else(|_| nome.to_lowercase())
    };
    let pacientes_do_dia: Vec<(String, PacienteResumo)> = linhas
        .iter()
        .map(|l| {
            (
                chave(&l.paciente_nome),
                PacienteResumo {
                    id: l.paciente_id,
                    nome: l.paciente_nome.clone(),
                    nascimento: l.nascimento,
                },
            )
        })
        .collect();
    let itens = linhas
        .into_iter()
        .map(|l| {
            let minha = chave(&l.paciente_nome);
            let mut homonimos: Vec<PacienteResumo> = pacientes_do_dia
                .iter()
                .filter(|(k, p)| *k == minha && p.id != l.paciente_id)
                .map(|(_, p)| p.clone())
                .collect();
            homonimos.dedup_by_key(|p| p.id);
            ItemAgenda {
                id: l.id,
                codigo: l.codigo,
                inicio: l.inicio,
                fim: l.fim,
                sala: Sala {
                    id: l.sala_id,
                    nome: l.sala_nome,
                },
                paciente: PacienteResumo {
                    id: l.paciente_id,
                    nome: l.paciente_nome,
                    nascimento: l.nascimento,
                },
                exame: Exame {
                    codigo_tuss: l.exame_tuss,
                    nome: l.exame_nome,
                },
                situacao: l.situacao,
                atendimento_id: l.atendimento_id,
                homonimos,
                requer_contraste: l.requer_contraste,
                laudo_situacao: l.laudo_situacao,
                expira_em: l.expira_em,
            }
        })
        .collect();
    Ok(Json(AgendaDoDia {
        data: filtro.data,
        itens,
    }))
}
