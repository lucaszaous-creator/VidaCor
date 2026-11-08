//! Glosas recebidas no demonstrativo e recurso com justificativa dentro do prazo do contrato
//! (docs/21 § 3.9: glosa com código e motivo; recurso com texto, prazo e resultado).

use super::contrato::{Glosa, PacienteDaConta, PedidoRecurso, Recurso};
use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;
use vidacor_comum::novo_id;

/// Glosas (de uma conta, uma glosa ou todas), com o recurso quando houver.
pub(crate) async fn carregar(
    conexao: &mut PgConnection,
    conta: Option<Uuid>,
    glosa: Option<Uuid>,
) -> Result<Vec<Glosa>, sqlx::Error> {
    let linhas = sqlx::query!(
        r#"SELECT g.id, g.codigo, g.conta_id, c.codigo AS conta_codigo, c.paciente_nome, c.paciente_nascimento,
                  c.operadora, g.codigo_glosa, g.motivo, g.valor_centavos, g.prazo_recurso, g.situacao,
                  r.id AS "recurso_id?", r.codigo AS "recurso_codigo?", r.justificativa AS "justificativa?",
                  r.situacao AS "recurso_situacao?", r.criado_em AS "recurso_em?"
             FROM faturamento.glosa g
             JOIN faturamento.conta c ON c.id = g.conta_id
             LEFT JOIN faturamento.recurso r ON r.glosa_id = g.id
            WHERE ($1::uuid IS NULL OR g.conta_id = $1) AND ($2::uuid IS NULL OR g.id = $2)
            ORDER BY g.situacao, g.prazo_recurso, g.codigo
            LIMIT 500"#,
        conta,
        glosa
    )
    .fetch_all(conexao)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| Glosa {
            id: l.id,
            codigo: l.codigo,
            conta_id: l.conta_id,
            conta_codigo: l.conta_codigo,
            paciente: PacienteDaConta {
                nome: l.paciente_nome,
                nascimento: l.paciente_nascimento,
            },
            operadora: l.operadora,
            codigo_glosa: l.codigo_glosa,
            motivo: l.motivo,
            valor_centavos: l.valor_centavos,
            prazo_recurso: l.prazo_recurso,
            situacao: l.situacao,
            recurso: match (
                l.recurso_id,
                l.recurso_codigo,
                l.justificativa,
                l.recurso_situacao,
                l.recurso_em,
            ) {
                (Some(id), Some(codigo), Some(justificativa), Some(situacao), Some(criado_em)) => {
                    Some(Recurso {
                        id,
                        codigo,
                        justificativa,
                        situacao,
                        criado_em,
                    })
                }
                _ => None,
            },
        })
        .collect())
}

/// Glosas recebidas, das abertas com prazo mais curto para as em recurso.
#[utoipa::path(get, operation_id = "listar_glosas", path = "/v1/faturamento/glosas", tag = "faturamento",
    responses((status = 200, body = Vec<Glosa>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<Glosa>>, ErroApi> {
    usuario.exigir(Ato::FaturamentoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let lista = carregar(&mut tx, None, None).await?;
    tx.commit().await?;
    Ok(Json(lista))
}

/// Registra o recurso de uma glosa (só o faturista), dentro do prazo do contrato. Um recurso por
/// glosa; o envio à operadora fica no outbox, visível até sair.
#[utoipa::path(post, operation_id = "recorrer_glosa", path = "/v1/faturamento/glosas/{id}/recurso", tag = "faturamento",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = PedidoRecurso,
    responses((status = 201, body = Glosa), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn recorrer(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoRecurso>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::GlosaRecorrer)?;
    let justificativa = p.justificativa.trim();
    if !(10..=500).contains(&justificativa.chars().count()) {
        return Err(ErroApi::invalido(
            "justificativa-invalida",
            "Escreva a justificativa do recurso (de 10 a 500 caracteres, o limite do TISS). Nada foi gravado.",
        ));
    }
    let rota = format!("POST /v1/faturamento/glosas/{id}/recurso");
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
    let g = sqlx::query!(
        "SELECT codigo, situacao, prazo_recurso FROM faturamento.glosa WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "glosa-nao-encontrada",
            "Esta glosa não existe no faturamento desta unidade. Nada foi alterado.",
        )
    })?;
    if g.situacao != "aberta" {
        return Err(ErroApi::conflito(
            "recurso-ja-registrado",
            "Esta glosa já tem recurso registrado. Nada foi alterado. Acompanhe o recurso na lista de glosas.",
        ));
    }
    if tempo::hoje_local() > g.prazo_recurso {
        return Err(ErroApi::invalido(
            "prazo-de-recurso-vencido",
            &format!(
                "O prazo de recurso desta glosa terminou em {}. Nada foi gravado. Fale com a operadora sobre a reconsideração.",
                g.prazo_recurso.format("%d/%m/%Y")
            ),
        ));
    }
    let recurso = novo_id();
    let codigo = crate::rotas::caixa::proximo(&mut tx, "RGL").await?;
    let chave_envio = novo_id();
    sqlx::query!(
        "INSERT INTO faturamento.recurso (id, codigo, glosa_id, justificativa, chave_envio, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'faturamento', $7, $8)",
        recurso, codigo.como_str(), id, justificativa, chave_envio, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE faturamento.glosa SET situacao = 'em_recurso', versao = versao + 1 WHERE id = $1",
        id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
         VALUES ($1, 'tiss.recurso_glosa', $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING",
        novo_id(), chave_envio, json!({ "recurso_id": recurso }), usuario.unidade_id, correlacao, recurso
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "faturamento",
            tipo: "recurso_registrado",
            registro_id: recurso,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "glosa": id, "glosa_codigo": g.codigo }),
            criado_por: Some(usuario.id),
            origem: "faturamento",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let glosa = carregar(&mut tx, None, Some(id))
        .await?
        .pop()
        .ok_or_else(|| ErroApi::interno("glosa gravada não foi relida"))?;
    let corpo = serde_json::to_value(&glosa).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}
