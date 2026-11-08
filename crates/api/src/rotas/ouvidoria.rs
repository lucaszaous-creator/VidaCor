//! Ouvidoria (Ouvidoria.dc.html): registrar manifestação (recepção e gerência), listar com prazo e
//! indicadores e responder (gerência). O prazo é do núcleo, conforme o tipo; a resposta é atômica e não
//! muda depois (DEC-2026-11-08-99). A recepção registra mas não lê a lista (o relato pode ter dado de
//! saúde): a resposta do registro traz só o protocolo e o prazo.

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
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;
use vidacor_dominio_recepcao::ouvidoria::{prazo, CANAIS, TIPOS};

#[derive(Deserialize, ToSchema)]
pub struct NovaManifestacao {
    /// reclamacao, elogio, sugestao, denuncia ou lgpd.
    pub tipo: String,
    /// telefone, presencial, email ou portal.
    pub canal: String,
    pub paciente_id: Option<Uuid>,
    pub nome_informado: Option<String>,
    /// Só para denúncia.
    #[serde(default)]
    pub anonima: bool,
    pub relato: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProtocoloDaManifestacao {
    pub id: Uuid,
    pub codigo: String,
    pub prazo_em: NaiveDate,
}

#[derive(Serialize, ToSchema)]
pub struct Manifestacao {
    pub id: Uuid,
    pub codigo: String,
    pub tipo: String,
    pub canal: String,
    /// Nome do paciente do cadastro ou o nome informado; nulo se anônima.
    pub quem: Option<String>,
    pub nascimento: Option<NaiveDate>,
    pub anonima: bool,
    pub relato: String,
    pub recebida_em: NaiveDate,
    pub prazo_em: NaiveDate,
    pub situacao: String,
    pub vencida: bool,
    pub resposta: Option<String>,
    pub respondida_em: Option<DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct PainelDaOuvidoria {
    pub abertas: i64,
    pub vencem_hoje: i64,
    /// Das respondidas nos últimos 30 dias, quantas dentro do prazo (%).
    pub no_prazo_percentual: Option<i64>,
    pub manifestacoes: Vec<Manifestacao>,
}

#[derive(Deserialize, ToSchema)]
pub struct RespostaDaManifestacao {
    pub resposta: String,
}

/// Registra a manifestação e devolve o protocolo e o prazo (recepção e gerência; Idempotency-Key).
#[utoipa::path(post, operation_id = "registrar_manifestacao", path = "/v1/ouvidoria", tag = "ouvidoria", params(("Idempotency-Key" = Uuid, Header)),
    request_body = NovaManifestacao,
    responses((status = 201, body = ProtocoloDaManifestacao), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn registrar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovaManifestacao>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::OuvidoriaRegistrar)?;
    let invalido = |d: &str| ErroApi::invalido("manifestacao-invalida", d);
    if !TIPOS.contains(&n.tipo.as_str()) || !CANAIS.contains(&n.canal.as_str()) {
        return Err(invalido("Escolha o tipo e o canal. Nada foi registrado."));
    }
    let relato = n.relato.trim();
    if !(10..=5000).contains(&relato.chars().count()) {
        return Err(invalido(
            "Escreva o que a pessoa relatou (10 a 5000 letras). Nada foi registrado.",
        ));
    }
    let nome = n
        .nome_informado
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if n.anonima && (n.tipo != "denuncia" || n.paciente_id.is_some() || nome.is_some()) {
        return Err(invalido(
            "Só denúncia pode ser anônima, e sem nome ou paciente. Nada foi registrado.",
        ));
    }
    if !n.anonima && n.paciente_id.is_none() && nome.is_none() {
        return Err(invalido("Escolha o paciente do cadastro ou informe o nome de quem se manifestou. Nada foi registrado."));
    }
    if n.tipo == "lgpd" && n.paciente_id.is_none() {
        return Err(invalido(
            "Pedido LGPD é do titular: escolha o paciente do cadastro. Nada foi registrado.",
        ));
    }
    let hoje = tempo::hoje_local();
    let prazo_em =
        prazo(&n.tipo, hoje).ok_or_else(|| invalido("Tipo desconhecido. Nada foi registrado."))?;
    const ROTA: &str = "POST /v1/ouvidoria";
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
    if let Some(p) = n.paciente_id {
        sqlx::query_scalar!("SELECT id FROM identidade.paciente WHERE id = $1", p)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                ErroApi::nao_encontrado(
                    "paciente-nao-encontrado",
                    "Paciente não encontrado nesta unidade. Nada foi registrado.",
                )
            })?;
    }
    let id = novo_id();
    let codigo = proximo(&mut tx, "OUV").await?;
    sqlx::query!(
        "INSERT INTO ouvidoria.manifestacao (id, codigo, tipo, canal, paciente_id, nome_informado, anonima, relato, recebida_em, prazo_em, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'ouvidoria', $12, $13)",
        id, codigo.como_str(), n.tipo, n.canal, n.paciente_id, nome, n.anonima, relato, hoje, prazo_em, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "ouvidoria",
            tipo: "manifestacao_registrada",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "tipo": n.tipo, "canal": n.canal, "prazo_em": prazo_em }),
            criado_por: Some(usuario.id),
            origem: "ouvidoria",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(ProtocoloDaManifestacao {
        id,
        codigo: codigo.como_str().to_owned(),
        prazo_em,
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

async fn carregar(
    tx: &mut sqlx::PgConnection,
    id: Option<Uuid>,
) -> Result<Vec<Manifestacao>, ErroApi> {
    let hoje = tempo::hoje_local();
    let linhas = sqlx::query!(
        r#"SELECT m.id, m.codigo, m.tipo, m.canal, coalesce(p.nome, m.nome_informado) AS quem, p.nascimento AS "nascimento?", m.anonima,
                  m.relato, m.recebida_em, m.prazo_em, m.situacao, m.resposta, m.respondida_em
             FROM ouvidoria.manifestacao m LEFT JOIN identidade.paciente p ON p.id = m.paciente_id
            WHERE ($1::uuid IS NULL OR m.id = $1)
              AND ($1::uuid IS NOT NULL OR m.situacao = 'aberta' OR m.respondida_em > now() - interval '30 days')
            ORDER BY m.situacao, m.prazo_em LIMIT 200"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| Manifestacao {
            vencida: l.situacao == "aberta" && l.prazo_em < hoje,
            id: l.id,
            codigo: l.codigo,
            tipo: l.tipo,
            canal: l.canal,
            quem: l.quem,
            nascimento: l.nascimento,
            anonima: l.anonima,
            relato: l.relato,
            recebida_em: l.recebida_em,
            prazo_em: l.prazo_em,
            situacao: l.situacao,
            resposta: l.resposta,
            respondida_em: l.respondida_em,
        })
        .collect())
}

