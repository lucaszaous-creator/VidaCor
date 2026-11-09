//! Regras de autorização da operadora por exame (pede senha, validade, documentos exigidos). Cada
//! mudança publica a versão inteira nova, com motivo; as anteriores ficam (nada é apagado).

use super::contratos::motivo_valido;
use super::{nao_encontrado, RegraDeAutorizacaoDoExame, RegrasDeAutorizacaoDoConvenio};
use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, HashSet};
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Deserialize, ToSchema)]
pub struct RegraDeAutorizacaoPedida {
    pub tuss: String,
    pub exige_senha: bool,
    /// Dias de validade da senha (só quando pede senha), de 1 a 365.
    pub validade_dias: Option<i32>,
    pub documentos: String,
}

#[derive(Deserialize, ToSchema)]
pub struct NovasRegrasDeAutorizacao {
    /// Versão das regras que a tela mostrou (nula quando ainda não há regras).
    pub versao_vista: Option<i32>,
    pub motivo: String,
    pub itens: Vec<RegraDeAutorizacaoPedida>,
}

/// Versão mais nova das regras de autorização do contrato.
pub(crate) async fn vigentes(
    tx: &mut sqlx::PgConnection,
    contrato: Uuid,
) -> Result<Option<RegrasDeAutorizacaoDoConvenio>, ErroApi> {
    let Some(r) = sqlx::query!(
        "SELECT r.id, r.codigo, r.versao_regras, r.motivo, u.nome AS quem, r.criado_em
           FROM faturamento.regra_autorizacao r JOIN identidade.usuario u ON u.id = r.criado_por
          WHERE r.contrato_id = $1 ORDER BY r.versao_regras DESC LIMIT 1",
        contrato
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(None);
    };
    let itens = sqlx::query_as!(
        RegraDeAutorizacaoDoExame,
        "SELECT i.tuss, p.nome, i.exige_senha, i.validade_dias, i.documentos
           FROM faturamento.regra_autorizacao_item i JOIN agenda.procedimento p ON p.tuss = i.tuss
          WHERE i.regra_id = $1 ORDER BY i.exige_senha DESC, p.nome",
        r.id
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(Some(RegrasDeAutorizacaoDoConvenio {
        codigo: r.codigo,
        versao_regras: r.versao_regras,
        motivo: r.motivo,
        publicada_por: r.quem,
        publicada_em: r.criado_em,
        itens,
    }))
}

fn invalida(detalhe: &str) -> ErroApi {
    ErroApi::invalido(
        "regras-invalidas",
        &format!("{detalhe} Nada foi publicado."),
    )
}

/// Publica a versão nova das regras de autorização do convênio (faturamento), com motivo. Informa a
/// versão vista: duas publicações sobre a mesma versão, só a primeira entra (restrição única, G2).
#[utoipa::path(post, operation_id = "publicar_regras_de_autorizacao", path = "/v1/convenios/{id}/regras-de-autorizacao", tag = "convenios",
    params(("id" = Uuid, Path)), request_body = NovasRegrasDeAutorizacao,
    responses((status = 201, body = RegrasDeAutorizacaoDoConvenio), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn publicar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(n): Corpo<NovasRegrasDeAutorizacao>,
) -> Result<(StatusCode, Json<RegrasDeAutorizacaoDoConvenio>), ErroApi> {
    usuario.exigir(Ato::ConveniosAdministrar)?;
    let motivo = motivo_valido(&n.motivo)?;
    if n.itens.is_empty() || n.itens.len() > 200 {
        return Err(invalida("As regras têm de 1 a 200 exames."));
    }
    let mut vistos = HashSet::new();
    let mut documentos = Vec::with_capacity(n.itens.len());
    for i in &n.itens {
        if !vistos.insert(i.tuss.as_str()) {
            return Err(invalida("Cada exame aparece uma vez."));
        }
        match (i.exige_senha, i.validade_dias) {
            (true, Some(d)) if (1..=365).contains(&d) => {}
            (false, None) => {}
            _ => {
                return Err(invalida(&format!(
                    "Exame {}: quando pede senha, diga a validade (1 a 365 dias); quando não pede, sem validade.",
                    i.tuss
                )))
            }
        }
        let d = i.documentos.trim();
        if !(3..=200).contains(&d.chars().count()) {
            return Err(invalida(&format!(
                "Exame {}: diga os documentos exigidos (3 a 200 letras).",
                i.tuss
            )));
        }
        documentos.push(d.to_owned());
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    sqlx::query_scalar!(
        "SELECT id FROM faturamento.contrato WHERE id = $1 AND unidade_id = $2",
        id,
        usuario.unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(nao_encontrado)?;
    let tuss: Vec<String> = n.itens.iter().map(|i| i.tuss.clone()).collect();
    let no_catalogo: BTreeMap<String, bool> = sqlx::query!(
        "SELECT tuss, ativo FROM agenda.procedimento WHERE tuss = ANY($1)",
        &tuss
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|p| (p.tuss, p.ativo))
    .collect();
    if let Some(fora) = tuss.iter().find(|t| no_catalogo.get(*t) != Some(&true)) {
        return Err(ErroApi::invalido(
            "procedimento-fora-do-catalogo",
            &format!("O exame {fora} não está ativo no catálogo da clínica. Nada foi publicado. Confira a lista."),
        ));
    }
    let atual = sqlx::query_scalar!(
        "SELECT max(versao_regras) FROM faturamento.regra_autorizacao WHERE contrato_id = $1",
        id
    )
    .fetch_one(&mut *tx)
    .await?;
    if atual != n.versao_vista {
        return Err(versao_mudou());
    }
    let rid = novo_id();
    let codigo = proximo(&mut tx, "RAU").await?;
    let versao = atual.unwrap_or(0) + 1;
    sqlx::query!(
        "INSERT INTO faturamento.regra_autorizacao (id, codigo, contrato_id, versao_regras, motivo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'convenios', $7, $8)",
        rid, codigo.como_str(), id, versao, motivo, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => versao_mudou(),
        _ => ErroApi::from(e),
    })?;
    let senhas: Vec<bool> = n.itens.iter().map(|i| i.exige_senha).collect();
    let validades: Vec<Option<i32>> = n.itens.iter().map(|i| i.validade_dias).collect();
    sqlx::query!(
        "INSERT INTO faturamento.regra_autorizacao_item (regra_id, tuss, exige_senha, validade_dias, documentos, unidade_id)
         SELECT $1, t, s, v, d, $6 FROM unnest($2::text[], $3::bool[], $4::int[], $5::text[]) AS u(t, s, v, d)",
        rid, &tuss, &senhas, &validades as &[Option<i32>], &documentos, usuario.unidade_id
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "convenios", tipo: "regras_de_autorizacao_publicadas", registro_id: rid, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "contrato_id": id, "versao_regras": versao, "itens": tuss.len(), "motivo": motivo }),
        criado_por: Some(usuario.id), origem: "convenios", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let r = vigentes(&mut tx, id)
        .await?
        .ok_or_else(|| ErroApi::interno("regras sumiram"))?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(r)))
}

fn versao_mudou() -> ErroApi {
    ErroApi::conflito(
        "versao-mudou",
        "Outra pessoa publicou regras deste convênio enquanto você editava. Nada foi publicado. Recarregue e confira antes de repetir.",
    )
}
