//! Autorização pedida pelo sistema (docs/22 E4, Autorizacao.dc.html): o pedido nasce do agendamento
//! ou da reserva provisória com os procedimentos do agendamento (G1: a tela não escolhe), vai à
//! operadora pelo webservice TISS (outbox, chave por pedido) ou é feito no portal da operadora e
//! registrado com o protocolo. A resposta preenche a guia sem digitação.

use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::autorizacoes_contrato::{
    AutorizacaoDetalhe, AutorizacaoResumo, ConsultaPendente, GuiaPreenchida, NovoPedidoAutorizacao,
    PacienteDoPedido, ProcedimentoDoPedido, RespostaRecebida,
};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgConnection;
use utoipa::IntoParams;
use uuid::Uuid;
use vidacor_comum::novo_id;
use vidacor_dominio_faturamento::autorizacao::{
    montar_solicitacao, PedidoAutorizacao, ProcedimentoSolicitado,
};
use vidacor_dominio_faturamento::Profissional;

pub(crate) fn nao_encontrado() -> ErroApi {
    ErroApi::nao_encontrado(
        "autorizacao-nao-encontrada",
        "Este pedido de autorização não existe nesta unidade. Nada foi alterado.",
    )
}

pub(crate) fn repetida(status: u16, corpo: serde_json::Value) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
        Json(corpo),
    )
        .into_response()
}

/// Lê em nome do núcleo (o faturista não lê agenda nem cadastro, docs/18 § 4), depois do ato.
pub(crate) fn como_sistema(u: &Usuario) -> ContextoBanco {
    ContextoBanco {
        usuario_id: Some(u.id),
        unidade_id: u.unidade_id,
        perfis: vec!["sistema".into()],
    }
}

fn digitos(v: &str, n: usize) -> bool {
    v.len() == n && v.chars().all(|c| c.is_ascii_digit())
}

fn validar(p: &NovoPedidoAutorizacao) -> Result<(), ErroApi> {
    let s = &p.solicitante;
    let numero_ok = (1..=15).contains(&s.numero.trim().len())
        && s.numero
            .trim()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "./-".contains(c));
    if !(3..=70).contains(&s.nome.trim().chars().count())
        || !digitos(&s.conselho, 2)
        || !digitos(&s.uf, 2)
        || !digitos(&s.cbos, 6)
        || !numero_ok
    {
        return Err(ErroApi::invalido(
            "solicitante-invalido",
            "Os dados do médico solicitante não estão no padrão TISS: nome, conselho com 2 dígitos (06 = CRM), número, UF com 2 dígitos (33 = RJ) e CBO com 6 dígitos. Nada foi pedido.",
        ));
    }
    let carteira: String = p
        .carteirinha
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if carteira.is_empty()
        || carteira.len() > 30
        || !carteira
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-".contains(c))
    {
        return Err(ErroApi::invalido(
            "carteirinha-invalida",
            "Confira a carteirinha: só letras, números, ponto e hífen. Nada foi pedido.",
        ));
    }
    if !(2..=80).contains(&p.operadora.trim().chars().count()) {
        return Err(ErroApi::invalido(
            "operadora-invalida",
            "Escolha a operadora. Nada foi pedido.",
        ));
    }
    if p.indicacao_clinica
        .as_ref()
        .is_some_and(|i| i.chars().count() > 500)
    {
        return Err(ErroApi::invalido(
            "indicacao-longa",
            "A indicação clínica vai até 500 caracteres (limite do TISS). Nada foi pedido.",
        ));
    }
    Ok(())
}

