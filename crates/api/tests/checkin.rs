//! Check-in com dois identificadores: testes de integração na rota HTTP real com PostgreSQL real
//! (G4). Cobre caminho feliz, erros, repetição, simultaneidade, limites e acesso indevido.
//! Requisitos: REQ-IDT-001 a 003, REQ-REC-001 a 005, REQ-LAS-001 e 002 (docs/29).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use comum::Ambiente;
use uuid::Uuid;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const NASC_ANTONIO: &str = "1954-06-15";

#[tokio::test]
async fn checkin_com_dois_identificadores_cria_atendimento_lastro_e_worklist() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (status, resp, corpo) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            "antonio carlos ribeiro",
            NASC_ANTONIO,
        ))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{corpo}");
    assert!(corpo["codigo"].as_str().unwrap().starts_with("ATD-"));
    assert!(corpo["accession"].as_str().unwrap().starts_with("ACC-"));
    assert!(
        corpo["accession"].as_str().unwrap().len() <= 16,
        "accession cabe no SH do DICOM"
    );
    assert_eq!(corpo["paciente"]["nome"], ANTONIO);
    assert_eq!(corpo["paciente"]["nascimento"], NASC_ANTONIO);
    assert_eq!(resp.headers()["cache-control"], "no-store");
    assert!(resp.headers().contains_key("x-trace-id"));

    let situacao: String =
        sqlx::query_scalar("SELECT situacao FROM agenda.agendamento WHERE id = $1")
            .bind(amb.agendamento_antonio)
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(situacao, "em_atendimento");
    assert_eq!(amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'checkin_realizado' AND hash IS NOT NULL").await, 1);
    // A worklist só sai no bipe da pulseira na sala (FC-PUL-01).
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox").await,
        0
    );
    assert_eq!(corpo["situacao"], "aguardando_sala");
    assert_eq!(corpo["senha"], "A001");
    assert!(corpo["pulseira"]["codigo"]
        .as_str()
        .unwrap()
        .starts_with("ETQ-"));
    assert_eq!(corpo["pulseira"]["situacao"], "valida");
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.etiqueta WHERE situacao = 'valida'")
            .await,
        1
    );
    let dados: serde_json::Value =
        sqlx::query_scalar("SELECT dados FROM lastro.evento WHERE tipo = 'checkin_realizado'")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert!(
        !dados.to_string().contains("Ribeiro"),
        "lastro não guarda nome do paciente"
    );
}

#[tokio::test]
async fn nascimento_divergente_nao_faz_checkin_e_nao_grava_nada() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (status, resp, corpo) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            ANTONIO,
            "1954-06-16",
        ))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(corpo["codigo"], "identificacao-nao-confere");
    assert_eq!(resp.headers()["content-type"], "application/problem+json");
    assert!(corpo["trace_id"].is_string());
    assert!(corpo["detail"].as_str().unwrap().contains("não foi feito"));
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.atendimento")
            .await,
        0
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.idempotencia")
            .await,
        0
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox").await,
        0
    );
}

#[tokio::test]
async fn homonimo_com_outro_nascimento_e_recusado() {
    // Maria Helena Duarte 12/03/1968 x duplicado 21/03/1968 (design/dados-ficticios.md)
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (status, _, corpo) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.agendamento_maria,
            "Maria Helena Duarte",
            "1968-03-21",
        ))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(corpo["codigo"], "identificacao-nao-confere");
}

#[tokio::test]
async fn nome_de_uma_palavra_e_recusado() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (status, _, corpo) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            "Antônio",
            NASC_ANTONIO,
        ))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(corpo["codigo"], "nome-incompleto");
}

#[tokio::test]
async fn duas_recepcoes_ao_mesmo_tempo_geram_um_so_atendimento() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let a = amb.checkin(
        &ana,
        Some(Uuid::now_v7()),
        amb.agendamento_antonio,
        ANTONIO,
        NASC_ANTONIO,
    );
    let b = amb.checkin(
        &ana,
        Some(Uuid::now_v7()),
        amb.agendamento_antonio,
        ANTONIO,
        NASC_ANTONIO,
    );
    let ((s1, _, c1), (s2, _, c2)) = tokio::join!(amb.pedir(a), amb.pedir(b));
    let mut status = [s1, s2];
    status.sort();
    assert_eq!(
        status,
        [StatusCode::CREATED, StatusCode::CONFLICT],
        "{c1} {c2}"
    );
    let conflito = if s1 == StatusCode::CONFLICT { c1 } else { c2 };
    assert_eq!(conflito["codigo"], "checkin-ja-realizado");
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.atendimento")
            .await,
        1
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.etiqueta").await,
        1,
        "uma só pulseira"
    );
}

#[tokio::test]
async fn repetir_com_a_mesma_idempotency_key_devolve_a_mesma_resposta() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let chave = Uuid::now_v7();
    let (s1, _, c1) = amb
        .pedir(amb.checkin(
            &ana,
            Some(chave),
            amb.agendamento_antonio,
            ANTONIO,
            NASC_ANTONIO,
        ))
        .await;
    let (s2, _, c2) = amb
        .pedir(amb.checkin(
            &ana,
            Some(chave),
            amb.agendamento_antonio,
            ANTONIO,
            NASC_ANTONIO,
        ))
        .await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(s2, StatusCode::CREATED);
    assert_eq!(c1, c2);
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.atendimento")
            .await,
        1
    );
}

