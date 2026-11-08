//! Laudo: fila, rascunho, assinatura, retificação, "conversar antes", publicação e acesso de
//! emergência (docs/27 FC-LAU e FC-ACE-03 a 07, docs/30 § 4). Registro clínico nunca é apagado nem
//! alterado depois de assinado: correção gera nova versão com motivo (CLAUDE.md).

use crate::atos::Ato;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::PacienteResumo;
use crate::sessao::Usuario;
use crate::{banco, tempo};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};

#[derive(Serialize, ToSchema)]
pub struct Versao {
    pub numero: i32,
    pub texto: String,
    /// rascunho, assinada ou substituida.
    pub situacao: String,
    pub motivo: Option<String>,
    pub assinada_por: Option<Uuid>,
    pub assinada_em: Option<DateTime<Utc>>,
    pub hash_documento: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct Laudo {
    pub id: Uuid,
    pub codigo: String,
    pub paciente: PacienteResumo,
    pub exame: String,
    /// rascunho, assinado ou retificacao_em_rascunho.
    pub situacao: String,
    pub conversar_antes: bool,
    pub publicado_no_portal: bool,
    pub versao_vigente: Option<i32>,
    pub medico_atribuido_id: Option<Uuid>,
    pub versoes: Vec<Versao>,
    pub estudo: Option<crate::rotas::atendimentos::EstudoResumo>,
}

fn sem_vinculo() -> ErroApi {
    ErroApi::novo(
        StatusCode::FORBIDDEN,
        "sem-vinculo-de-cuidado",
        "Sem vínculo com o paciente",
        "Você não tem vínculo de cuidado com este paciente. O laudo não foi aberto. Em urgência, use o acesso de emergência com justificativa: ele fica registrado e é revisado.",
    )
}

fn nao_encontrado() -> ErroApi {
    ErroApi::nao_encontrado(
        "laudo-nao-encontrado",
        "Este laudo não existe nesta unidade. Nada foi aberto.",
    )
}

/// Erro de leitura negada pelo RLS: falta de vínculo (403) ou laudo inexistente nesta unidade (404).
pub(crate) fn negacao(laudo_existe: bool) -> ErroApi {
    if laudo_existe {
        sem_vinculo()
    } else {
        nao_encontrado()
    }
}

pub(crate) async fn auditar(
    conexao: &mut PgConnection,
    quem: Uuid,
    laudo: Uuid,
    resultado: &str,
    justificativa: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO auditoria.acesso (id, quem, quem_tipo, recurso, recurso_id, resultado, justificativa, correlacao)
         VALUES ($1, $2, 'usuario', 'laudo', $3, $4, $5, $6)",
        novo_id(), quem, laudo, resultado, justificativa, crate::trace::atual().unwrap_or_default()
    )
    .execute(conexao)
    .await?;
    Ok(())
}

