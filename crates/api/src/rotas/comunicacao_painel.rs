//! Comunicação com pacientes (Comunicacao.dc.html), corte do MVP (docs/25: confirmação, laudo pronto,
//! opt-out): registro das mensagens enviadas com situação e resposta, e os avisos automáticos que a
//! gerência liga ou desliga com motivo. Como o Paciente Conectado da Medware e o MediAutonotify da
//! Medilab (docs/03), cada aviso tem chave própria; a diferença é que cada mudança é uma versão com
//! motivo no lastro (DEC-2026-11-09-131 e 132). A mensagem nunca leva dado de saúde.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::novo_id;

/// Avisos automáticos que existem no sistema (o que cada um diz e quando sai).
pub const AVISOS: [(&str, &str, &str); 4] = [
    (
        "confirmacao",
        "Confirmação do horário",
        "Sai quando a recepção pede a confirmação dos horários do dia. O paciente responde SIM para confirmar ou SAIR para não receber mais.",
    ),
    (
        "laudo_pronto",
        "Laudo pronto no portal",
        "Sai quando o laudo assinado é publicado no portal. Não leva o nome do exame nem o resultado.",
    ),
    (
        "reserva_expirada",
        "Reserva sem guia expirou",
        "Sai quando a reserva de convênio vence sem a guia; o paciente é convidado a ligar para remarcar.",
    ),
    (
        "oferta_vaga",
        "Vaga da lista de espera",
        "Sai quando uma vaga é oferecida a quem está na lista de espera, com dia e hora.",
    ),
];

