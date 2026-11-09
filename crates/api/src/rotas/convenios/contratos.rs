//! Cadastro e mudança do contrato do convênio (faturamento), sempre com motivo e histórico imutável.

use super::{detalhe, nao_encontrado, ConvenioDetalhe, TermosDoConvenio};
use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;
use vidacor_dominio_faturamento::auditoria;

#[derive(Deserialize, ToSchema)]
pub struct NovoConvenio {
    /// Nome da operadora como vem escrito na guia (ex.: "Unimed").
    pub operadora: String,
    /// Registro na ANS, 6 algarismos.
    pub registro_ans: String,
    pub termos: TermosDoConvenio,
    pub motivo: String,
}

#[derive(Deserialize, ToSchema)]
pub struct AlteracaoDoConvenio {
    /// Versão do cadastro que a tela mostrou.
    pub versao: i32,
    /// Estado inteiro dos termos depois da mudança.
    pub termos: TermosDoConvenio,
    pub motivo: String,
}

fn invalido(detalhe: &str) -> ErroApi {
    ErroApi::invalido("convenio-invalido", &format!("{detalhe} Nada foi gravado."))
}

fn texto(
    v: &Option<String>,
    min: usize,
    max: usize,
    campo: &str,
) -> Result<Option<String>, ErroApi> {
    match v.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(t) if (min..=max).contains(&t.chars().count()) => Ok(Some(t.to_owned())),
        Some(_) => Err(invalido(&format!("{campo}: de {min} a {max} letras."))),
    }
}

pub(crate) fn motivo_valido(m: &str) -> Result<String, ErroApi> {
    let m = m.trim();
    if (5..=300).contains(&m.chars().count()) {
        Ok(m.to_owned())
    } else {
        Err(ErroApi::invalido(
            "motivo-invalido",
            "Escreva o motivo (5 a 300 letras). Nada foi gravado.",
        ))
    }
}

/// Confere e normaliza os termos (o servidor decide o que é válido, G1).
fn termos_validos(t: &TermosDoConvenio) -> Result<TermosDoConvenio, ErroApi> {
    let codigo_prestador = t.codigo_prestador.trim();
    if codigo_prestador.is_empty()
        || codigo_prestador.len() > 14
        || !codigo_prestador.bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return Err(invalido(
            "Código do prestador na operadora: até 14 letras ou algarismos.",
        ));
    }
    let nome_prestador = t.nome_prestador.trim();
    if !(1..=70).contains(&nome_prestador.chars().count()) {
        return Err(invalido("Nome do prestador: até 70 letras."));
    }
    let cnes = t.cnes.trim();
    if cnes.len() != 7 || !cnes.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalido("CNES: 7 algarismos."));
    }
    if auditoria::conjunto(&t.regra_versao).is_none() {
        return Err(invalido(
            "Escolha um conjunto de regras da auditoria da lista.",
        ));
    }
    for (d, campo) in [
        (Some(t.prazo_envio_dias), "Prazo de envio"),
        (Some(t.prazo_recurso_dias), "Prazo de recurso"),
        (t.prazo_pagamento_dias, "Prazo de pagamento"),
    ] {
        if d.is_some_and(|d| !(1..=365).contains(&d)) {
            return Err(invalido(&format!("{campo}: de 1 a 365 dias.")));
        }
    }
    if let (Some(i), Some(f)) = (t.vigencia_inicio, t.vigencia_fim) {
        if f < i {
            return Err(invalido("O fim da vigência vem depois do início."));
        }
    }
    let telefone = texto(&t.contato_telefone, 8, 20, "Telefone")?;
    if telefone
        .as_deref()
        .is_some_and(|f| !f.chars().all(|c| c.is_ascii_digit() || "() +-".contains(c)))
    {
        return Err(invalido(
            "Telefone: só algarismos, espaço, parênteses, + e -.",
        ));
    }
    let email = texto(&t.contato_email, 6, 120, "E-mail")?;
    if email.as_deref().is_some_and(|e| {
        let partes: Vec<&str> = e.split('@').collect();
        partes.len() != 2
            || partes[0].is_empty()
            || !partes[1].contains('.')
            || e.chars().any(char::is_whitespace)
    }) {
        return Err(invalido("E-mail do contato fora do formato nome@dominio."));
    }
    Ok(TermosDoConvenio {
        codigo_prestador: codigo_prestador.to_owned(),
        nome_prestador: nome_prestador.to_owned(),
        cnes: cnes.to_owned(),
        regra_versao: t.regra_versao.clone(),
        prazo_envio_dias: t.prazo_envio_dias,
        prazo_recurso_dias: t.prazo_recurso_dias,
        prazo_pagamento_dias: t.prazo_pagamento_dias,
        vigencia_inicio: t.vigencia_inicio,
        vigencia_fim: t.vigencia_fim,
        reajuste: texto(&t.reajuste, 3, 120, "Reajuste")?,
        contato_nome: texto(&t.contato_nome, 2, 80, "Nome do contato")?,
        contato_telefone: telefone,
        contato_email: email,
        autorizacao_por_webservice: t.autorizacao_por_webservice,
        ativo: t.ativo,
    })
}

