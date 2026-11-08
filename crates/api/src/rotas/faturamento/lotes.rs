//! Lote TISS (docs/22 E5, docs/25 § 3.3): só guia sem pendência que impede entra; número atômico por
//! contrato (G2); arquivo TISS 4.03.00 imutável com hash do epílogo e SHA-256; envio pela porta
//! `Operadora` com chave de idempotência por lote, pelo outbox (G3, R7); demonstrativo de retorno.

use super::contrato::{EnvioPendente, Lote, LoteDetalhe, PedidoDemonstrativo, PedidoLote};
use super::{auditar_conta, consultas, contrato_por_id, montar_conta};
use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use sqlx::PgConnection;
use std::collections::BTreeMap;
use uuid::Uuid;
use vidacor_comum::novo_id;
use vidacor_dominio_faturamento::auditoria;
use vidacor_dominio_faturamento::tiss::MAX_GUIAS_POR_LOTE;
use vidacor_dominio_faturamento::{montar_lote, LoteTiss, VERSAO_TISS};

fn lote_nao_encontrado() -> ErroApi {
    ErroApi::nao_encontrado(
        "lote-nao-encontrado",
        "Este lote não existe no faturamento desta unidade. Nada foi alterado. Volte à lista de lotes.",
    )
}

fn repetida(status: u16, corpo: serde_json::Value) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
        Json(corpo),
    )
        .into_response()
}

async fn lotes(conexao: &mut PgConnection, id: Option<Uuid>) -> Result<Vec<Lote>, sqlx::Error> {
    let linhas = sqlx::query!(
        r#"SELECT l.id, l.codigo, l.numero, l.contrato_id, k.operadora, l.quantidade_guias, l.valor_total_centavos,
                  l.situacao, l.versao_tiss, l.hash_tiss, l.sha256, l.protocolo, l.criado_em, l.enviado_em,
                  o.tentativas AS "tentativas?", o.ultimo_erro, (o.id IS NOT NULL AND o.concluido_em IS NULL) AS "envio_pendente!",
                  d.valor_pago_centavos AS "pago?", d.valor_glosado_centavos AS "glosado?"
             FROM faturamento.lote l
             JOIN faturamento.contrato k ON k.id = l.contrato_id
             LEFT JOIN integracao.outbox o ON o.tipo = 'tiss.enviar_lote' AND o.referencia = l.id
             LEFT JOIN faturamento.demonstrativo d ON d.lote_id = l.id
            WHERE ($1::uuid IS NULL OR l.id = $1)
            ORDER BY l.criado_em DESC LIMIT 200"#,
        id
    )
    .fetch_all(conexao)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| Lote {
            id: l.id,
            codigo: l.codigo,
            numero: l.numero,
            contrato_id: l.contrato_id,
            operadora: l.operadora,
            quantidade_guias: l.quantidade_guias,
            valor_total_centavos: l.valor_total_centavos,
            situacao: l.situacao,
            versao_tiss: l.versao_tiss,
            hash_tiss: l.hash_tiss,
            sha256: l.sha256,
            protocolo: l.protocolo,
            criado_em: l.criado_em,
            enviado_em: l.enviado_em,
            envio_pendente: l.envio_pendente.then(|| EnvioPendente {
                tentativas: l.tentativas.unwrap_or(0),
                ultimo_erro: l.ultimo_erro,
            }),
            valor_pago_centavos: l.pago,
            valor_glosado_centavos: l.glosado,
        })
        .collect())
}

async fn detalhe(conexao: &mut PgConnection, id: Uuid) -> Result<LoteDetalhe, ErroApi> {
    let lote = lotes(conexao, Some(id))
        .await?
        .pop()
        .ok_or_else(lote_nao_encontrado)?;
    let contas = consultas::resumos(conexao, None, Some(id), None).await?;
    Ok(LoteDetalhe { lote, contas })
}

