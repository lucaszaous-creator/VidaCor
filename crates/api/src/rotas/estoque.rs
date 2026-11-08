//! Estoque com lote e validade (Estoque.dc.html), começando pelo contraste. Sai primeiro o lote que
//! vence antes (FEFO); a baixa é um UPDATE atômico que não deixa saldo negativo (G2); o contraste
//! aplicado fica ligado ao paciente e ao atendimento (docs/21) e o rastreio por lote lista quem recebeu.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::PacienteResumo;
use crate::rotas::triagem::exigir_lotacao;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoteDoItem {
    pub id: Uuid,
    pub numero: String,
    pub validade: NaiveDate,
    pub saldo: i32,
    pub vencido: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ItemDeEstoque {
    pub id: Uuid,
    pub codigo: String,
    pub nome: String,
    pub medida: String,
    pub minimo: i32,
    pub local: String,
    pub contraste: bool,
    /// Soma dos lotes dentro da validade.
    pub saldo_valido: i64,
    pub abaixo_do_minimo: bool,
    /// Lotes com saldo, na ordem de saída (vence antes, sai antes).
    pub lotes: Vec<LoteDoItem>,
}

/// Itens com saldo e lotes na ordem de saída.
#[utoipa::path(get, path = "/v1/estoque/itens", tag = "estoque",
    responses((status = 200, body = Vec<ItemDeEstoque>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn itens(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<ItemDeEstoque>>, ErroApi> {
    usuario.exigir(Ato::EstoqueVer)?;
    let hoje = tempo::hoje_local();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let itens = sqlx::query!("SELECT id, codigo, nome, medida, minimo, local, contraste FROM estoque.item WHERE ativo ORDER BY nome")
        .fetch_all(&mut *tx)
        .await?;
    let lotes = sqlx::query!("SELECT id, item_id, numero, validade, saldo FROM estoque.lote WHERE saldo > 0 ORDER BY validade, numero")
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(
        itens
            .into_iter()
            .map(|i| {
                let meus: Vec<LoteDoItem> = lotes
                    .iter()
                    .filter(|l| l.item_id == i.id)
                    .map(|l| LoteDoItem {
                        id: l.id,
                        numero: l.numero.clone(),
                        validade: l.validade,
                        saldo: l.saldo,
                        vencido: l.validade < hoje,
                    })
                    .collect();
                let saldo_valido: i64 = meus
                    .iter()
                    .filter(|l| !l.vencido)
                    .map(|l| i64::from(l.saldo))
                    .sum();
                ItemDeEstoque {
                    abaixo_do_minimo: saldo_valido < i64::from(i.minimo),
                    id: i.id,
                    codigo: i.codigo,
                    nome: i.nome,
                    medida: i.medida,
                    minimo: i.minimo,
                    local: i.local,
                    contraste: i.contraste,
                    saldo_valido,
                    lotes: meus,
                }
            })
            .collect(),
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct EntradaDeEstoque {
    pub item_id: Uuid,
    pub numero_lote: String,
    pub validade: NaiveDate,
    pub quantidade: i32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MovimentoFeito {
    pub movimento_id: Uuid,
    pub lote_id: Uuid,
    pub numero_lote: String,
    pub validade: NaiveDate,
    pub saldo_do_lote: i32,
}

async fn repetida(
    tx: &mut sqlx::PgConnection,
    usuario: &Usuario,
    rota: &str,
    chave: Uuid,
) -> Result<Option<Response>, ErroApi> {
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(tx, usuario.id, rota, chave).await?
    {
        return Ok(Some(
            (
                StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
                Json(corpo),
            )
                .into_response(),
        ));
    }
    Ok(None)
}

/// Entrada de mercadoria por lote (o lote existente soma; validade diferente para o mesmo número é recusada).
#[utoipa::path(post, path = "/v1/estoque/entradas", tag = "estoque", request_body = EntradaDeEstoque, params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = MovimentoFeito), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn entrada(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(e): Corpo<EntradaDeEstoque>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::EstoqueMovimentar)?;
    let numero = e.numero_lote.trim().to_uppercase();
    if !(1..=100_000).contains(&e.quantidade) || numero.len() < 2 || numero.len() > 30 {
        return Err(ErroApi::invalido(
            "entrada-invalida",
            "Informe o lote e uma quantidade entre 1 e 100.000. Nada entrou no estoque.",
        ));
    }
    if e.validade <= tempo::hoje_local() {
        return Err(ErroApi::invalido(
            "lote-vencido",
            "Lote vencido não entra no estoque. Nada entrou.",
        ));
    }
    const ROTA: &str = "POST /v1/estoque/entradas";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Some(r) = repetida(&mut tx, &usuario, ROTA, chave).await? {
        tx.commit().await?;
        return Ok(r);
    }
    let lote = sqlx::query!(
        "INSERT INTO estoque.lote (id, item_id, numero, validade, saldo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'estoque', $7, $8)
         ON CONFLICT (item_id, numero) DO UPDATE SET saldo = estoque.lote.saldo + EXCLUDED.saldo, versao = estoque.lote.versao + 1
           WHERE estoque.lote.validade = EXCLUDED.validade
         RETURNING id, saldo",
        novo_id(), e.item_id, numero, e.validade, e.quantidade, usuario.id, usuario.unidade_id, correlacao
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| match &err {
        sqlx::Error::Database(d) if d.is_foreign_key_violation() => ErroApi::nao_encontrado("item-nao-encontrado", "Item não encontrado. Nada entrou."),
        _ => ErroApi::from(err),
    })?
    .ok_or_else(|| ErroApi::conflito("validade-diferente", "Este número de lote já existe com outra validade. Confira a nota e o frasco. Nada entrou."))?;
    let mov = novo_id();
    sqlx::query!(
        "INSERT INTO estoque.movimento (id, lote_id, tipo, quantidade, criado_por, origem, unidade_id, correlacao) VALUES ($1, $2, 'entrada', $3, $4, 'estoque', $5, $6)",
        mov, lote.id, e.quantidade, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "estoque",
            tipo: "entrada",
            registro_id: mov,
            registro_codigo: Some(&numero),
            dados: json!({ "lote": lote.id, "quantidade": e.quantidade, "validade": e.validade }),
            criado_por: Some(usuario.id),
            origem: "estoque",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(MovimentoFeito {
        movimento_id: mov,
        lote_id: lote.id,
        numero_lote: numero,
        validade: e.validade,
        saldo_do_lote: lote.saldo,
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

#[derive(Deserialize, ToSchema)]
pub struct ContrasteAplicado {
    pub item_id: Uuid,
    pub quantidade: i32,
}

/// O técnico registra o contraste aplicado: baixa do lote que vence antes, ligada ao paciente.
#[utoipa::path(post, path = "/v1/atendimentos/{id}/contraste", tag = "estoque", params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)),
    request_body = ContrasteAplicado,
    responses((status = 201, body = MovimentoFeito), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn contraste(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(atendimento_id): Path<Uuid>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(c): Corpo<ContrasteAplicado>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::ContrasteRegistrar)?;
    if !(1..=10).contains(&c.quantidade) {
        return Err(ErroApi::invalido(
            "quantidade-invalida",
            "Informe de 1 a 10 unidades. Nada foi baixado.",
        ));
    }
    const ROTA: &str = "POST /v1/atendimentos/{id}/contraste";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Some(r) = repetida(&mut tx, &usuario, ROTA, chave).await? {
        tx.commit().await?;
        return Ok(r);
    }
    let a = sqlx::query!(
        "SELECT a.id, a.codigo, a.sala_id, a.paciente_id, g.requer_contraste FROM recepcao.atendimento a JOIN agenda.agendamento g ON g.id = a.agendamento_id WHERE a.id = $1",
        atendimento_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("atendimento-nao-encontrado", "Atendimento não encontrado. Nada foi baixado."))?;
    exigir_lotacao(&mut tx, usuario.id, a.sala_id).await?;
    if !a.requer_contraste {
        return Err(ErroApi::conflito(
            "exame-sem-contraste",
            "Este exame não é com contraste. Nada foi baixado.",
        ));
    }
    match crate::rotas::triagem::carregar(&mut tx, a.id).await? {
        Some(t) if t.exame_liberado => {}
        Some(_) => {
            return Err(ErroApi::conflito(
                "triagem-pede-conduta",
                "A triagem pede a conduta do médico antes do contraste. Nada foi baixado.",
            ))
        }
        None => {
            return Err(ErroApi::conflito(
                "sem-triagem",
                "Faça a triagem de segurança antes do contraste (FC-TRI-06). Nada foi baixado.",
            ))
        }
    }
    // FEFO + saldo: só o lote válido que vence antes e tem saldo; o UPDATE confere de novo (G2).
    let hoje = tempo::hoje_local();
    let lote = sqlx::query!(
        "UPDATE estoque.lote SET saldo = saldo - $2, versao = versao + 1
          WHERE id = (SELECT id FROM estoque.lote WHERE item_id = $1 AND validade >= $3 AND saldo >= $2
                       ORDER BY validade, numero LIMIT 1 FOR UPDATE SKIP LOCKED)
            AND saldo >= $2
          RETURNING id, numero, validade, saldo",
        c.item_id, c.quantidade, hoje
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::conflito("sem-lote-valido", "Não há lote dentro da validade com saldo para este contraste. Nada foi baixado. Avise o estoque."))?;
    let mov = novo_id();
    sqlx::query!(
        "INSERT INTO estoque.movimento (id, lote_id, tipo, quantidade, atendimento_id, paciente_id, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, 'baixa_exame', $3, $4, $5, $6, 'sala', $7, $8)",
        mov, lote.id, c.quantidade, a.id, a.paciente_id, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("movimento_contraste_unico") => {
            ErroApi::conflito("contraste-ja-registrado", "O contraste deste lote já foi registrado para este atendimento. Nada foi baixado de novo.")
        }
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "estoque",
            tipo: "contraste_aplicado",
            registro_id: mov,
            registro_codigo: Some(&lote.numero),
            dados: json!({ "atendimento": a.codigo, "lote": lote.id, "quantidade": c.quantidade }),
            criado_por: Some(usuario.id),
            origem: "sala",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(MovimentoFeito {
        movimento_id: mov,
        lote_id: lote.id,
        numero_lote: lote.numero,
        validade: lote.validade,
        saldo_do_lote: lote.saldo,
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PacienteDoLote {
    pub paciente: PacienteResumo,
    pub atendimento_codigo: String,
    pub quantidade: i32,
    pub aplicado_em: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RastreioDoLote {
    pub numero: String,
    pub item: String,
    pub validade: NaiveDate,
    pub pacientes: Vec<PacienteDoLote>,
}

/// Rastreio por lote (alerta da Anvisa): quem recebeu cada lote. A leitura fica no lastro.
#[utoipa::path(get, path = "/v1/estoque/lotes/{id}/rastreio", tag = "estoque", params(("id" = Uuid, Path)),
    responses((status = 200, body = RastreioDoLote), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn rastreio(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<RastreioDoLote>, ErroApi> {
    usuario.exigir(Ato::LoteRastrear)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let lote = sqlx::query!("SELECT l.numero, l.validade, i.nome FROM estoque.lote l JOIN estoque.item i ON i.id = l.item_id WHERE l.id = $1", id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::nao_encontrado("lote-nao-encontrado", "Lote não encontrado."))?;
    let pacientes = sqlx::query!(
        r#"SELECT p.id, p.nome, p.nascimento, a.codigo, m.quantidade, m.criado_em
           FROM estoque.movimento m JOIN identidade.paciente p ON p.id = m.paciente_id JOIN recepcao.atendimento a ON a.id = m.atendimento_id
           WHERE m.lote_id = $1 AND m.tipo = 'baixa_exame' ORDER BY m.criado_em"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    // Leitura de dado com paciente registrada (docs/21: leituras de dado clínico).
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "estoque",
            tipo: "rastreio_de_lote_lido",
            registro_id: id,
            registro_codigo: Some(&lote.numero),
            dados: json!({ "pacientes": pacientes.len() }),
            criado_por: Some(usuario.id),
            origem: "gerencia",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Json(RastreioDoLote {
        numero: lote.numero,
        item: lote.nome,
        validade: lote.validade,
        pacientes: pacientes
            .into_iter()
            .map(|p| PacienteDoLote {
                paciente: PacienteResumo {
                    id: p.id,
                    nome: p.nome,
                    nascimento: p.nascimento,
                },
                atendimento_codigo: p.codigo,
                quantidade: p.quantidade,
                aplicado_em: p.criado_em,
            })
            .collect(),
    }))
}
