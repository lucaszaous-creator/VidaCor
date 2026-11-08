//! Busca de paciente e correção de cadastro com versão e motivo (docs/27 FC-ETQ-04, docs/16 R5).

use crate::atos::Ato;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{Corpo, Estacao};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::PacienteResumo;
use crate::sessao::Usuario;
use crate::{banco, tempo};
use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::{novo_id, DataNascimento};

#[derive(Deserialize, IntoParams)]
pub struct Busca {
    /// Parte do nome (mínimo 3 letras).
    pub busca: String,
}

/// Busca pacientes da unidade pelo nome (até 20), com os dois identificadores.
#[utoipa::path(get, path = "/v1/pacientes", tag = "pacientes", params(Busca),
    responses((status = 200, body = Vec<PacienteResumo>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn buscar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(q): Query<Busca>,
) -> Result<Json<Vec<PacienteResumo>>, ErroApi> {
    usuario.exigir(Ato::AgendaVer)?;
    let termo = q.busca.trim();
    if termo.chars().count() < 3 || termo.chars().count() > 100 {
        return Err(ErroApi::invalido(
            "busca-curta",
            "Digite pelo menos 3 letras do nome. Nada foi buscado.",
        ));
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let linhas = sqlx::query_as!(
        PacienteResumo,
        "SELECT id, nome, nascimento FROM identidade.paciente
          WHERE nome ILIKE '%' || $1 || '%' ORDER BY nome, nascimento LIMIT 20",
        termo.replace(['%', '_'], "")
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(linhas))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoCorrecao {
    pub nascimento: NaiveDate,
    pub motivo: String,
}

#[derive(Serialize, ToSchema)]
pub struct Correcao {
    pub paciente: PacienteResumo,
    pub versao: i32,
    /// Etiquetas que foram invalidadas e precisam ser reimpressas.
    pub etiquetas_invalidadas: Vec<String>,
}

/// Corrige a data de nascimento com motivo: guarda a versão anterior e invalida as etiquetas válidas.
#[utoipa::path(post, path = "/v1/pacientes/{id}/correcao", tag = "pacientes", params(("id" = Uuid, Path)),
    request_body = PedidoCorrecao,
    responses((status = 200, body = Correcao), (status = 401, body = crate::openapi::ProblemaDoc),
              (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn corrigir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoCorrecao>,
) -> Result<Json<Correcao>, ErroApi> {
    usuario.exigir(Ato::PacienteCorrigir)?;
    usuario.exigir_reautenticacao()?;
    let motivo = p.motivo.trim().to_owned();
    if motivo.chars().count() < 3 {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Informe o motivo da correção. Nada foi alterado.",
        ));
    }
    DataNascimento::nova(p.nascimento, tempo::hoje_local()).map_err(|_| {
        ErroApi::invalido(
            "nascimento-invalido",
            "A data de nascimento não é válida. Nada foi alterado.",
        )
    })?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let atual = sqlx::query!(
        "SELECT id, codigo, nome, nascimento, sexo, versao, unidade_id FROM identidade.paciente WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("paciente-nao-encontrado", "Este paciente não existe nesta unidade. Nada foi alterado."))?;
    // Guarda a versão anterior antes de corrigir (nada se perde, docs/16 R5).
    sqlx::query!(
        "INSERT INTO identidade.paciente_versao
           (id, paciente_id, versao, nome, nascimento, sexo, motivo, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'balcao', $9, $10, $11)",
        novo_id(),
        atual.id,
        atual.versao,
        atual.nome,
        atual.nascimento,
        atual.sexo,
        motivo,
        usuario.id,
        estacao,
        atual.unidade_id,
        correlacao
    )
    .execute(&mut *tx)
    .await?;
    let versao = sqlx::query_scalar!(
        "UPDATE identidade.paciente SET nascimento = $2, versao = versao + 1 WHERE id = $1 RETURNING versao",
        atual.id,
        p.nascimento
    )
    .fetch_one(&mut *tx)
    .await?;
    let invalidadas = sqlx::query_scalar!(
        "UPDATE recepcao.etiqueta SET situacao = 'invalidada', motivo_invalidacao = 'cadastro corrigido: ' || $2,
                invalidada_por = $3, invalidada_em = now(), versao = versao + 1
          WHERE paciente_id = $1 AND situacao = 'valida' RETURNING codigo",
        atual.id,
        motivo,
        usuario.id
    )
    .fetch_all(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "pacientes",
            tipo: "cadastro_corrigido",
            registro_id: atual.id,
            registro_codigo: Some(&atual.codigo),
            dados: json!({ "campo": "nascimento", "versao_anterior": atual.versao, "motivo": motivo, "etiquetas_invalidadas": invalidadas }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: atual.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Json(Correcao {
        paciente: PacienteResumo {
            id: atual.id,
            nome: atual.nome,
            nascimento: p.nascimento,
        },
        versao,
        etiquetas_invalidadas: invalidadas,
    }))
}
