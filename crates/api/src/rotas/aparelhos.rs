//! Aparelhos e integração DICOM (Aparelhos.dc.html; docs/18: a TI administra aparelhos e integrações
//! sem dado clínico). A lista mostra a atividade do dia por aparelho só em números (listas de
//! trabalho enviadas, pendentes e com erro): nenhum nome, exame ou identificador de paciente.
//! Cadastrar, desativar e reativar pedem motivo e ficam em histórico imutável.

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

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AtividadeDoDia {
    pub listas_enviadas: i64,
    pub listas_pendentes: i64,
    pub listas_com_erro: i64,
    pub ultimo_envio: Option<DateTime<Utc>>,
    /// Último erro técnico do envio, sem dado de paciente.
    pub ultimo_erro: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AparelhoCadastrado {
    pub id: Uuid,
    pub codigo: String,
    pub nome: String,
    pub ae_title: String,
    pub modalidade: String,
    pub sala_id: Uuid,
    pub sala_nome: String,
    pub ativo: bool,
    pub versao: i32,
    /// conectado, atencao (pendentes há mais de 5 min ou erro) ou desativado.
    pub situacao: String,
    pub hoje: AtividadeDoDia,
}

#[derive(Serialize, ToSchema)]
pub struct PainelDosAparelhos {
    pub aparelhos: Vec<AparelhoCadastrado>,
    pub estudos_recebidos_hoje: i64,
    pub estudos_sem_paciente: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct NovoAparelho {
    pub nome: String,
    pub ae_title: String,
    pub modalidade: String,
    pub sala_id: Uuid,
    pub motivo: String,
}

#[derive(Deserialize, ToSchema)]
pub struct MudancaDeAparelho {
    /// desativar ou reativar.
    pub tipo: String,
    pub motivo: String,
}

fn motivo_valido(m: &str) -> Result<String, ErroApi> {
    let m = m.trim();
    let n = m.chars().count();
    if (5..=300).contains(&n) {
        Ok(m.to_owned())
    } else {
        Err(ErroApi::invalido(
            "motivo-invalido",
            "Escreva o motivo (5 a 300 letras). Nada foi alterado.",
        ))
    }
}

async fn carregar(
    tx: &mut sqlx::PgConnection,
    id: Option<Uuid>,
) -> Result<Vec<AparelhoCadastrado>, ErroApi> {
    let (inicio, fim) = tempo::limites_do_dia(tempo::hoje_local())
        .ok_or_else(|| ErroApi::interno("dia sem limites"))?;
    let linhas = sqlx::query!(
        r#"SELECT a.id, a.codigo, a.nome, a.ae_title, a.modalidade, a.sala_id, s.nome AS sala_nome, a.ativo, a.versao,
                  count(o.id) FILTER (WHERE o.concluido_em IS NOT NULL) AS "enviadas!",
                  count(o.id) FILTER (WHERE o.concluido_em IS NULL) AS "pendentes!",
                  count(o.id) FILTER (WHERE o.concluido_em IS NULL AND o.ultimo_erro IS NOT NULL) AS "com_erro!",
                  count(o.id) FILTER (WHERE o.concluido_em IS NULL AND o.criado_em < now() - interval '5 minutes') AS "atrasadas!",
                  max(o.concluido_em) AS ultimo_envio,
                  (array_agg(left(o.ultimo_erro, 200) ORDER BY o.criado_em DESC) FILTER (WHERE o.concluido_em IS NULL AND o.ultimo_erro IS NOT NULL))[1] AS ultimo_erro
             FROM agenda.aparelho a
             JOIN agenda.sala s ON s.id = a.sala_id
             LEFT JOIN integracao.outbox o ON o.tipo = 'worklist.criar' AND o.carga->>'aparelho_id' = a.id::text
                  AND o.criado_em >= $2 AND o.criado_em < $3
            WHERE a.unidade_id = comum.ctx_unidade() AND ($1::uuid IS NULL OR a.id = $1)
            GROUP BY a.id, s.nome
            ORDER BY s.nome, a.nome"#,
        id,
        inicio,
        fim
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| AparelhoCadastrado {
            situacao: if !l.ativo {
                "desativado"
            } else if l.com_erro > 0 || l.atrasadas > 0 {
                "atencao"
            } else {
                "conectado"
            }
            .to_owned(),
            hoje: AtividadeDoDia {
                listas_enviadas: l.enviadas,
                listas_pendentes: l.pendentes,
                listas_com_erro: l.com_erro,
                ultimo_envio: l.ultimo_envio,
                ultimo_erro: l.ultimo_erro,
            },
            id: l.id,
            codigo: l.codigo,
            nome: l.nome,
            ae_title: l.ae_title,
            modalidade: l.modalidade,
            sala_id: l.sala_id,
            sala_nome: l.sala_nome,
            ativo: l.ativo,
            versao: l.versao,
        })
        .collect())
}

/// Aparelhos da unidade com a atividade do dia, só em números (TI e gerência).
#[utoipa::path(get, operation_id = "listar_aparelhos", path = "/v1/aparelhos", tag = "aparelhos",
    responses((status = 200, body = PainelDosAparelhos), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<PainelDosAparelhos>, ErroApi> {
    usuario.exigir(Ato::AparelhoAdministrar)?;
    let (inicio, fim) = tempo::limites_do_dia(tempo::hoje_local())
        .ok_or_else(|| ErroApi::interno("dia sem limites"))?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let aparelhos = carregar(&mut tx, None).await?;
    // Contagens agregadas por função SECURITY DEFINER: a TI não lê a tabela de estudos (dado clínico).
    let c = sqlx::query!(
        r#"SELECT recebidos AS "recebidos!", sem_paciente AS "sem_paciente!" FROM exames.contagem_do_dia($1, $2)"#,
        inicio,
        fim
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(PainelDosAparelhos {
        aparelhos,
        estudos_recebidos_hoje: c.recebidos,
        estudos_sem_paciente: c.sem_paciente,
    }))
}

/// Cadastra o aparelho na sala (TI e gerência; Idempotency-Key). AE Title é único.
#[utoipa::path(post, operation_id = "cadastrar_aparelho", path = "/v1/aparelhos", tag = "aparelhos", params(("Idempotency-Key" = Uuid, Header)),
    request_body = NovoAparelho,
    responses((status = 201, body = AparelhoCadastrado), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn cadastrar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoAparelho>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::AparelhoAdministrar)?;
    let motivo = motivo_valido(&n.motivo)?;
    let nome = n.nome.trim();
    let ae = n.ae_title.trim().to_uppercase();
    let modalidade = n.modalidade.trim().to_uppercase();
    if !(2..=80).contains(&nome.chars().count())
        || ae.is_empty()
        || ae.len() > 16
        || !ae
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        || !(2..=4).contains(&modalidade.len())
        || !modalidade.bytes().all(|b| b.is_ascii_uppercase())
    {
        return Err(ErroApi::invalido(
            "aparelho-invalido",
            "Confira o nome (2 a 80 letras), o AE Title (até 16 letras, números ou _) e a modalidade (2 a 4 letras, ex.: US, CT). Nada foi cadastrado.",
        ));
    }
    const ROTA: &str = "POST /v1/aparelhos";
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
        "SELECT id FROM agenda.sala WHERE id = $1 AND unidade_id = $2",
        n.sala_id,
        usuario.unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "sala-nao-encontrada",
            "Sala não encontrada nesta unidade. Nada foi cadastrado.",
        )
    })?;
    let id = novo_id();
    let codigo = proximo(&mut tx, "APA").await?;
    sqlx::query!(
        "INSERT INTO agenda.aparelho (id, codigo, nome, ae_title, modalidade, sala_id, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'administracao', $8, $9)",
        id, codigo.como_str(), nome, ae, modalidade, n.sala_id, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("ae-title-em-uso", "Este AE Title já é de outro aparelho. Nada foi cadastrado. Cada aparelho precisa de um AE Title próprio."),
        _ => ErroApi::from(e),
    })?;
    registrar_mudanca(
        &mut tx,
        &usuario,
        id,
        "cadastrar",
        &motivo,
        1,
        json!({ "nome": nome, "ae_title": ae, "modalidade": modalidade, "sala_id": n.sala_id }),
        &correlacao,
    )
    .await?;
    let corpo = serde_json::to_value(
        carregar(&mut tx, Some(id))
            .await?
            .pop()
            .ok_or_else(|| ErroApi::interno("aparelho sumiu"))?,
    )
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

