//! "Conferir guias" (Faturamento.dc.html, Auditoria-Faturamento.dc.html): apura a conta de cada
//! atendimento com guia consumida e exame feito, confere todas as contas fora de lote pelas regras da
//! operadora e corrige a conta com versão e motivo (nada é apagado).

use super::contrato::{ContaDetalhe, PedidoCorrecao, ResultadoConferencia};
use super::{
    auditar_conta, consultas, conta_nao_encontrada, contrato_da_operadora, precos_vigentes,
    SITUACOES_ABERTAS,
};
use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;
use vidacor_comum::novo_id;
use vidacor_dominio_faturamento::auditoria;
use vidacor_dominio_faturamento::guia::cid_valido;

/// Contas apuradas e conferidas por pedido (limite de disponibilidade, SEC-DIS).
const LIMITE_POR_CONFERENCIA: i64 = 500;

fn repetida(status: u16, corpo: serde_json::Value) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
        Json(corpo),
    )
        .into_response()
}

/// Contexto de sistema da unidade com o usuário que pediu (o faturista não lê a agenda nem o
/// atendimento: a apuração lê em nome do núcleo, depois de conferido o ato).
pub(crate) fn como_sistema(usuario: &Usuario) -> ContextoBanco {
    ContextoBanco {
        usuario_id: Some(usuario.id),
        unidade_id: usuario.unidade_id,
        perfis: vec!["sistema".into()],
    }
}

/// Cria a conta de cada atendimento elegível que ainda não tem conta. Repetir não duplica
/// (atendimento e guia únicos na conta, G2).
async fn apurar(
    conexao: &mut PgConnection,
    usuario: &Usuario,
    correlacao: &str,
) -> Result<i64, ErroApi> {
    let elegiveis = sqlx::query!(
        r#"SELECT a.id AS atendimento_id, a.paciente_id, p.nome, p.nascimento,
                  g.id AS guia_id, g.operadora, g.numero, g.senha, g.validade, g.carteirinha, g.solicitante,
                  (g.criado_em AT TIME ZONE 'America/Sao_Paulo')::date AS "data_autorizacao!",
                  (ag.inicio AT TIME ZONE 'America/Sao_Paulo')::date AS "data_execucao!",
                  coalesce((SELECT array_agg(ap.tuss ORDER BY ap.tuss) FROM agenda.agendamento_procedimento ap
                             WHERE ap.agendamento_id = ag.id), ARRAY[ag.exame_tuss]) AS "tuss!"
             FROM recepcao.atendimento a
             JOIN agenda.agendamento ag ON ag.id = a.agendamento_id
             JOIN convenio.guia g ON g.agendamento_id = ag.id AND g.situacao = 'consumida'
             JOIN identidade.paciente p ON p.id = a.paciente_id
            WHERE a.unidade_id = $1 AND a.situacao <> 'cancelado'
              AND (a.situacao = 'exame_concluido'
                   OR EXISTS (SELECT 1 FROM laudos.laudo l WHERE l.atendimento_id = a.id))
              AND NOT EXISTS (SELECT 1 FROM faturamento.conta c WHERE c.atendimento_id = a.id)
            ORDER BY ag.inicio
            LIMIT $2"#,
        usuario.unidade_id,
        LIMITE_POR_CONFERENCIA
    )
    .fetch_all(&mut *conexao)
    .await?;
    let mut novas = 0;
    for e in elegiveis {
        let contrato = contrato_da_operadora(conexao, usuario.unidade_id, &e.operadora).await?;
        let (tabela, precos) = match &contrato {
            Some(k) => precos_vigentes(conexao, k.id, e.data_execucao).await?,
            None => (None, Default::default()),
        };
        let id = novo_id();
        let codigo = crate::rotas::caixa::proximo(conexao, "FAT").await?;
        let criada = sqlx::query_scalar!(
            "INSERT INTO faturamento.conta (id, codigo, atendimento_id, guia_id, contrato_id, operadora, paciente_id,
                    paciente_nome, paciente_nascimento, data_execucao, numero_guia_operadora, senha, data_autorizacao,
                    validade_senha, carteirinha, solicitante_nome, tabela_id, criado_por, origem, unidade_id, correlacao)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, 'faturamento', $19, $20)
             ON CONFLICT DO NOTHING RETURNING id",
            id, codigo.como_str(), e.atendimento_id, e.guia_id, contrato.as_ref().map(|k| k.id), e.operadora,
            e.paciente_id, e.nome, e.nascimento, e.data_execucao, e.numero, e.senha, e.data_autorizacao,
            e.validade, e.carteirinha, e.solicitante, tabela, usuario.id, usuario.unidade_id, correlacao
        )
        .fetch_optional(&mut *conexao)
        .await?;
        if criada.is_none() {
            continue;
        }
        let mut total: i64 = 0;
        for (i, tuss) in e.tuss.iter().enumerate() {
            let descricao =
                sqlx::query_scalar!("SELECT nome FROM agenda.procedimento WHERE tuss = $1", tuss)
                    .fetch_optional(&mut *conexao)
                    .await?
                    .unwrap_or_else(|| format!("Procedimento {tuss}"));
            let preco = precos.get(tuss).map(|c| c.0);
            total += preco.unwrap_or(0);
            sqlx::query!(
                "INSERT INTO faturamento.conta_item (conta_id, sequencial, tuss, descricao, quantidade, valor_unitario_centavos, unidade_id)
                 VALUES ($1, $2, $3, $4, 1, $5, $6)",
                id, i32::try_from(i + 1).unwrap_or(i32::MAX), tuss, descricao, preco, usuario.unidade_id
            )
            .execute(&mut *conexao)
            .await?;
        }
        sqlx::query!(
            "UPDATE faturamento.conta SET valor_total_centavos = $2 WHERE id = $1",
            id,
            total
        )
        .execute(&mut *conexao)
        .await?;
        lastro::registrar(
            conexao,
            NovoEvento {
                dominio: "faturamento",
                tipo: "conta_apurada",
                registro_id: id,
                registro_codigo: Some(codigo.como_str()),
                dados: json!({ "atendimento": e.atendimento_id, "guia": e.guia_id, "tabela": tabela,
                               "procedimentos": e.tuss, "valor_total_centavos": total }),
                criado_por: Some(usuario.id),
                origem: "faturamento",
                unidade_id: usuario.unidade_id,
                correlacao,
            },
        )
        .await?;
        novas += 1;
    }
    Ok(novas)
}

