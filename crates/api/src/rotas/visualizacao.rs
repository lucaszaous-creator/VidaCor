//! Abrir as imagens de um estudo no visualizador (Visualizador.dc.html; ADR-0006, OHIF). O núcleo
//! decide quem abre e registra cada abertura (docs/18 § 6 item 9): estudo sem paciente conciliado
//! nunca é aberto para laudo; médico só com vínculo de cuidado ou acesso de emergência; técnico só
//! na sala em que está lotado. O endereço devolvido leva só o StudyInstanceUID (nada de nome na URL).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Serialize, ToSchema)]
pub struct Visualizacao {
    pub estudo_codigo: String,
    pub study_uid: String,
    pub imagens: i32,
    /// Endereço do visualizador com o estudo; nulo se o visualizador não estiver instalado.
    pub url: Option<String>,
}

async fn auditar(
    c: &mut PgConnection,
    quem: Uuid,
    estudo: Uuid,
    resultado: &str,
) -> Result<(), ErroApi> {
    sqlx::query!(
        "INSERT INTO auditoria.acesso (id, quem, quem_tipo, recurso, recurso_id, resultado, correlacao) VALUES ($1, $2, 'usuario', 'estudo.imagens', $3, $4, $5)",
        novo_id(), quem, estudo, resultado, crate::trace::atual().unwrap_or_default()
    )
    .execute(c)
    .await?;
    Ok(())
}

/// Autoriza e registra a abertura das imagens do estudo (médico com vínculo, técnico lotado).
#[utoipa::path(get, operation_id = "abrir_imagens_do_estudo", path = "/v1/estudos/{id}/visualizacao", tag = "exames", params(("id" = Uuid, Path)),
    responses((status = 200, body = Visualizacao), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc, description = "estudo-sem-paciente")))]
pub async fn abrir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Response, ErroApi> {
    if !(usuario.pode(Ato::LaudoLer) || usuario.pode(Ato::SalaBipar)) {
        return Err(ErroApi::proibido());
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let e = sqlx::query!(
        r#"SELECT e.codigo, e.study_uid, e.imagens, e.conciliado, t.paciente_id AS "paciente_id?", t.sala_id AS "sala_id?"
             FROM exames.estudo e LEFT JOIN recepcao.atendimento t ON t.id = e.atendimento_id WHERE e.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("estudo-nao-encontrado", "Estudo não encontrado. Nada foi aberto."))?;
    let (Some(paciente), Some(sala), true) = (e.paciente_id, e.sala_id, e.conciliado) else {
        auditar(&mut tx, usuario.id, id, "negado").await?;
        tx.commit().await?;
        return Err(ErroApi::conflito(
            "estudo-sem-paciente",
            "Este estudo ainda não tem paciente conciliado. As imagens não foram abertas. Concilie o estudo pela tela de estudos.",
        ));
    };
    let permitido = if usuario.pode(Ato::LaudoLer) {
        crate::vinculo::pode_ler(&mut tx, paciente).await?
    } else {
        sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM identidade.lotacao WHERE usuario_id = $1 AND sala_id = $2) AS "x!""#,
            usuario.id,
            sala
        )
        .fetch_one(&mut *tx)
        .await?
    };
    auditar(
        &mut tx,
        usuario.id,
        id,
        if permitido { "permitido" } else { "negado" },
    )
    .await?;
    tx.commit().await?;
    if !permitido {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "sem-vinculo",
            "Sem vínculo com o paciente",
            "Você não tem vínculo de cuidado com este paciente nem está lotado na sala do exame. As imagens não foram abertas. A tentativa foi registrada.",
        ));
    }
    let url = estado.config.visualizador_url.as_ref().map(|base| {
        format!(
            "{}?StudyInstanceUIDs={}",
            base.trim_end_matches('/'),
            e.study_uid
        )
    });
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(Visualizacao {
            estudo_codigo: e.codigo,
            study_uid: e.study_uid,
            imagens: e.imagens,
            url,
        }),
    )
        .into_response())
}