/// Carrega o laudo visível no contexto (RLS: vínculo ou acesso de emergência).
pub(crate) async fn carregar(
    conexao: &mut PgConnection,
    id: Uuid,
) -> Result<Option<Laudo>, ErroApi> {
    let Some(l) = sqlx::query!(
        r#"SELECT l.id, l.codigo, l.situacao, l.conversar_antes, l.versao_vigente, l.medico_atribuido_id,
                  p.id AS paciente_id, p.nome, p.nascimento, ag.exame_nome,
                  e.id AS estudo_id, e.study_uid, e.imagens, e.recebido_em,
                  EXISTS (SELECT 1 FROM portal.publicacao_existe(l.id)) AS "publicado!"
             FROM laudos.laudo l
             JOIN identidade.paciente p ON p.id = l.paciente_id
             JOIN recepcao.atendimento t ON t.id = l.atendimento_id
             JOIN agenda.agendamento ag ON ag.id = t.agendamento_id
             JOIN exames.estudo e ON e.id = l.estudo_id
            WHERE l.id = $1"#,
        id
    )
    .fetch_optional(&mut *conexao)
    .await?
    else {
        return Ok(None);
    };
    let versoes = sqlx::query_as!(
        Versao,
        "SELECT numero, texto, situacao, motivo, assinada_por, assinada_em, hash_documento
           FROM laudos.versao WHERE laudo_id = $1 ORDER BY numero",
        l.id
    )
    .fetch_all(&mut *conexao)
    .await?;
    Ok(Some(Laudo {
        id: l.id,
        codigo: l.codigo,
        paciente: PacienteResumo {
            id: l.paciente_id,
            nome: l.nome,
            nascimento: l.nascimento,
        },
        exame: l.exame_nome,
        situacao: l.situacao,
        conversar_antes: l.conversar_antes,
        publicado_no_portal: l.publicado,
        versao_vigente: l.versao_vigente,
        medico_atribuido_id: l.medico_atribuido_id,
        versoes,
        estudo: Some(crate::rotas::atendimentos::EstudoResumo {
            id: l.estudo_id,
            study_uid: l.study_uid,
            imagens: l.imagens,
            recebido_em: l.recebido_em,
        }),
    }))
}

/// Busca o laudo para um ato do médico atribuído: 404 se não existe, 403 sem vínculo ou sem atribuição.
pub(crate) async fn do_medico_atribuido(
    conexao: &mut PgConnection,
    usuario: &Usuario,
    id: Uuid,
) -> Result<Laudo, ErroApi> {
    let laudo = match carregar(conexao, id).await? {
        Some(l) => l,
        None => {
            let existe = sqlx::query_scalar!("SELECT laudos.paciente_do_laudo($1)", id)
                .fetch_one(&mut *conexao)
                .await?;
            return Err(if existe.is_some() {
                sem_vinculo()
            } else {
                nao_encontrado()
            });
        }
    };
    if laudo.medico_atribuido_id != Some(usuario.id) {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "laudo-de-outro-medico",
            "Laudo atribuído a outro médico",
            "Este laudo está atribuído a outro médico. Nada foi alterado. Peça a reatribuição à coordenação, se for o caso.",
        ));
    }
    Ok(laudo)
}

#[derive(Serialize, ToSchema)]
pub struct ItemFilaLaudo {
    pub id: Uuid,
    pub codigo: String,
    pub paciente: PacienteResumo,
    pub exame: String,
    pub situacao: String,
    pub conversar_antes: bool,
    pub recebido_em: DateTime<Utc>,
}

/// Fila de laudos do médico: só estudos conciliados atribuídos a ele (FC-LAU-09).
#[utoipa::path(get, operation_id = "fila_de_laudos", path = "/v1/laudos/fila", tag = "laudos", responses((status = 200, body = Vec<ItemFilaLaudo>)))]
pub async fn fila(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<ItemFilaLaudo>>, ErroApi> {
    usuario.exigir(Ato::LaudoEscrever)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let linhas = sqlx::query!(
        r#"SELECT l.id, l.codigo, l.situacao, l.conversar_antes, p.id AS paciente_id, p.nome, p.nascimento, ag.exame_nome, e.recebido_em
             FROM laudos.laudo l
             JOIN exames.estudo e ON e.id = l.estudo_id AND e.conciliado
             JOIN identidade.paciente p ON p.id = l.paciente_id
             JOIN recepcao.atendimento t ON t.id = l.atendimento_id
             JOIN agenda.agendamento ag ON ag.id = t.agendamento_id
            WHERE l.medico_atribuido_id = $1 AND l.situacao IN ('rascunho', 'retificacao_em_rascunho')
            ORDER BY e.recebido_em LIMIT 200"#,
        usuario.id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        linhas
            .into_iter()
            .map(|l| ItemFilaLaudo {
                id: l.id,
                codigo: l.codigo,
                paciente: PacienteResumo {
                    id: l.paciente_id,
                    nome: l.nome,
                    nascimento: l.nascimento,
                },
                exame: l.exame_nome,
                situacao: l.situacao,
                conversar_antes: l.conversar_antes,
                recebido_em: l.recebido_em,
            })
            .collect(),
    ))
}

