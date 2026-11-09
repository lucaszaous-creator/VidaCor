//! Agendamento de convênio com guia autorizada (docs/21 § 3.3, docs/27 FC-AGD, docs/30 § 3).
//! A guia é consumida de forma atômica e só por um agendamento; sem guia, só reserva provisória
//! com prazo, que vence sozinha e devolve a vaga.

use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::America::Sao_Paulo;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};

/// Prazo da reserva provisória sem guia (FC-AGD-08; a confirmar com a recepção).
pub const PRAZO_RESERVA_HORAS: i64 = 48;
const DURACAO_MIN: i64 = 30;

#[derive(Deserialize, ToSchema)]
pub struct ProcedimentoGuia {
    pub tuss: String,
    pub quantidade: i32,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoGuia {
    pub operadora: String,
    pub numero: String,
    pub carteirinha: String,
    pub senha: String,
    pub validade: NaiveDate,
    pub paciente_id: Uuid,
    pub solicitante: String,
    pub procedimentos: Vec<ProcedimentoGuia>,
}

#[derive(Serialize, ToSchema)]
pub struct SaldoProcedimento {
    pub tuss: String,
    pub autorizado: i32,
    pub saldo: i32,
}

#[derive(Serialize, ToSchema)]
pub struct Guia {
    pub id: Uuid,
    pub codigo: String,
    pub operadora: String,
    pub numero: String,
    pub validade: NaiveDate,
    /// disponivel ou consumida.
    pub situacao: String,
    pub procedimentos: Vec<SaldoProcedimento>,
    /// Atendimento ou agendamento que consumiu a guia.
    pub em_uso_por: Option<String>,
}

fn tuss_valido(t: &str) -> bool {
    t.len() == 8 && t.chars().all(|c| c.is_ascii_digit())
}

/// Registra a guia autorizada pela operadora (carteirinha, número, senha, validade e procedimentos).
#[utoipa::path(post, path = "/v1/guias", tag = "agenda", request_body = PedidoGuia,
    params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = Guia), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn registrar_guia(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<PedidoGuia>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::GuiaRegistrar)?;
    if p.procedimentos.is_empty()
        || p.procedimentos.len() > 20
        || p.procedimentos
            .iter()
            .any(|x| !tuss_valido(&x.tuss) || !(1..=99).contains(&x.quantidade))
    {
        return Err(ErroApi::invalido(
            "guia-invalida",
            "Confira os procedimentos da guia: código TUSS de 8 dígitos e quantidade de 1 a 99. Nada foi gravado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, "POST /v1/guias", chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let paciente = sqlx::query_scalar!(
        "SELECT unidade_id FROM identidade.paciente WHERE id = $1",
        p.paciente_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "paciente-nao-encontrado",
            "Paciente não encontrado nesta unidade. Nada foi gravado.",
        )
    })?;
    let ano = tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero('GUI', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::interno("contador GUI sem valor"))?;
    let codigo = CodigoLegivel::novo("GUI", ano, n);
    let id = novo_id();
    sqlx::query!(
        "INSERT INTO convenio.guia (id, codigo, operadora, numero, carteirinha, senha, validade, paciente_id, solicitante,
                                    criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'balcao', $11, $12, $13)",
        id,
        codigo.como_str(),
        p.operadora.trim(),
        p.numero.trim(),
        // Carteirinha é guardada sem espaços (vem escrita de vários jeitos no cartão).
        p.carteirinha.split_whitespace().collect::<String>(),
        p.senha.trim(),
        p.validade,
        p.paciente_id,
        p.solicitante.trim(),
        usuario.id,
        estacao,
        paciente,
        correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito(
            "guia-ja-registrada",
            "Esta guia já foi registrada para esta operadora. Nada foi gravado. Use a guia já cadastrada.",
        ),
        sqlx::Error::Database(d) if d.is_check_violation() => ErroApi::invalido(
            "guia-invalida",
            "Algum campo da guia está em formato inválido (número, carteirinha, senha ou solicitante). Nada foi gravado.",
        ),
        _ => ErroApi::from(e),
    })?;
    for x in &p.procedimentos {
        sqlx::query!(
            "INSERT INTO convenio.guia_procedimento (guia_id, tuss, autorizado, saldo) VALUES ($1, $2, $3, $3)",
            id,
            x.tuss,
            x.quantidade
        )
        .execute(&mut *tx)
        .await?;
    }
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "convenio", tipo: "guia_registrada", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "operadora": p.operadora, "numero": p.numero, "validade": p.validade, "procedimentos": p.procedimentos.iter().map(|x| &x.tuss).collect::<Vec<_>>() }),
        criado_por: Some(usuario.id), origem: "balcao", unidade_id: paciente, correlacao: &correlacao,
    }).await?;
    let guia = carregar_guia(&mut tx, id).await?;
    let corpo = serde_json::to_value(&guia).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, "POST /v1/guias", chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