/// Lotes da unidade, do mais recente para o mais antigo.
#[utoipa::path(get, operation_id = "listar_lotes", path = "/v1/faturamento/lotes", tag = "faturamento",
    responses((status = 200, body = Vec<Lote>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<Lote>>, ErroApi> {
    usuario.exigir(Ato::FaturamentoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let l = lotes(&mut tx, None).await?;
    tx.commit().await?;
    Ok(Json(l))
}

/// Lote com as guias e a situação do envio.
#[utoipa::path(get, operation_id = "obter_lote", path = "/v1/faturamento/lotes/{id}", tag = "faturamento",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = LoteDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn obter(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<LoteDetalhe>, ErroApi> {
    usuario.exigir(Ato::FaturamentoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let d = detalhe(&mut tx, id).await?;
    tx.commit().await?;
    Ok(Json(d))
}

/// Arquivo XML do lote como foi gerado (para conferência). Nunca vai para cache.
#[utoipa::path(get, operation_id = "baixar_arquivo_do_lote", path = "/v1/faturamento/lotes/{id}/arquivo", tag = "faturamento",
    params(("id" = Uuid, Path)),
    responses((status = 200, description = "XML TISS em ISO-8859-1", content_type = "application/xml"),
              (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn arquivo(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::LoteGerar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let l = sqlx::query!(
        "SELECT codigo, arquivo FROM faturamento.lote WHERE id = $1",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(lote_nao_encontrado)?;
    tx.commit().await?;
    let mut r = (StatusCode::OK, l.arquivo).into_response();
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/xml; charset=ISO-8859-1"),
    );
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{}.xml\"", l.codigo)) {
        r.headers_mut().insert(header::CONTENT_DISPOSITION, v);
    }
    Ok(r)
}

/// Gera o lote de uma operadora com as guias prontas (só o faturista). Cada guia é conferida de novo
/// na hora; a que ganhou pendência fica de fora. Duas gerações simultâneas dão um lote só: o contrato
/// fica travado e cada conta entra em um lote só (G2).
#[utoipa::path(post, operation_id = "gerar_lote", path = "/v1/faturamento/lotes", tag = "faturamento",
    params(("Idempotency-Key" = Uuid, Header)), request_body = PedidoLote,
    responses((status = 201, body = LoteDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn gerar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<PedidoLote>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::LoteGerar)?;
    let rota = "POST /v1/faturamento/lotes";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    // Trava o contrato: gerações simultâneas da mesma operadora andam uma depois da outra.
    let travado = sqlx::query_scalar!(
        "SELECT id FROM faturamento.contrato WHERE id = $1 AND ativo FOR UPDATE",
        p.contrato_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let contrato = match travado {
        Some(id) => contrato_por_id(&mut tx, id).await?,
        None => None,
    }
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "contrato-nao-encontrado",
            "Esta operadora não tem contrato ativo nesta unidade. Nada foi gerado.",
        )
    })?;
    let candidatas = sqlx::query_scalar!(
        "SELECT id FROM faturamento.conta WHERE contrato_id = $1 AND situacao = 'pronta' AND lote_id IS NULL
          ORDER BY data_execucao, codigo LIMIT $2 FOR UPDATE",
        contrato.id,
        i64::try_from(MAX_GUIAS_POR_LOTE).unwrap_or(100)
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut guias = Vec::new();
    let mut contas = Vec::new();
    for c in candidatas {
        // Conferência na hora: o laudo, a senha e a tabela podem ter mudado desde a última.
        if !auditoria::pronta(&auditar_conta(&mut tx, c, usuario.id, &correlacao).await?) {
            continue;
        }
        if let Some(m) = montar_conta(&mut tx, c, false).await? {
            contas.push((c, m.codigo));
            guias.push(m.guia);
        }
    }
    if guias.is_empty() {
        return Err(ErroApi::conflito(
            "nenhuma-guia-pronta",
            "Nenhuma guia desta operadora está pronta para o lote. Nada foi gerado. Confira as pendências na auditoria.",
        ));
    }
    let numero = sqlx::query_scalar!(
        "UPDATE faturamento.contrato SET ultimo_lote = ultimo_lote + 1, versao = versao + 1 WHERE id = $1 RETURNING ultimo_lote",
        contrato.id
    )
    .fetch_one(&mut *tx)
    .await?;
    let gerado_em = tempo::agora()
        .with_timezone(&chrono_tz::America::Sao_Paulo)
        .naive_local();
    let a = montar_lote(&LoteTiss {
        numero,
        registro_ans: &contrato.registro_ans,
        codigo_prestador: &contrato.codigo_prestador,
        nome_prestador: &contrato.nome_prestador,
        gerado_em,
        guias: &guias,
    })
    .map_err(ErroApi::interno)?;
    let lote = novo_id();
    let codigo = crate::rotas::caixa::proximo(&mut tx, "LTI").await?;
    sqlx::query!(
        "INSERT INTO faturamento.lote (id, codigo, contrato_id, numero, versao_tiss, quantidade_guias, valor_total_centavos,
                arquivo, hash_tiss, sha256, chave_envio, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'faturamento', $13, $14)",
        lote, codigo.como_str(), contrato.id, numero, VERSAO_TISS, i32::try_from(contas.len()).unwrap_or(i32::MAX),
        a.valor_total.0, a.xml, a.hash_tiss, a.sha256, novo_id(), usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    for (i, (c, _)) in contas.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO faturamento.lote_item (lote_id, conta_id, sequencial, unidade_id) VALUES ($1, $2, $3, $4)",
            lote,
            c,
            i32::try_from(i + 1).unwrap_or(i32::MAX),
            usuario.unidade_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "UPDATE faturamento.conta SET situacao = 'em_lote', lote_id = $2, versao = versao + 1 WHERE id = $1",
            c,
            lote
        )
        .execute(&mut *tx)
        .await?;
    }
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "faturamento",
            tipo: "lote_gerado",
            registro_id: lote,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({
                "numero": numero, "contrato": contrato.id, "versao_tiss": VERSAO_TISS,
                "hash_tiss": a.hash_tiss, "sha256": a.sha256, "valor_total_centavos": a.valor_total.0,
                "contas": contas.iter().map(|(_, c)| c).collect::<Vec<_>>(),
            }),
            criado_por: Some(usuario.id),
            origem: "faturamento",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let d = detalhe(&mut tx, lote).await?;
    let corpo = serde_json::to_value(&d).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Pede o envio do lote à operadora (só o faturista). O envio sai pelo outbox com a chave do lote;
/// pedir de novo não cria outro envio.
#[utoipa::path(post, operation_id = "enviar_lote", path = "/v1/faturamento/lotes/{id}/envio", tag = "faturamento",
    params(("id" = Uuid, Path)),
    responses((status = 202, body = LoteDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn enviar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<LoteDetalhe>), ErroApi> {
    usuario.exigir(Ato::LoteGerar)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let l = sqlx::query!(
        "SELECT codigo, situacao, chave_envio FROM faturamento.lote WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(lote_nao_encontrado)?;
    if l.situacao == "gerado" {
        sqlx::query!(
            "UPDATE faturamento.lote SET situacao = 'aguardando_envio', versao = versao + 1 WHERE id = $1",
            id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
             VALUES ($1, 'tiss.enviar_lote', $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING",
            novo_id(), l.chave_envio, json!({ "lote_id": id }), usuario.unidade_id, correlacao, id
        )
        .execute(&mut *tx)
        .await?;
        lastro::registrar(
            &mut tx,
            NovoEvento {
                dominio: "faturamento",
                tipo: "lote_envio_pedido",
                registro_id: id,
                registro_codigo: Some(&l.codigo),
                dados: json!({ "chave_envio": l.chave_envio }),
                criado_por: Some(usuario.id),
                origem: "faturamento",
                unidade_id: usuario.unidade_id,
                correlacao: &correlacao,
            },
        )
        .await?;
    }
    let d = detalhe(&mut tx, id).await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok((StatusCode::ACCEPTED, Json(d)))
}

/// Envio do lote pelo trabalhador do outbox. Sem webservice configurado, falha e o item continua
/// pendente, visível no lote (R7). Repetir com a mesma chave não gera outro protocolo (G3).
///
/// # Errors
/// Operadora não configurada ou indisponível, ou falha de banco.
pub async fn enviar_pelo_outbox(
    estado: &Estado,
    unidade: Uuid,
    lote: Uuid,
    chave: Uuid,
) -> anyhow::Result<()> {
    let Some(operadora) = estado.operadora.clone() else {
        anyhow::bail!("webservice TISS da operadora não configurado");
    };
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade)).await?;
    let Some(l) = sqlx::query!(
        "SELECT l.codigo, l.situacao, l.arquivo, k.registro_ans FROM faturamento.lote l
           JOIN faturamento.contrato k ON k.id = l.contrato_id WHERE l.id = $1",
        lote
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        anyhow::bail!("lote do outbox não encontrado");
    };
    tx.commit().await?;
    if l.situacao != "aguardando_envio" {
        return Ok(());
    }
    let protocolo = operadora
        .enviar_lote(chave, &l.registro_ans, &l.arquivo)
        .await?;
    anyhow::ensure!(
        !protocolo.numero.is_empty()
            && protocolo.numero.len() <= 30
            && protocolo
                .numero
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "protocolo da operadora fora do formato"
    );
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade)).await?;
    let mudou = sqlx::query!(
        "UPDATE faturamento.lote SET situacao = 'enviado', protocolo = $2, enviado_em = now(), versao = versao + 1
          WHERE id = $1 AND situacao = 'aguardando_envio'",
        lote,
        protocolo.numero
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if mudou == 1 {
        sqlx::query!(
            "UPDATE faturamento.conta SET situacao = 'enviada', versao = versao + 1 WHERE lote_id = $1",
            lote
        )
        .execute(&mut *tx)
        .await?;
        lastro::registrar(
            &mut tx,
            NovoEvento {
                dominio: "faturamento",
                tipo: "lote_enviado",
                registro_id: lote,
                registro_codigo: Some(&l.codigo),
                dados: json!({ "protocolo": protocolo.numero }),
                criado_por: None,
                origem: "integracao",
                unidade_id: unidade,
                correlacao: &format!("tiss:{chave}"),
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn invalido(codigo: &str, detalhe: String) -> ErroApi {
    ErroApi::invalido(codigo, &format!("{detalhe} Nada foi gravado."))
}

/// Registra o demonstrativo de retorno do lote, conta a conta (só o faturista): valor pago e glosas
/// com código e motivo. Pago mais glosado fecha com o apresentado em cada guia (G1: o servidor
/// confere a conta).
#[utoipa::path(post, operation_id = "registrar_demonstrativo", path = "/v1/faturamento/lotes/{id}/demonstrativo", tag = "faturamento",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = PedidoDemonstrativo,
    responses((status = 201, body = LoteDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn demonstrativo(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoDemonstrativo>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::DemonstrativoRegistrar)?;
    if p.recebido_em > tempo::hoje_local() {
        return Err(invalido(
            "data-futura",
            "A data de recebimento do demonstrativo está no futuro.".into(),
        ));
    }
    let rota = format!("POST /v1/faturamento/lotes/{id}/demonstrativo");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let l = sqlx::query!(
        "SELECT l.codigo, l.situacao, l.valor_total_centavos, k.prazo_recurso_dias FROM faturamento.lote l
           JOIN faturamento.contrato k ON k.id = l.contrato_id WHERE l.id = $1 FOR UPDATE OF l",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(lote_nao_encontrado)?;
    match l.situacao.as_str() {
        "enviado" => {}
        "demonstrativo_recebido" => {
            return Err(ErroApi::conflito(
                "demonstrativo-ja-registrado",
                "O demonstrativo deste lote já foi registrado. Nada foi alterado.",
            ))
        }
        _ => {
            return Err(ErroApi::conflito(
                "lote-nao-enviado",
                "Este lote ainda não foi recebido pela operadora. Nada foi gravado. Registre o demonstrativo depois do protocolo.",
            ))
        }
    }
    let contas: BTreeMap<Uuid, (String, i64)> = sqlx::query!(
        "SELECT id, codigo, valor_total_centavos FROM faturamento.conta WHERE lote_id = $1",
        id
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|c| (c.id, (c.codigo, c.valor_total_centavos)))
    .collect();
    let mut vistas = std::collections::BTreeSet::new();
    for r in &p.contas {
        let Some((codigo, total)) = contas.get(&r.conta_id) else {
            return Err(invalido(
                "conta-fora-do-lote",
                "Uma das guias do demonstrativo não é deste lote.".into(),
            ));
        };
        if !vistas.insert(r.conta_id) {
            return Err(invalido(
                "conta-repetida",
                format!("A guia {codigo} aparece duas vezes no demonstrativo."),
            ));
        }
        if r.glosas.len() > 20 || r.valor_pago_centavos < 0 {
            return Err(invalido(
                "retorno-invalido",
                format!("O retorno da guia {codigo} tem valor negativo ou mais de 20 glosas."),
            ));
        }
        for g in &r.glosas {
            let motivo = g.motivo.trim().chars().count();
            if g.codigo_glosa.len() != 4
                || !g.codigo_glosa.chars().all(|c| c.is_ascii_digit())
                || !(3..=500).contains(&motivo)
                || g.valor_centavos <= 0
            {
                return Err(invalido("glosa-invalida", format!("Uma glosa da guia {codigo} está incompleta: código TISS com 4 dígitos, motivo e valor acima de zero.")));
            }
        }
        let glosado: i64 = r.glosas.iter().map(|g| g.valor_centavos).sum();
        if r.valor_pago_centavos + glosado != *total {
            return Err(invalido(
                "valores-nao-fecham",
                format!(
                    "Na guia {codigo}, pago mais glosado não fecha com o apresentado ({}).",
                    vidacor_dominio_faturamento::Centavos(*total).reais()
                ),
            ));
        }
    }
    if vistas.len() != contas.len() {
        return Err(invalido(
            "demonstrativo-incompleto",
            format!(
                "O demonstrativo traz {} de {} guias do lote.",
                vistas.len(),
                contas.len()
            ),
        ));
    }
    let pago: i64 = p.contas.iter().map(|r| r.valor_pago_centavos).sum();
    let demonstrativo = novo_id();
    let codigo = crate::rotas::caixa::proximo(&mut tx, "DMT").await?;
    sqlx::query!(
        "INSERT INTO faturamento.demonstrativo (id, codigo, lote_id, recebido_em, valor_apresentado_centavos, valor_pago_centavos,
                valor_glosado_centavos, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'faturamento', $9, $10)",
        demonstrativo, codigo.como_str(), id, p.recebido_em, l.valor_total_centavos, pago, l.valor_total_centavos - pago,
        usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    let prazo = p.recebido_em + chrono::Duration::days(i64::from(l.prazo_recurso_dias));
    for r in &p.contas {
        for g in &r.glosas {
            let gcodigo = crate::rotas::caixa::proximo(&mut tx, "GLO").await?;
            sqlx::query!(
                "INSERT INTO faturamento.glosa (id, codigo, conta_id, demonstrativo_id, codigo_glosa, motivo, valor_centavos,
                        prazo_recurso, criado_por, origem, unidade_id, correlacao)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'faturamento', $10, $11)",
                novo_id(), gcodigo.como_str(), r.conta_id, demonstrativo, g.codigo_glosa, g.motivo.trim(), g.valor_centavos,
                prazo, usuario.id, usuario.unidade_id, correlacao
            )
            .execute(&mut *tx)
            .await?;
        }
        let situacao = match (r.valor_pago_centavos, r.glosas.is_empty()) {
            (_, true) => "paga",
            (0, false) => "glosada",
            _ => "paga_com_glosa",
        };
        sqlx::query!(
            "UPDATE faturamento.conta SET situacao = $2, valor_pago_centavos = $3, versao = versao + 1 WHERE id = $1",
            r.conta_id,
            situacao,
            r.valor_pago_centavos
        )
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query!(
        "UPDATE faturamento.lote SET situacao = 'demonstrativo_recebido', versao = versao + 1 WHERE id = $1",
        id
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "faturamento",
            tipo: "demonstrativo_registrado",
            registro_id: demonstrativo,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "lote": id, "lote_codigo": l.codigo, "pago_centavos": pago,
                           "glosado_centavos": l.valor_total_centavos - pago,
                           "glosas": p.contas.iter().map(|r| r.glosas.len()).sum::<usize>() }),
            criado_por: Some(usuario.id),
            origem: "faturamento",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let d = detalhe(&mut tx, id).await?;
    let corpo = serde_json::to_value(&d).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}
