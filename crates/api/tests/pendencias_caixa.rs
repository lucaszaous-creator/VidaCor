//! Pendências da gerência no caixa: pedido de desconto acima do limite e recusa de estorno
//! (docs/27 FC-CXA-03, FC-CXA-05, FC-CXA-09; mockup Caixa). Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::{Ambiente, Sessao, UNIDADE_A};
use serde_json::{json, Value};
use uuid::Uuid;

const SONIA: &str = "Sônia Maria Castro";

async fn preparar(amb: &Ambiente) {
    let tabela = Uuid::now_v7();
    sqlx::query("INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, origem, unidade_id, correlacao) VALUES ($1, 12, current_date - 1, 'Particular 2026', 'seed', $2, 'seed')")
        .bind(tabela).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO financeiro.preco (tabela_id, tuss, valor_centavos) VALUES ($1, '40901114', 38000)")
        .bind(tabela).execute(&amb.admin).await.unwrap();
    let carla = amb.entrar("carla.caixa").await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/caixas",
            &carla,
            None,
            json!({ "fundo_centavos": 20000 }),
        ))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
}

async fn post(
    amb: &Ambiente,
    s: &Sessao,
    uri: &str,
    chave: Option<Uuid>,
    corpo: Value,
) -> (StatusCode, Value) {
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, uri, s, chave, corpo))
        .await;
    (st, v)
}

async fn pedir_desconto(amb: &Ambiente, chave: Uuid, pontos: i32) -> (StatusCode, Value) {
    let carla = amb.entrar("carla.caixa").await;
    post(amb, &carla, "/v1/pedidos-desconto", Some(chave),
        json!({ "paciente_id": amb.s.pacientes[SONIA], "pontos_base": pontos, "motivo": "tabela social" })).await
}

fn autorizacao(amb: &Ambiente, pedido: &Value) -> Value {
    json!({ "paciente_id": amb.s.pacientes[SONIA], "pontos_base": 1500, "motivo": "tabela social",
            "solicitante_id": amb.s.usuarios["carla.caixa"], "pedido_id": pedido["id"] })
}

#[tokio::test]
async fn caixa_pede_desconto_acima_do_limite_e_repetir_nao_duplica() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let chave = Uuid::now_v7();
    let (st, p) = pedir_desconto(&amb, chave, 1500).await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    assert_eq!(p["situacao"], "pedido");
    assert_eq!(p["paciente"]["nascimento"], "1962-02-03");
    let (st2, p2) = pedir_desconto(&amb, chave, 1500).await;
    assert_eq!(
        (st2, &p2["id"]),
        (StatusCode::CREATED, &p["id"]),
        "mesma chave, mesma resposta"
    );
    let (st3, e3) = pedir_desconto(&amb, Uuid::now_v7(), 1500).await;
    assert_eq!(st3, StatusCode::CONFLICT);
    assert_eq!(e3["codigo"], "pedido-ja-aberto");
    let (st4, e4) = pedir_desconto(&amb, Uuid::now_v7(), 500).await;
    assert_eq!(
        st4,
        StatusCode::UNPROCESSABLE_ENTITY,
        "dentro do limite não precisa de pedido"
    );
    assert_eq!(e4["limite_pontos_base"], 500);
    assert_eq!(
        amb.contar("SELECT count(*) FROM caixa.pedido_desconto")
            .await,
        1
    );
}

