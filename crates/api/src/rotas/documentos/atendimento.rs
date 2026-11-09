//! Documentos do atendimento (Preparo-Exame.dc.html e Documentos.dc.html; DEC-2026-11-09-107 e 108):
//! comprovante do agendamento com o preparo (recepção e gerência), declaração de comparecimento
//! (recepção e gerência) e recibo do recebimento pago (caixa). A leitura passa pelo RLS de quem pede
//! (outra unidade ou inexistente: 404); a emissão fica em `documentos.emissao` com o SHA-256 e no
//! lastro do registro. Nome do paciente nunca vai para a URL nem para o nome do arquivo.

use super::{
    clinica, data, data_hora, gerar, momento, proximo_codigo, registrar_emissao, resposta_pdf,
    Emissao,
};
use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::lastro::{self, NovoEvento};
use crate::rotas::paciente_visao::mascarar_cpf;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::response::Response;
use chrono::{Datelike, Duration, Utc, Weekday};
use serde_json::json;
use uuid::Uuid;
use vidacor_documentos as doc;

fn dia_da_semana(d: Weekday) -> &'static str {
    match d {
        Weekday::Mon => "Segunda-feira",
        Weekday::Tue => "Terça-feira",
        Weekday::Wed => "Quarta-feira",
        Weekday::Thu => "Quinta-feira",
        Weekday::Fri => "Sexta-feira",
        Weekday::Sat => "Sábado",
        Weekday::Sun => "Domingo",
    }
}

fn emissao_impressa(codigo: &str, usuario: &Usuario) -> doc::EmissaoImpressa {
    let agora = Utc::now();
    doc::EmissaoImpressa {
        codigo: codigo.to_owned(),
        em: data_hora(agora),
        por: usuario.nome.clone(),
        momento: momento(agora),
    }
}

struct Registro<'a> {
    tipo: &'a str,
    evento: &'a str,
    id: Uuid,
    codigo: &'a str,
}