async fn carregar_guia(conexao: &mut PgConnection, id: Uuid) -> Result<Guia, ErroApi> {
    let g = sqlx::query!(
        r#"SELECT g.id, g.codigo, g.operadora, g.numero, g.validade, g.situacao,
                  COALESCE(t.codigo, a.codigo) AS em_uso_por
             FROM convenio.guia g
             LEFT JOIN agenda.agendamento a ON a.id = g.agendamento_id
             LEFT JOIN recepcao.atendimento t ON t.agendamento_id = a.id
            WHERE g.id = $1"#,
        id
    )
    .fetch_one(&mut *conexao)
    .await?;
    let procedimentos = sqlx::query_as!(
        SaldoProcedimento,
        "SELECT tuss, autorizado, saldo FROM convenio.guia_procedimento WHERE guia_id = $1 ORDER BY tuss",
        id
    )
    .fetch_all(&mut *conexao)
    .await?;
    Ok(Guia {
        id: g.id,
        codigo: g.codigo,
        operadora: g.operadora,
        numero: g.numero,
        validade: g.validade,
        situacao: g.situacao,
        procedimentos,
        em_uso_por: g.em_uso_por,
    })
}

/// Confere e consome a guia para os procedimentos do agendamento, de forma atômica (G2).
pub(crate) async fn consumir_guia(
    conexao: &mut PgConnection,
    guia_id: Uuid,
    paciente_id: Uuid,
    data_exame: NaiveDate,
    procedimentos: &[String],
    agendamento_id: Uuid,
) -> Result<String, ErroApi> {
    // Trava a guia: duas recepções com a mesma guia esperam uma pela outra (FC-AGD-03).
    let g = sqlx::query!(
        "SELECT id, codigo, paciente_id, validade, situacao FROM convenio.guia WHERE id = $1 FOR UPDATE",
        guia_id
    )
    .fetch_optional(&mut *conexao)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("guia-nao-encontrada", "A guia informada não foi encontrada nesta unidade. Nada foi agendado. Registre a guia antes."))?;
    if g.paciente_id != paciente_id {
        return Err(ErroApi::invalido(
            "guia-de-outro-paciente",
            "Esta guia é de outro paciente. Nada foi agendado. Confira a carteirinha.",
        ));
    }
    if g.situacao != "disponivel" {
        let uso = carregar_guia(conexao, g.id)
            .await?
            .em_uso_por
            .unwrap_or_default();
        return Err(ErroApi::conflito(
            "guia-em-uso",
            &format!("A guia já está em uso no atendimento {uso}. Nada foi agendado. É preciso outra guia autorizada."),
        )
        .com_extra("atendimento_codigo", json!(uso)));
    }
    if data_exame > g.validade {
        return Err(ErroApi::invalido(
            "senha-vencida",
            &format!("A senha da guia vale até {}, antes da data do exame. Nada foi agendado. Remarque dentro da validade ou peça prorrogação à operadora.", g.validade.format("%d/%m/%Y")),
        )
        .com_extra("validade", json!(g.validade)));
    }
    for tuss in procedimentos {
        let saldo = sqlx::query_scalar!(
            "SELECT saldo FROM convenio.guia_procedimento WHERE guia_id = $1 AND tuss = $2",
            g.id,
            tuss
        )
        .fetch_optional(&mut *conexao)
        .await?;
        match saldo {
            None => {
                return Err(ErroApi::invalido(
                    "procedimento-nao-autorizado",
                    &format!("O procedimento {tuss} não está autorizado nesta guia. Nada foi agendado. Peça a autorização à operadora."),
                )
                .com_extra("tuss", json!(tuss)))
            }
            Some(0) => {
                return Err(ErroApi::invalido(
                    "saldo-esgotado",
                    &format!("A guia não tem mais saldo para o procedimento {tuss}. Nada foi agendado."),
                ))
            }
            Some(_) => {}
        }
        // Operação atômica conferindo as linhas: o saldo nunca fica negativo.
        let ok = sqlx::query!(
            "UPDATE convenio.guia_procedimento SET saldo = saldo - 1 WHERE guia_id = $1 AND tuss = $2 AND saldo > 0",
            g.id,
            tuss
        )
        .execute(&mut *conexao)
        .await?
        .rows_affected();
        if ok != 1 {
            return Err(ErroApi::invalido(
                "saldo-esgotado",
                &format!(
                    "A guia não tem mais saldo para o procedimento {tuss}. Nada foi agendado."
                ),
            ));
        }
    }
    let consumida = sqlx::query!(
        "UPDATE convenio.guia SET situacao = 'consumida', agendamento_id = $2, versao = versao + 1
          WHERE id = $1 AND situacao = 'disponivel'",
        g.id,
        agendamento_id
    )
    .execute(&mut *conexao)
    .await?
    .rows_affected();
    if consumida != 1 {
        return Err(ErroApi::conflito("guia-em-uso", "A guia acabou de ser usada por outro agendamento. Nada foi agendado. É preciso outra guia."));
    }
    Ok(g.codigo)
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoAgendamento {
    pub paciente_id: Uuid,
    pub sala_id: Uuid,
    pub inicio: DateTime<Utc>,
    pub procedimentos: Vec<String>,
    pub guia_id: Option<Uuid>,
    #[serde(default)]
    pub reserva_provisoria: bool,
    pub medico_id: Option<Uuid>,
}

