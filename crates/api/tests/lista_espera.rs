//! Bloqueios, lista de espera, oferta de vaga e cancelamento (Lista-Espera.dc.html; FC-AGD-08).
//! Rota real e PostgreSQL real (G4): caminho feliz, erros, repetição, 2 requisições simultâneas,
//! limites e acesso indevido.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use chrono::{Duration, TimeZone, Utc};
use comum::{Ambiente, UNIDADE_A};
use serde_json::{json, Value};
use uuid::Uuid;

/// Amanhã às 10:00 em Brasília (13:00 UTC), para a vaga não estar no passado.
fn amanha_10h(amb: &Ambiente) -> chrono::DateTime<Utc> {
    Utc.from_utc_datetime(&(amb.dia + Duration::days(1)).and_hms_opt(13, 0, 0).unwrap())
}

async fn agendamento_futuro(amb: &Ambiente, paciente: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO agenda.agendamento (id, codigo, sala_id, paciente_id, inicio, fim, exame_tuss, exame_nome, situacao, origem, unidade_id, correlacao)
                 VALUES ($1, $2, $3, $4, $5, $5 + interval '30 minutes', '40901114', 'Ecocardiograma transtorácico', 'agendado', 'seed', $6, 'seed')")
        .bind(id).bind(format!("AGD-T-{}", id.simple())).bind(amb.s.salas[&1]).bind(amb.s.pacientes[paciente]).bind(amanha_10h(amb)).bind(UNIDADE_A)
        .execute(&amb.admin).await.unwrap();
    id
}

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

async fn incluir(amb: &Ambiente, paciente: &str, prioridade: &str) -> (StatusCode, Value) {
    let mut corpo = json!({ "paciente_id": amb.s.pacientes[paciente], "exame_tuss": "40901114", "prioridade": prioridade, "preferencia": "qualquer" });
    if prioridade == "urgente" {
        corpo["justificativa"] = json!("suspeita de valvopatia grave");
        corpo["medico_solicitante"] = json!("Dr. Paulo Medeiros");
    }
    post(
        amb,
        "ana.souza",
        "/v1/lista-espera",
        Some(Uuid::now_v7()),
        corpo,
    )
    .await
}

