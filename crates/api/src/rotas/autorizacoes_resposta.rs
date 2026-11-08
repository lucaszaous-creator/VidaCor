//! Resposta da autorização (docs/22 E4): gravada como recebida (bruta, com SHA-256) antes de ser
//! interpretada; a guia é preenchida a partir dela na mesma transação e, se a autorização é total e o
//! agendamento é uma reserva provisória, a reserva é confirmada com a guia. Também os trabalhos do
//! outbox: envio da solicitação e consulta do pedido em análise com espera crescente.

use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::lastro::{self, NovoEvento};
use chrono::Utc;
use chrono_tz::America::Sao_Paulo;
use serde_json::json;
use sha2::{Digest as _, Sha256};
use sqlx::PgConnection;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};
use vidacor_dominio_faturamento::autorizacao::{
    ler_resposta, RespostaAutorizacao, StatusAutorizacao,
};

/// Situações em que o pedido ainda aceita resposta.
pub(crate) const ABERTAS: [&str; 5] = [
    "aguardando_envio",
    "aguardando_portal",
    "enviada",
    "em_analise",
    "pendente_documento",
];

/// Resultado de gravar uma resposta.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Gravada {
    /// A mesma resposta já tinha sido gravada: nada mudou.
    Repetida,
    /// Gravada; situação do pedido depois dela.
    Nova(String),
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn motivo_de(r: &RespostaAutorizacao) -> Option<String> {
    let mut partes = Vec::new();
    if !r.motivos.is_empty() {
        partes.push(format!("Código(s) da operadora: {}", r.motivos.join(", ")));
    }
    if let Some(o) = &r.observacao {
        partes.push(o.clone());
    }
    let negados: Vec<String> = r
        .procedimentos
        .iter()
        .filter(|p| p.autorizado < p.solicitado)
        .map(|p| format!("{} ({} de {})", p.tuss, p.autorizado, p.solicitado))
        .collect();
    if !negados.is_empty() {
        partes.push(format!("Autorizado a menos: {}", negados.join(", ")));
    }
    (!partes.is_empty()).then(|| partes.join(". ").chars().take(1000).collect())
}

