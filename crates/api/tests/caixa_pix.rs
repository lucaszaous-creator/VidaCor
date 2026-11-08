//! Caixa do particular e webhook PIX (docs/27 FC-CXA-01 a 09, FC-PIX-01 a 05). Rota real e
//! PostgreSQL real (G4); só o PSP é simulado (provedor externo).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use comum::{Ambiente, UNIDADE_A};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use uuid::Uuid;

const SONIA: &str = "Sônia Maria Castro";

/// Tabela de preços versão 12, vigente hoje: ecocardiograma R$ 380,00.
async fn preparar(amb: &Ambiente) {
    let tabela = Uuid::now_v7();
    sqlx::query("INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, origem, unidade_id, correlacao) VALUES ($1, 12, current_date - 1, 'Particular 2026', 'seed', $2, 'seed')")
        .bind(tabela).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO financeiro.preco (tabela_id, tuss, valor_centavos) VALUES ($1, '40901114', 38000), ($1, '40901300', 42000)")
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

fn recebimento(amb: &Ambiente, itens: Value, desconto: Value, forma: &str) -> Value {
    json!({ "paciente_id": amb.s.pacientes[SONIA], "atendimento_id": null, "itens": itens, "desconto": desconto, "forma": forma })
}

async fn receber(amb: &Ambiente, chave: Uuid, corpo: Value) -> (StatusCode, Value) {
    let carla = amb.entrar("carla.caixa").await;
    let (st, _, c) = amb
        .pedir(amb.json(Method::POST, "/v1/recebimentos", &carla, Some(chave), corpo))
        .await;
    (st, c)
}

async fn processar_outbox(amb: &Ambiente) {
    let orthanc = vidacor_api::orthanc::Orthanc::novo("http://127.0.0.1:9").unwrap();
    for _ in 0..20 {
        if !vidacor_api::outbox::processar_um(&amb.estado, &orthanc)
            .await
            .unwrap()
        {
            break;
        }
    }
}

#[tokio::test]
async fn fc_cxa_01_e_02_total_e_do_servidor_com_a_versao_da_tabela() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let (st, r) = receber(
        &amb,
        Uuid::now_v7(),
        recebimento(
            &amb,
            json!([{ "tuss": "40901114", "valor_centavos": 10000 }]),
            Value::Null,
            "dinheiro",
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    assert_eq!(
        r["total_centavos"], 38000,
        "o valor de R$ 100,00 enviado pela tela é ignorado"
    );
    assert_eq!(r["tabela_versao"], 12);
    assert_eq!(r["situacao"], "pago");
}

#[tokio::test]
async fn fc_cxa_03_desconto_dentro_do_limite_com_motivo() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let (st, r) = receber(
        &amb,
        Uuid::now_v7(),
        recebimento(
            &amb,
            json!([{ "tuss": "40901114" }]),
            json!({ "pontos_base": 500, "motivo": "paciente de longa data" }),
            "dinheiro",
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    assert_eq!(r["total_centavos"], 36100);
    let dados: Value =
        sqlx::query_scalar("SELECT dados FROM lastro.evento WHERE tipo = 'recebimento_registrado'")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(dados["desconto_motivo"], "paciente de longa data");
}

#[tokio::test]
async fn fc_cxa_04_desconto_acima_do_limite_sem_autorizacao_e_recusado() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let (st, r) = receber(
        &amb,
        Uuid::now_v7(),
        recebimento(
            &amb,
            json!([{ "tuss": "40901114" }]),
            json!({ "pontos_base": 1500, "motivo": "pedido do paciente" }),
            "dinheiro",
        ),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    assert_eq!(r["codigo"], "desconto-acima-do-limite");
    assert_eq!(r["limite_pontos_base"], 500);
    assert_eq!(
        amb.contar("SELECT count(*) FROM caixa.recebimento").await,
        0
    );
}

#[tokio::test]
async fn fc_cxa_05_gerente_autoriza_com_reautenticacao_e_o_lastro_guarda_os_dois() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let patricia = amb.entrar("patricia.lemos").await;
    let (sa, _, a) = amb
        .pedir(amb.json(Method::POST, "/v1/autorizacoes-desconto", &patricia, None, json!({
            "paciente_id": amb.s.pacientes[SONIA], "pontos_base": 1500, "motivo": "acordo da direção", "solicitante_id": amb.s.usuarios["carla.caixa"]
        })))
        .await;
    assert_eq!(sa, StatusCode::CREATED, "{a}");
    let (st, r) = receber(&amb, Uuid::now_v7(), recebimento(&amb, json!([{ "tuss": "40901114" }]), json!({ "pontos_base": 1500, "motivo": "acordo da direção", "autorizacao_id": a["id"] }), "dinheiro")).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    assert_eq!(r["total_centavos"], 32300);
    let dados: Value =
        sqlx::query_scalar("SELECT dados FROM lastro.evento WHERE tipo = 'desconto_autorizado'")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(dados["solicitante"], json!(amb.s.usuarios["carla.caixa"]));
    assert_eq!(
        dados["autorizado_por"],
        json!(amb.s.usuarios["patricia.lemos"])
    );
    // A autorização é de uso único.
    let (s2, _) = receber(&amb, Uuid::now_v7(), recebimento(&amb, json!([{ "tuss": "40901114" }]), json!({ "pontos_base": 1500, "motivo": "acordo da direção", "autorizacao_id": a["id"] }), "dinheiro")).await;
    assert_eq!(s2, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn autorizacao_sem_reautenticacao_e_recusada() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let patricia = amb.entrar("patricia.lemos").await;
    amb.expirar_reautenticacao().await;
    let (st, _, c) = amb
        .pedir(amb.json(Method::POST, "/v1/autorizacoes-desconto", &patricia, None, json!({
            "paciente_id": amb.s.pacientes[SONIA], "pontos_base": 1500, "motivo": "acordo da direção", "solicitante_id": amb.s.usuarios["carla.caixa"]
        })))
        .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    assert_eq!(c["codigo"], "reautenticacao-necessaria");
}

#[tokio::test]
async fn fc_cxa_06_recebimento_repetido_gera_um_recebimento_e_uma_cobranca_pix() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let chave = Uuid::now_v7();
    let corpo = recebimento(&amb, json!([{ "tuss": "40901114" }]), Value::Null, "pix");
    let ((s1, r1), (s2, r2)) = tokio::join!(
        receber(&amb, chave, corpo.clone()),
        receber(&amb, chave, corpo)
    );
    assert_eq!((s1, s2), (StatusCode::CREATED, StatusCode::CREATED));
    assert_eq!(r1, r2);
    processar_outbox(&amb).await;
    processar_outbox(&amb).await;
    assert_eq!(
        amb.contar("SELECT count(*) FROM caixa.recebimento").await,
        1
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM caixa.recebimento WHERE pix_txid IS NOT NULL")
            .await,
        1
    );
    assert_eq!(
        amb.psp
            .chamadas_de_criacao
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}

#[tokio::test]
async fn fc_cxa_07_valor_zero_ou_negativo_e_recusado() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    for v in [0, -100] {
        let (st, r) = receber(
            &amb,
            Uuid::now_v7(),
            recebimento(
                &amb,
                json!([{ "tuss": "40901114", "valor_centavos": v }]),
                Value::Null,
                "dinheiro",
            ),
        )
        .await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(r["codigo"], "valor-invalido");
    }
}

#[tokio::test]
async fn fc_cxa_08_sem_caixa_aberto_e_recusado_e_o_fechamento_mostra_a_diferenca() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    receber(
        &amb,
        Uuid::now_v7(),
        recebimento(
            &amb,
            json!([{ "tuss": "40901114" }]),
            Value::Null,
            "dinheiro",
        ),
    )
    .await;
    let carla = amb.entrar("carla.caixa").await;
    let (_, _, meu) = amb.pedir(amb.get("/v1/caixas/meu", Some(&carla))).await;
    let (sf, _, f) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/caixas/{}/fechamento", meu["id"].as_str().unwrap()),
            &carla,
            None,
            json!({ "contado_centavos": 57000 }),
        ))
        .await;
    assert_eq!(sf, StatusCode::OK, "{f}");
    assert_eq!(f["esperado_centavos"], 58000);
    assert_eq!(
        f["diferenca_centavos"], -1000,
        "diferença de R$ 10,00 não é escondida"
    );
    let (st, r) = receber(
        &amb,
        Uuid::now_v7(),
        recebimento(
            &amb,
            json!([{ "tuss": "40901114" }]),
            Value::Null,
            "dinheiro",
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(r["codigo"], "caixa-fechado");
}

#[tokio::test]
async fn fc_cxa_09_quem_pede_o_estorno_nao_aprova() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let (_, r) = receber(
        &amb,
        Uuid::now_v7(),
        recebimento(
            &amb,
            json!([{ "tuss": "40901114" }]),
            Value::Null,
            "dinheiro",
        ),
    )
    .await;
    let carla = amb.entrar("carla.caixa").await;
    let (sp, _, e) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/recebimentos/{}/estornos", r["id"].as_str().unwrap()),
            &carla,
            None,
            json!({ "motivo": "cobrado em duplicidade" }),
        ))
        .await;
    assert_eq!(sp, StatusCode::CREATED, "{e}");
    let uri = format!("/v1/estornos/{}/aprovacao", e["id"].as_str().unwrap());
    let (s1, _, _) = amb
        .pedir(amb.json(Method::POST, &uri, &carla, None, json!({})))
        .await;
    assert_eq!(s1, StatusCode::FORBIDDEN);
    let fin = amb.entrar("financeiro.a").await;
    let (s2, _, a) = amb
        .pedir(amb.json(Method::POST, &uri, &fin, None, json!({})))
        .await;
    assert_eq!(s2, StatusCode::OK, "{a}");
    assert_eq!(
        amb.contar("SELECT count(*) FROM caixa.recebimento WHERE situacao = 'estornado'")
            .await,
        1
    );
}