/// Lê o laudo (médico com vínculo de cuidado ou acesso de emergência ativo). Toda leitura é auditada.
#[utoipa::path(get, operation_id = "obter_laudo", path = "/v1/laudos/{id}", tag = "laudos", params(("id" = Uuid, Path)),
    responses((status = 200, body = Laudo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn obter(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<Laudo>, ErroApi> {
    usuario.exigir(Ato::LaudoLer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    match carregar(&mut tx, id).await? {
        Some(laudo) => {
            let emergencia = !crate::vinculo::tem_vinculo(&mut tx, laudo.paciente.id).await?;
            auditar(
                &mut tx,
                usuario.id,
                id,
                if emergencia {
                    "emergencia"
                } else {
                    "permitido"
                },
                None,
            )
            .await?;
            tx.commit().await?;
            Ok(Json(laudo))
        }
        None => {
            let erro = negado(&mut tx, usuario.id, id).await?;
            tx.commit().await?;
            Err(erro)
        }
    }
}

/// Leitura negada pelo RLS: audita a tentativa e diz se é falta de vínculo (403, com oferta de
/// acesso de emergência) ou laudo inexistente nesta unidade (404).
pub(crate) async fn negado(
    conexao: &mut PgConnection,
    quem: Uuid,
    id: Uuid,
) -> Result<ErroApi, ErroApi> {
    let existe = sqlx::query_scalar!("SELECT laudos.paciente_do_laudo($1)", id)
        .fetch_one(&mut *conexao)
        .await?;
    auditar(conexao, quem, id, "negado", None).await?;
    Ok(if existe.is_some() {
        sem_vinculo()
    } else {
        nao_encontrado()
    })
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoRascunho {
    pub texto: String,
    pub versao_numero: i32,
}

/// Salva o rascunho (só o médico atribuído; versão assinada não muda: 409).
#[utoipa::path(put, path = "/v1/laudos/{id}/rascunho", tag = "laudos", params(("id" = Uuid, Path)), request_body = PedidoRascunho,
    responses((status = 200, body = Laudo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn salvar_rascunho(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoRascunho>,
) -> Result<Json<Laudo>, ErroApi> {
    usuario.exigir(Ato::LaudoEscrever)?;
    if p.texto.chars().count() > 100_000 {
        return Err(ErroApi::invalido(
            "texto-longo",
            "O texto passou de 100 mil caracteres. Nada foi salvo.",
        ));
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    do_medico_atribuido(&mut tx, &usuario, id).await?;
    let alterou = sqlx::query!(
        "UPDATE laudos.versao SET texto = $3, versao = versao + 1 WHERE laudo_id = $1 AND numero = $2 AND situacao = 'rascunho'",
        id, p.versao_numero, p.texto
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if alterou != 1 {
        return Err(ErroApi::conflito(
            "laudo-assinado",
            "Esta versão do laudo já foi assinada e não pode ser alterada. Nada foi salvo. Para corrigir, faça uma retificação com motivo.",
        ));
    }
    let laudo = carregar(&mut tx, id).await?.ok_or_else(nao_encontrado)?;
    tx.commit().await?;
    Ok(Json(laudo))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoAssinatura {
    pub pin: String,
}

/// Assina a versão em rascunho com o certificado do médico (reautenticação e PIN). A versão anterior,
/// se houver, passa a "substituída". Publica no portal, salvo "conversar antes".
#[utoipa::path(post, path = "/v1/laudos/{id}/assinatura", tag = "laudos", params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)),
    request_body = PedidoAssinatura,
    responses((status = 200, body = Laudo), (status = 401, body = crate::openapi::ProblemaDoc), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc), (status = 503, body = crate::openapi::ProblemaDoc)))]
pub async fn assinar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoAssinatura>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::LaudoEscrever)?;
    usuario.exigir_reautenticacao()?;
    let assinador = estado.assinador.clone().ok_or_else(|| ErroApi::novo(
        StatusCode::SERVICE_UNAVAILABLE, "assinatura-indisponivel", "Assinatura indisponível",
        "O serviço de assinatura digital não está disponível neste servidor. O laudo continua em rascunho. Avise a TI.",
    ))?;
    let rota = format!("POST /v1/laudos/{id}/assinatura");
    let correlacao = crate::trace::atual().unwrap_or_default();

    // 1. Lê o rascunho e calcula o hash (fora da chamada externa).
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    tx.rollback().await?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let laudo = do_medico_atribuido(&mut tx, &usuario, id).await?;
    let rascunho = laudo.versoes.iter().find(|v| v.situacao == "rascunho").ok_or_else(|| ErroApi::conflito(
        "assinatura-concorrente", "Não há versão em rascunho para assinar: o laudo já foi assinado. Nada foi alterado.",
    ))?;
    if rascunho.texto.trim().is_empty() {
        return Err(ErroApi::invalido(
            "laudo-vazio",
            "O laudo está sem texto. Nada foi assinado.",
        ));
    }
    let numero = rascunho.numero;
    let texto = rascunho.texto.clone();
    // Achados estruturados entram no documento assinado (E2); sem achado, o hash é o de sempre.
    let achados = crate::rotas::achados::canonico_para_assinatura(&mut tx, id, numero).await?;
    let documento = if achados.is_empty() {
        texto.clone()
    } else {
        format!("{texto}\nVIDACOR-ACHADOS-1\n{achados}")
    };
    let hash = crate::laudos::hash_documento(
        &laudo.codigo,
        numero,
        &laudo.paciente.nome,
        &laudo.paciente.nascimento.to_string(),
        &laudo.exame,
        &documento,
    );
    tx.commit().await?;

    // 2. Chamada ao provedor de assinatura, sem transação aberta (docs/14 A2).
    let assinatura = assinador.assinar(usuario.id, &p.pin, &hash).await.map_err(|e| match e {
        crate::assinatura::ErroAssinatura::PinRecusado => ErroApi::invalido("pin-recusado", "O PIN do certificado foi recusado. O laudo continua em rascunho. Confira o PIN e tente de novo."),
        crate::assinatura::ErroAssinatura::Indisponivel(m) => ErroApi::interno(m),
    })?;

    // 3. Grava a assinatura se o rascunho ainda é o mesmo; a segunda assinatura simultânea perde (FC-LAU-07).
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let travado = sqlx::query!("SELECT unidade_id, conversar_antes, versao_vigente FROM laudos.laudo WHERE id = $1 FOR UPDATE", id)
        .fetch_one(&mut *tx)
        .await?;
    // Achado incluído ou removido enquanto o provedor assinava: o hash não confere mais.
    if crate::rotas::achados::canonico_para_assinatura(&mut tx, id, numero).await? != achados {
        return Err(ErroApi::conflito(
            "assinatura-concorrente",
            "Os achados do laudo mudaram enquanto você assinava. Nada foi assinado. Confira os achados e assine de novo.",
        ));
    }
    if let Some(anterior) = travado.versao_vigente {
        sqlx::query!("UPDATE laudos.versao SET situacao = 'substituida', versao = versao + 1 WHERE laudo_id = $1 AND numero = $2 AND situacao = 'assinada'", id, anterior)
            .execute(&mut *tx)
            .await?;
    }
    let assinou = sqlx::query!(
        "UPDATE laudos.versao SET situacao = 'assinada', assinada_por = $3, assinada_em = now(), certificado = $4,
                carimbo_tempo = $5, hash_documento = $6, versao = versao + 1
          WHERE laudo_id = $1 AND numero = $2 AND situacao = 'rascunho' AND texto = $7",
        id, numero, usuario.id, assinatura.certificado, assinatura.carimbo_tempo, hash, texto
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if assinou != 1 {
        return Err(ErroApi::conflito(
            "assinatura-concorrente",
            "O laudo foi assinado ou alterado por outra requisição enquanto você assinava. Nada foi alterado por esta. Abra o laudo de novo.",
        ));
    }
    sqlx::query!("UPDATE laudos.laudo SET situacao = 'assinado', versao_vigente = $2, versao = versao + 1 WHERE id = $1", id, numero)
        .execute(&mut *tx)
        .await?;
    crate::vinculo::registrar(
        &mut tx,
        usuario.id,
        laudo.paciente.id,
        travado.unidade_id,
        "laudo_assinado",
        &correlacao,
    )
    .await?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "laudos", tipo: "laudo_assinado", registro_id: id, registro_codigo: Some(&laudo.codigo),
        dados: json!({ "versao": numero, "hash_documento": hash, "certificado": assinatura.certificado, "carimbo_tempo": assinatura.carimbo_tempo, "reautenticado": true, "achados": achados.lines().count() }),
        criado_por: Some(usuario.id), origem: "consultorio", unidade_id: travado.unidade_id, correlacao: &correlacao,
    }).await?;
    if !travado.conversar_antes {
        enfileirar_publicacao(&mut tx, id, travado.unidade_id, &correlacao).await?;
    }
    let atualizado = carregar(&mut tx, id).await?.ok_or_else(nao_encontrado)?;
    let corpo: Value = serde_json::to_value(&atualizado).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 200, &corpo).await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok((StatusCode::OK, Json(corpo)).into_response())
}

async fn enfileirar_publicacao(
    conexao: &mut PgConnection,
    laudo_id: Uuid,
    unidade_id: Uuid,
    correlacao: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao)
         VALUES ($1, 'portal.publicar', $2, $3, $4, $5)",
        novo_id(), novo_id(), json!({ "laudo_id": laudo_id }), unidade_id, correlacao
    )
    .execute(conexao)
    .await?;
    Ok(())
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoRetificacao {
    pub texto: String,
    pub motivo: Option<String>,
}

/// Abre a retificação de um laudo assinado: nova versão em rascunho com motivo (FC-LAU-05, 06).
#[utoipa::path(post, path = "/v1/laudos/{id}/retificacao", tag = "laudos", params(("id" = Uuid, Path)), request_body = PedidoRetificacao,
    responses((status = 201, body = Laudo), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn retificar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoRetificacao>,
) -> Result<(StatusCode, Json<Laudo>), ErroApi> {
    usuario.exigir(Ato::LaudoEscrever)?;
    usuario.exigir_reautenticacao()?;
    let motivo = p.motivo.map(|m| m.trim().to_owned()).filter(|m| m.chars().count() >= 5).ok_or_else(|| {
        ErroApi::invalido("motivo-obrigatorio", "Informe o motivo da retificação (por exemplo, a medida que foi corrigida). Nada foi salvo.")
    })?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let laudo = do_medico_atribuido(&mut tx, &usuario, id).await?;
    if laudo.situacao != "assinado" {
        return Err(ErroApi::conflito("retificacao-indisponivel", "Só um laudo assinado e sem retificação em aberto pode ser retificado. Nada foi alterado."));
    }
    let proximo = laudo.versoes.iter().map(|v| v.numero).max().unwrap_or(0) + 1;
    let unidade = sqlx::query_scalar!("SELECT unidade_id FROM laudos.laudo WHERE id = $1", id)
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query!(
        "INSERT INTO laudos.versao (id, laudo_id, numero, texto, situacao, motivo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, 'rascunho', $5, $6, 'consultorio', $7, $8)",
        novo_id(), id, proximo, p.texto, motivo, usuario.id, unidade, correlacao
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("UPDATE laudos.laudo SET situacao = 'retificacao_em_rascunho', versao = versao + 1 WHERE id = $1", id).execute(&mut *tx).await?;
    // Os achados estruturados da versão vigente seguem para a retificação (E2, DEC-2026-11-08-64).
    let achados_copiados = match laudo.versao_vigente {
        Some(vigente) => {
            crate::rotas::achados::copiar_para_retificacao(
                &mut tx,
                id,
                vigente,
                proximo,
                &usuario,
                &correlacao,
            )
            .await?
        }
        None => 0,
    };
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "laudos",
            tipo: "retificacao_aberta",
            registro_id: id,
            registro_codigo: Some(&laudo.codigo),
            dados: json!({ "versao": proximo, "motivo": motivo, "achados_copiados": achados_copiados }),
            criado_por: Some(usuario.id),
            origem: "consultorio",
            unidade_id: unidade,
            correlacao: &correlacao,
        },
    )
    .await?;
    let atualizado = carregar(&mut tx, id).await?.ok_or_else(nao_encontrado)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(atualizado)))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoConversar {
    pub marcado: bool,
}

