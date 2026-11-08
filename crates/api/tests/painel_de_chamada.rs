//! TV de chamada (Telao.dc.html; docs/18: o dispositivo TV vê só senha, nome abreviado e sala).
//! Rota real e PostgreSQL real (G4): caminho feliz, rechamada, repetição, limites e acesso indevido.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

const ANTONIO: &str = "Antônio Carlos Ribeiro";

async fn chamar(
    amb: &Ambiente,
    login: &str,
    atendimento: &str,
    chave: Uuid,
    corpo: Value,
) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let uri = format!("/v1/atendimentos/{atendimento}/chamadas");
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, &uri, &s, Some(chave), corpo))
        .await;
    (st, v)
}

async fn painel(amb: &Ambiente, login: &str) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb.pedir(amb.get("/v1/painel-de-chamada", Some(&s))).await;
    (st, v)
}

#[tokio::test]
async fn recepcao_chama_ao_guiche_e_a_tv_mostra_so_senha_nome_abreviado_e_destino() {
    let amb = Ambiente::novo().await;
    let at = amb.fazer_checkin(ANTONIO, "1954-06-15").await;
    let id = at["id"].as_str().unwrap();
    let senha = at["senha"].as_str().unwrap().to_owned();
    let chave = Uuid::now_v7();
    let (st, c) = chamar(
        &amb,
        "ana.souza",
        id,
        chave,
        json!({ "destino": "guiche", "guiche": 2 }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    assert_eq!(c["rechamada"], false);
    // Clique duplo: a mesma chave devolve a mesma chamada e não grava outra.
    let (st2, c2) = chamar(
        &amb,
        "ana.souza",
        id,
        chave,
        json!({ "destino": "guiche", "guiche": 2 }),
    )
    .await;
    assert_eq!((st2, &c2), (StatusCode::CREATED, &c));
    assert_eq!(amb.contar("SELECT count(*) FROM recepcao.chamada").await, 1);

    let (sp, p) = painel(&amb, "tv.a").await;
    assert_eq!(sp, StatusCode::OK, "{p}");
    assert_eq!(p["agora"]["senha"].as_str(), Some(senha.as_str()));
    assert_eq!(p["agora"]["nome_abreviado"], "Antônio C. R.");
    assert_eq!(p["agora"]["destino"], "Guichê 2");
    // Só o que a TV pode ver: nada de nascimento, exame, código ou identificador do paciente.
    let texto = p.to_string();
    for proibido in [
        "1954",
        "Ecocardiograma",
        "Ribeiro",
        "Carlos",
        "paciente",
        "PAC-",
        "atendimento",
    ] {
        assert!(
            !texto.contains(proibido),
            "painel vazou {proibido}: {texto}"
        );
    }

    // O técnico lotado na sala rechama para a sala; o painel mostra a nova e guarda a anterior.
    let (st3, c3) = chamar(
        &amb,
        "bruno.sales",
        id,
        Uuid::now_v7(),
        json!({ "destino": "sala" }),
    )
    .await;
    assert_eq!(st3, StatusCode::CREATED, "{c3}");
    assert_eq!(
        (c3["destino"].as_str(), c3["rechamada"].as_bool()),
        (Some("Sala 1"), Some(true))
    );
    let (_, p2) = painel(&amb, "tv.a").await;
    assert_eq!(p2["agora"]["destino"], "Sala 1");
    assert_eq!(p2["ultimas"][0]["destino"], "Guichê 2");

    // A chamada é registro imutável.
    assert!(sqlx::query("UPDATE recepcao.chamada SET destino = 'X'")
        .execute(&amb.admin)
        .await
        .is_err());
    assert!(sqlx::query("DELETE FROM recepcao.chamada")
        .execute(&amb.admin)
        .await
        .is_err());
}

#[tokio::test]
async fn destino_errado_guiche_fora_do_limite_e_tecnico_de_outra_sala_sao_recusados() {
    let amb = Ambiente::novo().await;
    let at = amb.fazer_checkin(ANTONIO, "1954-06-15").await;
    let id = at["id"].as_str().unwrap();
    for (login, corpo) in [
        ("ana.souza", json!({ "destino": "guiche", "guiche": 0 })),
        ("ana.souza", json!({ "destino": "guiche", "guiche": 21 })),
        ("ana.souza", json!({ "destino": "sala" })),
        ("bruno.sales", json!({ "destino": "guiche", "guiche": 1 })),
        ("ana.souza", json!({ "destino": "telao", "guiche": 1 })),
    ] {
        let (st, v) = chamar(&amb, login, id, Uuid::now_v7(), corpo).await;
        assert_eq!(
            (st, v["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some("destino-invalido")),
            "{login} {v}"
        );
    }
    // Técnico lotado em outra sala não chama este paciente.
    let (st, _) = chamar(
        &amb,
        "livia.costa",
        id,
        Uuid::now_v7(),
        json!({ "destino": "sala" }),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Atendimento inexistente: nada é chamado.
    let (st, v) = chamar(
        &amb,
        "ana.souza",
        &Uuid::now_v7().to_string(),
        Uuid::now_v7(),
        json!({ "destino": "guiche", "guiche": 1 }),
    )
    .await;
    assert_eq!(
        (st, v["codigo"].as_str()),
        (StatusCode::NOT_FOUND, Some("atendimento-nao-encontrado"))
    );
    assert_eq!(amb.contar("SELECT count(*) FROM recepcao.chamada").await, 0);
}

#[tokio::test]
async fn tv_so_ve_o_painel_da_propria_unidade_e_nao_chama_ninguem() {
    let amb = Ambiente::novo().await;
    let at = amb.fazer_checkin(ANTONIO, "1954-06-15").await;
    let id = at["id"].as_str().unwrap();
    let (st, _) = chamar(
        &amb,
        "ana.souza",
        id,
        Uuid::now_v7(),
        json!({ "destino": "guiche", "guiche": 1 }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    // A TV não chama, não vê agenda, recepção, paciente nem laudo.
    let (st, _) = chamar(
        &amb,
        "tv.a",
        id,
        Uuid::now_v7(),
        json!({ "destino": "guiche", "guiche": 1 }),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let tv = amb.entrar("tv.a").await;
    for uri in [
        format!("/v1/agenda?data={}", amb.dia),
        format!("/v1/atendimentos/{id}"),
        "/v1/laudos/fila".to_owned(),
        "/v1/caixa/a-receber".to_owned(),
        "/v1/estudos/pendentes".to_owned(),
        "/v1/equipe".to_owned(),
    ] {
        let (s, _, _) = amb.pedir(amb.get(&uri, Some(&tv))).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "{uri}");
    }
    // A TV de outra unidade não vê a chamada (RLS); caixa e médico não abrem o painel.
    let (sb, pb) = painel(&amb, "tv.b").await;
    assert_eq!((sb, &pb["agora"]), (StatusCode::OK, &Value::Null));
    for login in ["carla.caixa", "medico.a", "ti.a", "financeiro.a"] {
        let (s, _) = painel(&amb, login).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "{login}");
    }
    // A recepção e a gerência podem conferir o painel.
    assert_eq!(painel(&amb, "ana.souza").await.0, StatusCode::OK);
    assert_eq!(painel(&amb, "patricia.lemos").await.0, StatusCode::OK);
}

#[tokio::test]
async fn conta_da_tv_nao_acumula_outro_perfil() {
    let amb = Ambiente::novo().await;
    let r = sqlx::query(
        "UPDATE identidade.usuario SET perfis = ARRAY['tv','recepcao'] WHERE login = 'tv.a'",
    )
    .execute(&amb.admin)
    .await;
    assert!(r.is_err(), "a conta da TV tem só o perfil da TV");
}
