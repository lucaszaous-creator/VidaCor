//! Estoque com lote e contraste (Estoque.dc.html; docs/25 § 3.3: "baixa de contraste não deixa
//! saldo negativo com duas baixas simultâneas; recall por lote lista os pacientes"). Rota real e
//! PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::{Ambiente, UNIDADE_A};
use serde_json::{json, Value};
use uuid::Uuid;

const VERA: &str = "Vera Lúcia Antunes";
const NASC_VERA: &str = "1957-09-18";

async fn item_contraste(amb: &Ambiente) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO estoque.item (id, codigo, nome, medida, minimo, local, contraste, origem, unidade_id, correlacao) VALUES ($1, 'ITM-CTR', 'Contraste iodado não iônico 350, 100 ml', 'frasco', 20, 'Tomografia', true, 'seed', $2, 'seed')")
        .bind(id).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    id
}

async fn entrada(
    amb: &Ambiente,
    item: Uuid,
    lote: &str,
    validade: &str,
    qtd: i32,
) -> (StatusCode, Value) {
    let s = amb.entrar("estoque.a").await;
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, "/v1/estoque/entradas", &s, Some(Uuid::now_v7()),
            json!({ "item_id": item, "numero_lote": lote, "validade": validade, "quantidade": qtd })))
        .await;
    (st, v)
}

fn daqui(dias: i64) -> String {
    (vidacor_api::tempo::hoje_local() + chrono::Duration::days(dias)).to_string()
}

/// Vera com check-in e triagem liberada (sem risco).
async fn vera_liberada(amb: &Ambiente) -> String {
    let atd = amb.fazer_checkin(VERA, NASC_VERA).await;
    let id = atd["id"].as_str().unwrap().to_owned();
    let livia = amb.entrar("livia.costa").await;
    let hoje = vidacor_api::tempo::hoje_local();
    let (st, _, t) = amb
        .pedir(amb.json(Method::POST, &format!("/v1/atendimentos/{id}/triagem"), &livia, None, json!({
            "reacao_previa_contraste": false, "gestante": false, "amamentando": false, "usa_metformina": false,
            "creatinina_mg_dl": "0.7", "creatinina_data": (hoje - chrono::Duration::days(11)).to_string()
        })))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    id
}

async fn aplicar(
    amb: &Ambiente,
    login: &str,
    atendimento: &str,
    item: Uuid,
    qtd: i32,
    chave: Uuid,
) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/atendimentos/{atendimento}/contraste"),
            &s,
            Some(chave),
            json!({ "item_id": item, "quantidade": qtd }),
        ))
        .await;
    (st, v)
}

