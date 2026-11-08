//! Agendamento de convênio com guia (docs/27 FC-AGD-01 a 11): rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use chrono::{Duration, TimeZone, Utc};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

const ANTONIO: &str = "Antônio Carlos Ribeiro";

/// Amanhã às 10:00 em Brasília (horário livre na Sala 1).
fn amanha_as(hora: u32, minuto: u32) -> chrono::DateTime<Utc> {
    let dia = vidacor_api::tempo::hoje_local() + Duration::days(1);
    chrono_tz::America::Sao_Paulo
        .from_local_datetime(&dia.and_hms_opt(hora, minuto, 0).unwrap())
        .unwrap()
        .with_timezone(&Utc)
}

async fn guia(amb: &Ambiente, numero: &str, validade_dias: i64, procedimentos: Value) -> Value {
    let ana = amb.entrar("ana.souza").await;
    let validade = vidacor_api::tempo::hoje_local() + Duration::days(validade_dias);
    let (st, _, g) = amb
        .pedir(amb.json(Method::POST, "/v1/guias", &ana, Some(Uuid::now_v7()), json!({
            "operadora": "Unimed", "numero": numero, "carteirinha": "0 123 4567", "senha": "UN-88231-X",
            "validade": validade, "paciente_id": amb.antonio, "solicitante": "Dr. Júlio Prado",
            "procedimentos": procedimentos
        })))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{g}");
    g
}

fn pedido(
    amb: &Ambiente,
    inicio: chrono::DateTime<Utc>,
    procedimentos: &[&str],
    guia: Option<&Value>,
    reserva: bool,
) -> Value {
    json!({
        "paciente_id": amb.antonio, "sala_id": amb.sala1, "inicio": inicio,
        "procedimentos": procedimentos, "guia_id": guia.map(|g| g["id"].clone()), "reserva_provisoria": reserva
    })
}

async fn agendar(amb: &Ambiente, login: &str, chave: Uuid, corpo: Value) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, c) = amb
        .pedir(amb.json(Method::POST, "/v1/agendamentos", &s, Some(chave), corpo))
        .await;
    (st, c)
}

fn dois() -> Value {
    json!([{ "tuss": "40901114", "quantidade": 1 }, { "tuss": "40901300", "quantidade": 1 }])
}

#[tokio::test]
async fn fc_agd_01_guia_valida_e_consumida_no_agendamento() {
    let amb = Ambiente::novo().await;
    let g = guia(&amb, "UN-55120", 10, dois()).await;
    let (st, a) = agendar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        pedido(
            &amb,
            amanha_as(10, 0),
            &["40901114", "40901300"],
            Some(&g),
            false,
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{a}");
    assert_eq!(a["situacao"], "confirmado");
    let saldos: Vec<i32> =
        sqlx::query_scalar("SELECT saldo FROM convenio.guia_procedimento ORDER BY tuss")
            .fetch_all(&amb.admin)
            .await
            .unwrap();
    assert_eq!(saldos, vec![0, 0]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM convenio.guia WHERE situacao = 'consumida'")
            .await,
        1
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'agendamento_criado'")
            .await,
        1
    );
}

#[tokio::test]
async fn fc_agd_02_guia_ja_usada_e_recusada_com_o_codigo_do_uso() {
    let amb = Ambiente::novo().await;
    let g = guia(
        &amb,
        "UN-55120",
        10,
        json!([{ "tuss": "40901114", "quantidade": 2 }]),
    )
    .await;
    let (s1, a1) = agendar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        pedido(&amb, amanha_as(10, 0), &["40901114"], Some(&g), false),
    )
    .await;
    assert_eq!(s1, StatusCode::CREATED);
    let (s2, a2) = agendar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        pedido(&amb, amanha_as(11, 0), &["40901114"], Some(&g), false),
    )
    .await;
    assert_eq!(s2, StatusCode::CONFLICT);
    assert_eq!(a2["codigo"], "guia-em-uso");
    assert_eq!(a2["atendimento_codigo"], a1["codigo"]);
}

