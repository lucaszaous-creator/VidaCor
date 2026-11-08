//! Documentos em PDF (Typst embutido, crate `vidacor-documentos`): laudo impresso para a equipe e
//! para o paciente no portal, e dossiê do lastro. O PDF só sai depois da mesma autorização da leitura
//! (vínculo de cuidado e RLS na equipe, RLS por paciente no portal, gerência e auditor no lastro);
//! cada entrega fica registrada em `documentos.emissao` com o SHA-256 dos bytes, devolvido também no
//! cabeçalho `X-Documento-Sha256`. Nada de dado de saúde na URL nem no log (SEC-DAD).

use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::lastro::{self, NovoEvento};
use crate::portal::{self, Paciente};
use crate::rotas::lastro_explorador::{contexto_da_cadeia, montar, validar_codigo};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};
use vidacor_documentos as doc;

/// Cabeçalho com o SHA-256 (hex) dos bytes do PDF entregue.
pub const CABECALHO_SHA256: &str = "x-documento-sha256";
/// Cabeçalho com o código da emissão registrada.
pub const CABECALHO_EMISSAO: &str = "x-documento-codigo";

fn data(d: DateTime<Utc>) -> String {
    d.with_timezone(&tempo::FUSO).format("%d/%m/%Y").to_string()
}

fn data_hora(d: DateTime<Utc>) -> String {
    d.with_timezone(&tempo::FUSO)
        .format("%d/%m/%Y às %H:%M")
        .to_string()
}

fn momento(d: DateTime<Utc>) -> doc::Momento {
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

fn idade(nascimento: NaiveDate, em: NaiveDate) -> i32 {
    let mut anos = em.year() - nascimento.year();
    if (em.month(), em.day()) < (nascimento.month(), nascimento.day()) {
        anos -= 1;
    }
    anos
}

/// Cabeçalho vigente da unidade; sem cadastro, só o nome da clínica e da unidade (nada inventado).
async fn clinica(conexao: &mut PgConnection, unidade_id: Uuid) -> Result<doc::Clinica, ErroApi> {
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

fn classificacao(sistema: &str, regra: &str) -> String {
    let nome = match sistema {
        "tirads" => "ACR TI-RADS",
        "birads" => "ACR BI-RADS",
        outro => outro,
    };
    format!("{nome} ({regra})")
}

fn milimetros(dmm: i32) -> String {
    if dmm % 10 == 0 {
        format!("{} mm", dmm / 10)
    } else {
        format!("{},{} mm", dmm / 10, dmm % 10)
    }
}

/// Monta o laudo impresso de uma versão assinada (vigente ou já substituída). `None` se a versão não
/// existe, não está assinada ou o contexto não a enxerga (RLS).
async fn montar_laudo(
    conexao: &mut PgConnection,
    laudo_id: Uuid,
    numero: i32,
    unidade_id: Uuid,
) -> Result<Option<doc::LaudoImpresso>, ErroApi> {
    let Some(v) = sqlx::query!(
        r#"SELECT l.codigo, p.nome, p.nascimento, ag.exame_nome, t.codigo AS protocolo, e.recebido_em,
                  v.numero, v.texto, v.situacao, v.motivo, v.assinada_em AS "assinada_em!", v.certificado,
                  v.hash_documento AS "hash_documento!", u.nome AS medico,
                  r.conselho AS "conselho?", r.uf AS "uf?", r.numero AS "registro?", r.rqe AS "rqe?"
             FROM laudos.versao v
             JOIN laudos.laudo l ON l.id = v.laudo_id
             JOIN identidade.paciente p ON p.id = l.paciente_id
             JOIN recepcao.atendimento t ON t.id = l.atendimento_id
             JOIN agenda.agendamento ag ON ag.id = t.agendamento_id
             JOIN exames.estudo e ON e.id = l.estudo_id
             JOIN identidade.usuario u ON u.id = v.assinada_por
             LEFT JOIN documentos.registro_profissional r ON r.usuario_id = v.assinada_por AND r.conselho = 'CRM' AND r.vigente
            WHERE v.laudo_id = $1 AND v.numero = $2 AND v.situacao IN ('assinada', 'substituida')"#,
        laudo_id,
        numero
    )
    .fetch_optional(&mut *conexao)
    .await?
    else {
        return Ok(None);
    };
    let achados = sqlx::query!(
        "SELECT numero, sistema, regra_versao, categoria, localizacao, maior_diametro_dmm, texto_achado, texto_conclusao
           FROM laudos.achado WHERE laudo_id = $1 AND versao_numero = $2 AND situacao = 'ativo' ORDER BY numero",
        laudo_id,
        numero
    )
    .fetch_all(&mut *conexao)
    .await?;
    let clinica = clinica(conexao, unidade_id).await?;
    let dia_do_exame = v.recebido_em.with_timezone(&tempo::FUSO).date_naive();
    let registro = match (v.conselho, v.uf, v.registro) {
        (Some(c), Some(uf), Some(n)) => Some(format!("{c}-{uf} {n}")),
        _ => None,
    };
    Ok(Some(doc::LaudoImpresso {
        clinica,
        codigo: v.codigo,
        versao: v.numero,
        paciente_nome: v.nome,
        paciente_nascimento: format!(
            "{} ({} anos)",
            v.nascimento.format("%d/%m/%Y"),
            idade(v.nascimento, dia_do_exame)
        ),
        protocolo: v.protocolo,
        exame: v.exame_nome,
        data_exame: data(v.recebido_em),
        texto: v.texto,
        achados: achados
            .into_iter()
            .map(|a| doc::AchadoImpresso {
                numero: a.numero,
                classificacao: classificacao(&a.sistema, &a.regra_versao),
                categoria: a.categoria,
                localizacao: a.localizacao,
                maior_diametro: a.maior_diametro_dmm.map(milimetros),
                descricao: a.texto_achado,
                conclusao: a.texto_conclusao,
            })
            .collect(),
        retificacao: (v.numero > 1).then(|| doc::Retificacao {
            versao_anterior: v.numero - 1,
            motivo: v.motivo.unwrap_or_default(),
        }),
        substituida: v.situacao == "substituida",
        medico: doc::MedicoImpresso {
            nome: v.medico,
            registro,
            rqe: v.rqe,
        },
        assinatura: doc::Assinatura {
            em: data_hora(v.assinada_em),
            certificado: v.certificado,
            hash_documento: v.hash_documento,
            momento: momento(v.assinada_em),
        },
    }))
}

/// Gera o PDF fora do executor assíncrono (composição tipográfica é trabalho de CPU).
async fn gerar<F>(f: F) -> Result<doc::Pdf, ErroApi>
where
    F: FnOnce() -> Result<doc::Pdf, doc::ErroDocumento> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(ErroApi::interno)?
        .map_err(ErroApi::interno)
}

struct Emissao<'a> {
    tipo: &'a str,
    registro_id: Uuid,
    registro_codigo: &'a str,
    versao_registro: Option<i32>,
    canal: &'a str,
    quem: Uuid,
    quem_tipo: &'a str,
    unidade_id: Uuid,
}

