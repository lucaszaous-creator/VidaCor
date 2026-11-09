//! Qualidade (Qualidade.dc.html): não conformidades e eventos de segurança do paciente. Qualquer pessoa
//! da equipe abre ou notifica (não a TV); a gerência (Núcleo de Segurança do Paciente) lê e trata.
//! Evento anônimo não mostra quem notificou. Dano grave ou óbito: 72 h para notificar a Anvisa
//! (RDC 36/2013). Nada é apagado (DEC-2026-11-08-105).

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
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

const CLASSIFICACOES: [&str; 4] = ["processo", "equipamento", "documentacao", "fornecedor"];
const GRAVIDADES_NC: [&str; 3] = ["leve", "moderada", "grave"];
const TIPOS_EVENTO: [&str; 6] = [
    "reacao_contraste",
    "queda",
    "erro_identificacao",
    "exame_errado",
    "falha_equipamento",
    "outro",
];
const GRAVIDADES_EVENTO: [&str; 5] = ["sem_dano", "leve", "moderado", "grave", "obito"];
const SITUACOES_NC: [&str; 4] = ["aberta", "em_acao", "verificacao", "encerrada"];

#[derive(Deserialize, ToSchema)]
pub struct NovaNaoConformidade {
    pub descricao: String,
    pub classificacao: String,
    pub gravidade: String,
}

