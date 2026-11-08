//! Lista de espera por exame, com prioridade clínica e preferência de horário (docs/04
//! Lista-Espera, DEC-2026-11-08-83). A posição é calculada pelo servidor (domínio); o cliente não
//! manda ordem nem posição (G1). Sair da lista é uma situação nova com motivo, nada é apagado.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::{Exame, PacienteResumo};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};
use vidacor_dominio_recepcao::espera::{
    conferir_inclusao, ordenar, ErroEspera, PedidoNaFila, Preferencia, Prioridade,
};

const ROTA_INCLUIR: &str = "POST /v1/lista-espera";

#[derive(Deserialize, ToSchema)]
pub struct PedidoInclusao {
    pub paciente_id: Uuid,
    pub exame_tuss: String,
    /// urgente, oncologico, gestante ou rotina.
    pub prioridade: String,
    /// qualquer, manha ou tarde.
    pub preferencia: String,
    /// Obrigatória para urgente.
    pub justificativa: Option<String>,
    /// Obrigatório para urgente.
    pub medico_solicitante: Option<String>,
    /// Convênio do paciente (nulo para particular).
    pub operadora: Option<String>,
    /// Guia já autorizada, consumida quando a oferta for aceita.
    pub guia_id: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
pub struct OfertaDoPedido {
    pub id: Uuid,
    pub codigo: String,
    pub sala_nome: String,
    pub inicio: DateTime<Utc>,
    pub expira_em: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct PedidoDeEspera {
    pub id: Uuid,
    pub codigo: String,
    /// Posição na fila (1 é o próximo), calculada pelo servidor; nula fora da fila.
    pub posicao: Option<i32>,
    pub paciente: PacienteResumo,
    pub exame: Exame,
    /// urgente, oncologico, gestante ou rotina.
    pub prioridade: String,
    /// qualquer, manha ou tarde.
    pub preferencia: String,
    pub justificativa: Option<String>,
    pub medico_solicitante: Option<String>,
    pub operadora: Option<String>,
    pub guia_codigo: Option<String>,
    /// aguardando, ofertado, encaixado ou saiu.
    pub situacao: String,
    pub desde: DateTime<Utc>,
    /// Oferta aguardando resposta, quando há.
    pub oferta: Option<OfertaDoPedido>,
}

fn sem_texto(t: Option<&String>) -> Option<String> {
    t.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

/// Inclui o paciente na lista de espera de um exame.
#[utoipa::path(post, operation_id = "incluir_na_espera", path = "/v1/lista-espera", tag = "agenda",
    request_body = PedidoInclusao, params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = PedidoDeEspera), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc, description = "paciente-nao-encontrado, guia-nao-encontrada"),
              (status = 409, body = crate::openapi::ProblemaDoc, description = "ja-na-espera"),
              (status = 422, body = crate::openapi::ProblemaDoc, description = "urgente-sem-justificativa, procedimento-desconhecido, guia-de-outro-paciente, pedido-invalido")))]
