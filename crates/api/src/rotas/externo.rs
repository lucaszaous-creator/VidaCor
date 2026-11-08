//! Exame externo (docs/22 E3, Exame-Externo.dc.html): importado de outra instituição para comparação,
//! só leitura, nunca vira laudo da VidaCor (tabelas próprias). A recepção confere os dois
//! identificadores: nascimento igual e nome da origem compatível com o cadastro, com o documento.

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
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::{novo_id, NomeCompleto};
use vidacor_dominio_recepcao::cadastro::nome_dicom_compativel;

#[derive(Deserialize, Serialize, ToSchema, Clone)]
pub struct EstudoExterno {
    pub study_uid: String,
    pub descricao: String,
    pub modalidade: String,
    pub data_estudo: NaiveDate,
    pub imagens: i32,
    /// SHA-256 (hex) do conjunto recebido.
    pub sha256: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ImportacaoExterna {
    pub instituicao: String,
    /// link, arquivo ou dicomweb.
    pub origem: String,
    /// Nome como veio nas imagens (formato DICOM, ex.: PRADO^LUCIA^F).
    pub nome_na_origem: String,
    pub nascimento_na_origem: NaiveDate,
    pub id_na_origem: Option<String>,
    /// A recepção conferiu com o documento do paciente.
    pub documento_conferido: bool,
    pub estudos: Vec<EstudoExterno>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ExameExterno {
    pub id: Uuid,
    pub codigo: String,
    pub instituicao: String,
    pub origem: String,
    /// Identificação original guardada (IHE PIR).
    pub nome_na_origem: String,
    pub id_na_origem: Option<String>,
    pub criado_em: DateTime<Utc>,
    pub estudos: Vec<EstudoExterno>,
    /// Sempre verdadeiro: exame externo é só leitura e nunca vira laudo da VidaCor.
    pub somente_leitura: bool,
}

async fn carregar(
    tx: &mut sqlx::PgConnection,
    paciente: Uuid,
    id: Option<Uuid>,
) -> Result<Vec<ExameExterno>, ErroApi> {
    let externos = sqlx::query!(
        "SELECT id, codigo, instituicao, origem, nome_na_origem, id_na_origem, criado_em FROM exames.externo
          WHERE paciente_id = $1 AND ($2::uuid IS NULL OR id = $2) ORDER BY criado_em DESC LIMIT 100",
        paciente,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    let ids: Vec<Uuid> = externos.iter().map(|e| e.id).collect();
    let estudos = sqlx::query!(
        "SELECT externo_id, study_uid, descricao, modalidade, data_estudo, imagens, sha256 FROM exames.externo_estudo WHERE externo_id = ANY($1) ORDER BY data_estudo DESC",
        &ids
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(externos
        .into_iter()
        .map(|e| ExameExterno {
            estudos: estudos
                .iter()
                .filter(|s| s.externo_id == e.id)
                .map(|s| EstudoExterno {
                    study_uid: s.study_uid.clone(),
                    descricao: s.descricao.clone(),
                    modalidade: s.modalidade.clone(),
                    data_estudo: s.data_estudo,
                    imagens: s.imagens,
                    sha256: s.sha256.clone(),
                })
                .collect(),
            id: e.id,
            codigo: e.codigo,
            instituicao: e.instituicao,
            origem: e.origem,
            nome_na_origem: e.nome_na_origem,
            id_na_origem: e.id_na_origem,
            criado_em: e.criado_em,
            somente_leitura: true,
        })
        .collect())
}

/// Exames externos do paciente (recepção, técnico, médico, gerência).
#[utoipa::path(get, operation_id = "listar_exames_externos", path = "/v1/pacientes/{id}/exames-externos", tag = "exames", params(("id" = Uuid, Path)),
    responses((status = 200, body = Vec<ExameExterno>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(paciente): Path<Uuid>,
) -> Result<Json<Vec<ExameExterno>>, ErroApi> {
    if !(usuario.pode(Ato::RecepcaoVer) || usuario.pode(Ato::SalaVer)) {
        return Err(ErroApi::proibido());
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = carregar(&mut tx, paciente, None).await?;
    tx.commit().await?;
    Ok(Json(r))
}

/// Importa o exame externo depois da conferência de identidade (recepção; Idempotency-Key).
#[utoipa::path(post, operation_id = "importar_exame_externo", path = "/v1/pacientes/{id}/exames-externos", tag = "exames", params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)),
    request_body = ImportacaoExterna,
    responses((status = 201, body = ExameExterno), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn importar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(paciente): Path<Uuid>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(i): Corpo<ImportacaoExterna>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::ExameExternoImportar)?;
    if !["link", "arquivo", "dicomweb"].contains(&i.origem.as_str())
        || i.instituicao.trim().len() < 2
        || i.estudos.is_empty()
        || i.estudos.len() > 50
    {
        return Err(ErroApi::invalido(
            "importacao-invalida",
            "Informe a instituição, a origem e de 1 a 50 estudos. Nada foi importado.",
        ));
    }
    if i.estudos.iter().any(|e| {
        e.imagens <= 0
            || e.sha256.len() != 64
            || !e
                .sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || e.study_uid.is_empty()
            || e.study_uid.len() > 64
            || !e.study_uid.bytes().all(|b| b.is_ascii_digit() || b == b'.')
    }) {
        return Err(ErroApi::invalido(
            "estudo-invalido",
            "Um estudo veio sem identificação, sem imagens ou sem o hash. Nada foi importado.",
        ));
    }
    if !i.documento_conferido {
        return Err(ErroApi::invalido("documento-nao-conferido", "Confira o nome e o nascimento com o documento do paciente antes de importar. Nada foi importado."));
    }
    const ROTA: &str = "POST /v1/pacientes/{id}/exames-externos";
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
    let p = sqlx::query!(
        "SELECT nome, nascimento FROM identidade.paciente WHERE id = $1",
        paciente
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "paciente-nao-encontrado",
            "Paciente não encontrado. Nada foi importado.",
        )
    })?;
    if p.nascimento != i.nascimento_na_origem {
        return Err(ErroApi::conflito("nascimento-diferente", "O nascimento no exame externo é diferente do cadastro. Nada foi importado: pode ser exame de outra pessoa."));
    }
    let nome = NomeCompleto::novo(&p.nome).map_err(ErroApi::interno)?;
    if !nome_dicom_compativel(&i.nome_na_origem, &nome) {
        return Err(ErroApi::conflito("nome-incompativel", "O nome no exame externo não combina com o cadastro. Nada foi importado: confira se é a mesma pessoa."));
    }
    let id = novo_id();
    let codigo = proximo(&mut tx, "EXT").await?;
    sqlx::query!(
        "INSERT INTO exames.externo (id, codigo, paciente_id, instituicao, origem, nome_na_origem, nascimento_na_origem, id_na_origem, documento_conferido, conferido_por, criado_por, origem_registro, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true, $9, $9, 'recepcao', $10, $11)",
        id, codigo.como_str(), paciente, i.instituicao.trim(), i.origem, i.nome_na_origem, i.nascimento_na_origem, i.id_na_origem, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    for e in &i.estudos {
        sqlx::query!(
            "INSERT INTO exames.externo_estudo (id, externo_id, paciente_id, study_uid, descricao, modalidade, data_estudo, imagens, sha256, criado_por, origem, unidade_id, correlacao)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'recepcao', $11, $12)",
            novo_id(), id, paciente, e.study_uid, e.descricao.chars().take(200).collect::<String>(), e.modalidade, e.data_estudo, e.imagens, e.sha256, usuario.id, usuario.unidade_id, correlacao
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| match &err {
            sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("estudo-ja-importado", "Um destes estudos já foi importado para este paciente. Nada foi importado de novo."),
            sqlx::Error::Database(d) if d.is_check_violation() => ErroApi::invalido("estudo-invalido", "Modalidade ou descrição do estudo inválida. Nada foi importado."),
            _ => ErroApi::from(err),
        })?;
    }
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "exames", tipo: "exame_externo_importado", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "estudos": i.estudos.iter().map(|e| json!({ "study_uid": e.study_uid, "sha256": e.sha256 })).collect::<Vec<_>>(), "origem": i.origem, "conferido_por": usuario.id }),
        criado_por: Some(usuario.id), origem: "recepcao", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let corpo = serde_json::to_value(
        carregar(&mut tx, paciente, Some(id))
            .await?
            .pop()
            .ok_or_else(|| ErroApi::interno("importação sumiu"))?,
    )
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}