/// Pede a autorização do agendamento (recepção ou faturista).
#[utoipa::path(post, operation_id = "pedir_autorizacao", path = "/v1/autorizacoes", tag = "autorizacoes",
    params(("Idempotency-Key" = Uuid, Header)), request_body = NovoPedidoAutorizacao,
    responses((status = 201, body = AutorizacaoDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn pedir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<NovoPedidoAutorizacao>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::AutorizacaoPedir)?;
    validar(&p)?;
    let rota = "POST /v1/autorizacoes";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &como_sistema(&usuario)).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let ag = sqlx::query!(
        "SELECT g.id, g.codigo, g.situacao, g.expira_em, g.guia_id, g.exame_tuss, g.unidade_id, p.id AS paciente_id, p.nome, p.nascimento
           FROM agenda.agendamento g JOIN identidade.paciente p ON p.id = g.paciente_id
          WHERE g.id = $1 AND g.unidade_id = $2 FOR UPDATE OF g",
        p.agendamento_id,
        usuario.unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("agendamento-nao-encontrado", "Agendamento não encontrado nesta unidade. Nada foi pedido."))?;
    let reserva_vencida =
        ag.situacao == "reserva_provisoria" && ag.expira_em.is_some_and(|e| e <= Utc::now());
    if !["reserva_provisoria", "agendado", "confirmado"].contains(&ag.situacao.as_str())
        || reserva_vencida
    {
        return Err(ErroApi::conflito("agendamento-encerrado", "Este agendamento não está mais ativo (reserva vencida, cancelado ou atendido). Nada foi pedido."));
    }
    if ag.guia_id.is_some() {
        return Err(ErroApi::conflito(
            "agendamento-com-guia",
            "Este agendamento já tem guia autorizada. Nada foi pedido.",
        ));
    }
    let tuss: Vec<String> = sqlx::query_scalar!(
        "SELECT tuss FROM agenda.agendamento_procedimento WHERE agendamento_id = $1 ORDER BY tuss",
        ag.id
    )
    .fetch_all(&mut *tx)
    .await?;
    let tuss = if tuss.is_empty() {
        vec![ag.exame_tuss.clone()]
    } else {
        tuss
    };
    let mut procedimentos = Vec::new();
    for t in &tuss {
        let descricao =
            sqlx::query_scalar!("SELECT nome FROM agenda.procedimento WHERE tuss = $1", t)
                .fetch_optional(&mut *tx)
                .await?
                .unwrap_or_else(|| format!("Procedimento {t}"));
        procedimentos.push(ProcedimentoSolicitado {
            tuss: t.clone(),
            descricao,
            quantidade: 1,
        });
    }
    let contrato =
        crate::rotas::faturamento::contrato_da_operadora(&mut tx, usuario.unidade_id, &p.operadora)
            .await?;
    let webservice = match &contrato {
        Some(k) => {
            sqlx::query_scalar!(
                "SELECT autorizacao_por_webservice FROM faturamento.contrato WHERE id = $1",
                k.id
            )
            .fetch_one(&mut *tx)
            .await?
        }
        None => false,
    };
    let id = novo_id();
    let codigo = crate::rotas::caixa::proximo(&mut tx, "AUT").await?;
    let carteirinha: String = p
        .carteirinha
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let solicitante = Profissional {
        nome: p.solicitante.nome.trim().to_owned(),
        conselho: Some(p.solicitante.conselho.clone()),
        numero: Some(p.solicitante.numero.trim().to_owned()),
        uf: Some(p.solicitante.uf.clone()),
        cbos: Some(p.solicitante.cbos.clone()),
    };
    let mensagem = match (&contrato, webservice) {
        (Some(k), true) => Some(
            montar_solicitacao(&PedidoAutorizacao {
                numero_guia_prestador: codigo.como_str(),
                sequencial: codigo
                    .como_str()
                    .rsplit('-')
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(1),
                registro_ans: &k.registro_ans,
                codigo_prestador: &k.codigo_prestador,
                nome_prestador: &k.nome_prestador,
                cnes: &k.cnes,
                carteirinha: &carteirinha,
                solicitante: &solicitante,
                data_solicitacao: tempo::hoje_local(),
                indicacao_clinica: p.indicacao_clinica.as_deref(),
                procedimentos: &procedimentos,
                gerado_em: tempo::agora()
                    .with_timezone(&chrono_tz::America::Sao_Paulo)
                    .naive_local(),
            })
            .map_err(|e| {
                ErroApi::invalido("pedido-incompleto", &format!("{e} Nada foi pedido."))
            })?,
        ),
        _ => None,
    };
    let chave_envio = novo_id();
    let situacao = if mensagem.is_some() {
        "aguardando_envio"
    } else {
        "aguardando_portal"
    };
    let inserido = sqlx::query!(
        "INSERT INTO convenio.autorizacao (id, codigo, agendamento_id, paciente_id, paciente_nome, paciente_nascimento, operadora, contrato_id,
                canal, carteirinha, solicitante_nome, solicitante_conselho, solicitante_numero, solicitante_uf, solicitante_cbos,
                indicacao_clinica, situacao, solicitacao, solicitacao_sha256, chave_envio, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, 'balcao', $22, $23, $24)",
        id, codigo.como_str(), ag.id, ag.paciente_id, ag.nome, ag.nascimento, p.operadora.trim(), contrato.as_ref().map(|k| k.id),
        if mensagem.is_some() { "webservice" } else { "portal" }, carteirinha, solicitante.nome, p.solicitante.conselho,
        p.solicitante.numero.trim(), p.solicitante.uf, p.solicitante.cbos, p.indicacao_clinica.as_deref().map(str::trim).filter(|i| !i.is_empty()),
        situacao, mensagem.as_ref().map(|m| m.xml.clone()), mensagem.as_ref().map(|m| m.sha256.clone()), chave_envio, usuario.id, estacao,
        ag.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await;
    if let Err(sqlx::Error::Database(d)) = &inserido {
        if d.is_unique_violation() {
            return Err(ErroApi::conflito(
                "autorizacao-em-andamento",
                "Este agendamento já tem pedido de autorização em andamento. Nada foi pedido de novo. Acompanhe o pedido na lista de autorizações.",
            ));
        }
    }
    inserido?;
    for x in &procedimentos {
        sqlx::query!(
            "INSERT INTO convenio.autorizacao_procedimento (autorizacao_id, tuss, descricao, solicitado, unidade_id) VALUES ($1, $2, $3, $4, $5)",
            id, x.tuss, x.descricao, i32::from(x.quantidade), ag.unidade_id
        )
        .execute(&mut *tx)
        .await?;
    }
    if mensagem.is_some() {
        sqlx::query!(
            "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
             VALUES ($1, 'tiss.solicitar_autorizacao', $2, $3, $4, $5, $6)",
            novo_id(), chave_envio, json!({ "autorizacao_id": id }), ag.unidade_id, correlacao, id
        )
        .execute(&mut *tx)
        .await?;
    }
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "convenio", tipo: "autorizacao_pedida", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "agendamento": ag.codigo, "operadora": p.operadora.trim(), "canal": if mensagem.is_some() { "webservice" } else { "portal" },
                       "procedimentos": tuss, "solicitacao_sha256": mensagem.as_ref().map(|m| &m.sha256) }),
        criado_por: Some(usuario.id), origem: "balcao", unidade_id: ag.unidade_id, correlacao: &correlacao,
    }).await?;
    let d = detalhe(&mut tx, id).await?;
    let corpo = serde_json::to_value(&d).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