pub async fn incluir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<PedidoInclusao>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::EsperaGerir)?;
    let justificativa = sem_texto(p.justificativa.as_ref());
    let medico = sem_texto(p.medico_solicitante.as_ref());
    let operadora = sem_texto(p.operadora.as_ref());
    let (Ok(prioridade), Ok(preferencia)) = (
        Prioridade::de_str(&p.prioridade),
        Preferencia::de_str(&p.preferencia),
    ) else {
        return Err(ErroApi::invalido("pedido-invalido", "Escolha a prioridade (rotina, gestante, oncológico ou urgente) e a preferência de horário (qualquer, manhã ou tarde). Nada foi incluído."));
    };
    conferir_inclusao(prioridade, justificativa.as_deref(), medico.as_deref()).map_err(
        |e| {
            match e {
        ErroEspera::UrgenteSemJustificativa => ErroApi::invalido(
            "urgente-sem-justificativa",
            "Pedido urgente precisa da justificativa e do médico solicitante. Nada foi incluído.",
        ),
    }
        },
    )?;
    if justificativa
        .as_ref()
        .is_some_and(|j| j.chars().count() > 500)
        || medico.as_ref().is_some_and(|m| m.chars().count() > 120)
        || operadora
            .as_ref()
            .is_some_and(|o| !(2..=80).contains(&o.chars().count()))
    {
        return Err(ErroApi::invalido("pedido-invalido", "Confira os campos: justificativa até 500 letras, médico até 120 e convênio de 2 a 80. Nada foi incluído."));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, ROTA_INCLUIR, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    // RLS: paciente de outra unidade não aparece (mesma resposta de inexistente).
    let paciente = sqlx::query_scalar!(
        "SELECT id FROM identidade.paciente WHERE id = $1",
        p.paciente_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "paciente-nao-encontrado",
            "Paciente não encontrado nesta unidade. Nada foi incluído.",
        )
    })?;
    let exame = sqlx::query_scalar!(
        "SELECT tuss FROM agenda.procedimento WHERE tuss = $1 AND ativo",
        p.exame_tuss
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::invalido(
            "procedimento-desconhecido",
            "Este exame não está no catálogo da clínica. Nada foi incluído.",
        )
    })?;
    if let Some(g) = p.guia_id {
        let dono = sqlx::query_scalar!("SELECT paciente_id FROM convenio.guia WHERE id = $1", g)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                ErroApi::nao_encontrado(
                    "guia-nao-encontrada",
                    "A guia informada não foi encontrada nesta unidade. Nada foi incluído.",
                )
            })?;
        if dono != paciente {
            return Err(ErroApi::invalido(
                "guia-de-outro-paciente",
                "Esta guia é de outro paciente. Nada foi incluído. Confira a carteirinha.",
            ));
        }
    }
    let ano = tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero('ESP', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::interno("contador ESP sem valor"))?;
    let codigo = CodigoLegivel::novo("ESP", ano, n);
    let id = novo_id();
    let inserido = sqlx::query!(
        "INSERT INTO agenda.espera (id, codigo, paciente_id, exame_tuss, operadora, guia_id, prioridade, justificativa,
                                    medico_solicitante, preferencia, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'balcao', $12, $13, $14)",
        id, codigo.como_str(), paciente, exame, operadora, p.guia_id, prioridade.como_str(), justificativa,
        medico, preferencia.como_str(), usuario.id, estacao, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await;
    if let Err(sqlx::Error::Database(d)) = &inserido {
        if d.constraint() == Some("espera_uma_ativa") {
            tx.rollback().await?;
            let mut tx2 = banco::transacao(&estado.banco, &usuario.contexto()).await?;
            let existente = sqlx::query_scalar!(
                "SELECT codigo FROM agenda.espera WHERE paciente_id = $1 AND exame_tuss = $2 AND situacao IN ('aguardando','ofertado')",
                p.paciente_id,
                p.exame_tuss
            )
            .fetch_optional(&mut *tx2)
            .await?
            .unwrap_or_default();
            tx2.commit().await?;
            return Err(ErroApi::conflito(
                "ja-na-espera",
                &format!("Este paciente já está na lista de espera deste exame ({existente}). Nada foi incluído."),
            )
            .com_extra("espera_codigo", json!(existente)));
        }
    }
    inserido?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "agenda", tipo: "espera_incluida", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "exame": exame, "prioridade": prioridade.como_str(), "preferencia": preferencia.como_str(),
                       "guia_id": p.guia_id, "estacao": estacao }),
        criado_por: Some(usuario.id), origem: "balcao", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let fila = carregar_fila(&mut tx, usuario.unidade_id).await?;
    let pedido = fila
        .into_iter()
        .find(|x| x.id == id)
        .ok_or_else(|| ErroApi::interno("pedido recém-incluído fora da fila"))?;
    let corpo = serde_json::to_value(&pedido).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA_INCLUIR, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Fila ativa (aguardando e com oferta), na ordem do domínio, com a posição de cada pedido.