/// Grava a resposta (bruta e lida) e aplica ao pedido. `lida` vem do domínio (webservice) ou da
/// resposta copiada do portal. A mesma resposta repetida não duplica (constraint única por hash).
///
/// # Errors
/// Falha de banco ou guia com o mesmo número já registrada para a operadora.
#[allow(clippy::too_many_lines)]
pub(crate) async fn gravar_resposta(
    conexao: &mut PgConnection,
    autorizacao: Uuid,
    bruta: &[u8],
    lida: Result<RespostaAutorizacao, String>,
    origem: &str,
    usuario: Option<Uuid>,
    correlacao: &str,
) -> Result<Gravada, ErroApi> {
    let a = sqlx::query!(
        "SELECT codigo, situacao, agendamento_id, paciente_id, operadora, carteirinha, solicitante_nome, protocolo, unidade_id
           FROM convenio.autorizacao WHERE id = $1 FOR UPDATE",
        autorizacao
    )
    .fetch_one(&mut *conexao)
    .await?;
    let hash = sha256(bruta);
    let repetida = sqlx::query_scalar!(
        "SELECT 1 AS um FROM convenio.autorizacao_resposta WHERE autorizacao_id = $1 AND sha256 = $2",
        autorizacao,
        hash
    )
    .fetch_optional(&mut *conexao)
    .await?;
    if repetida.is_some() {
        return Ok(Gravada::Repetida);
    }
    // Resposta de outro pedido não muda este (fica gravada com o erro).
    let lida = lida.and_then(|r| match &r.numero_guia_prestador {
        Some(n) if n != &a.codigo => Err(format!("A resposta é do pedido {n}, não deste.")),
        _ => Ok(r),
    });
    let aberta = ABERTAS.contains(&a.situacao.as_str());
    let aplicar = aberta && lida.is_ok();
    let (situacao_lida, erro, hash_confere) = match &lida {
        Ok(r) => (Some(r.status.codigo()), None, Some(r.hash_confere)),
        Err(e) => (None, Some(e.chars().take(500).collect::<String>()), None),
    };
    sqlx::query!(
        "INSERT INTO convenio.autorizacao_resposta (id, autorizacao_id, origem_resposta, bruta, sha256, situacao_lida, erro_leitura,
                hash_confere, aplicada, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'integracao', $11, $12)",
        novo_id(), autorizacao, origem, bruta, hash, situacao_lida, erro, hash_confere, aplicar, usuario, a.unidade_id, correlacao
    )
    .execute(&mut *conexao)
    .await?;
    let Ok(r) = lida else {
        lastro::registrar(
            conexao,
            NovoEvento {
                dominio: "convenio",
                tipo: "autorizacao_resposta_ilegivel",
                registro_id: autorizacao,
                registro_codigo: Some(&a.codigo),
                dados: json!({ "sha256": hash, "origem": origem }),
                criado_por: usuario,
                origem: "integracao",
                unidade_id: a.unidade_id,
                correlacao,
            },
        )
        .await?;
        return Ok(Gravada::Nova(a.situacao));
    };
    if !aplicar {
        return Ok(Gravada::Nova(a.situacao));
    }
    for p in &r.procedimentos {
        sqlx::query!(
            "UPDATE convenio.autorizacao_procedimento SET autorizado = $3 WHERE autorizacao_id = $1 AND tuss = $2",
            autorizacao,
            p.tuss,
            i32::from(p.autorizado)
        )
        .execute(&mut *conexao)
        .await?;
    }
    let mut guia: Option<(Uuid, String)> = None;
    if matches!(
        r.status,
        StatusAutorizacao::Autorizada | StatusAutorizacao::Parcial
    ) {
        let (Some(senha), Some(validade)) = (&r.senha, r.validade) else {
            return Err(ErroApi::interno(
                "autorização sem senha passou pela leitura",
            ));
        };
        let numero = r
            .numero_guia_operadora
            .clone()
            .unwrap_or_else(|| a.codigo.clone());
        let id = novo_id();
        let ano = crate::tempo::ano_local();
        let n = sqlx::query_scalar!("SELECT comum.proximo_numero('GUI', $1)", ano)
            .fetch_one(&mut *conexao)
            .await?
            .ok_or_else(|| ErroApi::interno("contador GUI sem valor"))?;
        let codigo = CodigoLegivel::novo("GUI", ano, n);
        let criada = sqlx::query_scalar!(
            "INSERT INTO convenio.guia (id, codigo, operadora, numero, carteirinha, senha, validade, paciente_id, solicitante,
                    criado_por, origem, unidade_id, correlacao)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'autorizacao', $11, $12)
             ON CONFLICT (operadora, numero) DO NOTHING RETURNING id",
            id, codigo.como_str(), a.operadora, numero, a.carteirinha, senha, validade, a.paciente_id, a.solicitante_nome,
            usuario, a.unidade_id, correlacao
        )
        .fetch_optional(&mut *conexao)
        .await?;
        if criada.is_none() {
            return Err(ErroApi::conflito(
                "guia-ja-registrada",
                &format!("Já existe guia {numero} desta operadora registrada à mão. A resposta não foi aplicada. Confira a guia na recepção."),
            ));
        }
        for p in r.procedimentos.iter().filter(|p| p.autorizado > 0) {
            sqlx::query!(
                "INSERT INTO convenio.guia_procedimento (guia_id, tuss, autorizado, saldo) VALUES ($1, $2, $3, $3)",
                id,
                p.tuss,
                i32::from(p.autorizado.min(99))
            )
            .execute(&mut *conexao)
            .await?;
        }
        guia = Some((id, codigo.como_str().to_owned()));
    }
    let mut motivo = motivo_de(&r);
    let situacao = r.status.codigo();
    sqlx::query!(
        "UPDATE convenio.autorizacao SET situacao = $2, motivo = $3, guia_id = $4, respondida_em = now(),
                protocolo = coalesce(protocolo, $5), versao = versao + 1
          WHERE id = $1",
        autorizacao,
        situacao,
        motivo,
        guia.as_ref().map(|g| g.0),
        r.numero_guia_operadora
    )
    .execute(&mut *conexao)
    .await?;
    // Autorização total de uma reserva provisória: confirma a reserva com a guia (FC-AGD-09, sem
    // digitação). Se não der (senha vence antes do exame, reserva vencida), a guia fica disponível.
    let mut confirmou = false;
    if let (StatusAutorizacao::Autorizada, Some((guia_id, _))) = (r.status, &guia) {
        let ag = sqlx::query!(
            "SELECT inicio, situacao, expira_em FROM agenda.agendamento WHERE id = $1 FOR UPDATE",
            a.agendamento_id
        )
        .fetch_one(&mut *conexao)
        .await?;
        if ag.situacao == "reserva_provisoria" && ag.expira_em.is_none_or(|e| e > Utc::now()) {
            let tuss: Vec<String> = sqlx::query_scalar!(
                "SELECT tuss FROM convenio.autorizacao_procedimento WHERE autorizacao_id = $1",
                autorizacao
            )
            .fetch_all(&mut *conexao)
            .await?;
            sqlx::query("SAVEPOINT confirmar_reserva")
                .execute(&mut *conexao)
                .await?;
            let dia = ag.inicio.with_timezone(&Sao_Paulo).date_naive();
            match crate::rotas::agendamentos::consumir_guia(
                conexao,
                *guia_id,
                a.paciente_id,
                dia,
                &tuss,
                a.agendamento_id,
            )
            .await
            {
                Ok(_) => {
                    sqlx::query!(
                        "UPDATE agenda.agendamento SET situacao = 'confirmado', guia_id = $2, expira_em = NULL, versao = versao + 1
                          WHERE id = $1 AND situacao = 'reserva_provisoria'",
                        a.agendamento_id,
                        guia_id
                    )
                    .execute(&mut *conexao)
                    .await?;
                    sqlx::query("RELEASE SAVEPOINT confirmar_reserva")
                        .execute(&mut *conexao)
                        .await?;
                    confirmou = true;
                }
                Err(e) => {
                    sqlx::query("ROLLBACK TO SAVEPOINT confirmar_reserva")
                        .execute(&mut *conexao)
                        .await?;
                    let nota = format!("Guia preenchida, mas a reserva não foi confirmada sozinha ({}). Confirme na agenda.", e.codigo());
                    motivo = Some(match motivo {
                        Some(m) => format!("{m}. {nota}"),
                        None => nota,
                    });
                    sqlx::query!(
                        "UPDATE convenio.autorizacao SET motivo = $2 WHERE id = $1",
                        autorizacao,
                        motivo
                    )
                    .execute(&mut *conexao)
                    .await?;
                }
            }
        }
    }
    lastro::registrar(conexao, NovoEvento {
        dominio: "convenio", tipo: "autorizacao_respondida", registro_id: autorizacao, registro_codigo: Some(&a.codigo),
        dados: json!({ "situacao": situacao, "sha256": hash, "origem": origem, "guia": guia.as_ref().map(|g| &g.1),
                       "reserva_confirmada": confirmou, "hash_confere": r.hash_confere }),
        criado_por: usuario, origem: "integracao", unidade_id: a.unidade_id, correlacao,
    }).await?;
    Ok(Gravada::Nova(situacao.to_owned()))
}