#[tokio::test]
async fn gerencia_ve_a_fila_e_outro_caixa_nao_ve_o_pedido_de_carla() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let (_, p) = pedir_desconto(&amb, Uuid::now_v7(), 1500).await;
    let patricia = amb.entrar("patricia.lemos").await;
    let (st, _, lista_g) = amb
        .pedir(amb.get("/v1/pedidos-desconto", Some(&patricia)))
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(lista_g[0]["id"], p["id"]);
    assert_eq!(lista_g[0]["solicitante"]["nome"], "Carla Mendes");
    let joao = amb.entrar("joao.caixa").await;
    let (_, _, lista_j) = amb
        .pedir(amb.get("/v1/pedidos-desconto", Some(&joao)))
        .await;
    assert_eq!(lista_j, json!([]), "RLS: caixa vê só os próprios pedidos");
    let (s404, _, _) = amb
        .pedir(amb.get(
            &format!("/v1/pedidos-desconto/{}", p["id"].as_str().unwrap()),
            Some(&joao),
        ))
        .await;
    assert_eq!(s404, StatusCode::NOT_FOUND, "ter o ID não dá acesso");
    let ana = amb.entrar("ana.souza").await;
    let (s403, _, _) = amb.pedir(amb.get("/v1/pedidos-desconto", Some(&ana))).await;
    assert_eq!(s403, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn autorizar_pelo_pedido_fecha_a_pendencia_e_o_caixa_usa_a_autorizacao() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let (_, p) = pedir_desconto(&amb, Uuid::now_v7(), 1500).await;
    let patricia = amb.entrar("patricia.lemos").await;
    let (sa, a) = post(
        &amb,
        &patricia,
        "/v1/autorizacoes-desconto",
        None,
        autorizacao(&amb, &p),
    )
    .await;
    assert_eq!(sa, StatusCode::CREATED, "{a}");
    let carla = amb.entrar("carla.caixa").await;
    let (_, _, visto) = amb
        .pedir(amb.get(
            &format!("/v1/pedidos-desconto/{}", p["id"].as_str().unwrap()),
            Some(&carla),
        ))
        .await;
    assert_eq!(visto["situacao"], "autorizado");
    assert_eq!(visto["autorizacao_id"], a["id"]);
    assert_eq!(visto["decidido_por"]["nome"], "Patrícia Lemos");
    let (sr, r) = post(&amb, &carla, "/v1/recebimentos", Some(Uuid::now_v7()), json!({
        "paciente_id": amb.s.pacientes[SONIA], "atendimento_id": null, "itens": [{ "tuss": "40901114" }],
        "desconto": { "pontos_base": 1500, "motivo": "tabela social", "autorizacao_id": a["id"] }, "forma": "dinheiro" })).await;
    assert_eq!(sr, StatusCode::CREATED, "{r}");
    assert_eq!(r["total_centavos"], 32300);
    let (s2, e2) = post(
        &amb,
        &patricia,
        "/v1/autorizacoes-desconto",
        None,
        autorizacao(&amb, &p),
    )
    .await;
    assert_eq!(s2, StatusCode::CONFLICT);
    assert_eq!(e2["codigo"], "pedido-ja-decidido");
    assert_eq!(
        amb.contar("SELECT count(*) FROM caixa.autorizacao_desconto")
            .await,
        1,
        "a segunda não ficou gravada"
    );
}

#[tokio::test]
async fn duas_autorizacoes_simultaneas_do_mesmo_pedido_so_uma_vale() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let (_, p) = pedir_desconto(&amb, Uuid::now_v7(), 1500).await;
    let patricia = amb.entrar("patricia.lemos").await;
    let (r1, r2) = tokio::join!(
        post(
            &amb,
            &patricia,
            "/v1/autorizacoes-desconto",
            None,
            autorizacao(&amb, &p)
        ),
        post(
            &amb,
            &patricia,
            "/v1/autorizacoes-desconto",
            None,
            autorizacao(&amb, &p)
        ),
    );
    let mut st = [r1.0, r2.0];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM caixa.autorizacao_desconto")
            .await,
        1
    );
}