#[tokio::test]
async fn gerente_que_pediu_o_estorno_tambem_nao_aprova() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let (_, r) = receber(
        &amb,
        Uuid::now_v7(),
        recebimento(
            &amb,
            json!([{ "tuss": "40901114" }]),
            Value::Null,
            "dinheiro",
        ),
    )
    .await;
    let patricia = amb.entrar("patricia.lemos").await;
    let (_, _, e) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/recebimentos/{}/estornos", r["id"].as_str().unwrap()),
            &patricia,
            None,
            json!({ "motivo": "cobrado em duplicidade" }),
        ))
        .await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/estornos/{}/aprovacao", e["id"].as_str().unwrap()),
            &patricia,
            None,
            json!({}),
        ))
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    assert_eq!(c["codigo"], "separacao-de-funcoes");
}

fn webhook(corpo: &str, assinatura: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method(Method::POST)
        .uri("/v1/webhooks/pix")
        .header("content-type", "application/json");
    let sig = assinatura.map(str::to_owned).unwrap_or_else(|| {
        let mut mac = Hmac::<Sha256>::new_from_slice(comum::SEGREDO_PIX.as_bytes()).unwrap();
        mac.update(corpo.as_bytes());
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    });
    b = b.header("x-assinatura", sig);
    b.body(Body::from(corpo.to_owned())).unwrap()
}