/// Próximo código legível do prefixo no ano corrente (DOC: impressão de laudo; DOS: dossiê).
async fn proximo_codigo(conexao: &mut PgConnection, prefixo: &str) -> Result<String, ErroApi> {
    let ano = tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero($1, $2)", prefixo, ano)
        .fetch_one(&mut *conexao)
        .await?
        .unwrap_or(0);
    Ok(CodigoLegivel::novo(prefixo, ano, n).to_string())
}

/// Grava a emissão (somente inserção) com o código dado. Exige contexto de sistema da unidade.
async fn registrar_emissao(
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

fn resposta_pdf(pdf: doc::Pdf, arquivo: &str, emissao: &str) -> Response {
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

async fn auditar(
    conexao: &mut PgConnection,
    quem: Uuid,
    quem_tipo: &str,
    recurso: &str,
    laudo: Uuid,
    resultado: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO auditoria.acesso (id, quem, quem_tipo, recurso, recurso_id, resultado, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        novo_id(), quem, quem_tipo, recurso, laudo, resultado, crate::trace::atual().unwrap_or_default()
    )
    .execute(conexao)
    .await?;
    Ok(())
}

fn nao_assinado() -> ErroApi {
    ErroApi::conflito(
        "laudo-nao-assinado",
        "Este laudo ainda não foi assinado e não pode ser impresso. Nenhum PDF foi gerado. A impressão fica disponível depois da assinatura.",
    )
}

/// PDF da versão assinada vigente (médico com vínculo de cuidado ou acesso de emergência). A leitura
/// é auditada como a do laudo; rascunho não imprime (409); retificação imprime a vigente e diz qual
/// versão ela substitui.
#[utoipa::path(get, operation_id = "baixar_pdf_do_laudo", path = "/v1/laudos/{id}/pdf", tag = "laudos", params(("id" = Uuid, Path)),
    responses((status = 200, description = "PDF/A-2b da versão assinada vigente; SHA-256 em X-Documento-Sha256", content_type = "application/pdf"),
              (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn laudo_pdf(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::LaudoLer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let Some(laudo) = crate::rotas::laudos::carregar(&mut tx, id).await? else {
        // RLS escondeu: sem vínculo (403, com oferta de emergência) ou inexistente (404). Auditado.
        let existe = sqlx::query_scalar!("SELECT laudos.paciente_do_laudo($1)", id)
            .fetch_one(&mut *tx)
            .await?;
        auditar(&mut tx, usuario.id, "usuario", "laudo.pdf", id, "negado").await?;
        tx.commit().await?;
        return Err(crate::rotas::laudos::negacao(existe.is_some()));
    };
    let numero = laudo.versao_vigente.ok_or_else(nao_assinado)?;
    let unidade_id = sqlx::query_scalar!("SELECT unidade_id FROM laudos.laudo WHERE id = $1", id)
        .fetch_one(&mut *tx)
        .await?;
    let impresso = montar_laudo(&mut tx, id, numero, unidade_id)
        .await?
        .ok_or_else(nao_assinado)?;
    let emergencia = !crate::vinculo::tem_vinculo(&mut tx, laudo.paciente.id).await?;
    let resultado = if emergencia {
        "emergencia"
    } else {
        "permitido"
    };
    auditar(&mut tx, usuario.id, "usuario", "laudo.pdf", id, resultado).await?;
    tx.commit().await?;

    let pdf = gerar(move || doc::laudo_pdf(&impresso)).await?;
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    let emissao = proximo_codigo(&mut tx, "DOC").await?;
    registrar_emissao(
        &mut tx,
        &emissao,
        &Emissao {
            tipo: "laudo_impresso",
            registro_id: id,
            registro_codigo: &laudo.codigo,
            versao_registro: Some(numero),
            canal: "equipe",
            quem: usuario.id,
            quem_tipo: "usuario",
            unidade_id,
        },
        &pdf,
    )
    .await?;
    tx.commit().await?;
    Ok(resposta_pdf(
        pdf,
        &format!("{}-v{numero}.pdf", laudo.codigo),
        &emissao,
    ))
}

/// PDF do laudo publicado do próprio paciente. De outro paciente ou inexistente: a mesma resposta 404
/// da leitura no portal, auditada (o RLS por paciente decide antes de qualquer outra leitura).
#[utoipa::path(get, operation_id = "baixar_pdf_do_laudo_no_portal", path = "/v1/portal/laudos/{id}/pdf", tag = "portal",
    params(("id" = Uuid, Path)),
    responses((status = 200, description = "PDF/A-2b da versão publicada; SHA-256 em X-Documento-Sha256", content_type = "application/pdf"),
              (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn portal_laudo_pdf(
    State(estado): State<Estado>,
    paciente: Paciente,
    Path(id): Path<Uuid>,
) -> Result<Response, ErroApi> {
    let mut tx = portal::transacao(&estado.banco, paciente.paciente_id).await?;
    let publicacao = sqlx::query!(
        "SELECT laudo_id, versao_numero, unidade_id FROM portal.publicacao WHERE laudo_id = $1 AND vigente",
        id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let resultado = if publicacao.is_some() {
        "permitido"
    } else {
        "negado"
    };
    auditar(
        &mut tx,
        paciente.paciente_id,
        "paciente",
        "portal.laudo.pdf",
        id,
        resultado,
    )
    .await?;
    tx.commit().await?;
    let p = publicacao.ok_or_else(crate::rotas::portal::nao_encontrado)?;

    // A posse foi provada pelo RLS do portal; o resto do documento vem em contexto de sistema da
    // unidade da publicação, só para este laudo e esta versão.
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(p.unidade_id)).await?;
    let impresso = montar_laudo(&mut tx, p.laudo_id, p.versao_numero, p.unidade_id)
        .await?
        .ok_or_else(crate::rotas::portal::nao_encontrado)?;
    tx.commit().await?;
    let codigo = impresso.codigo.clone();
    let numero = p.versao_numero;
    let pdf = gerar(move || doc::laudo_pdf(&impresso)).await?;
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(p.unidade_id)).await?;
    let emissao = proximo_codigo(&mut tx, "DOC").await?;
    registrar_emissao(
        &mut tx,
        &emissao,
        &Emissao {
            tipo: "laudo_impresso",
            registro_id: p.laudo_id,
            registro_codigo: &codigo,
            versao_registro: Some(numero),
            canal: "portal",
            quem: paciente.paciente_id,
            quem_tipo: "paciente",
            unidade_id: p.unidade_id,
        },
        &pdf,
    )
    .await?;
    tx.commit().await?;
    Ok(resposta_pdf(
        pdf,
        &format!("{codigo}-v{numero}.pdf"),
        &emissao,
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoDossie {
    /// Qualquer código do atendimento (o mesmo da busca no explorador).
    pub codigo: String,
}

/// Dossiê do lastro em PDF (gerência e auditor): a cadeia do atendimento com os hashes completos e a
/// conferência de cada evento. A emissão entra no lastro do atendimento com o SHA-256 do PDF.
#[utoipa::path(post, operation_id = "emitir_dossie_do_lastro", path = "/v1/lastro/dossie", tag = "lastro", request_body = PedidoDossie,
    responses((status = 200, description = "PDF/A-2b do dossiê; SHA-256 em X-Documento-Sha256", content_type = "application/pdf"),
              (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn dossie(
    State(estado): State<Estado>,
    usuario: Usuario,
    Corpo(p): Corpo<PedidoDossie>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::LastroExplorar)?;
    let codigo = validar_codigo(&p.codigo)?.to_owned();
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &contexto_da_cadeia(&usuario)).await?;
    let m = montar(&mut tx, usuario.unidade_id, &codigo).await?;
    let clinica = clinica(&mut tx, usuario.unidade_id).await?;
    let codigo_emissao = proximo_codigo(&mut tx, "DOS").await?;
    let agora = Utc::now();
    let quando = |d: DateTime<Utc>| {
        d.with_timezone(&tempo::FUSO)
            .format("%d/%m/%Y %H:%M:%S")
            .to_string()
    };
    let total = m.cadeia.eventos.len();
    let integra = m.cadeia.integra;
    let dossie = doc::DossieLastro {
        clinica,
        codigo_emissao: codigo_emissao.clone(),
        emitido_em: data_hora(agora),
        emitido_por: format!("{} ({})", usuario.nome, usuario.perfis.join(", ")),
        codigo_buscado: codigo.clone(),
        atendimento_codigo: m.cadeia.atendimento_codigo.clone(),
        paciente_nome: m.cadeia.paciente.nome.clone(),
        paciente_nascimento: m.cadeia.paciente.nascimento.format("%d/%m/%Y").to_string(),
        nos: m
            .cadeia
            .nos
            .iter()
            .map(|n| doc::NoDoDossie {
                tipo: n.tipo.clone(),
                codigo: n.codigo.clone(),
                situacao: n.situacao.clone(),
                criado_em: quando(n.criado_em),
            })
            .collect(),
        eventos: m
            .cadeia
            .eventos
            .iter()
            .zip(&m.hashes)
            .map(|(e, hash)| doc::EventoDoDossie {
                sequencia: e.sequencia,
                dominio: e.dominio.clone(),
                tipo: e.tipo.clone(),
                registro_codigo: e.registro_codigo.clone(),
                criado_em: quando(e.criado_em),
                origem: e.origem.clone(),
                hash: hash.clone(),
                integro: e.integro,
            })
            .collect(),
        momento: momento(agora),
    };
    let pdf = gerar(move || doc::dossie_pdf(&dossie)).await?;
    // Mesmo contexto da cadeia (sistema da unidade): a emissão e o evento entram na mesma transação.
    registrar_emissao(
        &mut tx,
        &codigo_emissao,
        &Emissao {
            tipo: "dossie_lastro",
            registro_id: m.atendimento_id,
            registro_codigo: &m.cadeia.atendimento_codigo,
            versao_registro: None,
            canal: "equipe",
            quem: usuario.id,
            quem_tipo: "usuario",
            unidade_id: usuario.unidade_id,
        },
        &pdf,
    )
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "lastro",
            tipo: "dossie_emitido",
            registro_id: m.atendimento_id,
            registro_codigo: Some(&m.cadeia.atendimento_codigo),
            dados: json!({
                "emissao": codigo_emissao, "sha256": pdf.sha256, "modelo": pdf.modelo,
                "eventos": total, "integra": integra, "por": usuario.id, "perfis": usuario.perfis,
            }),
            criado_por: Some(usuario.id),
            origem: "lastro",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(resposta_pdf(
        pdf,
        &format!("{codigo_emissao}.pdf"),
        &codigo_emissao,
    ))
}
