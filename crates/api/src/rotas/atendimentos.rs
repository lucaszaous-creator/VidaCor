//! Check-in com dois identificadores e documento com foto (docs/27 FC-CHK, docs/30 § 1).
//! O check-in emite a senha e a pulseira; a worklist só sai no bipe da pulseira na sala (FC-PUL-01).

use crate::atos::Ato;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::PacienteResumo;
use crate::sessao::Usuario;
use crate::{banco, tempo};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel, DataNascimento, NomeCompleto};
use vidacor_dominio_recepcao::{
    conferir_checkin, AgendamentoParaCheckin, ConfirmacaoIdentidade, ErroCheckin,
    SituacaoAgendamento,
};

const ROTA_CHECKIN: &str = "POST /v1/atendimentos";

#[derive(Deserialize, ToSchema)]
pub struct Confirmacao {
    /// Nome completo dito pelo paciente, como no documento.
    pub nome: Option<String>,
    pub nascimento: Option<NaiveDate>,
    /// A recepção conferiu o documento com foto.
    pub documento_com_foto: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoCheckin {
    pub agendamento_id: Uuid,
    pub confirmacao: Confirmacao,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct EstudoResumo {
    pub id: Uuid,
    pub study_uid: String,
    pub imagens: i32,
    pub recebido_em: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct Etiqueta {
    pub codigo: String,
    /// pulseira.
    pub tipo: String,
    /// valida ou invalidada.
    pub situacao: String,
    pub motivo_invalidacao: Option<String>,
    pub substituida_por: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct Atendimento {
    pub id: Uuid,
    pub codigo: String,
    pub accession: String,
    pub agendamento_id: Uuid,
    pub paciente: PacienteResumo,
    pub sala: crate::rotas::agenda::Sala,
    /// aguardando_sala, em_exame, exame_concluido ou cancelado.
    pub situacao: String,
    pub senha: Option<String>,
    pub pulseira: Option<Etiqueta>,
    pub requer_contraste: bool,
    pub triagem: Option<crate::rotas::triagem::Triagem>,
    /// Situação do laudo, sem conteúdo (a recepção vê só isto, FC-ACE-06).
    pub laudo_situacao: Option<String>,
    pub criado_em: DateTime<Utc>,
    pub estudo: Option<EstudoResumo>,
}

fn identificacao_nao_confere() -> ErroApi {
    ErroApi::invalido(
        "identificacao-nao-confere",
        "Nome ou data de nascimento não conferem com o cadastro. O check-in não foi feito. Confira o documento do paciente; se o cadastro estiver errado, peça a correção antes de continuar.",
    )
}

fn ja_realizado() -> ErroApi {
    ErroApi::conflito(
        "checkin-ja-realizado",
        "O check-in deste agendamento já foi feito. Nada foi alterado. Abra o atendimento na agenda do dia.",
    )
}

fn incompleta(falta: &str) -> ErroApi {
    ErroApi::invalido(
        "identificacao-incompleta",
        &format!("Falta conferir {falta}. O check-in não foi feito e nenhuma senha ou pulseira foi emitida. Confira com o paciente e o documento."),
    )
}

/// Cria uma pulseira válida para o atendimento (código ETQ único).
///
/// # Errors
/// Falha de banco.
pub async fn emitir_pulseira(
    conexao: &mut PgConnection,
    atendimento_id: Uuid,
    paciente_id: Uuid,
    unidade_id: Uuid,
    usuario_id: Uuid,
    estacao: Option<&str>,
    correlacao: &str,
) -> Result<Etiqueta, ErroApi> {
    let ano = tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero('ETQ', $1)", ano)
        .fetch_one(&mut *conexao)
        .await?
        .ok_or_else(|| ErroApi::interno("contador ETQ sem valor"))?;
    let codigo = CodigoLegivel::novo("ETQ", ano, n);
    let id = novo_id();
    sqlx::query!(
        "INSERT INTO recepcao.etiqueta
           (id, codigo, tipo, atendimento_id, paciente_id, situacao, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, 'pulseira', $3, $4, 'valida', $5, 'balcao', $6, $7, $8)",
        id,
        codigo.como_str(),
        atendimento_id,
        paciente_id,
        usuario_id,
        estacao,
        unidade_id,
        correlacao
    )
    .execute(&mut *conexao)
    .await?;
    lastro::registrar(
        conexao,
        NovoEvento {
            dominio: "recepcao",
            tipo: "etiqueta_emitida",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "tipo": "pulseira", "estacao": estacao }),
            criado_por: Some(usuario_id),
            origem: "balcao",
            unidade_id,
            correlacao,
        },
    )
    .await?;
    Ok(Etiqueta {
        codigo: codigo.to_string(),
        tipo: "pulseira".into(),
        situacao: "valida".into(),
        motivo_invalidacao: None,
        substituida_por: None,
    })
}

/// Faz o check-in de um agendamento conferindo nome completo, nascimento e documento com foto.
#[utoipa::path(post, path = "/v1/atendimentos", tag = "recepcao", request_body = PedidoCheckin,
    params(("Idempotency-Key" = Uuid, Header), ("X-CSRF-Token" = String, Header), ("X-Estacao" = Option<String>, Header)),
    responses(
        (status = 201, body = Atendimento),
        (status = 400, body = crate::openapi::ProblemaDoc, description = "idempotency-key-ausente"),
        (status = 401, body = crate::openapi::ProblemaDoc),
        (status = 403, body = crate::openapi::ProblemaDoc),
        (status = 404, body = crate::openapi::ProblemaDoc, description = "agendamento-nao-encontrado"),
        (status = 409, body = crate::openapi::ProblemaDoc, description = "checkin-ja-realizado ou situacao-nao-permite-checkin"),
        (status = 422, body = crate::openapi::ProblemaDoc, description = "identificacao-incompleta ou identificacao-nao-confere")))]
