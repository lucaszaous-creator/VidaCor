//! Minha agenda do médico e ausência com aprovação da coordenação (docs/27 FC-MAG-01 a 08, proposta
//! aguardando aprovação; DEC-2026-11-09-140 a 143): rota HTTP real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use chrono::{Duration, TimeZone, Utc};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

fn dia_as(dias: i64, hora: u32) -> chrono::DateTime<Utc> {
    let dia = vidacor_api::tempo::hoje_local() + Duration::days(dias);
    chrono_tz::America::Sao_Paulo
        .from_local_datetime(&dia.and_hms_opt(hora, 0, 0).unwrap())
        .unwrap()
        .with_timezone(&Utc)
}

async fn minha_agenda(amb: &Ambiente, login: &str) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, c) = amb.pedir(amb.get("/v1/minha-agenda", Some(&s))).await;
    (st, c)
}

async fn pedir_ausencia(
    amb: &Ambiente,
    login: &str,
    chave: Uuid,
    de: i64,
    ate: i64,
) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/minha-agenda/ausencias",
            &s,
            Some(chave),
            json!({ "inicio": dia_as(de, 0), "fim": dia_as(ate, 0), "motivo": "congresso", "detalhe": "Congresso de cardiologia" }),
        ))
        .await;
    (st, c)
}

async fn decidir(amb: &Ambiente, login: &str, id: &str, corpo: Value) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/ausencias/{id}/decisao"),
            &s,
            Some(Uuid::now_v7()),
            corpo,
        ))
        .await;
    (st, c)
}

async fn agendar_com_medico(
    amb: &Ambiente,
    dias: i64,
    hora: u32,
    medico: Uuid,
) -> (StatusCode, Value) {
    let ana = amb.entrar("ana.souza").await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/agendamentos",
            &ana,
            Some(Uuid::now_v7()),
            json!({ "paciente_id": amb.maria, "sala_id": amb.sala1, "inicio": dia_as(dias, hora),
                    "procedimentos": ["40901114"], "guia_id": null, "reserva_provisoria": false, "medico_id": medico }),
        ))
        .await;
    (st, c)
}

#[tokio::test]
async fn fc_mag_01_o_medico_ve_so_os_proprios_pacientes_com_dois_identificadores() {
    let amb = Ambiente::novo().await;
    let (st, a) = minha_agenda(&amb, "medico.a").await;
    assert_eq!(st, StatusCode::OK, "{a}");
    let itens = a["itens"].as_array().unwrap();
    // Na semente, só Antônio está agendado com o Dr. Paulo (Maria não tem médico; Vera é da Dra. Cecília).
    assert_eq!(itens.len(), 1, "{a}");
    assert_eq!(itens[0]["paciente"]["nome"], "Antônio Carlos Ribeiro");
    assert_eq!(itens[0]["paciente"]["nascimento"], "1954-06-15");
    assert_eq!(itens[0]["grupo"], "a_chegar");
    assert_eq!(a["pacientes"], 1);
    assert_eq!(a["medico"], "Dr. Paulo Medeiros");
    assert_eq!(a["turnos"][0]["sala"], "Sala 1");
    assert_eq!(a["semana"].as_array().unwrap().len(), 6);
    let (_, c) = minha_agenda(&amb, "cecilia.souto").await;
    assert_eq!(c["itens"].as_array().unwrap().len(), 1);
    assert_eq!(c["itens"][0]["paciente"]["nome"], "Vera Lúcia Antunes");
}

#[tokio::test]
async fn fc_mag_02_depois_do_checkin_o_paciente_passa_a_esperando() {
    let amb = Ambiente::novo().await;
    amb.fazer_checkin("Antônio Carlos Ribeiro", "1954-06-15")
        .await;
    let (_, a) = minha_agenda(&amb, "medico.a").await;
    assert_eq!(a["itens"][0]["grupo"], "esperando", "{a}");
    assert!(a["itens"][0]["esperando_desde"].is_string());
    assert!(a["itens"][0]["senha"].is_string());
    assert_eq!(a["esperando"], 1);
}

