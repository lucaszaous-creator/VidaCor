//! Convite e login do portal do paciente (docs/25 Fase 1: "link do convite de uso único"; SEC-AUT-6,
//! SEC-DIS-5). Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const NASC: &str = "1954-06-15";

fn sem_sessao(
    metodo: Method,
    uri: &str,
    corpo: Option<Value>,
    cookie: Option<&str>,
) -> Request<Body> {
    let mut b = Request::builder()
        .method(metodo)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    b.body(corpo.map_or_else(Body::empty, |c| Body::from(c.to_string())))
        .unwrap()
}

async fn com_email(amb: &Ambiente) {
    sqlx::query(
        "UPDATE identidade.paciente SET email = 'antonio.ribeiro@exemplo.invalid' WHERE id = $1",
    )
    .bind(amb.s.pacientes[ANTONIO])
    .execute(&amb.admin)
    .await
    .unwrap();
}

/// Processa o outbox e devolve o token do último e-mail enviado ao paciente.
async fn token_do_email(amb: &Ambiente) -> String {
    let orthanc = vidacor_api::orthanc::Orthanc::novo("http://127.0.0.1:9").unwrap();
    while vidacor_api::outbox::processar_um(&amb.estado, &orthanc)
        .await
        .unwrap()
    {}
    let enviadas = amb.correio.enviadas.lock().unwrap();
    let (_, m) = enviadas.last().expect("e-mail do convite enviado");
    assert_eq!(m.para, "antonio.ribeiro@exemplo.invalid");
    m.texto
        .split("/portal/convite/")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned()
}

async fn convidar(amb: &Ambiente, chave: Uuid) -> (StatusCode, Value) {
    com_email(amb).await;
    let ana = amb.entrar("ana.souza").await;
    let uri = format!("/v1/pacientes/{}/convites-portal", amb.s.pacientes[ANTONIO]);
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, &uri, &ana, Some(chave), json!({})))
        .await;
    (st, v)
}

async fn aceitar(
    amb: &Ambiente,
    token: &str,
    nasc: &str,
    login: &str,
) -> (StatusCode, Option<String>, Value) {
    let (st, r, v) = amb
        .pedir(sem_sessao(
            Method::POST,
            &format!("/v1/portal/convites/{token}/aceite"),
            Some(json!({ "nascimento": nasc, "login": login, "senha": "uma senha longa 2026" })),
            None,
        ))
        .await;
    let cookie = r
        .headers()
        .get(header::SET_COOKIE)
        .map(|c| c.to_str().unwrap().split(';').next().unwrap().to_owned());
    (st, cookie, v)
}