/// Manifestações abertas e respondidas nos últimos 30 dias, com indicadores (gerência).
#[utoipa::path(get, operation_id = "painel_da_ouvidoria", path = "/v1/ouvidoria", tag = "ouvidoria",
    responses((status = 200, body = PainelDaOuvidoria), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn painel(State(estado): State<Estado>, usuario: Usuario) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::OuvidoriaTratar)?;
    let hoje = tempo::hoje_local();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let manifestacoes = carregar(&mut tx, None).await?;
    sqlx::query!(
        "INSERT INTO auditoria.acesso (id, quem, quem_tipo, recurso, resultado, correlacao) VALUES ($1, $2, 'usuario', 'ouvidoria.lista', 'permitido', $3)",
        novo_id(), usuario.id, crate::trace::atual().unwrap_or_default()
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let abertas = manifestacoes
        .iter()
        .filter(|m| m.situacao == "aberta")
        .count() as i64;
    let vencem_hoje = manifestacoes
        .iter()
        .filter(|m| m.situacao == "aberta" && m.prazo_em == hoje)
        .count() as i64;
    let respondidas: Vec<&Manifestacao> = manifestacoes
        .iter()
        .filter(|m| m.situacao == "respondida")
        .collect();
    let no_prazo = (!respondidas.is_empty()).then(|| {
        let ok = respondidas
            .iter()
            .filter(|m| {
                m.respondida_em
                    .is_some_and(|r| r.with_timezone(&tempo::FUSO).date_naive() <= m.prazo_em)
            })
            .count();
        (ok * 100 / respondidas.len()) as i64
    });
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(PainelDaOuvidoria {
            abertas,
            vencem_hoje,
            no_prazo_percentual: no_prazo,
            manifestacoes,
        }),
    )
        .into_response())
}

/// Responde a manifestação aberta (gerência). Atômico: duas respostas ao mesmo tempo, só uma vale.
#[utoipa::path(post, operation_id = "responder_manifestacao", path = "/v1/ouvidoria/{id}/resposta", tag = "ouvidoria", params(("id" = Uuid, Path)),
    request_body = RespostaDaManifestacao,
    responses((status = 200, body = Manifestacao), (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn responder(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(r): Corpo<RespostaDaManifestacao>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::OuvidoriaTratar)?;
    let resposta = r.resposta.trim();
    if !(20..=5000).contains(&resposta.chars().count()) {
        return Err(ErroApi::invalido(
            "resposta-invalida",
            "Diga o que aconteceu, o que foi feito e o que muda (20 a 5000 letras). Nada foi enviado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let feito = sqlx::query_scalar!(
        "UPDATE ouvidoria.manifestacao SET situacao = 'respondida', resposta = $2, respondida_por = $3, respondida_em = now(), versao = versao + 1
          WHERE id = $1 AND situacao = 'aberta' RETURNING codigo",
        id, resposta, usuario.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(codigo) = feito else {
        let existe = sqlx::query_scalar!(
            "SELECT situacao FROM ouvidoria.manifestacao WHERE id = $1",
            id
        )
        .fetch_optional(&mut *tx)
        .await?;
        return Err(match existe {
            None => ErroApi::nao_encontrado(
                "manifestacao-nao-encontrada",
                "Manifestação não encontrada. Nada foi enviado.",
            ),
            Some(_) => ErroApi::conflito(
                "manifestacao-ja-respondida",
                "Esta manifestação já foi respondida. Nada foi enviado de novo.",
            ),
        });
    };
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "ouvidoria",
            tipo: "manifestacao_respondida",
            registro_id: id,
            registro_codigo: Some(&codigo),
            dados: json!({ "respondida_por": usuario.id }),
            criado_por: Some(usuario.id),
            origem: "ouvidoria",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let m = carregar(&mut tx, Some(id))
        .await?
        .pop()
        .ok_or_else(|| ErroApi::interno("manifestação sumiu"))?;
    tx.commit().await?;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(m)).into_response())
}
