//! Preparo do exame (Preparo-Exame.dc.html; DEC-2026-11-09-106): passos numerados por procedimento
//! que o paciente recebe no comprovante do agendamento. É da clínica, como o catálogo. A gerência
//! publica versão nova com motivo; a anterior fica guardada (nada é apagado) e o comprovante já
//! emitido continua com a versão que imprimiu. Duas edições sobre a mesma versão: só a primeira entra.

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
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Serialize, ToSchema)]
pub struct VersaoDoPreparo {
    pub id: Uuid,
    pub codigo: String,
    pub numero: i32,
    pub passos: Vec<String>,
    pub chegar_antes_min: i32,
    pub vigente: bool,
    pub motivo: Option<String>,
    pub criado_em: DateTime<Utc>,
    pub criado_por: String,
}

#[derive(Serialize, ToSchema)]
pub struct PreparoDoExame {
    pub tuss: String,
    pub nome: String,
    pub modalidade: Option<String>,
    /// Versão vigente; ausente quando a clínica ainda não cadastrou orientação para o exame.
    pub vigente: Option<VersaoDoPreparo>,
}

#[derive(Deserialize, ToSchema)]
pub struct NovoPreparo {
    /// Número da versão vigente que a pessoa viu (ausente quando ainda não há nenhuma).
    pub versao_vista: Option<i32>,
    /// De 1 a 12 passos, cada um com 3 a 300 caracteres.
    pub passos: Vec<String>,
    /// Quanto tempo antes do horário o paciente deve chegar (0 a 120 minutos).
    pub chegar_antes_min: i32,
    /// Obrigatório a partir da segunda versão (5 a 300 caracteres).
    pub motivo: Option<String>,
}

/// Passos limpos ou o problema, em texto para a pessoa (puro, testado abaixo).
///
/// # Errors
/// Lista vazia, mais de 12 passos ou passo fora de 3 a 300 caracteres.
pub fn passos_validos(passos: &[String]) -> Result<Vec<String>, &'static str> {
    let limpos: Vec<String> = passos
        .iter()
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty())
        .collect();
    if limpos.is_empty() || limpos.len() > 12 {
        return Err("O preparo tem de 1 a 12 passos. Nada foi publicado.");
    }
    if limpos
        .iter()
        .any(|p| !(3..=300).contains(&p.chars().count()))
    {
        return Err("Cada passo tem de 3 a 300 caracteres. Nada foi publicado.");
    }
    Ok(limpos)
}

async fn versoes_de(
    tx: &mut sqlx::PgConnection,
    tuss: &str,
    so_vigente: bool,
) -> Result<Vec<VersaoDoPreparo>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT p.id, p.codigo, p.numero, p.passos, p.chegar_antes_min, p.vigente, p.motivo, p.criado_em, u.nome AS "criado_por!"
             FROM agenda.preparo p JOIN identidade.usuario u ON u.id = p.criado_por
            WHERE p.tuss = $1 AND (p.vigente OR NOT $2) ORDER BY p.numero DESC"#,
        tuss,
        so_vigente
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| VersaoDoPreparo {
            id: l.id,
            codigo: l.codigo,
            numero: l.numero,
            passos: l.passos,
            chegar_antes_min: l.chegar_antes_min,
            vigente: l.vigente,
            motivo: l.motivo,
            criado_em: l.criado_em,
            criado_por: l.criado_por,
        })
        .collect())
}