#[tokio::test]
async fn fc_agd_03_dois_agendamentos_simultaneos_com_a_mesma_guia() {
    let amb = Ambiente::novo().await;
    let g = guia(
        &amb,
        "UN-55120",
        10,
        json!([{ "tuss": "40901114", "quantidade": 1 }]),
    )
    .await;
    let ((s1, _), (s2, _)) = tokio::join!(
        agendar(
            &amb,
            "ana.souza",
            Uuid::now_v7(),
            pedido(&amb, amanha_as(10, 0), &["40901114"], Some(&g), false)
        ),
        agendar(
            &amb,
            "ana.souza",
            Uuid::now_v7(),
            pedido(&amb, amanha_as(11, 0), &["40901114"], Some(&g), false)
        )
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM agenda.agendamento WHERE guia_id IS NOT NULL")
            .await,
        1
    );
    let saldo: i32 = sqlx::query_scalar("SELECT saldo FROM convenio.guia_procedimento")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    assert_eq!(saldo, 0, "nunca negativo");
}

#[tokio::test]
async fn fc_agd_04_clique_duplo_nao_duplica() {
    let amb = Ambiente::novo().await;
    let g = guia(&amb, "UN-55120", 10, dois()).await;
    let chave = Uuid::now_v7();
    let corpo = pedido(
        &amb,
        amanha_as(10, 0),
        &["40901114", "40901300"],
        Some(&g),
        false,
    );
    let ((s1, a1), (s2, a2)) = tokio::join!(
        agendar(&amb, "ana.souza", chave, corpo.clone()),
        agendar(&amb, "ana.souza", chave, corpo)
    );
    assert_eq!((s1, s2), (StatusCode::CREATED, StatusCode::CREATED));
    assert_eq!(a1, a2);
    assert_eq!(amb.contar("SELECT count(*) FROM agenda.agendamento WHERE codigo LIKE 'AGD-%' AND origem = 'balcao'").await, 1);
}

#[tokio::test]
async fn fc_agd_05_procedimento_fora_do_autorizado_e_recusado() {
    let amb = Ambiente::novo().await;
    let g = guia(&amb, "UN-55120", 10, dois()).await;
    let (st, a) = agendar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        pedido(&amb, amanha_as(10, 0), &["40901220"], Some(&g), false),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(a["codigo"], "procedimento-nao-autorizado");
    assert_eq!(a["tuss"], "40901220");
    assert_eq!(
        amb.contar("SELECT count(*) FROM convenio.guia WHERE situacao = 'consumida'")
            .await,
        0
    );
}

#[tokio::test]
async fn fc_agd_06_senha_vencida_na_data_do_exame_e_recusada() {
    let amb = Ambiente::novo().await;
    let g = guia(&amb, "UN-55120", 0, dois()).await; // vale até hoje; exame amanhã
    let (st, a) = agendar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        pedido(&amb, amanha_as(10, 0), &["40901114"], Some(&g), false),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(a["codigo"], "senha-vencida");
    assert!(a["validade"].is_string());
}

#[tokio::test]
async fn fc_agd_07_saldo_zero_e_recusado() {
    let amb = Ambiente::novo().await;
    let g = guia(&amb, "UN-55120", 10, dois()).await;
    sqlx::query("UPDATE convenio.guia_procedimento SET saldo = 0 WHERE tuss = '40901114'")
        .execute(&amb.admin)
        .await
        .unwrap();
    let (st, a) = agendar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        pedido(&amb, amanha_as(10, 0), &["40901114"], Some(&g), false),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(a["codigo"], "saldo-esgotado");
}