/// Conferir guias: apura as contas novas e confere todas as contas fora de lote pelas regras de
/// cada operadora (só o faturista).
#[utoipa::path(post, operation_id = "conferir_guias", path = "/v1/faturamento/conferencias", tag = "faturamento",
    params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 200, body = ResultadoConferencia), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn conferir(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::FaturamentoAuditar)?;
    let rota = "POST /v1/faturamento/conferencias";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &como_sistema(&usuario)).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let novas = apurar(&mut tx, &usuario, &correlacao).await?;
    let abertas = sqlx::query_scalar!(
        "SELECT id FROM faturamento.conta WHERE unidade_id = $1 AND situacao = ANY($2)
          ORDER BY data_execucao, codigo LIMIT $3",
        usuario.unidade_id,
        &SITUACOES_ABERTAS.map(String::from)[..],
        LIMITE_POR_CONFERENCIA
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut prontas = 0;
    for conta in &abertas {
        let p = auditar_conta(&mut tx, *conta, usuario.id, &correlacao).await?;
        if auditoria::pronta(&p) {
            prontas += 1;
        }
    }
    let conferidas = i64::try_from(abertas.len()).unwrap_or(i64::MAX);
    let r = ResultadoConferencia {
        novas,
        conferidas,
        prontas,
        com_pendencia: conferidas - prontas,
    };
    let corpo = serde_json::to_value(&r).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, rota, chave, 200, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::OK, Json(corpo)).into_response())
}

fn so_digitos(v: &str, n: usize) -> bool {
    v.len() == n && v.chars().all(|c| c.is_ascii_digit())
}

/// Confere o pedido de correção antes de abrir a transação (erro de preenchimento não toca no banco).
fn validar(p: &PedidoCorrecao) -> Result<(), ErroApi> {
    let motivo = p.motivo.trim().chars().count();
    if !(5..=300).contains(&motivo) {
        return Err(ErroApi::invalido(
            "motivo-invalido",
            "Informe o motivo da correção (de 5 a 300 caracteres). Nada foi gravado.",
        ));
    }
    if let Some(s) = &p.solicitante {
        let numero_ok = (1..=15).contains(&s.numero.trim().len())
            && s.numero
                .trim()
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "./-".contains(c));
        if !(so_digitos(&s.conselho, 2)
            && so_digitos(&s.uf, 2)
            && so_digitos(&s.cbos, 6)
            && numero_ok)
        {
            return Err(ErroApi::invalido(
                "solicitante-invalido",
                "Os dados do solicitante não estão no padrão TISS: conselho com 2 dígitos (06 = CRM), número do conselho, UF com 2 dígitos (33 = RJ) e CBO com 6 dígitos. Nada foi gravado.",
            ));
        }
    }
    if let Some(cid) = &p.cid {
        if !cid_valido(cid) {
            return Err(ErroApi::invalido(
                "cid-invalido",
                "O CID-10 vai sem ponto: letra e 2 ou 3 dígitos (ex.: R51, I650). Nada foi gravado.",
            ));
        }
    }
    if p.solicitante.is_none()
        && p.cid.is_none()
        && p.assinatura_beneficiario.is_none()
        && !p.reaplicar_tabela
    {
        return Err(ErroApi::invalido(
            "correcao-vazia",
            "A correção não muda nada na guia. Nada foi gravado. Escolha o que corrigir.",
        ));
    }
    Ok(())
}