/// Marca ou desmarca "conversar antes de liberar" (resultado delicado, docs/16 R11).
#[utoipa::path(post, path = "/v1/laudos/{id}/conversar-antes", tag = "laudos", params(("id" = Uuid, Path)), request_body = PedidoConversar,
    responses((status = 200, body = Laudo), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn conversar_antes(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoConversar>,
) -> Result<Json<Laudo>, ErroApi> {
    usuario.exigir(Ato::LaudoEscrever)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let laudo = do_medico_atribuido(&mut tx, &usuario, id).await?;
    if laudo.publicado_no_portal {
        return Err(ErroApi::conflito(
            "ja-publicado",
            "Este laudo já está no portal. Nada foi alterado. Se precisar, faça uma retificação.",
        ));
    }
    let unidade = sqlx::query_scalar!("UPDATE laudos.laudo SET conversar_antes = $2, versao = versao + 1 WHERE id = $1 RETURNING unidade_id", id, p.marcado)
        .fetch_one(&mut *tx)
        .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "laudos",
            tipo: if p.marcado {
                "conversar_antes_marcado"
            } else {
                "conversar_antes_desmarcado"
            },
            registro_id: id,
            registro_codigo: Some(&laudo.codigo),
            dados: json!({}),
            criado_por: Some(usuario.id),
            origem: "consultorio",
            unidade_id: unidade,
            correlacao: &correlacao,
        },
    )
    .await?;
    let atualizado = carregar(&mut tx, id).await?.ok_or_else(nao_encontrado)?;
    tx.commit().await?;
    Ok(Json(atualizado))
}