#[tokio::test]
async fn bloqueio_impede_agendar_e_dois_bloqueios_iguais_ao_mesmo_tempo_so_um_vale() {
    let amb = Ambiente::novo().await;
    let inicio = amanha_10h(&amb) + Duration::hours(2);
    let corpo = json!({ "sala_ids": [amb.s.salas[&2]], "tipo": "manutencao", "inicio": inicio, "fim": inicio + Duration::hours(2), "motivo": "manutenção preventiva" });
    let s = amb.entrar("ana.souza").await;
    let (r1, r2) = tokio::join!(
        amb.pedir(amb.json(
            Method::POST,
            "/v1/bloqueios",
            &s,
            Some(Uuid::now_v7()),
            corpo.clone()
        )),
        amb.pedir(amb.json(
            Method::POST,
            "/v1/bloqueios",
            &s,
            Some(Uuid::now_v7()),
            corpo.clone()
        ))
    );
    let mut st = [r1.0, r2.0];
    st.sort();
    assert_eq!(st[0], StatusCode::CREATED, "{} {}", r1.2, r2.2);
    assert_ne!(st[1], StatusCode::CREATED, "só um bloqueio sobreposto vale");
    assert_eq!(
        amb.contar("SELECT count(*) FROM agenda.bloqueio WHERE situacao = 'ativo'")
            .await,
        1
    );
    // Agendar dentro do bloqueio: o banco recusa.
    let r = sqlx::query("INSERT INTO agenda.agendamento (id, codigo, sala_id, paciente_id, inicio, fim, exame_tuss, exame_nome, situacao, origem, unidade_id, correlacao)
                 VALUES ($1, 'AGD-BLOQ', $2, $3, $4, $4 + interval '30 minutes', '40901114', 'Eco', 'agendado', 'teste', $5, 'teste')")
        .bind(Uuid::now_v7()).bind(amb.s.salas[&2]).bind(amb.s.pacientes["Maria Helena Duarte"]).bind(inicio + Duration::minutes(30)).bind(UNIDADE_A)
        .execute(&amb.admin).await;
    assert!(r.is_err(), "agendamento dentro do bloqueio");
    // Bloquear por cima de agendamento existente: recusado, nada gravado.
    agendamento_futuro(&amb, "Antônio Carlos Ribeiro").await;
    let (sb, vb) = post(&amb, "ana.souza", "/v1/bloqueios", Some(Uuid::now_v7()),
        json!({ "sala_ids": [amb.s.salas[&1]], "tipo": "feriado", "inicio": amanha_10h(&amb), "fim": amanha_10h(&amb) + Duration::hours(1), "motivo": "feriado municipal" })).await;
    assert_eq!(sb, StatusCode::CONFLICT, "{vb}");
    // Bloqueio não se apaga.
    assert!(sqlx::query("DELETE FROM agenda.bloqueio")
        .execute(&amb.admin)
        .await
        .is_err());
}

#[tokio::test]
async fn espera_repetida_nao_duplica_urgente_pede_justificativa_e_acesso_indevido() {
    let amb = Ambiente::novo().await;
    let (st, p) = incluir(&amb, "Maria Helena Duarte", "rotina").await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    let (s2, _) = incluir(&amb, "Maria Helena Duarte", "rotina").await;
    assert_eq!(
        s2,
        StatusCode::CONFLICT,
        "um pedido ativo por paciente e exame"
    );
    let (s3, v3) = post(&amb, "ana.souza", "/v1/lista-espera", Some(Uuid::now_v7()),
        json!({ "paciente_id": amb.s.pacientes["Vera Lúcia Antunes"], "exame_tuss": "40901114", "prioridade": "urgente", "preferencia": "qualquer" })).await;
    assert_eq!(s3, StatusCode::UNPROCESSABLE_ENTITY, "{v3}");
    for login in [
        "carla.caixa",
        "medico.a",
        "ti.a",
        "tv.a",
        "bruno.sales",
        "financeiro.a",
    ] {
        let s = amb.entrar(login).await;
        let (sl, _, _) = amb.pedir(amb.get("/v1/lista-espera", Some(&s))).await;
        assert_eq!(sl, StatusCode::FORBIDDEN, "{login}");
    }
    let b = amb.entrar("recepcao.b").await;
    let (_, _, lb) = amb.pedir(amb.get("/v1/lista-espera", Some(&b))).await;
    assert_eq!(lb.as_array().map(Vec::len), Some(0), "outra unidade não vê");
}

#[tokio::test]
async fn cancelar_devolve_a_vaga_ao_primeiro_da_fila_e_aceite_duplo_agenda_uma_vez() {
    let amb = Ambiente::novo().await;
    let ag = agendamento_futuro(&amb, "Antônio Carlos Ribeiro").await;
    let (_, rotina) = incluir(&amb, "Vera Lúcia Antunes", "rotina").await;
    let (_, urgente) = incluir(&amb, "Maria Helena Duarte", "urgente").await;
    // Motivo curto não cancela.
    let uri = format!("/v1/agendamentos/{ag}/cancelamento");
    let (sm, _) = post(&amb, "ana.souza", &uri, None, json!({ "motivo": "x" })).await;
    assert_eq!(sm, StatusCode::UNPROCESSABLE_ENTITY);
    let (sc, c) = post(
        &amb,
        "ana.souza",
        &uri,
        None,
        json!({ "motivo": "paciente viajou" }),
    )
    .await;
    assert_eq!(
        (sc, c["situacao"].as_str()),
        (StatusCode::OK, Some("cancelado")),
        "{c}"
    );
    let (sc2, _) = post(
        &amb,
        "ana.souza",
        &uri,
        None,
        json!({ "motivo": "paciente viajou" }),
    )
    .await;
    assert_eq!(sc2, StatusCode::CONFLICT, "cancelar de novo não muda nada");
    // O trabalhador do outbox oferece a vaga: o urgente passa na frente da rotina.
    let carga: Value = sqlx::query_scalar("SELECT carga FROM integracao.outbox WHERE tipo = 'lista_espera.oferecer_vaga' AND referencia = $1")
        .bind(ag).fetch_one(&amb.admin).await.unwrap();
    vidacor_api::rotas::vagas::oferecer_vaga(&amb.estado, UNIDADE_A, &carga)
        .await
        .unwrap();
    let (oferta_id, espera_id): (Uuid, Uuid) = sqlx::query_as("SELECT id, espera_id FROM agenda.oferta WHERE situacao = 'aguardando' ORDER BY inicio LIMIT 1")
        .fetch_one(&amb.admin).await.unwrap();
    assert_eq!(espera_id.to_string(), urgente["id"].as_str().unwrap());
    assert_ne!(espera_id.to_string(), rotina["id"].as_str().unwrap());
    // Aceite com nome errado: nada agendado.
    let aceite = format!("/v1/ofertas/{oferta_id}/aceite");
    let (se, _) = post(
        &amb,
        "ana.souza",
        &aceite,
        Some(Uuid::now_v7()),
        json!({ "confirmacao": { "nome": "Maria Duarte", "nascimento": "1968-03-12" } }),
    )
    .await;
    assert_eq!(se, StatusCode::UNPROCESSABLE_ENTITY);
    // Dois aceites ao mesmo tempo, com chaves diferentes: um só agendamento.
    let s = amb.entrar("ana.souza").await;
    let corpo =
        json!({ "confirmacao": { "nome": "Maria Helena Duarte", "nascimento": "1968-03-12" } });
    let (a1, a2) = tokio::join!(
        amb.pedir(amb.json(
            Method::POST,
            &aceite,
            &s,
            Some(Uuid::now_v7()),
            corpo.clone()
        )),
        amb.pedir(amb.json(
            Method::POST,
            &aceite,
            &s,
            Some(Uuid::now_v7()),
            corpo.clone()
        ))
    );
    let mut st = [a1.0, a2.0];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT], "{} {}", a1.2, a2.2);
    assert_eq!(
        amb.contar("SELECT count(*) FROM agenda.oferta WHERE situacao = 'aceita'")
            .await,
        1
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM agenda.espera WHERE situacao = 'encaixado'")
            .await,
        1
    );
    // O aviso da oferta foi para o outbox (WhatsApp).
    assert!(
        amb.contar("SELECT count(*) FROM integracao.outbox WHERE tipo = 'aviso.oferta_vaga'")
            .await
            >= 1
    );
}
