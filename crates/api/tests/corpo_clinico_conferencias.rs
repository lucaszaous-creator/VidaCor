//! Consulta do CRM no CFM e certificado ICP-Brasil do médico (DEC-2026-11-09-120 e 113). Rota HTTP
//! real e PostgreSQL real (G4); só o CFM é simulado (provedor externo). Certificados fictícios gerados
//! para teste, sem chave privada no repositório.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use base64::Engine;
use comum::Ambiente;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;
use vidacor_api::cfm::RespostaDoCfm;

const ICP: &[u8] = include_bytes!("dados/certificado-icp-ficticio.pem");
const COMUM: &[u8] = include_bytes!("dados/certificado-comum-ficticio.pem");

async fn post(amb: &Ambiente, login: &str, uri: &str, corpo: Value) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, uri, &s, None, corpo))
        .await;
    (st, v)
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[tokio::test]
async fn consulta_ao_cfm_pelo_webservice_ou_pelo_portal_fica_registrada_e_imutavel() {
    let amb = Ambiente::novo().await;
    let paulo = amb.s.usuarios["medico.a"];
    let uri = format!("/v1/corpo-clinico/{paulo}/consultas-crm");
    amb.cfm.registros.lock().unwrap().insert(
        ("SP".into(), "123456".into()),
        RespostaDoCfm {
            situacao: "regular".into(),
            nome: Some("Paulo Medeiros".into()),
            especialidades: vec!["Cardiologia (RQE 7788)".into()],
            resposta_sha256: "a".repeat(64),
        },
    );
    let (st, c) = post(
        &amb,
        "patricia.lemos",
        &uri,
        json!({ "uf": "sp", "numero": "123456", "fonte": "webservice_cfm" }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    assert_eq!(
        (c["situacao"].as_str(), c["fonte"].as_str()),
        (Some("regular"), Some("webservice_cfm"))
    );
    assert_eq!(c["resposta_sha256"], "a".repeat(64));
    assert!(c["codigo"].as_str().unwrap().starts_with("CCF-"));
    // Registro que o conselho não conhece volta "não encontrado".
    let (st, c) = post(
        &amb,
        "patricia.lemos",
        &uri,
        json!({ "uf": "SP", "numero": "999", "fonte": "webservice_cfm" }),
    )
    .await;
    assert_eq!(
        (st, c["situacao"].as_str()),
        (StatusCode::CREATED, Some("nao_encontrado"))
    );
    // Pelo portal: situação do enum fechado e observação obrigatória.
    for (corpo, codigo) in [
        (
            json!({ "uf": "SP", "numero": "123456", "fonte": "portal_cfm", "situacao": "inventada", "observacao": "portal do CFM" }),
            "corpo-clinico-invalido",
        ),
        (
            json!({ "uf": "SP", "numero": "123456", "fonte": "portal_cfm", "situacao": "regular" }),
            "corpo-clinico-invalido",
        ),
        (
            json!({ "uf": "XX", "numero": "123456", "fonte": "portal_cfm", "situacao": "regular", "observacao": "portal do CFM" }),
            "corpo-clinico-invalido",
        ),
        (
            json!({ "uf": "SP", "numero": "12a", "fonte": "portal_cfm", "situacao": "regular", "observacao": "portal do CFM" }),
            "corpo-clinico-invalido",
        ),
        (
            json!({ "uf": "SP", "numero": "123456", "fonte": "telefone" }),
            "corpo-clinico-invalido",
        ),
    ] {
        let (st, e) = post(&amb, "patricia.lemos", &uri, corpo).await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo))
        );
    }
    let (st, c) = post(&amb, "patricia.lemos", &uri, json!({ "uf": "SP", "numero": "123456", "fonte": "portal_cfm", "situacao": "suspenso", "observacao": "portal do CFM, busca por CRM e UF" })).await;
    assert_eq!(
        (st, c["situacao"].as_str()),
        (StatusCode::CREATED, Some("suspenso"))
    );
    // A lista do corpo clínico mostra a última; o histórico mostra todas.
    let g = amb.entrar("patricia.lemos").await;
    let (_, _, lista) = amb.pedir(amb.get("/v1/corpo-clinico", Some(&g))).await;
    let p = lista
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == paulo.to_string())
        .unwrap()
        .clone();
    assert_eq!(p["consulta_crm"]["situacao"], "suspenso");
    let (_, _, todas) = amb.pedir(amb.get(&uri, Some(&g))).await;
    assert_eq!(todas.as_array().unwrap().len(), 3);
    // Quem não é da gerência não consulta; a consulta nunca é apagada nem alterada.
    for login in ["medico.a", "ana.souza", "ti.a", "carla.caixa"] {
        let (st, _) = post(
            &amb,
            login,
            &uri,
            json!({ "uf": "SP", "numero": "123456", "fonte": "webservice_cfm" }),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    assert!(sqlx::query("DELETE FROM corpo.consulta_crm")
        .execute(&amb.admin)
        .await
        .is_err());
    assert!(
        sqlx::query("UPDATE corpo.consulta_crm SET situacao = 'regular'")
            .execute(&amb.admin)
            .await
            .is_err()
    );
    // Médico inexistente: 404 sem chamar o CFM.
    let (st, _) = post(
        &amb,
        "patricia.lemos",
        &format!("/v1/corpo-clinico/{}/consultas-crm", Uuid::now_v7()),
        json!({ "uf": "SP", "numero": "1", "fonte": "webservice_cfm" }),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sem_o_cfm_configurado_a_consulta_automatica_responde_503_e_o_portal_continua() {
    let amb = Ambiente::novo().await;
    let mut estado = amb.estado.clone();
    estado.cfm = None;
    let app = vidacor_api::aplicacao(estado);
    let paulo = amb.s.usuarios["medico.a"];
    let g = amb.entrar("patricia.lemos").await;
    let uri = format!("/v1/corpo-clinico/{paulo}/consultas-crm");
    let r = app
        .clone()
        .oneshot(amb.json(
            Method::POST,
            &uri,
            &g,
            None,
            json!({ "uf": "SP", "numero": "1", "fonte": "webservice_cfm" }),
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
    let r = app
        .oneshot(amb.json(Method::POST, &uri, &g, None, json!({ "uf": "SP", "numero": "1", "fonte": "portal_cfm", "situacao": "regular", "observacao": "portal do CFM, busca por CRM" })))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn certificado_icp_do_medico_so_publico_vigente_e_do_proprio_medico() {
    let amb = Ambiente::novo().await;
    let paulo = amb.s.usuarios["medico.a"];
    let uri = format!("/v1/corpo-clinico/{paulo}/certificados");
    // Recusas: não ICP, chave privada, lixo, certificado de outra pessoa.
    for (arquivo, codigo) in [
        (b64(COMUM), "certificado-nao-icp"),
        (
            b64(b"-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n"),
            "certificado-invalido",
        ),
        (b64(b"nada a ver"), "certificado-invalido"),
        ("%%%".into(), "certificado-invalido"),
    ] {
        let (st, e) = post(
            &amb,
            "patricia.lemos",
            &uri,
            json!({ "arquivo_base64": arquivo }),
        )
        .await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo)),
            "{e}"
        );
    }
    let cecilia = amb.s.usuarios["cecilia.souto"];
    let (st, e) = post(
        &amb,
        "patricia.lemos",
        &format!("/v1/corpo-clinico/{cecilia}/certificados"),
        json!({ "arquivo_base64": b64(ICP) }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("certificado-de-outra-pessoa")
        )
    );

    // Caminho feliz: titular, CPF mascarado, tipo e validade lidos pelo servidor.
    let (st, m) = post(
        &amb,
        "patricia.lemos",
        &uri,
        json!({ "arquivo_base64": b64(ICP) }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{m}");
    let c = &m["certificado"];
    assert_eq!(
        (c["titular"].as_str(), c["tipo"].as_str()),
        (Some("PAULO MEDEIROS"), Some("A3"))
    );
    assert_eq!(c["cpf_mascarado"], "***.***.***-35");
    assert!(c["vence_em_dias"].as_i64().unwrap() > 3000);
    assert!(
        !m.to_string().contains("11144477735"),
        "CPF completo nunca sai"
    );
    // Repetir o mesmo arquivo não cria outro.
    let (st, e) = post(
        &amb,
        "patricia.lemos",
        &uri,
        json!({ "arquivo_base64": b64(ICP) }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("certificado-ja-cadastrado"))
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM corpo.certificado").await,
        1
    );
    // Sem reautenticação recente: 401; outros perfis: 403.
    let g = amb.entrar("patricia.lemos").await;
    amb.expirar_reautenticacao().await;
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &g,
            None,
            json!({ "arquivo_base64": b64(ICP) }),
        ))
        .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    for login in ["medico.a", "ana.souza", "ti.a"] {
        let (st, _) = post(&amb, login, &uri, json!({ "arquivo_base64": b64(ICP) })).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    // O certificado guardado nunca é apagado nem reescrito.
    assert!(sqlx::query("DELETE FROM corpo.certificado")
        .execute(&amb.admin)
        .await
        .is_err());
    assert!(
        sqlx::query("UPDATE corpo.certificado SET titular = 'outro'")
            .execute(&amb.admin)
            .await
            .is_err()
    );
}
