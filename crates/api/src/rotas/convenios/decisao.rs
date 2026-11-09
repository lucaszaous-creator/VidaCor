//! Decisão da proposta de tabela do convênio (gerência que não propôs, com reautenticação). Aprovada,
//! nasce a versão nova da tabela com os preços propostos; recusada, nada muda na tabela.

use super::contratos::motivo_valido;
use super::{proposta, PropostaDeTabela};
use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Deserialize, ToSchema)]
pub struct DecisaoDaPropostaDeTabela {
    pub aprovar: bool,
    /// Obrigatório para recusar.
    pub motivo: Option<String>,
}

/// Decide a proposta (gerência que não propôs; reautenticação). Aprovada, nasce a versão nova da
/// tabela com os preços propostos; recusada, nada muda na tabela. Uma decisão só (UPDATE atômico).
#[utoipa::path(post, operation_id = "decidir_proposta_de_tabela", path = "/v1/convenios/propostas/{id}/decisao", tag = "convenios",
    params(("id" = Uuid, Path)), request_body = DecisaoDaPropostaDeTabela,
    responses((status = 200, body = PropostaDeTabela), (status = 401, body = crate::openapi::ProblemaDoc), (status = 403, body = crate::openapi::ProblemaDoc),
        (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn decidir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(d): Corpo<DecisaoDaPropostaDeTabela>,
) -> Result<Json<PropostaDeTabela>, ErroApi> {
    usuario.exigir(Ato::ConveniosAprovar)?;
    let motivo = match d.motivo.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
        Some(m) => Some(motivo_valido(m)?),
        None if !d.aprovar => {
            return Err(ErroApi::invalido(
                "motivo-invalido",
                "Para recusar, escreva o motivo (5 a 300 letras). Nada foi alterado.",
            ))
        }
        None => None,
    };
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let p = sqlx::query!(
        "SELECT p.contrato_id, p.criado_por, p.vigente_desde, p.descricao, p.motivo, p.codigo
           FROM faturamento.tabela_proposta p WHERE p.id = $1 AND p.unidade_id = $2",
        id,
        usuario.unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "proposta-nao-encontrada",
            "Proposta não encontrada nesta unidade. Nada foi alterado.",
        )
    })?;
    if p.criado_por == usuario.id {
        return Err(ErroApi::novo(StatusCode::FORBIDDEN, "separacao-de-funcoes", "Quem propõe não decide",
            "A aprovação da tabela é de outra pessoa: quem propôs não decide. Nada foi alterado. Peça a outra pessoa da gerência."));
    }
    usuario.exigir_reautenticacao()?;
    if d.aprovar {
        let hoje = tempo::hoje_local();
        if p.vigente_desde <= hoje {
            return Err(ErroApi::conflito("proposta-vencida",
                "A data de início da proposta já chegou: aprovar agora mudaria a tabela vigente no meio do caminho. Nada foi alterado. Recuse e peça uma proposta nova com outra data."));
        }
    }
    let situacao = if d.aprovar { "aprovada" } else { "recusada" };
    let decidida = sqlx::query!(
        "UPDATE faturamento.tabela_proposta SET situacao = $2, decidido_por = $3, decidido_em = now(), motivo_decisao = $4, versao = versao + 1
          WHERE id = $1 AND situacao = 'pendente'",
        id, situacao, usuario.id, motivo
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if decidida != 1 {
        return Err(ErroApi::conflito("proposta-ja-decidida",
            "Esta proposta já foi decidida por outra pessoa. Nada foi alterado. Recarregue para ver a decisão."));
    }
    let mut tabela_id = None;
    if d.aprovar {
        let ultima = sqlx::query!(
            "SELECT max(versao) AS versao, max(vigente_desde) AS desde FROM faturamento.tabela WHERE contrato_id = $1",
            p.contrato_id
        )
        .fetch_one(&mut *tx)
        .await?;
        if ultima.desde.is_some_and(|x| p.vigente_desde <= x) {
            return Err(tabela_concorrente());
        }
        let tid = novo_id();
        let codigo = proximo(&mut tx, "TBC").await?;
        sqlx::query!(
            "INSERT INTO faturamento.tabela (id, codigo, contrato_id, versao, vigente_desde, descricao, motivo, proposta_id, criado_por, origem, unidade_id, correlacao)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'convenios', $10, $11)",
            tid, codigo.como_str(), p.contrato_id, ultima.versao.unwrap_or(0) + 1, p.vigente_desde, p.descricao, p.motivo, id,
            usuario.id, usuario.unidade_id, correlacao
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(x) if x.is_unique_violation() => tabela_concorrente(),
            _ => ErroApi::from(e),
        })?;
        sqlx::query!(
            "INSERT INTO faturamento.preco (tabela_id, tuss, descricao, valor_centavos, unidade_id)
             SELECT $1, tuss, descricao, valor_centavos, unidade_id FROM faturamento.tabela_proposta_preco WHERE proposta_id = $2",
            tid, id
        )
        .execute(&mut *tx)
        .await?;
        tabela_id = Some(tid);
    }
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "convenios", tipo: if d.aprovar { "tabela_aprovada" } else { "tabela_recusada" }, registro_id: id,
        registro_codigo: Some(&p.codigo),
        dados: json!({ "contrato_id": p.contrato_id, "tabela_id": tabela_id, "vigente_desde": p.vigente_desde, "motivo": motivo }),
        criado_por: Some(usuario.id), origem: "convenios", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let r = proposta(&mut tx, id)
        .await?
        .ok_or_else(|| ErroApi::interno("proposta sumiu"))?;
    tx.commit().await?;
    Ok(Json(r))
}

fn tabela_concorrente() -> ErroApi {
    ErroApi::conflito(
        "tabela-concorrente",
        "Outra versão da tabela foi criada ao mesmo tempo ou já começa nesse dia. Nada foi alterado. Recarregue e confira.",
    )
}
