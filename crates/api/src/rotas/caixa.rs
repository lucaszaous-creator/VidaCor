//! Caixa do particular (docs/27 FC-CXA, docs/30 § 5). O servidor calcula o total pela tabela
//! vigente e ignora o valor vindo da tela (G1); desconto acima do limite só com autorização da
//! gerência com reautenticação; estorno com separação de funções. A cobrança PIX sai pelo outbox.

use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};

/// Limite de desconto sem autorização, em pontos base (500 = 5%). A confirmar com a direção (docs/18 § 9).
pub const LIMITE_DESCONTO_CAIXA_PB: i32 = 500;

#[derive(Serialize, ToSchema)]
pub struct Caixa {
    pub id: Uuid,
    pub codigo: String,
    pub fundo_centavos: i64,
    pub aberto_em: DateTime<Utc>,
    pub fechado_em: Option<DateTime<Utc>>,
    pub esperado_centavos: Option<i64>,
    pub contado_centavos: Option<i64>,
    pub diferenca_centavos: Option<i64>,
}

async fn proximo(conexao: &mut PgConnection, prefixo: &str) -> Result<CodigoLegivel, ErroApi> {
    let ano = tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero($1, $2)", prefixo, ano)
        .fetch_one(conexao)
        .await?
        .ok_or_else(|| ErroApi::interno("contador sem valor"))?;
    Ok(CodigoLegivel::novo(prefixo, ano, n))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoAbertura {
    pub fundo_centavos: i64,
}

/// Abre o caixa do operador (um aberto por vez).
#[utoipa::path(post, path = "/v1/caixas", tag = "caixa", request_body = PedidoAbertura,
    responses((status = 201, body = Caixa), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn abrir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    Corpo(p): Corpo<PedidoAbertura>,
) -> Result<(StatusCode, Json<Caixa>), ErroApi> {
    usuario.exigir(Ato::CaixaOperar)?;
    if !(0..=1_000_000).contains(&p.fundo_centavos) {
        return Err(ErroApi::invalido(
            "valor-invalido",
            "O fundo de troco deve ser de R$ 0,00 a R$ 10.000,00. O caixa não foi aberto.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let codigo = proximo(&mut tx, "CXA").await?;
    let id = novo_id();
    sqlx::query!(
        "INSERT INTO caixa.caixa (id, codigo, operador_id, fundo_centavos, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $3, 'caixa', $5, $6, $7)",
        id, codigo.como_str(), usuario.id, p.fundo_centavos, estacao, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("caixa-ja-aberto", "Você já tem um caixa aberto. Nada foi alterado. Use o caixa aberto ou feche-o antes."),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "caixa",
            tipo: "caixa_aberto",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "fundo_centavos": p.fundo_centavos, "estacao": estacao }),
            criado_por: Some(usuario.id),
            origem: "caixa",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let c = carregar_caixa(&mut tx, id).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(c)))
}

async fn carregar_caixa(conexao: &mut PgConnection, id: Uuid) -> Result<Caixa, ErroApi> {
    Ok(sqlx::query_as!(
        Caixa,
        "SELECT id, codigo, fundo_centavos, aberto_em, fechado_em, esperado_centavos, contado_centavos, diferenca_centavos FROM caixa.caixa WHERE id = $1",
        id
    )
    .fetch_one(conexao)
    .await?)
}

/// Caixa aberto do usuário, se houver.
#[utoipa::path(get, path = "/v1/caixas/meu", tag = "caixa", responses((status = 200, body = Option<Caixa>)))]
pub async fn meu(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Option<Caixa>>, ErroApi> {
    usuario.exigir(Ato::CaixaOperar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let c = sqlx::query_as!(
        Caixa,
        "SELECT id, codigo, fundo_centavos, aberto_em, fechado_em, esperado_centavos, contado_centavos, diferenca_centavos
           FROM caixa.caixa WHERE operador_id = $1 AND fechado_em IS NULL",
        usuario.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(c))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoFechamento {
    pub contado_centavos: i64,
}

/// Fecha o caixa com o valor contado: grava esperado, contado e diferença, sem esconder (docs/27 T-CXA-05).
#[utoipa::path(post, path = "/v1/caixas/{id}/fechamento", tag = "caixa", params(("id" = Uuid, Path)), request_body = PedidoFechamento,
    responses((status = 200, body = Caixa), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn fechar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoFechamento>,
) -> Result<Json<Caixa>, ErroApi> {
    usuario.exigir(Ato::CaixaOperar)?;
    if p.contado_centavos < 0 {
        return Err(ErroApi::invalido(
            "valor-invalido",
            "O valor contado não pode ser negativo. O caixa não foi fechado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let c = sqlx::query!("SELECT codigo, fundo_centavos FROM caixa.caixa WHERE id = $1 AND operador_id = $2 AND fechado_em IS NULL FOR UPDATE", id, usuario.id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::conflito("caixa-fechado", "Este caixa não está aberto em seu nome. Nada foi alterado."))?;
    // Esperado em dinheiro: fundo + recebimentos em dinheiro pagos e não estornados.
    let dinheiro = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(total_centavos), 0)::bigint AS \"soma!\" FROM caixa.recebimento WHERE caixa_id = $1 AND forma = 'dinheiro' AND situacao = 'pago'",
        id
    )
    .fetch_one(&mut *tx)
    .await?;
    let esperado = c.fundo_centavos + dinheiro;
    let diferenca = p.contado_centavos - esperado;
    sqlx::query!(
        "UPDATE caixa.caixa SET fechado_em = now(), esperado_centavos = $2, contado_centavos = $3, diferenca_centavos = $4, versao = versao + 1 WHERE id = $1",
        id, esperado, p.contado_centavos, diferenca
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "caixa", tipo: "caixa_fechado", registro_id: id, registro_codigo: Some(&c.codigo),
        dados: json!({ "esperado_centavos": esperado, "contado_centavos": p.contado_centavos, "diferenca_centavos": diferenca }),
        criado_por: Some(usuario.id), origem: "caixa", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let caixa = carregar_caixa(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(caixa))
}

#[derive(Deserialize, ToSchema)]
pub struct ItemPedido {
    pub tuss: String,
    /// Ignorado no cálculo (o servidor usa a tabela vigente); zero ou negativo é recusado (FC-CXA-07).
    pub valor_centavos: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct DescontoPedido {
    pub pontos_base: i32,
    pub motivo: String,
    pub autorizacao_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoRecebimento {
    pub paciente_id: Uuid,
    pub atendimento_id: Option<Uuid>,
    pub itens: Vec<ItemPedido>,
    pub desconto: Option<DescontoPedido>,
    /// pix, dinheiro ou cartao.
    pub forma: String,
}

#[derive(Serialize, ToSchema)]
pub struct ItemRecebimento {
    pub tuss: String,
    pub descricao: String,
    pub valor_centavos: i64,
}

#[derive(Serialize, ToSchema)]
pub struct Pix {
    pub txid: String,
    pub copia_e_cola: String,
}

#[derive(Serialize, ToSchema)]
pub struct Recebimento {
    pub id: Uuid,
    pub codigo: String,
    pub itens: Vec<ItemRecebimento>,
    pub subtotal_centavos: i64,
    pub desconto_centavos: i64,
    pub total_centavos: i64,
    pub tabela_versao: i32,
    pub forma: String,
    /// aguardando_pagamento, pago ou estornado.
    pub situacao: String,
    /// Fica nulo até o PSP devolver a cobrança (pelo outbox).
    pub pix: Option<Pix>,
}

async fn carregar_recebimento(
    conexao: &mut PgConnection,
    id: Uuid,
) -> Result<Option<Recebimento>, ErroApi> {
    let Some(r) = sqlx::query!(
        "SELECT id, codigo, subtotal_centavos, desconto_centavos, total_centavos, tabela_versao, forma, situacao, pix_txid, pix_copia_e_cola
           FROM caixa.recebimento WHERE id = $1",
        id
    )
    .fetch_optional(&mut *conexao)
    .await?
    else {
        return Ok(None);
    };
    let itens = sqlx::query_as!(ItemRecebimento, "SELECT tuss, descricao, valor_centavos FROM caixa.recebimento_item WHERE recebimento_id = $1 ORDER BY tuss", id)
        .fetch_all(&mut *conexao)
        .await?;
    Ok(Some(Recebimento {
        id: r.id,
        codigo: r.codigo,
        itens,
        subtotal_centavos: r.subtotal_centavos,
        desconto_centavos: r.desconto_centavos,
        total_centavos: r.total_centavos,
        tabela_versao: r.tabela_versao,
        forma: r.forma,
        situacao: r.situacao,
        pix: r
            .pix_txid
            .zip(r.pix_copia_e_cola)
            .map(|(txid, copia_e_cola)| Pix { txid, copia_e_cola }),
    }))
}

/// Registra um recebimento: total pela tabela vigente, desconto dentro do limite ou autorizado.
#[utoipa::path(post, path = "/v1/recebimentos", tag = "caixa", request_body = PedidoRecebimento, params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = Recebimento), (status = 403, body = crate::openapi::ProblemaDoc, description = "desconto-acima-do-limite"),
              (status = 409, body = crate::openapi::ProblemaDoc, description = "caixa-fechado"), (status = 422, body = crate::openapi::ProblemaDoc, description = "valor-invalido")))]
pub async fn receber(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<PedidoRecebimento>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::CaixaOperar)?;
    if p.itens.is_empty() || p.itens.len() > 20 {
        return Err(ErroApi::invalido(
            "itens-invalidos",
            "Inclua de 1 a 20 itens. Nada foi recebido.",
        ));
    }
    if p.itens
        .iter()
        .any(|i| i.valor_centavos.is_some_and(|v| v <= 0))
    {
        return Err(ErroApi::invalido(
            "valor-invalido",
            "Há item com valor zero ou negativo. Nada foi recebido. Confira os itens.",
        ));
    }
    if !["pix", "dinheiro", "cartao"].contains(&p.forma.as_str()) {
        return Err(ErroApi::invalido(
            "forma-invalida",
            "Forma de pagamento desconhecida. Nada foi recebido.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, "POST /v1/recebimentos", chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let caixa = sqlx::query!(
        "SELECT id FROM caixa.caixa WHERE operador_id = $1 AND fechado_em IS NULL",
        usuario.id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::conflito(
            "caixa-fechado",
            "Não há caixa aberto em seu nome. Nada foi recebido. Abra o caixa e tente de novo.",
        )
    })?;
    let paciente = sqlx::query_scalar!(
        "SELECT id FROM identidade.paciente WHERE id = $1",
        p.paciente_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "paciente-nao-encontrado",
            "Paciente não encontrado nesta unidade. Nada foi recebido.",
        )
    })?;
    // Tabela de preços vigente hoje (G1: o preço é do servidor).
    let tabela = sqlx::query!(
        "SELECT id, versao FROM financeiro.tabela_preco WHERE unidade_id = $1 AND vigente_desde <= $2 ORDER BY versao DESC LIMIT 1",
        usuario.unidade_id, tempo::hoje_local()
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::conflito("sem-tabela-de-precos", "Não há tabela de preços vigente nesta unidade. Nada foi recebido. Avise a gerência."))?;
    let mut itens = Vec::new();
    for i in &p.itens {
        let preco = sqlx::query!(
            "SELECT pr.valor_centavos, pc.nome FROM financeiro.preco pr JOIN agenda.procedimento pc ON pc.tuss = pr.tuss WHERE pr.tabela_id = $1 AND pr.tuss = $2",
            tabela.id, i.tuss
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::invalido("procedimento-sem-preco", &format!("O procedimento {} não tem preço na tabela vigente. Nada foi recebido.", i.tuss)))?;
        itens.push(ItemRecebimento {
            tuss: i.tuss.clone(),
            descricao: preco.nome,
            valor_centavos: preco.valor_centavos,
        });
    }
    let subtotal: i64 = itens.iter().map(|i| i.valor_centavos).sum();

    let (desconto_centavos, desconto_pb, motivo, autorizacao) = match &p.desconto {
        None => (0, 0, None, None),
        Some(d) => {
            if !(1..=10_000).contains(&d.pontos_base) || d.motivo.trim().chars().count() < 3 {
                return Err(ErroApi::invalido(
                    "desconto-invalido",
                    "Informe o percentual e o motivo do desconto. Nada foi recebido.",
                ));
            }
            let autorizacao = match d.autorizacao_id {
                Some(a) => {
                    // Autorização da gerência: do mesmo paciente, válida, não usada, cobre o percentual.
                    let ok = sqlx::query!(
                        "SELECT id FROM caixa.autorizacao_desconto WHERE id = $1 AND paciente_id = $2 AND usada_em IS NULL
                            AND valida_ate > now() AND pontos_base >= $3 FOR UPDATE",
                        a, paciente, d.pontos_base
                    )
                    .fetch_optional(&mut *tx)
                    .await?;
                    if ok.is_none() {
                        return Err(ErroApi::novo(StatusCode::FORBIDDEN, "autorizacao-invalida", "Autorização inválida",
                            "A autorização de desconto não vale para este recebimento (outro paciente, percentual menor, vencida ou já usada). Nada foi recebido."));
                    }
                    Some(a)
                }
                None if d.pontos_base > LIMITE_DESCONTO_CAIXA_PB => {
                    return Err(ErroApi::novo(StatusCode::FORBIDDEN, "desconto-acima-do-limite", "Desconto acima do seu limite",
                        "Este desconto passa do seu limite. Nada foi recebido. Peça autorização à gerência e aplique com a autorização.")
                        .com_extra("limite_pontos_base", json!(LIMITE_DESCONTO_CAIXA_PB)));
                }
                None => None,
            };
            // Arredondamento: centavo para baixo no desconto (a favor do paciente nunca passa do percentual).
            let desconto = subtotal * i64::from(d.pontos_base) / 10_000;
            (
                desconto,
                d.pontos_base,
                Some(d.motivo.trim().to_owned()),
                autorizacao,
            )
        }
    };
    let total = subtotal - desconto_centavos;
    if total <= 0 {
        return Err(ErroApi::invalido(
            "valor-invalido",
            "O total ficaria zero ou negativo. Nada foi recebido.",
        ));
    }
    let codigo = proximo(&mut tx, "REC").await?;
    let id = novo_id();
    let situacao = if p.forma == "pix" {
        "aguardando_pagamento"
    } else {
        "pago"
    };
    sqlx::query!(
        "INSERT INTO caixa.recebimento (id, codigo, caixa_id, paciente_id, atendimento_id, subtotal_centavos, desconto_centavos, desconto_pontos_base,
                desconto_motivo, desconto_aplicado_por, autorizacao_id, total_centavos, tabela_versao, forma, situacao, chave_psp, pago_em,
                criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, CASE WHEN $15 = 'pago' THEN now() END,
                 $10, 'caixa', $17, $18, $19)",
        id, codigo.como_str(), caixa.id, paciente, p.atendimento_id, subtotal, desconto_centavos, desconto_pb, motivo,
        usuario.id, autorizacao, total, tabela.versao, p.forma, situacao, novo_id(), estacao, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    for i in &itens {
        sqlx::query!(
            "INSERT INTO caixa.recebimento_item (recebimento_id, tuss, descricao, valor_centavos, criado_por, estacao, unidade_id, correlacao)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            id, i.tuss, i.descricao, i.valor_centavos, usuario.id, estacao, usuario.unidade_id, correlacao
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(a) = autorizacao {
        sqlx::query!("UPDATE caixa.autorizacao_desconto SET usada_em = now(), versao = versao + 1 WHERE id = $1 AND usada_em IS NULL", a)
            .execute(&mut *tx)
            .await?;
    }
    if p.forma == "pix" {
        // Uma cobrança por recebimento (referência única no outbox + chave de idempotência no PSP).
        sqlx::query!(
            "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
             VALUES ($1, 'pix.criar_cobranca', $2, $3, $4, $5, $6)",
            novo_id(), novo_id(), json!({ "recebimento_id": id }), usuario.unidade_id, correlacao, id
        )
        .execute(&mut *tx)
        .await?;
    }
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "caixa", tipo: "recebimento_registrado", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({
            "tabela_versao": tabela.versao, "subtotal_centavos": subtotal, "desconto_pontos_base": desconto_pb,
            "desconto_motivo": motivo, "autorizacao": autorizacao, "total_centavos": total, "forma": p.forma, "estacao": estacao
        }),
        criado_por: Some(usuario.id), origem: "caixa", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let r = carregar_recebimento(&mut tx, id)
        .await?
        .ok_or_else(|| ErroApi::interno("recebimento sumiu"))?;
    let corpo = serde_json::to_value(&r).map_err(ErroApi::interno)?;
    idempotencia::concluir(
        &mut tx,
        usuario.id,
        "POST /v1/recebimentos",
        chave,
        201,
        &corpo,
    )
    .await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Um recebimento (para acompanhar a cobrança PIX).
#[utoipa::path(get, path = "/v1/recebimentos/{id}", tag = "caixa", params(("id" = Uuid, Path)),
    responses((status = 200, body = Recebimento), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn obter_recebimento(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<Recebimento>, ErroApi> {
    if !(usuario.pode(Ato::CaixaOperar) || usuario.pode(Ato::EstornoAprovar)) {
        return Err(ErroApi::proibido());
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = carregar_recebimento(&mut tx, id).await?.ok_or_else(|| {
        ErroApi::nao_encontrado("recebimento-nao-encontrado", "Recebimento não encontrado.")
    })?;
    tx.commit().await?;
    Ok(Json(r))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoAutorizacao {
    pub paciente_id: Uuid,
    pub pontos_base: i32,
    pub motivo: String,
    pub solicitante_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct AutorizacaoDesconto {
    pub id: Uuid,
    pub codigo: String,
    pub valida_ate: DateTime<Utc>,
}

/// Gerência autoriza desconto acima do limite (reautenticação; quem pede não autoriza).
#[utoipa::path(post, path = "/v1/autorizacoes-desconto", tag = "caixa", request_body = PedidoAutorizacao,
    responses((status = 201, body = AutorizacaoDesconto), (status = 401, body = crate::openapi::ProblemaDoc), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn autorizar_desconto(
    State(estado): State<Estado>,
    usuario: Usuario,
    Corpo(p): Corpo<PedidoAutorizacao>,
) -> Result<(StatusCode, Json<AutorizacaoDesconto>), ErroApi> {
    usuario.exigir(Ato::DescontoAutorizar)?;
    usuario.exigir_reautenticacao()?;
    if p.solicitante_id == usuario.id {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "separacao-de-funcoes",
            "Quem pede não autoriza",
            "Você não pode autorizar um desconto pedido por você. Nada foi autorizado.",
        ));
    }
    if !(1..=10_000).contains(&p.pontos_base) || p.motivo.trim().chars().count() < 5 {
        return Err(ErroApi::invalido(
            "autorizacao-invalida",
            "Informe o percentual e o motivo (mínimo 5 caracteres). Nada foi autorizado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let codigo = proximo(&mut tx, "AUD").await?;
    let id = novo_id();
    let valida_ate = Utc::now() + Duration::hours(2);
    sqlx::query!(
        "INSERT INTO caixa.autorizacao_desconto (id, codigo, paciente_id, pontos_base, motivo, solicitante_id, autorizado_por, valida_ate, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $7, 'gerencia', $9, $10)",
        id, codigo.como_str(), p.paciente_id, p.pontos_base, p.motivo.trim(), p.solicitante_id, usuario.id, valida_ate, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "caixa", tipo: "desconto_autorizado", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "pontos_base": p.pontos_base, "motivo": p.motivo.trim(), "solicitante": p.solicitante_id, "autorizado_por": usuario.id, "reautenticado": true }),
        criado_por: Some(usuario.id), origem: "gerencia", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(AutorizacaoDesconto {
            id,
            codigo: codigo.to_string(),
            valida_ate,
        }),
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoEstorno {
    pub motivo: String,
}

/// Pede o estorno de um recebimento (o aprovador será outra pessoa).
#[utoipa::path(post, path = "/v1/recebimentos/{id}/estornos", tag = "caixa", params(("id" = Uuid, Path)), request_body = PedidoEstorno,
    responses((status = 201, body = crate::rotas::respostas::EstornoPedido), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn pedir_estorno(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoEstorno>,
) -> Result<(StatusCode, Json<serde_json::Value>), ErroApi> {
    usuario.exigir(Ato::EstornoPedir)?;
    if p.motivo.trim().chars().count() < 5 {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Informe o motivo do estorno. Nada foi pedido.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = sqlx::query!(
        "SELECT id, codigo, situacao FROM caixa.recebimento WHERE id = $1",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado("recebimento-nao-encontrado", "Recebimento não encontrado.")
    })?;
    if r.situacao != "pago" {
        return Err(ErroApi::conflito(
            "estorno-indisponivel",
            "Só recebimento pago pode ser estornado. Nada foi pedido.",
        ));
    }
    let codigo = proximo(&mut tx, "EST").await?;
    let estorno = novo_id();
    sqlx::query!(
        "INSERT INTO caixa.estorno (id, codigo, recebimento_id, motivo, pedido_por, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $5, 'caixa', $6, $7)",
        estorno, codigo.como_str(), r.id, p.motivo.trim(), usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("estorno-ja-pedido", "Já existe pedido de estorno para este recebimento. Nada foi alterado."),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "caixa",
            tipo: "estorno_pedido",
            registro_id: estorno,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "recebimento": r.codigo, "motivo": p.motivo.trim() }),
            criado_por: Some(usuario.id),
            origem: "caixa",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(crate::rotas::respostas::valor(
            &crate::rotas::respostas::EstornoPedido {
                id: estorno,
                codigo: codigo.to_string(),
                situacao: "pedido".into(),
            },
        )?),
    ))
}

/// Aprova o estorno (financeiro ou gerência, reautenticação; quem pediu não aprova, FC-CXA-09).
#[utoipa::path(post, path = "/v1/estornos/{id}/aprovacao", tag = "caixa", params(("id" = Uuid, Path)),
    responses((status = 200, body = crate::rotas::respostas::EstornoAprovado), (status = 401, body = crate::openapi::ProblemaDoc), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn aprovar_estorno(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ErroApi> {
    usuario.exigir(Ato::EstornoAprovar)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let e = sqlx::query!("SELECT id, codigo, recebimento_id, pedido_por, situacao, unidade_id FROM caixa.estorno WHERE id = $1 FOR UPDATE", id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::nao_encontrado("estorno-nao-encontrado", "Pedido de estorno não encontrado."))?;
    if e.pedido_por == usuario.id {
        return Err(ErroApi::novo(StatusCode::FORBIDDEN, "separacao-de-funcoes", "Quem pede não aprova",
            "Você pediu este estorno e não pode aprová-lo. Nada foi alterado. Outra pessoa do financeiro ou da gerência aprova."));
    }
    usuario.exigir_reautenticacao()?;
    if e.situacao != "pedido" {
        return Err(ErroApi::conflito(
            "estorno-ja-aprovado",
            "Este estorno já foi aprovado. Nada foi alterado.",
        ));
    }
    sqlx::query!("UPDATE caixa.estorno SET situacao = 'aprovado', aprovado_por = $2, aprovado_em = now(), versao = versao + 1 WHERE id = $1", e.id, usuario.id)
        .execute(&mut *tx)
        .await?;
    let r = sqlx::query!("UPDATE caixa.recebimento SET situacao = 'estornado', versao = versao + 1 WHERE id = $1 AND situacao = 'pago' RETURNING forma, pix_txid", e.recebimento_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::conflito("estorno-indisponivel", "O recebimento não está pago. Nada foi estornado."))?;
    if r.forma == "pix" {
        sqlx::query!(
            "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
             VALUES ($1, 'pix.estornar', $2, $3, $4, $5, $6)",
            novo_id(), novo_id(), json!({ "txid": r.pix_txid }), e.unidade_id, correlacao, e.recebimento_id
        )
        .execute(&mut *tx)
        .await?;
    }
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "caixa", tipo: "estorno_aprovado", registro_id: e.id, registro_codigo: Some(&e.codigo),
        dados: json!({ "pedido_por": e.pedido_por, "aprovado_por": usuario.id, "reautenticado": true }), criado_por: Some(usuario.id), origem: "financeiro",
        unidade_id: e.unidade_id, correlacao: &correlacao,
    }).await?;
    tx.commit().await?;
    Ok(Json(crate::rotas::respostas::valor(
        &crate::rotas::respostas::EstornoAprovado {
            id: e.id,
            situacao: "aprovado".into(),
        },
    )?))
}

/// Trabalhador: cria a cobrança no PSP com a chave do recebimento e grava txid e copia-e-cola.
///
/// # Errors
/// PSP fora do ar ou falha de banco (o item volta ao outbox com espera).
pub async fn criar_cobranca(
    estado: &Estado,
    unidade_id: Uuid,
    recebimento_id: Uuid,
) -> anyhow::Result<()> {
    let psp = estado
        .psp
        .clone()
        .ok_or_else(|| anyhow::anyhow!("PSP do PIX não configurado"))?;
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    let r = sqlx::query!(
        "SELECT codigo, total_centavos, chave_psp, pix_txid FROM caixa.recebimento WHERE id = $1",
        recebimento_id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    if r.pix_txid.is_some() {
        return Ok(());
    }
    let cobranca = psp
        .criar_cobranca(r.chave_psp, r.total_centavos, &r.codigo)
        .await?;
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    sqlx::query!(
        "UPDATE caixa.recebimento SET pix_txid = $2, pix_copia_e_cola = $3, versao = versao + 1 WHERE id = $1 AND pix_txid IS NULL",
        recebimento_id, cobranca.txid, cobranca.copia_e_cola
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Trabalhador: aplica o evento do PSP consultando o estado atual (não depende da ordem, FC-PIX-04).
/// A baixa só passa de "aguardando" para "pago" (nunca regride) e pede a NFS-e uma vez.
///
/// # Errors
/// PSP fora do ar ou falha de banco.
pub async fn processar_evento_pix(
    estado: &Estado,
    unidade_id: Uuid,
    txid: &str,
) -> anyhow::Result<()> {
    let psp = estado
        .psp
        .clone()
        .ok_or_else(|| anyhow::anyhow!("PSP do PIX não configurado"))?;
    let atual = psp.consultar(txid).await?;
    if !atual.pago {
        return Ok(());
    }
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    let baixa = sqlx::query!(
        "UPDATE caixa.recebimento SET situacao = 'pago', pago_em = now(), pix_end_to_end = $2, versao = versao + 1
          WHERE pix_txid = $1 AND situacao = 'aguardando_pagamento' RETURNING id, codigo",
        txid, atual.end_to_end
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(r) = baixa {
        sqlx::query!(
            "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
             VALUES ($1, 'nfse.emitir', $2, $3, $4, 'pix', $5) ON CONFLICT (tipo, referencia) WHERE referencia IS NOT NULL DO NOTHING",
            novo_id(), novo_id(), json!({ "recebimento_id": r.id }), unidade_id, r.id
        )
        .execute(&mut *tx)
        .await?;
        lastro::registrar(
            &mut tx,
            NovoEvento {
                dominio: "caixa",
                tipo: "pix_pago",
                registro_id: r.id,
                registro_codigo: Some(&r.codigo),
                dados: json!({ "txid": txid, "end_to_end": atual.end_to_end }),
                criado_por: None,
                origem: "psp",
                unidade_id,
                correlacao: "pix",
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