async fn cobranca_paga(amb: &Ambiente) -> String {
    preparar(amb).await;
    receber(
        amb,
        Uuid::now_v7(),
        recebimento(amb, json!([{ "tuss": "40901114" }]), Value::Null, "pix"),
    )
    .await;
    processar_outbox(amb).await;
    let txid: String = sqlx::query_scalar("SELECT pix_txid FROM caixa.recebimento")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    amb.psp.marcar_pago(&txid);
    txid
}

#[tokio::test]
async fn fc_pix_01_e_02_pagamento_baixa_uma_vez_e_o_repetido_para() {
    let amb = Ambiente::novo().await;
    let txid = cobranca_paga(&amb).await;
    let corpo = json!({ "id": "evt-001", "tipo": "pix.pago", "txid": txid }).to_string();
    let (s1, _, c1) = amb.pedir(webhook(&corpo, None)).await;
    assert_eq!(s1, StatusCode::OK, "{c1}");
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.evento_recebido")
            .await,
        1
    );
    processar_outbox(&amb).await;
    let (situacao, e2e): (String, Option<String>) =
        sqlx::query_as("SELECT situacao, pix_end_to_end FROM caixa.recebimento")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(situacao, "pago");
    assert!(e2e.is_some());
    let (s2, _, c2) = amb.pedir(webhook(&corpo, None)).await;
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(c2["situacao"], "ja_processado");
    processar_outbox(&amb).await;
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox WHERE tipo = 'nfse.emitir'")
            .await,
        1
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'pix_pago'")
            .await,
        1
    );
}