#[derive(Deserialize, ToSchema)]
pub struct NovoEventoAdverso {
    pub tipo: String,
    pub quase_erro: bool,
    pub gravidade: String,
    pub descricao: String,
    pub paciente_id: Option<Uuid>,
    #[serde(default)]
    pub anonimo: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProtocoloDeQualidade {
    pub id: Uuid,
    pub codigo: String,
    /// Só para evento com dano grave ou óbito.
    pub notificar_anvisa_ate: Option<DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct AndamentoDaNc {
    pub texto: String,
    pub situacao_nova: String,
    pub por: String,
    pub em: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct NaoConformidade {
    pub id: Uuid,
    pub codigo: String,
    pub descricao: String,
    pub classificacao: String,
    pub gravidade: String,
    pub responsavel: Option<String>,
    pub prazo_em: Option<NaiveDate>,
    pub situacao: String,
    pub eficaz: Option<bool>,
    pub vencida: bool,
    pub aberta_por: String,
    pub aberta_em: DateTime<Utc>,
    pub versao: i32,
    pub andamentos: Vec<AndamentoDaNc>,
}

#[derive(Serialize, ToSchema)]
pub struct EventoDeSeguranca {
    pub id: Uuid,
    pub codigo: String,
    pub tipo: String,
    pub quase_erro: bool,
    pub gravidade: String,
    pub descricao: String,
    pub paciente: Option<String>,
    /// Nulo quando anônimo.
    pub notificado_por: Option<String>,
    pub notificado_em: DateTime<Utc>,
    pub notificar_anvisa_ate: Option<DateTime<Utc>>,
    pub situacao: String,
}

#[derive(Serialize, ToSchema)]
pub struct PainelDaQualidade {
    pub nc_abertas: i64,
    pub nc_vencidas: i64,
    pub eventos_no_mes: i64,
    pub quase_erros_no_mes: i64,
    pub graves_no_mes: i64,
    pub anvisa_pendentes: i64,
    pub nao_conformidades: Vec<NaoConformidade>,
    pub eventos: Vec<EventoDeSeguranca>,
}

#[derive(Deserialize, ToSchema)]
pub struct AndamentoNovo {
    pub texto: String,
    /// aberta, em_acao, verificacao ou encerrada.
    pub situacao: String,
    pub responsavel_id: Option<Uuid>,
    pub prazo_em: Option<NaiveDate>,
    /// Obrigatório ao encerrar: a ação foi eficaz?
    pub eficaz: Option<bool>,
    /// Versão que a tela viu (evita sobrescrever o andamento de outra pessoa).
    pub versao: i32,
}

#[derive(Deserialize, ToSchema)]
pub struct SituacaoDoEvento {
    /// em_analise ou concluido.
    pub situacao: String,
}

fn sem_tv(usuario: &Usuario) -> Result<(), ErroApi> {
    if usuario.tem_perfil("tv") || usuario.perfis.is_empty() {
        Err(ErroApi::proibido())
    } else {
        Ok(())
    }
}

async fn reservar(
    tx: &mut sqlx::PgConnection,
    usuario: &Usuario,
    rota: &str,
    chave: Uuid,
) -> Result<Option<Response>, ErroApi> {
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut *tx, usuario.id, rota, chave).await?
    {
        return Ok(Some(
            (
                StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
                Json(corpo),
            )
                .into_response(),
        ));
    }
    Ok(None)
}

/// Abre uma não conformidade (qualquer pessoa da equipe; Idempotency-Key).
#[utoipa::path(post, operation_id = "abrir_nao_conformidade", path = "/v1/qualidade/nao-conformidades", tag = "qualidade", params(("Idempotency-Key" = Uuid, Header)),
    request_body = NovaNaoConformidade,
    responses((status = 201, body = ProtocoloDeQualidade), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn abrir_nc(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovaNaoConformidade>,
) -> Result<Response, ErroApi> {
    sem_tv(&usuario)?;
    let descricao = n.descricao.trim();
    if !CLASSIFICACOES.contains(&n.classificacao.as_str())
        || !GRAVIDADES_NC.contains(&n.gravidade.as_str())
        || !(10..=3000).contains(&descricao.chars().count())
    {
        return Err(ErroApi::invalido("nao-conformidade-invalida", "Descreva o que aconteceu (10 letras ou mais), a classificação e a gravidade. Nada foi registrado."));
    }
    const ROTA: &str = "POST /v1/qualidade/nao-conformidades";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Some(r) = reservar(&mut tx, &usuario, ROTA, chave).await? {
        tx.commit().await?;
        return Ok(r);
    }
    let id = novo_id();
    let codigo = proximo(&mut tx, "NCF").await?;
    sqlx::query!(
        "INSERT INTO qualidade.nao_conformidade (id, codigo, descricao, classificacao, gravidade, criado_por, origem, unidade_id, correlacao) VALUES ($1, $2, $3, $4, $5, $6, 'qualidade', $7, $8)",
        id, codigo.como_str(), descricao, n.classificacao, n.gravidade, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "qualidade",
            tipo: "nao_conformidade_aberta",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "classificacao": n.classificacao, "gravidade": n.gravidade }),
            criado_por: Some(usuario.id),
            origem: "qualidade",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(ProtocoloDeQualidade {
        id,
        codigo: codigo.como_str().to_owned(),
        notificar_anvisa_ate: None,
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Notifica evento ou quase-erro (qualquer pessoa da equipe; pode ser anônimo; Idempotency-Key).
#[utoipa::path(post, operation_id = "notificar_evento", path = "/v1/qualidade/eventos", tag = "qualidade", params(("Idempotency-Key" = Uuid, Header)),
    request_body = NovoEventoAdverso,
    responses((status = 201, body = ProtocoloDeQualidade), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn notificar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoEventoAdverso>,
) -> Result<Response, ErroApi> {
    sem_tv(&usuario)?;
    let descricao = n.descricao.trim();
    if !TIPOS_EVENTO.contains(&n.tipo.as_str())
        || !GRAVIDADES_EVENTO.contains(&n.gravidade.as_str())
        || !(10..=5000).contains(&descricao.chars().count())
    {
        return Err(ErroApi::invalido("evento-invalido", "Escolha o tipo e a gravidade e descreva o que aconteceu (10 letras ou mais). Nada foi notificado."));
    }
    if n.quase_erro && n.gravidade != "sem_dano" {
        return Err(ErroApi::invalido(
            "evento-invalido",
            "Quase-erro não chegou ao paciente: a gravidade é sem dano. Nada foi notificado.",
        ));
    }
    let prazo_anvisa = matches!(n.gravidade.as_str(), "grave" | "obito")
        .then(|| tempo::agora() + Duration::hours(72));
    const ROTA: &str = "POST /v1/qualidade/eventos";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Some(r) = reservar(&mut tx, &usuario, ROTA, chave).await? {
        tx.commit().await?;
        return Ok(r);
    }
    if let Some(p) = n.paciente_id {
        sqlx::query_scalar!("SELECT id FROM identidade.paciente WHERE id = $1", p)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                ErroApi::nao_encontrado(
                    "paciente-nao-encontrado",
                    "Paciente não encontrado nesta unidade. Nada foi notificado.",
                )
            })?;
    }
    let id = novo_id();
    let codigo = proximo(&mut tx, "EVT").await?;
    sqlx::query!(
        "INSERT INTO qualidade.evento (id, codigo, tipo, quase_erro, gravidade, descricao, paciente_id, anonimo, notificar_anvisa_ate, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'qualidade', $11, $12)",
        id, codigo.como_str(), n.tipo, n.quase_erro, n.gravidade, descricao, n.paciente_id, n.anonimo, prazo_anvisa, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    // No lastro, o evento anônimo não leva quem notificou.
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "qualidade", tipo: "evento_notificado", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "tipo": n.tipo, "gravidade": n.gravidade, "quase_erro": n.quase_erro, "anonimo": n.anonimo }),
        criado_por: if n.anonimo { None } else { Some(usuario.id) }, origem: "qualidade", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    })
    .await?;
    let corpo = serde_json::to_value(ProtocoloDeQualidade {
        id,
        codigo: codigo.como_str().to_owned(),
        notificar_anvisa_ate: prazo_anvisa,
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Painel do Núcleo de Segurança do Paciente e das não conformidades (gerência).
#[utoipa::path(get, operation_id = "painel_da_qualidade", path = "/v1/qualidade", tag = "qualidade",
    responses((status = 200, body = PainelDaQualidade), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn painel(State(estado): State<Estado>, usuario: Usuario) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::QualidadeTratar)?;
    let hoje = tempo::hoje_local();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let ncs = sqlx::query!(
        r#"SELECT n.id, n.codigo, n.descricao, n.classificacao, n.gravidade, r.nome AS "responsavel?", n.prazo_em, n.situacao, n.eficaz,
                  a.nome AS aberta_por, n.criado_em, n.versao
             FROM qualidade.nao_conformidade n JOIN identidade.usuario a ON a.id = n.criado_por LEFT JOIN identidade.usuario r ON r.id = n.responsavel_id
            WHERE n.situacao <> 'encerrada' OR n.criado_em > now() - interval '90 days'
            ORDER BY (n.situacao = 'encerrada'), n.prazo_em NULLS LAST, n.criado_em LIMIT 200"#
    )
    .fetch_all(&mut *tx)
    .await?;
    let ids: Vec<Uuid> = ncs.iter().map(|n| n.id).collect();
    let andamentos = sqlx::query!(
        "SELECT a.nc_id, a.texto, a.situacao_nova, u.nome, a.criado_em FROM qualidade.nc_andamento a JOIN identidade.usuario u ON u.id = a.criado_por WHERE a.nc_id = ANY($1) ORDER BY a.criado_em",
        &ids
    )
    .fetch_all(&mut *tx)
    .await?;
    let eventos = sqlx::query!(
        r#"SELECT e.id, e.codigo, e.tipo, e.quase_erro, e.gravidade, e.descricao, p.nome AS "paciente?", e.anonimo, u.nome AS notificado_por,
                  e.criado_em, e.notificar_anvisa_ate, e.situacao
             FROM qualidade.evento e JOIN identidade.usuario u ON u.id = e.criado_por LEFT JOIN identidade.paciente p ON p.id = e.paciente_id
            WHERE e.situacao <> 'concluido' OR e.criado_em > now() - interval '90 days'
            ORDER BY (e.situacao = 'concluido'), e.notificar_anvisa_ate NULLS LAST, e.criado_em DESC LIMIT 200"#
    )
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO auditoria.acesso (id, quem, quem_tipo, recurso, resultado, correlacao) VALUES ($1, $2, 'usuario', 'qualidade.painel', 'permitido', $3)",
        novo_id(), usuario.id, crate::trace::atual().unwrap_or_default()
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let inicio_mes = hoje.with_day(1).unwrap_or(hoje);
    let no_mes = |d: &DateTime<Utc>| d.with_timezone(&tempo::FUSO).date_naive() >= inicio_mes;
    let corpo = PainelDaQualidade {
        nc_abertas: ncs.iter().filter(|n| n.situacao != "encerrada").count() as i64,
        nc_vencidas: ncs
            .iter()
            .filter(|n| n.situacao != "encerrada" && n.prazo_em.is_some_and(|p| p < hoje))
            .count() as i64,
        eventos_no_mes: eventos.iter().filter(|e| no_mes(&e.criado_em)).count() as i64,
        quase_erros_no_mes: eventos
            .iter()
            .filter(|e| e.quase_erro && no_mes(&e.criado_em))
            .count() as i64,
        graves_no_mes: eventos
            .iter()
            .filter(|e| matches!(e.gravidade.as_str(), "grave" | "obito") && no_mes(&e.criado_em))
            .count() as i64,
        anvisa_pendentes: eventos
            .iter()
            .filter(|e| e.notificar_anvisa_ate.is_some() && e.situacao != "concluido")
            .count() as i64,
        nao_conformidades: ncs
            .into_iter()
            .map(|n| NaoConformidade {
                andamentos: andamentos
                    .iter()
                    .filter(|a| a.nc_id == n.id)
                    .map(|a| AndamentoDaNc {
                        texto: a.texto.clone(),
                        situacao_nova: a.situacao_nova.clone(),
                        por: a.nome.clone(),
                        em: a.criado_em,
                    })
                    .collect(),
                vencida: n.situacao != "encerrada" && n.prazo_em.is_some_and(|p| p < hoje),
                id: n.id,
                codigo: n.codigo,
                descricao: n.descricao,
                classificacao: n.classificacao,
                gravidade: n.gravidade,
                responsavel: n.responsavel,
                prazo_em: n.prazo_em,
                situacao: n.situacao,
                eficaz: n.eficaz,
                aberta_por: n.aberta_por,
                aberta_em: n.criado_em,
                versao: n.versao,
            })
            .collect(),
        eventos: eventos
            .into_iter()
            .map(|e| EventoDeSeguranca {
                notificado_por: if e.anonimo {
                    None
                } else {
                    Some(e.notificado_por)
                },
                id: e.id,
                codigo: e.codigo,
                tipo: e.tipo,
                quase_erro: e.quase_erro,
                gravidade: e.gravidade,
                descricao: e.descricao,
                paciente: e.paciente,
                notificado_em: e.criado_em,
                notificar_anvisa_ate: e.notificar_anvisa_ate,
                situacao: e.situacao,
            })
            .collect(),
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(corpo)).into_response())
}

/// Registra andamento da não conformidade (gerência). Atômico pela versão: quem viu versão antiga recebe 409.
#[utoipa::path(post, operation_id = "andamento_da_nao_conformidade", path = "/v1/qualidade/nao-conformidades/{id}/andamento", tag = "qualidade", params(("id" = Uuid, Path)),
    request_body = AndamentoNovo,
    responses((status = 204, description = "Andamento registrado"), (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn andamento(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(a): Corpo<AndamentoNovo>,
) -> Result<StatusCode, ErroApi> {
    usuario.exigir(Ato::QualidadeTratar)?;
    let texto = a.texto.trim();
    if !SITUACOES_NC.contains(&a.situacao.as_str()) || !(5..=3000).contains(&texto.chars().count())
    {
        return Err(ErroApi::invalido(
            "andamento-invalido",
            "Escreva o andamento (5 letras ou mais) e a situação. Nada foi registrado.",
        ));
    }
    if (a.situacao == "encerrada") != a.eficaz.is_some() {
        return Err(ErroApi::invalido(
            "andamento-invalido",
            "Para encerrar, diga se a ação foi eficaz; só ao encerrar. Nada foi registrado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let nova = sqlx::query_scalar!(
        "UPDATE qualidade.nao_conformidade SET situacao = $3, responsavel_id = coalesce($4, responsavel_id), prazo_em = coalesce($5, prazo_em), eficaz = $6, versao = versao + 1
          WHERE id = $1 AND versao = $2 AND situacao <> 'encerrada' RETURNING versao",
        id, a.versao, a.situacao, a.responsavel_id, a.prazo_em, a.eficaz
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(versao) = nova else {
        let existe = sqlx::query_scalar!(
            "SELECT situacao FROM qualidade.nao_conformidade WHERE id = $1",
            id
        )
        .fetch_optional(&mut *tx)
        .await?;
        return Err(match existe {
            None => ErroApi::nao_encontrado("nao-conformidade-nao-encontrada", "Não conformidade não encontrada. Nada foi registrado."),
            Some(_) => ErroApi::conflito("nao-conformidade-mudou", "Outra pessoa registrou andamento antes, ou ela já foi encerrada. Nada foi registrado. Abra de novo para ver a situação atual."),
        });
    };
    sqlx::query!(
        "INSERT INTO qualidade.nc_andamento (id, nc_id, texto, situacao_nova, versao_da_nc, criado_por, origem, unidade_id, correlacao) VALUES ($1, $2, $3, $4, $5, $6, 'qualidade', $7, $8)",
        novo_id(), id, texto, a.situacao, versao, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "qualidade",
            tipo: "nao_conformidade_andamento",
            registro_id: id,
            registro_codigo: None,
            dados: json!({ "situacao": a.situacao, "eficaz": a.eficaz, "versao": versao }),
            criado_por: Some(usuario.id),
            origem: "qualidade",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Muda a situação do evento (gerência): recebido -> em_analise -> concluido, atômico.
#[utoipa::path(post, operation_id = "situacao_do_evento", path = "/v1/qualidade/eventos/{id}/situacao", tag = "qualidade", params(("id" = Uuid, Path)),
    request_body = SituacaoDoEvento,
    responses((status = 204, description = "Situação registrada"), (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn situacao_evento(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(s): Corpo<SituacaoDoEvento>,
) -> Result<StatusCode, ErroApi> {
    usuario.exigir(Ato::QualidadeTratar)?;
    let anterior = match s.situacao.as_str() {
        "em_analise" => "recebido",
        "concluido" => "em_analise",
        _ => {
            return Err(ErroApi::invalido(
                "situacao-invalida",
                "A situação deve ser em_analise ou concluido. Nada foi alterado.",
            ))
        }
    };
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let feito = sqlx::query_scalar!(
        "UPDATE qualidade.evento SET situacao = $2, versao = versao + 1 WHERE id = $1 AND situacao = $3 RETURNING codigo",
        id, s.situacao, anterior
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(codigo) = feito else {
        let existe = sqlx::query_scalar!("SELECT situacao FROM qualidade.evento WHERE id = $1", id)
            .fetch_optional(&mut *tx)
            .await?;
        return Err(match existe {
            None => ErroApi::nao_encontrado(
                "evento-nao-encontrado",
                "Evento não encontrado. Nada foi alterado.",
            ),
            Some(_) => ErroApi::conflito(
                "evento-fora-de-ordem",
                "O evento não está na etapa anterior a esta. Nada foi alterado.",
            ),
        });
    };
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "qualidade",
            tipo: "evento_situacao",
            registro_id: id,
            registro_codigo: Some(&codigo),
            dados: json!({ "situacao": s.situacao }),
            criado_por: Some(usuario.id),
            origem: "qualidade",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
