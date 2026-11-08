//! Oferta de vaga a um paciente da lista de espera: oferta manual pela recepção, aceite com dois
//! identificadores e recusa com motivo (docs/04 Lista-Espera, DEC-2026-11-08-83). O aceite é atômico:
//! a oferta só passa de "aguardando" para "aceita" uma vez, com a sala travada (G2).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::{Exame, PacienteResumo, Sala};
use crate::rotas::vagas::{self, NovaOferta, DURACAO_MIN};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel, DataNascimento, NomeCompleto};
use vidacor_dominio_recepcao::espera::{prazo_da_oferta, PRAZOS_MIN};

const ROTA_OFERECER: &str = "POST /v1/ofertas";
const ROTA_ACEITE: &str = "POST /v1/ofertas/{id}/aceite";

#[derive(Serialize, ToSchema)]
pub struct Oferta {
    pub id: Uuid,
    pub codigo: String,
    pub espera_id: Uuid,
    pub espera_codigo: String,
    pub paciente: PacienteResumo,
    pub exame: Exame,
    pub sala: Sala,
    pub inicio: DateTime<Utc>,
    pub fim: DateTime<Utc>,
    pub expira_em: DateTime<Utc>,
    /// aguardando, aceita, recusada, expirada ou cancelada.
    pub situacao: String,
    /// cancelamento, bloqueio_retirado, reserva_expirada, oferta_sem_aceite ou manual.
    pub origem_vaga: String,
    pub motivo: Option<String>,
    /// Agendamento criado pelo aceite.
    pub agendamento_codigo: Option<String>,
}

async fn carregar(conexao: &mut PgConnection, id: Uuid) -> Result<Oferta, ErroApi> {
    let o = sqlx::query!(
        r#"SELECT o.id, o.codigo, o.espera_id, e.codigo AS espera_codigo, o.inicio, o.fim, o.expira_em, o.situacao,
                  o.origem_vaga, o.motivo, a.codigo AS "agendamento_codigo?",
                  p.id AS paciente_id, p.nome, p.nascimento, c.tuss, c.nome AS exame_nome, s.id AS sala_id, s.nome AS sala_nome
             FROM agenda.oferta o
             JOIN agenda.espera e ON e.id = o.espera_id
             JOIN identidade.paciente p ON p.id = e.paciente_id
             JOIN agenda.procedimento c ON c.tuss = e.exame_tuss
             JOIN agenda.sala s ON s.id = o.sala_id
             LEFT JOIN agenda.agendamento a ON a.id = o.agendamento_id
            WHERE o.id = $1"#,
        id
    )
    .fetch_optional(&mut *conexao)
    .await?
    .ok_or_else(nao_encontrada)?;
    Ok(Oferta {
        id: o.id,
        codigo: o.codigo,
        espera_id: o.espera_id,
        espera_codigo: o.espera_codigo,
        paciente: PacienteResumo {
            id: o.paciente_id,
            nome: o.nome,
            nascimento: o.nascimento,
        },
        exame: Exame {
            codigo_tuss: o.tuss,
            nome: o.exame_nome,
        },
        sala: Sala {
            id: o.sala_id,
            nome: o.sala_nome,
        },
        inicio: o.inicio,
        fim: o.fim,
        expira_em: o.expira_em,
        situacao: o.situacao,
        origem_vaga: o.origem_vaga,
        motivo: o.motivo,
        agendamento_codigo: o.agendamento_codigo,
    })
}

fn nao_encontrada() -> ErroApi {
    ErroApi::nao_encontrado(
        "oferta-nao-encontrada",
        "Esta oferta não existe nesta unidade. Nada foi alterado.",
    )
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoOferta {
    pub espera_id: Uuid,
    pub sala_id: Uuid,
    pub inicio: DateTime<Utc>,
    /// Prazo de resposta em minutos: 15, 20, 30, 60 ou 120 (nunca passa de 30 minutos antes do exame).
    pub prazo_min: i64,
}

/// Oferece um horário livre a um pedido escolhido da lista (encaixe pela recepção).
#[utoipa::path(post, operation_id = "oferecer_vaga", path = "/v1/ofertas", tag = "agenda",
    request_body = PedidoOferta, params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = Oferta), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc, description = "espera-nao-encontrada, sala-nao-encontrada"),
              (status = 409, body = crate::openapi::ProblemaDoc, description = "pedido-indisponivel, vaga-ja-ofertada, horario-ocupado"),
              (status = 422, body = crate::openapi::ProblemaDoc, description = "prazo-invalido, vaga-perto-demais, exame-incompativel-com-a-sala")))]