async fn situacao(amb: &Ambiente, token: &str) -> String {
    let (_, _, v) = amb
        .pedir(sem_sessao(
            Method::GET,
            &format!("/v1/portal/convites/{token}"),
            None,
            None,
        ))
        .await;
    v["situacao"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn recepcao_convida_o_link_vai_por_email_e_nao_fica_guardado() {
    let amb = Ambiente::novo().await;
    let chave = Uuid::now_v7();
    let (st, c) = convidar(&amb, chave).await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    assert_eq!(
        c["enviado_para"], "an***@exemplo.invalid",
        "a recepção vê o e-mail mascarado, não o link"
    );
    assert!(c.get("caminho").is_none());
    let (st2, c2) = convidar(&amb, chave).await;
    assert_eq!((st2, &c2["id"]), (StatusCode::CREATED, &c["id"]));
    assert_eq!(amb.contar("SELECT count(*) FROM portal.convite").await, 1);
    let t = token_do_email(&amb).await;
    assert_eq!(situacao(&amb, &t).await, "valido");
    let mensagem = amb
        .correio
        .enviadas
        .lock()
        .unwrap()
        .last()
        .unwrap()
        .1
        .clone();
    assert!(mensagem.texto.contains("Olá, Antônio.") && mensagem.texto.contains("uma vez só"));
    assert!(
        !mensagem.texto.contains("1954") && !mensagem.texto.contains("Eco"),
        "sem nascimento nem exame no e-mail"
    );
    // Nada guarda o token: nem o convite, nem o outbox, nem a idempotência.
    for sql in [
        "SELECT carga::text FROM integracao.outbox",
        "SELECT resposta::text FROM integracao.idempotencia",
    ] {
        let textos: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .fetch_all(&amb.admin)
            .await
            .unwrap();
        assert!(textos.iter().all(|x| !x.contains(&t)), "{sql}");
    }
    // Reprocessar o outbox não manda outro e-mail (mesma chave).
    token_do_email(&amb).await;
    assert_eq!(amb.correio.enviadas.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn sem_email_no_cadastro_nao_convida() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let uri = format!("/v1/pacientes/{}/convites-portal", amb.s.pacientes[ANTONIO]);
    let (st, _, e) = amb
        .pedir(amb.json(Method::POST, &uri, &ana, Some(Uuid::now_v7()), json!({})))
        .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("paciente-sem-email"))
    );
    assert_eq!(amb.contar("SELECT count(*) FROM portal.convite").await, 0);
}

#[tokio::test]
async fn aceite_confere_o_nascimento_entra_e_o_convite_nao_serve_de_novo() {
    let amb = Ambiente::novo().await;
    convidar(&amb, Uuid::now_v7()).await;
    let t = token_do_email(&amb).await;
    let (s1, _, e1) = aceitar(&amb, &t, "1954-06-16", "antonio.ribeiro").await;
    assert_eq!(s1, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        e1["detail"]
            .as_str()
            .unwrap()
            .contains("Restam 4 tentativas"),
        "{e1}"
    );
    let (s2, cookie, _) = aceitar(&amb, &t, NASC, "antonio.ribeiro").await;
    assert_eq!(s2, StatusCode::NO_CONTENT);
    let cookie = cookie.unwrap();
    assert!(cookie.starts_with("vc_portal="));
    let (s3, _, sessao) = amb
        .pedir(sem_sessao(
            Method::GET,
            "/v1/portal/sessao",
            None,
            Some(&cookie),
        ))
        .await;
    assert_eq!(
        (s3, sessao["primeiro_nome"].as_str()),
        (StatusCode::OK, Some("Antônio"))
    );
    let (s4, _, e4) = aceitar(&amb, &t, NASC, "outro.login").await;
    assert_eq!(
        (s4, e4["codigo"].as_str()),
        (StatusCode::GONE, Some("convite-indisponivel")),
        "uso único"
    );
    assert_eq!(situacao(&amb, &t).await, "indisponivel");
    let hash: String = sqlx::query_scalar("SELECT senha_hash FROM portal.conta")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    assert!(hash.starts_with("$argon2id$") && !hash.contains("uma senha longa"));
}

#[tokio::test]
async fn dois_aceites_ao_mesmo_tempo_so_um_entra() {
    let amb = Ambiente::novo().await;
    convidar(&amb, Uuid::now_v7()).await;
    let t = token_do_email(&amb).await;
    let (a, b) = tokio::join!(
        aceitar(&amb, &t, NASC, "antonio.um"),
        aceitar(&amb, &t, NASC, "antonio.dois")
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(st, [StatusCode::NO_CONTENT, StatusCode::GONE]);
    assert_eq!(amb.contar("SELECT count(*) FROM portal.conta").await, 1);
}

#[tokio::test]
async fn cinco_nascimentos_errados_revogam_e_convite_novo_troca_o_anterior() {
    let amb = Ambiente::novo().await;
    convidar(&amb, Uuid::now_v7()).await;
    let t = token_do_email(&amb).await;
    for _ in 0..4 {
        assert_eq!(
            aceitar(&amb, &t, "2000-01-01", "antonio.r").await.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(
        aceitar(&amb, &t, "2000-01-01", "antonio.r").await.0,
        StatusCode::GONE
    );
    assert_eq!(
        aceitar(&amb, &t, NASC, "antonio.r").await.0,
        StatusCode::GONE,
        "nem com o nascimento certo"
    );
    convidar(&amb, Uuid::now_v7()).await;
    let t2 = token_do_email(&amb).await;
    convidar(&amb, Uuid::now_v7()).await;
    let t3 = token_do_email(&amb).await;
    assert_eq!(
        situacao(&amb, &t2).await,
        "indisponivel",
        "o convite novo revoga o anterior"
    );
    assert_eq!(
        aceitar(&amb, &t3, NASC, "antonio.r").await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        situacao(&amb, "token-que-nao-existe-0000000000").await,
        "indisponivel"
    );
}

async fn entrar(amb: &Ambiente, login: &str, senha: &str) -> (StatusCode, Value) {
    let (st, _, v) = amb
        .pedir(sem_sessao(
            Method::POST,
            "/v1/portal/sessao",
            Some(json!({ "login": login, "senha": senha })),
            None,
        ))
        .await;
    (st, v)
}

#[tokio::test]
async fn login_tem_mensagem_unica_e_espera_depois_de_cinco_erros() {
    let amb = Ambiente::novo().await;
    convidar(&amb, Uuid::now_v7()).await;
    let t = token_do_email(&amb).await;
    let (_, cookie, _) = aceitar(&amb, &t, NASC, "Antonio.Ribeiro").await;
    assert_eq!(
        entrar(&amb, "antonio.ribeiro", "uma senha longa 2026")
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    let (s1, e1) = entrar(&amb, "antonio.ribeiro", "errada errada").await;
    let (s2, e2) = entrar(&amb, "ninguem.aqui", "errada errada").await;
    assert_eq!(
        (s1, s2),
        (StatusCode::UNAUTHORIZED, StatusCode::UNAUTHORIZED)
    );
    assert_eq!(e1["detail"], e2["detail"], "não revela se o login existe");
    for _ in 0..4 {
        entrar(&amb, "antonio.ribeiro", "errada errada").await;
    }
    let (s3, e3) = entrar(&amb, "antonio.ribeiro", "uma senha longa 2026").await;
    assert_eq!(
        (s3, e3["codigo"].as_str()),
        (StatusCode::TOO_MANY_REQUESTS, Some("acesso-em-espera"))
    );
    // Sair revoga a sessão no servidor.
    let cookie = cookie.unwrap();
    let (ss, _, _) = amb
        .pedir(sem_sessao(
            Method::POST,
            "/v1/portal/sessao/encerramento",
            None,
            Some(&cookie),
        ))
        .await;
    assert_eq!(ss, StatusCode::NO_CONTENT);
    let (s4, _, _) = amb
        .pedir(sem_sessao(
            Method::GET,
            "/v1/portal/sessao",
            None,
            Some(&cookie),
        ))
        .await;
    assert_eq!(s4, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn caixa_nao_convida_e_outra_unidade_nao_acha_o_paciente() {
    let amb = Ambiente::novo().await;
    let uri = format!("/v1/pacientes/{}/convites-portal", amb.s.pacientes[ANTONIO]);
    let carla = amb.entrar("carla.caixa").await;
    let (s1, _, _) = amb
        .pedir(amb.json(Method::POST, &uri, &carla, Some(Uuid::now_v7()), json!({})))
        .await;
    assert_eq!(s1, StatusCode::FORBIDDEN);
    let norte = amb.entrar("recepcao.b").await;
    let (s2, _, _) = amb
        .pedir(amb.json(Method::POST, &uri, &norte, Some(Uuid::now_v7()), json!({})))
        .await;
    assert_eq!(s2, StatusCode::NOT_FOUND);
    assert_eq!(amb.contar("SELECT count(*) FROM portal.convite").await, 0);
}

#[tokio::test]
async fn com_a_sessao_do_portal_o_paciente_so_le_o_proprio_cadastro() {
    let amb = Ambiente::novo().await;
    convidar(&amb, Uuid::now_v7()).await;
    let t = token_do_email(&amb).await;
    aceitar(&amb, &t, NASC, "antonio.ribeiro").await;
    let antonio = amb.s.pacientes[ANTONIO];
    let mut tx = vidacor_api::portal::transacao(&amb.app_pool, antonio)
        .await
        .unwrap();
    let visiveis: i64 = sqlx::query_scalar("SELECT count(*) FROM identidade.paciente")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(visiveis, 1, "RLS por paciente no portal");
}
