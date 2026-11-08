//! Pedido feito no portal da operadora (operadoras sem webservice, docs/25 MVP): a recepção ou o
//! faturista registra o protocolo e depois a resposta copiada do portal, que fica gravada como
//! recebida (JSON canônico, com hash) e preenche a guia como a resposta do webservice.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::autorizacoes::{como_sistema, detalhe, repetida, travar};
use crate::rotas::autorizacoes_contrato::{AutorizacaoDetalhe, PedidoProtocolo, RespostaDoPortal};
use crate::rotas::autorizacoes_resposta::{gravar_resposta, ABERTAS};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use uuid::Uuid;
use vidacor_dominio_faturamento::autorizacao::{
    ProcedimentoAutorizado, RespostaAutorizacao, StatusAutorizacao,
};

/// Pedido feito no portal da operadora: registra o protocolo (canal portal).
#[utoipa::path(post, operation_id = "registrar_protocolo_autorizacao", path = "/v1/autorizacoes/{id}/protocolo", tag = "autorizacoes",
    params(("id" = Uuid, Path)), request_body = PedidoProtocolo,
    responses((status = 200, body = AutorizacaoDetalhe), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn protocolo(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoProtocolo>,
) -> Result<Json<AutorizacaoDetalhe>, ErroApi> {
    usuario.exigir(Ato::AutorizacaoPedir)?;
    let prot = p.protocolo.trim();
    if prot.is_empty()
        || prot.len() > 40
        || !prot
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "./-".contains(c))
    {
        return Err(ErroApi::invalido("protocolo-invalido", "Copie o protocolo como aparece no portal da operadora: letras, números, ponto, barra e hífen, até 40. Nada foi gravado."));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &como_sistema(&usuario)).await?;
    let (codigo, canal, situacao) = travar(&mut tx, id).await?;
    if canal != "portal" || situacao != "aguardando_portal" {
        return Err(ErroApi::conflito(
            "protocolo-fora-de-hora",
            "Este pedido não espera protocolo do portal. Nada foi alterado.",
        ));
    }
    sqlx::query!(
        "UPDATE convenio.autorizacao SET situacao = 'enviada', protocolo = $2, enviada_em = now(), versao = versao + 1 WHERE id = $1",
        id, prot
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "convenio",
            tipo: "autorizacao_enviada_pelo_portal",
            registro_id: id,
            registro_codigo: Some(&codigo),
            dados: json!({ "protocolo": prot }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let d = detalhe(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(d))
}

fn ler_portal(r: &RespostaDoPortal, codigo: &str) -> Result<RespostaAutorizacao, ErroApi> {
    let invalido =
        |m: &str| ErroApi::invalido("resposta-invalida", &format!("{m} Nada foi gravado."));
    let status = match r.situacao.as_str() {
        "autorizada" => StatusAutorizacao::Autorizada,
        "parcial" => StatusAutorizacao::Parcial,
        "negada" => StatusAutorizacao::Negada,
        "em_analise" => StatusAutorizacao::EmAnalise,
        "pendente_documento" => StatusAutorizacao::PendenteDocumento,
        _ => return Err(invalido("Escolha o que o portal respondeu.")),
    };
    let texto_ok = |v: &Option<String>| {
        v.as_deref().is_some_and(|s| {
            (1..=20).contains(&s.trim().len())
                && s.trim()
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
    };
    let mut status = status;
    if matches!(
        status,
        StatusAutorizacao::Autorizada | StatusAutorizacao::Parcial
    ) {
        if !texto_ok(&r.senha) || r.validade.is_none() {
            return Err(invalido(
                "Autorizada pede a senha (até 20 letras e números) e a validade, como no portal.",
            ));
        }
        if r.numero_guia_operadora.is_some() && !texto_ok(&r.numero_guia_operadora) {
            return Err(invalido(
                "O número da guia vai com até 20 letras, números e hífen.",
            ));
        }
        if r.validade.is_some_and(|v| v < tempo::hoje_local()) {
            return Err(invalido("A validade da senha já passou."));
        }
        if !r.procedimentos.iter().any(|p| p.autorizado > 0) {
            return Err(invalido(
                "Informe a quantidade autorizada de cada procedimento.",
            ));
        }
    }
    if status == StatusAutorizacao::Negada && r.motivo.as_deref().is_none_or(|m| m.trim().len() < 3)
    {
        return Err(invalido(
            "Copie o motivo da negativa que aparece no portal.",
        ));
    }
    let procedimentos: Vec<ProcedimentoAutorizado> = r
        .procedimentos
        .iter()
        .map(|p| ProcedimentoAutorizado {
            tuss: p.tuss.clone(),
            solicitado: 1,
            autorizado: u16::try_from(p.autorizado.clamp(0, 99)).unwrap_or(0),
        })
        .collect();
    if status == StatusAutorizacao::Autorizada
        && procedimentos.iter().any(|p| p.autorizado < p.solicitado)
    {
        status = StatusAutorizacao::Parcial;
    }
    Ok(RespostaAutorizacao {
        numero_guia_prestador: Some(codigo.to_owned()),
        status,
        numero_guia_operadora: r
            .numero_guia_operadora
            .as_deref()
            .map(|s| s.trim().to_owned()),
        senha: r.senha.as_deref().map(|s| s.trim().to_owned()),
        data_autorizacao: r.data_autorizacao,
        validade: r.validade,
        procedimentos,
        motivos: Vec::new(),
        observacao: r
            .motivo
            .as_deref()
            .map(|m| m.trim().chars().take(500).collect()),
        hash_confere: true,
    })
}

/// Registra a resposta copiada do portal da operadora (canal portal): fica gravada como recebida e
/// preenche a guia como a do webservice.
#[utoipa::path(post, operation_id = "registrar_resposta_do_portal", path = "/v1/autorizacoes/{id}/resposta", tag = "autorizacoes",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = RespostaDoPortal,
    responses((status = 200, body = AutorizacaoDetalhe), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn resposta_do_portal(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(r): Corpo<RespostaDoPortal>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::AutorizacaoPedir)?;
    let rota = format!("POST /v1/autorizacoes/{id}/resposta");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &como_sistema(&usuario)).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let (codigo, canal, situacao) = travar(&mut tx, id).await?;
    if canal != "portal" {
        return Err(ErroApi::conflito(
            "resposta-pelo-webservice",
            "Este pedido foi pelo webservice: a resposta chega sozinha. Nada foi gravado.",
        ));
    }
    if situacao == "aguardando_portal" {
        return Err(ErroApi::conflito(
            "sem-protocolo",
            "Registre antes o protocolo do pedido feito no portal. Nada foi gravado.",
        ));
    }
    if !ABERTAS.contains(&situacao.as_str()) {
        return Err(ErroApi::conflito(
            "autorizacao-respondida",
            "Este pedido já tem resposta final. Nada foi gravado.",
        ));
    }
    let lida = ler_portal(&r, &codigo)?;
    let tuss_do_pedido: Vec<String> = sqlx::query_scalar!(
        "SELECT tuss FROM convenio.autorizacao_procedimento WHERE autorizacao_id = $1",
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    if lida
        .procedimentos
        .iter()
        .any(|p| !tuss_do_pedido.contains(&p.tuss))
    {
        return Err(ErroApi::invalido(
            "procedimento-fora-do-pedido",
            "Há procedimento na resposta que não está no pedido. Nada foi gravado.",
        ));
    }
    // A resposta digitada é guardada como veio (JSON canônico), com hash, como a do webservice.
    let bruta = serde_json::to_vec(&r).map_err(ErroApi::interno)?;
    gravar_resposta(
        &mut tx,
        id,
        &bruta,
        Ok(lida),
        "portal",
        Some(usuario.id),
        &correlacao,
    )
    .await?;
    let d = detalhe(&mut tx, id).await?;
    let corpo = serde_json::to_value(&d).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 200, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::OK, Json(corpo)).into_response())
}