pub async fn oferecer(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<PedidoOferta>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::EsperaGerir)?;
    if !PRAZOS_MIN.contains(&p.prazo_min) {
        return Err(ErroApi::invalido(
            "prazo-invalido",
            "Escolha o prazo de resposta: 15, 20, 30, 60 ou 120 minutos. Nada foi oferecido.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, ROTA_OFERECER, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let espera = sqlx::query!(
        "SELECT e.id, c.modalidade FROM agenda.espera e JOIN agenda.procedimento c ON c.tuss = e.exame_tuss WHERE e.id = $1",
        p.espera_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("espera-nao-encontrada", "Este pedido de espera não existe nesta unidade. Nada foi oferecido."))?;
    vagas::vencer(
        &mut tx,
        usuario.unidade_id,
        Some((p.sala_id, p.inicio)),
        &correlacao,
    )
    .await?;
    // Trava a sala antes de decidir (G2): agendar, bloquear e oferecer na sala esperam.
    let sala = sqlx::query_scalar!(
        "SELECT modalidade FROM agenda.sala WHERE id = $1 AND unidade_id = $2 FOR UPDATE",
        p.sala_id,
        usuario.unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "sala-nao-encontrada",
            "Sala não encontrada nesta unidade. Nada foi oferecido.",
        )
    })?;
    if espera.modalidade.as_deref().is_some_and(|m| m != sala) {
        return Err(ErroApi::invalido("exame-incompativel-com-a-sala", "Esta sala não faz o exame pedido. Nada foi oferecido. Escolha uma sala com o aparelho do exame."));
    }
    let Some(expira_em) = prazo_da_oferta(tempo::agora(), p.inicio, p.prazo_min) else {
        return Err(ErroApi::invalido("vaga-perto-demais", "Este horário está perto demais (ou já passou) para esperar a resposta do paciente. Nada foi oferecido. Encaixe direto pela agenda, se ele estiver na clínica."));
    };
    let criada = vagas::criar_oferta(&mut tx, NovaOferta {
        unidade_id: usuario.unidade_id,
        espera_id: espera.id,
        sala_id: p.sala_id,
        inicio: p.inicio,
        expira_em,
        origem_vaga: "manual",
        criado_por: Some(usuario.id),
        origem: "balcao",
        estacao,
        correlacao: &correlacao,
    })
    .await?
    .ok_or_else(|| ErroApi::conflito("pedido-indisponivel", "Este paciente já tem uma oferta aguardando resposta ou saiu da lista. Nada foi oferecido."))?;
    let oferta = carregar(&mut tx, criada).await?;
    let corpo = serde_json::to_value(&oferta).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA_OFERECER, chave, 201, &corpo).await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

#[derive(Deserialize, IntoParams)]
pub struct FiltroOfertas {
    /// aguardando (padrão), aceita, recusada, expirada ou cancelada.
    pub situacao: Option<String>,
    /// Dia do horário oferecido (AAAA-MM-DD); sem dia, as próximas.
    pub data: Option<NaiveDate>,
}

/// Ofertas da unidade (por padrão, as que aguardam resposta), pelo prazo.
#[utoipa::path(get, operation_id = "listar_ofertas", path = "/v1/ofertas", tag = "agenda", params(FiltroOfertas),
    responses((status = 200, body = Vec<Oferta>), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 422, body = crate::openapi::ProblemaDoc, description = "filtro-invalido")))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroOfertas>,
) -> Result<Json<Vec<Oferta>>, ErroApi> {
    usuario.exigir(Ato::EsperaGerir)?;
    let situacao = f.situacao.unwrap_or_else(|| "aguardando".into());
    if !["aguardando", "aceita", "recusada", "expirada", "cancelada"].contains(&situacao.as_str()) {
        return Err(ErroApi::invalido(
            "filtro-invalido",
            "Situação de oferta desconhecida.",
        ));
    }
    let (ini, fim) = match f.data {
        Some(d) => tempo::limites_do_dia(d).ok_or_else(|| ErroApi::interno("dia inválido"))?,
        None => (
            tempo::agora() - Duration::days(1),
            tempo::agora() + Duration::days(400),
        ),
    };
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let ids = sqlx::query_scalar!(
        "SELECT id FROM agenda.oferta WHERE unidade_id = $1 AND situacao = $2 AND inicio >= $3 AND inicio < $4
          ORDER BY expira_em, codigo LIMIT 200",
        usuario.unidade_id,
        situacao,
        ini,
        fim
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut saida = Vec::with_capacity(ids.len());
    for id in ids {
        saida.push(carregar(&mut tx, id).await?);
    }
    tx.commit().await?;
    Ok(Json(saida))
}

#[derive(Deserialize, ToSchema)]
pub struct ConfirmacaoDoAceite {
    /// Nome completo dito pelo paciente.
    pub nome: String,
    pub nascimento: NaiveDate,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoAceite {
    pub confirmacao: ConfirmacaoDoAceite,
}

fn identificacao_nao_confere() -> ErroApi {
    ErroApi::invalido(
        "identificacao-nao-confere",
        "Nome ou data de nascimento não conferem com o cadastro do paciente da oferta. Nada foi agendado. Confira com o paciente.",
    )
}

/// Registra o aceite do paciente (dois identificadores) e cria o agendamento no horário oferecido.
#[utoipa::path(post, operation_id = "aceitar_oferta", path = "/v1/ofertas/{id}/aceite", tag = "agenda",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = PedidoAceite,
    responses((status = 200, body = Oferta), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc, description = "oferta-indisponivel, horario-ocupado, guia-em-uso"),
              (status = 422, body = crate::openapi::ProblemaDoc, description = "identificacao-nao-confere, senha-vencida, saldo-esgotado")))]