pub(crate) async fn resumos(
    conexao: &mut PgConnection,
    situacao: Option<&str>,
    id: Option<Uuid>,
) -> Result<Vec<AutorizacaoResumo>, sqlx::Error> {
    let linhas = sqlx::query!(
        r#"SELECT a.id, a.codigo, a.paciente_nome, a.paciente_nascimento, a.operadora, a.canal, a.situacao, a.protocolo, a.motivo,
                  a.agendamento_id, a.criado_em, g.inicio,
                  (SELECT coalesce(array_agg(p.tuss || ' ' || p.descricao ORDER BY p.tuss), '{}') FROM convenio.autorizacao_procedimento p
                    WHERE p.autorizacao_id = a.id) AS "procedimentos!"
             FROM convenio.autorizacao a
             JOIN LATERAL (SELECT inicio FROM agenda.agendamento WHERE id = a.agendamento_id) g ON true
            WHERE ($1::text IS NULL OR a.situacao = $1) AND ($2::uuid IS NULL OR a.id = $2)
            ORDER BY CASE WHEN a.situacao IN ('negada','parcial','recusada','pendente_documento','aguardando_portal') THEN 0
                          WHEN a.situacao IN ('aguardando_envio','enviada','em_analise') THEN 1 ELSE 2 END, a.criado_em DESC
            LIMIT 300"#,
        situacao,
        id
    )
    .fetch_all(conexao)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| AutorizacaoResumo {
            id: l.id,
            codigo: l.codigo,
            paciente: PacienteDoPedido {
                nome: l.paciente_nome,
                nascimento: l.paciente_nascimento,
            },
            operadora: l.operadora,
            canal: l.canal,
            situacao: l.situacao,
            procedimentos: l.procedimentos,
            protocolo: l.protocolo,
            motivo: l.motivo,
            agendamento_id: l.agendamento_id,
            inicio_do_exame: l.inicio,
            criado_em: l.criado_em,
        })
        .collect())
}