async fn carregar_fila(
    conexao: &mut PgConnection,
    unidade_id: Uuid,
) -> Result<Vec<PedidoDeEspera>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT e.id, e.codigo, e.prioridade, e.preferencia, e.justificativa, e.medico_solicitante, e.operadora,
                  e.situacao, e.criado_em, g.codigo AS "guia_codigo?", p.id AS paciente_id, p.nome, p.nascimento,
                  c.tuss, c.nome AS exame_nome, c.modalidade,
                  o.id AS "oferta_id?", o.codigo AS "oferta_codigo?", s.nome AS "sala_nome?",
                  o.inicio AS "oferta_inicio?", o.expira_em AS "oferta_expira?"
             FROM agenda.espera e
             JOIN identidade.paciente p ON p.id = e.paciente_id
             JOIN agenda.procedimento c ON c.tuss = e.exame_tuss
             LEFT JOIN convenio.guia g ON g.id = e.guia_id
             LEFT JOIN agenda.oferta o ON o.espera_id = e.id AND o.situacao = 'aguardando'
             LEFT JOIN agenda.sala s ON s.id = o.sala_id
            WHERE e.unidade_id = $1 AND e.situacao IN ('aguardando','ofertado')
            LIMIT 1000"#,
        unidade_id
    )
    .fetch_all(&mut *conexao)
    .await?;
    let mut ordem = Vec::with_capacity(linhas.len());
    for l in &linhas {
        ordem.push(PedidoNaFila {
            id: l.id,
            prioridade: Prioridade::de_str(&l.prioridade).map_err(ErroApi::interno)?,
            preferencia: Preferencia::de_str(&l.preferencia).map_err(ErroApi::interno)?,
            modalidade: l.modalidade.clone(),
            desde: l.criado_em,
        });
    }
    ordenar(&mut ordem);
    let mut saida = Vec::with_capacity(ordem.len());
    for (i, item) in ordem.iter().enumerate() {
        let Some(l) = linhas.iter().find(|l| l.id == item.id) else {
            continue;
        };
        let oferta = match (
            &l.oferta_id,
            &l.oferta_codigo,
            &l.sala_nome,
            l.oferta_inicio,
            l.oferta_expira,
        ) {
            (Some(id), Some(codigo), Some(sala), Some(inicio), Some(expira)) => {
                Some(OfertaDoPedido {
                    id: *id,
                    codigo: codigo.clone(),
                    sala_nome: sala.clone(),
                    inicio,
                    expira_em: expira,
                })
            }
            _ => None,
        };
        saida.push(PedidoDeEspera {
            id: l.id,
            codigo: l.codigo.clone(),
            posicao: Some(i32::try_from(i + 1).unwrap_or(i32::MAX)),
            paciente: PacienteResumo {
                id: l.paciente_id,
                nome: l.nome.clone(),
                nascimento: l.nascimento,
            },
            exame: Exame {
                codigo_tuss: l.tuss.clone(),
                nome: l.exame_nome.clone(),
            },
            prioridade: item.prioridade.como_str().to_owned(),
            preferencia: item.preferencia.como_str().to_owned(),
            justificativa: l.justificativa.clone(),
            medico_solicitante: l.medico_solicitante.clone(),
            operadora: l.operadora.clone(),
            guia_codigo: l.guia_codigo.clone(),
            situacao: l.situacao.clone(),
            desde: l.criado_em,
            oferta,
        });
    }
    Ok(saida)
}

/// Lista de espera da unidade, na ordem da fila.
#[utoipa::path(get, operation_id = "listar_espera", path = "/v1/lista-espera", tag = "agenda",
    responses((status = 200, body = Vec<PedidoDeEspera>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<PedidoDeEspera>>, ErroApi> {
    usuario.exigir(Ato::EsperaGerir)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let fila = carregar_fila(&mut tx, usuario.unidade_id).await?;
    tx.commit().await?;
    Ok(Json(fila))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoSaida {
    pub motivo: String,
}

#[derive(Serialize, ToSchema)]
pub struct SaidaDaEspera {
    pub id: Uuid,
    pub codigo: String,
    pub situacao: String,
    pub motivo_saida: String,
}

/// Tira o pedido da lista de espera, com motivo (o paciente desistiu, agendou por fora).
#[utoipa::path(post, operation_id = "tirar_da_espera", path = "/v1/lista-espera/{id}/saida", tag = "agenda",
    params(("id" = Uuid, Path)), request_body = PedidoSaida,
    responses((status = 200, body = SaidaDaEspera), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc, description = "oferta-em-andamento, pedido-fora-da-fila"),
              (status = 422, body = crate::openapi::ProblemaDoc, description = "motivo-invalido")))]
pub async fn sair(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoSaida>,
) -> Result<Json<SaidaDaEspera>, ErroApi> {
    usuario.exigir(Ato::EsperaGerir)?;
    let motivo = p.motivo.trim().to_owned();
    if !(5..=300).contains(&motivo.chars().count()) {
        return Err(ErroApi::invalido(
            "motivo-invalido",
            "Escreva o motivo da saída da lista (de 5 a 300 letras). Nada foi alterado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let saiu = sqlx::query!(
        "UPDATE agenda.espera SET situacao = 'saiu', motivo_saida = $2, versao = versao + 1
          WHERE id = $1 AND situacao = 'aguardando' RETURNING codigo",
        id,
        motivo
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(s) = saiu else {
        let atual = sqlx::query_scalar!("SELECT situacao FROM agenda.espera WHERE id = $1", id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                ErroApi::nao_encontrado(
                    "espera-nao-encontrada",
                    "Este pedido de espera não existe nesta unidade. Nada foi alterado.",
                )
            })?;
        return Err(if atual == "ofertado" {
            ErroApi::conflito("oferta-em-andamento", "Há uma oferta aguardando resposta deste paciente. Nada foi alterado. Registre a recusa da oferta antes.")
        } else {
            ErroApi::conflito(
                "pedido-fora-da-fila",
                "Este pedido já saiu da lista ou já foi encaixado. Nada foi alterado.",
            )
        });
    };
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "agenda",
            tipo: "espera_saida",
            registro_id: id,
            registro_codigo: Some(&s.codigo),
            dados: json!({ "motivo": motivo }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Json(SaidaDaEspera {
        id,
        codigo: s.codigo,
        situacao: "saiu".into(),
        motivo_saida: motivo,
    }))
}