#[derive(Deserialize, IntoParams)]
pub struct FiltroDeMensagens {
    /// todas, nao_entregues (opt-out, sem celular, aviso desligado) ou confirmaram.
    pub filtro: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct MensagemDoRegistro {
    pub id: Uuid,
    pub paciente_id: Uuid,
    pub nome: String,
    pub nascimento: NaiveDate,
    pub modelo: String,
    /// enviada, bloqueada_opt_out, sem_celular ou modelo_desligado.
    pub situacao: String,
    /// confirmou, saiu ou nenhuma.
    pub resposta: Option<String>,
    pub criado_em: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct AvisoAutomatico {
    pub modelo: String,
    pub nome: String,
    pub descricao: String,
    pub ativo: bool,
    pub motivo: Option<String>,
    pub alterado_em: Option<DateTime<Utc>>,
    pub alterado_por: Option<String>,
    /// Versão vigente (informar na mudança). Sem versão: ligado desde a instalação.
    pub versao_id: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
pub struct ContagemDeHoje {
    pub enviadas: i64,
    pub confirmaram: i64,
    pub opt_out: i64,
    pub sem_celular: i64,
    pub desligadas: i64,
}

#[derive(Serialize, ToSchema)]
pub struct PainelDeComunicacao {
    /// O núcleo tem o provedor de WhatsApp configurado (sem ele, os avisos ficam no outbox).
    pub whatsapp_configurado: bool,
    pub hoje: ContagemDeHoje,
    /// Últimos 7 dias, até 200, mais recentes primeiro.
    pub mensagens: Vec<MensagemDoRegistro>,
    pub automaticas: Vec<AvisoAutomatico>,
    /// Pacientes com WhatsApp desligado (opt-out) na unidade.
    pub pacientes_sem_whatsapp: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct MudancaDeAviso {
    pub ativo: bool,
    pub motivo: String,
    /// Versão vista na tela (`versao_id`); `null` quando o aviso nunca foi mudado.
    pub versao_vista: Option<Uuid>,
}

/// Registro de mensagens e avisos automáticos (recepção e gerência).
#[utoipa::path(get, operation_id = "painel_de_comunicacao", path = "/v1/comunicacao", tag = "comunicacao", params(FiltroDeMensagens),
    responses((status = 200, body = PainelDeComunicacao), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn painel(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroDeMensagens>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::ComunicacaoEnviar)?;
    let filtro = f.filtro.as_deref().unwrap_or("todas");
    if !["todas", "nao_entregues", "confirmaram"].contains(&filtro) {
        return Err(ErroApi::invalido(
            "filtro-invalido",
            "Filtro desconhecido. Use todas, não entregues ou confirmaram.",
        ));
    }
    let hoje = tempo::hoje_local();
    let desde = Utc::now() - Duration::days(7);
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let c = sqlx::query!(
        r#"SELECT count(*) FILTER (WHERE situacao = 'enviada') AS "enviadas!",
                  count(*) FILTER (WHERE resposta = 'confirmou') AS "confirmaram!",
                  count(*) FILTER (WHERE situacao = 'bloqueada_opt_out') AS "opt_out!",
                  count(*) FILTER (WHERE situacao = 'sem_celular') AS "sem_celular!",
                  count(*) FILTER (WHERE situacao = 'modelo_desligado') AS "desligadas!"
             FROM comunicacao.mensagem WHERE (criado_em AT TIME ZONE 'America/Sao_Paulo')::date = $1"#,
        hoje
    )
    .fetch_one(&mut *tx)
    .await?;
    let mensagens = sqlx::query_as!(
        MensagemDoRegistro,
        r#"SELECT m.id, m.paciente_id, p.nome, p.nascimento, m.modelo, m.situacao, m.resposta, m.criado_em
             FROM comunicacao.mensagem m JOIN identidade.paciente p ON p.id = m.paciente_id
            WHERE m.criado_em >= $1
              AND ($2 = 'todas' OR ($2 = 'nao_entregues' AND m.situacao <> 'enviada') OR ($2 = 'confirmaram' AND m.resposta = 'confirmou'))
            ORDER BY m.criado_em DESC LIMIT 200"#,
        desde,
        filtro
    )
    .fetch_all(&mut *tx)
    .await?;
    let vigentes = sqlx::query!(
        r#"SELECT x.id, x.modelo, x.ativo, x.motivo, x.criado_em, u.nome AS "por?"
             FROM comunicacao.envio_automatico x LEFT JOIN identidade.usuario u ON u.id = x.criado_por
            WHERE NOT EXISTS (SELECT 1 FROM comunicacao.envio_automatico s WHERE s.substitui = x.id)"#
    )
    .fetch_all(&mut *tx)
    .await?;
    let sem_whatsapp = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM comunicacao.preferencia WHERE NOT whatsapp"#
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let automaticas = AVISOS
        .iter()
        .map(|(modelo, nome, descricao)| {
            let v = vigentes.iter().find(|v| v.modelo == *modelo);
            AvisoAutomatico {
                modelo: (*modelo).to_owned(),
                nome: (*nome).to_owned(),
                descricao: (*descricao).to_owned(),
                ativo: v.is_none_or(|v| v.ativo),
                motivo: v.map(|v| v.motivo.clone()),
                alterado_em: v.map(|v| v.criado_em),
                alterado_por: v.and_then(|v| v.por.clone()),
                versao_id: v.map(|v| v.id),
            }
        })
        .collect();
    let painel = PainelDeComunicacao {
        whatsapp_configurado: estado.mensageiro.is_some(),
        hoje: ContagemDeHoje {
            enviadas: c.enviadas,
            confirmaram: c.confirmaram,
            opt_out: c.opt_out,
            sem_celular: c.sem_celular,
            desligadas: c.desligadas,
        },
        mensagens,
        automaticas,
        pacientes_sem_whatsapp: sem_whatsapp,
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(painel)).into_response())
}

/// Liga ou desliga um aviso automático da unidade (gerência), com motivo e a versão vista.
#[utoipa::path(post, operation_id = "mudar_aviso_automatico", path = "/v1/comunicacao/automaticas/{modelo}", tag = "comunicacao",
    params(("modelo" = String, Path), ("Idempotency-Key" = Uuid, Header)), request_body = MudancaDeAviso,
    responses((status = 200, body = AvisoAutomatico), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc),
              (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn mudar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(modelo): Path<String>,
    Corpo(m): Corpo<MudancaDeAviso>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::ComunicacaoConfigurar)?;
    let (_, nome, descricao) = AVISOS
        .iter()
        .find(|(id, _, _)| *id == modelo)
        .ok_or_else(|| {
            ErroApi::nao_encontrado("aviso-desconhecido", "Este aviso automático não existe.")
        })?;
    let motivo = m.motivo.trim().to_owned();
    if !(5..=300).contains(&motivo.chars().count()) {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Diga o motivo da mudança (5 a 300 letras). Nada mudou.",
        ));
    }
    let rota = format!("POST /v1/comunicacao/automaticas/{modelo}");
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
    let vigente = sqlx::query!(
        "SELECT id, ativo FROM comunicacao.envio_automatico x
          WHERE modelo = $1 AND NOT EXISTS (SELECT 1 FROM comunicacao.envio_automatico s WHERE s.substitui = x.id)",
        modelo
    )
    .fetch_optional(&mut *tx)
    .await?;
    let mudou_outra = || {
        ErroApi::conflito(
            "aviso-mudou",
            "Outra pessoa mudou este aviso enquanto você via a tela. Nada foi gravado; veja como está agora e decida de novo.",
        )
    };
    if vigente.as_ref().map(|v| v.id) != m.versao_vista {
        return Err(mudou_outra());
    }
    if vigente.as_ref().is_none_or(|v| v.ativo) == m.ativo {
        return Err(ErroApi::conflito(
            "aviso-ja-assim",
            if m.ativo {
                "Este aviso já está ligado. Nada mudou."
            } else {
                "Este aviso já está desligado. Nada mudou."
            },
        ));
    }
    let id = novo_id();
    let inserido = sqlx::query!(
        "INSERT INTO comunicacao.envio_automatico (id, modelo, ativo, motivo, substitui, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'gerencia', $7, $8, $9)",
        id, modelo, m.ativo, motivo, m.versao_vista, usuario.id, estacao, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await;
    if let Err(sqlx::Error::Database(e)) = &inserido {
        if e.is_unique_violation() {
            return Err(mudou_outra());
        }
    }
    inserido?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "comunicacao",
            tipo: if m.ativo {
                "aviso_automatico_ligado"
            } else {
                "aviso_automatico_desligado"
            },
            registro_id: id,
            registro_codigo: None,
            dados: json!({ "modelo": modelo, "substitui": m.versao_vista, "estacao": estacao }),
            criado_por: Some(usuario.id),
            origem: "gerencia",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(AvisoAutomatico {
        modelo: modelo.clone(),
        nome: (*nome).to_owned(),
        descricao: (*descricao).to_owned(),
        ativo: m.ativo,
        motivo: Some(motivo),
        alterado_em: Some(Utc::now()),
        alterado_por: Some(usuario.nome.clone()),
        versao_id: Some(id),
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 200, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::OK, Json(corpo)).into_response())
}