#[tokio::test]
async fn entrada_soma_no_lote_e_validade_diferente_e_recusada() {
    let amb = Ambiente::novo().await;
    let item = item_contraste(&amb).await;
    assert_eq!(
        entrada(&amb, item, "L2408-77", &daqui(300), 5).await.0,
        StatusCode::CREATED
    );
    let (st, v) = entrada(&amb, item, "l2408-77", &daqui(300), 3).await;
    assert_eq!(
        (st, v["saldo_do_lote"].as_i64()),
        (StatusCode::CREATED, Some(8)),
        "mesmo lote soma"
    );
    let (s2, e2) = entrada(&amb, item, "L2408-77", &daqui(200), 1).await;
    assert_eq!(
        (s2, e2["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("validade-diferente"))
    );
    let (s3, e3) = entrada(&amb, item, "L-VENCIDO", &daqui(-1), 1).await;
    assert_eq!(
        (s3, e3["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("lote-vencido"))
    );
    let tecnico = amb.entrar("livia.costa").await;
    let (s4, _, _) = amb
        .pedir(amb.json(Method::POST, "/v1/estoque/entradas", &tecnico, Some(Uuid::now_v7()),
            json!({ "item_id": item, "numero_lote": "X1", "validade": daqui(100), "quantidade": 1 })))
        .await;
    assert_eq!(s4, StatusCode::FORBIDDEN, "técnico não dá entrada");
}

#[tokio::test]
async fn contraste_sai_do_lote_que_vence_antes_e_fica_ligado_a_vera() {
    let amb = Ambiente::novo().await;
    let item = item_contraste(&amb).await;
    entrada(&amb, item, "L-TARDE", &daqui(400), 5).await;
    entrada(&amb, item, "L-CEDO", &daqui(60), 2).await;
    let atd = vera_liberada(&amb).await;
    let chave = Uuid::now_v7();
    let (st, m) = aplicar(&amb, "livia.costa", &atd, item, 1, chave).await;
    assert_eq!(st, StatusCode::CREATED, "{m}");
    assert_eq!(
        (m["numero_lote"].as_str(), m["saldo_do_lote"].as_i64()),
        (Some("L-CEDO"), Some(1))
    );
    let (st2, m2) = aplicar(&amb, "livia.costa", &atd, item, 1, chave).await;
    assert_eq!(
        (st2, &m2["movimento_id"]),
        (StatusCode::CREATED, &m["movimento_id"]),
        "repetir não baixa de novo"
    );
    // Rastreio do lote (recall): a gerência vê Vera com nascimento; a leitura vai ao lastro.
    let patricia = amb.entrar("patricia.lemos").await;
    let (sr, _, r) = amb
        .pedir(amb.get(
            &format!(
                "/v1/estoque/lotes/{}/rastreio",
                m["lote_id"].as_str().unwrap()
            ),
            Some(&patricia),
        ))
        .await;
    assert_eq!(sr, StatusCode::OK, "{r}");
    assert_eq!(r["pacientes"][0]["paciente"]["nome"], VERA);
    assert_eq!(r["pacientes"][0]["paciente"]["nascimento"], NASC_VERA);
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'rastreio_de_lote_lido'")
            .await,
        1
    );
    let estoque = amb.entrar("estoque.a").await;
    let (s403, _, _) = amb
        .pedir(amb.get(
            &format!(
                "/v1/estoque/lotes/{}/rastreio",
                m["lote_id"].as_str().unwrap()
            ),
            Some(&estoque),
        ))
        .await;
    assert_eq!(
        s403,
        StatusCode::FORBIDDEN,
        "o estoque vê consumo sem identificar o paciente"
    );
}

#[tokio::test]
async fn duas_baixas_simultaneas_no_ultimo_frasco_so_uma_tem_efeito() {
    let amb = Ambiente::novo().await;
    let item = item_contraste(&amb).await;
    entrada(&amb, item, "L-UNICO", &daqui(100), 1).await;
    let atd = vera_liberada(&amb).await;
    let (a, b) = tokio::join!(
        aplicar(&amb, "livia.costa", &atd, item, 1, Uuid::now_v7()),
        aplicar(&amb, "livia.costa", &atd, item, 1, Uuid::now_v7()),
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(
        st,
        [StatusCode::CREATED, StatusCode::CONFLICT],
        "{a:?} {b:?}"
    );
    let saldo: i32 = sqlx::query_scalar("SELECT saldo FROM estoque.lote WHERE numero = 'L-UNICO'")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    assert_eq!(saldo, 0, "nunca negativo");
}

#[tokio::test]
async fn sem_triagem_vencido_ou_outra_sala_nao_baixa() {
    let amb = Ambiente::novo().await;
    let item = item_contraste(&amb).await;
    entrada(&amb, item, "L-OK", &daqui(100), 3).await;
    let atd = amb.fazer_checkin(VERA, NASC_VERA).await;
    let id = atd["id"].as_str().unwrap();
    let (s1, e1) = aplicar(&amb, "livia.costa", id, item, 1, Uuid::now_v7()).await;
    assert_eq!(
        (s1, e1["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("sem-triagem")),
        "FC-TRI-06"
    );
    let id = vera_liberada_existente(&amb, id).await;
    let (s2, _) = aplicar(&amb, "bruno.sales", &id, item, 1, Uuid::now_v7()).await;
    assert_eq!(s2, StatusCode::FORBIDDEN, "técnico de outra sala");
    let (s3, _) = aplicar(&amb, "livia.costa", &id, item, 0, Uuid::now_v7()).await;
    assert_eq!(s3, StatusCode::UNPROCESSABLE_ENTITY, "quantidade zero");
    sqlx::query("UPDATE estoque.lote SET validade = current_date - 1 WHERE numero = 'L-OK'")
        .execute(&amb.admin)
        .await
        .unwrap();
    let (s4, e4) = aplicar(&amb, "livia.costa", &id, item, 1, Uuid::now_v7()).await;
    assert_eq!(
        (s4, e4["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("sem-lote-valido")),
        "lote vencido não sai"
    );
}

async fn vera_liberada_existente(amb: &Ambiente, id: &str) -> String {
    let livia = amb.entrar("livia.costa").await;
    let hoje = vidacor_api::tempo::hoje_local();
    let (st, _, t) = amb
        .pedir(amb.json(Method::POST, &format!("/v1/atendimentos/{id}/triagem"), &livia, None, json!({
            "reacao_previa_contraste": false, "gestante": false, "amamentando": false, "usa_metformina": false,
            "creatinina_mg_dl": "0.7", "creatinina_data": (hoje - chrono::Duration::days(11)).to_string()
        })))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    id.to_owned()
}

#[tokio::test]
async fn movimento_e_imutavel_e_itens_mostram_lotes_na_ordem_de_saida() {
    let amb = Ambiente::novo().await;
    let item = item_contraste(&amb).await;
    entrada(&amb, item, "L-B", &daqui(400), 5).await;
    entrada(&amb, item, "L-A", &daqui(50), 2).await;
    let r = sqlx::query("UPDATE estoque.movimento SET quantidade = 99")
        .execute(&amb.admin)
        .await;
    assert!(r.is_err(), "movimento não se altera");
    let estoque = amb.entrar("estoque.a").await;
    let (st, _, itens) = amb
        .pedir(amb.get("/v1/estoque/itens", Some(&estoque)))
        .await;
    assert_eq!(st, StatusCode::OK);
    let lotes: Vec<&str> = itens[0]["lotes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["numero"].as_str().unwrap())
        .collect();
    assert_eq!(lotes, ["L-A", "L-B"]);
    assert_eq!(
        (
            itens[0]["saldo_valido"].as_i64(),
            itens[0]["abaixo_do_minimo"].as_bool()
        ),
        (Some(7), Some(true))
    );
    let carla = amb.entrar("carla.caixa").await;
    let (s403, _, _) = amb.pedir(amb.get("/v1/estoque/itens", Some(&carla))).await;
    assert_eq!(s403, StatusCode::FORBIDDEN);
}
