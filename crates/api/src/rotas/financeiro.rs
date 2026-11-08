//! Financeiro essencial (Financeiro.dc.html; docs/25 MVP): títulos a pagar e a receber, aprovação
//! com separação de funções (acima de R$ 10 mil, duas aprovações), baixa única, extrato importado sem
//! duplicar e conciliação exata. Repasse médico em `repasse.rs`.

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
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::novo_id;

/// Título a pagar acima deste valor exige duas aprovações (mockup Financeiro).
pub const DUAS_APROVACOES_ACIMA_CENTAVOS: i64 = 1_000_000;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Titulo {
    pub id: Uuid,
    pub codigo: String,
    pub tipo: String,
    pub descricao: String,
    pub contraparte: String,
    pub categoria: String,
    pub valor_centavos: i64,
    pub vencimento: NaiveDate,
    pub situacao: String,
    pub aprovacoes_exigidas: i32,
    pub aprovacoes: i64,
    pub vencido: bool,
}

async fn carregar(
    tx: &mut sqlx::PgConnection,
    id: Option<Uuid>,
    tipo: Option<&str>,
    situacao: Option<&str>,
) -> Result<Vec<Titulo>, ErroApi> {
    let hoje = tempo::hoje_local();
    let linhas = sqlx::query!(
        r#"SELECT t.id, t.codigo, t.tipo, t.descricao, t.contraparte, t.categoria, t.valor_centavos, t.vencimento, t.situacao, t.aprovacoes_exigidas,
                  (SELECT count(*) FROM financeiro.aprovacao a WHERE a.titulo_id = t.id) AS "aprovacoes!"
           FROM financeiro.titulo t
           WHERE ($1::uuid IS NULL OR t.id = $1) AND ($2::text IS NULL OR t.tipo = $2) AND ($3::text IS NULL OR t.situacao = $3)
           ORDER BY t.vencimento, t.codigo LIMIT 500"#,
        id,
        tipo,
        situacao
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| Titulo {
            vencido: l.situacao == "aberto" && l.vencimento < hoje,
            id: l.id,
            codigo: l.codigo,
            tipo: l.tipo,
            descricao: l.descricao,
            contraparte: l.contraparte,
            categoria: l.categoria,
            valor_centavos: l.valor_centavos,
            vencimento: l.vencimento,
            situacao: l.situacao,
            aprovacoes_exigidas: l.aprovacoes_exigidas,
            aprovacoes: l.aprovacoes,
        })
        .collect())
}

async fn um(tx: &mut sqlx::PgConnection, id: Uuid) -> Result<Titulo, ErroApi> {
    carregar(tx, Some(id), None, None)
        .await?
        .pop()
        .ok_or_else(|| ErroApi::nao_encontrado("titulo-nao-encontrado", "Título não encontrado."))
}

#[derive(Deserialize, IntoParams)]
pub struct FiltroTitulos {
    /// pagar ou receber.
    pub tipo: Option<String>,
    /// aberto, pago ou cancelado.
    pub situacao: Option<String>,
}

