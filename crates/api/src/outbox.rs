//! Trabalhadores do núcleo (docs/11 § 6, ADR-0007):
//! - outbox: entrega a worklist ao Orthanc com retentativa e backoff, fora da transação do negócio;
//! - conciliação: lê os estudos estáveis do Orthanc e liga cada um ao atendimento pelo accession
//!   (REQ-EXA-002). Estudo sem atendimento fica pendente e nunca vai para laudo.

use crate::banco::{self, ContextoBanco};
use crate::estado::Estado;
use crate::eventos::EventoTela;
use crate::lastro::{self, NovoEvento};
use crate::orthanc::{self, ItemWorklist, Orthanc};
use anyhow::{Context, Result};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};

const MAX_TENTATIVAS: i32 = 12;

/// Processa um item do outbox, se houver. Devolve `true` quando processou algum.
///
/// # Errors
/// Falha de banco ao reservar ou concluir o item.
pub async fn processar_um(estado: &Estado, orthanc: &Orthanc) -> Result<bool> {
    // Reserva com prazo (lease): outro trabalhador não pega o mesmo item; se este cair, o item volta.
    let Some(item) = sqlx::query!(
        "UPDATE integracao.outbox o SET tentativas = o.tentativas + 1, proximo_em = now() + interval '2 minutes'
          WHERE o.id = (SELECT id FROM integracao.outbox
                         WHERE concluido_em IS NULL AND proximo_em <= now() AND tentativas < $1
                         ORDER BY criado_em LIMIT 1 FOR UPDATE SKIP LOCKED)
          RETURNING o.id, o.tipo, o.carga, o.unidade_id, o.tentativas, o.chave_idempotencia",
        MAX_TENTATIVAS
    )
    .fetch_optional(&estado.banco)
    .await?
    else {
        return Ok(false);
    };

    let resultado = match item.tipo.as_str() {
        "worklist.criar" => criar_worklist(estado, orthanc, item.unidade_id, &item.carga).await,
        "portal.publicar" => match item.carga["laudo_id"].as_str().and_then(|v| v.parse().ok()) {
            Some(laudo) => crate::laudos::publicar(estado, laudo, item.unidade_id)
                .await
                .map(|_| ())
                .map_err(|e| anyhow::anyhow!("publicação no portal: {}", e.codigo())),
            None => Err(anyhow::anyhow!("carga sem laudo_id")),
        },
        "pix.criar_cobranca" => match item.carga["recebimento_id"]
            .as_str()
            .and_then(|v| v.parse().ok())
        {
            Some(r) => crate::rotas::caixa::criar_cobranca(estado, item.unidade_id, r).await,
            None => Err(anyhow::anyhow!("carga sem recebimento_id")),
        },
        "pix.processar_evento" => match item.carga["txid"].as_str() {
            Some(txid) => {
                crate::rotas::caixa::processar_evento_pix(estado, item.unidade_id, txid).await
            }
            None => Err(anyhow::anyhow!("carga sem txid")),
        },
        "email.convite_portal" => match item.carga["convite_id"]
            .as_str()
            .and_then(|v| v.parse().ok())
        {
            Some(c) => {
                crate::rotas::portal_acesso::enviar_convite(
                    estado,
                    item.unidade_id,
                    c,
                    item.chave_idempotencia,
                )
                .await
            }
            None => Err(anyhow::anyhow!("carga sem convite_id")),
        },
        "nfse.emitir" | "nfse.cancelar" => match item.carga["recebimento_id"]
            .as_str()
            .and_then(|v| v.parse().ok())
        {
            Some(r) if item.tipo == "nfse.emitir" => {
                crate::rotas::nfse::emitir(estado, item.unidade_id, r, item.chave_idempotencia)
                    .await
            }
            Some(r) => {
                crate::rotas::nfse::cancelar(estado, item.unidade_id, r, item.chave_idempotencia)
                    .await
            }
            None => Err(anyhow::anyhow!("carga sem recebimento_id")),
        },
        "tiss.enviar_lote" => match item.carga["lote_id"].as_str().and_then(|v| v.parse().ok()) {
            Some(lote) => {
                crate::rotas::faturamento::lotes::enviar_pelo_outbox(
                    estado,
                    item.unidade_id,
                    lote,
                    item.chave_idempotencia,
                )
                .await
            }
            None => Err(anyhow::anyhow!("carga sem lote_id")),
        },
        // Recurso de glosa por webservice (tissRecursoGlosa) entra numa próxima entrega: fica
        // pendente no outbox, visível, sem se perder (R7).
        "tiss.solicitar_autorizacao" => match item.carga["autorizacao_id"]
            .as_str()
            .and_then(|v| v.parse().ok())
        {
            Some(a) => {
                crate::rotas::autorizacoes_resposta::enviar_pelo_outbox(
                    estado,
                    item.unidade_id,
                    a,
                    item.chave_idempotencia,
                )
                .await
            }
            None => Err(anyhow::anyhow!("carga sem autorizacao_id")),
        },
        "tiss.consultar_autorizacao" => match item.carga["autorizacao_id"]
            .as_str()
            .and_then(|v| v.parse().ok())
        {
            Some(a) => {
                crate::rotas::autorizacoes_resposta::consultar_pelo_outbox(
                    estado,
                    item.unidade_id,
                    a,
                )
                .await
            }
            None => Err(anyhow::anyhow!("carga sem autorizacao_id")),
        },
        "tiss.recurso_glosa" => Err(anyhow::anyhow!(
            "envio de recurso de glosa por webservice ainda não configurado"
        )),
        "pix.estornar" => match (estado.psp.clone(), item.carga["txid"].as_str()) {
            (Some(psp), Some(txid)) => psp.estornar(item.chave_idempotencia, txid).await,
            _ => Err(anyhow::anyhow!("PSP não configurado ou carga sem txid")),
        },
        "aviso.reserva_expirada" | "aviso.confirmacao" => match item.carga["agendamento_id"]
            .as_str()
            .and_then(|v| v.parse().ok())
        {
            Some(g) => {
                let modelo = if item.tipo == "aviso.confirmacao" {
                    "confirmacao"
                } else {
                    "reserva_expirada"
                };
                crate::rotas::comunicacao::enviar_aviso(
                    estado,
                    item.unidade_id,
                    modelo,
                    g,
                    item.chave_idempotencia,
                )
                .await
            }
            None => Err(anyhow::anyhow!("carga sem agendamento_id")),
        },
        "aviso.laudo_pronto" => match item.carga["publicacao_id"]
            .as_str()
            .and_then(|v| v.parse().ok())
        {
            Some(p) => {
                crate::rotas::comunicacao::enviar_aviso(
                    estado,
                    item.unidade_id,
                    "laudo_pronto",
                    p,
                    item.chave_idempotencia,
                )
                .await
            }
            None => Err(anyhow::anyhow!("carga sem publicacao_id")),
        },
        "whatsapp.resposta" => match (
            item.carga["de"].as_str(),
            item.carga["texto"].as_str(),
            item.carga["evento"].as_str(),
        ) {
            (Some(de), Some(texto), Some(ev)) => {
                crate::rotas::comunicacao::processar_resposta(
                    estado,
                    item.unidade_id,
                    de,
                    texto,
                    ev,
                )
                .await
            }
            _ => Err(anyhow::anyhow!("carga de resposta incompleta")),
        },
        // Oferta à lista de espera, aviso interno de emergência e NFS-e: o provedor entra numa próxima
        // entrega. Ficam pendentes no outbox, visíveis, sem se perder (R7).
        "lista_espera.oferecer_vaga" | "aviso.acesso_emergencia" => Err(anyhow::anyhow!(
            "provedor de {} ainda não configurado",
            item.tipo
        )),
        outro => Err(anyhow::anyhow!("tipo de outbox desconhecido: {outro}")),
    };

    match resultado {
        Ok(()) => {
            sqlx::query!(
                "UPDATE integracao.outbox SET concluido_em = now(), ultimo_erro = NULL WHERE id = $1",
                item.id
            )
            .execute(&estado.banco)
            .await?;
        }
        Err(erro) => {
            // Backoff exponencial com teto (SEC-DIS-7). O erro não leva dado de paciente.
            let espera = 2_i64.saturating_pow(u32::try_from(item.tentativas.min(10)).unwrap_or(10));
            tracing::warn!(outbox = %item.id, tentativas = item.tentativas, erro = %erro, "entrega do outbox falhou");
            sqlx::query!(
                "UPDATE integracao.outbox SET ultimo_erro = $2, proximo_em = now() + make_interval(secs => $3)
                  WHERE id = $1",
                item.id,
                erro.to_string().chars().take(500).collect::<String>(),
                f64::from(i32::try_from(espera.min(3600)).unwrap_or(3600))
            )
            .execute(&estado.banco)
            .await?;
        }
    }
    Ok(true)
}