pub(crate) async fn detalhe(
    conexao: &mut PgConnection,
    id: Uuid,
) -> Result<AutorizacaoDetalhe, ErroApi> {
    let resumo = resumos(conexao, None, Some(id))
        .await?
        .pop()
        .ok_or_else(nao_encontrado)?;
    let a = sqlx::query!(
        r#"SELECT a.carteirinha, a.solicitante_nome, a.solicitante_conselho, a.solicitante_numero, a.solicitante_uf, a.indicacao_clinica,
                  a.enviada_em, a.respondida_em, a.guia_id, laudos_ag.situacao AS "situacao_ag!"
             FROM convenio.autorizacao a
             JOIN LATERAL (SELECT situacao FROM agenda.agendamento WHERE id = a.agendamento_id) laudos_ag ON true
            WHERE a.id = $1"#,
        id
    )
    .fetch_one(&mut *conexao)
    .await?;
    let itens = sqlx::query_as!(
        ProcedimentoDoPedido,
        "SELECT tuss, descricao, solicitado, autorizado FROM convenio.autorizacao_procedimento WHERE autorizacao_id = $1 ORDER BY tuss",
        id
    )
    .fetch_all(&mut *conexao)
    .await?;
    let respostas = sqlx::query_as!(
        RespostaRecebida,
        r#"SELECT origem_resposta AS origem, sha256, situacao_lida, erro_leitura, hash_confere, aplicada, recebida_em
             FROM convenio.autorizacao_resposta WHERE autorizacao_id = $1 ORDER BY recebida_em DESC LIMIT 50"#,
        id
    )
    .fetch_all(&mut *conexao)
    .await?;
    let guia = match a.guia_id {
        Some(g) => {
            sqlx::query_as!(
            GuiaPreenchida,
            "SELECT id, codigo, numero, senha, validade, situacao FROM convenio.guia WHERE id = $1",
            g
        )
            .fetch_optional(&mut *conexao)
            .await?
        }
        None => None,
    };
    let pendente = sqlx::query!(
        "SELECT tentativas, proximo_em, ultimo_erro FROM integracao.outbox
          WHERE referencia = $1 AND tipo IN ('tiss.solicitar_autorizacao','tiss.consultar_autorizacao') AND concluido_em IS NULL
          ORDER BY criado_em DESC LIMIT 1",
        id
    )
    .fetch_optional(&mut *conexao)
    .await?
    .map(|o| ConsultaPendente { tentativas: o.tentativas, proxima_em: o.proximo_em, ultimo_erro: o.ultimo_erro });
    Ok(AutorizacaoDetalhe {
        resumo,
        carteirinha: a.carteirinha,
        solicitante: format!(
            "{}, conselho {} {}, UF {}",
            a.solicitante_nome, a.solicitante_conselho, a.solicitante_numero, a.solicitante_uf
        ),
        indicacao_clinica: a.indicacao_clinica,
        enviada_em: a.enviada_em,
        respondida_em: a.respondida_em,
        itens,
        respostas,
        guia,
        pendente_no_outbox: pendente,
        situacao_do_agendamento: a.situacao_ag,
    })
}

