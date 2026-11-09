//! Revisão por pares, lado do médico (Revisao-Pares.dc.html; DEC-2026-11-08-101 e 102). O revisor vê
//! a sua fila, abre o caso (o vínculo de revisão é registrado na hora e a leitura é auditada), registra
//! o parecer na escala RADPEER ou declara conflito. O autor do laudo vê as revisões dos seus laudos,
//! sem saber quem revisou, e decide a pendência das discrepâncias significativas: retificou (pelo
//! fluxo do laudo, com nova versão e motivo) ou mantém, com motivo. A revisão nunca altera o laudo.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::PacienteResumo;
use crate::rotas::modelos_laudo::{repetida, texto_valido};
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;

/// Escala RADPEER na ordem da tela.
pub const CLASSES: [&str; 5] = ["1", "2a", "2b", "3a", "3b"];
/// Discrepância significativa (pode ter impacto clínico ou deveria ser percebida): abre pendência.
pub const SIGNIFICATIVAS: [&str; 3] = ["2b", "3a", "3b"];

#[derive(Serialize, ToSchema)]
pub struct ItemDaFilaDeRevisao {
    pub id: Uuid,
    pub codigo: String,
    pub laudo_codigo: String,
    pub exame: String,
    /// designada, concluida ou conflito_declarado.
    pub situacao: String,
    pub classe: Option<String>,
    pub designada_em: DateTime<Utc>,
    pub concluida_em: Option<DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct CasoDeRevisao {
    pub id: Uuid,
    pub codigo: String,
    pub situacao: String,
    pub laudo_id: Uuid,
    pub laudo_codigo: String,
    pub exame: String,
    /// Versão assinada revisada.
    pub versao: i32,
    pub paciente: PacienteResumo,
    /// Texto da versão revisada (só enquanto a revisão está aberta).
    pub texto: Option<String>,
    pub assinada_em: Option<DateTime<Utc>>,
    pub estudo_id: Option<Uuid>,
    /// Quem laudou: oculto até o parecer ser enviado.
    pub autor: Option<String>,
    pub classe: Option<String>,
    pub comentario: Option<String>,
    pub concluida_em: Option<DateTime<Utc>>,
}

#[derive(Deserialize, ToSchema)]
pub struct ParecerDeRevisao {
    /// 1, 2a, 2b, 3a ou 3b (RADPEER).
    pub classe: String,
    pub comentario: String,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoConflito {
    pub motivo: String,
}

#[derive(Serialize, ToSchema)]
pub struct RevisaoDoMeuLaudo {
    pub id: Uuid,
    pub codigo: String,
    pub laudo_id: Uuid,
    pub laudo_codigo: String,
    pub exame: String,
    pub versao: i32,
    pub paciente: PacienteResumo,
    pub classe: String,
    pub comentario: Option<String>,
    pub concluida_em: DateTime<Utc>,
    /// aguarda_autor, retificado ou mantido (só nas discrepâncias significativas).
    pub pendencia: Option<String>,
    pub pendencia_motivo: Option<String>,
    pub pendencia_resolvida_em: Option<DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct RevisoesDoAutor {
    pub revisados: i64,
    pub concordantes: i64,
    pub concordancia_percentual: Option<i64>,
    pub pendentes: i64,
    pub revisoes: Vec<RevisaoDoMeuLaudo>,
}

#[derive(Deserialize, ToSchema)]
pub struct DecisaoDePendencia {
    /// retificado (já há versão nova do laudo) ou mantido (com motivo).
    pub decisao: String,
    pub motivo: Option<String>,
}

fn nao_encontrada() -> ErroApi {
    ErroApi::nao_encontrado(
        "revisao-nao-encontrada",
        "Esta revisão não existe nesta unidade ou não é sua. Nada foi aberto.",
    )
}

fn de_outro() -> ErroApi {
    ErroApi::novo(
        StatusCode::FORBIDDEN,
        "revisao-de-outro-medico",
        "Revisão designada a outro médico",
        "Esta revisão está designada a outro médico. Quem laudou não revisa o próprio laudo. Nada foi alterado.",
    )
}

/// Encerra o vínculo de revisão do médico com o paciente se não houver outra revisão aberta dele.
async fn encerrar_vinculo(
    tx: &mut PgConnection,
    medico: Uuid,
    paciente: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE cuidado.vinculo SET expira_em = now(), versao = versao + 1
          WHERE medico_id = $1 AND paciente_id = $2 AND motivo = 'revisao_por_par' AND expira_em > now()
            AND NOT EXISTS (SELECT 1 FROM laudos.revisao r WHERE r.revisor_id = $1 AND r.paciente_id = $2 AND r.situacao = 'designada')",
        medico,
        paciente
    )
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Fila do revisor: revisões designadas a ele (sem o nome do autor nem do paciente).
#[utoipa::path(get, operation_id = "fila_de_revisao_por_pares", path = "/v1/revisoes-por-pares/fila", tag = "revisao-por-pares",
    responses((status = 200, body = Vec<ItemDaFilaDeRevisao>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn fila(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<ItemDaFilaDeRevisao>>, ErroApi> {
    usuario.exigir(Ato::RevisaoPorParRevisar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let itens = sqlx::query_as!(
        ItemDaFilaDeRevisao,
        r#"SELECT id, codigo, laudo_codigo, exame, situacao, classe, criado_em AS designada_em, concluida_em
             FROM laudos.revisao WHERE revisor_id = $1
            ORDER BY (situacao = 'designada') DESC, criado_em DESC LIMIT 100"#,
        usuario.id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(itens))
}

struct Linha {
    id: Uuid,
    codigo: String,
    situacao: String,
    laudo_id: Uuid,
    laudo_codigo: String,
    exame: String,
    versao: i32,
    paciente_id: Uuid,
    revisor_id: Uuid,
    autor: String,
    classe: Option<String>,
    concluida_em: Option<DateTime<Utc>>,
    unidade_id: Uuid,
}

async fn linha(tx: &mut PgConnection, id: Uuid) -> Result<Option<Linha>, sqlx::Error> {
    sqlx::query_as!(
        Linha,
        r#"SELECT r.id, r.codigo, r.situacao, r.laudo_id, r.laudo_codigo, r.exame, r.laudo_versao AS versao, r.paciente_id,
                  r.revisor_id, a.nome AS autor, r.classe, r.concluida_em, r.unidade_id
             FROM laudos.revisao r JOIN identidade.usuario a ON a.id = r.autor_id WHERE r.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await
}

async fn montar(
    tx: &mut PgConnection,
    r: Linha,
    texto: Option<(String, Option<DateTime<Utc>>, Option<Uuid>)>,
) -> Result<CasoDeRevisao, ErroApi> {
    let p = sqlx::query!(
        "SELECT id, nome, nascimento FROM identidade.paciente WHERE id = $1",
        r.paciente_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let comentario = sqlx::query_scalar!(
        "SELECT comentario FROM laudos.revisao_parecer WHERE revisao_id = $1",
        r.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let (texto, assinada_em, estudo_id) = match texto {
        Some((t, a, e)) => (Some(t), a, e),
        None => (None, None, None),
    };
    let concluida = r.situacao == "concluida";
    Ok(CasoDeRevisao {
        id: r.id,
        codigo: r.codigo,
        laudo_id: r.laudo_id,
        laudo_codigo: r.laudo_codigo,
        exame: r.exame,
        versao: r.versao,
        paciente: PacienteResumo {
            id: p.id,
            nome: p.nome,
            nascimento: p.nascimento,
        },
        texto,
        assinada_em,
        estudo_id,
        autor: concluida.then_some(r.autor),
        classe: r.classe,
        comentario,
        concluida_em: r.concluida_em,
        situacao: r.situacao,
    })
}

/// Abre o caso para o revisor. Revisão aberta: registra o vínculo de revisão com o paciente, lê a
/// versão assinada revisada e audita a leitura. Concluída: só o parecer (o laudo não fica aberto).
#[utoipa::path(get, operation_id = "obter_revisao_por_par", path = "/v1/revisoes-por-pares/{id}", tag = "revisao-por-pares", params(("id" = Uuid, Path)),
    responses((status = 200, body = CasoDeRevisao), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn obter(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<CasoDeRevisao>, ErroApi> {
    usuario.exigir(Ato::RevisaoPorParRevisar)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = linha(&mut tx, id).await?.ok_or_else(nao_encontrada)?;
    if r.revisor_id != usuario.id {
        return Err(de_outro());
    }
    let texto = match r.situacao.as_str() {
        "designada" => {
            crate::vinculo::registrar(
                &mut tx,
                usuario.id,
                r.paciente_id,
                r.unidade_id,
                "revisao_por_par",
                &correlacao,
            )
            .await?;
            let laudo = crate::rotas::laudos::carregar(&mut tx, r.laudo_id)
                .await?
                .ok_or_else(|| ErroApi::interno("laudo da revisão sem leitura"))?;
            let v = laudo
                .versoes
                .into_iter()
                .find(|v| v.numero == r.versao)
                .ok_or_else(|| ErroApi::interno("versão revisada sumiu"))?;
            crate::rotas::laudos::auditar(
                &mut tx,
                usuario.id,
                r.laudo_id,
                "permitido",
                Some(&format!("revisao_por_par {}", r.codigo)),
            )
            .await?;
            Some((v.texto, v.assinada_em, laudo.estudo.map(|e| e.id)))
        }
        "concluida" => None,
        _ => {
            return Err(ErroApi::conflito(
                "revisao-encerrada",
                "Você declarou conflito nesta revisão; o caso passou para outro revisor. Nada foi aberto.",
            ))
        }
    };
    let caso = montar(&mut tx, r, texto).await?;
    tx.commit().await?;
    Ok(Json(caso))
}

/// Registra o parecer (RADPEER). Discrepância exige comentário; as significativas (2b, 3a, 3b) abrem
/// pendência para o autor. Uma decisão só: a segunda requisição recebe 409. Idempotency-Key.
#[utoipa::path(post, operation_id = "registrar_parecer_de_revisao", path = "/v1/revisoes-por-pares/{id}/parecer", tag = "revisao-por-pares",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = ParecerDeRevisao,
    responses((status = 200, body = CasoDeRevisao), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn parecer(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<ParecerDeRevisao>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::RevisaoPorParRevisar)?;
    let classe = p.classe.trim().to_lowercase();
    if !CLASSES.contains(&classe.as_str()) {
        return Err(ErroApi::invalido(
            "classe-invalida",
            "Escolha uma classificação da escala (1, 2a, 2b, 3a ou 3b). Nada foi enviado.",
        ));
    }
    let comentario = p.comentario.trim().to_owned();
    if comentario.chars().count() > 2000 {
        return Err(ErroApi::invalido(
            "comentario-longo",
            "O comentário passou de 2000 caracteres. Nada foi enviado.",
        ));
    }
    if classe != "1" && comentario.chars().count() < 10 {
        return Err(ErroApi::invalido(
            "comentario-obrigatorio",
            "Na discrepância, explique ao colega o que foi visto (10 letras ou mais). Nada foi enviado.",
        ));
    }
    let rota = format!("POST /v1/revisoes-por-pares/{id}/parecer");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let significativa = SIGNIFICATIVAS.contains(&classe.as_str());
    let feita = sqlx::query!(
        "UPDATE laudos.revisao SET situacao = 'concluida', classe = $3, concluida_em = now(),
                pendencia = CASE WHEN $4 THEN 'aguarda_autor' END, versao = versao + 1
          WHERE id = $1 AND revisor_id = $2 AND situacao = 'designada'
          RETURNING paciente_id, codigo",
        id,
        usuario.id,
        classe,
        significativa
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(feita) = feita else {
        let r = linha(&mut tx, id).await?.ok_or_else(nao_encontrada)?;
        return Err(if r.revisor_id != usuario.id {
            de_outro()
        } else {
            ErroApi::conflito(
                "revisao-ja-concluida",
                "Esta revisão já foi enviada ou encerrada. Nada foi alterado.",
            )
        });
    };
    sqlx::query!(
        "INSERT INTO laudos.revisao_parecer (revisao_id, comentario, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, 'consultorio', $4, $5)",
        id, comentario, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    encerrar_vinculo(&mut tx, usuario.id, feita.paciente_id).await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "laudos",
            tipo: "revisao_concluida",
            registro_id: id,
            registro_codigo: Some(&feita.codigo),
            dados: json!({ "classe": classe, "pendencia": significativa }),
            criado_por: Some(usuario.id),
            origem: "consultorio",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let r = linha(&mut tx, id).await?.ok_or_else(nao_encontrada)?;
    let caso = montar(&mut tx, r, None).await?;
    let corpo = serde_json::to_value(&caso).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 200, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::OK, Json(corpo)).into_response())
}

/// O revisor declara conflito (ex.: atendeu o paciente, parentesco): a revisão é encerrada sem parecer
/// e a coordenação médica designa outro revisor.
#[utoipa::path(post, operation_id = "declarar_conflito_de_revisao", path = "/v1/revisoes-por-pares/{id}/conflito", tag = "revisao-por-pares",
    params(("id" = Uuid, Path)), request_body = PedidoConflito,
    responses((status = 200, body = ItemDaFilaDeRevisao), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn conflito(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoConflito>,
) -> Result<Json<ItemDaFilaDeRevisao>, ErroApi> {
    usuario.exigir(Ato::RevisaoPorParRevisar)?;
    let motivo = texto_valido(
        &p.motivo,
        5,
        500,
        "motivo-obrigatorio",
        "Diga qual é o conflito (5 a 500 letras). Nada foi alterado.",
    )?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let feita = sqlx::query!(
        "UPDATE laudos.revisao SET situacao = 'conflito_declarado', motivo_conflito = $3, conflito_em = now(), versao = versao + 1
          WHERE id = $1 AND revisor_id = $2 AND situacao = 'designada' RETURNING paciente_id, codigo",
        id, usuario.id, motivo
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(feita) = feita else {
        let r = linha(&mut tx, id).await?.ok_or_else(nao_encontrada)?;
        return Err(if r.revisor_id != usuario.id {
            de_outro()
        } else {
            ErroApi::conflito(
                "revisao-ja-concluida",
                "Esta revisão já foi enviada ou encerrada. Nada foi alterado.",
            )
        });
    };
    encerrar_vinculo(&mut tx, usuario.id, feita.paciente_id).await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "laudos",
            tipo: "revisao_conflito_declarado",
            registro_id: id,
            registro_codigo: Some(&feita.codigo),
            dados: json!({ "motivo": motivo }),
            criado_por: Some(usuario.id),
            origem: "consultorio",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let item = sqlx::query_as!(
        ItemDaFilaDeRevisao,
        r#"SELECT id, codigo, laudo_codigo, exame, situacao, classe, criado_em AS designada_em, concluida_em FROM laudos.revisao WHERE id = $1"#,
        id
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(item))
}

async fn do_autor(
    tx: &mut PgConnection,
    autor: Uuid,
    id: Option<Uuid>,
) -> Result<Vec<RevisaoDoMeuLaudo>, sqlx::Error> {
    let linhas = sqlx::query!(
        r#"SELECT r.id, r.codigo, r.laudo_id, r.laudo_codigo, r.exame, r.laudo_versao, r.classe AS "classe!", r.concluida_em AS "concluida_em!",
                  r.pendencia, r.pendencia_motivo, r.pendencia_resolvida_em, c.comentario AS "comentario?",
                  p.id AS paciente_id, p.nome, p.nascimento
             FROM laudos.revisao r
             JOIN identidade.paciente p ON p.id = r.paciente_id
             LEFT JOIN laudos.revisao_parecer c ON c.revisao_id = r.id
            WHERE r.autor_id = $1 AND r.situacao = 'concluida' AND ($2::uuid IS NULL OR r.id = $2)
            ORDER BY (r.pendencia = 'aguarda_autor') DESC NULLS LAST, r.concluida_em DESC LIMIT 200"#,
        autor,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| RevisaoDoMeuLaudo {
            id: l.id,
            codigo: l.codigo,
            laudo_id: l.laudo_id,
            laudo_codigo: l.laudo_codigo,
            exame: l.exame,
            versao: l.laudo_versao,
            paciente: PacienteResumo {
                id: l.paciente_id,
                nome: l.nome,
                nascimento: l.nascimento,
            },
            classe: l.classe,
            comentario: l.comentario,
            concluida_em: l.concluida_em,
            pendencia: l.pendencia,
            pendencia_motivo: l.pendencia_motivo,
            pendencia_resolvida_em: l.pendencia_resolvida_em,
        })
        .collect())
}

/// Revisões dos laudos do médico (o revisor não é revelado ao autor) e o resultado dele.
#[utoipa::path(get, operation_id = "revisoes_dos_meus_laudos", path = "/v1/revisoes-por-pares/meus-laudos", tag = "revisao-por-pares",
    responses((status = 200, body = RevisoesDoAutor), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn meus_laudos(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<RevisoesDoAutor>, ErroApi> {
    usuario.exigir(Ato::RevisaoPorParRevisar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let revisoes = do_autor(&mut tx, usuario.id, None).await?;
    tx.commit().await?;
    let revisados = revisoes.len() as i64;
    let concordantes = revisoes.iter().filter(|r| r.classe == "1").count() as i64;
    Ok(Json(RevisoesDoAutor {
        revisados,
        concordantes,
        concordancia_percentual: (revisados > 0).then(|| concordantes * 100 / revisados),
        pendentes: revisoes
            .iter()
            .filter(|r| r.pendencia.as_deref() == Some("aguarda_autor"))
            .count() as i64,
        revisoes,
    }))
}

/// O autor decide a pendência: "retificado" (o laudo já tem versão nova, aberta pelo fluxo de
/// retificação) ou "mantido" (com motivo). Uma decisão só.
#[utoipa::path(post, operation_id = "decidir_pendencia_de_revisao", path = "/v1/revisoes-por-pares/{id}/pendencia", tag = "revisao-por-pares",
    params(("id" = Uuid, Path)), request_body = DecisaoDePendencia,
    responses((status = 200, body = RevisaoDoMeuLaudo), (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn decidir_pendencia(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<DecisaoDePendencia>,
) -> Result<Json<RevisaoDoMeuLaudo>, ErroApi> {
    usuario.exigir(Ato::RevisaoPorParRevisar)?;
    let motivo = p.motivo.as_deref().map(str::trim).filter(|m| !m.is_empty());
    let motivo = match (p.decisao.as_str(), motivo) {
        ("mantido", None) => {
            return Err(ErroApi::invalido(
                "motivo-obrigatorio",
                "Para manter o laudo, explique por quê (5 letras ou mais). Nada foi alterado.",
            ));
        }
        ("mantido" | "retificado", Some(m)) => Some(texto_valido(
            m,
            5,
            1000,
            "motivo-invalido",
            "O motivo tem 5 a 1000 letras. Nada foi alterado.",
        )?),
        ("retificado", None) => None,
        _ => {
            return Err(ErroApi::invalido(
                "decisao-invalida",
                "A decisão é retificado ou mantido. Nada foi alterado.",
            ));
        }
    };
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let atual = do_autor(&mut tx, usuario.id, Some(id))
        .await?
        .pop()
        .ok_or_else(nao_encontrada)?;
    if p.decisao == "retificado" {
        let ha_nova = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM laudos.versao WHERE laudo_id = $1 AND numero > $2) AS "ha!""#,
            atual.laudo_id,
            atual.versao
        )
        .fetch_one(&mut *tx)
        .await?;
        if !ha_nova {
            return Err(ErroApi::conflito(
                "retificacao-nao-encontrada",
                "O laudo ainda não tem versão nova. Nada foi alterado. Abra a retificação no laudo, com motivo, e depois marque como retificado.",
            ));
        }
    }
    let n = sqlx::query!(
        "UPDATE laudos.revisao SET pendencia = $3, pendencia_motivo = $4, pendencia_resolvida_em = now(), versao = versao + 1
          WHERE id = $1 AND autor_id = $2 AND pendencia = 'aguarda_autor'",
        id, usuario.id, p.decisao, motivo
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if n != 1 {
        return Err(ErroApi::conflito(
            "pendencia-ja-decidida",
            "Esta revisão não tem pendência aberta. Nada foi alterado.",
        ));
    }
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "laudos",
            tipo: "pendencia_de_revisao_decidida",
            registro_id: id,
            registro_codigo: Some(&atual.codigo),
            dados: json!({ "decisao": p.decisao, "motivo": motivo, "laudo": atual.laudo_codigo }),
            criado_por: Some(usuario.id),
            origem: "consultorio",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let atualizada = do_autor(&mut tx, usuario.id, Some(id))
        .await?
        .pop()
        .ok_or_else(nao_encontrada)?;
    tx.commit().await?;
    Ok(Json(atualizada))
}