/// Envio da solicitação pelo outbox, com a chave do pedido (G3). Sem webservice configurado, falha e
/// fica pendente, visível no pedido (R7).
///
/// # Errors
/// Operadora não configurada ou indisponível, ou falha de banco.
pub async fn enviar_pelo_outbox(
    estado: &Estado,
    unidade: Uuid,
    autorizacao: Uuid,
    chave: Uuid,
) -> anyhow::Result<()> {
    let Some(operadora) = estado.operadora.clone() else {
        anyhow::bail!("webservice TISS da operadora não configurado");
    };
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade)).await?;
    let a = sqlx::query!(
        r#"SELECT a.codigo, a.situacao, a.solicitacao AS "solicitacao!", k.registro_ans
             FROM convenio.autorizacao a JOIN faturamento.contrato k ON k.id = a.contrato_id WHERE a.id = $1"#,
        autorizacao
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    if a.situacao != "aguardando_envio" {
        return Ok(());
    }
    let bruta = operadora
        .solicitar_autorizacao(chave, &a.registro_ans, &a.codigo, &a.solicitacao)
        .await?;
    let correlacao = format!("tiss:{chave}");
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade)).await?;
    sqlx::query!(
        "UPDATE convenio.autorizacao SET situacao = 'enviada', enviada_em = now(), versao = versao + 1
          WHERE id = $1 AND situacao = 'aguardando_envio'",
        autorizacao
    )
    .execute(&mut *tx)
    .await?;
    let lida = ler_resposta(&bruta).map_err(|e| e.to_string());
    let g = gravar_resposta(
        &mut tx,
        autorizacao,
        &bruta,
        lida,
        "webservice",
        None,
        &correlacao,
    )
    .await
    .map_err(|e| anyhow::anyhow!("resposta não aplicada: {}", e.codigo()))?;
    if matches!(&g, Gravada::Nova(s) if !s.is_empty() && ABERTAS.contains(&s.as_str())) {
        // Em análise ou pendente: consulta de novo pelo outbox, com espera crescente.
        sqlx::query!(
            "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia, proximo_em)
             VALUES ($1, 'tiss.consultar_autorizacao', $2, $3, $4, $5, $6, now() + interval '2 minutes')
             ON CONFLICT (tipo, referencia) WHERE referencia IS NOT NULL DO NOTHING",
            novo_id(), novo_id(), json!({ "autorizacao_id": autorizacao }), unidade, correlacao, autorizacao
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok(())
}

/// Consulta do pedido em análise pelo outbox. Enquanto a operadora não decide, devolve erro para o
/// outbox tentar de novo com espera crescente (o erro fica visível no pedido).
///
/// # Errors
/// Pedido ainda em análise, operadora indisponível ou falha de banco.
pub async fn consultar_pelo_outbox(
    estado: &Estado,
    unidade: Uuid,
    autorizacao: Uuid,
) -> anyhow::Result<()> {
    let Some(operadora) = estado.operadora.clone() else {
        anyhow::bail!("webservice TISS da operadora não configurado");
    };
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade)).await?;
    let a = sqlx::query!(
        "SELECT a.codigo, a.situacao, k.registro_ans FROM convenio.autorizacao a
           JOIN faturamento.contrato k ON k.id = a.contrato_id WHERE a.id = $1",
        autorizacao
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    if !ABERTAS.contains(&a.situacao.as_str()) {
        return Ok(());
    }
    let bruta = operadora
        .consultar_autorizacao(&a.registro_ans, &a.codigo)
        .await?;
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade)).await?;
    let lida = ler_resposta(&bruta).map_err(|e| e.to_string());
    let g = gravar_resposta(
        &mut tx,
        autorizacao,
        &bruta,
        lida,
        "webservice",
        None,
        &format!("consulta:{autorizacao}"),
    )
    .await
    .map_err(|e| anyhow::anyhow!("resposta não aplicada: {}", e.codigo()))?;
    tx.commit().await?;
    match g {
        Gravada::Nova(s) if !ABERTAS.contains(&s.as_str()) => Ok(()),
        _ => Err(anyhow::anyhow!(
            "a operadora ainda está analisando o pedido"
        )),
    }
}