pub async fn aceitar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoAceite>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::EsperaGerir)?;
    let nome = NomeCompleto::novo(&p.confirmacao.nome).map_err(|_| identificacao_nao_confere())?;
    let nascimento = DataNascimento::nova(p.confirmacao.nascimento, tempo::hoje_local())
        .map_err(|_| identificacao_nao_confere())?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, ROTA_ACEITE, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let o = sqlx::query!(
        "SELECT o.codigo, o.sala_id, o.inicio, o.fim, e.id AS espera_id, e.codigo AS espera_codigo, e.paciente_id,
                e.exame_tuss, e.guia_id, p.nome, p.nascimento
           FROM agenda.oferta o JOIN agenda.espera e ON e.id = o.espera_id JOIN identidade.paciente p ON p.id = e.paciente_id
          WHERE o.id = $1",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(nao_encontrada)?;
    // Dois identificadores (nome e nascimento), sem dizer qual divergiu (SEC-DAD-4).
    let cadastro_nome = NomeCompleto::novo(&o.nome).map_err(ErroApi::interno)?;
    let cadastro_nasc =
        DataNascimento::nova(o.nascimento, tempo::hoje_local()).map_err(ErroApi::interno)?;
    if !(cadastro_nome.confere_com(&nome) && cadastro_nasc == nascimento) {
        return Err(identificacao_nao_confere());
    }
    // Trava a sala primeiro (mesma ordem de quem agenda e oferece), depois pega a oferta de forma
    // atômica: dois aceites ao mesmo tempo, só um muda a linha (G2).
    sqlx::query_scalar!(
        "SELECT id FROM agenda.sala WHERE id = $1 FOR UPDATE",
        o.sala_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let agendamento_id = novo_id();
    let pegou = sqlx::query!(
        "UPDATE agenda.oferta SET situacao = 'aceita', agendamento_id = $2, respondida_em = now(), respondida_por = $3,
                versao = versao + 1
          WHERE id = $1 AND situacao = 'aguardando' AND expira_em > now()",
        id,
        agendamento_id,
        usuario.id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if pegou != 1 {
        let atual = carregar(&mut tx, id).await?;
        return Err(ErroApi::conflito(
            "oferta-indisponivel",
            &format!(
                "Esta oferta não está mais aguardando resposta (situação: {}). Nada foi agendado.",
                atual.situacao
            ),
        )
        .com_extra("situacao", json!(atual.situacao)));
    }
    let exame = sqlx::query!(
        "SELECT nome, requer_contraste FROM agenda.procedimento WHERE tuss = $1",
        o.exame_tuss
    )
    .fetch_one(&mut *tx)
    .await?;
    let ano = tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero('AGD', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::interno("contador AGD sem valor"))?;
    let codigo = CodigoLegivel::novo("AGD", ano, n);
    let situacao = if o.guia_id.is_some() {
        "confirmado"
    } else {
        "agendado"
    };
    sqlx::query!(
        "INSERT INTO agenda.agendamento (id, codigo, sala_id, paciente_id, inicio, fim, exame_tuss, exame_nome, situacao,
                requer_contraste, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'lista_espera', $12, $13, $14)",
        agendamento_id, codigo.como_str(), o.sala_id, o.paciente_id, o.inicio, o.fim, o.exame_tuss, exame.nome,
        situacao, exame.requer_contraste, usuario.id, estacao, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d)
            if matches!(d.constraint(), Some("agendamento_horario_sala_unico" | "horario_bloqueado" | "horario_em_oferta")) =>
        {
            ErroApi::conflito("horario-ocupado", "O horário da oferta não está mais livre. Nada foi agendado. Ofereça outro horário ao paciente.")
        }
        _ => ErroApi::from(e),
    })?;
    sqlx::query!(
        "INSERT INTO agenda.agendamento_procedimento (agendamento_id, tuss, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, 'lista_espera', $4, $5, $6)",
        agendamento_id, o.exame_tuss, usuario.id, estacao, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    let guia_codigo = match o.guia_id {
        Some(g) => {
            let dia = o.inicio.with_timezone(&tempo::FUSO).date_naive();
            let c = crate::rotas::agendamentos::consumir_guia(
                &mut tx,
                g,
                o.paciente_id,
                dia,
                std::slice::from_ref(&o.exame_tuss),
                agendamento_id,
            )
            .await?;
            sqlx::query!(
                "UPDATE agenda.agendamento SET guia_id = $2 WHERE id = $1",
                agendamento_id,
                g
            )
            .execute(&mut *tx)
            .await?;
            Some(c)
        }
        None => None,
    };
    let encaixou = sqlx::query!(
        "UPDATE agenda.espera SET situacao = 'encaixado', agendamento_id = $2, versao = versao + 1
          WHERE id = $1 AND situacao = 'ofertado'",
        o.espera_id,
        agendamento_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if encaixou != 1 {
        return Err(ErroApi::interno(
            "pedido de espera fora da situação ofertado no aceite",
        ));
    }
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "agenda", tipo: "oferta_aceita", registro_id: id, registro_codigo: Some(&o.codigo),
        dados: json!({ "agendamento": codigo.como_str(), "espera": o.espera_codigo, "identificadores": "nome e nascimento conferidos", "estacao": estacao }),
        criado_por: Some(usuario.id), origem: "balcao", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "agenda", tipo: "agendamento_criado", registro_id: agendamento_id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "situacao": situacao, "guia": guia_codigo, "procedimentos": [o.exame_tuss], "canal": "lista_espera",
                       "oferta": o.codigo, "estacao": estacao }),
        criado_por: Some(usuario.id), origem: "lista_espera", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let oferta = carregar(&mut tx, id).await?;
    let corpo = serde_json::to_value(&oferta).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA_ACEITE, chave, 200, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::OK, Json(corpo)).into_response())
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoRecusa {
    pub motivo: String,
}