pub async fn checkin(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(pedido): Corpo<PedidoCheckin>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::RecepcaoCheckin)?;
    let c = pedido.confirmacao;
    let nome_texto = c
        .nome
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| incompleta("o nome completo"))?;
    let nascimento_valor = c
        .nascimento
        .ok_or_else(|| incompleta("a data de nascimento"))?;
    if c.documento_com_foto != Some(true) {
        return Err(incompleta("o documento com foto"));
    }
    let nome = NomeCompleto::novo(&nome_texto).map_err(|_| {
        ErroApi::invalido(
            "nome-incompleto",
            "Informe o nome completo do paciente, como no documento. O check-in não foi feito.",
        )
    })?;
    let nascimento = DataNascimento::nova(nascimento_valor, tempo::hoje_local())
        .map_err(|_| identificacao_nao_confere())?;
    let correlacao = crate::trace::atual().unwrap_or_default();

    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, ROTA_CHECKIN, chave).await?
    {
        tx.commit().await?;
        let status = StatusCode::from_u16(status).unwrap_or(StatusCode::OK);
        return Ok((status, Json(corpo)).into_response());
    }

    // Trava o agendamento: duas recepções no mesmo paciente esperam uma pela outra (G2).
    let ag = sqlx::query!(
        r#"SELECT a.id, a.codigo, a.situacao, a.sala_id, a.unidade_id, a.requer_contraste, a.medico_id,
                  s.nome AS sala_nome, p.id AS paciente_id, p.nome, p.nascimento
             FROM agenda.agendamento a
             JOIN agenda.sala s ON s.id = a.sala_id
             JOIN identidade.paciente p ON p.id = a.paciente_id
            WHERE a.id = $1
            FOR UPDATE OF a"#,
        pedido.agendamento_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "agendamento-nao-encontrado",
            "Este agendamento não existe ou não é desta unidade. Nada foi alterado. Atualize a agenda do dia.",
        )
    })?;

    let situacao = SituacaoAgendamento::de_str(&ag.situacao).map_err(ErroApi::interno)?;
    if situacao == SituacaoAgendamento::EmAtendimento {
        return Err(ja_realizado());
    }
    let cadastro = AgendamentoParaCheckin {
        situacao,
        nome_cadastro: NomeCompleto::novo(&ag.nome).map_err(ErroApi::interno)?,
        nascimento_cadastro: DataNascimento::nova(ag.nascimento, tempo::hoje_local())
            .map_err(ErroApi::interno)?,
    };
    conferir_checkin(&cadastro, &ConfirmacaoIdentidade { nome, nascimento }).map_err(|e| match e {
        ErroCheckin::IdentificacaoNaoConfere => identificacao_nao_confere(),
        ErroCheckin::SituacaoNaoPermite(_) => ErroApi::conflito(
            "situacao-nao-permite-checkin",
            "A situação deste agendamento não permite check-in. Nada foi alterado. Confira com a coordenação da recepção.",
        ),
    })?;

    let ano = tempo::ano_local();
    let n_atd = sqlx::query_scalar!("SELECT comum.proximo_numero('ATD', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::interno("contador ATD sem valor"))?;
    let n_acc = sqlx::query_scalar!("SELECT comum.proximo_numero('ACC', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::interno("contador ACC sem valor"))?;
    let n_senha = sqlx::query_scalar!(
        "SELECT recepcao.proxima_senha($1, $2)",
        ag.unidade_id,
        tempo::hoje_local()
    )
    .fetch_one(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::interno("senha sem valor"))?;
    let id = novo_id();
    let codigo = CodigoLegivel::novo("ATD", ano, n_atd);
    let accession = CodigoLegivel::novo("ACC", ano, n_acc);
    let senha = format!("A{n_senha:03}");

    sqlx::query!(
        "INSERT INTO recepcao.atendimento
           (id, codigo, accession, agendamento_id, paciente_id, sala_id, situacao, senha, documento_conferido,
            criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'aguardando_sala', $7, true, $8, 'balcao', $9, $10, $11)",
        id,
        codigo.como_str(),
        accession.como_str(),
        ag.id,
        ag.paciente_id,
        ag.sala_id,
        senha,
        usuario.id,
        estacao,
        ag.unidade_id,
        correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ja_realizado(),
        _ => ErroApi::from(e),
    })?;

    // Operação atômica conferindo as linhas afetadas (G2, segunda barreira).
    let alterou = sqlx::query!(
        "UPDATE agenda.agendamento SET situacao = 'em_atendimento', versao = versao + 1
          WHERE id = $1 AND situacao IN ('agendado', 'confirmado')",
        ag.id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if alterou != 1 {
        return Err(ja_realizado());
    }

    let pulseira = emitir_pulseira(
        &mut tx,
        id,
        ag.paciente_id,
        ag.unidade_id,
        usuario.id,
        estacao.as_deref(),
        &correlacao,
    )
    .await?;

    // Vínculo de cuidado do médico do agendamento (docs/18 § 6 item 1).
    if let Some(medico) = ag.medico_id {
        crate::vinculo::registrar(
            &mut tx,
            medico,
            ag.paciente_id,
            ag.unidade_id,
            "agendamento",
            &correlacao,
        )
        .await?;
    }

    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "recepcao",
            tipo: "checkin_realizado",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({
                "agendamento": ag.codigo,
                "accession": accession.como_str(),
                "senha": senha,
                "pulseira": pulseira.codigo,
                "conferencias": ["nome", "nascimento", "documento_com_foto"],
                "estacao": estacao
            }),
            criado_por: Some(usuario.id),
            origem: "balcao",
            unidade_id: ag.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;

    let resposta = Atendimento {
        id,
        codigo: codigo.to_string(),
        accession: accession.to_string(),
        agendamento_id: ag.id,
        paciente: PacienteResumo {
            id: ag.paciente_id,
            nome: ag.nome,
            nascimento: ag.nascimento,
        },
        sala: crate::rotas::agenda::Sala {
            id: ag.sala_id,
            nome: ag.sala_nome,
        },
        situacao: "aguardando_sala".to_owned(),
        senha: Some(senha),
        pulseira: Some(pulseira),
        requer_contraste: ag.requer_contraste,
        triagem: None,
        laudo_situacao: None,
        criado_em: Utc::now(),
        estudo: None,
    };
    let corpo: Value = serde_json::to_value(&resposta).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA_CHECKIN, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Carrega o atendimento completo dentro de uma transação com contexto (RLS filtra a unidade).
///
/// # Errors
/// Falha de banco.
pub async fn carregar(
    conexao: &mut PgConnection,
    id: Uuid,
) -> Result<Option<Atendimento>, ErroApi> {
    let Some(l) = sqlx::query!(
        r#"SELECT t.id, t.codigo, t.accession, t.agendamento_id, t.situacao, t.senha, t.criado_em,
                  s.id AS sala_id, s.nome AS sala_nome, a.requer_contraste,
                  p.id AS paciente_id, p.nome, p.nascimento,
                  e.id AS "estudo_id?", e.study_uid AS "study_uid?", e.imagens AS "imagens?", e.recebido_em AS "recebido_em?",
                  g.codigo AS "pulseira_codigo?", g.situacao AS "pulseira_situacao?"
             FROM recepcao.atendimento t
             JOIN agenda.agendamento a ON a.id = t.agendamento_id
             JOIN agenda.sala s ON s.id = t.sala_id
             JOIN identidade.paciente p ON p.id = t.paciente_id
             LEFT JOIN exames.estudo e ON e.atendimento_id = t.id AND e.conciliado
             LEFT JOIN recepcao.etiqueta g ON g.atendimento_id = t.id AND g.situacao = 'valida'
            WHERE t.id = $1"#,
        id
    )
    .fetch_optional(&mut *conexao)
    .await?
    else {
        return Ok(None);
    };
    let estudo = match (l.estudo_id, l.study_uid, l.imagens, l.recebido_em) {
        (Some(id), Some(study_uid), Some(imagens), Some(recebido_em)) => Some(EstudoResumo {
            id,
            study_uid,
            imagens,
            recebido_em,
        }),
        _ => None,
    };
    let pulseira = l.pulseira_codigo.map(|codigo| Etiqueta {
        codigo,
        tipo: "pulseira".into(),
        situacao: l.pulseira_situacao.unwrap_or_default(),
        motivo_invalidacao: None,
        substituida_por: None,
    });
    let triagem = crate::rotas::triagem::carregar(&mut *conexao, l.id).await?;
    // Só a palavra da situação, sem conteúdo, acima do RLS do laudo (FC-ACE-06).
    let laudo_situacao = sqlx::query_scalar!("SELECT laudos.situacao_do_atendimento($1)", l.id)
        .fetch_one(&mut *conexao)
        .await?;
    Ok(Some(Atendimento {
        id: l.id,
        codigo: l.codigo,
        accession: l.accession,
        agendamento_id: l.agendamento_id,
        paciente: PacienteResumo {
            id: l.paciente_id,
            nome: l.nome,
            nascimento: l.nascimento,
        },
        sala: crate::rotas::agenda::Sala {
            id: l.sala_id,
            nome: l.sala_nome,
        },
        situacao: l.situacao,
        senha: l.senha,
        pulseira,
        requer_contraste: l.requer_contraste,
        triagem,
        laudo_situacao,
        criado_em: l.criado_em,
        estudo,
    }))
}

/// Um atendimento da unidade, com pulseira, triagem, situação do laudo e estudo recebido.
#[utoipa::path(get, operation_id = "obter_atendimento", path = "/v1/atendimentos/{id}", tag = "recepcao", params(("id" = Uuid, Path)),
    responses((status = 200, body = Atendimento), (status = 401, body = crate::openapi::ProblemaDoc),
              (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn obter(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<Atendimento>, ErroApi> {
    usuario.exigir(Ato::RecepcaoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    // RLS filtra por unidade e perfil: atendimento de outra unidade simplesmente não aparece.
    let atendimento = carregar(&mut tx, id).await?.ok_or_else(|| {
        ErroApi::nao_encontrado(
            "atendimento-nao-encontrado",
            "Este atendimento não existe ou não é desta unidade. Nada foi alterado. Volte para a agenda do dia.",
        )
    })?;
    tx.commit().await?;
    Ok(Json(atendimento))
}
