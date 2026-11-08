//! Cadastro de paciente com busca de possível duplicado antes de criar (Cadastro-Paciente.dc.html,
//! docs/04). A regra de duplicado é do domínio (`dominio_recepcao::cadastro`); aqui só a busca dos
//! candidatos e a gravação com versão, lastro e `Idempotency-Key`.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::PacienteResumo;
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::{novo_id, DataNascimento, NomeCompleto};
use vidacor_dominio_recepcao::cadastro::{cpf_digitos, cpf_valido, possivel_duplicado};

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct PossivelDuplicado {
    pub paciente: PacienteResumo,
    pub codigo: String,
    /// Por que parece a mesma pessoa.
    pub motivo: String,
    /// Mesmo CPF: não pode ser descartado (o CPF é único).
    pub bloqueante: bool,
}

async fn candidatos(
    tx: &mut sqlx::PgConnection,
    nome: &NomeCompleto,
    nascimento: NaiveDate,
    cpf: Option<&str>,
) -> Result<Vec<PossivelDuplicado>, ErroApi> {
    let primeiro = nome
        .forma_de_comparacao()
        .split(' ')
        .next()
        .unwrap_or_default()
        .to_owned();
    let linhas = sqlx::query!(
        r#"SELECT id, codigo, nome, nascimento, cpf FROM identidade.paciente
           WHERE ($3::text IS NOT NULL AND cpf = $3)
              OR nascimento = $1
              OR (nascimento BETWEEN $1 - 400 AND $1 + 400
                  AND translate(lower(nome), 'áàâãäéèêëíìîïóòôõöúùûüç', 'aaaaaeeeeiiiiooooouuuuc') LIKE $2 || '%')
           ORDER BY nome LIMIT 200"#,
        nascimento,
        primeiro,
        cpf
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .filter_map(|l| {
            let existente = NomeCompleto::novo(&l.nome).ok()?;
            let m = possivel_duplicado(
                nome,
                nascimento,
                cpf,
                &existente,
                l.nascimento,
                l.cpf.as_deref(),
            )?;
            Some(PossivelDuplicado {
                bloqueante: m == vidacor_dominio_recepcao::cadastro::MotivoDeDuplicado::MesmoCpf,
                motivo: m.texto().to_owned(),
                codigo: l.codigo,
                paciente: PacienteResumo {
                    id: l.id,
                    nome: l.nome,
                    nascimento: l.nascimento,
                },
            })
        })
        .collect())
}

fn pode_cadastrar(u: &Usuario) -> Result<(), ErroApi> {
    u.exigir(Ato::PacienteCadastrar)
}

#[derive(Deserialize, IntoParams)]
pub struct ConsultaDuplicado {
    pub nome: String,
    pub nascimento: NaiveDate,
    pub cpf: Option<String>,
}

