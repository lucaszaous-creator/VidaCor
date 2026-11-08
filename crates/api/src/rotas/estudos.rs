//! Fila de estudos sem conciliação e conciliação manual com dois identificadores (CLAUDE.md: imagem
//! sem paciente conciliado nunca vai para laudo; docs/27 FC-CTG-08; docs/18 § 5 vínculo manual).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::NomeCompleto;

#[derive(Serialize, ToSchema)]
pub struct EstudoPendente {
    pub id: Uuid,
    pub codigo: String,
    pub accession: Option<String>,
    pub imagens: i32,
    /// conciliacao ou conciliacao_contingencia.
    pub pendencia: Option<String>,
    pub recebido_em: DateTime<Utc>,
}

/// Estudos recebidos que ainda não têm paciente conciliado.
#[utoipa::path(get, path = "/v1/estudos/pendentes", tag = "exames", responses((status = 200, body = Vec<EstudoPendente>)))]
pub async fn pendentes(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<EstudoPendente>>, ErroApi> {
    usuario.exigir(Ato::EstudoConciliar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let l = sqlx::query_as!(
        EstudoPendente,
        "SELECT id, codigo, accession, imagens, pendencia, recebido_em FROM exames.estudo WHERE NOT conciliado ORDER BY recebido_em LIMIT 200"
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(l))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoConciliacao {
    pub atendimento_id: Uuid,
    pub nome: String,
    pub nascimento: NaiveDate,
}

/// Concilia o estudo com o atendimento depois de conferir nome e nascimento; só então entra no laudo.
#[utoipa::path(post, path = "/v1/estudos/{id}/conciliacao", tag = "exames", params(("id" = Uuid, Path)), request_body = PedidoConciliacao,
    responses((status = 200), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn conciliar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoConciliacao>,
) -> Result<Json<serde_json::Value>, ErroApi> {
    usuario.exigir(Ato::EstudoConciliar)?;
    let nome = NomeCompleto::novo(&p.nome).map_err(|_| {
        ErroApi::invalido(
            "nome-incompleto",
            "Informe o nome completo. Nada foi conciliado.",
        )
    })?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let e = sqlx::query!(
        "SELECT id, codigo, conciliado, unidade_id FROM exames.estudo WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("estudo-nao-encontrado", "Estudo não encontrado."))?;
    if e.conciliado {
        return Err(ErroApi::conflito(
            "estudo-ja-conciliado",
            "Este estudo já está conciliado. Nada foi alterado.",
        ));
    }
    let a = sqlx::query!(
        "SELECT t.id, t.codigo, p.nome, p.nascimento FROM recepcao.atendimento t JOIN identidade.paciente p ON p.id = t.paciente_id WHERE t.id = $1",
        p.atendimento_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("atendimento-nao-encontrado", "Atendimento não encontrado."))?;
    let confere = NomeCompleto::novo(&a.nome)
        .map(|n| n.confere_com(&nome))
        .unwrap_or(false)
        && a.nascimento == p.nascimento;
    if !confere {
        return Err(ErroApi::invalido("identificacao-nao-confere", "Nome ou nascimento não conferem com o atendimento. O estudo não foi conciliado e continua fora do laudo."));
    }
    tx.commit().await?;
    // Decisão tomada e conferida no contexto do médico; a gravação (estudo, laudo e vínculo) é do
    // núcleo, com o médico registrado no lastro como quem conciliou.
    let mut tx = banco::transacao(
        &estado.banco,
        &crate::banco::ContextoBanco::sistema(e.unidade_id),
    )
    .await?;
    let alterou = sqlx::query!(
        "UPDATE exames.estudo SET conciliado = true, atendimento_id = $2, pendencia = NULL, versao = versao + 1 WHERE id = $1 AND NOT conciliado",
        e.id, a.id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if alterou != 1 {
        return Err(ErroApi::conflito(
            "estudo-ja-conciliado",
            "Este estudo acabou de ser conciliado por outra pessoa. Nada foi alterado.",
        ));
    }
    crate::laudos::criar_para_estudo(&mut tx, e.id, a.id, e.unidade_id, &correlacao).await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "exames",
            tipo: "estudo_conciliado_manual",
            registro_id: e.id,
            registro_codigo: Some(&e.codigo),
            dados: json!({ "atendimento": a.codigo, "conferencia": "nome_e_nascimento" }),
            criado_por: Some(usuario.id),
            origem: "consultorio",
            unidade_id: e.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Json(
        json!({ "estudo": e.codigo, "atendimento": a.codigo, "conciliado": true }),
    ))
}
