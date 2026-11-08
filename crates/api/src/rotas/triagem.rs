//! Triagem com contraste (docs/27 FC-TRI, docs/30 § 2). A TFG e o resultado são do servidor.

use crate::atos::Ato;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{Corpo, Estacao};
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use crate::{banco, tempo};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};
use vidacor_dominio_triagem::{avaliar, DadosTriagem, Sexo, REGRA_VERSAO};

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct Conduta {
    pub texto: String,
    pub medico_id: Uuid,
    pub registrada_em: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct Triagem {
    pub codigo: String,
    /// liberado, liberado_com_ressalvas ou precisa_aprovacao_medica.
    pub resultado: String,
    pub tfg_calculada: Option<i32>,
    pub motivos: Vec<String>,
    pub regra_versao: String,
    pub conduta: Option<Conduta>,
    /// Exame pode iniciar: liberado, com ressalvas, ou com conduta médica registrada.
    pub exame_liberado: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoTriagem {
    pub reacao_previa_contraste: bool,
    pub gestante: bool,
    pub amamentando: bool,
    pub usa_metformina: bool,
    /// Texto decimal com ponto (ex.: "1.4").
    pub creatinina_mg_dl: Option<String>,
    pub creatinina_data: Option<NaiveDate>,
    /// Ignorado: o servidor calcula a TFG (FC-TRI-07).
    pub tfg: Option<serde_json::Value>,
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoConduta {
    pub conduta: String,
}

/// # Errors
/// Falha de banco.
pub async fn carregar(
    conexao: &mut PgConnection,
    atendimento_id: Uuid,
) -> Result<Option<Triagem>, ErroApi> {
    let Some(l) = sqlx::query!(
        "SELECT codigo, resultado, tfg_calculada, motivos, regra_versao, conduta, conduta_por, conduta_em
           FROM triagem.triagem WHERE atendimento_id = $1",
        atendimento_id
    )
    .fetch_optional(conexao)
    .await?
    else {
        return Ok(None);
    };
    let conduta = match (l.conduta, l.conduta_por, l.conduta_em) {
        (Some(texto), Some(medico_id), Some(registrada_em)) => Some(Conduta {
            texto,
            medico_id,
            registrada_em,
        }),
        _ => None,
    };
    let exame_liberado = l.resultado != "precisa_aprovacao_medica" || conduta.is_some();
    Ok(Some(Triagem {
        codigo: l.codigo,
        resultado: l.resultado,
        tfg_calculada: l.tfg_calculada,
        motivos: l.motivos,
        regra_versao: l.regra_versao,
        conduta,
        exame_liberado,
    }))
}

fn idade_em(nascimento: NaiveDate, hoje: NaiveDate) -> u32 {
    let mut anos = hoje.year() - nascimento.year();
    if (hoje.month(), hoje.day()) < (nascimento.month(), nascimento.day()) {
        anos -= 1;
    }
    u32::try_from(anos.max(0)).unwrap_or(0)
}

fn nao_encontrado() -> ErroApi {
    ErroApi::nao_encontrado(
        "atendimento-nao-encontrado",
        "Este atendimento não existe ou não é desta unidade. Nada foi gravado. Volte à fila da sala.",
    )
}

/// Registra a triagem de segurança (técnico lotado na sala do atendimento).
#[utoipa::path(post, path = "/v1/atendimentos/{id}/triagem", tag = "triagem", params(("id" = Uuid, Path)),
    request_body = PedidoTriagem,
    responses((status = 201, body = Triagem), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn registrar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoTriagem>,
) -> Result<(StatusCode, Json<Triagem>), ErroApi> {
    usuario.exigir(Ato::TriagemRegistrar)?;
    let hoje = tempo::hoje_local();
    let creatinina = match (&p.creatinina_mg_dl, p.creatinina_data) {
        (Some(texto), Some(data)) => {
            let valor: f64 = texto.trim().replace(',', ".").parse().map_err(|_| {
                ErroApi::invalido(
                    "creatinina-invalida",
                    "A creatinina deve ser um número em mg/dL, como 1.4. Nada foi gravado.",
                )
            })?;
            if !(valor > 0.0 && valor < 30.0) || data > hoje {
                return Err(ErroApi::invalido(
                    "creatinina-invalida",
                    "A creatinina ou a data do exame estão fora do esperado. Nada foi gravado. Confira o resultado do laboratório.",
                ));
            }
            Some((
                valor,
                (hoje - data).num_days(),
                texto.trim().replace(',', "."),
                data,
            ))
        }
        (None, None) => None,
        _ => {
            return Err(ErroApi::invalido(
                "creatinina-incompleta",
                "Informe a creatinina e a data do exame juntas. Nada foi gravado.",
            ))
        }
    };
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let a = sqlx::query!(
        r#"SELECT t.id, t.sala_id, t.unidade_id, t.paciente_id, p.nascimento, p.sexo
             FROM recepcao.atendimento t JOIN identidade.paciente p ON p.id = t.paciente_id
            WHERE t.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(nao_encontrado)?;
    exigir_lotacao(&mut tx, usuario.id, a.sala_id).await?;
    let sexo = match a.sexo.as_deref() {
        Some("F") => Sexo::Feminino,
        Some("M") => Sexo::Masculino,
        _ => {
            return Err(ErroApi::invalido(
                "cadastro-sem-sexo",
                "O cadastro do paciente não tem o sexo, necessário para calcular a função renal. Nada foi gravado. Peça a correção do cadastro à recepção.",
            ))
        }
    };
    let avaliacao = avaliar(&DadosTriagem {
        reacao_previa_contraste: p.reacao_previa_contraste,
        gestante: p.gestante,
        amamentando: p.amamentando,
        usa_metformina: p.usa_metformina,
        creatinina: creatinina.as_ref().map(|c| (c.0, c.1)),
        idade_anos: idade_em(a.nascimento, hoje),
        sexo,
    });
    let ano = tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero('TRI', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::interno("contador TRI sem valor"))?;
    let codigo = CodigoLegivel::novo("TRI", ano, n);
    let triagem_id = novo_id();
    let tfg = avaliacao.tfg.map(|t| i32::try_from(t).unwrap_or(i32::MAX));
    let creatinina_texto = creatinina.as_ref().map(|c| c.2.clone());
    sqlx::query!(
        "INSERT INTO triagem.triagem
           (id, codigo, atendimento_id, paciente_id, reacao_previa_contraste, gestante, amamentando, usa_metformina,
            creatinina_mg_dl, creatinina_data, tfg_calculada, resultado, motivos, regra_versao,
            criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::text::numeric, $10, $11, $12, $13, $14, $15, 'sala', $16, $17, $18)",
        triagem_id,
        codigo.como_str(),
        a.id,
        a.paciente_id,
        p.reacao_previa_contraste,
        p.gestante,
        p.amamentando,
        p.usa_metformina,
        creatinina_texto,
        creatinina.as_ref().map(|c| c.3),
        tfg,
        avaliacao.resultado.como_str(),
        &avaliacao.motivos,
        REGRA_VERSAO,
        usuario.id,
        estacao,
        a.unidade_id,
        correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito(
            "triagem-ja-registrada",
            "A triagem deste atendimento já foi registrada. Nada foi alterado. Veja o resultado na tela do atendimento.",
        ),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "triagem",
            tipo: "triagem_registrada",
            registro_id: triagem_id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "resultado": avaliacao.resultado.como_str(), "regra_versao": REGRA_VERSAO, "estacao": estacao }),
            criado_por: Some(usuario.id),
            origem: "sala",
            unidade_id: a.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let triagem = carregar(&mut tx, a.id)
        .await?
        .ok_or_else(|| ErroApi::interno("triagem recém-gravada sumiu"))?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(triagem)))
}

/// Registra a conduta médica que libera a triagem (médico com vínculo; reautenticação).
#[utoipa::path(post, path = "/v1/atendimentos/{id}/triagem/conduta", tag = "triagem", params(("id" = Uuid, Path)),
    request_body = PedidoConduta,
    responses((status = 200, body = Triagem), (status = 401, body = crate::openapi::ProblemaDoc),
              (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn conduta(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoConduta>,
) -> Result<Json<Triagem>, ErroApi> {
    usuario.exigir(Ato::TriagemConduta)?;
    usuario.exigir_reautenticacao()?;
    let texto = p.conduta.trim().to_owned();
    if texto.chars().count() < 5 || texto.chars().count() > 2000 {
        return Err(ErroApi::invalido(
            "conduta-invalida",
            "Descreva a conduta (de 5 a 2.000 caracteres). Nada foi gravado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let t = sqlx::query!(
        "SELECT t.id, t.codigo, t.paciente_id, t.unidade_id, t.resultado, t.conduta
           FROM triagem.triagem t WHERE t.atendimento_id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "triagem-nao-encontrada",
            "Não há triagem registrada para este atendimento. Nada foi gravado.",
        )
    })?;
    if !crate::vinculo::pode_ler(&mut tx, t.paciente_id).await? {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "sem-vinculo-de-cuidado",
            "Sem vínculo com o paciente",
            "Você não tem vínculo de cuidado com este paciente. Nada foi gravado. Se for urgência, use o acesso de emergência com justificativa.",
        ));
    }
    if t.resultado != "precisa_aprovacao_medica" || t.conduta.is_some() {
        return Err(ErroApi::conflito(
            "conduta-desnecessaria",
            "Esta triagem não espera conduta médica. Nada foi alterado.",
        ));
    }
    sqlx::query!(
        "UPDATE triagem.triagem SET conduta = $2, conduta_por = $3, conduta_em = now(), versao = versao + 1
          WHERE id = $1 AND conduta IS NULL",
        t.id,
        texto,
        usuario.id
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "triagem",
            tipo: "conduta_registrada",
            registro_id: t.id,
            registro_codigo: Some(&t.codigo),
            dados: json!({ "regra_versao": REGRA_VERSAO, "reautenticado": true }),
            criado_por: Some(usuario.id),
            origem: "consultorio",
            unidade_id: t.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let triagem = carregar(&mut tx, id)
        .await?
        .ok_or_else(|| ErroApi::interno("triagem sumiu"))?;
    tx.commit().await?;
    Ok(Json(triagem))
}

/// O técnico só atua na sala em que está lotado (FC-PUL-05).
///
/// # Errors
/// 403 quando não está lotado na sala.
pub async fn exigir_lotacao(
    conexao: &mut PgConnection,
    usuario_id: Uuid,
    sala_id: Uuid,
) -> Result<(), ErroApi> {
    let lotado = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM identidade.lotacao WHERE usuario_id = $1 AND sala_id = $2) AS \"lotado!\"",
        usuario_id,
        sala_id
    )
    .fetch_one(conexao)
    .await?;
    if lotado {
        Ok(())
    } else {
        Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "fora-da-sala",
            "Outra sala",
            "Você não está lotado nesta sala. Nada foi feito. O exame deve ser feito pela equipe da sala do paciente.",
        ))
    }
}