#[tokio::test]
async fn recusa_tem_motivo_e_cancelamento_e_so_de_quem_pediu() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let (_, p) = pedir_desconto(&amb, Uuid::now_v7(), 1500).await;
    let id = p["id"].as_str().unwrap();
    let patricia = amb.entrar("patricia.lemos").await;
    let (s0, _) = post(
        &amb,
        &patricia,
        &format!("/v1/pedidos-desconto/{id}/recusa"),
        None,
        json!({ "motivo": "" }),
    )
    .await;
    assert_eq!(s0, StatusCode::UNPROCESSABLE_ENTITY);
    let (s1, _) = post(
        &amb,
        &patricia,
        &format!("/v1/pedidos-desconto/{id}/cancelamento"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(s1, StatusCode::FORBIDDEN, "gerente não é caixa");
    let (s2, r) = post(
        &amb,
        &patricia,
        &format!("/v1/pedidos-desconto/{id}/recusa"),
        None,
        json!({ "motivo": "fora da política social" }),
    )
    .await;
    assert_eq!(s2, StatusCode::OK, "{r}");
    assert_eq!(r["situacao"], "recusado");
    assert_eq!(r["motivo_decisao"], "fora da política social");
    let carla = amb.entrar("carla.caixa").await;
    let (s3, e3) = post(
        &amb,
        &carla,
        &format!("/v1/pedidos-desconto/{id}/cancelamento"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(s3, StatusCode::CONFLICT, "já decidido: {e3}");
    // Depois da recusa, Carla pode pedir de novo e cancelar o próprio pedido.
    let (_, p2) = pedir_desconto(&amb, Uuid::now_v7(), 1200).await;
    let (s4, c) = post(
        &amb,
        &carla,
        &format!(
            "/v1/pedidos-desconto/{}/cancelamento",
            p2["id"].as_str().unwrap()
        ),
        None,
        json!({}),
    )
    .await;
    assert_eq!(
        (s4, c["situacao"].as_str()),
        (StatusCode::OK, Some("cancelado"))
    );
    let tipos: Vec<String> = sqlx::query_scalar("SELECT tipo FROM lastro.evento WHERE dominio = 'caixa' AND tipo LIKE 'desconto%' ORDER BY id")
        .fetch_all(&amb.admin).await.unwrap();
    assert_eq!(
        tipos,
        [
            "desconto_pedido",
            "desconto_recusado",
            "desconto_pedido",
            "desconto_pedido_cancelado"
        ]
    );
}

async fn estorno_pedido(amb: &Ambiente) -> Value {
    let carla = amb.entrar("carla.caixa").await;
    let (_, r) = post(amb, &carla, "/v1/recebimentos", Some(Uuid::now_v7()), json!({
        "paciente_id": amb.s.pacientes[SONIA], "atendimento_id": null, "itens": [{ "tuss": "40901114" }],
        "desconto": null, "forma": "dinheiro" })).await;
    let (st, e) = post(
        amb,
        &carla,
        &format!("/v1/recebimentos/{}/estornos", r["id"].as_str().unwrap()),
        None,
        json!({ "motivo": "cobrado em duplicidade" }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{e}");
    e
}

#[tokio::test]
async fn estornos_pendentes_listados_e_recusa_permite_novo_pedido() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let e = estorno_pedido(&amb).await;
    let fin = amb.entrar("financeiro.a").await;
    let (_, _, lista) = amb.pedir(amb.get("/v1/estornos", Some(&fin))).await;
    assert_eq!(lista[0]["codigo"], e["codigo"]);
    assert_eq!(lista[0]["paciente"]["nome"], SONIA);
    assert_eq!(lista[0]["total_centavos"], 38000);
    assert_eq!(lista[0]["pedido_por"]["nome"], "Carla Mendes");
    let id = e["id"].as_str().unwrap();
    let (sr, r) = post(
        &amb,
        &fin,
        &format!("/v1/estornos/{id}/recusa"),
        None,
        json!({ "motivo": "pagamento único conferido no extrato" }),
    )
    .await;
    assert_eq!(sr, StatusCode::OK, "{r}");
    assert_eq!(r["situacao"], "recusado");
    let (sa, ea) = post(
        &amb,
        &fin,
        &format!("/v1/estornos/{id}/aprovacao"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(
        (sa, ea["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("estorno-recusado"))
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM caixa.recebimento WHERE situacao = 'pago'")
            .await,
        1
    );
    let carla = amb.entrar("carla.caixa").await;
    let (s2, _) = post(
        &amb,
        &carla,
        &format!(
            "/v1/recebimentos/{}/estornos",
            r["recebimento_id"].as_str().unwrap()
        ),
        None,
        json!({ "motivo": "paciente desistiu do exame" }),
    )
    .await;
    assert_eq!(
        s2,
        StatusCode::CREATED,
        "depois da recusa, pode pedir de novo"
    );
}

#[tokio::test]
async fn quem_pediu_o_estorno_nao_recusa_e_recepcao_nao_lista() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let e = estorno_pedido(&amb).await;
    let carla = amb.entrar("carla.caixa").await;
    let (s1, _) = post(
        &amb,
        &carla,
        &format!("/v1/estornos/{}/recusa", e["id"].as_str().unwrap()),
        None,
        json!({ "motivo": "não quero mais" }),
    )
    .await;
    assert_eq!(s1, StatusCode::FORBIDDEN);
    let ana = amb.entrar("ana.souza").await;
    let (s2, _, _) = amb.pedir(amb.get("/v1/estornos", Some(&ana))).await;
    assert_eq!(s2, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn aprovar_e_recusar_ao_mesmo_tempo_so_uma_decisao_vale() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let e = estorno_pedido(&amb).await;
    let id = e["id"].as_str().unwrap();
    let fin = amb.entrar("financeiro.a").await;
    let patricia = amb.entrar("patricia.lemos").await;
    let uri_a = format!("/v1/estornos/{id}/aprovacao");
    let uri_r = format!("/v1/estornos/{id}/recusa");
    let (a, r) = tokio::join!(
        post(&amb, &fin, &uri_a, None, json!({})),
        post(
            &amb,
            &patricia,
            &uri_r,
            None,
            json!({ "motivo": "conferir antes no extrato" })
        ),
    );
    let mut st = [a.0, r.0];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT], "{a:?} {r:?}");
    assert_eq!(
        amb.contar("SELECT count(*) FROM caixa.estorno WHERE situacao <> 'pedido'")
            .await,
        1
    );
}
