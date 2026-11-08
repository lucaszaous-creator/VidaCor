//! Cadastro de paciente com busca de possível duplicado (Cadastro-Paciente.dc.html, docs/04).
//! Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

fn novo(nome: &str, nasc: &str, cpf: Option<&str>) -> Value {
    json!({ "nome": nome, "nascimento": nasc, "sexo": "F", "cpf": cpf, "celular": "(11) 98765-4321",
            "alergias": [], "nega_alergias": true })
}

async fn cadastrar(amb: &Ambiente, login: &str, chave: Uuid, corpo: Value) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, "/v1/pacientes", &s, Some(chave), corpo))
        .await;
    (st, v)
}

#[tokio::test]
async fn cadastra_com_versao_e_lastro_sem_dado_pessoal_e_repetir_nao_duplica() {
    let amb = Ambiente::novo().await;
    let chave = Uuid::now_v7();
    let corpo = novo(
        "Beatriz Nogueira Alves",
        "1979-08-21",
        Some("529.982.247-25"),
    );
    let (st, p) = cadastrar(&amb, "ana.souza", chave, corpo.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    assert!(p["codigo"].as_str().unwrap().starts_with("PAC-"));
    let (st2, p2) = cadastrar(&amb, "ana.souza", chave, corpo).await;
    assert_eq!((st2, &p2["id"]), (StatusCode::CREATED, &p["id"]));
    assert_eq!(
        amb.contar(
            "SELECT count(*) FROM identidade.paciente WHERE nome = 'Beatriz Nogueira Alves'"
        )
        .await,
        1
    );
    let (cpf, celular): (String, String) = sqlx::query_as(
        "SELECT cpf, celular FROM identidade.paciente WHERE nome = 'Beatriz Nogueira Alves'",
    )
    .fetch_one(&amb.admin)
    .await
    .unwrap();
    assert_eq!(
        (cpf.as_str(), celular.as_str()),
        ("52998224725", "11987654321"),
        "gravados só com dígitos"
    );
    assert_eq!(
        amb.contar(
            "SELECT count(*) FROM identidade.paciente_versao WHERE motivo = 'cadastro inicial'"
        )
        .await,
        1
    );
    let dados: String = sqlx::query_scalar(
        "SELECT dados::text FROM lastro.evento WHERE tipo = 'paciente_cadastrado'",
    )
    .fetch_one(&amb.admin)
    .await
    .unwrap();
    assert!(
        !dados.contains("Beatriz") && !dados.contains("52998224725"),
        "{dados}"
    );
}

#[tokio::test]
async fn possivel_duplicado_bloqueia_ate_a_recepcao_conferir() {
    let amb = Ambiente::novo().await;
    // Semente: Maria Helena Duarte, 12/03/1968. Digitada 21/03/1968 (dígitos invertidos).
    let ana = amb.entrar("ana.souza").await;
    let (sd, _, lista) = amb
        .pedir(amb.get(
            "/v1/pacientes/possiveis-duplicados?nome=Maria%20Helena%20Duarte&nascimento=1968-03-21",
            Some(&ana),
        ))
        .await;
    assert_eq!(sd, StatusCode::OK);
    assert_eq!(lista[0]["paciente"]["nascimento"], "1968-03-12");
    assert!(lista[0]["motivo"]
        .as_str()
        .unwrap()
        .contains("nascimento parecido"));
    let (st, e) = cadastrar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        novo("Maria Helena Duarte", "1968-03-21", None),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("possivel-duplicado"))
    );
    assert_eq!(
        e["candidatos"][0]["paciente"]["nome"],
        "Maria Helena Duarte"
    );
    let mut conferido = novo("Maria Helena Duarte", "1968-03-21", None);
    conferido["duplicado_conferido"] = json!(true);
    let (st2, p) = cadastrar(&amb, "ana.souza", Uuid::now_v7(), conferido).await;
    assert_eq!(st2, StatusCode::CREATED, "{p}");
    let dados: Value =
        sqlx::query_scalar("SELECT dados FROM lastro.evento WHERE tipo = 'paciente_cadastrado'")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(dados["duplicado_conferido"], true);
    assert_eq!(dados["parecidos"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn cpf_invalido_recusado_e_cpf_repetido_nao_passa_nem_conferido() {
    let amb = Ambiente::novo().await;
    let (s1, e1) = cadastrar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        novo("Carlos Eduardo Pinto", "1980-01-01", Some("529.982.247-26")),
    )
    .await;
    assert_eq!(
        (s1, e1["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("cpf-invalido"))
    );
    assert_eq!(
        cadastrar(
            &amb,
            "ana.souza",
            Uuid::now_v7(),
            novo("Carlos Eduardo Pinto", "1980-01-01", Some("52998224725"))
        )
        .await
        .0,
        StatusCode::CREATED
    );
    let mut outro = novo("Roberta Lins", "1991-05-05", Some("529.982.247-25"));
    outro["duplicado_conferido"] = json!(true);
    let (s2, e2) = cadastrar(&amb, "ana.souza", Uuid::now_v7(), outro).await;
    assert_eq!(
        (s2, e2["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("cpf-ja-cadastrado"))
    );
}

#[tokio::test]
async fn duas_recepcoes_com_o_mesmo_cpf_ao_mesmo_tempo_so_um_cadastro() {
    let amb = Ambiente::novo().await;
    let (a, b) = tokio::join!(
        cadastrar(
            &amb,
            "ana.souza",
            Uuid::now_v7(),
            novo("Fernanda Couto", "1985-02-02", Some("52998224725"))
        ),
        cadastrar(
            &amb,
            "ana.souza",
            Uuid::now_v7(),
            novo("Fernanda Couto Reis", "1985-02-02", Some("52998224725"))
        ),
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(
        st,
        [StatusCode::CREATED, StatusCode::CONFLICT],
        "{a:?} {b:?}"
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM identidade.paciente WHERE cpf = '52998224725'")
            .await,
        1
    );
}

#[tokio::test]
async fn limites_e_acesso_indevido() {
    let amb = Ambiente::novo().await;
    let mut sem_alergia = novo("Paula Reis", "1990-01-01", None);
    sem_alergia["nega_alergias"] = json!(false);
    assert_eq!(
        cadastrar(&amb, "ana.souza", Uuid::now_v7(), sem_alergia)
            .await
            .1["codigo"],
        "alergias-obrigatorias"
    );
    assert_eq!(
        cadastrar(
            &amb,
            "ana.souza",
            Uuid::now_v7(),
            novo("Paula Reis", "2999-01-01", None)
        )
        .await
        .1["codigo"],
        "nascimento-invalido"
    );
    assert_eq!(
        cadastrar(
            &amb,
            "ana.souza",
            Uuid::now_v7(),
            novo("Paula", "1990-01-01", None)
        )
        .await
        .1["codigo"],
        "nome-invalido"
    );
    let mut sem_celular = novo("Paula Reis", "1990-01-01", None);
    sem_celular["celular"] = json!("123");
    assert_eq!(
        cadastrar(&amb, "ana.souza", Uuid::now_v7(), sem_celular)
            .await
            .1["codigo"],
        "celular-invalido"
    );
    assert_eq!(
        cadastrar(
            &amb,
            "carla.caixa",
            Uuid::now_v7(),
            novo("Paula Reis", "1990-01-01", None)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let norte = amb.entrar("recepcao.b").await;
    let (_, _, lista) = amb
        .pedir(amb.get(
            "/v1/pacientes/possiveis-duplicados?nome=Maria%20Helena%20Duarte&nascimento=1968-03-12",
            Some(&norte),
        ))
        .await;
    assert_eq!(
        lista,
        json!([]),
        "outra unidade não descobre o cadastro (RLS)"
    );
}
