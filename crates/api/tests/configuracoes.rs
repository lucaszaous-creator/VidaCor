//! Configurações e integrações (Configuracoes.dc.html; DEC-2026-11-08-78): a tela vê só o nome das
//! variáveis e se estão preenchidas, nunca o valor; só TI e gerência. O adaptador da Cloud API do
//! WhatsApp é testado contra um servidor falso local (mock só do provedor externo, G4).
//! Um único teste por binário: as variáveis de ambiente valem para o processo inteiro.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use comum::Ambiente;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use vidacor_api::mensageiro::{Mensageiro, MensageiroCloudApi, MensagemWhatsapp};

#[tokio::test]
async fn integracoes_sem_valor_na_tela_e_adaptador_do_whatsapp_monta_o_modelo() {
    let amb = Ambiente::novo().await;
    // Ambiente: WhatsApp só com o token (incompleto) e NFS-e no simulado de desenvolvimento.
    std::env::set_var("VIDACOR_WHATSAPP_TOKEN", "token-secreto-de-teste-XYZ");
    std::env::remove_var("VIDACOR_WHATSAPP_NUMERO_ID");
    std::env::set_var("VIDACOR_NFSE_PROVEDOR", "desenvolvimento");
    let ti = amb.entrar("ti.a").await;
    let (st, resp, v) = amb
        .pedir(amb.get("/v1/configuracoes/integracoes", Some(&ti)))
        .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(resp.headers()["cache-control"], "no-store");
    let texto = v.to_string();
    assert!(
        !texto.contains("token-secreto-de-teste-XYZ"),
        "valor de segredo vazou"
    );
    assert!(
        !texto.contains("segredo-de-teste"),
        "nenhum valor de variável na resposta"
    );
    let achar = |id: &str| {
        v.as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == id)
            .unwrap()
            .clone()
    };
    let w = achar("whatsapp");
    assert_eq!(
        w["situacao"], "em_uso",
        "nos testes o mensageiro em memória está ligado"
    );
    let token = w["variaveis"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["nome"] == "VIDACOR_WHATSAPP_TOKEN")
        .unwrap();
    assert_eq!(
        (token["preenchida"].as_bool(), token["segredo"].as_bool()),
        (Some(true), Some(true))
    );
    assert_eq!(achar("nfse")["situacao"], "desenvolvimento");
    assert_eq!(
        achar("email")["situacao"],
        "em_uso",
        "correio em memória nos testes"
    );
    for login in [
        "ana.souza",
        "medico.a",
        "carla.caixa",
        "financeiro.a",
        "tv.a",
        "bruno.sales",
    ] {
        let s = amb.entrar(login).await;
        let (sl, _, _) = amb
            .pedir(amb.get("/v1/configuracoes/integracoes", Some(&s)))
            .await;
        assert_eq!(sl, StatusCode::FORBIDDEN, "{login}");
    }
    std::env::remove_var("VIDACOR_WHATSAPP_TOKEN");
    std::env::remove_var("VIDACOR_NFSE_PROVEDOR");

    // Adaptador real contra um servidor falso da Meta.
    let recebidos: Arc<Mutex<Vec<(String, Value)>>> = Arc::default();
    let r = recebidos.clone();
    let app = Router::new().route(
        "/v21.0/123456/messages",
        post(move |h: axum::http::HeaderMap, Json(c): Json<Value>| {
            let r = r.clone();
            async move {
                let auth = h
                    .get("authorization")
                    .map(|x| x.to_str().unwrap().to_owned())
                    .unwrap_or_default();
                r.lock().unwrap().push((auth, c));
                Json(json!({ "messages": [{ "id": "wamid.TESTE" }] }))
            }
        }),
    );
    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let porta = ouvinte.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(ouvinte, app).await.unwrap() });
    assert!(
        MensageiroCloudApi::novo("http://exemplo.com/v21.0", "123456", "t").is_err(),
        "sem https recusa"
    );
    assert!(
        MensageiroCloudApi::novo("https://graph.facebook.com/v21.0", "12a", "t").is_err(),
        "número só dígitos"
    );
    let m = MensageiroCloudApi::novo(&format!("http://127.0.0.1:{porta}/v21.0"), "123456", "tok")
        .unwrap();
    let id = m
        .enviar(
            uuid::Uuid::now_v7(),
            &MensagemWhatsapp {
                para: "5511987654321".into(),
                modelo: "oferta_vaga".into(),
                parametros: vec!["Maria".into(), "09/11".into(), "10:00".into()],
            },
        )
        .await
        .unwrap();
    assert_eq!(id, "wamid.TESTE");
    let (auth, corpo) = recebidos.lock().unwrap()[0].clone();
    assert_eq!(auth, "Bearer tok");
    assert_eq!(corpo["template"]["name"], "oferta_vaga");
    assert_eq!(corpo["template"]["language"]["code"], "pt_BR");
    assert_eq!(
        corpo["template"]["components"][0]["parameters"][0]["text"],
        "Maria"
    );
    assert_eq!(corpo["to"], "5511987654321");
}
