//! Versão nova da tabela de preços da unidade (gerência). A vigente nunca é alterada.

use super::{
    motivo_valido, tabelas, tuss_valido, NovaVersaoDaTabela, VersaoDaTabelaDePreco,
    PRECO_MAXIMO_CENTAVOS,
};
use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Duration;
use serde_json::json;
use std::collections::HashSet;
use vidacor_comum::novo_id;

/// Publica versão nova da tabela de preços desta unidade (gerência; `Idempotency-Key`). A vigente
/// não muda: a nova começa numa data futura, depois do início da última versão. Duas versões no
/// mesmo dia, ou publicadas ao mesmo tempo, só uma entra (restrição única no banco, G2).
#[utoipa::path(post, operation_id = "publicar_tabela_de_precos", path = "/v1/catalogo/tabelas-de-preco", tag = "catalogo",
    params(("Idempotency-Key" = Uuid, Header)), request_body = NovaVersaoDaTabela,
    responses((status = 201, body = VersaoDaTabelaDePreco), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn publicar_tabela(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovaVersaoDaTabela>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::CatalogoAdministrar)?;
    let nada = "Nada foi publicado.";
    let motivo = motivo_valido(&n.motivo, nada)?;
    let descricao = n.descricao.trim();
    if !(3..=80).contains(&descricao.chars().count()) {
        return Err(ErroApi::invalido(
            "tabela-invalida",
            "Dê um nome à versão (3 a 80 letras). Nada foi publicado.",
        ));
    }
    let hoje = tempo::hoje_local();
    if n.vigente_desde <= hoje || n.vigente_desde > hoje + Duration::days(366) {
        return Err(ErroApi::invalido(
            "vigencia-invalida",
            "A versão nova começa a partir de amanhã e em até um ano: a tabela vigente não muda no meio do dia. Nada foi publicado.",
        ));
    }
    if n.precos.is_empty() || n.precos.len() > 1000 {
        return Err(ErroApi::invalido(
            "tabela-invalida",
            "A versão tem de 1 a 1000 preços. Nada foi publicado.",
        ));
    }
    let mut vistos = HashSet::new();
    for p in &n.precos {
        if !tuss_valido(&p.tuss) || !vistos.insert(p.tuss.as_str()) {
            return Err(ErroApi::invalido(
                "tabela-invalida",
                "Cada exame aparece uma vez, com código TUSS de 8 algarismos. Nada foi publicado.",
            ));
        }
        if !(1..=PRECO_MAXIMO_CENTAVOS).contains(&p.valor_centavos) {
            return Err(ErroApi::invalido(
                "preco-invalido",
                &format!("Preço do exame {} fora do aceito (de R$ 0,01 a R$ 100.000,00). Nada foi publicado.", p.tuss),
            ));
        }
    }
    const ROTA: &str = "POST /v1/catalogo/tabelas-de-preco";
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
    let tuss: Vec<String> = n.precos.iter().map(|p| p.tuss.clone()).collect();
    let ativos = sqlx::query_scalar!(
        "SELECT count(*) AS \"n!\" FROM agenda.procedimento WHERE tuss = ANY($1) AND ativo",
        &tuss
    )
    .fetch_one(&mut *tx)
    .await?;
    if usize::try_from(ativos).ok() != Some(tuss.len()) {
        return Err(ErroApi::invalido(
            "procedimento-fora-do-catalogo",
            "Há preço para exame fora do catálogo ou desativado. Nada foi publicado. Confira a lista.",
        ));
    }
    let ultima = sqlx::query!(
        "SELECT max(versao) AS versao, max(vigente_desde) AS desde FROM financeiro.tabela_preco WHERE unidade_id = $1",
        usuario.unidade_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if let Some(desde) = ultima.desde.filter(|d| n.vigente_desde <= *d) {
        return Err(ErroApi::invalido(
            "vigencia-invalida",
            &format!(
                "A versão nova precisa começar depois de {} (início da última versão). Nada foi publicado.",
                desde.format("%d/%m/%Y")
            ),
        ));
    }
    let versao = ultima.versao.unwrap_or(0) + 1;
    let id = novo_id();
    sqlx::query!(
        "INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, motivo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'catalogo', $7, $8)",
        id, versao, n.vigente_desde, descricao, motivo, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito(
            "tabela-concorrente",
            "Outra versão da tabela foi publicada ao mesmo tempo ou já começa nesse dia. Nada foi publicado. Recarregue e confira.",
        ),
        _ => ErroApi::from(e),
    })?;
    let valores: Vec<i64> = n.precos.iter().map(|p| p.valor_centavos).collect();
    sqlx::query!(
        "INSERT INTO financeiro.preco (tabela_id, tuss, valor_centavos) SELECT $1, t, v FROM unnest($2::text[], $3::bigint[]) AS x(t, v)",
        id,
        &tuss,
        &valores
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "catalogo",
            tipo: "tabela_de_preco_publicada",
            registro_id: id,
            registro_codigo: None,
            dados: json!({ "versao": versao, "vigente_desde": n.vigente_desde, "itens": tuss.len(), "motivo": motivo }),
            criado_por: Some(usuario.id),
            origem: "catalogo",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let criada = tabelas(&mut tx, usuario.unidade_id)
        .await?
        .0
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| ErroApi::interno("tabela sumiu"))?;
    let corpo = serde_json::to_value(criada).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}