/// Registra a recusa do paciente (com motivo): ele volta à fila e o horário vai ao próximo.
#[utoipa::path(post, operation_id = "recusar_oferta", path = "/v1/ofertas/{id}/recusa", tag = "agenda",
    params(("id" = Uuid, Path)), request_body = PedidoRecusa,
    responses((status = 200, body = Oferta), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc, description = "oferta-indisponivel"),
              (status = 422, body = crate::openapi::ProblemaDoc, description = "motivo-invalido")))]
pub async fn recusar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoRecusa>,
) -> Result<Json<Oferta>, ErroApi> {
    usuario.exigir(Ato::EsperaGerir)?;
    let motivo = p.motivo.trim().to_owned();
    if !(3..=300).contains(&motivo.chars().count()) {
        return Err(ErroApi::invalido("motivo-invalido", "Escreva o motivo da recusa (de 3 a 300 letras), por exemplo \"não pode nesse horário\". Nada foi alterado."));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let recusada = sqlx::query!(
        "UPDATE agenda.oferta SET situacao = 'recusada', motivo = $2, respondida_em = now(), respondida_por = $3,
                versao = versao + 1
          WHERE id = $1 AND situacao = 'aguardando' RETURNING codigo, espera_id, sala_id, inicio, fim",
        id,
        motivo,
        usuario.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(r) = recusada else {
        let atual = carregar(&mut tx, id).await?;
        return Err(ErroApi::conflito(
            "oferta-indisponivel",
            &format!(
                "Esta oferta não está mais aguardando resposta (situação: {}). Nada foi alterado.",
                atual.situacao
            ),
        ));
    };
    // O paciente volta à fila na mesma posição (a data do pedido não muda).
    sqlx::query!(
        "UPDATE agenda.espera SET situacao = 'aguardando', versao = versao + 1 WHERE id = $1 AND situacao = 'ofertado'",
        r.espera_id
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "agenda",
            tipo: "oferta_recusada",
            registro_id: id,
            registro_codigo: Some(&r.codigo),
            dados: json!({ "motivo": motivo }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    if r.inicio > tempo::agora() + Duration::minutes(DURACAO_MIN) {
        vagas::enfileirar_vaga(
            &mut tx,
            usuario.unidade_id,
            id,
            r.sala_id,
            r.inicio,
            r.fim,
            "oferta_sem_aceite",
            &correlacao,
        )
        .await?;
    }
    let oferta = carregar(&mut tx, id).await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok(Json(oferta))
}
