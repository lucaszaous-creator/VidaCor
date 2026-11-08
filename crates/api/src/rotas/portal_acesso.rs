//! Convite e login do portal do paciente (docs/25 Fase 1; SEC-AUT-6, SEC-DIS-5, SEC-CRI-4).
//! - A recepção emite o convite: token aleatório de uso único, 72 h, um vivo por paciente; o link
//!   aparece uma vez na tela (o banco guarda só o hash).
//! - O paciente aceita conferindo o nascimento (segundo identificador) e escolhe login e senha
//!   (argon2id). O aceite é atômico (G2); 5 erros revogam o convite.
//! - Login com mensagem única para login ou senha errados; 5 erros bloqueiam a conta por 15 min.

use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::portal::{self, Paciente};
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

pub const VALIDADE_CONVITE_HORAS: i64 = 72;
pub const MAX_TENTATIVAS: i32 = 5;
pub const BLOQUEIO_MIN: i64 = 15;

fn hash_token(t: &str) -> Vec<u8> {
    Sha256::digest(t.as_bytes()).to_vec()
}

fn token_valido(t: &str) -> bool {
    (20..=64).contains(&t.len())
        && t.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn cookie(segredo: &str, seguro: bool) -> String {
    let seguro = if seguro { "; Secure" } else { "" };
    format!(
        "{}={segredo}; Path=/; HttpOnly; SameSite=Strict; Max-Age=1800{seguro}",
        portal::COOKIE
    )
}

/// Token do convite: HMAC(segredo do servidor, id do convite). Recalculável só pelo servidor; o
/// banco guarda o hash para a busca e nada guarda o token (DEC-2026-11-08-60).
fn token_do_convite(segredo: &str, id: Uuid) -> Result<String, ErroApi> {
    use hmac::{Hmac, Mac};
    let mut mac = Hmac::<Sha256>::new_from_slice(segredo.as_bytes()).map_err(ErroApi::interno)?;
    mac.update(id.as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn mascarar(email: &str) -> String {
    let (nome, dominio) = email.split_once('@').unwrap_or((email, ""));
    let inicio: String = nome.chars().take(2).collect();
    format!("{inicio}***@{dominio}")
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ConviteEmitido {
    pub id: Uuid,
    pub codigo: String,
    pub expira_em: DateTime<Utc>,
    /// E-mail do cadastro, mascarado (o link vai só para lá; a recepção não o vê).
    pub enviado_para: String,
}

/// A recepção convida o paciente para o portal: o link vai por e-mail para o endereço do cadastro
/// (convite novo revoga o anterior; serve também para quem esqueceu a senha).
#[utoipa::path(post, operation_id = "convidar_para_o_portal", path = "/v1/pacientes/{id}/convites-portal", tag = "portal", params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = ConviteEmitido), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 422, body = crate::openapi::ProblemaDoc), (status = 503, body = crate::openapi::ProblemaDoc)))]
pub async fn convidar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(paciente_id): Path<Uuid>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::PortalConvidar)?;
    let segredo = estado.config.convite_segredo.clone().ok_or_else(|| {
        ErroApi::novo(StatusCode::SERVICE_UNAVAILABLE, "convite-indisponivel-no-servidor", "Convite indisponível",
            "O servidor ainda não está configurado para convites do portal. Nada foi enviado. Avise a TI.")
    })?;
    let rota = "POST /v1/pacientes/{id}/convites-portal";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, rota, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let email = sqlx::query_scalar!("SELECT email FROM identidade.paciente WHERE id = $1", paciente_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::nao_encontrado("paciente-nao-encontrado", "Paciente não encontrado. Nenhum convite foi criado."))?
        .ok_or_else(|| ErroApi::invalido("paciente-sem-email",
            "O cadastro não tem e-mail. Nada foi enviado. Confira o e-mail com o paciente e corrija o cadastro antes de convidar."))?;
    sqlx::query!(
        "UPDATE portal.convite SET revogado_em = now(), motivo_revogacao = 'convite novo', versao = versao + 1
          WHERE paciente_id = $1 AND usado_em IS NULL AND revogado_em IS NULL",
        paciente_id
    )
    .execute(&mut *tx)
    .await?;
    let id = novo_id();
    let token = token_do_convite(&segredo, id)?;
    let codigo = proximo(&mut tx, "CNV").await?;
    let expira_em = Utc::now() + Duration::hours(VALIDADE_CONVITE_HORAS);
    sqlx::query!(
        "INSERT INTO portal.convite (id, codigo, paciente_id, token_hash, expira_em, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'recepcao', $7, $8)",
        id, codigo.como_str(), paciente_id, hash_token(&token), expira_em, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    // O e-mail sai pelo outbox com chave de idempotência (G3); a carga leva só o ID do convite.
    sqlx::query!(
        "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
         VALUES ($1, 'email.convite_portal', $2, $3, $4, $5, $6)",
        novo_id(), novo_id(), json!({ "convite_id": id }), usuario.unidade_id, correlacao, id
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "portal",
            tipo: "convite_emitido",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "paciente": paciente_id, "expira_em": expira_em, "canal": "email" }),
            criado_por: Some(usuario.id),
            origem: "recepcao",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(ConviteEmitido {
        id,
        codigo: codigo.to_string(),
        expira_em,
        enviado_para: mascarar(&email),
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Trabalhador do outbox: recalcula o link e envia o e-mail do convite. Convite já usado, revogado ou
/// vencido não é enviado (conclui sem enviar). A mensagem não tem dado de saúde.
///
/// # Errors
/// Correio ou servidor sem configuração, ou falha de envio (o item volta ao outbox com espera).
pub async fn enviar_convite(
    estado: &Estado,
    unidade_id: Uuid,
    convite_id: Uuid,
    chave: Uuid,
) -> anyhow::Result<()> {
    let correio = estado
        .correio
        .clone()
        .ok_or_else(|| anyhow::anyhow!("correio não configurado"))?;
    let segredo = estado
        .config
        .convite_segredo
        .clone()
        .ok_or_else(|| anyhow::anyhow!("segredo do convite não configurado"))?;
    let sistema = ContextoBanco {
        usuario_id: None,
        unidade_id,
        perfis: vec!["sistema".into()],
    };
    let mut tx = banco::transacao(&estado.banco, &sistema).await?;
    let c = sqlx::query!(
        r#"SELECT c.codigo, c.expira_em, c.usado_em, c.revogado_em, p.nome, p.email AS "email?"
           FROM portal.convite c JOIN identidade.paciente p ON p.id = c.paciente_id WHERE c.id = $1"#,
        convite_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if c.usado_em.is_some() || c.revogado_em.is_some() || c.expira_em <= Utc::now() {
        tx.commit().await?;
        return Ok(());
    }
    let para = c
        .email
        .ok_or_else(|| anyhow::anyhow!("cadastro sem e-mail"))?;
    let token = token_do_convite(&segredo, convite_id)
        .map_err(|e| anyhow::anyhow!(e.codigo().to_owned()))?;
    let link = format!(
        "{}/portal/convite/{token}",
        estado.config.url_aplicacao.trim_end_matches('/')
    );
    let primeiro = c.nome.split_whitespace().next().unwrap_or_default();
    let validade = c
        .expira_em
        .with_timezone(&crate::tempo::FUSO)
        .format("%d/%m/%Y às %H:%M");
    let m = crate::correio::Mensagem {
        para,
        assunto: "Seu acesso ao portal VidaCor".into(),
        texto: format!(
            "Olá, {primeiro}.\n\nA VidaCor Imagem criou o seu acesso ao portal de resultados. Para começar, abra o link abaixo, confirme a sua data de nascimento e crie a sua senha:\n\n{link}\n\nO link vale até {validade} e pode ser usado uma vez só. Se não foi você quem pediu, ignore este e-mail.\n\nVidaCor Imagem"
        ),
    };
    correio.enviar(chave, &m).await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "portal",
            tipo: "convite_enviado",
            registro_id: convite_id,
            registro_codigo: Some(&c.codigo),
            dados: json!({ "canal": "email" }),
            criado_por: None,
            origem: "sistema",
            unidade_id,
            correlacao: &chave.to_string(),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SituacaoDoConvite {
    /// valido ou indisponivel (vencido, usado, revogado ou inexistente: a mesma resposta).
    pub situacao: String,
}

struct Convite {
    id: Uuid,
    paciente_id: Uuid,
    unidade_id: Uuid,
    expira_em: DateTime<Utc>,
    usado_em: Option<DateTime<Utc>>,
    revogado_em: Option<DateTime<Utc>>,
    tentativas: i32,
}

impl Convite {
    fn valido(&self) -> bool {
        self.usado_em.is_none()
            && self.revogado_em.is_none()
            && self.expira_em > Utc::now()
            && self.tentativas < MAX_TENTATIVAS
    }
}

async fn buscar(estado: &Estado, token: &str) -> Result<Option<Convite>, ErroApi> {
    if !token_valido(token) {
        return Ok(None);
    }
    let l = sqlx::query!(
        r#"SELECT id AS "id!", paciente_id AS "paciente_id!", unidade_id AS "unidade_id!", expira_em AS "expira_em!",
                  usado_em, revogado_em, tentativas AS "tentativas!" FROM portal.convite_por_hash($1)"#,
        hash_token(token)
    )
    .fetch_optional(&estado.banco)
    .await?;
    Ok(l.map(|l| Convite {
        id: l.id,
        paciente_id: l.paciente_id,
        unidade_id: l.unidade_id,
        expira_em: l.expira_em,
        usado_em: l.usado_em,
        revogado_em: l.revogado_em,
        tentativas: l.tentativas,
    }))
}

fn indisponivel() -> ErroApi {
    ErroApi::novo(StatusCode::GONE, "convite-indisponivel", "Convite indisponível",
        "Este convite venceu, já foi usado ou foi trocado por outro. Nada foi alterado. Peça um convite novo na recepção.")
}

/// Situação do convite antes de pedir os dados (sem dizer de quem é).
#[utoipa::path(get, operation_id = "situacao_do_convite", path = "/v1/portal/convites/{token}", tag = "portal", params(("token" = String, Path)),
    responses((status = 200, body = SituacaoDoConvite)))]
pub async fn situacao(
    State(estado): State<Estado>,
    Path(token): Path<String>,
) -> Result<Json<SituacaoDoConvite>, ErroApi> {
    let valido = buscar(&estado, &token).await?.is_some_and(|c| c.valido());
    Ok(Json(SituacaoDoConvite {
        situacao: if valido { "valido" } else { "indisponivel" }.into(),
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct Aceite {
    /// Nascimento do paciente (AAAA-MM-DD), conferido com o cadastro.
    pub nascimento: NaiveDate,
    pub login: String,
    pub senha: String,
}

fn validar_credenciais(login: &str, senha: &str) -> Result<String, ErroApi> {
    let login = login.trim().to_lowercase();
    let login_ok = (4..=80).contains(&login.len())
        && login
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".@_+-".contains(&b));
    if !login_ok {
        return Err(ErroApi::invalido("login-invalido", "O login precisa ter de 4 a 80 letras sem acento, números ou . _ @ + -. Nada foi gravado."));
    }
    let n = senha.chars().count();
    if !(10..=128).contains(&n) {
        return Err(ErroApi::invalido(
            "senha-fraca",
            "A senha precisa ter pelo menos 10 caracteres. Nada foi gravado.",
        ));
    }
    Ok(login)
}

fn hash_senha(senha: &str) -> Result<String, ErroApi> {
    let sal = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(senha.as_bytes(), &sal)
        .map(|h| h.to_string())
        .map_err(|e| ErroApi::interno(e.to_string()))
}

/// Hash calculado uma vez, usado quando o login não existe (mesmo custo de conferência).
fn hash_de_referencia() -> Result<String, ErroApi> {
    static REFERENCIA: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    if let Some(h) = REFERENCIA.get() {
        return Ok(h.clone());
    }
    let h = hash_senha("referencia-sem-conta-vidacor")?;
    Ok(REFERENCIA.get_or_init(|| h).clone())
}

/// O paciente aceita o convite: confere o nascimento, escolhe login e senha e já entra.
#[utoipa::path(post, operation_id = "aceitar_convite", path = "/v1/portal/convites/{token}/aceite", tag = "portal", params(("token" = String, Path)), request_body = Aceite,
    responses((status = 204), (status = 409, body = crate::openapi::ProblemaDoc), (status = 410, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn aceitar(
    State(estado): State<Estado>,
    Path(token): Path<String>,
    Corpo(a): Corpo<Aceite>,
) -> Result<Response, ErroApi> {
    let c = buscar(&estado, &token)
        .await?
        .filter(Convite::valido)
        .ok_or_else(indisponivel)?;
    let login = validar_credenciais(&a.login, &a.senha)?;
    let contexto = ContextoBanco {
        usuario_id: None,
        unidade_id: c.unidade_id,
        perfis: vec!["sistema".into()],
    };
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &contexto).await?;
    let nascimento = sqlx::query_scalar!(
        "SELECT nascimento FROM identidade.paciente WHERE id = $1",
        c.paciente_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if nascimento != a.nascimento {
        // Conta a tentativa fora do rollback: o erro precisa ficar registrado (SEC-DIS-5).
        tx.rollback().await?;
        let mut t2 = banco::transacao(&estado.banco, &contexto).await?;
        let tentativas = sqlx::query_scalar!(
            "UPDATE portal.convite SET tentativas = tentativas + 1,
                    revogado_em = CASE WHEN tentativas + 1 >= $2 THEN now() ELSE revogado_em END,
                    motivo_revogacao = CASE WHEN tentativas + 1 >= $2 THEN 'tentativas esgotadas' ELSE motivo_revogacao END
              WHERE id = $1 RETURNING tentativas",
            c.id, MAX_TENTATIVAS
        )
        .fetch_one(&mut *t2)
        .await?;
        t2.commit().await?;
        let restam = (MAX_TENTATIVAS - tentativas).max(0);
        return Err(if restam == 0 {
            indisponivel()
        } else {
            ErroApi::invalido("nascimento-nao-confere", &format!("A data de nascimento não confere com o cadastro. Nada foi gravado. Restam {restam} tentativas."))
        });
    }
    let usado = sqlx::query_scalar!(
        "UPDATE portal.convite SET usado_em = now(), versao = versao + 1
          WHERE id = $1 AND usado_em IS NULL AND revogado_em IS NULL AND expira_em > now() RETURNING id",
        c.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if usado.is_none() {
        return Err(indisponivel());
    }
    let senha_hash = hash_senha(&a.senha)?;
    let conta = sqlx::query_scalar!(
        "INSERT INTO portal.conta (id, paciente_id, login, senha_hash, convite_id, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, 'portal', $6, $7)
         ON CONFLICT (paciente_id) DO UPDATE SET login = EXCLUDED.login, senha_hash = EXCLUDED.senha_hash, convite_id = EXCLUDED.convite_id,
                ativa = true, falhas = 0, bloqueada_ate = NULL, versao = portal.conta.versao + 1
         RETURNING id",
        novo_id(), c.paciente_id, login, senha_hash, c.id, c.unidade_id, correlacao
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("login-indisponivel", "Este login não pode ser usado. Escolha outro. Nada foi gravado."),
        _ => ErroApi::from(e),
    })?;
    // Convite novo também serve para recuperar o acesso: as sessões antigas caem.
    sqlx::query!(
        "UPDATE portal.sessao SET revogada_em = now() WHERE conta_id = $1 AND revogada_em IS NULL",
        conta
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "portal",
            tipo: "convite_aceito",
            registro_id: c.id,
            registro_codigo: None,
            dados: json!({ "conta": conta }),
            criado_por: None,
            origem: "portal",
            unidade_id: c.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    let segredo = portal::criar_sessao(&estado.banco, conta).await?;
    Ok((
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            cookie(&segredo, estado.config.cookie_seguro),
        )],
    )
        .into_response())
}

#[derive(Deserialize, ToSchema)]
pub struct Entrada {
    pub login: String,
    pub senha: String,
}

fn nao_confere() -> ErroApi {
    ErroApi::novo(StatusCode::UNAUTHORIZED, "login-nao-confere", "Login ou senha não conferem",
        "Login ou senha não conferem. Confira e tente de novo; depois de 5 erros o acesso espera 15 minutos. Esqueceu? Peça um convite novo na recepção.")
}

/// Login do paciente no portal (mensagem única: não diz se o login existe).
#[utoipa::path(post, operation_id = "portal_entrar", path = "/v1/portal/sessao", tag = "portal", request_body = Entrada,
    responses((status = 204), (status = 401, body = crate::openapi::ProblemaDoc), (status = 429, body = crate::openapi::ProblemaDoc)))]
pub async fn entrar(
    State(estado): State<Estado>,
    Corpo(e): Corpo<Entrada>,
) -> Result<Response, ErroApi> {
    let login = e.login.trim().to_lowercase();
    if login.len() > 80 || e.senha.len() > 128 {
        return Err(nao_confere());
    }
    let conta = sqlx::query!(
        "SELECT id, senha_hash, ativa, bloqueada_ate FROM portal.conta WHERE login = $1",
        login
    )
    .fetch_optional(&estado.banco)
    .await?;
    // Sem conta: faz o mesmo trabalho de conferência, para o tempo não revelar nada.
    let hash = match conta.as_ref().and_then(|c| c.senha_hash.clone()) {
        Some(h) => h,
        None => hash_de_referencia()?,
    };
    let confere = PasswordHash::new(&hash).is_ok_and(|h| {
        Argon2::default()
            .verify_password(e.senha.as_bytes(), &h)
            .is_ok()
    });
    let Some(conta) = conta else {
        return Err(nao_confere());
    };
    if conta.bloqueada_ate.is_some_and(|b| b > Utc::now()) {
        return Err(ErroApi::novo(StatusCode::TOO_MANY_REQUESTS, "acesso-em-espera", "Acesso em espera",
            "Muitas tentativas erradas. O acesso volta em até 15 minutos. Nada foi alterado. Se esqueceu a senha, peça um convite novo na recepção."));
    }
    if !confere || !conta.ativa {
        sqlx::query!(
            "UPDATE portal.conta SET falhas = falhas + 1,
                    bloqueada_ate = CASE WHEN falhas + 1 >= $2 THEN now() + make_interval(mins => $3) ELSE bloqueada_ate END
              WHERE id = $1",
            conta.id, MAX_TENTATIVAS, BLOQUEIO_MIN as i32
        )
        .execute(&estado.banco)
        .await
        .ok();
        return Err(nao_confere());
    }
    sqlx::query!(
        "UPDATE portal.conta SET falhas = 0, bloqueada_ate = NULL WHERE id = $1",
        conta.id
    )
    .execute(&estado.banco)
    .await?;
    let segredo = portal::criar_sessao(&estado.banco, conta.id).await?;
    Ok((
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            cookie(&segredo, estado.config.cookie_seguro),
        )],
    )
        .into_response())
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SessaoDoPortal {
    /// Primeiro nome, para a saudação.
    pub primeiro_nome: String,
}

/// Sessão atual do paciente (401 sem sessão: a tela leva ao login).
#[utoipa::path(get, operation_id = "portal_sessao", path = "/v1/portal/sessao", tag = "portal", responses((status = 200, body = SessaoDoPortal), (status = 401, body = crate::openapi::ProblemaDoc)))]
pub async fn atual(
    State(estado): State<Estado>,
    paciente: Paciente,
) -> Result<Json<SessaoDoPortal>, ErroApi> {
    let mut tx = portal::transacao(&estado.banco, paciente.paciente_id).await?;
    let nome = sqlx::query_scalar!(
        "SELECT nome FROM identidade.paciente WHERE id = $1",
        paciente.paciente_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or_default();
    tx.commit().await?;
    Ok(Json(SessaoDoPortal {
        primeiro_nome: nome
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_owned(),
    }))
}

/// Sai do portal: a sessão é revogada no servidor e o cookie apagado.
#[utoipa::path(post, operation_id = "portal_sair", path = "/v1/portal/sessao/encerramento", tag = "portal", responses((status = 204)))]
pub async fn sair(State(estado): State<Estado>, paciente: Paciente) -> Result<Response, ErroApi> {
    sqlx::query!(
        "UPDATE portal.sessao SET revogada_em = now() WHERE conta_id = $1 AND revogada_em IS NULL",
        paciente.conta_id
    )
    .execute(&estado.banco)
    .await?;
    let seguro = if estado.config.cookie_seguro {
        "; Secure"
    } else {
        ""
    };
    Ok((
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            format!(
                "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{seguro}",
                portal::COOKIE
            ),
        )],
    )
        .into_response())
}