#[tokio::test]
async fn fc_pix_03_assinatura_invalida_e_recusada_antes_de_processar() {
    let amb = Ambiente::novo().await;
    let txid = cobranca_paga(&amb).await;
    let corpo = json!({ "id": "evt-002", "tipo": "pix.pago", "txid": txid }).to_string();
    let (st, _, c) = amb.pedir(webhook(&corpo, Some("00ff00ff"))).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    assert_eq!(c["codigo"], "assinatura-invalida");
    // Corpo alterado depois de assinado também é recusado.
    let mut mac = Hmac::<Sha256>::new_from_slice(comum::SEGREDO_PIX.as_bytes()).unwrap();
    mac.update(corpo.as_bytes());
    let sig: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let (st2, _, _) = amb
        .pedir(webhook(&corpo.replace("evt-002", "evt-003"), Some(&sig)))
        .await;
    assert_eq!(st2, StatusCode::UNAUTHORIZED);
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.evento_recebido")
            .await,
        0
    );
    let situacao: String = sqlx::query_scalar("SELECT situacao FROM caixa.recebimento")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    assert_eq!(situacao, "aguardando_pagamento");
}

#[tokio::test]
async fn fc_pix_04_evento_antigo_depois_do_pago_nao_regride() {
    let amb = Ambiente::novo().await;
    let txid = cobranca_paga(&amb).await;
    amb.pedir(webhook(
        &json!({ "id": "evt-pago", "tipo": "pix.pago", "txid": txid }).to_string(),
        None,
    ))
    .await;
    processar_outbox(&amb).await;
    let (st, _, _) = amb
        .pedir(webhook(
            &json!({ "id": "evt-criada", "tipo": "cobranca.criada", "txid": txid }).to_string(),
            None,
        ))
        .await;
    assert_eq!(st, StatusCode::OK);
    processar_outbox(&amb).await;
    let situacao: String = sqlx::query_scalar("SELECT situacao FROM caixa.recebimento")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    assert_eq!(situacao, "pago");
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox WHERE tipo = 'nfse.emitir'")
            .await,
        1
    );
}

#[tokio::test]
async fn fc_pix_05_dois_webhooks_iguais_ao_mesmo_tempo() {
    let amb = Ambiente::novo().await;
    let txid = cobranca_paga(&amb).await;
    let corpo = json!({ "id": "evt-010", "tipo": "pix.pago", "txid": txid }).to_string();
    let ((s1, _, _), (s2, _, _)) = tokio::join!(
        amb.pedir(webhook(&corpo, None)),
        amb.pedir(webhook(&corpo, None))
    );
    assert_eq!((s1, s2), (StatusCode::OK, StatusCode::OK));
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox WHERE tipo = 'pix.processar_evento'")
            .await,
        1
    );
    processar_outbox(&amb).await;
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'pix_pago'")
            .await,
        1
    );
}

#[tokio::test]
async fn caixa_e_financeiro_nao_veem_dado_clinico() {
    let amb = Ambiente::novo().await;
    for perfis in ["caixa", "financeiro"] {
        let mut tx = amb.app_pool.begin().await.unwrap();
        sqlx::query(
            "SELECT set_config('app.unidade_id', $1, true), set_config('app.perfis', $2, true)",
        )
        .bind(UNIDADE_A.to_string())
        .bind(perfis)
        .execute(&mut *tx)
        .await
        .unwrap();
        for tabela in [
            "triagem.triagem",
            "laudos.laudo",
            "laudos.versao",
            "exames.estudo",
            "agenda.agendamento",
        ] {
            let n: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM {tabela}"
            )))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
            assert_eq!(n, 0, "{perfis} não lê {tabela}");
        }
        tx.rollback().await.unwrap();
    }
}