/// Horários livres de 30 minutos da sala num dia (07:00 às 19:00, horário de Brasília).
async fn livres(
    conexao: &mut PgConnection,
    sala_id: Uuid,
    dia: NaiveDate,
    a_partir: DateTime<Utc>,
) -> Result<Vec<DateTime<Utc>>, ErroApi> {
    let (ini, fim) = tempo::limites_do_dia(dia).ok_or_else(|| ErroApi::interno("dia inválido"))?;
    let ocupados: Vec<DateTime<Utc>> = sqlx::query_scalar!(
        "SELECT inicio FROM agenda.agendamento WHERE sala_id = $1 AND inicio >= $2 AND inicio < $3
            AND situacao NOT IN ('cancelado','faltou','expirada')",
        sala_id,
        ini,
        fim
    )
    .fetch_all(&mut *conexao)
    .await?;
    let mut saida = Vec::new();
    let Some(primeiro) = dia
        .and_hms_opt(7, 0, 0)
        .and_then(|d| Sao_Paulo.from_local_datetime(&d).earliest())
    else {
        return Ok(saida);
    };
    let mut h = primeiro.with_timezone(&Utc);
    for _ in 0..24 {
        if h >= a_partir && !ocupados.contains(&h) {
            saida.push(h);
        }
        h += Duration::minutes(DURACAO_MIN);
    }
    Ok(saida)
}

/// Agenda um exame (com guia, reserva provisória ou particular).
#[utoipa::path(post, path = "/v1/agendamentos", tag = "agenda", request_body = PedidoAgendamento,
    params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = crate::rotas::agenda::ItemAgenda), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc, description = "guia-em-uso, horario-ocupado (com sugestoes)"),
              (status = 422, body = crate::openapi::ProblemaDoc, description = "procedimento-nao-autorizado, senha-vencida, saldo-esgotado")))]
