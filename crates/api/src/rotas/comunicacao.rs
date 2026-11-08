//! Avisos ao paciente por WhatsApp (docs/25 MVP: confirmação, laudo pronto, opt-out). Tudo sai pelo
//! outbox (G3) com chave de idempotência; cada mensagem fica registrada (docs/21), inclusive as não
//! enviadas por opt-out ou falta de celular. A mensagem não leva dado de saúde: só primeiro nome, data
//! e hora, e o aviso de que há resultado no portal.

use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::lastro::{self, NovoEvento};
use crate::mensageiro::MensagemWhatsapp;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

fn sistema(unidade_id: Uuid) -> ContextoBanco {
    ContextoBanco {
        usuario_id: None,
        unidade_id,
        perfis: vec!["sistema".into()],
    }
}

/// Celular do cadastro (10 ou 11 dígitos) no formato internacional do Brasil.
#[must_use]
pub fn internacional(celular: &str) -> String {
    format!("55{celular}")
}

/// Número que chega do WhatsApp (55 + DDD + número) no formato do cadastro.
#[must_use]
pub fn do_whatsapp(numero: &str) -> Option<String> {
    let d: String = numero.chars().filter(char::is_ascii_digit).collect();
    let local = d.strip_prefix("55").unwrap_or(&d).to_owned();
    (10..=11).contains(&local.len()).then_some(local)
}