#[allow(clippy::too_many_arguments)]
async fn registrar_mudanca(
    tx: &mut sqlx::PgConnection,
    usuario: &Usuario,
    aparelho: Uuid,
    tipo: &str,
    motivo: &str,
    versao: i32,
    dados: serde_json::Value,
    correlacao: &str,
) -> Result<(), ErroApi> {
    let codigo = proximo(&mut *tx, "APM").await?;
    sqlx::query!(
        "INSERT INTO agenda.aparelho_mudanca (id, codigo, aparelho_id, tipo, motivo, versao_do_aparelho, dados, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'administracao', $9, $10)",
        novo_id(), codigo.como_str(), aparelho, tipo, motivo, versao, dados, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut *tx,
        NovoEvento {
            dominio: "integracao",
            tipo: &format!("aparelho_{tipo}"),
            registro_id: aparelho,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "motivo": motivo, "versao": versao, "dados": dados }),
            criado_por: Some(usuario.id),
            origem: "administracao",
            unidade_id: usuario.unidade_id,
            correlacao,
        },
    )
    .await?;
    Ok(())
}

/// Desativa ou reativa o aparelho, com motivo (TI e gerência). Atômico: só muda se estiver no estado
/// esperado. Desativado não recebe lista de trabalho; nada é apagado.
#[utoipa::path(post, operation_id = "mudar_aparelho", path = "/v1/aparelhos/{id}/mudancas", tag = "aparelhos", params(("id" = Uuid, Path)),
    request_body = MudancaDeAparelho,
    responses((status = 200, body = AparelhoCadastrado), (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn mudar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(m): Corpo<MudancaDeAparelho>,
) -> Result<Json<AparelhoCadastrado>, ErroApi> {
    usuario.exigir(Ato::AparelhoAdministrar)?;
    let ativar = match m.tipo.as_str() {
        "desativar" => false,
        "reativar" => true,
        _ => {
            return Err(ErroApi::invalido(
                "mudanca-invalida",
                "A mudança deve ser desativar ou reativar. Nada foi alterado.",
            ))
        }
    };
    let motivo = motivo_valido(&m.motivo)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let versao = sqlx::query_scalar!(
        "UPDATE agenda.aparelho SET ativo = $2, versao = versao + 1 WHERE id = $1 AND unidade_id = $3 AND ativo <> $2 RETURNING versao",
        id, ativar, usuario.unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(versao) = versao else {
        let existe = sqlx::query_scalar!(
            "SELECT ativo FROM agenda.aparelho WHERE id = $1 AND unidade_id = $2",
            id,
            usuario.unidade_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        return Err(match existe {
            None => ErroApi::nao_encontrado(
                "aparelho-nao-encontrado",
                "AparelhoCadastrado não encontrado nesta unidade. Nada foi alterado.",
            ),
            Some(_) => ErroApi::conflito(
                "aparelho-ja-nesse-estado",
                "O aparelho já estava assim. Nada foi alterado.",
            ),
        });
    };
    registrar_mudanca(
        &mut tx,
        &usuario,
        id,
        &m.tipo,
        &motivo,
        versao,
        json!({ "ativo": ativar }),
        &correlacao,
    )
    .await?;
    let r = carregar(&mut tx, Some(id))
        .await?
        .pop()
        .ok_or_else(|| ErroApi::interno("aparelho sumiu"))?;
    tx.commit().await?;
    Ok(Json(r))
}