#[tokio::test]
async fn fc_mag_03_quem_nao_e_medico_nao_abre_minha_agenda() {
    let amb = Ambiente::novo().await;
    for login in [
        "ana.souza",
        "carla.caixa",
        "bruno.sales",
        "patricia.lemos",
        "ti.a",
        "rita.faturamento",
    ] {
        let (st, _) = minha_agenda(&amb, login).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    let (st, _, _) = amb.pedir(amb.get("/v1/minha-agenda", None)).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn fc_mag_04_pedido_conta_os_afetados_e_so_bloqueia_depois_da_aprovacao() {
    let amb = Ambiente::novo().await;
    let paulo = amb.s.usuarios["medico.a"];
    // Um paciente já agendado com o Dr. Paulo daqui a 3 dias.
    let (st, _) = agendar_com_medico(&amb, 3, 10, paulo).await;
    assert_eq!(st, StatusCode::CREATED);
    let (st, p) = pedir_ausencia(&amb, "medico.a", Uuid::now_v7(), 3, 5).await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    assert_eq!(p["situacao"], "aguardando");
    assert_eq!(p["afetados_no_pedido"], 1);
    assert!(p["codigo"].as_str().unwrap().starts_with("AUS-"));
    // Enquanto aguarda, a agenda continua aberta.
    let (st, _) = agendar_com_medico(&amb, 4, 9, paulo).await;
    assert_eq!(st, StatusCode::CREATED);
    let id = p["id"].as_str().unwrap();
    let (st, d) = decidir(&amb, "patricia.lemos", id, json!({ "aprovar": true })).await;
    assert_eq!(st, StatusCode::OK, "{d}");
    assert_eq!(d["situacao"], "aprovada");
    assert_eq!(d["afetados_na_decisao"], 2);
    assert_eq!(d["decidido_por"], "Patrícia Lemos");
    // Aprovado: agendamento novo com o Dr. Paulo no período é recusado; com outro médico, não.
    let (st, e) = agendar_com_medico(&amb, 4, 11, paulo).await;
    assert_eq!(st, StatusCode::CONFLICT, "{e}");
    assert_eq!(e["codigo"], "medico-ausente");
    let (st, _) = agendar_com_medico(&amb, 4, 13, amb.s.usuarios["marcos.freire"]).await;
    assert_eq!(st, StatusCode::CREATED);
    // A semana mostra a ausência; o lastro registra pedido e aprovação.
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo IN ('ausencia_pedida','ausencia_aprovada')").await,
        2
    );
}

#[tokio::test]
async fn fc_mag_05_o_medico_nao_aprova_o_proprio_pedido_e_a_recusa_exige_motivo() {
    let amb = Ambiente::novo().await;
    let (_, p) = pedir_ausencia(&amb, "medico.a", Uuid::now_v7(), 7, 8).await;
    let id = p["id"].as_str().unwrap();
    let (st, _) = decidir(&amb, "medico.a", id, json!({ "aprovar": true })).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = decidir(&amb, "ana.souza", id, json!({ "aprovar": true })).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, e) = decidir(&amb, "patricia.lemos", id, json!({ "aprovar": false })).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(e["codigo"], "motivo-obrigatorio");
    let (st, r) = decidir(
        &amb,
        "patricia.lemos",
        id,
        json!({ "aprovar": false, "motivo": "Semana de mutirão já marcada" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(r["situacao"], "recusada");
    // Já decidido: nova decisão é recusada.
    let (st, e) = decidir(&amb, "patricia.lemos", id, json!({ "aprovar": true })).await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(e["codigo"], "ausencia-ja-decidida");
}

#[tokio::test]
async fn fc_mag_06_outro_medico_nao_ve_nem_cancela_o_pedido() {
    let amb = Ambiente::novo().await;
    let (_, p) = pedir_ausencia(&amb, "medico.a", Uuid::now_v7(), 7, 8).await;
    let id = p["id"].as_str().unwrap();
    let cecilia = amb.entrar("cecilia.souto").await;
    let (st, _, lista) = amb.pedir(amb.get("/v1/ausencias", Some(&cecilia))).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(lista.as_array().unwrap().len(), 0, "{lista}");
    let (st, _, e) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/ausencias/{id}/cancelamento"),
            &cecilia,
            Some(Uuid::now_v7()),
            json!({}),
        ))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "{e}");
    // A recepção vê o pedido da unidade (para remarcar); o caixa não.
    let ana = amb.entrar("ana.souza").await;
    let (_, _, lista) = amb.pedir(amb.get("/v1/ausencias", Some(&ana))).await;
    assert_eq!(lista.as_array().unwrap().len(), 1);
    let carla = amb.entrar("carla.caixa").await;
    let (st, _, _) = amb.pedir(amb.get("/v1/ausencias", Some(&carla))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // O próprio médico cancela enquanto aguarda.
    let paulo = amb.entrar("medico.a").await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/ausencias/{id}/cancelamento"),
            &paulo,
            Some(Uuid::now_v7()),
            json!({}),
        ))
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(c["situacao"], "cancelada");
}

