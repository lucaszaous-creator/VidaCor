//! Ouvidoria (Ouvidoria.dc.html): prazo pelo tipo, recepção registra e não lê, gerência responde de
//! forma atômica, resposta não muda, denúncia anônima só quando é denúncia, acesso indevido. Rota real e
//! PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

async fn registrar(amb: &Ambiente, login: &str, chave: Uuid, corpo: Value) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, "/v1/ouvidoria", &s, Some(chave), corpo))
        .await;
    (st, v)
}

#[tokio::test]
async fn registra_com_prazo_do_servidor_recepcao_nao_le_e_resposta_e_atomica() {
    let amb = Ambiente::novo().await;
    let chave = Uuid::now_v7();
    let corpo = json!({ "tipo": "reclamacao", "canal": "presencial", "paciente_id": amb.s.pacientes["Maria Helena Duarte"], "relato": "Esperou 50 minutos na recepção sem aviso." , "prazo_em": "2020-01-01" });
    let (st, p) = registrar(&amb, "ana.souza", chave, corpo.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    assert_ne!(p["prazo_em"], "2020-01-01", "prazo vem do servidor");
    let (_, p2) = registrar(&amb, "ana.souza", chave, corpo).await;
    assert_eq!(p2["id"], p["id"], "repetir não duplica");
    assert_eq!(
        amb.contar("SELECT count(*) FROM ouvidoria.manifestacao")
            .await,
        1
    );
    // A recepção registra, mas não lê a lista.
    let ana = amb.entrar("ana.souza").await;
    assert_eq!(
        amb.pedir(amb.get("/v1/ouvidoria", Some(&ana))).await.0,
        StatusCode::FORBIDDEN
    );
    let g = amb.entrar("patricia.lemos").await;
    let (sg, _, painel) = amb.pedir(amb.get("/v1/ouvidoria", Some(&g))).await;
    assert_eq!((sg, painel["abertas"].as_i64()), (StatusCode::OK, Some(1)));
    assert_eq!(painel["manifestacoes"][0]["quem"], "Maria Helena Duarte");
    // Resposta curta recusada; duas respostas ao mesmo tempo, só uma vale.
    let uri = format!("/v1/ouvidoria/{}/resposta", p["id"].as_str().unwrap());
    let (sc, _, _) = amb
        .pedir(amb.json(Method::POST, &uri, &g, None, json!({ "resposta": "ok" })))
        .await;
    assert_eq!(sc, StatusCode::UNPROCESSABLE_ENTITY);
    let r = json!({ "resposta": "Houve atraso no Raio X; reforçamos a equipe e avisaremos a espera prevista." });
    let (a, b) = tokio::join!(
        amb.pedir(amb.json(Method::POST, &uri, &g, None, r.clone())),
        amb.pedir(amb.json(Method::POST, &uri, &g, None, r.clone()))
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    // Resposta não muda nem se apaga.
    assert!(sqlx::query(
        "UPDATE ouvidoria.manifestacao SET resposta = 'outra resposta qualquer, longa o bastante'"
    )
    .execute(&amb.admin)
    .await
    .is_err());
    assert!(sqlx::query("DELETE FROM ouvidoria.manifestacao")
        .execute(&amb.admin)
        .await
        .is_err());
}

#[tokio::test]
async fn limites_anonimato_e_acesso_indevido() {
    let amb = Ambiente::novo().await;
    let casos = [
        (
            json!({ "tipo": "reclamacao", "canal": "presencial", "anonima": true, "relato": "relato com tamanho suficiente" }),
            "manifestacao-invalida",
        ),
        (
            json!({ "tipo": "reclamacao", "canal": "presencial", "relato": "sem ninguém identificado aqui" }),
            "manifestacao-invalida",
        ),
        (
            json!({ "tipo": "lgpd", "canal": "email", "nome_informado": "Fulano de Tal", "relato": "quero meus dados agora mesmo" }),
            "manifestacao-invalida",
        ),
        (
            json!({ "tipo": "fofoca", "canal": "presencial", "nome_informado": "Fulano", "relato": "relato com tamanho suficiente" }),
            "manifestacao-invalida",
        ),
        (
            json!({ "tipo": "elogio", "canal": "presencial", "nome_informado": "Fulano", "relato": "curto" }),
            "manifestacao-invalida",
        ),
    ];
    for (c, codigo) in casos {
        let (st, v) = registrar(&amb, "ana.souza", Uuid::now_v7(), c.clone()).await;
        assert_eq!(
            (st, v["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo)),
            "{c}"
        );
    }
    let (sd, _) = registrar(&amb, "ana.souza", Uuid::now_v7(), json!({ "tipo": "denuncia", "canal": "telefone", "anonima": true, "relato": "Relato de conduta inadequada na sala 2." })).await;
    assert_eq!(sd, StatusCode::CREATED, "denúncia anônima vale");
    for login in [
        "carla.caixa",
        "medico.a",
        "ti.a",
        "tv.a",
        "bruno.sales",
        "financeiro.a",
    ] {
        let (st, _) = registrar(&amb, login, Uuid::now_v7(), json!({ "tipo": "elogio", "canal": "presencial", "nome_informado": "Fulano", "relato": "Atendimento excelente hoje." })).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
        let s = amb.entrar(login).await;
        assert_eq!(
            amb.pedir(amb.get("/v1/ouvidoria", Some(&s))).await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    // Recepção de outra unidade não registra paciente desta unidade (RLS: paciente não encontrado).
    let (sb, vb) = registrar(&amb, "recepcao.b", Uuid::now_v7(), json!({ "tipo": "elogio", "canal": "presencial", "paciente_id": amb.s.pacientes["Maria Helena Duarte"], "relato": "Atendimento excelente hoje." })).await;
    assert_eq!(
        (sb, vb["codigo"].as_str()),
        (StatusCode::NOT_FOUND, Some("paciente-nao-encontrado"))
    );
}
