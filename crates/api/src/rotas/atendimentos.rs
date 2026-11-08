//! Check-in com dois identificadores (REQ-REC-001 a 005, FC-CHK do docs/27).

use crate::atos::Ato;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::PacienteResumo;
use crate::sessao::Usuario;
use crate::{banco, tempo};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel, DataNascimento, NomeCompleto};
use vidacor_dominio_recepcao::{
    conferir_checkin, AgendamentoParaCheckin, ConfirmacaoIdentidade, ErroCheckin,
    SituacaoAgendamento,
};

const ROTA_CHECKIN: &str = "POST /v1/atendimentos";

#[derive(Deserialize, ToSchema)]
pub struct Confirmacao {
    /// Nome completo como no documento do paciente.
    pub nome: String,
    pub nascimento: NaiveDate,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoCheckin {
    pub agendamento_id: Uuid,
    pub confirmacao: Confirmacao,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct EstudoResumo {
    pub study_uid: String,
    pub imagens: i32,
    pub recebido_em: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct Atendimento {
    pub id: Uuid,
    pub codigo: String,
    pub accession: String,
    pub agendamento_id: Uuid,
    pub paciente: PacienteResumo,
    /// aguardando_exame, em_exame, exame_concluido ou cancelado.
    pub situacao: String,
    pub criado_em: DateTime<Utc>,
    pub estudo: Option<EstudoResumo>,
}

fn identificacao_nao_confere() -> ErroApi {
    ErroApi::invalido(
        "identificacao-nao-confere",
        "Nome ou data de nascimento não conferem com o cadastro. O check-in não foi feito. Confira o documento do paciente; se o cadastro estiver errado, peça a correção antes de continuar.",
    )
}

fn ja_realizado() -> ErroApi {
    ErroApi::conflito(
        "checkin-ja-realizado",
        "O check-in deste agendamento já foi feito. Nada foi alterado. Abra o atendimento na agenda do dia.",
    )
}

/// Faz o check-in de um agendamento, conferindo nome completo e data de nascimento.
#[utoipa::path(post, path = "/v1/atendimentos", tag = "recepcao", request_body = PedidoCheckin,
    params(("Idempotency-Key" = Uuid, Header), ("X-CSRF-Token" = String, Header)),
    responses(
        (status = 201, body = Atendimento),
        (status = 400, body = crate::openapi::ProblemaDoc, description = "idempotency-key-ausente"),
        (status = 401, body = crate::openapi::ProblemaDoc),
        (status = 403, body = crate::openapi::ProblemaDoc),
        (status = 404, body = crate::openapi::ProblemaDoc, description = "agendamento-nao-encontrado"),
        (status = 409, body = crate::openapi::ProblemaDoc, description = "checkin-ja-realizado ou situacao-nao-permite-checkin"),
        (status = 422, body = crate::openapi::ProblemaDoc, description = "identificacao-nao-confere")))]
pub async fn checkin(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(pedido): Corpo<PedidoCheckin>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::RecepcaoCheckin)?;
    let nome = NomeCompleto::novo(&pedido.confirmacao.nome).map_err(|_| {
        ErroApi::invalido(
            "nome-incompleto",
            "Informe o nome completo do paciente, como no documento. O check-in não foi feito.",
        )
    })?;
    let nascimento = DataNascimento::nova(pedido.confirmacao.nascimento, tempo::hoje_local())
        .map_err(|_| identificacao_nao_confere())?;
    let correlacao = crate::trace::atual().unwrap_or_default();

    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, ROTA_CHECKIN, chave).await?
    {
        tx.commit().await?;
        let status = StatusCode::from_u16(status).unwrap_or(StatusCode::OK);
        return Ok((status, Json(corpo)).into_response());
    }

    // Trava o agendamento: duas recepções no mesmo paciente esperam uma pela outra (G2).
    let ag = sqlx::query!(
        r#"SELECT a.id, a.codigo, a.situacao, a.sala_id, a.unidade_id,
                  p.id AS paciente_id, p.nome, p.nascimento
             FROM agenda.agendamento a
             JOIN identidade.paciente p ON p.id = a.paciente_id
            WHERE a.id = $1
            FOR UPDATE OF a"#,
        pedido.agendamento_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "agendamento-nao-encontrado",
            "Este agendamento não existe ou não é desta unidade. Nada foi alterado. Atualize a agenda do dia.",
        )
    })?;

    let situacao = SituacaoAgendamento::de_str(&ag.situacao).map_err(ErroApi::interno)?;
    if situacao == SituacaoAgendamento::EmAtendimento {
        return Err(ja_realizado());
    }
    let cadastro = AgendamentoParaCheckin {
        situacao,
        nome_cadastro: NomeCompleto::novo(&ag.nome).map_err(ErroApi::interno)?,
        nascimento_cadastro: DataNascimento::nova(ag.nascimento, tempo::hoje_local())
            .map_err(ErroApi::interno)?,
    };
    conferir_checkin(&cadastro, &ConfirmacaoIdentidade { nome, nascimento }).map_err(|e| match e {
        ErroCheckin::IdentificacaoNaoConfere => identificacao_nao_confere(),
        ErroCheckin::SituacaoNaoPermite(_) => ErroApi::conflito(
            "situacao-nao-permite-checkin",
            "A situação deste agendamento não permite check-in. Nada foi alterado. Confira com a coordenação da recepção.",
        ),
    })?;

    let ano = tempo::ano_local();
    let n_atd = sqlx::query_scalar!("SELECT comum.proximo_numero('ATD', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::interno("contador ATD sem valor"))?;
    let n_acc = sqlx::query_scalar!("SELECT comum.proximo_numero('ACC', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::interno("contador ACC sem valor"))?;
    let id = novo_id();
    let codigo = CodigoLegivel::novo("ATD", ano, n_atd);
    let accession = CodigoLegivel::novo("ACC", ano, n_acc);

    let criado_em = sqlx::query_scalar!(
        "INSERT INTO recepcao.atendimento
           (id, codigo, accession, agendamento_id, paciente_id, sala_id, situacao,
            criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'aguardando_exame', $7, 'balcao', $8, $9)
         RETURNING criado_em",
        id,
        codigo.como_str(),
        accession.como_str(),
        ag.id,
        ag.paciente_id,
        ag.sala_id,
        usuario.id,
        ag.unidade_id,
        correlacao
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ja_realizado(),
        _ => ErroApi::from(e),
    })?;

    // Operação atômica conferindo as linhas afetadas (G2, segunda barreira).
    let alterou = sqlx::query!(
        "UPDATE agenda.agendamento SET situacao = 'em_atendimento', versao = versao + 1
          WHERE id = $1 AND situacao IN ('agendado', 'confirmado')",
        ag.id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if alterou != 1 {
        return Err(ja_realizado());
    }

    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "recepcao",
            tipo: "checkin_realizado",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({
                "agendamento": ag.codigo,
                "accession": accession.como_str(),
                "conferencia": "nome_e_nascimento"
            }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: ag.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;

    // Worklist do aparelho pelo outbox, na mesma transação (REQ-REC-005).
    sqlx::query!(
        "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao)
         VALUES ($1, 'worklist.criar', $2, $3, $4, $5)",
        novo_id(),
        novo_id(),
        json!({ "atendimento_id": id }),
        ag.unidade_id,
        correlacao
    )
    .execute(&mut *tx)
    .await?;

    let resposta = Atendimento {
        id,
        codigo: codigo.to_string(),
        accession: accession.to_string(),
        agendamento_id: ag.id,
        paciente: PacienteResumo {
            id: ag.paciente_id,
            nome: ag.nome,
            nascimento: ag.nascimento,
        },
        situacao: "aguardando_exame".to_owned(),
        criado_em,
        estudo: None,
    };
    let corpo: Value = serde_json::to_value(&resposta).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA_CHECKIN, chave, 201, &corpo).await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Um atendimento da unidade, com o estudo recebido, se houver.
#[utoipa::path(get, path = "/v1/atendimentos/{id}", tag = "recepcao", params(("id" = Uuid, Path)),
    responses((status = 200, body = Atendimento), (status = 401, body = crate::openapi::ProblemaDoc),
              (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn obter(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<Atendimento>, ErroApi> {
    usuario.exigir(Ato::RecepcaoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    // RLS filtra por unidade e perfil: atendimento de outra unidade simplesmente não aparece.
    let l = sqlx::query!(
        r#"SELECT t.id, t.codigo, t.accession, t.agendamento_id, t.situacao, t.criado_em,
                  p.id AS paciente_id, p.nome, p.nascimento,
                  e.study_uid AS "study_uid?", e.imagens AS "imagens?", e.recebido_em AS "recebido_em?"
             FROM recepcao.atendimento t
             JOIN identidade.paciente p ON p.id = t.paciente_id
             LEFT JOIN exames.estudo e ON e.atendimento_id = t.id AND e.conciliado
            WHERE t.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "atendimento-nao-encontrado",
            "Este atendimento não existe ou não é desta unidade. Nada foi alterado. Volte para a agenda do dia.",
        )
    })?;
    tx.commit().await?;
    let estudo = match (l.study_uid, l.imagens, l.recebido_em) {
        (Some(study_uid), Some(imagens), Some(recebido_em)) => Some(EstudoResumo {
            study_uid,
            imagens,
            recebido_em,
        }),
        _ => None,
    };
    Ok(Json(Atendimento {
        id: l.id,
        codigo: l.codigo,
        accession: l.accession,
        agendamento_id: l.agendamento_id,
        paciente: PacienteResumo {
            id: l.paciente_id,
            nome: l.nome,
            nascimento: l.nascimento,
        },
        situacao: l.situacao,
        criado_em: l.criado_em,
        estudo,
    }))
}
