//! Relatórios e exportação (Relatorios.dc.html; DEC-2026-11-09-150 a 154). Catálogo fechado calculado
//! no núcleo, prévia na tela e exportação em CSV com reautenticação (SEC-AUT-5). Toda exportação fica
//! registrada com o SHA-256 do arquivo e entra no lastro; o arquivo não é guardado no servidor nem
//! no navegador (baixa uma vez, na hora). Dado de saúde nunca sai com nome: paciente vira PAC e exame
//! vira TUSS.

pub mod consultas;

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use consultas::{Definicao, LinhaDoRelatorio};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use utoipa::{IntoParams, ToSchema};
use vidacor_comum::novo_id;

#[derive(Serialize, ToSchema)]
pub struct RelatorioDoCatalogo {
    pub id: String,
    pub grupo: String,
    pub titulo: String,
    pub descricao: String,
    /// O relatório tem dado de saúde: no arquivo, paciente vira código PAC e exame vira código TUSS.
    pub dado_de_saude: bool,
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct PeriodoDoRelatorio {
    /// Primeiro dia (AAAA-MM-DD). Sem data: primeiro dia do mês corrente.
    pub de: Option<NaiveDate>,
    /// Último dia, inclusive (AAAA-MM-DD). Sem data: hoje. Até 366 dias.
    pub ate: Option<NaiveDate>,
}

#[derive(Serialize, ToSchema)]
pub struct PreviaDoRelatorio {
    pub id: String,
    pub titulo: String,
    pub de: NaiveDate,
    pub ate: NaiveDate,
    pub colunas: Vec<String>,
    pub linhas: Vec<LinhaDoRelatorio>,
    pub dado_de_saude: bool,
    pub gerado_em: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct ExportacaoFeita {
    pub codigo: String,
    pub relatorio: String,
    pub titulo: String,
    pub formato: String,
    pub de: NaiveDate,
    pub ate: NaiveDate,
    /// sem_dado_de_saude ou pseudonimizado.
    pub conteudo: String,
    pub linhas: i32,
    pub sha256: String,
    pub quem: String,
    pub quando: DateTime<Utc>,
}

fn catalogo_do(usuario: &Usuario) -> impl Iterator<Item = &'static Definicao> + '_ {
    consultas::CATALOGO
        .iter()
        .filter(|d| d.perfis.iter().any(|p| usuario.tem_perfil(p)))
}

fn definicao(usuario: &Usuario, id: &str) -> Result<&'static Definicao, ErroApi> {
    usuario.exigir(Ato::RelatoriosVer)?;
    let d = consultas::buscar(id).ok_or_else(|| {
        ErroApi::nao_encontrado(
            "relatorio-inexistente",
            "Este relatório não existe no catálogo.",
        )
    })?;
    if d.perfis.iter().any(|p| usuario.tem_perfil(p)) {
        Ok(d)
    } else {
        Err(ErroApi::proibido())
    }
}

/// Período pedido em dias locais e os limites em UTC.
struct Periodo {
    de: NaiveDate,
    ate: NaiveDate,
    inicio: DateTime<Utc>,
    fim: DateTime<Utc>,
    dias: i64,
}

/// Período em dias locais e limites em UTC (puro na validação).
fn periodo(p: &PeriodoDoRelatorio) -> Result<Periodo, ErroApi> {
    let hoje = tempo::hoje_local();
    let ate = p.ate.unwrap_or(hoje);
    let de = p.de.unwrap_or_else(|| ate.with_day0(0).unwrap_or(ate));
    let invalido = || {
        ErroApi::invalido(
            "periodo-invalido",
            "O período vai de 1 a 366 dias, com o início antes do fim. Nada foi gerado. Ajuste as datas.",
        )
    };
    if de > ate || (ate - de).num_days() > 365 {
        return Err(invalido());
    }
    let (i, _) = tempo::limites_do_dia(de).ok_or_else(invalido)?;
    let (_, f) = tempo::limites_do_dia(ate).ok_or_else(invalido)?;
    Ok(Periodo {
        de,
        ate,
        inicio: i,
        fim: f,
        dias: (ate - de).num_days() + 1,
    })
}

/// Relatórios que o perfil pode ver.
#[utoipa::path(get, operation_id = "catalogo_de_relatorios", path = "/v1/relatorios", tag = "relatorios",
    responses((status = 200, body = Vec<RelatorioDoCatalogo>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn catalogo(usuario: Usuario) -> Result<Json<Vec<RelatorioDoCatalogo>>, ErroApi> {
    usuario.exigir(Ato::RelatoriosVer)?;
    Ok(Json(
        catalogo_do(&usuario)
            .map(|d| RelatorioDoCatalogo {
                id: d.id.into(),
                grupo: d.grupo.into(),
                titulo: d.titulo.into(),
                descricao: d.descricao.into(),
                dado_de_saude: d.saude,
            })
            .collect(),
    ))
}

/// Prévia do relatório no período (as mesmas linhas que vão para o arquivo).
#[utoipa::path(get, operation_id = "previa_do_relatorio", path = "/v1/relatorios/{id}", tag = "relatorios",
    params(("id" = String, Path), PeriodoDoRelatorio),
    responses((status = 200, body = PreviaDoRelatorio), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn previa(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<String>,
    Query(p): Query<PeriodoDoRelatorio>,
) -> Result<Response, ErroApi> {
    let d = definicao(&usuario, &id)?;
    let Periodo {
        de,
        ate,
        inicio: i,
        fim: f,
        dias,
    } = periodo(&p)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let linhas = consultas::linhas(&mut tx, d.id, i, f, dias).await?;
    tx.commit().await?;
    let corpo = PreviaDoRelatorio {
        id: d.id.into(),
        titulo: d.titulo.into(),
        de,
        ate,
        colunas: d.colunas.iter().map(|c| (*c).to_owned()).collect(),
        linhas,
        dado_de_saude: d.saude,
        gerado_em: tempo::agora(),
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(corpo)).into_response())
}

/// Exporta o relatório em CSV (reautenticação). O arquivo vem na resposta e não fica guardado; o
/// registro guarda o SHA-256, o filtro e se havia dado de saúde (removido).
#[utoipa::path(post, operation_id = "exportar_relatorio", path = "/v1/relatorios/{id}/exportacao", tag = "relatorios",
    params(("id" = String, Path)), request_body = PeriodoDoRelatorio,
    responses((status = 200, description = "CSV (UTF-8, separador ;); SHA-256 em X-Documento-Sha256 e código em X-Documento-Codigo", content_type = "text/csv"),
              (status = 401, body = crate::openapi::ProblemaDoc), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn exportar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<String>,
    Corpo(p): Corpo<PeriodoDoRelatorio>,
) -> Result<Response, ErroApi> {
    let d = definicao(&usuario, &id)?;
    usuario.exigir_reautenticacao()?;
    let Periodo {
        de,
        ate,
        inicio: i,
        fim: f,
        dias,
    } = periodo(&p)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let linhas = consultas::linhas(&mut tx, d.id, i, f, dias).await?;
    let bytes = consultas::csv(&d.colunas, &linhas);
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let codigo = proximo(&mut tx, "EXP").await?;
    let eid = novo_id();
    let conteudo = if d.saude {
        "pseudonimizado"
    } else {
        "sem_dado_de_saude"
    };
    let n = i32::try_from(linhas.len()).unwrap_or(i32::MAX);
    sqlx::query!(
        "INSERT INTO relatorios.exportacao (id, codigo, relatorio, formato, de, ate, conteudo, linhas, sha256, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, 'csv', $4, $5, $6, $7, $8, $9, 'relatorios', $10, $11)",
        eid, codigo.como_str(), d.id, de, ate, conteudo, n, sha, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "relatorios",
            tipo: "relatorio_exportado",
            registro_id: eid,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "relatorio": d.id, "de": de, "ate": ate, "formato": "csv", "conteudo": conteudo,
                           "linhas": n, "sha256": sha, "por": usuario.id, "perfis": usuario.perfis }),
            criado_por: Some(usuario.id),
            origem: "relatorios",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    let mut r = (StatusCode::OK, bytes).into_response();
    let h = r.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    // Nome do arquivo só com códigos e datas.
    if let Ok(v) = HeaderValue::from_str(&format!(
        "attachment; filename=\"{}-{}-{de}-{ate}.csv\"",
        codigo.como_str(),
        d.id
    )) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    if let Ok(v) = HeaderValue::from_str(&sha) {
        h.insert(HeaderName::from_static("x-documento-sha256"), v);
    }
    if let Ok(v) = HeaderValue::from_str(codigo.como_str()) {
        h.insert(HeaderName::from_static("x-documento-codigo"), v);
    }
    Ok(r)
}

/// Exportações feitas: as próprias; a gerência vê as da unidade (últimos 90 dias).
#[utoipa::path(get, operation_id = "exportacoes_feitas", path = "/v1/relatorios/exportacoes", tag = "relatorios",
    responses((status = 200, body = Vec<ExportacaoFeita>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn exportacoes(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<ExportacaoFeita>>, ErroApi> {
    usuario.exigir(Ato::RelatoriosVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = sqlx::query!(
        r#"SELECT e.codigo, e.relatorio, e.formato, e.de, e.ate, e.conteudo, e.linhas, e.sha256, u.nome AS quem, e.criado_em
             FROM relatorios.exportacao e JOIN identidade.usuario u ON u.id = e.criado_por
            WHERE e.criado_em >= $1 ORDER BY e.criado_em DESC LIMIT 200"#,
        tempo::agora() - Duration::days(90)
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        r.into_iter()
            .map(|e| ExportacaoFeita {
                titulo: consultas::buscar(&e.relatorio)
                    .map_or_else(|| e.relatorio.clone(), |d| d.titulo.to_owned()),
                codigo: e.codigo,
                relatorio: e.relatorio,
                formato: e.formato,
                de: e.de,
                ate: e.ate,
                conteudo: e.conteudo,
                linhas: e.linhas,
                sha256: e.sha256,
                quem: e.quem,
                quando: e.criado_em,
            })
            .collect(),
    ))
}