#[tokio::test]
async fn fc_mag_07_limites_do_pedido() {
    let amb = Ambiente::novo().await;
    let s = amb.entrar("medico.a").await;
    for (corpo, codigo) in [
        (
            json!({ "inicio": dia_as(3, 8), "fim": dia_as(3, 8), "motivo": "ferias" }),
            "ausencia-invalida",
        ),
        (
            json!({ "inicio": dia_as(3, 8), "fim": dia_as(70, 8), "motivo": "ferias" }),
            "ausencia-invalida",
        ),
        (
            json!({ "inicio": dia_as(-5, 8), "fim": dia_as(-4, 8), "motivo": "ferias" }),
            "ausencia-invalida",
        ),
        (
            json!({ "inicio": dia_as(3, 8), "fim": dia_as(4, 8), "motivo": "viagem" }),
            "ausencia-invalida",
        ),
    ] {
        let (st, _, e) = amb
            .pedir(amb.json(
                Method::POST,
                "/v1/minha-agenda/ausencias",
                &s,
                Some(Uuid::now_v7()),
                corpo,
            ))
            .await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{e}");
        assert_eq!(e["codigo"], codigo);
    }
    assert_eq!(
        amb.contar("SELECT count(*) FROM agenda.ausencia_medico")
            .await,
        0
    );
}

#[tokio::test]
async fn fc_mag_08_clique_duplo_e_pedidos_simultaneos_nao_duplicam() {
    let amb = Ambiente::novo().await;
    // Mesma chave: mesma resposta, um registro só.
    let chave = Uuid::now_v7();
    let (s1, a) = pedir_ausencia(&amb, "medico.a", chave, 10, 12).await;
    let (s2, b) = pedir_ausencia(&amb, "medico.a", chave, 10, 12).await;
    assert_eq!((s1, s2), (StatusCode::CREATED, StatusCode::CREATED));
    assert_eq!(a["id"], b["id"]);
    // Duas abas, chaves diferentes, períodos sobrepostos ao mesmo tempo: só um entra (G2).
    let (r1, r2) = tokio::join!(
        pedir_ausencia(&amb, "cecilia.souto", Uuid::now_v7(), 20, 22),
        pedir_ausencia(&amb, "cecilia.souto", Uuid::now_v7(), 21, 23)
    );
    let mut st = [r1.0, r2.0];
    st.sort();
    assert_eq!(
        st,
        [StatusCode::CREATED, StatusCode::CONFLICT],
        "{} {}",
        r1.1,
        r2.1
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM agenda.ausencia_medico")
            .await,
        2
    );
    // Duas decisões ao mesmo tempo sobre o mesmo pedido: só uma tem efeito.
    let id = a["id"].as_str().unwrap().to_owned();
    let (d1, d2) = tokio::join!(
        decidir(&amb, "patricia.lemos", &id, json!({ "aprovar": true })),
        decidir(
            &amb,
            "patricia.lemos",
            &id,
            json!({ "aprovar": false, "motivo": "Período de mutirão" })
        )
    );
    let mut st = [d1.0, d2.0];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo IN ('ausencia_aprovada','ausencia_recusada')").await,
        1
    );
}