/// Libera no portal depois da conversa com o paciente (FC-LAU-08).
#[utoipa::path(post, path = "/v1/laudos/{id}/liberacao", tag = "laudos", params(("id" = Uuid, Path)),
    responses((status = 200, body = Laudo), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn liberar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<Laudo>, ErroApi> {
    usuario.exigir(Ato::LaudoEscrever)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let laudo = do_medico_atribuido(&mut tx, &usuario, id).await?;
    if laudo.situacao != "assinado" || !laudo.conversar_antes {
        return Err(ErroApi::conflito("liberacao-indisponivel", "Só um laudo assinado marcado para conversar antes é liberado por aqui. Nada foi alterado."));
    }
    let unidade = sqlx::query_scalar!(
        "UPDATE laudos.laudo SET liberado_apos_conversa_por = $2, liberado_apos_conversa_em = now(), versao = versao + 1
          WHERE id = $1 AND liberado_apos_conversa_em IS NULL RETURNING unidade_id",
        id, usuario.id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::conflito("ja-liberado", "Este laudo já foi liberado. Nada foi alterado."))?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "laudos",
            tipo: "liberado_apos_conversa",
            registro_id: id,
            registro_codigo: Some(&laudo.codigo),
            dados: json!({}),
            criado_por: Some(usuario.id),
            origem: "consultorio",
            unidade_id: unidade,
            correlacao: &correlacao,
        },
    )
    .await?;
    enfileirar_publicacao(&mut tx, id, unidade, &correlacao).await?;
    let atualizado = carregar(&mut tx, id).await?.ok_or_else(nao_encontrado)?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok(Json(atualizado))
}