#[allow(clippy::too_many_arguments)]
async fn registrar_mudanca(
    tx: &mut sqlx::PgConnection,
    usuario: &Usuario,
    contrato: Uuid,
    tipo: &str,
    motivo: &str,
    versao: i32,
    antes: Option<&TermosDoConvenio>,
    depois: &TermosDoConvenio,
    correlacao: &str,
) -> Result<(), ErroApi> {
    let codigo = proximo(&mut *tx, "MCV").await?;
    let id = novo_id();
    let antes = antes
        .map(serde_json::to_value)
        .transpose()
        .map_err(ErroApi::interno)?;
    let depois = serde_json::to_value(depois).map_err(ErroApi::interno)?;
    sqlx::query!(
        "INSERT INTO faturamento.contrato_mudanca (id, codigo, contrato_id, tipo, motivo, versao_do_contrato, antes, depois, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'convenios', $10, $11)",
        id, codigo.como_str(), contrato, tipo, motivo, versao, antes, depois, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => versao_mudou(),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut *tx,
        NovoEvento {
            dominio: "convenios",
            tipo: &format!("contrato_{tipo}"),
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "contrato_id": contrato, "motivo": motivo, "versao": versao, "antes": antes, "depois": depois }),
            criado_por: Some(usuario.id),
            origem: "convenios",
            unidade_id: usuario.unidade_id,
            correlacao,
        },
    )
    .await?;
    Ok(())
}

fn versao_mudou() -> ErroApi {
    ErroApi::conflito(
        "versao-mudou",
        "Outra pessoa mudou este convênio enquanto você editava. Nada foi alterado. Recarregue e confira antes de repetir.",
    )
}

