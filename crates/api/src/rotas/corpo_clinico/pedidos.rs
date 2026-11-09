//! Pedido de mudança no cadastro do médico e a conferência pela segunda pessoa (gerência).

use super::{
    invalido, modalidades, motivo_valido, um, validar_registro, DadosDaHabilitacao,
    DadosDoRegistro, DecisaoDoCorpoClinico, MedicoDoCorpoClinico, NovoPedidoDoCorpoClinico,
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
use uuid::Uuid;
use vidacor_comum::novo_id;

/// Pede uma mudança no cadastro do médico (registro ou habilitação). Só vale depois da conferência
/// de outra pessoa (gerência; reautenticação; `Idempotency-Key`).
#[utoipa::path(post, operation_id = "pedir_mudanca_no_corpo_clinico", path = "/v1/corpo-clinico/{id}/pedidos", tag = "corpo-clinico",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = NovoPedidoDoCorpoClinico,
    responses((status = 201, body = MedicoDoCorpoClinico), (status = 401, body = crate::openapi::ProblemaDoc), (status = 403, body = crate::openapi::ProblemaDoc),
        (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn pedir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoPedidoDoCorpoClinico>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::CorpoClinicoAdministrar)?;
    if id == usuario.id {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "separacao-de-funcoes",
            "Próprio cadastro",
            "Ninguém pede mudança no próprio cadastro profissional. Peça a outra pessoa da gerência. Nada foi alterado.",
        ));
    }
    usuario.exigir_reautenticacao()?;
    let nada = "Nada foi pedido.";
    let motivo = motivo_valido(&n.motivo, nada)?;
    let dados = match (n.tipo.as_str(), &n.registro, &n.habilitacao) {
        ("registro", Some(r), None) => {
            serde_json::to_value(validar_registro(r)?).map_err(ErroApi::interno)?
        }
        ("habilitacao", None, Some(h)) => serde_json::to_value(DadosDaHabilitacao {
            realiza: modalidades(&h.realiza)?,
            lauda: modalidades(&h.lauda)?,
        })
        .map_err(ErroApi::interno)?,
        _ => {
            return Err(invalido(
                "Peça a mudança do registro (com os dados do CRM) ou do que realiza e lauda (com as modalidades). Nada foi pedido.",
            ))
        }
    };
    const ROTA: &str = "POST /v1/corpo-clinico/{id}/pedidos";
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
    let atual = um(&mut tx, usuario.unidade_id, id).await?;
    let igual = match n.tipo.as_str() {
        "registro" => atual.registro.as_ref().is_some_and(|r| {
            serde_json::to_value(DadosDoRegistro {
                uf: r.uf.clone(),
                numero: r.numero.clone(),
                rqe: r.rqe.clone(),
                especialidade: r.especialidade.clone(),
            })
            .ok()
                == Some(dados.clone())
        }),
        _ => atual.habilitacao.as_ref().is_some_and(|h| {
            serde_json::to_value(DadosDaHabilitacao {
                realiza: h.realiza.clone(),
                lauda: h.lauda.clone(),
            })
            .ok()
                == Some(dados.clone())
        }),
    };
    if igual {
        return Err(ErroApi::invalido(
            "nada-a-mudar",
            "O pedido é igual ao cadastro vigente. Nada foi pedido.",
        ));
    }
    if n.tipo == "registro" {
        let em_uso = sqlx::query_scalar!(
            "SELECT u.nome FROM documentos.registro_profissional r JOIN identidade.usuario u ON u.id = r.usuario_id
              WHERE r.unidade_id = $1 AND r.conselho = 'CRM' AND r.uf = $2 AND r.numero = $3 AND r.vigente AND r.usuario_id <> $4",
            usuario.unidade_id, dados["uf"].as_str(), dados["numero"].as_str(), id
        )
        .fetch_optional(&mut *tx)
        .await?;
        if em_uso.is_some() {
            return Err(crm_em_uso());
        }
    }
    let pedido = novo_id();
    let codigo = proximo(&mut tx, "PCC").await?;
    sqlx::query!(
        "INSERT INTO corpo.pedido (id, codigo, medico_id, tipo, dados, motivo, pedido_por, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7, 'corpo_clinico', $8, $9)",
        pedido, codigo.como_str(), id, n.tipo, dados, motivo, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito(
            "pedido-aguardando",
            "Já há um pedido deste tipo aguardando conferência para este médico. Decida esse antes. Nada foi pedido.",
        ),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "corpo_clinico",
            tipo: "mudanca_pedida",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "pedido_id": pedido, "tipo": n.tipo, "dados": dados, "motivo": motivo, "por": usuario.id, "reautenticado": true }),
            criado_por: Some(usuario.id),
            origem: "corpo_clinico",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(um(&mut tx, usuario.unidade_id, id).await?)
        .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

fn crm_em_uso() -> ErroApi {
    ErroApi::conflito(
        "crm-em-uso",
        "Este CRM já está no cadastro de outro médico desta unidade. Nada foi alterado. Confira o número com o documento do conselho.",
    )
}

