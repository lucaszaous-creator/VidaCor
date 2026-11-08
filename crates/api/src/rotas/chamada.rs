//! Chamada de senha e painel da TV (Telao.dc.html; docs/04 TV de chamada; docs/18: o dispositivo TV
//! vê só senha, nome abreviado e sala). A recepção chama para o guichê; o técnico chama para a sala
//! em que está lotado. Rechamar é outra chamada (registro imutável). O nome abreviado e o destino são
//! decididos aqui e gravados: a TV não lê cadastro, agenda nem exame.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::rotas::caixa::proximo;
use crate::rotas::triagem::exigir_lotacao;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;
use vidacor_dominio_recepcao::chamada::{guiche_valido, nome_abreviado};

#[derive(Deserialize, ToSchema)]
pub struct PedidoDeChamada {
    /// `guiche` (recepção) ou `sala` (técnico lotado na sala do atendimento).
    pub destino: String,
    /// Número do guichê (1 a 20), só para a recepção.
    pub guiche: Option<i32>,
}

/// O que a TV mostra. Sem exame, sem nascimento, sem identificador do paciente.
#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct ChamadaNoPainel {
    pub senha: String,
    pub nome_abreviado: String,
    pub destino: String,
    pub rechamada: bool,
    pub chamada_em: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct Painel {
    pub agora: Option<ChamadaNoPainel>,
    /// Até quatro chamadas anteriores do dia, da mais recente para a mais antiga.
    pub ultimas: Vec<ChamadaNoPainel>,
}

/// Chama (ou rechama) a senha do atendimento (recepção para o guichê, técnico para a sala).
#[utoipa::path(post, operation_id = "chamar_senha", path = "/v1/atendimentos/{id}/chamadas", tag = "recepcao", params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)),
    request_body = PedidoDeChamada,
    responses((status = 201, body = ChamadaNoPainel), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn chamar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(atendimento): Path<Uuid>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<PedidoDeChamada>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::ChamadaFazer)?;
    // O destino segue o perfil: a recepção chama ao guichê; o técnico, à sala.
    let guiche = match (p.destino.as_str(), p.guiche) {
        ("guiche", Some(n)) if usuario.tem_perfil("recepcao") && guiche_valido(n) => Some(n),
        ("sala", None) if usuario.tem_perfil("tecnico") => None,
        _ => {
            return Err(ErroApi::invalido(
                "destino-invalido",
                "A recepção chama para um guichê de 1 a 20; o técnico chama para a sala. Ninguém foi chamado.",
            ))
        }
    };
    const ROTA: &str = "POST /v1/atendimentos/{id}/chamadas";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, ROTA, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let (inicio, fim) = tempo::limites_do_dia(tempo::hoje_local())
        .ok_or_else(|| ErroApi::interno("dia sem limites"))?;
    // RLS limita à unidade e aos perfis de atendimento; outro ID não dá direito.
    let a = sqlx::query!(
        r#"SELECT t.senha, t.situacao, t.sala_id, s.nome AS sala_nome, p.nome, p.nome_social
             FROM recepcao.atendimento t
             JOIN agenda.sala s ON s.id = t.sala_id
             JOIN identidade.paciente p ON p.id = t.paciente_id
            WHERE t.id = $1 AND t.criado_em >= $2 AND t.criado_em < $3"#,
        atendimento,
        inicio,
        fim
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "atendimento-nao-encontrado",
            "Atendimento de hoje não encontrado. Ninguém foi chamado.",
        )
    })?;
    let senha = a.senha.ok_or_else(|| {
        ErroApi::conflito(
            "sem-senha",
            "Este atendimento não tem senha. Ninguém foi chamado.",
        )
    })?;
    if matches!(a.situacao.as_str(), "cancelado" | "exame_concluido") {
        return Err(ErroApi::conflito(
            "atendimento-encerrado",
            "O atendimento já foi encerrado. Ninguém foi chamado.",
        ));
    }
    let destino = if let Some(n) = guiche {
        format!("Guichê {n}")
    } else {
        exigir_lotacao(&mut tx, usuario.id, a.sala_id).await?;
        a.sala_nome.chars().take(80).collect()
    };
    // Nome social, quando houver, é o que se chama em voz alta e no painel.
    let abreviado = nome_abreviado(a.nome_social.as_deref().unwrap_or(&a.nome));
    let rechamada = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM recepcao.chamada WHERE atendimento_id = $1) AS "x!""#,
        atendimento
    )
    .fetch_one(&mut *tx)
    .await?;
    let codigo = proximo(&mut tx, "CHA").await?;
    let chamada_em = sqlx::query_scalar!(
        "INSERT INTO recepcao.chamada (id, codigo, atendimento_id, senha, nome_abreviado, destino_tipo, guiche, sala_id, destino, rechamada, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING criado_em",
        novo_id(), codigo.como_str(), atendimento, senha, abreviado, p.destino, guiche,
        if guiche.is_none() { Some(a.sala_id) } else { None },
        destino, rechamada, usuario.id,
        if guiche.is_some() { "recepcao" } else { "sala" },
        usuario.unidade_id, correlacao
    )
    .fetch_one(&mut *tx)
    .await?;
    let corpo = serde_json::to_value(ChamadaNoPainel {
        senha,
        nome_abreviado: abreviado,
        destino,
        rechamada,
        chamada_em,
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Painel da TV: a chamada atual e as quatro anteriores do dia (dispositivo TV, recepção, gerência).
#[utoipa::path(get, operation_id = "painel_de_chamada", path = "/v1/painel-de-chamada", tag = "recepcao",
    responses((status = 200, body = Painel), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn painel(State(estado): State<Estado>, usuario: Usuario) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::PainelVer)?;
    let (inicio, fim) = tempo::limites_do_dia(tempo::hoje_local())
        .ok_or_else(|| ErroApi::interno("dia sem limites"))?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let linhas = sqlx::query!(
        "SELECT senha, nome_abreviado, destino, rechamada, criado_em FROM recepcao.chamada
          WHERE criado_em >= $1 AND criado_em < $2 ORDER BY criado_em DESC, id DESC LIMIT 5",
        inicio,
        fim
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut chamadas = linhas.into_iter().map(|l| ChamadaNoPainel {
        senha: l.senha,
        nome_abreviado: l.nome_abreviado,
        destino: l.destino,
        rechamada: l.rechamada,
        chamada_em: l.criado_em,
    });
    let agora = chamadas.next();
    // Nada do painel vai para cache (nome abreviado ainda é dado pessoal).
    Ok((
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        Json(Painel {
            agora,
            ultimas: chamadas.collect(),
        }),
    )
        .into_response())
}