#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct FiltroAutorizacoes {
    pub situacao: Option<String>,
}

/// Pedidos de autorização da unidade: negados, parciais e pendentes primeiro.
#[utoipa::path(get, operation_id = "listar_autorizacoes", path = "/v1/autorizacoes", tag = "autorizacoes", params(FiltroAutorizacoes),
    responses((status = 200, body = Vec<AutorizacaoResumo>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroAutorizacoes>,
) -> Result<Json<Vec<AutorizacaoResumo>>, ErroApi> {
    usuario.exigir(Ato::AutorizacaoVer)?;
    let mut tx = banco::transacao(&estado.banco, &como_sistema(&usuario)).await?;
    let l = resumos(&mut tx, f.situacao.as_deref(), None).await?;
    tx.commit().await?;
    Ok(Json(l))
}

/// Pedido com andamento, respostas recebidas (com hash) e a guia preenchida.
#[utoipa::path(get, operation_id = "obter_autorizacao", path = "/v1/autorizacoes/{id}", tag = "autorizacoes", params(("id" = Uuid, Path)),
    responses((status = 200, body = AutorizacaoDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn obter(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<AutorizacaoDetalhe>, ErroApi> {
    usuario.exigir(Ato::AutorizacaoVer)?;
    let mut tx = banco::transacao(&estado.banco, &como_sistema(&usuario)).await?;
    let d = detalhe(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(d))
}

pub(crate) async fn travar(
    conexao: &mut PgConnection,
    id: Uuid,
) -> Result<(String, String, String), ErroApi> {
    let a = sqlx::query!(
        "SELECT codigo, canal, situacao FROM convenio.autorizacao WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *conexao)
    .await?
    .ok_or_else(nao_encontrado)?;
    Ok((a.codigo, a.canal, a.situacao))
}

/// "Consultar operadora agora": antecipa a consulta automática do pedido em análise.
#[utoipa::path(post, operation_id = "consultar_autorizacao_agora", path = "/v1/autorizacoes/{id}/consulta", tag = "autorizacoes",
    params(("id" = Uuid, Path)),
    responses((status = 202, body = AutorizacaoDetalhe), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn consultar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<AutorizacaoDetalhe>), ErroApi> {
    usuario.exigir(Ato::AutorizacaoPedir)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &como_sistema(&usuario)).await?;
    let (_, canal, situacao) = travar(&mut tx, id).await?;
    if canal != "webservice"
        || !["enviada", "em_analise", "pendente_documento"].contains(&situacao.as_str())
    {
        return Err(ErroApi::conflito(
            "consulta-fora-de-hora",
            "Este pedido não está em análise na operadora. Nada foi consultado.",
        ));
    }
    sqlx::query!(
        "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
         VALUES ($1, 'tiss.consultar_autorizacao', $2, $3, $4, $5, $6)
         ON CONFLICT (tipo, referencia) WHERE referencia IS NOT NULL
         DO UPDATE SET proximo_em = now(), tentativas = 0, concluido_em = NULL",
        novo_id(), novo_id(), json!({ "autorizacao_id": id }), usuario.unidade_id, correlacao, id
    )
    .execute(&mut *tx)
    .await?;
    let d = detalhe(&mut tx, id).await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok((StatusCode::ACCEPTED, Json(d)))
}
