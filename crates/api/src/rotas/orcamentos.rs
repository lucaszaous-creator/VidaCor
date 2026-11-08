//! Orçamento do particular (Orcamento.dc.html; docs/04 Agenda). O núcleo calcula tudo pela tabela
//! vigente e pela regra de pacote (G1); a tela só pede exames, pacote e desconto. Desconto até o
//! limite da recepção com motivo; acima, o orçamento é recusado e o desconto fica para o caixa, com
//! autorização da gerência (DEC-2026-11-08-74). Emitido não muda: cancela-se com motivo.

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
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::novo_id;
use vidacor_dominio_recepcao::orcamento::{
    calcular, ErroDoCalculo, Pacote, LIMITE_DESCONTO_ORCAMENTO_PB, MOTIVOS_DE_DESCONTO,
    VALIDADE_DIAS,
};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ExameComPreco {
    pub tuss: String,
    pub nome: String,
    pub valor_centavos: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PacoteDoCatalogo {
    pub codigo: String,
    pub nome: String,
    pub regra_versao: i32,
    pub tuss: Vec<String>,
    pub valor_centavos: i64,
}

#[derive(Serialize, ToSchema)]
pub struct CatalogoDoOrcamento {
    pub tabela_versao: i32,
    pub exames: Vec<ExameComPreco>,
    pub pacotes: Vec<PacoteDoCatalogo>,
    /// Limite de desconto da recepção, em pontos base (1000 = 10%).
    pub limite_desconto_pontos_base: i32,
}

#[derive(Deserialize, ToSchema)]
pub struct DescontoDoOrcamento {
    pub pontos_base: i32,
    /// paciente_frequente, funcionario_ou_familiar, condicao_social ou cobrir_outra_clinica.
    pub motivo: String,
}

#[derive(Deserialize, ToSchema)]
pub struct NovoOrcamento {
    pub paciente_id: Uuid,
    /// Códigos TUSS dos exames (o preço é do servidor).
    pub itens: Vec<String>,
    /// Código do pacote a aplicar, se a tela ofereceu.
    pub pacote: Option<String>,
    pub desconto: Option<DescontoDoOrcamento>,
    /// pix, dinheiro ou cartao.
    pub pagamento_previsto: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ItemDoOrcamento {
    pub tuss: String,
    pub descricao: String,
    pub valor_centavos: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Orcamento {
    pub id: Uuid,
    pub codigo: String,
    pub paciente_id: Uuid,
    pub paciente_nome: String,
    pub paciente_nascimento: NaiveDate,
    pub itens: Vec<ItemDoOrcamento>,
    pub tabela_versao: i32,
    pub pacote: Option<String>,
    pub pacote_regra_versao: Option<i32>,
    pub subtotal_centavos: i64,
    pub pacote_centavos: i64,
    pub desconto_centavos: i64,
    pub desconto_pontos_base: Option<i32>,
    pub desconto_motivo: Option<String>,
    pub total_centavos: i64,
    pub pagamento_previsto: String,
    pub valido_ate: NaiveDate,
    /// emitido, vencido ou cancelado (vencido é calculado pela data).
    pub situacao: String,
    pub motivo_cancelamento: Option<String>,
    pub criado_em: DateTime<Utc>,
}

#[derive(Deserialize, IntoParams)]
pub struct FiltroDeOrcamentos {
    pub paciente_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
pub struct CancelamentoDeOrcamento {
    pub motivo: String,
}

fn invalido(e: &ErroDoCalculo) -> ErroApi {
    let (c, d) = match e {
        ErroDoCalculo::SemItens => (
            "sem-exames",
            "Inclua ao menos um exame. Nenhum orçamento foi emitido.",
        ),
        ErroDoCalculo::ItemRepetido => (
            "exame-repetido",
            "Há exame repetido. Nenhum orçamento foi emitido.",
        ),
        ErroDoCalculo::PacoteNaoCabe => (
            "pacote-nao-cabe",
            "O pacote só vale com todos os exames dele no orçamento. Nenhum orçamento foi emitido.",
        ),
        ErroDoCalculo::DescontoInvalido => (
            "desconto-invalido",
            "Desconto deve ser de 0,01% a 100%. Nenhum orçamento foi emitido.",
        ),
    };
    ErroApi::invalido(c, d)
}

async fn tabela_vigente(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
) -> Result<(Uuid, i32), ErroApi> {
    let t = sqlx::query!(
        "SELECT id, versao FROM financeiro.tabela_preco WHERE unidade_id = $1 AND vigente_desde <= $2 ORDER BY versao DESC LIMIT 1",
        unidade, tempo::hoje_local()
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::conflito("sem-tabela-de-precos", "Não há tabela de preços vigente nesta unidade. Nenhum orçamento foi emitido. Avise a gerência."))?;
    Ok((t.id, t.versao))
}

async fn carregar(
    tx: &mut sqlx::PgConnection,
    filtro_id: Option<Uuid>,
    paciente: Option<Uuid>,
) -> Result<Vec<Orcamento>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT o.id, o.codigo, o.paciente_id, p.nome AS paciente_nome, p.nascimento AS paciente_nascimento, o.tabela_versao,
                  k.codigo AS "pacote?", k.regra_versao AS "pacote_regra_versao?", o.subtotal_centavos, o.pacote_centavos, o.desconto_centavos,
                  o.desconto_pontos_base, o.desconto_motivo, o.total_centavos, o.pagamento_previsto, o.valido_ate, o.situacao,
                  o.motivo_cancelamento, o.criado_em
             FROM financeiro.orcamento o
             JOIN identidade.paciente p ON p.id = o.paciente_id
             LEFT JOIN financeiro.pacote k ON k.id = o.pacote_id
            WHERE ($1::uuid IS NULL OR o.id = $1) AND ($2::uuid IS NULL OR o.paciente_id = $2)
              AND ($1::uuid IS NOT NULL OR o.criado_em > now() - interval '30 days')
            ORDER BY o.criado_em DESC LIMIT 100"#,
        filtro_id,
        paciente
    )
    .fetch_all(&mut *tx)
    .await?;
    let ids: Vec<Uuid> = linhas.iter().map(|l| l.id).collect();
    let itens = sqlx::query!(
        "SELECT orcamento_id, tuss, descricao, valor_centavos FROM financeiro.orcamento_item WHERE orcamento_id = ANY($1) ORDER BY descricao",
        &ids
    )
    .fetch_all(&mut *tx)
    .await?;
    let hoje = tempo::hoje_local();
    Ok(linhas
        .into_iter()
        .map(|l| Orcamento {
            itens: itens
                .iter()
                .filter(|i| i.orcamento_id == l.id)
                .map(|i| ItemDoOrcamento {
                    tuss: i.tuss.clone(),
                    descricao: i.descricao.clone(),
                    valor_centavos: i.valor_centavos,
                })
                .collect(),
            situacao: if l.situacao == "emitido" && l.valido_ate < hoje {
                "vencido".to_owned()
            } else {
                l.situacao
            },
            id: l.id,
            codigo: l.codigo,
            paciente_id: l.paciente_id,
            paciente_nome: l.paciente_nome,
            paciente_nascimento: l.paciente_nascimento,
            tabela_versao: l.tabela_versao,
            pacote: l.pacote,
            pacote_regra_versao: l.pacote_regra_versao,
            subtotal_centavos: l.subtotal_centavos,
            pacote_centavos: l.pacote_centavos,
            desconto_centavos: l.desconto_centavos,
            desconto_pontos_base: l.desconto_pontos_base,
            desconto_motivo: l.desconto_motivo,
            total_centavos: l.total_centavos,
            pagamento_previsto: l.pagamento_previsto,
            valido_ate: l.valido_ate,
            motivo_cancelamento: l.motivo_cancelamento,
            criado_em: l.criado_em,
        })
        .collect())
}

/// Exames com o preço da tabela vigente e os pacotes vigentes (recepção e gerência).
#[utoipa::path(get, operation_id = "catalogo_do_orcamento", path = "/v1/orcamentos/catalogo", tag = "orcamento",
    responses((status = 200, body = CatalogoDoOrcamento), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn catalogo(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<CatalogoDoOrcamento>, ErroApi> {
    usuario.exigir(Ato::OrcamentoEmitir)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let (tabela, versao) = tabela_vigente(&mut tx, usuario.unidade_id).await?;
    let exames = sqlx::query_as!(
        ExameComPreco,
        "SELECT pr.tuss, pc.nome, pr.valor_centavos FROM financeiro.preco pr JOIN agenda.procedimento pc ON pc.tuss = pr.tuss
          WHERE pr.tabela_id = $1 AND pc.ativo ORDER BY pc.nome",
        tabela
    )
    .fetch_all(&mut *tx)
    .await?;
    let pacotes = sqlx::query_as!(
        PacoteDoCatalogo,
        "SELECT DISTINCT ON (codigo) codigo, nome, regra_versao, tuss, valor_centavos FROM financeiro.pacote
          WHERE unidade_id = $1 AND vigente_desde <= $2 AND (vigente_ate IS NULL OR vigente_ate >= $2)
          ORDER BY codigo, regra_versao DESC",
        usuario.unidade_id,
        tempo::hoje_local()
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(CatalogoDoOrcamento {
        tabela_versao: versao,
        exames,
        pacotes,
        limite_desconto_pontos_base: LIMITE_DESCONTO_ORCAMENTO_PB,
    }))
}

/// Emite o orçamento calculado pelo servidor (recepção e gerência; Idempotency-Key).
#[utoipa::path(post, operation_id = "emitir_orcamento", path = "/v1/orcamentos", tag = "orcamento", params(("Idempotency-Key" = Uuid, Header)),
    request_body = NovoOrcamento,
    responses((status = 201, body = Orcamento), (status = 403, body = crate::openapi::ProblemaDoc, description = "desconto-acima-do-limite"),
        (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn emitir(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoOrcamento>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::OrcamentoEmitir)?;
    if !["pix", "dinheiro", "cartao"].contains(&n.pagamento_previsto.as_str()) {
        return Err(ErroApi::invalido(
            "forma-invalida",
            "Forma de pagamento desconhecida. Nenhum orçamento foi emitido.",
        ));
    }
    if n.itens.len() > 30 {
        return Err(ErroApi::invalido(
            "exames-demais",
            "Até 30 exames por orçamento. Nenhum orçamento foi emitido.",
        ));
    }
    if let Some(d) = &n.desconto {
        if !MOTIVOS_DE_DESCONTO.contains(&d.motivo.as_str()) {
            return Err(ErroApi::invalido(
                "motivo-invalido",
                "Escolha o motivo do desconto. Nenhum orçamento foi emitido.",
            ));
        }
        if d.pontos_base > LIMITE_DESCONTO_ORCAMENTO_PB {
            return Err(ErroApi::novo(StatusCode::FORBIDDEN, "desconto-acima-do-limite", "Desconto acima do limite",
                "Este desconto passa do limite de 10% do orçamento. Nenhum orçamento foi emitido. Emita com até 10% e peça o desconto maior no caixa, com autorização da gerência.")
                .com_extra("limite_pontos_base", json!(LIMITE_DESCONTO_ORCAMENTO_PB)));
        }
    }
    const ROTA: &str = "POST /v1/orcamentos";
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
    sqlx::query_scalar!(
        "SELECT id FROM identidade.paciente WHERE id = $1",
        n.paciente_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "paciente-nao-encontrado",
            "Paciente não encontrado nesta unidade. Nenhum orçamento foi emitido.",
        )
    })?;
    let (tabela, tabela_versao) = tabela_vigente(&mut tx, usuario.unidade_id).await?;
    let mut itens: Vec<ItemDoOrcamento> = Vec::new();
    for tuss in &n.itens {
        let p = sqlx::query!(
            "SELECT pr.valor_centavos, pc.nome FROM financeiro.preco pr JOIN agenda.procedimento pc ON pc.tuss = pr.tuss WHERE pr.tabela_id = $1 AND pr.tuss = $2 AND pc.ativo",
            tabela, tuss
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::invalido("exame-sem-preco", &format!("O exame {tuss} não tem preço na tabela vigente. Nenhum orçamento foi emitido.")))?;
        itens.push(ItemDoOrcamento {
            tuss: tuss.clone(),
            descricao: p.nome,
            valor_centavos: p.valor_centavos,
        });
    }
    let pacote = match &n.pacote {
        None => None,
        Some(codigo) => Some(
            sqlx::query!(
                "SELECT id, codigo, regra_versao, tuss, valor_centavos FROM financeiro.pacote
                  WHERE unidade_id = $1 AND codigo = $2 AND vigente_desde <= $3 AND (vigente_ate IS NULL OR vigente_ate >= $3)
                  ORDER BY regra_versao DESC LIMIT 1",
                usuario.unidade_id, codigo, tempo::hoje_local()
            )
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| invalido(&ErroDoCalculo::PacoteNaoCabe))?,
        ),
    };
    let regra = pacote.as_ref().map(|p| Pacote {
        codigo: p.codigo.clone(),
        versao: p.regra_versao,
        tuss: p.tuss.clone(),
        valor_centavos: p.valor_centavos,
    });
    let pares: Vec<(String, i64)> = itens
        .iter()
        .map(|i| (i.tuss.clone(), i.valor_centavos))
        .collect();
    let c = calcular(
        &pares,
        regra.as_ref(),
        n.desconto.as_ref().map(|d| d.pontos_base),
    )
    .map_err(|e| invalido(&e))?;
    let id = novo_id();
    let codigo = proximo(&mut tx, "ORC").await?;
    let valido_ate = tempo::hoje_local() + chrono::Duration::days(VALIDADE_DIAS);
    sqlx::query!(
        "INSERT INTO financeiro.orcamento (id, codigo, paciente_id, tabela_versao, pacote_id, subtotal_centavos, pacote_centavos, desconto_centavos,
                desconto_pontos_base, desconto_motivo, total_centavos, pagamento_previsto, valido_ate, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 'recepcao', $15, $16)",
        id, codigo.como_str(), n.paciente_id, tabela_versao, pacote.as_ref().map(|p| p.id), c.subtotal_centavos, c.pacote_centavos,
        c.desconto_centavos, n.desconto.as_ref().map(|d| d.pontos_base), n.desconto.as_ref().map(|d| d.motivo.clone()), c.total_centavos,
        n.pagamento_previsto, valido_ate, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    for i in &itens {
        sqlx::query!(
            "INSERT INTO financeiro.orcamento_item (orcamento_id, tuss, descricao, valor_centavos, unidade_id) VALUES ($1, $2, $3, $4, $5)",
            id, i.tuss, i.descricao, i.valor_centavos, usuario.unidade_id
        )
        .execute(&mut *tx)
        .await?;
    }
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "financeiro", tipo: "orcamento_emitido", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "tabela_versao": tabela_versao, "pacote": regra.as_ref().map(|p| json!({ "codigo": p.codigo, "regra_versao": p.versao })),
                       "total_centavos": c.total_centavos, "desconto_pontos_base": n.desconto.as_ref().map(|d| d.pontos_base) }),
        criado_por: Some(usuario.id), origem: "recepcao", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let corpo = serde_json::to_value(
        carregar(&mut tx, Some(id), None)
            .await?
            .pop()
            .ok_or_else(|| ErroApi::interno("orçamento sumiu"))?,
    )
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Orçamentos dos últimos 30 dias da unidade, ou de um paciente (recepção e gerência).
#[utoipa::path(get, operation_id = "listar_orcamentos", path = "/v1/orcamentos", tag = "orcamento", params(FiltroDeOrcamentos),
    responses((status = 200, body = Vec<Orcamento>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroDeOrcamentos>,
) -> Result<Json<Vec<Orcamento>>, ErroApi> {
    usuario.exigir(Ato::OrcamentoEmitir)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = carregar(&mut tx, None, f.paciente_id).await?;
    tx.commit().await?;
    Ok(Json(r))
}

/// Cancela o orçamento emitido, com motivo (recepção e gerência). Nada é apagado.
#[utoipa::path(post, operation_id = "cancelar_orcamento", path = "/v1/orcamentos/{id}/cancelamento", tag = "orcamento", params(("id" = Uuid, Path)),
    request_body = CancelamentoDeOrcamento,
    responses((status = 200, body = Orcamento), (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn cancelar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(c): Corpo<CancelamentoDeOrcamento>,
) -> Result<Json<Orcamento>, ErroApi> {
    usuario.exigir(Ato::OrcamentoEmitir)?;
    let motivo = c.motivo.trim();
    if motivo.chars().count() < 5 || motivo.chars().count() > 300 {
        return Err(ErroApi::invalido(
            "motivo-invalido",
            "Escreva o motivo do cancelamento (5 a 300 letras). Nada foi alterado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    // Atômico (G2): só cancela o que ainda está emitido.
    let feito = sqlx::query_scalar!(
        "UPDATE financeiro.orcamento SET situacao = 'cancelado', cancelado_por = $2, cancelado_em = now(), motivo_cancelamento = $3, versao = versao + 1
          WHERE id = $1 AND situacao = 'emitido' RETURNING codigo",
        id, usuario.id, motivo
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(codigo) = feito else {
        let existe = sqlx::query_scalar!(
            "SELECT situacao FROM financeiro.orcamento WHERE id = $1",
            id
        )
        .fetch_optional(&mut *tx)
        .await?;
        return Err(match existe {
            None => ErroApi::nao_encontrado(
                "orcamento-nao-encontrado",
                "Orçamento não encontrado. Nada foi alterado.",
            ),
            Some(_) => ErroApi::conflito(
                "orcamento-ja-cancelado",
                "Este orçamento já estava cancelado. Nada foi alterado.",
            ),
        });
    };
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "financeiro",
            tipo: "orcamento_cancelado",
            registro_id: id,
            registro_codigo: Some(&codigo),
            dados: json!({ "motivo": motivo }),
            criado_por: Some(usuario.id),
            origem: "recepcao",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let r = carregar(&mut tx, Some(id), None)
        .await?
        .pop()
        .ok_or_else(|| ErroApi::interno("orçamento sumiu"))?;
    tx.commit().await?;
    Ok(Json(r))
}