/// Gera o PDF e registra a emissão e o evento no lastro, em contexto de sistema da unidade (a
/// autorização já foi decidida pela leitura com o contexto de quem pediu).
async fn emitir<F>(
    estado: &Estado,
    usuario: &Usuario,
    r: Registro<'_>,
    codigo_emissao: String,
    f: F,
) -> Result<Response, ErroApi>
where
    F: FnOnce() -> Result<doc::Pdf, doc::ErroDocumento> + Send + 'static,
{
    let pdf = gerar(f).await?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx =
        banco::transacao(&estado.banco, &ContextoBanco::sistema(usuario.unidade_id)).await?;
    registrar_emissao(
        &mut tx,
        &codigo_emissao,
        &Emissao {
            tipo: r.tipo,
            registro_id: r.id,
            registro_codigo: r.codigo,
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
            dominio: "documentos",
            tipo: r.evento,
            registro_id: r.id,
            registro_codigo: Some(r.codigo),
            dados: json!({ "emissao": codigo_emissao, "sha256": pdf.sha256, "modelo": pdf.modelo, "por": usuario.id }),
            criado_por: Some(usuario.id),
            origem: "documentos",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(resposta_pdf(
        pdf,
        &format!("{}-{codigo_emissao}.pdf", r.codigo),
        &codigo_emissao,
    ))
}

/// Reserva o código da emissão no contexto de sistema (o contador é do núcleo).
async fn reservar_codigo(
    estado: &Estado,
    usuario: &Usuario,
    prefixo: &str,
) -> Result<String, ErroApi> {
    let mut tx =
        banco::transacao(&estado.banco, &ContextoBanco::sistema(usuario.unidade_id)).await?;
    let c = proximo_codigo(&mut tx, prefixo).await?;
    tx.commit().await?;
    Ok(c)
}

/// Comprovante do agendamento com o preparo vigente do exame (recepção e gerência).
#[utoipa::path(get, operation_id = "baixar_comprovante_do_agendamento", path = "/v1/agendamentos/{id}/comprovante", tag = "agenda",
    params(("id" = Uuid, Path)),
    responses((status = 200, description = "PDF/A-2b do comprovante; SHA-256 em X-Documento-Sha256", content_type = "application/pdf"),
              (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn comprovante(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::DocumentoEmitir)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let a = sqlx::query!(
        r#"SELECT a.codigo, a.inicio, a.fim, a.exame_tuss, a.exame_nome, a.situacao, p.nome AS paciente_nome,
                  p.nascimento, s.nome AS sala, g.operadora AS "operadora?"
             FROM agenda.agendamento a
             JOIN identidade.paciente p ON p.id = a.paciente_id
             JOIN agenda.sala s ON s.id = a.sala_id
             LEFT JOIN convenio.guia g ON g.id = a.guia_id
            WHERE a.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("agendamento-nao-encontrado", "Agendamento não encontrado."))?;
    if matches!(a.situacao.as_str(), "cancelado" | "faltou" | "expirada") {
        return Err(ErroApi::conflito(
            "agendamento-encerrado",
            "Este agendamento foi cancelado ou encerrado; não há comprovante a entregar. Se o paciente vai remarcar, faça um novo agendamento.",
        ));
    }
    let preparo = sqlx::query!(
        "SELECT numero, passos, chegar_antes_min, criado_em FROM agenda.preparo WHERE tuss = $1 AND vigente",
        a.exame_tuss
    )
    .fetch_optional(&mut *tx)
    .await?;
    let clinica = clinica(&mut tx, usuario.unidade_id).await?;
    tx.commit().await?;

    let codigo_emissao = reservar_codigo(&estado, &usuario, "CPV").await?;
    let inicio = a.inicio.with_timezone(&tempo::FUSO);
    let antecedencia = preparo.as_ref().map_or(15, |p| p.chegar_antes_min);
    let minutos = (a.fim - a.inicio).num_minutes();
    let c = doc::ComprovanteAgendamento {
        clinica,
        emissao: emissao_impressa(&codigo_emissao, &usuario),
        agendamento_codigo: a.codigo.clone(),
        paciente_nome: a.paciente_nome,
        paciente_nascimento: a.nascimento.format("%d/%m/%Y").to_string(),
        exame: a.exame_nome,
        dia_semana: dia_da_semana(inicio.weekday()).into(),
        data: inicio.format("%d/%m/%Y").to_string(),
        hora: inicio.format("%H:%M").to_string(),
        chegada: (inicio - Duration::minutes(i64::from(antecedencia)))
            .format("%H:%M")
            .to_string(),
        antecedencia_min: antecedencia,
        duracao: format!("Cerca de {minutos} min, {}", a.sala),
        convenio: a.operadora.map_or_else(
            || "Particular".to_owned(),
            |o| format!("{o}, guia autorizada"),
        ),
        preparo: preparo.map(|p| doc::PreparoImpresso {
            versao: p.numero,
            vigente_desde: data(p.criado_em),
            passos: p.passos,
        }),
    };
    emitir(
        &estado,
        &usuario,
        Registro {
            tipo: "comprovante_agendamento",
            evento: "comprovante_emitido",
            id,
            codigo: &a.codigo,
        },
        codigo_emissao,
        move || doc::comprovante_pdf(&c),
    )
    .await
}

/// Declaração de comparecimento do atendimento (recepção e gerência). Administrativa: não substitui
/// atestado médico. Só a chegada registrada no check-in é afirmada; atendimento cancelado não tem.
#[utoipa::path(get, operation_id = "baixar_declaracao_de_comparecimento", path = "/v1/atendimentos/{id}/declaracao", tag = "recepcao",
    params(("id" = Uuid, Path)),
    responses((status = 200, description = "PDF/A-2b da declaração; SHA-256 em X-Documento-Sha256", content_type = "application/pdf"),
              (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn declaracao(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::DocumentoEmitir)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let t = sqlx::query!(
        r#"SELECT t.codigo, t.criado_em, t.situacao, a.exame_nome, p.nome AS paciente_nome, p.nascimento, p.cpf
             FROM recepcao.atendimento t
             JOIN agenda.agendamento a ON a.id = t.agendamento_id
             JOIN identidade.paciente p ON p.id = t.paciente_id
            WHERE t.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("atendimento-nao-encontrado", "Atendimento não encontrado."))?;
    if t.situacao == "cancelado" {
        return Err(ErroApi::conflito(
            "atendimento-cancelado",
            "Este atendimento foi cancelado; a declaração não pode ser emitida.",
        ));
    }
    let clinica = clinica(&mut tx, usuario.unidade_id).await?;
    tx.commit().await?;

    let codigo_emissao = reservar_codigo(&estado, &usuario, "DCL").await?;
    let chegada = t.criado_em.with_timezone(&tempo::FUSO);
    let d = doc::DeclaracaoComparecimento {
        clinica,
        emissao: emissao_impressa(&codigo_emissao, &usuario),
        paciente_nome: t.paciente_nome,
        paciente_nascimento: t.nascimento.format("%d/%m/%Y").to_string(),
        paciente_cpf: t.cpf.as_deref().map(mascarar_cpf),
        dia: chegada.format("%d/%m/%Y").to_string(),
        chegada: chegada.format("%H:%M").to_string(),
        exame: t.exame_nome.to_lowercase(),
    };
    emitir(
        &estado,
        &usuario,
        Registro {
            tipo: "declaracao_comparecimento",
            evento: "declaracao_emitida",
            id,
            codigo: &t.codigo,
        },
        codigo_emissao,
        move || doc::declaracao_pdf(&d),
    )
    .await
}

/// Recibo do recebimento pago (caixa). Aguardando pagamento ou estornado não tem recibo.
#[utoipa::path(get, operation_id = "baixar_recibo", path = "/v1/caixa/recebimentos/{id}/recibo", tag = "caixa",
    params(("id" = Uuid, Path)),
    responses((status = 200, description = "PDF/A-2b do recibo; SHA-256 em X-Documento-Sha256", content_type = "application/pdf"),
              (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn recibo(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::CaixaOperar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = sqlx::query!(
        r#"SELECT r.codigo, r.situacao, r.forma, r.desconto_centavos, r.total_centavos, r.pago_em, p.nome AS paciente_nome, p.cpf,
                  u.nome AS "recebido_por?", n.numero AS "nfse?"
             FROM caixa.recebimento r
             JOIN identidade.paciente p ON p.id = r.paciente_id
             LEFT JOIN identidade.usuario u ON u.id = r.criado_por
             LEFT JOIN caixa.nota_fiscal n ON n.recebimento_id = r.id AND n.situacao = 'emitida'
            WHERE r.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("recebimento-nao-encontrado", "Recebimento não encontrado."))?;
    match r.situacao.as_str() {
        "pago" => {}
        "estornado" => {
            return Err(ErroApi::conflito(
                "recebimento-estornado",
                "Este recebimento foi estornado; não há recibo válido a entregar.",
            ))
        }
        _ => {
            return Err(ErroApi::conflito(
                "recebimento-nao-pago",
                "O pagamento ainda não foi confirmado. O recibo sai depois da confirmação.",
            ))
        }
    }
    let itens = sqlx::query!(
        "SELECT descricao, valor_centavos FROM caixa.recebimento_item WHERE recebimento_id = $1 ORDER BY descricao",
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    let clinica = clinica(&mut tx, usuario.unidade_id).await?;
    tx.commit().await?;

    let codigo_emissao = reservar_codigo(&estado, &usuario, "RCB").await?;
    let recibo = doc::Recibo {
        clinica,
        emissao: emissao_impressa(&codigo_emissao, &usuario),
        recebimento_codigo: r.codigo.clone(),
        pagador_nome: r.paciente_nome,
        pagador_cpf: r.cpf.as_deref().map(mascarar_cpf),
        itens: itens
            .into_iter()
            .map(|i| doc::ItemDoRecibo {
                descricao: i.descricao,
                valor_centavos: i.valor_centavos,
            })
            .collect(),
        desconto_centavos: r.desconto_centavos,
        total_centavos: r.total_centavos,
        pago_em: r.pago_em.map(data).unwrap_or_default(),
        forma: r.forma,
        nfse: r.nfse,
        recebido_por: r.recebido_por.unwrap_or_else(|| "Caixa".into()),
    };
    emitir(
        &estado,
        &usuario,
        Registro {
            tipo: "recibo",
            evento: "recibo_emitido",
            id,
            codigo: &r.codigo,
        },
        codigo_emissao,
        move || doc::recibo_pdf(&recibo),
    )
    .await
}
