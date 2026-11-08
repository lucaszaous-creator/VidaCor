//! Dossiê do lastro em PDF (gerência e auditor), com a emissão registrada no lastro.

use super::{
    clinica, data_hora, gerar, momento, proximo_codigo, registrar_emissao, resposta_pdf, Emissao,
};
use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::lastro::{self, NovoEvento};
use crate::rotas::lastro_explorador::{contexto_da_cadeia, montar, validar_codigo};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::State;
use axum::response::Response;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use utoipa::ToSchema;
use vidacor_documentos as doc;

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
