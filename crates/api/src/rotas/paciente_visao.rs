//! Paciente 360 (Paciente.dc.html), primeiro corte: identificação, próximos agendamentos e histórico
//! do paciente, numa leitura só. Quem vê é decidido pelo RLS (unidade e perfis de atendimento) e pelo
//! ato; o conteúdo de laudo não vem aqui (fica na tela do laudo, com vínculo de cuidado). CPF sai
//! mascarado. Cada leitura fica em `auditoria.acesso` (docs/18 § 6 item 9).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Serialize, ToSchema)]
pub struct IdentificacaoDoPaciente {
    pub id: Uuid,
    /// Prontuário (código legível).
    pub codigo: String,
    pub nome: String,
    pub nome_social: Option<String>,
    pub nascimento: NaiveDate,
    pub sexo: Option<String>,
    /// Só os dois últimos dígitos (***.***.***-17).
    pub cpf_mascarado: Option<String>,
    pub celular: Option<String>,
    pub email: Option<String>,
    pub alergias: Vec<String>,
    pub nega_alergias: Option<bool>,
}

#[derive(Serialize, ToSchema)]
pub struct AgendamentoDoPaciente {
    pub id: Uuid,
    pub codigo: String,
    pub inicio: DateTime<Utc>,
    pub exame: String,
    pub sala: String,
    pub medico: Option<String>,
    pub situacao: String,
}

#[derive(Serialize, ToSchema)]
pub struct VisaoDoPaciente {
    pub identificacao: IdentificacaoDoPaciente,
    pub proximos: Vec<AgendamentoDoPaciente>,
    pub historico: Vec<AgendamentoDoPaciente>,
    pub faltas: i64,
    pub exames_externos: i64,
}

pub(crate) fn mascarar_cpf(cpf: &str) -> String {
    let fim: String = cpf
        .chars()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("***.***.***-{fim}")
}

/// Visão geral do paciente (recepção, gerência, técnico e médico, na unidade).
#[utoipa::path(get, operation_id = "visao_do_paciente", path = "/v1/pacientes/{id}/visao", tag = "pacientes", params(("id" = Uuid, Path)),
    responses((status = 200, body = VisaoDoPaciente), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn visao(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Response, ErroApi> {
    if !(usuario.pode(Ato::PacienteCadastrar) || usuario.pode(Ato::SalaVer)) {
        return Err(ErroApi::proibido());
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    // RLS: paciente de outra unidade ou fora dos perfis de atendimento não aparece (404, sem oráculo).
    let p = sqlx::query!(
        "SELECT id, codigo, nome, nome_social, nascimento, sexo, cpf, celular, email, alergias, nega_alergias FROM identidade.paciente WHERE id = $1",
        id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(p) = p else {
        return Err(ErroApi::nao_encontrado(
            "paciente-nao-encontrado",
            "Paciente não encontrado nesta unidade.",
        ));
    };
    let linhas = sqlx::query!(
        r#"SELECT a.id, a.codigo, a.inicio, a.exame_nome, s.nome AS sala, u.nome AS "medico?", a.situacao
             FROM agenda.agendamento a JOIN agenda.sala s ON s.id = a.sala_id
             LEFT JOIN identidade.usuario u ON u.id = a.medico_id
            WHERE a.paciente_id = $1 ORDER BY a.inicio DESC LIMIT 200"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    let externos = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM exames.externo WHERE paciente_id = $1"#,
        id
    )
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO auditoria.acesso (id, quem, quem_tipo, recurso, recurso_id, resultado, correlacao) VALUES ($1, $2, 'usuario', 'paciente.visao', $3, 'permitido', $4)",
        novo_id(), usuario.id, id, crate::trace::atual().unwrap_or_default()
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let agora = crate::tempo::agora();
    let ativos = ["agendado", "confirmado", "reserva_provisoria"];
    let faltas = linhas.iter().filter(|l| l.situacao == "faltou").count() as i64;
    let (mut proximos, mut historico) = (Vec::new(), Vec::new());
    for l in linhas {
        let futuro = l.inicio >= agora && ativos.contains(&l.situacao.as_str());
        let a = AgendamentoDoPaciente {
            id: l.id,
            codigo: l.codigo,
            inicio: l.inicio,
            exame: l.exame_nome,
            sala: l.sala,
            medico: l.medico,
            situacao: l.situacao,
        };
        if futuro {
            proximos.push(a);
        } else {
            historico.push(a);
        }
    }
    proximos.reverse();
    let corpo = VisaoDoPaciente {
        identificacao: IdentificacaoDoPaciente {
            id: p.id,
            codigo: p.codigo,
            nome: p.nome,
            nome_social: p.nome_social,
            nascimento: p.nascimento,
            sexo: p.sexo,
            cpf_mascarado: p.cpf.as_deref().map(mascarar_cpf),
            celular: p.celular,
            email: p.email,
            alergias: p.alergias.unwrap_or_default(),
            nega_alergias: p.nega_alergias,
        },
        proximos,
        historico,
        faltas,
        exames_externos: externos,
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(corpo)).into_response())
}