/// Trabalhador do outbox: envia (ou registra por que não enviou) um aviso. Sem mensageiro configurado
/// devolve erro e o item fica no outbox, visível, para tentar depois.
///
/// # Errors
/// Mensageiro não configurado, falha do provedor ou do banco.
pub async fn enviar_aviso(
    estado: &Estado,
    unidade_id: Uuid,
    modelo: &str,
    referencia: Uuid,
    chave: Uuid,
) -> anyhow::Result<()> {
    let mut tx = banco::transacao(&estado.banco, &sistema(unidade_id)).await?;
    let ja = sqlx::query_scalar!(
        "SELECT situacao FROM comunicacao.mensagem WHERE modelo = $1 AND referencia = $2",
        modelo,
        referencia
    )
    .fetch_optional(&mut *tx)
    .await?;
    if ja.is_some() {
        tx.commit().await?;
        return Ok(());
    }
    let alvo = match modelo {
        "laudo_pronto" => sqlx::query!(
            r#"SELECT p.id, p.nome, p.celular, NULL::timestamptz AS "inicio?" FROM portal.publicacao u JOIN identidade.paciente p ON p.id = u.paciente_id WHERE u.id = $1"#,
            referencia
        )
        .fetch_optional(&mut *tx)
        .await?
        .map(|l| (l.id, l.nome, l.celular, l.inicio)),
        "confirmacao" | "reserva_expirada" => sqlx::query!(
            r#"SELECT p.id, p.nome, p.celular, g.inicio AS "inicio?" FROM agenda.agendamento g JOIN identidade.paciente p ON p.id = g.paciente_id WHERE g.id = $1"#,
            referencia
        )
        .fetch_optional(&mut *tx)
        .await?
        .map(|l| (l.id, l.nome, l.celular, l.inicio)),
        outro => anyhow::bail!("modelo desconhecido: {outro}"),
    };
    let Some((paciente_id, nome, celular, inicio)) = alvo else {
        anyhow::bail!("referência do aviso não encontrada")
    };
    let permitido = sqlx::query_scalar!(
        "SELECT whatsapp FROM comunicacao.preferencia WHERE paciente_id = $1",
        paciente_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(true);
    let mut provedor_id = None;
    let situacao = match (permitido, celular) {
        (false, _) => "bloqueada_opt_out",
        (true, None) => "sem_celular",
        (true, Some(celular)) => {
            let mensageiro = estado
                .mensageiro
                .clone()
                .ok_or_else(|| anyhow::anyhow!("WhatsApp não configurado"))?;
            let primeiro = nome
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_owned();
            let mut parametros = vec![primeiro];
            if let Some(i) = inicio {
                let local = i.with_timezone(&tempo::FUSO);
                parametros.push(local.format("%d/%m").to_string());
                parametros.push(local.format("%H:%M").to_string());
            }
            let m = MensagemWhatsapp {
                para: internacional(&celular),
                modelo: modelo.to_owned(),
                parametros,
            };
            provedor_id = Some(mensageiro.enviar(chave, &m).await?);
            "enviada"
        }
    };
    sqlx::query!(
        "INSERT INTO comunicacao.mensagem (id, paciente_id, canal, modelo, referencia, situacao, chave_idempotencia, provedor_id, origem, unidade_id, correlacao)
         VALUES ($1, $2, 'whatsapp', $3, $4, $5, $6, $7, 'sistema', $8, $9) ON CONFLICT (modelo, referencia) DO NOTHING",
        novo_id(), paciente_id, modelo, referencia, situacao, chave, provedor_id, unidade_id, chave.to_string()
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "comunicacao",
            tipo: "aviso_whatsapp",
            registro_id: referencia,
            registro_codigo: None,
            dados: json!({ "modelo": modelo, "situacao": situacao }),
            criado_por: None,
            origem: "sistema",
            unidade_id,
            correlacao: &chave.to_string(),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Trabalhador do outbox: resposta do paciente ("SAIR" desliga; "1"/"SIM" confirma o próximo horário).
///
/// # Errors
/// Falha de banco.
pub async fn processar_resposta(
    estado: &Estado,
    unidade_id: Uuid,
    de: &str,
    texto: &str,
    evento: &str,
) -> anyhow::Result<()> {
    let Some(celular) = do_whatsapp(de) else {
        return Ok(());
    };
    let palavra = texto.trim().to_uppercase();
    let mut tx = banco::transacao(&estado.banco, &sistema(unidade_id)).await?;
    let pacientes: Vec<Uuid> = sqlx::query_scalar!(
        "SELECT id FROM identidade.paciente WHERE celular = $1",
        celular
    )
    .fetch_all(&mut *tx)
    .await?;
    if matches!(
        palavra.as_str(),
        "SAIR" | "PARAR" | "STOP" | "CANCELAR AVISOS"
    ) {
        // O número pode ser de mais de um paciente (família): desliga para todos com ele.
        for p in &pacientes {
            sqlx::query!(
                "INSERT INTO comunicacao.preferencia (paciente_id, whatsapp, motivo, origem, unidade_id, correlacao)
                 VALUES ($1, false, 'resposta SAIR pelo WhatsApp', 'whatsapp', $2, $3)
                 ON CONFLICT (paciente_id) DO UPDATE SET whatsapp = false, motivo = EXCLUDED.motivo, alterado_em = now(),
                        versao = comunicacao.preferencia.versao + 1",
                p, unidade_id, evento
            )
            .execute(&mut *tx)
            .await?;
            lastro::registrar(
                &mut tx,
                NovoEvento {
                    dominio: "comunicacao",
                    tipo: "opt_out_whatsapp",
                    registro_id: *p,
                    registro_codigo: None,
                    dados: json!({ "origem": "resposta do paciente" }),
                    criado_por: None,
                    origem: "whatsapp",
                    unidade_id,
                    correlacao: evento,
                },
            )
            .await?;
        }
    } else if matches!(palavra.as_str(), "1" | "SIM" | "CONFIRMO" | "CONFIRMAR") {
        let alvo = sqlx::query!(
            "SELECT m.id, m.referencia FROM comunicacao.mensagem m JOIN agenda.agendamento g ON g.id = m.referencia
              WHERE m.paciente_id = ANY($1) AND m.modelo = 'confirmacao' AND m.situacao = 'enviada' AND m.respondida_em IS NULL
                AND g.inicio > now() AND g.situacao = 'agendado'
              ORDER BY g.inicio LIMIT 1",
            &pacientes
        )
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(a) = alvo {
            let confirmou = sqlx::query!("UPDATE agenda.agendamento SET situacao = 'confirmado', versao = versao + 1 WHERE id = $1 AND situacao = 'agendado'", a.referencia)
                .execute(&mut *tx)
                .await?
                .rows_affected();
            sqlx::query!("UPDATE comunicacao.mensagem SET respondida_em = now(), resposta = 'confirmou', versao = versao + 1 WHERE id = $1", a.id)
                .execute(&mut *tx)
                .await?;
            if confirmou == 1 {
                lastro::registrar(
                    &mut tx,
                    NovoEvento {
                        dominio: "agenda",
                        tipo: "presenca_confirmada",
                        registro_id: a.referencia,
                        registro_codigo: None,
                        dados: json!({ "canal": "whatsapp" }),
                        criado_por: None,
                        origem: "whatsapp",
                        unidade_id,
                        correlacao: evento,
                    },
                )
                .await?;
            }
        }
    }
    tx.commit().await?;
    Ok(())
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoLembretes {
    /// Dia dos agendamentos a confirmar (AAAA-MM-DD).
    pub data: NaiveDate,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LembretesEnfileirados {
    pub enfileirados: i64,
}

/// Enfileira a confirmação por WhatsApp dos agendamentos do dia (repetir não manda de novo).
#[utoipa::path(post, path = "/v1/comunicacao/lembretes", tag = "comunicacao", request_body = PedidoLembretes,
    responses((status = 200, body = LembretesEnfileirados), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn lembretes(
    State(estado): State<Estado>,
    usuario: Usuario,
    Corpo(p): Corpo<PedidoLembretes>,
) -> Result<Json<LembretesEnfileirados>, ErroApi> {
    usuario.exigir(Ato::ComunicacaoEnviar)?;
    let (de, ate) = tempo::limites_do_dia(p.data)
        .ok_or_else(|| ErroApi::invalido("data-invalida", "Data inválida."))?;
    if ate <= Utc::now() {
        return Err(ErroApi::invalido(
            "dia-passado",
            "Só dá para pedir confirmação de horários que ainda vão acontecer. Nada foi enviado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let n = sqlx::query!(
        "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
         SELECT gen_random_uuid(), 'aviso.confirmacao', gen_random_uuid(), jsonb_build_object('agendamento_id', g.id), g.unidade_id, $3, g.id
           FROM agenda.agendamento g WHERE g.inicio >= $1 AND g.inicio < $2 AND g.inicio > now() AND g.situacao = 'agendado'
         ON CONFLICT (tipo, referencia) WHERE referencia IS NOT NULL DO NOTHING",
        de, ate, correlacao
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok(Json(LembretesEnfileirados {
        enfileirados: i64::try_from(n).unwrap_or(i64::MAX),
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct PreferenciaWhatsapp {
    pub whatsapp: bool,
    pub motivo: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MensagemAoPaciente {
    pub modelo: String,
    pub situacao: String,
    pub resposta: Option<String>,
    pub criado_em: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ComunicacaoDoPaciente {
    pub whatsapp: bool,
    pub motivo: Option<String>,
    pub mensagens: Vec<MensagemAoPaciente>,
}

async fn ler(tx: &mut sqlx::PgConnection, id: Uuid) -> Result<ComunicacaoDoPaciente, ErroApi> {
    sqlx::query_scalar!("SELECT id FROM identidade.paciente WHERE id = $1", id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            ErroApi::nao_encontrado("paciente-nao-encontrado", "Paciente não encontrado.")
        })?;
    let p = sqlx::query!(
        "SELECT whatsapp, motivo FROM comunicacao.preferencia WHERE paciente_id = $1",
        id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let mensagens = sqlx::query_as!(
        MensagemAoPaciente,
        "SELECT modelo, situacao, resposta, criado_em FROM comunicacao.mensagem WHERE paciente_id = $1 ORDER BY criado_em DESC LIMIT 50",
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(ComunicacaoDoPaciente {
        whatsapp: p.as_ref().is_none_or(|p| p.whatsapp),
        motivo: p.map(|p| p.motivo),
        mensagens,
    })
}

/// Preferência e histórico de mensagens do paciente (recepção e gerência).
#[utoipa::path(get, path = "/v1/pacientes/{id}/comunicacao", tag = "comunicacao", params(("id" = Uuid, Path)),
    responses((status = 200, body = ComunicacaoDoPaciente), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn do_paciente(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<ComunicacaoDoPaciente>, ErroApi> {
    usuario.exigir(Ato::ComunicacaoEnviar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = ler(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(r))
}

/// Liga ou desliga o WhatsApp do paciente, com motivo (cada mudança sobe a versão e vai ao lastro).
#[utoipa::path(post, path = "/v1/pacientes/{id}/comunicacao", tag = "comunicacao", params(("id" = Uuid, Path)), request_body = PreferenciaWhatsapp,
    responses((status = 200, body = ComunicacaoDoPaciente), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn preferir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PreferenciaWhatsapp>,
) -> Result<Json<ComunicacaoDoPaciente>, ErroApi> {
    usuario.exigir(Ato::ComunicacaoEnviar)?;
    if p.motivo.trim().chars().count() < 3 {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Diga o motivo (ex.: pedido do paciente no balcão). Nada foi alterado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    ler(&mut tx, id).await?;
    sqlx::query!(
        "INSERT INTO comunicacao.preferencia (paciente_id, whatsapp, motivo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, 'recepcao', $5, $6)
         ON CONFLICT (paciente_id) DO UPDATE SET whatsapp = EXCLUDED.whatsapp, motivo = EXCLUDED.motivo, alterado_em = now(),
                versao = comunicacao.preferencia.versao + 1",
        id, p.whatsapp, p.motivo.trim(), usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "comunicacao",
            tipo: if p.whatsapp {
                "opt_in_whatsapp"
            } else {
                "opt_out_whatsapp"
            },
            registro_id: id,
            registro_codigo: None,
            dados: json!({ "motivo": p.motivo.trim() }),
            criado_por: Some(usuario.id),
            origem: "recepcao",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let r = ler(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(r))
}