/// Cadastra o convênio (faturamento; `Idempotency-Key`). Uma operadora por unidade (índice único).
#[utoipa::path(post, operation_id = "cadastrar_convenio", path = "/v1/convenios", tag = "convenios",
    params(("Idempotency-Key" = Uuid, Header)), request_body = NovoConvenio,
    responses((status = 201, body = ConvenioDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn cadastrar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoConvenio>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::ConveniosAdministrar)?;
    let motivo = motivo_valido(&n.motivo)?;
    let operadora = n.operadora.split_whitespace().collect::<Vec<_>>().join(" ");
    if !(2..=80).contains(&operadora.chars().count()) {
        return Err(invalido("Nome da operadora: de 2 a 80 letras."));
    }
    let ans = n.registro_ans.trim().replace(['.', '-'], "");
    if ans.len() != 6 || !ans.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalido("Registro na ANS: 6 algarismos."));
    }
    let mut termos = termos_validos(&n.termos)?;
    termos.ativo = true;
    const ROTA: &str = "POST /v1/convenios";
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
    let codigo = proximo(&mut tx, "CVN").await?;
    sqlx::query!(
        "INSERT INTO faturamento.contrato (id, codigo, operadora, registro_ans, codigo_prestador, nome_prestador, cnes, regra_versao,
                prazo_envio_dias, prazo_recurso_dias, prazo_pagamento_dias, vigencia_inicio, vigencia_fim, reajuste, contato_nome,
                contato_telefone, contato_email, autorizacao_por_webservice, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, 'convenios', $20, $21)",
        id, codigo.como_str(), operadora, ans, termos.codigo_prestador, termos.nome_prestador, termos.cnes, termos.regra_versao,
        termos.prazo_envio_dias, termos.prazo_recurso_dias, termos.prazo_pagamento_dias, termos.vigencia_inicio, termos.vigencia_fim,
        termos.reajuste, termos.contato_nome, termos.contato_telefone, termos.contato_email, termos.autorizacao_por_webservice,
        usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito(
            "operadora-ja-cadastrada",
            "Esta operadora já tem convênio nesta unidade. Nada foi gravado. Abra o convênio na lista e, se preciso, reative.",
        ),
        _ => ErroApi::from(e),
    })?;
    registrar_mudanca(
        &mut tx,
        &usuario,
        id,
        "cadastrar",
        &motivo,
        1,
        None,
        &termos,
        &correlacao,
    )
    .await?;
    let corpo = serde_json::to_value(detalhe(&mut tx, usuario.unidade_id, id).await?)
        .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Muda os termos do convênio, desativa ou reativa (faturamento), com motivo. Informa a versão vista:
/// duas mudanças sobre a mesma versão, só a primeira entra (G2).
#[utoipa::path(post, operation_id = "alterar_convenio", path = "/v1/convenios/{id}/alteracoes", tag = "convenios",
    params(("id" = Uuid, Path)), request_body = AlteracaoDoConvenio,
    responses((status = 200, body = ConvenioDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn alterar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(a): Corpo<AlteracaoDoConvenio>,
) -> Result<Json<ConvenioDetalhe>, ErroApi> {
    usuario.exigir(Ato::ConveniosAdministrar)?;
    let motivo = motivo_valido(&a.motivo)?;
    let novos = termos_validos(&a.termos)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let atual = detalhe(&mut tx, usuario.unidade_id, id).await?;
    if atual.versao != a.versao {
        return Err(versao_mudou());
    }
    if atual.termos == novos {
        return Err(ErroApi::invalido(
            "nada-a-mudar",
            "Os termos são iguais aos do cadastro. Nada foi gravado.",
        ));
    }
    let tipo = match (atual.termos.ativo, novos.ativo) {
        (true, false) => "desativar",
        (false, true) => "reativar",
        _ => "alterar",
    };
    let nova_versao = atual.versao + 1;
    registrar_mudanca(
        &mut tx,
        &usuario,
        id,
        tipo,
        &motivo,
        nova_versao,
        Some(&atual.termos),
        &novos,
        &correlacao,
    )
    .await?;
    let mudou = sqlx::query!(
        "UPDATE faturamento.contrato SET codigo_prestador = $3, nome_prestador = $4, cnes = $5, regra_versao = $6,
                prazo_envio_dias = $7, prazo_recurso_dias = $8, prazo_pagamento_dias = $9, vigencia_inicio = $10, vigencia_fim = $11,
                reajuste = $12, contato_nome = $13, contato_telefone = $14, contato_email = $15, autorizacao_por_webservice = $16,
                ativo = $17, versao = versao + 1
          WHERE id = $1 AND versao = $2 AND unidade_id = $18",
        id, a.versao, novos.codigo_prestador, novos.nome_prestador, novos.cnes, novos.regra_versao, novos.prazo_envio_dias,
        novos.prazo_recurso_dias, novos.prazo_pagamento_dias, novos.vigencia_inicio, novos.vigencia_fim, novos.reajuste,
        novos.contato_nome, novos.contato_telefone, novos.contato_email, novos.autorizacao_por_webservice, novos.ativo,
        usuario.unidade_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if mudou != 1 {
        return Err(versao_mudou());
    }
    let r = detalhe(&mut tx, usuario.unidade_id, id)
        .await
        .map_err(|_| nao_encontrado())?;
    tx.commit().await?;
    Ok(Json(r))
}
