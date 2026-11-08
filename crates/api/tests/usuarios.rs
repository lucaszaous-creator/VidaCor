//! Usuários e perfis (docs/18 § 6: segunda pessoa para perfil clínico; TI sem dado clínico;
//! separação de funções; mudar permissão com reautenticação). Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

async fn post(
    amb: &Ambiente,
    login: &str,
    uri: &str,
    chave: Option<Uuid>,
    corpo: Value,
) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, uri, &s, chave, corpo))
        .await;
    (st, v)
}

#[tokio::test]
async fn medico_novo_nasce_inativo_e_so_entra_com_a_segunda_pessoa() {
    let amb = Ambiente::novo().await;
    let chave = Uuid::now_v7();
    let corpo = json!({ "login": "renata.alves", "nome": "Dra. Renata Alves", "perfis": ["medico"], "motivo": "contratação, ultrassom" });
    let (st, u) = post(&amb, "ti.a", "/v1/usuarios", Some(chave), corpo.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{u}");
    assert_eq!(
        (u["ativo"].as_bool(), u["pendente"]["tipo"].as_str()),
        (Some(false), Some("criar"))
    );
    let (_, u2) = post(&amb, "ti.a", "/v1/usuarios", Some(chave), corpo).await;
    assert_eq!(u2["id"], u["id"], "repetir não duplica");
    let pedido = u["pendente"]["id"].as_str().unwrap();
    let decisao = json!({ "aprovar": true, "motivo": "conferido com o contrato" });
    // A TI que pediu não confere; a gerência confere.
    assert_eq!(
        post(
            &amb,
            "ti.a",
            &format!("/v1/usuarios/pedidos/{pedido}/decisao"),
            None,
            decisao.clone()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (sd, d) = post(
        &amb,
        "patricia.lemos",
        &format!("/v1/usuarios/pedidos/{pedido}/decisao"),
        None,
        decisao.clone(),
    )
    .await;
    assert_eq!(
        (sd, d["ativo"].as_bool()),
        (StatusCode::OK, Some(true)),
        "{d}"
    );
    assert_eq!(
        post(
            &amb,
            "patricia.lemos",
            &format!("/v1/usuarios/pedidos/{pedido}/decisao"),
            None,
            decisao
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'perfil_conferido'")
            .await,
        1
    );
}

#[tokio::test]
async fn separacao_de_funcoes_e_propria_conta() {
    let amb = Ambiente::novo().await;
    let ti_med = json!({ "login": "x.y", "nome": "Fulano Teste", "perfis": ["ti", "medico"], "motivo": "teste de conflito" });
    let (s1, e1) = post(
        &amb,
        "patricia.lemos",
        "/v1/usuarios",
        Some(Uuid::now_v7()),
        ti_med,
    )
    .await;
    assert_eq!(
        (s1, e1["codigo"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("separacao-de-funcoes")
        )
    );
    let carla = amb.s.usuarios["carla.caixa"];
    let (s2, e2) = post(
        &amb,
        "patricia.lemos",
        &format!("/v1/usuarios/{carla}/mudancas"),
        None,
        json!({ "tipo": "conceder", "perfil": "financeiro", "motivo": "cobrir férias" }),
    )
    .await;
    assert_eq!(
        (s2, e2["codigo"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("separacao-de-funcoes")
        ),
        "caixa não acumula financeiro"
    );
    let patricia = amb.s.usuarios["patricia.lemos"];
    let (s3, _) = post(
        &amb,
        "patricia.lemos",
        &format!("/v1/usuarios/{patricia}/mudancas"),
        None,
        json!({ "tipo": "conceder", "perfil": "medico", "motivo": "ninguém se concede" }),
    )
    .await;
    assert_eq!(s3, StatusCode::FORBIDDEN);
    // Mesmo pela porta do banco, o papel da aplicação não põe perfil clínico na TI.
    let r = sqlx::query(
        "UPDATE identidade.usuario SET perfis = ARRAY['ti','medico'] WHERE login = 'ti.a'",
    )
    .execute(&amb.admin)
    .await;
    assert!(r.is_err(), "o gatilho barra TI com perfil clínico");
}

#[tokio::test]
async fn desativar_derruba_a_sessao_na_hora_e_reativar_pede_segunda_pessoa() {
    let amb = Ambiente::novo().await;
    let ana_sessao = amb.entrar("ana.souza").await;
    let ana = amb.s.usuarios["ana.souza"];
    let (sd, d) = post(
        &amb,
        "ti.a",
        &format!("/v1/usuarios/{ana}/mudancas"),
        None,
        json!({ "tipo": "desativar", "motivo": "saiu da clínica" }),
    )
    .await;
    assert_eq!(
        (sd, d["ativo"].as_bool()),
        (StatusCode::OK, Some(false)),
        "{d}"
    );
    let (s_agenda, _, _) = amb
        .pedir(amb.get(&format!("/v1/agenda?data={}", amb.s.dia), Some(&ana_sessao)))
        .await;
    assert_eq!(s_agenda, StatusCode::UNAUTHORIZED, "sessão aberta caiu");
    let (sr, r) = post(
        &amb,
        "ti.a",
        &format!("/v1/usuarios/{ana}/mudancas"),
        None,
        json!({ "tipo": "reativar", "motivo": "voltou do afastamento" }),
    )
    .await;
    assert_eq!(
        (sr, r["ativo"].as_bool(), r["pendente"]["tipo"].as_str()),
        (StatusCode::OK, Some(false), Some("reativar"))
    );
    let (s2, _) = post(
        &amb,
        "ti.a",
        &format!("/v1/usuarios/{ana}/mudancas"),
        None,
        json!({ "tipo": "retirar", "perfil": "recepcao", "motivo": "outro pedido" }),
    )
    .await;
    assert_eq!(
        s2,
        StatusCode::UNPROCESSABLE_ENTITY,
        "um pedido aguardando por vez, e ela está inativa"
    );
}

#[tokio::test]
async fn so_ti_e_gerencia_e_reautenticacao() {
    let amb = Ambiente::novo().await;
    for login in ["ana.souza", "medico.a", "financeiro.a", "carla.caixa"] {
        let s = amb.entrar(login).await;
        let (st, _, _) = amb.pedir(amb.get("/v1/usuarios", Some(&s))).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    let ti = amb.entrar("ti.a").await;
    let (st, _, lista) = amb.pedir(amb.get("/v1/usuarios", Some(&ti))).await;
    assert_eq!(st, StatusCode::OK);
    assert!(
        lista
            .as_array()
            .unwrap()
            .iter()
            .all(|u| u["nome"] != "Recepção Norte"),
        "outra unidade não aparece"
    );
    let ana = amb.s.usuarios["ana.souza"];
    let s = amb.entrar("ti.a").await;
    amb.expirar_reautenticacao().await;
    let (sr, _, e) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/usuarios/{ana}/mudancas"),
            &s,
            None,
            json!({ "tipo": "desativar", "motivo": "teste de reautenticação" }),
        ))
        .await;
    assert_eq!(
        (sr, e["codigo"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("reautenticacao-necessaria"))
    );
}