pub async fn agendar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<PedidoAgendamento>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::AgendaAgendar)?;
    if p.procedimentos.is_empty()
        || p.procedimentos.len() > 10
        || p.procedimentos.iter().any(|t| !tuss_valido(t))
    {
        return Err(ErroApi::invalido(
            "procedimentos-invalidos",
            "Escolha de 1 a 10 procedimentos com código TUSS de 8 dígitos. Nada foi agendado.",
        ));
    }
    if p.guia_id.is_some() && p.reserva_provisoria {
        return Err(ErroApi::invalido("pedido-ambiguo", "Com guia o agendamento é confirmado; reserva provisória é só para quem ainda não tem guia. Nada foi agendado."));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let dia = p.inicio.with_timezone(&Sao_Paulo).date_naive();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, "POST /v1/agendamentos", chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let sala = sqlx::query!(
        "SELECT id, nome FROM agenda.sala WHERE id = $1 AND unidade_id = $2",
        p.sala_id,
        usuario.unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "sala-nao-encontrada",
            "Sala não encontrada nesta unidade. Nada foi agendado.",
        )
    })?;
    let paciente = sqlx::query!(
        "SELECT id FROM identidade.paciente WHERE id = $1",
        p.paciente_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "paciente-nao-encontrado",
            "Paciente não encontrado nesta unidade. Nada foi agendado.",
        )
    })?;
    let catalogo = sqlx::query!(
        "SELECT tuss, nome, requer_contraste FROM agenda.procedimento WHERE tuss = ANY($1) AND ativo",
        &p.procedimentos
    )
    .fetch_all(&mut *tx)
    .await?;
    if catalogo.len() != p.procedimentos.len() {
        return Err(ErroApi::invalido(
            "procedimento-desconhecido",
            "Algum procedimento não está no catálogo da clínica. Nada foi agendado.",
        ));
    }
    let exame_nome = p
        .procedimentos
        .iter()
        .filter_map(|t| {
            catalogo
                .iter()
                .find(|c| &c.tuss == t)
                .map(|c| c.nome.clone())
        })
        .collect::<Vec<_>>()
        .join(" + ");
    let contraste = catalogo.iter().any(|c| c.requer_contraste);

    let ano = tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero('AGD', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::interno("contador AGD sem valor"))?;
    let codigo = CodigoLegivel::novo("AGD", ano, n);
    let id = novo_id();
    let situacao = if p.guia_id.is_some() {
        "confirmado"
    } else if p.reserva_provisoria {
        "reserva_provisoria"
    } else {
        "agendado"
    };
    let expira_em = (situacao == "reserva_provisoria")
        .then(|| Utc::now() + Duration::hours(PRAZO_RESERVA_HORAS));

    let inserido = sqlx::query!(
        "INSERT INTO agenda.agendamento (id, codigo, sala_id, paciente_id, inicio, fim, exame_tuss, exame_nome, situacao,
                requer_contraste, medico_id, expira_em, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'balcao', $14, $15, $16)",
        id,
        codigo.como_str(),
        sala.id,
        paciente.id,
        p.inicio,
        p.inicio + Duration::minutes(DURACAO_MIN),
        p.procedimentos[0],
        exame_nome,
        situacao,
        contraste,
        p.medico_id,
        expira_em,
        usuario.id,
        estacao,
        usuario.unidade_id,
        correlacao
    )
    .execute(&mut *tx)
    .await;
    if let Err(sqlx::Error::Database(d)) = &inserido {
        // Ausência do médico aprovada pela coordenação (gatilho da migration 0080, DEC-2026-11-09-142).
        if d.constraint() == Some("medico_ausente") {
            return Err(ErroApi::conflito(
                "medico-ausente",
                "O médico escolhido está com a agenda bloqueada neste horário (ausência aprovada pela coordenação). Nada foi agendado. Escolha outro médico ou outro horário.",
            ));
        }
        // Só o índice do horário da sala vira "horário ocupado"; outra violação é falha real (R8).
        if d.is_unique_violation() && d.constraint() == Some("agendamento_horario_sala_unico") {
            // Horário ocupado (G2: índice único). Desfaz e devolve os próximos livres (FC-AGD-11).
            tx.rollback().await?;
            let mut tx2 = banco::transacao(&estado.banco, &usuario.contexto()).await?;
            let sugestoes: Vec<DateTime<Utc>> = livres(&mut tx2, sala.id, dia, p.inicio)
                .await?
                .into_iter()
                .take(3)
                .collect();
            tx2.commit().await?;
            return Err(ErroApi::conflito(
                "horario-ocupado",
                &format!("Este horário da {} acabou de ser ocupado. Nada foi agendado. Escolha um dos próximos horários livres.", sala.nome),
            )
            .com_extra("sugestoes", json!(sugestoes)));
        }
    }
    inserido?;
    for t in &p.procedimentos {
        sqlx::query!(
            "INSERT INTO agenda.agendamento_procedimento (agendamento_id, tuss, criado_por, origem, estacao, unidade_id, correlacao)
             VALUES ($1, $2, $3, 'balcao', $4, $5, $6)",
            id, t, usuario.id, estacao, usuario.unidade_id, correlacao
        )
        .execute(&mut *tx)
        .await?;
    }
    let guia_codigo = match p.guia_id {
        Some(g) => Some(consumir_guia(&mut tx, g, paciente.id, dia, &p.procedimentos, id).await?),
        None => None,
    };
    if let Some(g) = p.guia_id {
        sqlx::query!(
            "UPDATE agenda.agendamento SET guia_id = $2 WHERE id = $1",
            id,
            g
        )
        .execute(&mut *tx)
        .await?;
    }
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "agenda", tipo: "agendamento_criado", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "situacao": situacao, "guia": guia_codigo, "procedimentos": p.procedimentos, "canal": "balcao", "estacao": estacao }),
        criado_por: Some(usuario.id), origem: "balcao", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let item = carregar_item(&mut tx, id).await?;
    let corpo = serde_json::to_value(&item).map_err(ErroApi::interno)?;
    idempotencia::concluir(
        &mut tx,
        usuario.id,
        "POST /v1/agendamentos",
        chave,
        201,
        &corpo,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

pub(crate) async fn carregar_item(
    conexao: &mut PgConnection,
    id: Uuid,
) -> Result<crate::rotas::agenda::ItemAgenda, ErroApi> {
    use crate::rotas::agenda::{Exame, ItemAgenda, PacienteResumo, Sala};
    let l = sqlx::query!(
        r#"SELECT a.id, a.codigo, a.inicio, a.fim, a.situacao, a.exame_tuss, a.exame_nome, a.requer_contraste, a.expira_em,
                  s.id AS sala_id, s.nome AS sala_nome, p.id AS paciente_id, p.nome, p.nascimento
             FROM agenda.agendamento a JOIN agenda.sala s ON s.id = a.sala_id JOIN identidade.paciente p ON p.id = a.paciente_id
            WHERE a.id = $1"#,
        id
    )
    .fetch_one(conexao)
    .await?;
    Ok(ItemAgenda {
        id: l.id,
        codigo: l.codigo,
        inicio: l.inicio,
        fim: l.fim,
        sala: Sala {
            id: l.sala_id,
            nome: l.sala_nome,
        },
        paciente: PacienteResumo {
            id: l.paciente_id,
            nome: l.nome,
            nascimento: l.nascimento,
        },
        exame: Exame {
            codigo_tuss: l.exame_tuss,
            nome: l.exame_nome,
        },
        situacao: l.situacao,
        atendimento_id: None,
        homonimos: Vec::new(),
        requer_contraste: l.requer_contraste,
        laudo_situacao: None,
        expira_em: l.expira_em,
    })
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoGuiaDaReserva {
    pub guia_id: Uuid,
}

/// Registra a guia de uma reserva provisória dentro do prazo e confirma o agendamento (FC-AGD-09).
#[utoipa::path(post, path = "/v1/agendamentos/{id}/guia", tag = "agenda", params(("id" = Uuid, Path)),
    request_body = PedidoGuiaDaReserva,
    responses((status = 200, body = crate::rotas::agenda::ItemAgenda), (status = 409, body = crate::openapi::ProblemaDoc),
              (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn confirmar_com_guia(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoGuiaDaReserva>,
) -> Result<Json<crate::rotas::agenda::ItemAgenda>, ErroApi> {
    usuario.exigir(Ato::AgendaAgendar)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let a = sqlx::query!(
        "SELECT id, codigo, paciente_id, inicio, situacao, expira_em, unidade_id FROM agenda.agendamento WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("agendamento-nao-encontrado", "Agendamento não encontrado nesta unidade."))?;
    if a.situacao != "reserva_provisoria" || a.expira_em.is_some_and(|e| e <= Utc::now()) {
        return Err(ErroApi::conflito("reserva-indisponivel", "Esta reserva não está mais provisória ou já venceu. Nada foi alterado. Faça um novo agendamento."));
    }
    let procedimentos: Vec<String> = sqlx::query_scalar!(
        "SELECT tuss FROM agenda.agendamento_procedimento WHERE agendamento_id = $1",
        a.id
    )
    .fetch_all(&mut *tx)
    .await?;
    let dia = a.inicio.with_timezone(&Sao_Paulo).date_naive();
    let guia = consumir_guia(&mut tx, p.guia_id, a.paciente_id, dia, &procedimentos, a.id).await?;
    sqlx::query!(
        "UPDATE agenda.agendamento SET situacao = 'confirmado', guia_id = $2, expira_em = NULL, versao = versao + 1
          WHERE id = $1 AND situacao = 'reserva_provisoria'",
        a.id,
        p.guia_id
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "agenda",
            tipo: "reserva_confirmada_com_guia",
            registro_id: a.id,
            registro_codigo: Some(&a.codigo),
            dados: json!({ "guia": guia }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: a.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let item = carregar_item(&mut tx, a.id).await?;
    tx.commit().await?;
    Ok(Json(item))
}

#[derive(Deserialize, IntoParams)]
pub struct DiaQuery {
    pub data: NaiveDate,
}

/// Horários livres de 30 minutos da sala no dia.
#[utoipa::path(get, path = "/v1/salas/{sala_id}/horarios-livres", tag = "agenda", params(("sala_id" = Uuid, Path), DiaQuery),
    responses((status = 200, body = Vec<DateTime<Utc>>)))]
pub async fn horarios_livres(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Query(q): Query<DiaQuery>,
) -> Result<Json<Vec<DateTime<Utc>>>, ErroApi> {
    usuario.exigir(Ato::AgendaVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let horarios = livres(&mut tx, id, q.data, Utc::now()).await?;
    tx.commit().await?;
    Ok(Json(horarios))
}

/// Vence as reservas provisórias sem guia no prazo: devolve a vaga e avisa o paciente (FC-AGD-08).
///
/// # Errors
/// Falha de banco.
pub async fn expirar_reservas(estado: &Estado, unidade_id: Uuid) -> Result<u64, ErroApi> {
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    let vencidas = sqlx::query!(
        "UPDATE agenda.agendamento SET situacao = 'expirada', motivo = 'guia não entregue no prazo', versao = versao + 1
          WHERE situacao = 'reserva_provisoria' AND expira_em <= now() AND unidade_id = $1
          RETURNING id, codigo, sala_id, inicio",
        unidade_id
    )
    .fetch_all(&mut *tx)
    .await?;
    for v in &vencidas {
        lastro::registrar(
            &mut tx,
            NovoEvento {
                dominio: "agenda",
                tipo: "reserva_expirada",
                registro_id: v.id,
                registro_codigo: Some(&v.codigo),
                dados: json!({ "motivo": "guia não entregue no prazo" }),
                criado_por: None,
                origem: "nucleo",
                unidade_id,
                correlacao: "expiracao",
            },
        )
        .await?;
        // Aviso ao paciente pelo canal autorizado e oferta da vaga à lista de espera, pelo outbox.
        for tipo in ["aviso.reserva_expirada", "lista_espera.oferecer_vaga"] {
            sqlx::query!(
                "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
                 VALUES ($1, $2, $3, $4, $5, 'expiracao', $6) ON CONFLICT (tipo, referencia) WHERE referencia IS NOT NULL DO NOTHING",
                novo_id(), tipo, novo_id(), json!({ "agendamento_id": v.id, "sala_id": v.sala_id, "inicio": v.inicio }), unidade_id, v.id
            )
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(vencidas.len() as u64)
}