async fn criar_worklist(
    estado: &Estado,
    orthanc: &Orthanc,
    unidade_id: Uuid,
    carga: &serde_json::Value,
) -> Result<()> {
    let atendimento_id: Uuid = carga["atendimento_id"]
        .as_str()
        .context("carga sem atendimento_id")?
        .parse()?;
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    let aparelho_id: Option<Uuid> = carga["aparelho_id"].as_str().and_then(|v| v.parse().ok());
    // A worklist vai para o AE Title do aparelho bipado na sala; sem aparelho, o da sala.
    let d = sqlx::query!(
        r#"SELECT t.accession, p.codigo AS paciente_codigo, p.nome, p.nascimento,
                  COALESCE(ap.ae_title, s.ae_title) AS ae_title,
                  COALESCE(ap.modalidade, s.modalidade) AS "modalidade!", a.inicio
             FROM recepcao.atendimento t
             JOIN identidade.paciente p ON p.id = t.paciente_id
             JOIN agenda.sala s ON s.id = t.sala_id
             JOIN agenda.agendamento a ON a.id = t.agendamento_id
             LEFT JOIN agenda.aparelho ap ON ap.id = $2
            WHERE t.id = $1"#,
        atendimento_id,
        aparelho_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .context("atendimento do outbox não encontrado")?;
    tx.commit().await?;

    let nome = orthanc::nome_dicom(&d.nome);
    let nascimento = d.nascimento.format("%Y%m%d").to_string();
    let data = d.inicio.format("%Y%m%d").to_string();
    orthanc
        .criar_worklist(&ItemWorklist {
            accession: &d.accession,
            paciente_codigo: &d.paciente_codigo,
            paciente_nome_dicom: &nome,
            nascimento: &nascimento,
            ae_title: d.ae_title.as_deref().unwrap_or("VIDACOR"),
            modalidade: &d.modalidade,
            data_agendada: &data,
        })
        .await
}

/// Lê as mudanças do Orthanc desde o cursor e concilia os estudos estáveis.
///
/// # Errors
/// Falha de banco ou do Orthanc.
pub async fn conciliar(estado: &Estado, orthanc: &Orthanc, unidade_id: Uuid) -> Result<()> {
    let desde = sqlx::query_scalar!(
        "INSERT INTO integracao.cursor_orthanc (unidade_id) VALUES ($1)
         ON CONFLICT (unidade_id) DO UPDATE SET unidade_id = EXCLUDED.unidade_id
         RETURNING ultima_mudanca",
        unidade_id
    )
    .fetch_one(&estado.banco)
    .await?;
    let mudancas = orthanc.mudancas(desde).await?;
    for mudanca in mudancas.itens.iter().filter(|m| m.tipo == "StableStudy") {
        let estudo = orthanc.estudo(&mudanca.id).await?;
        registrar_estudo(estado, unidade_id, &estudo).await?;
        sqlx::query!(
            "UPDATE integracao.cursor_orthanc SET ultima_mudanca = $2, atualizado_em = now() WHERE unidade_id = $1",
            unidade_id,
            mudanca.seq
        )
        .execute(&estado.banco)
        .await?;
    }
    sqlx::query!(
        "UPDATE integracao.cursor_orthanc SET ultima_mudanca = GREATEST(ultima_mudanca, $2), atualizado_em = now()
          WHERE unidade_id = $1",
        unidade_id,
        mudancas.ultima
    )
    .execute(&estado.banco)
    .await?;
    Ok(())
}

/// Grava o estudo e o concilia pelo accession. Repetir não duplica (study_uid único, G3).
///
/// # Errors
/// Falha de banco.
pub async fn registrar_estudo(
    estado: &Estado,
    unidade_id: Uuid,
    estudo: &orthanc::EstudoOrthanc,
) -> Result<()> {
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    // Estudo feito em contingência (Patient ID "C-NNNN") nunca é conciliado sozinho: vai para a fila
    // própria e só entra no laudo depois da conferência com nome e nascimento (FC-CTG-08).
    let de_contingencia = estudo
        .patient_id
        .as_deref()
        .is_some_and(|p| p.starts_with("C-"));
    let atendimento = match &estudo.accession {
        Some(_) if de_contingencia => None,
        Some(acc) => {
            sqlx::query!(
                "SELECT id, codigo FROM recepcao.atendimento WHERE accession = $1",
                acc
            )
            .fetch_optional(&mut *tx)
            .await?
        }
        None => None,
    };
    let ano = crate::tempo::ano_local();
    let numero = sqlx::query_scalar!("SELECT comum.proximo_numero('EST', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .context("contador EST sem valor")?;
    let codigo = CodigoLegivel::novo("EST", ano, numero);
    let correlacao = format!("orthanc:{}", estudo.orthanc_id);
    let gravado = sqlx::query!(
        r#"INSERT INTO exames.estudo
             (id, codigo, study_uid, orthanc_id, accession, atendimento_id, imagens, conciliado,
              origem, unidade_id, correlacao, pendencia)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'aparelho', $9, $10, $11)
           ON CONFLICT (study_uid) DO UPDATE SET imagens = EXCLUDED.imagens, versao = exames.estudo.versao + 1
           RETURNING id, (xmax = 0) AS "novo!""#,
        novo_id(),
        codigo.como_str(),
        estudo.study_uid,
        estudo.orthanc_id,
        estudo.accession,
        atendimento.as_ref().map(|a| a.id),
        estudo.imagens,
        atendimento.is_some(),
        unidade_id,
        correlacao,
        if atendimento.is_some() {
            None
        } else if de_contingencia {
            Some("conciliacao_contingencia")
        } else {
            Some("conciliacao")
        }
    )
    .fetch_one(&mut *tx)
    .await?;

    if gravado.novo {
        if let Some(a) = &atendimento {
            // Estudo conciliado entra na fila de laudo do médico do agendamento (FC-LAU-09).
            crate::laudos::criar_para_estudo(&mut tx, gravado.id, a.id, unidade_id, &correlacao)
                .await?;
        }
        lastro::registrar(
            &mut tx,
            NovoEvento {
                dominio: "exames",
                tipo: if atendimento.is_some() {
                    "estudo_conciliado"
                } else {
                    "estudo_sem_conciliacao"
                },
                registro_id: gravado.id,
                registro_codigo: Some(codigo.como_str()),
                dados: json!({
                    "study_uid": estudo.study_uid,
                    "accession": estudo.accession,
                    "atendimento": atendimento.as_ref().map(|a| a.codigo.clone()),
                    "imagens": estudo.imagens
                }),
                criado_por: None,
                origem: "aparelho",
                unidade_id,
                correlacao: &correlacao,
            },
        )
        .await?;
    }
    tx.commit().await?;

    if let Some(a) = atendimento {
        let _ = estado.eventos.send(EventoTela {
            unidade_id,
            tipo: "estudo-recebido",
            perfis: &["recepcao", "tecnico", "medico", "gerente"],
            dados: json!({ "atendimento_id": a.id, "accession": estudo.accession, "imagens": estudo.imagens }),
        });
    }
    Ok(())
}

/// Laço dos trabalhadores. Erros são registrados e o laço continua (o atendimento não para, R7).
pub async fn rodar(estado: Estado, orthanc: Orthanc) {
    let unidade = estado.config.unidade_servidor;
    loop {
        loop {
            match processar_um(&estado, &orthanc).await {
                Ok(true) => continue,
                Ok(false) => break,
                Err(e) => {
                    tracing::error!(erro = %e, "trabalhador do outbox");
                    break;
                }
            }
        }
        if let Some(u) = unidade {
            // Kit de contingência do dia a partir das 06:00 (docs/23 § 5); repetir não gera outro.
            let agora = chrono::Utc::now().with_timezone(&chrono_tz::America::Sao_Paulo);
            if chrono::Timelike::hour(&agora) >= 6 {
                if let Err(e) =
                    crate::rotas::contingencia::gerar_kit(&estado, u, agora.date_naive(), None)
                        .await
                {
                    tracing::warn!(erro = ?e, "kit de contingência do dia");
                }
            }
            if let Err(e) = crate::rotas::agendamentos::expirar_reservas(&estado, u).await {
                tracing::warn!(erro = ?e, "expiração de reservas provisórias");
            }
            if let Err(e) = conciliar(&estado, &orthanc, u).await {
                tracing::warn!(erro = %e, "conciliação com o Orthanc");
            }
        }
        tokio::select! {
            () = estado.outbox_aviso.notified() => {}
            () = tokio::time::sleep(Duration::from_secs(2)) => {}
        }
    }
}