/// Preparo vigente de cada exame ativo do catálogo (recepção e gerência).
#[utoipa::path(get, operation_id = "listar_preparos", path = "/v1/preparos", tag = "catalogo",
    responses((status = 200, body = Vec<PreparoDoExame>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<PreparoDoExame>>, ErroApi> {
    usuario.exigir(Ato::CatalogoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let exames = sqlx::query!(
        "SELECT tuss, nome, modalidade FROM agenda.procedimento WHERE ativo ORDER BY nome"
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut r = Vec::with_capacity(exames.len());
    for e in exames {
        let vigente = versoes_de(&mut tx, &e.tuss, true).await?.into_iter().next();
        r.push(PreparoDoExame {
            tuss: e.tuss,
            nome: e.nome,
            modalidade: e.modalidade,
            vigente,
        });
    }
    tx.commit().await?;
    Ok(Json(r))
}

/// Todas as versões do preparo de um exame, da mais nova para a mais antiga.
#[utoipa::path(get, operation_id = "versoes_do_preparo", path = "/v1/preparos/{tuss}", tag = "catalogo",
    params(("tuss" = String, Path)),
    responses((status = 200, body = Vec<VersaoDoPreparo>), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn versoes(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(tuss): Path<String>,
) -> Result<Json<Vec<VersaoDoPreparo>>, ErroApi> {
    usuario.exigir(Ato::CatalogoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    exame_existe(&mut tx, &tuss).await?;
    let v = versoes_de(&mut tx, &tuss, false).await?;
    tx.commit().await?;
    Ok(Json(v))
}

async fn exame_existe(tx: &mut sqlx::PgConnection, tuss: &str) -> Result<(), ErroApi> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM agenda.procedimento WHERE tuss = $1) AS "e!""#,
        tuss
    )
    .fetch_one(&mut *tx)
    .await?;
    if existe {
        Ok(())
    } else {
        Err(ErroApi::nao_encontrado(
            "exame-nao-encontrado",
            "Este exame não está no catálogo.",
        ))
    }
}

fn preparo_mudou() -> ErroApi {
    ErroApi::conflito(
        "preparo-mudou",
        "Outra pessoa publicou uma versão do preparo enquanto você editava. Nada foi publicado. Recarregue, confira a versão nova e repita se ainda for preciso.",
    )
}

/// Publica versão nova do preparo (gerência; `Idempotency-Key`). A versão vista precisa ser a
/// vigente (`409 preparo-mudou`); a partir da segunda versão o motivo é obrigatório.
#[utoipa::path(post, operation_id = "publicar_preparo", path = "/v1/preparos/{tuss}", tag = "catalogo",
    params(("tuss" = String, Path), ("Idempotency-Key" = Uuid, Header)), request_body = NovoPreparo,
    responses((status = 201, body = VersaoDoPreparo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn publicar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(tuss): Path<String>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoPreparo>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::PreparoAdministrar)?;
    let passos = passos_validos(&n.passos).map_err(|m| ErroApi::invalido("preparo-invalido", m))?;
    if !(0..=120).contains(&n.chegar_antes_min) {
        return Err(ErroApi::invalido(
            "preparo-invalido",
            "A antecedência de chegada vai de 0 a 120 minutos. Nada foi publicado.",
        ));
    }
    let motivo = n
        .motivo
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_owned);
    if motivo
        .as_ref()
        .is_some_and(|m| !(5..=300).contains(&m.chars().count()))
        || (n.versao_vista.is_some() && motivo.is_none())
    {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Diga por que o preparo mudou (5 a 300 caracteres). Nada foi publicado.",
        ));
    }
    let rota = format!("POST /v1/preparos/{tuss}");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    exame_existe(&mut tx, &tuss).await?;
    // A vigente é travada; quem chegar depois vê a versão nova e recebe 409 (G2). Sem nenhuma
    // vigente, a constraint única (tuss, numero) decide entre dois primeiros cadastros.
    let atual = sqlx::query_scalar!(
        "SELECT numero FROM agenda.preparo WHERE tuss = $1 AND vigente FOR UPDATE",
        tuss
    )
    .fetch_optional(&mut *tx)
    .await?;
    if atual != n.versao_vista {
        return Err(preparo_mudou());
    }
    if let Some(numero) = atual {
        sqlx::query!(
            "UPDATE agenda.preparo SET vigente = false WHERE tuss = $1 AND numero = $2 AND vigente",
            tuss,
            numero
        )
        .execute(&mut *tx)
        .await?;
    }
    let numero = atual.unwrap_or(0) + 1;
    let id = novo_id();
    let codigo = proximo(&mut tx, "PRP").await?;
    sqlx::query!(
        "INSERT INTO agenda.preparo (id, codigo, tuss, numero, passos, chegar_antes_min, motivo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'catalogo', $9, $10)",
        id, codigo.como_str(), tuss, numero, &passos, n.chegar_antes_min, motivo, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => preparo_mudou(),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "catalogo",
            tipo: "preparo_publicado",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "tuss": tuss, "numero": numero, "passos": passos.len(), "chegar_antes_min": n.chegar_antes_min, "motivo": motivo }),
            criado_por: Some(usuario.id),
            origem: "catalogo",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let v = versoes_de(&mut tx, &tuss, true)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| ErroApi::interno("preparo publicado sem vigente"))?;
    let corpo = serde_json::to_value(v).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn passos_limpos_e_limites() {
        assert_eq!(
            passos_validos(&["  Venha   com roupa leve. ".into(), "   ".into()]),
            Ok(vec!["Venha com roupa leve.".into()])
        );
        assert!(passos_validos(&[]).is_err());
        assert!(passos_validos(&["ab".into()]).is_err());
        assert!(passos_validos(&vec!["Passo válido".to_owned(); 13]).is_err());
        assert!(passos_validos(&["x".repeat(301)]).is_err());
    }
}