/// A segunda pessoa confere o pedido (gerência; quem pediu e o próprio médico não decidem;
/// reautenticação). Aprovado, a versão nova entra vigente e a anterior deixa de ser.
#[utoipa::path(post, operation_id = "decidir_pedido_do_corpo_clinico", path = "/v1/corpo-clinico/pedidos/{id}/decisao", tag = "corpo-clinico",
    params(("id" = Uuid, Path)), request_body = DecisaoDoCorpoClinico,
    responses((status = 200, body = MedicoDoCorpoClinico), (status = 401, body = crate::openapi::ProblemaDoc), (status = 403, body = crate::openapi::ProblemaDoc),
        (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn decidir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(pedido): Path<Uuid>,
    Corpo(d): Corpo<DecisaoDoCorpoClinico>,
) -> Result<Json<MedicoDoCorpoClinico>, ErroApi> {
    usuario.exigir(Ato::PerfilConferir)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let p = sqlx::query!(
        "SELECT codigo, medico_id, tipo, dados, motivo, pedido_por, situacao FROM corpo.pedido WHERE id = $1 AND unidade_id = $2 FOR UPDATE",
        pedido, usuario.unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("pedido-nao-encontrado", "Pedido não encontrado nesta unidade. Nada foi alterado."))?;
    if p.pedido_por == usuario.id || p.medico_id == usuario.id {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "separacao-de-funcoes",
            "Segunda pessoa",
            "A conferência é de outra pessoa: quem pediu, e o próprio médico, não decidem. Nada foi alterado.",
        ));
    }
    usuario.exigir_reautenticacao()?;
    if p.situacao != "aguardando" {
        return Err(ErroApi::conflito(
            "pedido-ja-decidido",
            "Este pedido já foi decidido. Nada foi alterado.",
        ));
    }
    let motivo = motivo_valido(&d.motivo, "Nada foi alterado.")?;
    // Registro de CRM só é aprovado com consulta regular no CFM dos últimos 30 dias (DEC-2026-11-09-120).
    if d.aprovar && p.tipo == "registro" {
        let r: DadosDoRegistro =
            serde_json::from_value(p.dados.clone()).map_err(ErroApi::interno)?;
        super::conferencias::exigir_consulta_regular(
            &mut tx,
            usuario.unidade_id,
            p.medico_id,
            &r.uf,
            &r.numero,
        )
        .await?;
    }
    let feito = sqlx::query!(
        "UPDATE corpo.pedido SET situacao = $2, decidido_por = $3, decidido_em = now(), motivo_decisao = $4, versao = versao + 1
          WHERE id = $1 AND situacao = 'aguardando'",
        pedido, if d.aprovar { "aplicado" } else { "recusado" }, usuario.id, motivo
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if feito != 1 {
        return Err(ErroApi::conflito(
            "pedido-ja-decidido",
            "Este pedido já foi decidido. Nada foi alterado.",
        ));
    }
    if d.aprovar {
        aplicar(
            &mut tx,
            &usuario,
            pedido,
            &p.tipo,
            p.medico_id,
            &p.dados,
            &p.motivo,
            &correlacao,
        )
        .await?;
    }
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "corpo_clinico",
            tipo: if d.aprovar { "mudanca_conferida" } else { "mudanca_recusada" },
            registro_id: p.medico_id,
            registro_codigo: Some(&p.codigo),
            dados: json!({ "pedido_id": pedido, "tipo": p.tipo, "pedido_por": p.pedido_por, "decidido_por": usuario.id, "motivo": motivo, "reautenticado": true }),
            criado_por: Some(usuario.id),
            origem: "corpo_clinico",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let r = um(&mut tx, usuario.unidade_id, p.medico_id).await?;
    tx.commit().await?;
    Ok(Json(r))
}

/// Versão nova vigente; a anterior só deixa de ser vigente (o gatilho de 0062 não aceita outra mudança).
#[allow(clippy::too_many_arguments)]
async fn aplicar(
    tx: &mut sqlx::PgConnection,
    usuario: &Usuario,
    pedido: Uuid,
    tipo: &str,
    medico: Uuid,
    dados: &serde_json::Value,
    motivo: &str,
    correlacao: &str,
) -> Result<(), ErroApi> {
    if tipo == "registro" {
        let r: DadosDoRegistro = serde_json::from_value(dados.clone()).map_err(ErroApi::interno)?;
        sqlx::query!(
            "UPDATE documentos.registro_profissional SET vigente = false, versao = versao + 1
              WHERE usuario_id = $1 AND conselho = 'CRM' AND vigente AND unidade_id = $2",
            medico,
            usuario.unidade_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO documentos.registro_profissional (id, usuario_id, conselho, uf, numero, rqe, especialidade, motivo, pedido_id, criado_por, origem, unidade_id, correlacao)
             VALUES ($1, $2, 'CRM', $3, $4, $5, $6, $7, $8, $9, 'corpo_clinico', $10, $11)",
            novo_id(), medico, r.uf, r.numero, r.rqe, r.especialidade, motivo, pedido, usuario.id, usuario.unidade_id, correlacao
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(d) if d.is_unique_violation() => crm_em_uso(),
            _ => ErroApi::from(e),
        })?;
    } else {
        let h: DadosDaHabilitacao =
            serde_json::from_value(dados.clone()).map_err(ErroApi::interno)?;
        sqlx::query!(
            "UPDATE corpo.habilitacao SET vigente = false, versao = versao + 1 WHERE usuario_id = $1 AND vigente AND unidade_id = $2",
            medico,
            usuario.unidade_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO corpo.habilitacao (id, usuario_id, realiza, lauda, pedido_id, motivo, criado_por, origem, unidade_id, correlacao)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'corpo_clinico', $8, $9)",
            novo_id(), medico, &h.realiza, &h.lauda, pedido, motivo, usuario.id, usuario.unidade_id, correlacao
        )
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}
