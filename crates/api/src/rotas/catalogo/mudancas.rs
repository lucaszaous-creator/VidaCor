//! Inclusão e mudança de procedimento no catálogo, com motivo e histórico imutável (gerência).

use super::{
    duracao_valida, modalidade_valida, motivo_valido, tuss_valido, um, MudancaNoProcedimento,
    NovoProcedimento, ProcedimentoDoCatalogo,
};
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
use serde_json::json;
use vidacor_comum::novo_id;

#[allow(clippy::too_many_arguments)]
async fn registrar_mudanca(
    tx: &mut sqlx::PgConnection,
    usuario: &Usuario,
    tuss: &str,
    tipo: &str,
    motivo: &str,
    versao: i32,
    antes: Option<serde_json::Value>,
    depois: serde_json::Value,
    correlacao: &str,
) -> Result<(), ErroApi> {
    let codigo = proximo(&mut *tx, "PRM").await?;
    sqlx::query!(
        "INSERT INTO agenda.procedimento_mudanca (id, codigo, tuss, tipo, motivo, versao_do_procedimento, antes, depois, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'catalogo', $10, $11)",
        novo_id(), codigo.como_str(), tuss, tipo, motivo, versao, antes, depois, usuario.id, usuario.unidade_id, correlacao
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
            dominio: "catalogo",
            tipo: &format!("procedimento_{tipo}"),
            registro_id: novo_id(),
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "tuss": tuss, "motivo": motivo, "versao": versao, "antes": antes, "depois": depois }),
            criado_por: Some(usuario.id),
            origem: "catalogo",
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
        "Outra pessoa mudou este procedimento enquanto você editava. Nada foi alterado. Recarregue e confira antes de repetir.",
    )
}

fn estado_de(p: &ProcedimentoDoCatalogo) -> serde_json::Value {
    json!({ "nome": p.nome, "modalidade": p.modalidade, "duracao_min": p.duracao_min, "requer_contraste": p.requer_contraste, "ativo": p.ativo })
}

/// Inclui procedimento no catálogo (gerência; `Idempotency-Key`). O código TUSS é único.
#[utoipa::path(post, operation_id = "incluir_procedimento", path = "/v1/catalogo/procedimentos", tag = "catalogo",
    params(("Idempotency-Key" = Uuid, Header)), request_body = NovoProcedimento,
    responses((status = 201, body = ProcedimentoDoCatalogo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn incluir(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoProcedimento>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::CatalogoAdministrar)?;
    let motivo = motivo_valido(&n.motivo, "Nada foi incluído.")?;
    let tuss = n.tuss.trim();
    let nome = n.nome.trim();
    let modalidade = modalidade_valida(&n.modalidade);
    let Some(modalidade) = modalidade.filter(|_| {
        tuss_valido(tuss)
            && (3..=120).contains(&nome.chars().count())
            && duracao_valida(n.duracao_min)
    }) else {
        return Err(ErroApi::invalido(
            "procedimento-invalido",
            "Confira o código TUSS (8 algarismos), o nome (3 a 120 letras), a modalidade (2 a 4 letras, ex.: US, CT) e a duração (5 a 240 minutos, de 5 em 5). Nada foi incluído.",
        ));
    };
    const ROTA: &str = "POST /v1/catalogo/procedimentos";
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
    sqlx::query!(
        "INSERT INTO agenda.procedimento (tuss, nome, requer_contraste, duracao_min, modalidade, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'catalogo', $7, $8)",
        tuss, nome, n.requer_contraste, n.duracao_min, modalidade, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito(
            "tuss-em-uso",
            "Este código TUSS já está no catálogo. Nada foi incluído. Procure o exame na lista e, se preciso, reative.",
        ),
        _ => ErroApi::from(e),
    })?;
    let p = um(&mut tx, usuario.unidade_id, tuss).await?;
    registrar_mudanca(
        &mut tx,
        &usuario,
        tuss,
        "incluir",
        &motivo,
        1,
        None,
        estado_de(&p),
        &correlacao,
    )
    .await?;
    let corpo = serde_json::to_value(p).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Muda duração, modalidade ou ativo, com motivo (gerência). Atômico pela versão vista: duas mudanças
/// sobre a mesma versão, só a primeira entra (G2). Nada é apagado.
#[utoipa::path(post, operation_id = "mudar_procedimento", path = "/v1/catalogo/procedimentos/{tuss}/mudancas", tag = "catalogo",
    params(("tuss" = String, Path)), request_body = MudancaNoProcedimento,
    responses((status = 200, body = ProcedimentoDoCatalogo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn mudar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(tuss): Path<String>,
    Corpo(m): Corpo<MudancaNoProcedimento>,
) -> Result<Json<ProcedimentoDoCatalogo>, ErroApi> {
    usuario.exigir(Ato::CatalogoAdministrar)?;
    let motivo = motivo_valido(&m.motivo, "Nada foi alterado.")?;
    let modalidade = match m.modalidade.as_deref() {
        None => None,
        Some(x) => Some(modalidade_valida(x).ok_or_else(|| {
            ErroApi::invalido(
                "procedimento-invalido",
                "A modalidade tem 2 a 4 letras (ex.: US, CT). Nada foi alterado.",
            )
        })?),
    };
    if m.duracao_min.is_some_and(|d| !duracao_valida(d)) {
        return Err(ErroApi::invalido(
            "procedimento-invalido",
            "A duração vai de 5 a 240 minutos, de 5 em 5. Nada foi alterado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if !tuss_valido(&tuss) {
        return Err(ErroApi::nao_encontrado(
            "procedimento-nao-encontrado",
            "Procedimento não encontrado no catálogo. Nada foi alterado.",
        ));
    }
    let antes = um(&mut tx, usuario.unidade_id, &tuss).await?;
    let duracao = m.duracao_min.unwrap_or(antes.duracao_min);
    let modalidade = modalidade.or_else(|| antes.modalidade.clone());
    let ativo = m.ativo.unwrap_or(antes.ativo);
    if duracao == antes.duracao_min && modalidade == antes.modalidade && ativo == antes.ativo {
        return Err(ErroApi::invalido(
            "nada-a-mudar",
            "A mudança é igual ao cadastro atual. Nada foi alterado.",
        ));
    }
    let tipo =
        if ativo != antes.ativo && duracao == antes.duracao_min && modalidade == antes.modalidade {
            if ativo {
                "reativar"
            } else {
                "desativar"
            }
        } else {
            "alterar"
        };
    let versao = sqlx::query_scalar!(
        "UPDATE agenda.procedimento SET duracao_min = $3, modalidade = $4, ativo = $5, versao = versao + 1
          WHERE tuss = $1 AND versao = $2 RETURNING versao",
        tuss, m.versao, duracao, modalidade, ativo
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(versao_mudou)?;
    let depois = um(&mut tx, usuario.unidade_id, &tuss).await?;
    registrar_mudanca(
        &mut tx,
        &usuario,
        &tuss,
        tipo,
        &motivo,
        versao,
        Some(estado_de(&antes)),
        estado_de(&depois),
        &correlacao,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(depois))
}
