//! Possíveis duplicados (aba da lista de pacientes): pares de cadastros com o mesmo nome e um segundo
//! sinal (mesmo nascimento, mesmo celular, mesma mãe ou nascimento no mesmo ano). A recepção decide
//! "pedir junção" ou "são pessoas diferentes"; nada é juntado aqui (DEC-2026-11-09-126 e 127). A decisão
//! é imutável e única por par (G2: constraint única); mudar de ideia é nova decisão com motivo.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::rotas::pacientes_lista::ve_pacientes;
use crate::sessao::Usuario;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Serialize, ToSchema)]
pub struct CadastroDoPar {
    pub id: Uuid,
    pub codigo: String,
    pub nome: String,
    pub nascimento: NaiveDate,
    pub cpf_mascarado: Option<String>,
    pub celular: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ParDuplicado {
    pub a: CadastroDoPar,
    pub b: CadastroDoPar,
    /// Por que o par apareceu (ex.: "Mesmo nome e mesmo celular").
    pub motivo: String,
    /// aberto ou juncao_pedida (os marcados como pessoas diferentes saem da lista).
    pub situacao: String,
    pub decidido_em: Option<DateTime<Utc>>,
    pub decidido_por: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct DecisaoSobreDuplicado {
    pub paciente_a: Uuid,
    pub paciente_b: Uuid,
    /// juncao_pedida ou pessoas_diferentes.
    pub decisao: String,
    /// Obrigatório quando a decisão substitui outra já registrada.
    pub motivo: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DecisaoRegistrada {
    pub id: Uuid,
    pub codigo: String,
    pub decisao: String,
}

/// Pares abertos (sem decisão "pessoas diferentes" vigente), para a contagem da aba.
pub(crate) async fn contar_abertos(tx: &mut PgConnection) -> Result<i64, ErroApi> {
    Ok(sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identidade.paciente a JOIN identidade.paciente b
             ON a.id < b.id AND a.unidade_id = b.unidade_id AND comum.sem_acento(a.nome) = comum.sem_acento(b.nome)
            AND (a.nascimento = b.nascimento OR a.celular = b.celular OR a.nome_mae = b.nome_mae
                 OR date_part('year', a.nascimento) = date_part('year', b.nascimento))
          WHERE NOT EXISTS (SELECT 1 FROM identidade.par_duplicado d
                             WHERE d.paciente_menor = a.id AND d.paciente_maior = b.id AND d.decisao = 'pessoas_diferentes'
                               AND NOT EXISTS (SELECT 1 FROM identidade.par_duplicado s WHERE s.substitui = d.id))"#
    )
    .fetch_one(tx)
    .await?)
}

/// Pares de cadastros parecidos da unidade (recepção e gerência), até 100.
#[utoipa::path(get, operation_id = "listar_pares_duplicados", path = "/v1/pacientes/pares-duplicados", tag = "pacientes",
    responses((status = 200, body = Vec<ParDuplicado>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(State(estado): State<Estado>, usuario: Usuario) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::PacienteCadastrar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let linhas = sqlx::query!(
        r#"SELECT a.id AS a_id, a.codigo AS a_codigo, a.nome AS a_nome, a.nascimento AS a_nasc, a.cpf AS a_cpf, a.celular AS a_cel,
                  b.id AS b_id, b.codigo AS b_codigo, b.nome AS b_nome, b.nascimento AS b_nasc, b.cpf AS b_cpf, b.celular AS b_cel,
                  (a.nascimento = b.nascimento) AS "mesmo_nasc!", coalesce(a.celular = b.celular, false) AS "mesmo_cel!",
                  coalesce(a.nome_mae = b.nome_mae, false) AS "mesma_mae!",
                  d.decisao AS "decisao?", d.criado_em AS "decidido_em?", u.nome AS "decidido_por?"
             FROM identidade.paciente a JOIN identidade.paciente b
               ON a.id < b.id AND a.unidade_id = b.unidade_id AND comum.sem_acento(a.nome) = comum.sem_acento(b.nome)
              AND (a.nascimento = b.nascimento OR a.celular = b.celular OR a.nome_mae = b.nome_mae
                   OR date_part('year', a.nascimento) = date_part('year', b.nascimento))
             LEFT JOIN LATERAL (SELECT x.decisao, x.criado_em, x.criado_por FROM identidade.par_duplicado x
                                 WHERE x.paciente_menor = a.id AND x.paciente_maior = b.id
                                   AND NOT EXISTS (SELECT 1 FROM identidade.par_duplicado s WHERE s.substitui = x.id)) d ON true
             LEFT JOIN identidade.usuario u ON u.id = d.criado_por
            WHERE d.decisao IS DISTINCT FROM 'pessoas_diferentes'
            ORDER BY a.nome, a.nascimento LIMIT 100"#
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mascara = |c: Option<String>| c.as_deref().map(crate::rotas::paciente_visao::mascarar_cpf);
    let pares: Vec<ParDuplicado> = linhas
        .into_iter()
        .map(|l| {
            let motivo = if l.mesmo_nasc {
                "Mesmo nome e mesmo nascimento"
            } else if l.mesmo_cel {
                "Mesmo nome e mesmo celular, nascimento diferente"
            } else if l.mesma_mae {
                "Mesmo nome e mesma mãe, nascimento diferente"
            } else {
                "Mesmo nome, nascimento no mesmo ano"
            };
            ParDuplicado {
                a: CadastroDoPar {
                    id: l.a_id,
                    codigo: l.a_codigo,
                    nome: l.a_nome,
                    nascimento: l.a_nasc,
                    cpf_mascarado: mascara(l.a_cpf),
                    celular: l.a_cel,
                },
                b: CadastroDoPar {
                    id: l.b_id,
                    codigo: l.b_codigo,
                    nome: l.b_nome,
                    nascimento: l.b_nasc,
                    cpf_mascarado: mascara(l.b_cpf),
                    celular: l.b_cel,
                },
                motivo: motivo.to_owned(),
                situacao: l.decisao.unwrap_or_else(|| "aberto".to_owned()),
                decidido_em: l.decidido_em,
                decidido_por: l.decidido_por,
            }
        })
        .collect();
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(pares)).into_response())
}

/// Registra a decisão sobre um par. Primeira decisão: única por par (`409 par-ja-decidido` para a
/// segunda simultânea). Nova decisão sobre par já decidido exige motivo e substitui a anterior.
#[utoipa::path(post, operation_id = "decidir_par_duplicado", path = "/v1/pacientes/pares-duplicados", tag = "pacientes",
    params(("Idempotency-Key" = Uuid, Header)), request_body = DecisaoSobreDuplicado,
    responses((status = 201, body = DecisaoRegistrada), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc),
              (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn decidir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(d): Corpo<DecisaoSobreDuplicado>,
) -> Result<Response, ErroApi> {
    ve_pacientes(&usuario)?;
    usuario.exigir(Ato::PacienteCadastrar)?;
    if d.decisao != "juncao_pedida" && d.decisao != "pessoas_diferentes" {
        return Err(ErroApi::invalido(
            "decisao-invalida",
            "Escolha pedir junção ou são pessoas diferentes. Nada foi registrado.",
        ));
    }
    if d.paciente_a == d.paciente_b {
        return Err(ErroApi::invalido(
            "par-invalido",
            "Escolha dois cadastros diferentes. Nada foi registrado.",
        ));
    }
    let motivo = d.motivo.as_deref().map(str::trim).filter(|m| !m.is_empty());
    if motivo.is_some_and(|m| !(5..=500).contains(&m.chars().count())) {
        return Err(ErroApi::invalido(
            "motivo-invalido",
            "Escreva o motivo com 5 a 500 letras. Nada foi registrado.",
        ));
    }
    let (menor, maior) = if d.paciente_a < d.paciente_b {
        (d.paciente_a, d.paciente_b)
    } else {
        (d.paciente_b, d.paciente_a)
    };
    const ROTA: &str = "POST /v1/pacientes/pares-duplicados";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, ROTA, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    // Os dois cadastros precisam existir nesta unidade (RLS) e ser parecidos de fato: ninguém marca
    // par de cadastros quaisquer pela rota (acesso por objeto, SEC).
    let parecidos = sqlx::query_scalar!(
        r#"SELECT (comum.sem_acento(a.nome) = comum.sem_acento(b.nome)) AS "ok!" FROM identidade.paciente a, identidade.paciente b
            WHERE a.id = $1 AND b.id = $2"#,
        menor,
        maior
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "paciente-nao-encontrado",
            "Cadastro não encontrado nesta unidade. Nada foi registrado.",
        )
    })?;
    if !parecidos {
        return Err(ErroApi::invalido(
            "par-nao-parecido",
            "Os cadastros não têm o mesmo nome: não são possível duplicado. Nada foi registrado.",
        ));
    }
    // A decisão vigente do par (a última, não substituída). Duas mudanças simultâneas tentam substituir a
    // mesma decisão: a restrição única de `substitui` deixa só uma passar (G2, sem lock em memória).
    let vigente = sqlx::query!(
        "SELECT id, decisao FROM identidade.par_duplicado x
          WHERE paciente_menor = $1 AND paciente_maior = $2
            AND NOT EXISTS (SELECT 1 FROM identidade.par_duplicado s WHERE s.substitui = x.id)",
        menor,
        maior
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(v) = &vigente {
        if v.decisao == d.decisao {
            return Err(ErroApi::conflito(
                "par-ja-decidido",
                "Este par já tem essa decisão registrada. Nada mudou.",
            ));
        }
        if motivo.is_none() {
            return Err(ErroApi::conflito(
                "par-ja-decidido",
                "Este par já tem outra decisão. Para mudar, escreva o motivo; a anterior fica no histórico.",
            ));
        }
    }
    let id = novo_id();
    let codigo = proximo(&mut tx, "DUP").await?;
    let inserido = sqlx::query!(
        "INSERT INTO identidade.par_duplicado (id, codigo, paciente_menor, paciente_maior, decisao, motivo, substitui, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'recepcao', $9, $10, $11)",
        id, codigo.como_str(), menor, maior, d.decisao, motivo, vigente.as_ref().map(|v| v.id), usuario.id, estacao, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await;
    if let Err(sqlx::Error::Database(e)) = &inserido {
        if e.is_unique_violation() {
            return Err(ErroApi::conflito(
                "par-ja-decidido",
                "Outra pessoa acabou de decidir sobre este par. Veja a decisão dela; nada foi registrado.",
            ));
        }
    }
    inserido?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "identidade",
            tipo: "par_duplicado_decidido",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "paciente_menor": menor, "paciente_maior": maior, "decisao": d.decisao, "substitui": vigente.as_ref().map(|v| v.id) }),
            criado_por: Some(usuario.id),
            origem: "recepcao",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(DecisaoRegistrada {
        id,
        codigo: codigo.como_str().to_owned(),
        decisao: d.decisao,
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}
