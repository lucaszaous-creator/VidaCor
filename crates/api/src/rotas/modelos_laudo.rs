//! Modelos de laudo (Modelos-Laudo.dc.html; docs/04 Laudar; DEC-2026-11-08-100). Modelo da clínica:
//! o médico propõe e a coordenação médica aprova (quem propõe não aprova, conferido também no banco).
//! Modelo pessoal: só o próprio médico, publica direto. Editar é nova versão com motivo; desativar
//! exige motivo; nada é apagado. O modelo não tem dado de paciente. Usar um modelo num laudo grava
//! a versão do modelo usada (lastro).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgConnection;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct VersaoDeModelo {
    pub numero: i32,
    pub texto: String,
    pub motivo: Option<String>,
    /// proposta, publicada, substituida ou recusada.
    pub situacao: String,
    pub proposta_por: String,
    pub decidido_por: Option<String>,
    pub decidido_em: Option<DateTime<Utc>>,
    pub motivo_decisao: Option<String>,
    pub criado_em: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ModeloDeLaudo {
    pub id: Uuid,
    pub codigo: String,
    /// clinica ou pessoal.
    pub tipo: String,
    pub nome: String,
    pub exame: String,
    pub modalidade: String,
    /// Dono do modelo pessoal; "Clínica" no da clínica.
    pub autor: String,
    pub ativo: bool,
    pub motivo_desativacao: Option<String>,
    pub versao_publicada: Option<i32>,
    pub proposta_em_aberto: Option<i32>,
    /// Laudos que usaram o modelo (qualquer versão).
    pub usos: i64,
    /// O servidor diz o que esta pessoa pode fazer (a tela só mostra).
    pub pode_editar: bool,
    pub pode_decidir: bool,
    pub pode_desativar: bool,
    /// Da mais nova para a mais antiga.
    pub versoes: Vec<VersaoDeModelo>,
}