/// Títulos por tipo e situação (financeiro e gerência).
#[utoipa::path(get, path = "/v1/financeiro/titulos", tag = "financeiro", params(FiltroTitulos),
    responses((status = 200, body = Vec<Titulo>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn titulos(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroTitulos>,
) -> Result<Json<Vec<Titulo>>, ErroApi> {
    usuario.exigir(Ato::FinanceiroOperar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = carregar(&mut tx, None, f.tipo.as_deref(), f.situacao.as_deref()).await?;
    tx.commit().await?;
    Ok(Json(r))
}

#[derive(Deserialize, ToSchema)]
pub struct NovoTitulo {
    pub tipo: String,
    pub descricao: String,
    pub contraparte: String,
    pub categoria: String,
    pub valor_centavos: i64,
    pub vencimento: NaiveDate,
}

/// Lança um título. A pagar sempre precisa de aprovação de outra pessoa (duas acima de R$ 10 mil).
#[utoipa::path(post, path = "/v1/financeiro/titulos", tag = "financeiro", request_body = NovoTitulo, params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = Titulo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn lancar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoTitulo>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::FinanceiroOperar)?;
    if !["pagar", "receber"].contains(&n.tipo.as_str())
        || ![
            "fornecedor",
            "folha",
            "repasse",
            "imposto",
            "operadora",
            "particular",
            "outros",
        ]
        .contains(&n.categoria.as_str())
    {
        return Err(ErroApi::invalido(
            "titulo-invalido",
            "Tipo ou categoria desconhecidos. Nada foi lançado.",
        ));
    }
    if !(1..=100_000_000_000).contains(&n.valor_centavos)
        || n.descricao.trim().len() < 3
        || n.contraparte.trim().len() < 2
    {
        return Err(ErroApi::invalido(
            "titulo-invalido",
            "Informe descrição, contraparte e um valor maior que zero. Nada foi lançado.",
        ));
    }
    let exigidas = match (
        n.tipo.as_str(),
        n.valor_centavos > DUAS_APROVACOES_ACIMA_CENTAVOS,
    ) {
        ("pagar", true) => 2,
        ("pagar", false) => 1,
        _ => 0,
    };
    const ROTA: &str = "POST /v1/financeiro/titulos";
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
    let id = novo_id();
    let codigo = proximo(&mut tx, "TIT").await?;
    sqlx::query!(
        "INSERT INTO financeiro.titulo (id, codigo, tipo, descricao, contraparte, categoria, valor_centavos, vencimento, aprovacoes_exigidas, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'financeiro', $11, $12)",
        id, codigo.como_str(), n.tipo, n.descricao.trim(), n.contraparte.trim(), n.categoria, n.valor_centavos, n.vencimento, exigidas, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "financeiro", tipo: "titulo_lancado", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "tipo": n.tipo, "valor_centavos": n.valor_centavos, "aprovacoes_exigidas": exigidas }),
        criado_por: Some(usuario.id), origem: "financeiro", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let corpo = serde_json::to_value(um(&mut tx, id).await?).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Aprova um título a pagar (reautenticação; quem lançou não aprova; cada pessoa aprova uma vez).
#[utoipa::path(post, path = "/v1/financeiro/titulos/{id}/aprovacoes", tag = "financeiro", params(("id" = Uuid, Path)),
    responses((status = 200, body = Titulo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn aprovar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<Titulo>, ErroApi> {
    usuario.exigir(Ato::FinanceiroAprovar)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let t = sqlx::query!(
        "SELECT codigo, tipo, situacao, criado_por FROM financeiro.titulo WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("titulo-nao-encontrado", "Título não encontrado."))?;
    if t.criado_por == Some(usuario.id) {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "separacao-de-funcoes",
            "Quem lança não aprova",
            "Você lançou este título e não pode aprová-lo. Nada foi aprovado.",
        ));
    }
    usuario.exigir_reautenticacao()?;
    if t.tipo != "pagar" || t.situacao != "aberto" {
        return Err(ErroApi::conflito(
            "aprovacao-indisponivel",
            "Só título a pagar em aberto recebe aprovação. Nada foi aprovado.",
        ));
    }
    sqlx::query!("INSERT INTO financeiro.aprovacao (titulo_id, aprovado_por, unidade_id, correlacao) VALUES ($1, $2, $3, $4)", id, usuario.id, usuario.unidade_id, correlacao)
        .execute(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("ja-aprovado-por-voce", "Você já aprovou este título. A segunda aprovação é de outra pessoa."),
            _ => ErroApi::from(e),
        })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "financeiro",
            tipo: "titulo_aprovado",
            registro_id: id,
            registro_codigo: Some(&t.codigo),
            dados: json!({ "por": usuario.id, "reautenticado": true }),
            criado_por: Some(usuario.id),
            origem: "financeiro",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let r = um(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(r))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoBaixa {
    pub conta_id: Uuid,
    pub data: NaiveDate,
    /// Linha do extrato que comprova (conciliação na mesma operação). Valor e sinal precisam bater.
    pub extrato_linha_id: Option<Uuid>,
}

/// Baixa (pagamento ou recebimento) do título: uma só (G2); a pagar só com as aprovações completas.
#[utoipa::path(post, path = "/v1/financeiro/titulos/{id}/baixa", tag = "financeiro", params(("id" = Uuid, Path)), request_body = PedidoBaixa,
    responses((status = 200, body = Titulo), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn baixar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(b): Corpo<PedidoBaixa>,
) -> Result<Json<Titulo>, ErroApi> {
    usuario.exigir(Ato::FinanceiroOperar)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let atual = um(&mut tx, id).await?;
    if atual.aprovacoes < i64::from(atual.aprovacoes_exigidas) {
        return Err(ErroApi::conflito(
            "aprovacao-pendente",
            &format!(
                "Faltam aprovações: {} de {}. Nada foi pago.",
                atual.aprovacoes, atual.aprovacoes_exigidas
            ),
        ));
    }
    if let Some(linha) = b.extrato_linha_id {
        let l = sqlx::query!(
            "SELECT valor_centavos, conta_id FROM financeiro.extrato_linha WHERE id = $1",
            linha
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            ErroApi::nao_encontrado("extrato-nao-encontrado", "Linha do extrato não encontrada.")
        })?;
        let esperado = if atual.tipo == "pagar" {
            -atual.valor_centavos
        } else {
            atual.valor_centavos
        };
        if l.valor_centavos != esperado || l.conta_id != b.conta_id {
            return Err(ErroApi::invalido("extrato-nao-confere", "A linha do extrato não tem o mesmo valor, sinal ou conta do título. Nada foi conciliado."));
        }
    }
    let feito = sqlx::query!("UPDATE financeiro.titulo SET situacao = 'pago', versao = versao + 1 WHERE id = $1 AND situacao = 'aberto'", id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if feito != 1 {
        return Err(ErroApi::conflito(
            "titulo-fechado",
            "Este título já foi baixado ou cancelado. Nada foi alterado.",
        ));
    }
    sqlx::query!(
        "INSERT INTO financeiro.baixa (id, titulo_id, conta_id, data, extrato_linha_id, criado_por, origem, unidade_id, correlacao) VALUES ($1, $2, $3, $4, $5, $6, 'financeiro', $7, $8)",
        novo_id(), id, b.conta_id, b.data, b.extrato_linha_id, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("extrato-ja-conciliado", "Esta linha do extrato já comprova outro título. Nada foi alterado."),
        sqlx::Error::Database(d) if d.is_foreign_key_violation() => ErroApi::nao_encontrado("conta-nao-encontrada", "Conta bancária não encontrada. Nada foi alterado."),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "financeiro",
            tipo: "titulo_baixado",
            registro_id: id,
            registro_codigo: Some(&atual.codigo),
            dados: json!({ "conta": b.conta_id, "extrato": b.extrato_linha_id }),
            criado_por: Some(usuario.id),
            origem: "financeiro",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let r = um(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(r))
}

#[derive(Deserialize, ToSchema)]
pub struct LinhaDoExtrato {
    pub id_banco: String,
    pub data: NaiveDate,
    /// Positivo entra, negativo sai.
    pub valor_centavos: i64,
    pub descricao: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ImportacaoDeExtrato {
    pub conta_id: Uuid,
    pub linhas: Vec<LinhaDoExtrato>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ExtratoImportado {
    pub novas: i64,
    pub repetidas: i64,
}

/// Importa linhas do extrato (o mesmo ID do banco não entra duas vezes).
#[utoipa::path(post, path = "/v1/financeiro/extratos", tag = "financeiro", request_body = ImportacaoDeExtrato,
    responses((status = 200, body = ExtratoImportado), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn importar_extrato(
    State(estado): State<Estado>,
    usuario: Usuario,
    Corpo(i): Corpo<ImportacaoDeExtrato>,
) -> Result<Json<ExtratoImportado>, ErroApi> {
    usuario.exigir(Ato::FinanceiroOperar)?;
    if i.linhas.is_empty()
        || i.linhas.len() > 5000
        || i.linhas
            .iter()
            .any(|l| l.valor_centavos == 0 || l.id_banco.is_empty() || l.id_banco.len() > 80)
    {
        return Err(ErroApi::invalido("extrato-invalido", "Envie de 1 a 5.000 linhas, cada uma com ID do banco e valor diferente de zero. Nada foi importado."));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let mut novas = 0_i64;
    for l in &i.linhas {
        let r = sqlx::query!(
            "INSERT INTO financeiro.extrato_linha (id, conta_id, id_banco, data, valor_centavos, descricao, criado_por, origem, unidade_id, correlacao)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'extrato', $8, $9) ON CONFLICT (conta_id, id_banco) DO NOTHING",
            novo_id(), i.conta_id, l.id_banco, l.data, l.valor_centavos, l.descricao.chars().take(200).collect::<String>(), usuario.id, usuario.unidade_id, correlacao
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(d) if d.is_foreign_key_violation() => ErroApi::nao_encontrado("conta-nao-encontrada", "Conta bancária não encontrada. Nada foi importado."),
            _ => ErroApi::from(e),
        })?;
        novas += i64::try_from(r.rows_affected()).unwrap_or(0);
    }
    tx.commit().await?;
    Ok(Json(ExtratoImportado {
        novas,
        repetidas: i64::try_from(i.linhas.len()).unwrap_or(0) - novas,
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ResumoFinanceiro {
    pub entradas_do_mes_centavos: i64,
    pub saidas_do_mes_centavos: i64,
    pub saldo_em_contas_centavos: i64,
    pub a_pagar_aberto_centavos: i64,
    pub a_receber_aberto_centavos: i64,
    pub aguardando_aprovacao: i64,
    pub extrato_sem_conciliacao: i64,
}

/// Resumo do mês (extrato e títulos em aberto).
#[utoipa::path(get, path = "/v1/financeiro/resumo", tag = "financeiro", responses((status = 200, body = ResumoFinanceiro)))]
pub async fn resumo(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<ResumoFinanceiro>, ErroApi> {
    usuario.exigir(Ato::FinanceiroOperar)?;
    let hoje = tempo::hoje_local();
    let inicio = hoje.with_day(1).unwrap_or(hoje);
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = sqlx::query!(
        r#"SELECT
             coalesce((SELECT sum(valor_centavos) FROM financeiro.extrato_linha WHERE valor_centavos > 0 AND data >= $1), 0)::bigint AS "entradas!",
             coalesce((SELECT -sum(valor_centavos) FROM financeiro.extrato_linha WHERE valor_centavos < 0 AND data >= $1), 0)::bigint AS "saidas!",
             (coalesce((SELECT sum(saldo_inicial_centavos) FROM financeiro.conta_bancaria), 0) + coalesce((SELECT sum(valor_centavos) FROM financeiro.extrato_linha), 0))::bigint AS "saldo!",
             coalesce((SELECT sum(valor_centavos) FROM financeiro.titulo WHERE tipo = 'pagar' AND situacao = 'aberto'), 0)::bigint AS "pagar!",
             coalesce((SELECT sum(valor_centavos) FROM financeiro.titulo WHERE tipo = 'receber' AND situacao = 'aberto'), 0)::bigint AS "receber!",
             (SELECT count(*) FROM financeiro.titulo t WHERE t.situacao = 'aberto' AND (SELECT count(*) FROM financeiro.aprovacao a WHERE a.titulo_id = t.id) < t.aprovacoes_exigidas) AS "aguardando!",
             (SELECT count(*) FROM financeiro.extrato_linha e WHERE NOT EXISTS (SELECT 1 FROM financeiro.baixa b WHERE b.extrato_linha_id = e.id)) AS "sem_conciliacao!""#,
        inicio
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(ResumoFinanceiro {
        entradas_do_mes_centavos: r.entradas,
        saidas_do_mes_centavos: r.saidas,
        saldo_em_contas_centavos: r.saldo,
        a_pagar_aberto_centavos: r.pagar,
        a_receber_aberto_centavos: r.receber,
        aguardando_aprovacao: r.aguardando,
        extrato_sem_conciliacao: r.sem_conciliacao,
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ContaBancaria {
    pub id: Uuid,
    pub codigo: String,
    pub nome: String,
}

/// Contas bancárias da unidade (para a baixa).
#[utoipa::path(get, path = "/v1/financeiro/contas", tag = "financeiro", responses((status = 200, body = Vec<ContaBancaria>)))]
pub async fn contas(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<ContaBancaria>>, ErroApi> {
    usuario.exigir(Ato::FinanceiroOperar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = sqlx::query_as!(
        ContaBancaria,
        "SELECT id, codigo, nome FROM financeiro.conta_bancaria ORDER BY codigo"
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(r))
}