#[tokio::test]
async fn fc_agd_08_reserva_provisoria_vence_devolve_a_vaga_e_avisa() {
    let amb = Ambiente::novo().await;
    let (st, a) = agendar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        pedido(&amb, amanha_as(10, 0), &["40901114"], None, true),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{a}");
    assert_eq!(a["situacao"], "reserva_provisoria");
    let expira: chrono::DateTime<Utc> = serde_json::from_value(a["expira_em"].clone()).unwrap();
    assert!(expira > Utc::now() + Duration::hours(47));
    sqlx::query("UPDATE agenda.agendamento SET expira_em = now() - interval '1 minute' WHERE situacao = 'reserva_provisoria'")
        .execute(&amb.admin).await.unwrap();
    let vencidas =
        vidacor_api::rotas::agendamentos::expirar_reservas(&amb.estado, comum::UNIDADE_A)
            .await
            .unwrap();
    assert_eq!(vencidas, 1);
    let (situacao, motivo): (String, String) =
        sqlx::query_as("SELECT situacao, motivo FROM agenda.agendamento WHERE id = $1")
            .bind(Uuid::parse_str(a["id"].as_str().unwrap()).unwrap())
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(
        (situacao.as_str(), motivo.as_str()),
        ("expirada", "guia não entregue no prazo")
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox WHERE tipo = 'aviso.reserva_expirada'")
            .await,
        1
    );
    // A vaga voltou: outro paciente pode ocupar o mesmo horário.
    let (s2, _) = agendar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        pedido(&amb, amanha_as(10, 0), &["40901114"], None, false),
    )
    .await;
    assert_eq!(s2, StatusCode::CREATED);
}

#[tokio::test]
async fn fc_agd_09_guia_entregue_no_prazo_confirma_a_reserva() {
    let amb = Ambiente::novo().await;
    let (_, a) = agendar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        pedido(&amb, amanha_as(10, 0), &["40901114"], None, true),
    )
    .await;
    let g = guia(&amb, "UN-55120", 10, dois()).await;
    let ana = amb.entrar("ana.souza").await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/agendamentos/{}/guia", a["id"].as_str().unwrap()),
            &ana,
            None,
            json!({ "guia_id": g["id"] }),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    assert_eq!(c["situacao"], "confirmado");
    assert_eq!(
        amb.contar("SELECT count(*) FROM convenio.guia WHERE situacao = 'consumida'")
            .await,
        1
    );
}

#[tokio::test]
async fn fc_agd_10_perfil_sem_permissao_nao_agenda() {
    let amb = Ambiente::novo().await;
    let g = guia(&amb, "UN-55120", 10, dois()).await;
    let (st, _) = agendar(
        &amb,
        "ti.a",
        Uuid::now_v7(),
        pedido(&amb, amanha_as(10, 0), &["40901114"], Some(&g), false),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    assert_eq!(
        amb.contar("SELECT count(*) FROM convenio.guia WHERE situacao = 'consumida'")
            .await,
        0
    );
}

#[tokio::test]
async fn fc_agd_11_mesmo_horario_da_sala_ao_mesmo_tempo_sugere_proximos_livres() {
    let amb = Ambiente::novo().await;
    let mut maria = pedido(&amb, amanha_as(9, 0), &["40901114"], None, false);
    maria["paciente_id"] = json!(amb.maria);
    let antonio = pedido(&amb, amanha_as(9, 0), &["40901114"], None, false);
    let ((s1, c1), (s2, c2)) = tokio::join!(
        agendar(&amb, "ana.souza", Uuid::now_v7(), antonio),
        agendar(&amb, "ana.souza", Uuid::now_v7(), maria)
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
    let conflito = if s1 == StatusCode::CONFLICT { c1 } else { c2 };
    assert_eq!(conflito["codigo"], "horario-ocupado");
    let sugestoes = conflito["sugestoes"].as_array().unwrap();
    assert_eq!(sugestoes.len(), 3);
    assert_eq!(
        serde_json::from_value::<chrono::DateTime<Utc>>(sugestoes[0].clone()).unwrap(),
        amanha_as(9, 30)
    );
    let _ = ANTONIO;
}