/// Corrige a conta antes do lote (só o faturista): a versão anterior fica no histórico com o motivo,
/// a conta ganha versão nova e é conferida de novo. Conta em lote não muda (corrige-se por recurso).
#[utoipa::path(post, operation_id = "corrigir_conta", path = "/v1/faturamento/contas/{id}/correcao", tag = "faturamento",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = PedidoCorrecao,
    responses((status = 200, body = ContaDetalhe), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn corrigir(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoCorrecao>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::FaturamentoAuditar)?;
    validar(&p)?;
    let rota = format!("POST /v1/faturamento/contas/{id}/correcao");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let atual = sqlx::query!(
        r#"SELECT c.codigo, c.versao, c.lote_id, c.contrato_id, c.data_execucao,
                  to_jsonb(c) || jsonb_build_object('itens',
                     (SELECT coalesce(jsonb_agg(to_jsonb(i) ORDER BY i.sequencial), '[]'::jsonb)
                        FROM faturamento.conta_item i WHERE i.conta_id = c.id)) AS "anterior!"
             FROM faturamento.conta c WHERE c.id = $1 FOR UPDATE"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(conta_nao_encontrada)?;
    if atual.lote_id.is_some() {
        return Err(ErroApi::conflito(
            "conta-em-lote",
            "Esta guia já está em um lote e não muda mais. Nada foi alterado. Se a operadora glosar, corrija pelo recurso de glosa.",
        ));
    }
    sqlx::query!(
        "INSERT INTO faturamento.conta_historico (id, conta_id, versao, motivo, anterior, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'faturamento', $7, $8)",
        novo_id(), id, atual.versao, p.motivo.trim(), atual.anterior, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    let s = p.solicitante.as_ref();
    sqlx::query!(
        "UPDATE faturamento.conta SET
            solicitante_conselho = coalesce($2, solicitante_conselho), solicitante_numero = coalesce($3, solicitante_numero),
            solicitante_uf = coalesce($4, solicitante_uf), solicitante_cbos = coalesce($5, solicitante_cbos),
            cid = coalesce($6, cid), assinatura_beneficiario = coalesce($7, assinatura_beneficiario), versao = versao + 1
          WHERE id = $1",
        id,
        s.map(|x| x.conselho.as_str()),
        s.map(|x| x.numero.trim()),
        s.map(|x| x.uf.as_str()),
        s.map(|x| x.cbos.as_str()),
        p.cid.as_deref(),
        p.assinatura_beneficiario
    )
    .execute(&mut *tx)
    .await?;
    let mut campos: Vec<&str> = Vec::new();
    if s.is_some() {
        campos.push("solicitante");
    }
    if p.cid.is_some() {
        campos.push("cid");
    }
    if p.assinatura_beneficiario.is_some() {
        campos.push("assinatura_beneficiario");
    }
    if p.reaplicar_tabela {
        campos.push("tabela");
        let contrato = match atual.contrato_id {
            Some(c) => Some(c),
            None => sqlx::query_scalar!(
                "SELECT k.id FROM faturamento.contrato k JOIN faturamento.conta c ON c.id = $1
                  WHERE k.unidade_id = c.unidade_id AND lower(trim(k.operadora)) = lower(trim(c.operadora)) AND k.ativo",
                id
            )
            .fetch_optional(&mut *tx)
            .await?,
        };
        let (tabela, precos) = match contrato {
            Some(k) => precos_vigentes(&mut tx, k, atual.data_execucao).await?,
            None => (None, Default::default()),
        };
        let itens = sqlx::query!(
            "SELECT sequencial, tuss, quantidade FROM faturamento.conta_item WHERE conta_id = $1",
            id
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut total = 0;
        for i in itens {
            let preco = precos.get(&i.tuss).map(|c| c.0);
            total += preco.unwrap_or(0) * i64::from(i.quantidade);
            sqlx::query!(
                "UPDATE faturamento.conta_item SET valor_unitario_centavos = $3 WHERE conta_id = $1 AND sequencial = $2",
                id,
                i.sequencial,
                preco
            )
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query!(
            "UPDATE faturamento.conta SET tabela_id = $2, contrato_id = coalesce(contrato_id, $3), valor_total_centavos = $4 WHERE id = $1",
            id,
            tabela,
            contrato,
            total
        )
        .execute(&mut *tx)
        .await?;
    }
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "faturamento",
            tipo: "conta_corrigida",
            registro_id: id,
            registro_codigo: Some(&atual.codigo),
            dados: json!({ "versao_anterior": atual.versao, "campos": campos }),
            criado_por: Some(usuario.id),
            origem: "faturamento",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    auditar_conta(&mut tx, id, usuario.id, &correlacao).await?;
    let detalhe = consultas::detalhe_da_conta(&mut tx, id)
        .await?
        .ok_or_else(conta_nao_encontrada)?;
    let corpo = serde_json::to_value(&detalhe).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 200, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::OK, Json(corpo)).into_response())
}
