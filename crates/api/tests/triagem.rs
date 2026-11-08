//! Triagem com contraste (docs/27 FC-TRI): TFG e resultado calculados pelo servidor, conduta
//! médica com vínculo e reautenticação, bloqueio do exame na sala. Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

const VERA: &str = "Vera Lúcia Antunes";
const NASC_VERA: &str = "1957-09-18";

fn triagem(creatinina: Option<&str>, reacao: bool) -> Value {
    let hoje = vidacor_api::tempo::hoje_local();
    json!({
        "reacao_previa_contraste": reacao, "gestante": false, "amamentando": false, "usa_metformina": false,
        "creatinina_mg_dl": creatinina, "creatinina_data": creatinina.map(|_| (hoje - chrono::Duration::days(11)).to_string()),
        "tfg": 90
    })
}

async fn registrar(
    amb: &Ambiente,
    login: &str,
    atendimento: &str,
    corpo: Value,
) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/atendimentos/{atendimento}/triagem"),
            &s,
            None,
            corpo,
        ))
        .await;
    (st, c)
}

async fn bipar(amb: &Ambiente, etiqueta: &str) -> (StatusCode, Value) {
    let s = amb.entrar("livia.costa").await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/salas/{}/bipes", amb.s.salas[&6]),
            &s,
            Some(Uuid::now_v7()),
            json!({ "codigo_etiqueta": etiqueta }),
        ))
        .await;
    (st, c)
}

#[tokio::test]
async fn fc_tri_01_sem_risco_libera_o_exame() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(VERA, NASC_VERA).await;
    let (st, t) = registrar(
        &amb,
        "livia.costa",
        atd["id"].as_str().unwrap(),
        triagem(Some("0.7"), false),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    assert_eq!(t["resultado"], "liberado");
    assert_eq!(t["exame_liberado"], true);
    let (sb, b) = bipar(&amb, atd["pulseira"]["codigo"].as_str().unwrap()).await;
    assert_eq!(sb, StatusCode::OK, "{b}");
}

#[tokio::test]
async fn fc_tri_02_e_07_tfg_baixa_calculada_pelo_servidor_exige_aprovacao() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(VERA, NASC_VERA).await;
    // A tela manda TFG 90; o servidor ignora e calcula 40 pela CKD-EPI 2021.
    let (st, t) = registrar(
        &amb,
        "livia.costa",
        atd["id"].as_str().unwrap(),
        triagem(Some("1.4"), false),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    assert_eq!(t["tfg_calculada"], 40);
    assert_eq!(t["resultado"], "precisa_aprovacao_medica");
    assert_eq!(t["regra_versao"], "TRI-2026.1");
    let gravada: i32 = sqlx::query_scalar("SELECT tfg_calculada FROM triagem.triagem")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    assert_eq!(gravada, 40);
}

#[tokio::test]
async fn fc_tri_03_exame_bloqueado_nao_recebe_worklist() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(VERA, NASC_VERA).await;
    registrar(
        &amb,
        "livia.costa",
        atd["id"].as_str().unwrap(),
        triagem(Some("1.4"), false),
    )
    .await;
    let (sb, b) = bipar(&amb, atd["pulseira"]["codigo"].as_str().unwrap()).await;
    assert_eq!(sb, StatusCode::CONFLICT);
    assert_eq!(b["codigo"], "conduta-medica-pendente");
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox").await,
        0
    );
}

#[tokio::test]
async fn fc_tri_04_medica_com_vinculo_registra_conduta_e_libera() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(VERA, NASC_VERA).await;
    let id = atd["id"].as_str().unwrap();
    registrar(&amb, "livia.costa", id, triagem(Some("1.4"), false)).await;
    let cecilia = amb.entrar("cecilia.souto").await;
    let (st, _, t) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/atendimentos/{id}/triagem/conduta"),
            &cecilia,
            None,
            json!({ "conduta": "prosseguir com hidratação" }),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{t}");
    assert_eq!(t["exame_liberado"], true);
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'conduta_registrada'")
            .await,
        1
    );
    let (sb, _) = bipar(&amb, atd["pulseira"]["codigo"].as_str().unwrap()).await;
    assert_eq!(sb, StatusCode::OK);
}

#[tokio::test]
async fn conduta_sem_reautenticacao_recente_e_recusada() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(VERA, NASC_VERA).await;
    let id = atd["id"].as_str().unwrap();
    registrar(&amb, "livia.costa", id, triagem(Some("1.4"), false)).await;
    let cecilia = amb.entrar("cecilia.souto").await;
    amb.expirar_reautenticacao().await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/atendimentos/{id}/triagem/conduta"),
            &cecilia,
            None,
            json!({ "conduta": "prosseguir com hidratação" }),
        ))
        .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    assert_eq!(c["codigo"], "reautenticacao-necessaria");
}

#[tokio::test]
async fn medico_sem_vinculo_nao_registra_conduta() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(VERA, NASC_VERA).await;
    let id = atd["id"].as_str().unwrap();
    registrar(&amb, "livia.costa", id, triagem(Some("1.4"), false)).await;
    let marcos = amb.entrar("marcos.freire").await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/atendimentos/{id}/triagem/conduta"),
            &marcos,
            None,
            json!({ "conduta": "prosseguir" }),
        ))
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    assert_eq!(c["codigo"], "sem-vinculo-de-cuidado");
}

#[tokio::test]
async fn fc_tri_05_tecnico_nao_aprova_a_propria_triagem() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(VERA, NASC_VERA).await;
    let id = atd["id"].as_str().unwrap();
    registrar(&amb, "livia.costa", id, triagem(Some("1.4"), false)).await;
    let livia = amb.entrar("livia.costa").await;
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/atendimentos/{id}/triagem/conduta"),
            &livia,
            None,
            json!({ "conduta": "prosseguir" }),
        ))
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn fc_tri_06_exame_com_contraste_sem_triagem_nao_inicia() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(VERA, NASC_VERA).await;
    let (sb, b) = bipar(&amb, atd["pulseira"]["codigo"].as_str().unwrap()).await;
    assert_eq!(sb, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(b["codigo"], "triagem-ausente");
}

#[tokio::test]
async fn triagem_por_tecnico_de_outra_sala_e_recusada() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(VERA, NASC_VERA).await;
    let (st, _) = registrar(
        &amb,
        "bruno.sales",
        atd["id"].as_str().unwrap(),
        triagem(Some("0.7"), false),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}
