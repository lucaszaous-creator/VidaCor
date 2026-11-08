//! Documentos em PDF (Typst embutido, crate `vidacor-documentos`): laudo impresso para a equipe e
//! para o paciente no portal, e dossiê do lastro. O PDF só sai depois da mesma autorização da leitura
//! (vínculo de cuidado e RLS na equipe, RLS por paciente no portal, gerência e auditor no lastro);
//! cada entrega fica registrada em `documentos.emissao` com o SHA-256 dos bytes, devolvido também no
//! cabeçalho `X-Documento-Sha256`. Nada de dado de saúde na URL nem no log (SEC-DAD).

pub mod dossie;
pub mod laudo;

use crate::erro::ErroApi;
use crate::tempo;
use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::{DateTime, Datelike, Timelike, Utc};
use sqlx::PgConnection;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};
use vidacor_documentos as doc;

/// Cabeçalho com o SHA-256 (hex) dos bytes do PDF entregue.
pub const CABECALHO_SHA256: &str = "x-documento-sha256";
/// Cabeçalho com o código da emissão registrada.
pub const CABECALHO_EMISSAO: &str = "x-documento-codigo";

pub(super) fn data(d: DateTime<Utc>) -> String {
    d.with_timezone(&tempo::FUSO).format("%d/%m/%Y").to_string()
}

pub(super) fn data_hora(d: DateTime<Utc>) -> String {
    d.with_timezone(&tempo::FUSO)
        .format("%d/%m/%Y às %H:%M")
        .to_string()
}

pub(super) fn momento(d: DateTime<Utc>) -> doc::Momento {
    // Componentes de data e hora sempre cabem em u8 (mês, dia, hora, minuto, segundo).
    let pequeno = |v: u32| u8::try_from(v).unwrap_or(0);
    doc::Momento {
        ano: d.year(),
        mes: pequeno(d.month()),
        dia: pequeno(d.day()),
        hora: pequeno(d.hour()),
        minuto: pequeno(d.minute()),
        segundo: pequeno(d.second()),
    }
}

/// Cabeçalho vigente da unidade; sem cadastro, só o nome da clínica e da unidade (nada inventado).
pub(super) async fn clinica(
    conexao: &mut PgConnection,
    unidade_id: Uuid,
) -> Result<doc::Clinica, ErroApi> {
    let c = sqlx::query!(
        "SELECT nome, endereco, telefone, site, cnpj, responsavel_tecnico FROM documentos.cabecalho WHERE unidade_id = $1 AND vigente",
        unidade_id
    )
    .fetch_optional(&mut *conexao)
    .await?;
    if let Some(c) = c {
        return Ok(doc::Clinica {
            nome: c.nome,
            endereco: c.endereco,
            telefone: c.telefone,
            site: c.site,
            cnpj: c.cnpj,
            responsavel_tecnico: c.responsavel_tecnico,
        });
    }
    let unidade = sqlx::query_scalar!(
        "SELECT nome FROM identidade.unidade WHERE id = $1",
        unidade_id
    )
    .fetch_one(&mut *conexao)
    .await?;
    Ok(doc::Clinica {
        nome: "VidaCor Imagem".into(),
        endereco: Some(format!("Unidade {unidade}")),
        telefone: None,
        site: None,
        cnpj: None,
        responsavel_tecnico: None,
    })
}

/// Gera o PDF fora do executor assíncrono (composição tipográfica é trabalho de CPU).
pub(super) async fn gerar<F>(f: F) -> Result<doc::Pdf, ErroApi>
where
    F: FnOnce() -> Result<doc::Pdf, doc::ErroDocumento> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(ErroApi::interno)?
        .map_err(ErroApi::interno)
}

pub(super) struct Emissao<'a> {
    pub(super) tipo: &'a str,
    pub(super) registro_id: Uuid,
    pub(super) registro_codigo: &'a str,
    pub(super) versao_registro: Option<i32>,
    pub(super) canal: &'a str,
    pub(super) quem: Uuid,
    pub(super) quem_tipo: &'a str,
    pub(super) unidade_id: Uuid,
}

/// Próximo código legível do prefixo no ano corrente (DOC: impressão de laudo; DOS: dossiê).
pub(super) async fn proximo_codigo(
    conexao: &mut PgConnection,
    prefixo: &str,
) -> Result<String, ErroApi> {
    let ano = tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero($1, $2)", prefixo, ano)
        .fetch_one(&mut *conexao)
        .await?
        .unwrap_or(0);
    Ok(CodigoLegivel::novo(prefixo, ano, n).to_string())
}

/// Grava a emissão (somente inserção) com o código dado. Exige contexto de sistema da unidade.
pub(super) async fn registrar_emissao(
    conexao: &mut PgConnection,
    codigo: &str,
    e: &Emissao<'_>,
    pdf: &doc::Pdf,
) -> Result<(), ErroApi> {
    sqlx::query!(
        "INSERT INTO documentos.emissao (id, codigo, tipo, registro_id, registro_codigo, versao_registro, modelo, sha256, bytes,
                                         canal, quem, quem_tipo, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        novo_id(), codigo, e.tipo, e.registro_id, e.registro_codigo, e.versao_registro, pdf.modelo, pdf.sha256,
        i32::try_from(pdf.bytes.len()).unwrap_or(i32::MAX), e.canal, e.quem, e.quem_tipo, e.canal, e.unidade_id,
        crate::trace::atual().unwrap_or_default()
    )
    .execute(&mut *conexao)
    .await?;
    Ok(())
}

pub(super) fn resposta_pdf(pdf: doc::Pdf, arquivo: &str, emissao: &str) -> Response {
    let sha = pdf.sha256.clone();
    let digest = sha
        .as_bytes()
        .chunks(2)
        .filter_map(|par| {
            std::str::from_utf8(par)
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        })
        .collect::<Vec<u8>>();
    let mut r = (StatusCode::OK, pdf.bytes).into_response();
    let h = r.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    // Nome do arquivo só com códigos (nunca nome do paciente nem exame).
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{arquivo}\"")) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    if let Ok(v) = HeaderValue::from_str(&sha) {
        h.insert(HeaderName::from_static(CABECALHO_SHA256), v);
    }
    if let Ok(v) = HeaderValue::from_str(&format!("sha-256=:{}:", STANDARD.encode(digest))) {
        h.insert(HeaderName::from_static("repr-digest"), v);
    }
    if let Ok(v) = HeaderValue::from_str(emissao) {
        h.insert(HeaderName::from_static(CABECALHO_EMISSAO), v);
    }
    r
}