#[tokio::test]
async fn mesma_chave_ao_mesmo_tempo_tambem_nao_duplica() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let chave = Uuid::now_v7();
    let a = amb.checkin(
        &ana,
        Some(chave),
        amb.agendamento_antonio,
        ANTONIO,
        NASC_ANTONIO,
    );
    let b = amb.checkin(
        &ana,
        Some(chave),
        amb.agendamento_antonio,
        ANTONIO,
        NASC_ANTONIO,
    );
    let ((s1, _, c1), (s2, _, c2)) = tokio::join!(amb.pedir(a), amb.pedir(b));
    assert_eq!((s1, s2), (StatusCode::CREATED, StatusCode::CREATED));
    assert_eq!(c1, c2);
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.atendimento")
            .await,
        1
    );
}

#[tokio::test]
async fn sem_idempotency_key_nada_e_gravado() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (status, _, corpo) = amb
        .pedir(amb.checkin(&ana, None, amb.agendamento_antonio, ANTONIO, NASC_ANTONIO))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(corpo["codigo"], "idempotency-key-ausente");
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.atendimento")
            .await,
        0
    );
}

#[tokio::test]
async fn escrita_sem_csrf_e_recusada() {
    let amb = Ambiente::novo().await;
    let mut ana = amb.entrar("ana.souza").await;
    ana.csrf = "token-forjado".into();
    let (status, _, corpo) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            ANTONIO,
            NASC_ANTONIO,
        ))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(corpo["codigo"], "csrf-invalido");
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.atendimento")
            .await,
        0
    );
}

#[tokio::test]
async fn sem_sessao_responde_401() {
    let amb = Ambiente::novo().await;
    let (status, _, corpo) = amb.pedir(amb.get("/v1/agenda?data=2026-11-06", None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(corpo["codigo"], "sessao-ausente");
}

#[tokio::test]
async fn sessao_revogada_nao_entra() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    sqlx::query("UPDATE identidade.sessao SET revogada_em = now()")
        .execute(&amb.admin)
        .await
        .unwrap();
    let (status, _, _) = amb.pedir(amb.get("/v1/sessao", Some(&ana))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn perfil_sem_o_ato_nao_faz_checkin() {
    let amb = Ambiente::novo().await;
    let medico = amb.entrar("medico.a").await;
    let (status, _, corpo) = amb
        .pedir(amb.checkin(
            &medico,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            ANTONIO,
            NASC_ANTONIO,
        ))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(corpo["codigo"], "sem-permissao");
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.atendimento")
            .await,
        0
    );
}

#[tokio::test]
async fn ti_nao_ve_agenda_nem_dado_de_paciente() {
    let amb = Ambiente::novo().await;
    let ti = amb.entrar("ti.a").await;
    let uri = format!("/v1/agenda?data={}", amb.dia);
    let (status, _, _) = amb.pedir(amb.get(&uri, Some(&ti))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn recepcao_de_outra_unidade_nao_alcanca_o_agendamento() {
    let amb = Ambiente::novo().await;
    let outra = amb.entrar("recepcao.b").await;
    let (status, _, corpo) = amb
        .pedir(amb.checkin(
            &outra,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            ANTONIO,
            NASC_ANTONIO,
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{corpo}");
    assert_eq!(corpo["codigo"], "agendamento-nao-encontrado");
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.atendimento")
            .await,
        0
    );
}

#[tokio::test]
async fn atendimento_de_outra_unidade_nao_aparece() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (_, _, corpo) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            ANTONIO,
            NASC_ANTONIO,
        ))
        .await;
    let id = corpo["id"].as_str().unwrap();
    let (s_ok, _, c_ok) = amb
        .pedir(amb.get(&format!("/v1/atendimentos/{id}"), Some(&ana)))
        .await;
    assert_eq!(s_ok, StatusCode::OK);
    assert_eq!(c_ok["codigo"], corpo["codigo"]);
    let outra = amb.entrar("recepcao.b").await;
    let (status, _, _) = amb
        .pedir(amb.get(&format!("/v1/atendimentos/{id}"), Some(&outra)))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn checkin_ja_feito_e_recusado_depois() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (s1, _, _) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            ANTONIO,
            NASC_ANTONIO,
        ))
        .await;
    let (s2, _, c2) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            ANTONIO,
            NASC_ANTONIO,
        ))
        .await;
    assert_eq!(s1, StatusCode::CREATED);
    assert_eq!(s2, StatusCode::CONFLICT);
    assert_eq!(c2["codigo"], "checkin-ja-realizado");
}

#[tokio::test]
async fn agendamento_cancelado_nao_permite_checkin() {
    let amb = Ambiente::novo().await;
    sqlx::query("UPDATE agenda.agendamento SET situacao = 'cancelado' WHERE id = $1")
        .bind(amb.agendamento_antonio)
        .execute(&amb.admin)
        .await
        .unwrap();
    let ana = amb.entrar("ana.souza").await;
    let (status, _, corpo) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            ANTONIO,
            NASC_ANTONIO,
        ))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(corpo["codigo"], "situacao-nao-permite-checkin");
}

#[tokio::test]
async fn agendamento_inexistente_responde_404() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (status, _, _) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            Uuid::now_v7(),
            ANTONIO,
            NASC_ANTONIO,
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn corpo_malformado_responde_422_em_problem_details() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/atendimentos")
        .header("content-type", "application/json")
        .header("cookie", &ana.cookie)
        .header("x-csrf-token", &ana.csrf)
        .header("idempotency-key", Uuid::now_v7().to_string())
        .body(axum::body::Body::from("{\"agendamento_id\": 5}"))
        .unwrap();
    let (status, _, corpo) = amb.pedir(req).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(corpo["codigo"], "corpo-invalido");
}
