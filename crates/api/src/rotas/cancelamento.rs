//! Cancelamento de agendamento com motivo (docs/04 Agenda; FC-AGD-08: a vaga volta a ficar livre e é
//! oferecida à lista de espera). Nada é apagado: o agendamento muda para "cancelado" com o motivo, por
//! UPDATE atômico (G2). Agendamento com guia consumida não é cancelado aqui: a devolução da guia é do
//! faturamento (DEC-2026-11-08-76).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::ItemAgenda;
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct CancelamentoDeAgendamento {
    pub motivo: String,
}

/// Cancela o agendamento (recepção e gerência) e oferece a vaga à lista de espera.
#[utoipa::path(post, operation_id = "cancelar_agendamento", path = "/v1/agendamentos/{id}/cancelamento", tag = "agenda", params(("id" = Uuid, Path)),
    request_body = CancelamentoDeAgendamento,
    responses((status = 200, body = ItemAgenda), (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc),
        (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn cancelar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(c): Corpo<CancelamentoDeAgendamento>,
) -> Result<Json<ItemAgenda>, ErroApi> {
    usuario.exigir(Ato::AgendaAgendar)?;
    let motivo = c.motivo.trim();
    if !(5..=300).contains(&motivo.chars().count()) {
        return Err(ErroApi::invalido(
            "motivo-invalido",
            "Escreva o motivo do cancelamento (5 a 300 letras). Nada foi cancelado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let feito = sqlx::query!(
        "UPDATE agenda.agendamento SET situacao = 'cancelado', motivo = $2, versao = versao + 1
          WHERE id = $1 AND situacao IN ('agendado','confirmado','reserva_provisoria')
            AND NOT EXISTS (SELECT 1 FROM convenio.guia g WHERE g.agendamento_id = agenda.agendamento.id)
          RETURNING codigo, sala_id, inicio, fim",
        id,
        motivo
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(f) = feito else {
        let atual = sqlx::query!(
            r#"SELECT a.situacao, EXISTS (SELECT 1 FROM convenio.guia g WHERE g.agendamento_id = a.id) AS "com_guia!"
                 FROM agenda.agendamento a WHERE a.id = $1"#,
            id
        )
        .fetch_optional(&mut *tx)
        .await?;
        return Err(match atual {
            None => ErroApi::nao_encontrado("agendamento-nao-encontrado", "Agendamento não encontrado. Nada foi cancelado."),
            Some(a) if a.com_guia && matches!(a.situacao.as_str(), "agendado" | "confirmado") => ErroApi::conflito(
                "guia-consumida",
                "Este agendamento consumiu uma guia do convênio. Nada foi cancelado. Peça ao faturamento para devolver a guia e cancelar.",
            ),
            Some(_) => ErroApi::conflito(
                "agendamento-nao-cancelavel",
                "Este agendamento já foi atendido, cancelado ou expirou. Nada foi cancelado.",
            ),
        });
    };
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "agenda",
            tipo: "agendamento_cancelado",
            registro_id: id,
            registro_codigo: Some(&f.codigo),
            dados: json!({ "motivo": motivo }),
            criado_por: Some(usuario.id),
            origem: "recepcao",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    crate::rotas::vagas::enfileirar_vaga(
        &mut tx,
        usuario.unidade_id,
        id,
        f.sala_id,
        f.inicio,
        f.fim,
        "cancelamento",
        &correlacao,
    )
    .await?;
    let item = crate::rotas::agendamentos::carregar_item(&mut tx, id).await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok(Json(item))
}
