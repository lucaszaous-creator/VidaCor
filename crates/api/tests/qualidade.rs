//! Qualidade (Qualidade.dc.html): não conformidade com andamento atômico e eventos de segurança com
//! prazo da Anvisa, anonimato e acesso só do Núcleo de Segurança do Paciente. Rota real e PostgreSQL
//! real (G4).

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
async fn evento_grave_tem_prazo_anvisa_anonimo_nao_mostra_quem_e_so_a_gerencia_le() {
    let amb = Ambiente::novo().await;
    let chave = Uuid::now_v7();
    let corpo = json!({ "tipo": "reacao_contraste", "quase_erro": false, "gravidade": "grave", "descricao": "Reação anafilática após contraste iodado na sala 6.", "paciente_id": amb.s.pacientes["Vera Lúcia Antunes"], "anonimo": true });
    let (st, p) = post(
        &amb,
        "bruno.sales",
        "/v1/qualidade/eventos",
        Some(chave),
        corpo.clone(),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    assert!(
        p["notificar_anvisa_ate"].is_string(),
        "grave tem prazo de 72 h"
    );
    let (_, p2) = post(
        &amb,
        "bruno.sales",
        "/v1/qualidade/eventos",
        Some(chave),
        corpo,
    )
    .await;
    assert_eq!(p2["id"], p["id"]);
    let (sq, vq) = post(&amb, "ana.souza", "/v1/qualidade/eventos", Some(Uuid::now_v7()), json!({ "tipo": "queda", "quase_erro": true, "gravidade": "leve", "descricao": "Quase caiu na rampa da recepção." })).await;
    assert_eq!(
        (sq, vq["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("evento-invalido")),
        "quase-erro é sem dano"
    );
    let g = amb.entrar("patricia.lemos").await;
    let (sg, _, painel) = amb.pedir(amb.get("/v1/qualidade", Some(&g))).await;
    assert_eq!(sg, StatusCode::OK, "{painel}");
    let ev = &painel["eventos"][0];
    assert!(
        ev["notificado_por"].is_null(),
        "anônimo não mostra quem notificou"
    );
    assert!(
        !painel.to_string().contains("Bruno"),
        "nome de quem notificou não vaza"
    );
    assert_eq!(painel["anvisa_pendentes"].as_i64(), Some(1));
    for login in [
        "ana.souza",
        "bruno.sales",
        "medico.a",
        "carla.caixa",
        "ti.a",
        "tv.a",
    ] {
        let s = amb.entrar(login).await;
        assert_eq!(
            amb.pedir(amb.get("/v1/qualidade", Some(&s))).await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    let (stv, _) = post(&amb, "tv.a", "/v1/qualidade/eventos", Some(Uuid::now_v7()), json!({ "tipo": "outro", "quase_erro": true, "gravidade": "sem_dano", "descricao": "Tentativa pela conta da TV." })).await;
    assert_eq!(stv, StatusCode::FORBIDDEN);
    // Relato do evento não muda; situação anda na ordem e de forma atômica.
    assert!(
        sqlx::query("UPDATE qualidade.evento SET descricao = 'reescrito pelo banco'")
            .execute(&amb.admin)
            .await
            .is_err()
    );
    let uri = format!(
        "/v1/qualidade/eventos/{}/situacao",
        p["id"].as_str().unwrap()
    );
    assert_eq!(
        post(
            &amb,
            "patricia.lemos",
            &uri,
            None,
            json!({ "situacao": "concluido" })
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (a, b) = tokio::join!(
        amb.pedir(amb.json(
            Method::POST,
            &uri,
            &g,
            None,
            json!({ "situacao": "em_analise" })
        )),
        amb.pedir(amb.json(
            Method::POST,
            &uri,
            &g,
            None,
            json!({ "situacao": "em_analise" })
        ))
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
}

#[tokio::test]
async fn nao_conformidade_andamento_pela_versao_e_encerramento_com_eficacia() {
    let amb = Ambiente::novo().await;
    let (st, nc) = post(&amb, "ti.a", "/v1/qualidade/nao-conformidades", Some(Uuid::now_v7()), json!({ "descricao": "Nobreak da sala 6 sem manutenção há 8 meses.", "classificacao": "equipamento", "gravidade": "moderada" })).await;
    assert_eq!(st, StatusCode::CREATED, "{nc}");
    let (si, _) = post(
        &amb,
        "ti.a",
        "/v1/qualidade/nao-conformidades",
        Some(Uuid::now_v7()),
        json!({ "descricao": "curto", "classificacao": "equipamento", "gravidade": "moderada" }),
    )
    .await;
    assert_eq!(si, StatusCode::UNPROCESSABLE_ENTITY);
    let uri = format!(
        "/v1/qualidade/nao-conformidades/{}/andamento",
        nc["id"].as_str().unwrap()
    );
    let g = amb.entrar("patricia.lemos").await;
    let corpo = json!({ "texto": "Chamado aberto com o fornecedor do nobreak.", "situacao": "em_acao", "prazo_em": "2026-12-01", "versao": 1 });
    let (a, b) = tokio::join!(
        amb.pedir(amb.json(Method::POST, &uri, &g, None, corpo.clone())),
        amb.pedir(amb.json(Method::POST, &uri, &g, None, corpo.clone()))
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(
        st,
        [StatusCode::OK, StatusCode::CONFLICT],
        "quem viu a versão antiga recebe 409"
    );
    // Encerrar exige eficácia.
    let (se, _) = post(
        &amb,
        "patricia.lemos",
        &uri,
        None,
        json!({ "texto": "Nobreak trocado e testado.", "situacao": "encerrada", "versao": 2 }),
    )
    .await;
    assert_eq!(se, StatusCode::UNPROCESSABLE_ENTITY);
    let (sok, _) = post(&amb, "patricia.lemos", &uri, None, json!({ "texto": "Nobreak trocado e testado.", "situacao": "encerrada", "eficaz": true, "versao": 2 })).await;
    assert_eq!(sok, StatusCode::OK);
    assert_eq!(
        post(
            &amb,
            "ana.souza",
            &uri,
            None,
            json!({ "texto": "tentativa", "situacao": "em_acao", "versao": 3 })
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert!(sqlx::query("DELETE FROM qualidade.nc_andamento")
        .execute(&amb.admin)
        .await
        .is_err());
    let (_, _, painel) = amb.pedir(amb.get("/v1/qualidade", Some(&g))).await;
    assert_eq!(
        painel["nao_conformidades"][0]["andamentos"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
}
