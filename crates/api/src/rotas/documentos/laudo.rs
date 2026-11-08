//! Laudo impresso: equipe (vínculo de cuidado) e portal (RLS por paciente), só versão assinada.

use super::{
    clinica, data, data_hora, gerar, momento, proximo_codigo, registrar_emissao, resposta_pdf,
    Emissao,
};
use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::portal::{self, Paciente};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::response::Response;
use chrono::{Datelike, NaiveDate};
use sqlx::PgConnection;
use uuid::Uuid;
use vidacor_comum::novo_id;
use vidacor_documentos as doc;

fn idade(nascimento: NaiveDate, em: NaiveDate) -> i32 {
    let mut anos = em.year() - nascimento.year();
    if (em.month(), em.day()) < (nascimento.month(), nascimento.day()) {
        anos -= 1;
    }
    anos
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