#[derive(Deserialize, ToSchema)]
pub struct NovoModeloDeLaudo {
    /// clinica ou pessoal.
    pub tipo: String,
    pub nome: String,
    pub exame: String,
    pub modalidade: String,
    pub texto: String,
    /// Modelo da clínica de onde o pessoal foi derivado.
    pub derivado_de: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
pub struct NovaVersaoDeModelo {
    pub texto: String,
    pub motivo: String,
    /// Versão publicada que a pessoa estava vendo (nenhuma: nulo). Diferente da atual: 409.
    pub versao_base: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct DecisaoDeVersaoDeModelo {
    pub aprovar: bool,
    pub motivo: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoDeDesativacao {
    pub motivo: String,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoUsoDeModelo {
    pub modelo_id: Uuid,
    pub numero: i32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UsoDeModelo {
    pub modelo_codigo: String,
    pub numero: i32,
    pub laudo_versao: i32,
    pub texto: String,
}

/// Coordenação médica: perfil gerente com perfil médico (DEC-2026-11-08-100).
pub(crate) fn exigir_coordenacao_medica(u: &Usuario) -> Result<(), ErroApi> {
    u.exigir(Ato::LaudoCoordenar)?;
    if u.tem_perfil("medico") {
        Ok(())
    } else {
        Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "coordenacao-medica",
            "Só a coordenação médica",
            "Este ato é da coordenação médica (médico com perfil de coordenação). Nada foi alterado.",
        ))
    }
}

pub(crate) fn texto_valido(
    t: &str,
    min: usize,
    max: usize,
    codigo: &str,
    detalhe: &str,
) -> Result<String, ErroApi> {
    let t = t.trim();
    if (min..=max).contains(&t.chars().count()) {
        Ok(t.to_owned())
    } else {
        Err(ErroApi::invalido(codigo, detalhe))
    }
}

/// Modelo e frase não levam dado de paciente: recusa CPF (11 dígitos) e data completa (dd/mm/aaaa).
pub(crate) fn parece_dado_de_paciente(t: &str) -> bool {
    let c: Vec<char> = t.chars().collect();
    let mut digitos = 0;
    for (i, ch) in c.iter().enumerate() {
        if ch.is_ascii_digit() {
            digitos += 1;
            if digitos >= 11 {
                return true;
            }
        } else if !(matches!(ch, '.' | '-') && i > 0 && c[i - 1].is_ascii_digit()) {
            digitos = 0;
        }
    }
    c.windows(10).any(|w| {
        w.iter().enumerate().all(|(i, ch)| match i {
            2 | 5 => *ch == '/',
            _ => ch.is_ascii_digit(),
        })
    })
}

fn sem_dado_de_paciente(t: &str) -> Result<(), ErroApi> {
    if parece_dado_de_paciente(t) {
        Err(ErroApi::invalido(
            "modelo-com-dado-de-paciente",
            "O texto tem o que parece CPF ou data completa. Modelo e frase não levam dado de paciente: use um campo variável entre colchetes, como [data]. Nada foi gravado.",
        ))
    } else {
        Ok(())
    }
}

fn nao_encontrado() -> ErroApi {
    ErroApi::nao_encontrado(
        "modelo-nao-encontrado",
        "Este modelo não existe ou não está disponível para você. Nada foi alterado.",
    )
}

pub(crate) fn repetida(status: u16, corpo: Value) -> Response {
    (
        StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
        Json(corpo),
    )
        .into_response()
}

/// Modelos visíveis no contexto (RLS: os da clínica e os pessoais do próprio médico).
async fn carregar(
    tx: &mut PgConnection,
    usuario: &Usuario,
    id: Option<Uuid>,
) -> Result<Vec<ModeloDeLaudo>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT m.id, m.codigo, m.tipo, m.nome, m.exame, m.modalidade, m.ativo, m.motivo_desativacao, m.dono_id, d.nome AS "dono_nome?"
             FROM laudos.modelo m LEFT JOIN identidade.usuario d ON d.id = m.dono_id
            WHERE ($1::uuid IS NULL OR m.id = $1)
            ORDER BY m.modalidade, lower(m.nome) LIMIT 300"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    let ids: Vec<Uuid> = linhas.iter().map(|l| l.id).collect();
    let versoes = sqlx::query!(
        r#"SELECT v.modelo_id, v.numero, v.texto, v.motivo, v.situacao, v.proposta_por AS proposta_por_id, p.nome AS proposta_por,
                  d.nome AS "decidido_por?", v.decidido_em, v.motivo_decisao, v.criado_em
             FROM laudos.modelo_versao v
             JOIN identidade.usuario p ON p.id = v.proposta_por
             LEFT JOIN identidade.usuario d ON d.id = v.decidido_por
            WHERE v.modelo_id = ANY($1) ORDER BY v.modelo_id, v.numero DESC"#,
        &ids
    )
    .fetch_all(&mut *tx)
    .await?;
    let usos: HashMap<Uuid, i64> = sqlx::query!(
        r#"SELECT modelo_id AS "modelo_id!", usos AS "usos!" FROM laudos.usos_dos_modelos($1)"#,
        &ids
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|u| (u.modelo_id, u.usos))
    .collect();
    let coordenacao = exigir_coordenacao_medica(usuario).is_ok();
    let medico = usuario.tem_perfil("medico");
    let mut por_modelo: HashMap<Uuid, Vec<(Uuid, VersaoDeModelo)>> = HashMap::new();
    for v in versoes {
        por_modelo.entry(v.modelo_id).or_default().push((
            v.proposta_por_id,
            VersaoDeModelo {
                numero: v.numero,
                texto: v.texto,
                motivo: v.motivo,
                situacao: v.situacao,
                proposta_por: v.proposta_por,
                decidido_por: v.decidido_por,
                decidido_em: v.decidido_em,
                motivo_decisao: v.motivo_decisao,
                criado_em: v.criado_em,
            },
        ));
    }
    Ok(linhas
        .into_iter()
        .map(|l| {
            let vs = por_modelo.remove(&l.id).unwrap_or_default();
            let publicada = vs
                .iter()
                .find(|(_, v)| v.situacao == "publicada")
                .map(|(_, v)| v.numero);
            let proposta = vs.iter().find(|(_, v)| v.situacao == "proposta");
            let clinica = l.tipo == "clinica";
            let dono = l.dono_id == Some(usuario.id);
            ModeloDeLaudo {
                id: l.id,
                codigo: l.codigo,
                autor: l.dono_nome.unwrap_or_else(|| "Clínica".to_owned()),
                nome: l.nome,
                exame: l.exame,
                modalidade: l.modalidade,
                ativo: l.ativo,
                motivo_desativacao: l.motivo_desativacao,
                versao_publicada: publicada,
                proposta_em_aberto: proposta.map(|(_, v)| v.numero),
                usos: usos.get(&l.id).copied().unwrap_or(0),
                pode_editar: l.ativo && medico && (dono || (clinica && proposta.is_none())),
                pode_decidir: l.ativo
                    && clinica
                    && coordenacao
                    && proposta.is_some_and(|(quem, _)| *quem != usuario.id),
                pode_desativar: l.ativo && ((dono && medico) || (clinica && coordenacao)),
                tipo: l.tipo,
                versoes: vs.into_iter().map(|(_, v)| v).collect(),
            }
        })
        .collect())
}

async fn um(tx: &mut PgConnection, usuario: &Usuario, id: Uuid) -> Result<ModeloDeLaudo, ErroApi> {
    carregar(tx, usuario, Some(id))
        .await?
        .pop()
        .ok_or_else(nao_encontrado)
}

fn exigir_leitura(usuario: &Usuario) -> Result<(), ErroApi> {
    if usuario.pode(Ato::ModeloLaudoUsar) || exigir_coordenacao_medica(usuario).is_ok() {
        Ok(())
    } else {
        Err(ErroApi::proibido())
    }
}

/// Modelos da clínica e os pessoais do médico, com as versões (médico e coordenação médica).
#[utoipa::path(get, operation_id = "listar_modelos_de_laudo", path = "/v1/modelos-de-laudo", tag = "modelos-de-laudo",
    responses((status = 200, body = Vec<ModeloDeLaudo>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<ModeloDeLaudo>>, ErroApi> {
    exigir_leitura(&usuario)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let modelos = carregar(&mut tx, &usuario, None).await?;
    tx.commit().await?;
    Ok(Json(modelos))
}

/// Um modelo com todas as versões.
#[utoipa::path(get, operation_id = "obter_modelo_de_laudo", path = "/v1/modelos-de-laudo/{id}", tag = "modelos-de-laudo", params(("id" = Uuid, Path)),
    responses((status = 200, body = ModeloDeLaudo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn obter(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<ModeloDeLaudo>, ErroApi> {
    exigir_leitura(&usuario)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let m = um(&mut tx, &usuario, id).await?;
    tx.commit().await?;
    Ok(Json(m))
}

fn evento<'a>(
    usuario: &Usuario,
    tipo: &'a str,
    id: Uuid,
    codigo: &'a str,
    dados: Value,
    correlacao: &'a str,
) -> NovoEvento<'a> {
    NovoEvento {
        dominio: "laudos",
        tipo,
        registro_id: id,
        registro_codigo: Some(codigo),
        dados,
        criado_por: Some(usuario.id),
        origem: "consultorio",
        unidade_id: usuario.unidade_id,
        correlacao,
    }
}

/// Cria um modelo: pessoal (publicado na hora, versão 1) ou proposta de modelo da clínica (aguarda a
/// coordenação médica). Idempotency-Key.
#[utoipa::path(post, operation_id = "criar_modelo_de_laudo", path = "/v1/modelos-de-laudo", tag = "modelos-de-laudo",
    params(("Idempotency-Key" = Uuid, Header)), request_body = NovoModeloDeLaudo,
    responses((status = 201, body = ModeloDeLaudo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn criar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoModeloDeLaudo>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::ModeloLaudoUsar)?;
    if !matches!(n.tipo.as_str(), "clinica" | "pessoal") {
        return Err(ErroApi::invalido(
            "tipo-invalido",
            "O modelo é da clínica ou pessoal. Nada foi criado.",
        ));
    }
    let nome = texto_valido(
        &n.nome,
        3,
        120,
        "nome-invalido",
        "Dê um nome ao modelo (3 a 120 letras). Nada foi criado.",
    )?;
    let exame = texto_valido(
        &n.exame,
        3,
        160,
        "exame-invalido",
        "Diga para qual exame é o modelo (3 a 160 letras). Nada foi criado.",
    )?;
    let modalidade = n.modalidade.trim().to_uppercase();
    if !(2..=4).contains(&modalidade.len()) || !modalidade.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(ErroApi::invalido(
            "modalidade-invalida",
            "A modalidade tem 2 a 4 letras (ex.: US, CT, DX). Nada foi criado.",
        ));
    }
    let texto = texto_valido(
        &n.texto,
        1,
        20_000,
        "texto-invalido",
        "O texto do modelo está vazio ou passou de 20 mil caracteres. Nada foi criado.",
    )?;
    sem_dado_de_paciente(&format!("{nome}\n{exame}\n{texto}"))?;
    let rota = "POST /v1/modelos-de-laudo";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    if let Some(base) = n.derivado_de {
        let ok = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM laudos.modelo WHERE id = $1 AND tipo = 'clinica') AS "ok!""#,
            base
        )
        .fetch_one(&mut *tx)
        .await?;
        if !ok {
            return Err(nao_encontrado());
        }
    }
    let clinica = n.tipo == "clinica";
    let id = novo_id();
    let codigo = proximo(&mut tx, "MOD").await?;
    sqlx::query!(
        "INSERT INTO laudos.modelo (id, codigo, tipo, dono_id, nome, exame, modalidade, derivado_de, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'consultorio', $10, $11)",
        id, codigo.como_str(), n.tipo, (!clinica).then_some(usuario.id), nome, exame, modalidade, n.derivado_de, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("nome-em-uso", "Já existe um modelo ativo com este nome. Nada foi criado. Escolha outro nome ou edite o existente."),
        _ => ErroApi::from(e),
    })?;
    let situacao = if clinica { "proposta" } else { "publicada" };
    sqlx::query!(
        "INSERT INTO laudos.modelo_versao (id, modelo_id, numero, texto, situacao, proposta_por, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, 1, $3, $4, $5, $5, 'consultorio', $6, $7)",
        novo_id(), id, texto, situacao, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        evento(
            &usuario,
            "modelo_criado",
            id,
            codigo.como_str(),
            json!({ "tipo": n.tipo, "versao": 1, "situacao": situacao, "derivado_de": n.derivado_de }),
            &correlacao,
        ),
    )
    .await?;
    let corpo = serde_json::to_value(um(&mut tx, &usuario, id).await?).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Nova versão: no pessoal publica na hora (a anterior fica substituída); na clínica vira proposta
/// para a coordenação médica. Motivo obrigatório. Idempotency-Key.
#[utoipa::path(post, operation_id = "propor_versao_de_modelo", path = "/v1/modelos-de-laudo/{id}/versoes", tag = "modelos-de-laudo",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = NovaVersaoDeModelo,
    responses((status = 201, body = ModeloDeLaudo), (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn nova_versao(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<NovaVersaoDeModelo>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::ModeloLaudoUsar)?;
    let texto = texto_valido(
        &p.texto,
        1,
        20_000,
        "texto-invalido",
        "O texto do modelo está vazio ou passou de 20 mil caracteres. Nada foi gravado.",
    )?;
    let motivo = texto_valido(
        &p.motivo,
        5,
        500,
        "motivo-obrigatorio",
        "Diga o que mudou e por quê (5 a 500 letras). Nada foi gravado.",
    )?;
    sem_dado_de_paciente(&texto)?;
    let rota = format!("POST /v1/modelos-de-laudo/{id}/versoes");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let m = um(&mut tx, &usuario, id).await?;
    if !m.ativo {
        return Err(ErroApi::conflito(
            "modelo-desativado",
            "Este modelo está desativado. Nada foi gravado. Crie um modelo novo, se precisar.",
        ));
    }
    let clinica = m.tipo == "clinica";
    if !clinica && !m.pode_editar {
        return Err(nao_encontrado());
    }
    if m.versao_publicada != p.versao_base {
        return Err(ErroApi::conflito("versao-desatualizada", "O modelo mudou enquanto você editava. Nada foi gravado. Abra a versão atual e refaça a mudança."));
    }
    let numero = m.versoes.iter().map(|v| v.numero).max().unwrap_or(0) + 1;
    if !clinica {
        if let Some(base) = p.versao_base {
            let n = sqlx::query!(
                "UPDATE laudos.modelo_versao SET situacao = 'substituida', versao = versao + 1 WHERE modelo_id = $1 AND numero = $2 AND situacao = 'publicada'",
                id, base
            )
            .execute(&mut *tx)
            .await?
            .rows_affected();
            if n != 1 {
                return Err(ErroApi::conflito("versao-desatualizada", "O modelo mudou enquanto você editava. Nada foi gravado. Abra a versão atual e refaça a mudança."));
            }
        }
    }
    let situacao = if clinica { "proposta" } else { "publicada" };
    sqlx::query!(
        "INSERT INTO laudos.modelo_versao (id, modelo_id, numero, texto, motivo, situacao, proposta_por, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7, 'consultorio', $8, $9)",
        novo_id(), id, numero, texto, motivo, situacao, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("proposta-em-aberto", "Já há uma versão deste modelo aguardando a coordenação médica ou outra pessoa gravou agora. Nada foi gravado. Aguarde a decisão e tente de novo."),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        evento(
            &usuario,
            if clinica {
                "modelo_versao_proposta"
            } else {
                "modelo_versao_publicada"
            },
            id,
            &m.codigo,
            json!({ "versao": numero, "motivo": motivo }),
            &correlacao,
        ),
    )
    .await?;
    let corpo = serde_json::to_value(um(&mut tx, &usuario, id).await?).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Aprova ou recusa a proposta de versão de um modelo da clínica (coordenação médica; quem propôs
/// não decide). Aprovar publica a versão e substitui a anterior; recusar exige motivo.
#[utoipa::path(post, operation_id = "decidir_versao_de_modelo", path = "/v1/modelos-de-laudo/{id}/versoes/{numero}/decisao", tag = "modelos-de-laudo",
    params(("id" = Uuid, Path), ("numero" = i32, Path)), request_body = DecisaoDeVersaoDeModelo,
    responses((status = 200, body = ModeloDeLaudo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn decidir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path((id, numero)): Path<(Uuid, i32)>,
    Corpo(p): Corpo<DecisaoDeVersaoDeModelo>,
) -> Result<Json<ModeloDeLaudo>, ErroApi> {
    exigir_coordenacao_medica(&usuario)?;
    let motivo = match (
        p.aprovar,
        p.motivo.as_deref().map(str::trim).filter(|m| !m.is_empty()),
    ) {
        (_, Some(m)) => Some(texto_valido(
            m,
            5,
            500,
            "motivo-invalido",
            "O motivo tem 5 a 500 letras. Nada foi decidido.",
        )?),
        (true, None) => None,
        (false, None) => {
            return Err(ErroApi::invalido(
                "motivo-obrigatorio",
                "Para recusar, diga o motivo a quem propôs. Nada foi decidido.",
            ));
        }
    };
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let m = um(&mut tx, &usuario, id).await?;
    if m.tipo != "clinica" || !m.ativo {
        return Err(ErroApi::conflito(
            "modelo-sem-aprovacao",
            "Só modelo ativo da clínica passa por aprovação. Nada foi decidido.",
        ));
    }
    let proposta = sqlx::query!(
        "SELECT proposta_por, situacao FROM laudos.modelo_versao WHERE modelo_id = $1 AND numero = $2 FOR UPDATE",
        id, numero
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("versao-nao-encontrada", "Esta versão do modelo não existe. Nada foi decidido."))?;
    if proposta.situacao != "proposta" {
        return Err(ErroApi::conflito(
            "proposta-ja-decidida",
            "Esta proposta já foi decidida. Nada foi alterado.",
        ));
    }
    if proposta.proposta_por == usuario.id {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "quem-propoe-nao-aprova",
            "Quem propõe não aprova",
            "Você propôs esta versão; outra pessoa da coordenação médica precisa decidir. Nada foi alterado.",
        ));
    }
    if p.aprovar {
        sqlx::query!(
            "UPDATE laudos.modelo_versao SET situacao = 'substituida', versao = versao + 1 WHERE modelo_id = $1 AND situacao = 'publicada'",
            id
        )
        .execute(&mut *tx)
        .await?;
    }
    let decidiu = sqlx::query!(
        "UPDATE laudos.modelo_versao SET situacao = $3, decidido_por = $4, decidido_em = now(), motivo_decisao = $5, versao = versao + 1
          WHERE modelo_id = $1 AND numero = $2 AND situacao = 'proposta' AND proposta_por <> $4",
        id, numero, if p.aprovar { "publicada" } else { "recusada" }, usuario.id, motivo
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if decidiu != 1 {
        return Err(ErroApi::conflito(
            "proposta-ja-decidida",
            "Esta proposta já foi decidida. Nada foi alterado.",
        ));
    }
    lastro::registrar(
        &mut tx,
        evento(
            &usuario,
            if p.aprovar {
                "modelo_versao_aprovada"
            } else {
                "modelo_versao_recusada"
            },
            id,
            &m.codigo,
            json!({ "versao": numero, "motivo": motivo }),
            &correlacao,
        ),
    )
    .await?;
    let atualizado = um(&mut tx, &usuario, id).await?;
    tx.commit().await?;
    Ok(Json(atualizado))
}

/// Desativa o modelo com motivo (pessoal: o dono; da clínica: a coordenação médica). Nada é apagado;
/// laudos que usaram o modelo continuam com a versão gravada.
#[utoipa::path(post, operation_id = "desativar_modelo_de_laudo", path = "/v1/modelos-de-laudo/{id}/desativacao", tag = "modelos-de-laudo",
    params(("id" = Uuid, Path)), request_body = PedidoDeDesativacao,
    responses((status = 200, body = ModeloDeLaudo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn desativar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoDeDesativacao>,
) -> Result<Json<ModeloDeLaudo>, ErroApi> {
    exigir_leitura(&usuario)?;
    let motivo = texto_valido(
        &p.motivo,
        5,
        500,
        "motivo-obrigatorio",
        "Diga por que o modelo sai de uso (5 a 500 letras). Nada foi alterado.",
    )?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let m = um(&mut tx, &usuario, id).await?;
    if !m.ativo {
        return Err(ErroApi::conflito(
            "modelo-desativado",
            "Este modelo já está desativado. Nada foi alterado.",
        ));
    }
    if !m.pode_desativar {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "coordenacao-medica",
            "Só a coordenação médica",
            "Modelo da clínica sai de uso só pela coordenação médica. Nada foi alterado. Proponha a mudança, se for o caso.",
        ));
    }
    let n = sqlx::query!(
        "UPDATE laudos.modelo SET ativo = false, desativado_em = now(), desativado_por = $2, motivo_desativacao = $3, versao = versao + 1
          WHERE id = $1 AND ativo",
        id, usuario.id, motivo
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if n != 1 {
        return Err(ErroApi::conflito(
            "modelo-desativado",
            "Este modelo já está desativado. Nada foi alterado.",
        ));
    }
    lastro::registrar(
        &mut tx,
        evento(
            &usuario,
            "modelo_desativado",
            id,
            &m.codigo,
            json!({ "motivo": motivo }),
            &correlacao,
        ),
    )
    .await?;
    let atualizado = um(&mut tx, &usuario, id).await?;
    tx.commit().await?;
    Ok(Json(atualizado))
}

/// Usa um modelo no rascunho do laudo: devolve o texto da versão publicada e grava qual versão do
/// modelo entrou em qual versão do laudo (só o médico atribuído; Idempotency-Key). O texto entra no
/// rascunho pelo salvamento normal do laudo.
#[utoipa::path(post, operation_id = "registrar_uso_de_modelo", path = "/v1/laudos/{id}/usos-de-modelo", tag = "modelos-de-laudo",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = PedidoUsoDeModelo,
    responses((status = 201, body = UsoDeModelo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn usar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoUsoDeModelo>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::LaudoEscrever)?;
    usuario.exigir(Ato::ModeloLaudoUsar)?;
    let rota = format!("POST /v1/laudos/{id}/usos-de-modelo");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok(repetida(status, corpo));
    }
    let laudo = crate::rotas::laudos::do_medico_atribuido(&mut tx, &usuario, id).await?;
    let rascunho = laudo
        .versoes
        .iter()
        .find(|v| v.situacao == "rascunho")
        .map(|v| v.numero)
        .ok_or_else(|| ErroApi::conflito("laudo-sem-rascunho", "O laudo está assinado e sem retificação aberta. Nada foi inserido. Para mudar o texto, abra uma retificação com motivo."))?;
    let v = sqlx::query!(
        r#"SELECT v.id, v.texto, m.codigo FROM laudos.modelo_versao v JOIN laudos.modelo m ON m.id = v.modelo_id
            WHERE v.modelo_id = $1 AND v.numero = $2 AND v.situacao = 'publicada' AND m.ativo"#,
        p.modelo_id, p.numero
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::conflito("versao-de-modelo-indisponivel", "Esta versão do modelo não está mais publicada. Nada foi inserido. Recarregue os modelos e escolha de novo."))?;
    sqlx::query!(
        "INSERT INTO laudos.uso_modelo (id, laudo_id, laudo_versao, modelo_versao_id, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, 'consultorio', $6, $7)",
        novo_id(), id, rascunho, v.id, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        evento(
            &usuario,
            "modelo_usado",
            id,
            &laudo.codigo,
            json!({ "modelo": v.codigo, "modelo_versao": p.numero, "laudo_versao": rascunho }),
            &correlacao,
        ),
    )
    .await?;
    let corpo = serde_json::to_value(UsoDeModelo {
        modelo_codigo: v.codigo,
        numero: p.numero,
        laudo_versao: rascunho,
        texto: v.texto,
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

#[cfg(test)]
mod testes {
    use super::parece_dado_de_paciente;

    #[test]
    fn recusa_cpf_e_data_completa_e_aceita_medidas() {
        assert!(parece_dado_de_paciente("CPF 123.456.789-09"));
        assert!(parece_dado_de_paciente("12345678909"));
        assert!(parece_dado_de_paciente("exame de 07/11/2026"));
        assert!(!parece_dado_de_paciente("DBP [dbp], CC 178 mm, peso 412 g"));
        assert!(!parece_dado_de_paciente("Nódulo de 1,2 x 0,8 cm em [data]"));
    }
}