/// Possíveis duplicados antes de salvar (a tela pergunta "é a mesma pessoa?").
#[utoipa::path(get, path = "/v1/pacientes/possiveis-duplicados", tag = "pacientes", params(ConsultaDuplicado),
    responses((status = 200, body = Vec<PossivelDuplicado>), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn duplicados(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(q): Query<ConsultaDuplicado>,
) -> Result<Json<Vec<PossivelDuplicado>>, ErroApi> {
    pode_cadastrar(&usuario)?;
    let nome = NomeCompleto::novo(&q.nome).map_err(|_| {
        ErroApi::invalido("nome-invalido", "Digite o nome completo. Nada foi buscado.")
    })?;
    let cpf = q.cpf.as_deref().map(cpf_digitos).filter(|c| !c.is_empty());
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = candidatos(&mut tx, &nome, q.nascimento, cpf.as_deref()).await?;
    tx.commit().await?;
    Ok(Json(r))
}

#[derive(Deserialize, ToSchema)]
pub struct NovoPaciente {
    pub nome: String,
    pub nome_social: Option<String>,
    pub nascimento: NaiveDate,
    /// F ou M.
    pub sexo: String,
    pub cpf: Option<String>,
    pub nome_mae: Option<String>,
    /// Só dígitos, com DDD (10 ou 11).
    pub celular: String,
    pub email: Option<String>,
    #[serde(default)]
    pub alergias: Vec<String>,
    #[serde(default)]
    pub nega_alergias: bool,
    /// A recepção conferiu os possíveis duplicados mostrados e confirma que é outra pessoa.
    #[serde(default)]
    pub duplicado_conferido: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PacienteCadastrado {
    pub id: Uuid,
    pub codigo: String,
    pub nome: String,
    pub nascimento: NaiveDate,
}

fn vazio_e_none(s: Option<&String>) -> Option<String> {
    s.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty())
}

/// Cadastra o paciente (recepção). Possível duplicado não conferido: `409 possivel-duplicado` com a lista.
#[utoipa::path(post, path = "/v1/pacientes", tag = "pacientes", request_body = NovoPaciente, params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = PacienteCadastrado), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn cadastrar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(p): Corpo<NovoPaciente>,
) -> Result<Response, ErroApi> {
    pode_cadastrar(&usuario)?;
    let nome = NomeCompleto::novo(&p.nome).map_err(|_| {
        ErroApi::invalido(
            "nome-invalido",
            "Digite o nome completo (nome e sobrenome). Nada foi gravado.",
        )
    })?;
    let nascimento = DataNascimento::nova(p.nascimento, crate::tempo::hoje_local())
        .map_err(|_| {
            ErroApi::invalido(
                "nascimento-invalido",
                "A data de nascimento não pode ser futura nem anterior a 1900. Nada foi gravado.",
            )
        })?
        .valor();
    if p.sexo != "F" && p.sexo != "M" {
        return Err(ErroApi::invalido("sexo-obrigatorio", "Informe o sexo do cadastro (usado em exames e no cálculo da função renal). Nada foi gravado."));
    }
    let celular: String = p.celular.chars().filter(char::is_ascii_digit).collect();
    if !(10..=11).contains(&celular.len()) {
        return Err(ErroApi::invalido(
            "celular-invalido",
            "Digite o celular com DDD (10 ou 11 números). Nada foi gravado.",
        ));
    }
    let cpf = vazio_e_none(p.cpf.as_ref());
    if cpf.as_deref().is_some_and(|c| !cpf_valido(c)) {
        return Err(ErroApi::invalido(
            "cpf-invalido",
            "O CPF não confere. Corrija ou deixe em branco. Nada foi gravado.",
        ));
    }
    let cpf = cpf.map(|c| cpf_digitos(&c));
    let alergias: Vec<String> = p
        .alergias
        .iter()
        .map(|a| a.trim().to_owned())
        .filter(|a| !a.is_empty())
        .collect();
    if alergias.is_empty() == !p.nega_alergias {
        return Err(ErroApi::invalido("alergias-obrigatorias", "Informe as alergias ou marque que o paciente nega alergias (só um dos dois). Nada foi gravado."));
    }
    let email = vazio_e_none(p.email.as_ref());
    if email
        .as_deref()
        .is_some_and(|e| !e.contains('@') || e.len() > 254)
    {
        return Err(ErroApi::invalido(
            "email-invalido",
            "O e-mail não parece certo. Corrija ou deixe em branco. Nada foi gravado.",
        ));
    }
    const ROTA: &str = "POST /v1/pacientes";
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
    let parecidos = candidatos(&mut tx, &nome, nascimento, cpf.as_deref()).await?;
    if parecidos.iter().any(|c| c.bloqueante) {
        return Err(ErroApi::conflito(
            "cpf-ja-cadastrado",
            "Este CPF já está em outro cadastro. Nada foi gravado. Abra o cadastro existente.",
        )
        .com_extra("candidatos", json!(parecidos)));
    }
    if !parecidos.is_empty() && !p.duplicado_conferido {
        return Err(ErroApi::conflito("possivel-duplicado", "Há cadastro parecido. Nada foi gravado. Confira nome e nascimento: se for a mesma pessoa, use o cadastro existente.")
            .com_extra("candidatos", json!(parecidos)));
    }
    let id = novo_id();
    let codigo = proximo(&mut tx, "PAC").await?;
    sqlx::query!(
        "INSERT INTO identidade.paciente (id, codigo, nome, nascimento, sexo, nome_social, cpf, nome_mae, celular, email, alergias, nega_alergias, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'recepcao', $14, $15)",
        id, codigo.como_str(), nome.como_str(), nascimento, p.sexo, vazio_e_none(p.nome_social.as_ref()), cpf, vazio_e_none(p.nome_mae.as_ref()),
        celular, email, &alergias, p.nega_alergias, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.constraint() == Some("paciente_cpf_unico") => ErroApi::conflito("cpf-ja-cadastrado", "Este CPF já está em outro cadastro. Nada foi gravado."),
        _ => ErroApi::from(e),
    })?;
    sqlx::query!(
        "INSERT INTO identidade.paciente_versao (id, paciente_id, versao, nome, nascimento, sexo, motivo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, 1, $3, $4, $5, 'cadastro inicial', $6, 'recepcao', $7, $8)",
        novo_id(), id, nome.como_str(), nascimento, p.sexo, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    // Lastro sem dado pessoal: só identificadores e a decisão sobre os parecidos.
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "identidade", tipo: "paciente_cadastrado", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "duplicado_conferido": p.duplicado_conferido, "parecidos": parecidos.iter().map(|c| c.paciente.id).collect::<Vec<_>>() }),
        criado_por: Some(usuario.id), origem: "recepcao", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let corpo = serde_json::to_value(PacienteCadastrado {
        id,
        codigo: codigo.to_string(),
        nome: nome.como_str().to_owned(),
        nascimento,
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}