/// Publicação manual no portal: rascunho nunca é publicado (FC-LAU-02).
#[utoipa::path(post, path = "/v1/laudos/{id}/publicacao", tag = "laudos", params(("id" = Uuid, Path)),
    responses((status = 200, body = crate::rotas::respostas::Publicacao), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn publicar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ErroApi> {
    usuario.exigir(Ato::LaudoEscrever)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    do_medico_atribuido(&mut tx, &usuario, id).await?;
    tx.commit().await?;
    let nova = crate::laudos::publicar(&estado, id, usuario.unidade_id).await?;
    Ok(Json(crate::rotas::respostas::valor(
        &crate::rotas::respostas::Publicacao {
            publicado: true,
            nova_publicacao: nova,
        },
    )?))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoEmergencia {
    pub justificativa: String,
}

/// Acesso de emergência (break-glass) com justificativa: vale 1 hora, é auditado em destaque, avisa o
/// responsável clínico e entra na fila de revisão de 48 horas (FC-ACE-04).
#[utoipa::path(post, path = "/v1/laudos/{id}/acesso-emergencia", tag = "laudos", params(("id" = Uuid, Path)), request_body = PedidoEmergencia,
    responses((status = 201, body = crate::rotas::respostas::AcessoEmergencia), (status = 401, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn acesso_emergencia(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoEmergencia>,
) -> Result<(StatusCode, Json<Value>), ErroApi> {
    usuario.exigir(Ato::LaudoLer)?;
    usuario.exigir_reautenticacao()?;
    let justificativa = p.justificativa.trim().to_owned();
    if justificativa.chars().count() < 20 {
        return Err(ErroApi::invalido(
            "justificativa-curta",
            "Explique a urgência em pelo menos 20 caracteres. O acesso não foi aberto.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let paciente = sqlx::query_scalar!("SELECT laudos.paciente_do_laudo($1)", id)
        .fetch_one(&mut *tx)
        .await?
        .ok_or_else(nao_encontrado)?;
    let ano = tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero('EMG', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);
    let codigo = CodigoLegivel::novo("EMG", ano, n);
    let acesso = novo_id();
    let valido_ate = Utc::now() + Duration::hours(1);
    sqlx::query!(
        "INSERT INTO cuidado.acesso_emergencia (id, codigo, medico_id, paciente_id, justificativa, valido_ate, revisar_ate, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, now() + interval '48 hours', $3, 'consultorio', $7, $8)",
        acesso, codigo.como_str(), usuario.id, paciente, justificativa, valido_ate, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    auditar(&mut tx, usuario.id, id, "emergencia", Some(&justificativa)).await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "cuidado",
            tipo: "acesso_emergencia",
            registro_id: acesso,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "laudo": id, "valido_ate": valido_ate, "revisao_em_horas": 48 }),
            criado_por: Some(usuario.id),
            origem: "consultorio",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    sqlx::query!(
        "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
         VALUES ($1, 'aviso.acesso_emergencia', $2, $3, $4, $5, $1)",
        acesso, novo_id(), json!({ "acesso": codigo.como_str() }), usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(crate::rotas::respostas::valor(
            &crate::rotas::respostas::AcessoEmergencia {
                codigo: codigo.to_string(),
                valido_ate,
            },
        )?),
    ))
}
