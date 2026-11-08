//! Avisos por WhatsApp (docs/25 MVP: confirmação, laudo pronto, opt-out; docs/21: mensagens
//! rastreadas). Rota real e PostgreSQL real; só o provedor (WhatsApp) é simulado (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use comum::{Ambiente, SEGREDO_WHATSAPP};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use uuid::Uuid;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const NASC: &str = "1954-06-15";

async fn processar_outbox(amb: &Ambiente) {
    let orthanc = vidacor_api::orthanc::Orthanc::novo("http://127.0.0.1:9").unwrap();
    while vidacor_api::outbox::processar_um(&amb.estado, &orthanc)
        .await
        .unwrap()
    {}
}

async fn com_celular(amb: &Ambiente, paciente: &str, celular: &str) {
    sqlx::query("UPDATE identidade.paciente SET celular = $2 WHERE id = $1")
        .bind(amb.s.pacientes[paciente])
        .bind(celular)
        .execute(&amb.admin)
        .await
        .unwrap();
}

fn webhook(corpo: &Value, assinar: bool) -> Request<Body> {
    let bruto = corpo.to_string();
    let mut mac = Hmac::<Sha256>::new_from_slice(SEGREDO_WHATSAPP.as_bytes()).unwrap();
    mac.update(bruto.as_bytes());
    let hex: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Request::builder()
        .method(Method::POST)
        .uri("/v1/webhooks/whatsapp")
        .header("content-type", "application/json")
        .header(
            "x-hub-signature-256",
            if assinar {
                format!("sha256={hex}")
            } else {
                "sha256=00".into()
            },
        )
        .body(Body::from(bruto))
        .unwrap()
}

fn resposta(id: &str, de: &str, texto: &str) -> Value {
    json!({ "entry": [{ "changes": [{ "value": { "messages": [{ "id": id, "from": de, "type": "text", "text": { "body": texto } }] } }] }] })
}

async fn assinar(amb: &Ambiente, laudo: Uuid) {
    let s = amb.entrar("medico.a").await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::PUT,
            &format!("/v1/laudos/{laudo}/rascunho"),
            &s,
            None,
            json!({ "texto": "Exame sem alterações.", "versao_numero": 1 }),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/laudos/{laudo}/assinatura"),
            &s,
            Some(Uuid::now_v7()),
            json!({ "pin": "4821" }),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{c}");
}

#[tokio::test]
async fn laudo_assinado_avisa_uma_vez_sem_dado_de_saude() {
    let amb = Ambiente::novo().await;
    com_celular(&amb, ANTONIO, "11987654321").await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    assinar(&amb, laudo).await;
    processar_outbox(&amb).await;
    processar_outbox(&amb).await;
    let enviadas = amb.mensageiro.enviadas.lock().unwrap().clone();
    let avisos: Vec<_> = enviadas
        .iter()
        .filter(|(_, m)| m.modelo == "laudo_pronto")
        .collect();
    assert_eq!(avisos.len(), 1, "FC-LAU-03: avisado uma vez");
    assert_eq!(avisos[0].1.para, "5511987654321");
    assert_eq!(
        avisos[0].1.parametros,
        ["Antônio"],
        "só o primeiro nome; nada do exame"
    );
    assert_eq!(amb.contar("SELECT count(*) FROM comunicacao.mensagem WHERE modelo = 'laudo_pronto' AND situacao = 'enviada'").await, 1);
}

#[tokio::test]
async fn confirmacao_pelo_whatsapp_e_sair_desliga_os_avisos() {
    let amb = Ambiente::novo().await;
    com_celular(&amb, ANTONIO, "11987654321").await;
    let ag = amb.s.agendamentos[ANTONIO];
    sqlx::query("UPDATE agenda.agendamento SET situacao = 'agendado', inicio = now() + interval '1 day', fim = now() + interval '1 day 30 minutes' WHERE id = $1")
        .bind(ag).execute(&amb.admin).await.unwrap();
    let dia = (chrono::Utc::now() + chrono::Duration::days(1))
        .with_timezone(&vidacor_api::tempo::FUSO)
        .date_naive();
    let ana = amb.entrar("ana.souza").await;
    let (st, _, l) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/comunicacao/lembretes",
            &ana,
            None,
            json!({ "data": dia }),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{l}");
    assert!(l["enfileirados"].as_i64().unwrap() >= 1);
    let (_, _, l2) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/comunicacao/lembretes",
            &ana,
            None,
            json!({ "data": dia }),
        ))
        .await;
    assert_eq!(l2["enfileirados"], 0, "repetir não manda de novo");
    processar_outbox(&amb).await;
    let conf = amb
        .mensageiro
        .enviadas
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, m)| m.modelo == "confirmacao")
        .count();
    assert!(conf >= 1);

    let r = resposta("wamid.1", "5511987654321", " sim ");
    let (s1, _, _) = amb.pedir(webhook(&r, true)).await;
    let (s2, _, d) = amb.pedir(webhook(&r, true)).await;
    assert_eq!(
        (s1, s2, d["situacao"].as_str()),
        (StatusCode::OK, StatusCode::OK, Some("ja_processado"))
    );
    processar_outbox(&amb).await;
    let situacao: String =
        sqlx::query_scalar("SELECT situacao FROM agenda.agendamento WHERE id = $1")
            .bind(ag)
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(situacao, "confirmado");

    amb.pedir(webhook(&resposta("wamid.2", "5511987654321", "SAIR"), true))
        .await;
    processar_outbox(&amb).await;
    let (_, _, c) = amb
        .pedir(amb.get(
            &format!("/v1/pacientes/{}/comunicacao", amb.s.pacientes[ANTONIO]),
            Some(&ana),
        ))
        .await;
    assert_eq!(c["whatsapp"], false);
    assert!(c["mensagens"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["resposta"] == "confirmou"));
}

#[tokio::test]
async fn opt_out_registra_sem_enviar_e_webhook_sem_assinatura_e_recusado() {
    let amb = Ambiente::novo().await;
    com_celular(&amb, ANTONIO, "11987654321").await;
    let ana = amb.entrar("ana.souza").await;
    let uri = format!("/v1/pacientes/{}/comunicacao", amb.s.pacientes[ANTONIO]);
    let (s0, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &ana,
            None,
            json!({ "whatsapp": false, "motivo": "" }),
        ))
        .await;
    assert_eq!(s0, StatusCode::UNPROCESSABLE_ENTITY);
    let (s1, _, p) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &ana,
            None,
            json!({ "whatsapp": false, "motivo": "pedido no balcão" }),
        ))
        .await;
    assert_eq!((s1, &p["whatsapp"]), (StatusCode::OK, &json!(false)));
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    assinar(&amb, laudo).await;
    processar_outbox(&amb).await;
    assert_eq!(
        amb.mensageiro.enviadas.lock().unwrap().len(),
        0,
        "opt-out: nada sai"
    );
    assert_eq!(
        amb.contar(
            "SELECT count(*) FROM comunicacao.mensagem WHERE situacao = 'bloqueada_opt_out'"
        )
        .await,
        1
    );
    let (s2, _, _) = amb
        .pedir(webhook(&resposta("wamid.9", "5511987654321", "SIM"), false))
        .await;
    assert_eq!(s2, StatusCode::UNAUTHORIZED);
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.evento_recebido WHERE provedor = 'whatsapp'")
            .await,
        0
    );
    for login in ["carla.caixa", "medico.a", "ti.a"] {
        let s = amb.entrar(login).await;
        let (st, _, _) = amb.pedir(amb.get(&uri, Some(&s))).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
}
