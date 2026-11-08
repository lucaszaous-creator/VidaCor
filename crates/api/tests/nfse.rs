//! NFS-e do recebimento (docs/25 MVP): uma nota por recebimento, cancelada no estorno aprovado,
//! descrição sem o exame. Rota real e PostgreSQL real; só a prefeitura é simulada (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::{Ambiente, UNIDADE_A};
use serde_json::{json, Value};
use uuid::Uuid;

const SONIA: &str = "Sônia Maria Castro";

async fn processar_outbox(amb: &Ambiente) {
    let orthanc = vidacor_api::orthanc::Orthanc::novo("http://127.0.0.1:9").unwrap();
    while vidacor_api::outbox::processar_um(&amb.estado, &orthanc)
        .await
        .unwrap()
    {}
}

async fn recebimento_em_dinheiro(amb: &Ambiente) -> Value {
    let tabela = Uuid::now_v7();
    sqlx::query("INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, origem, unidade_id, correlacao) VALUES ($1, 12, current_date - 1, 'Particular', 'seed', $2, 'seed')")
        .bind(tabela).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO financeiro.preco (tabela_id, tuss, valor_centavos) VALUES ($1, '40901114', 38000)").bind(tabela).execute(&amb.admin).await.unwrap();
    let carla = amb.entrar("carla.caixa").await;
    amb.pedir(amb.json(
        Method::POST,
        "/v1/caixas",
        &carla,
        None,
        json!({ "fundo_centavos": 20000 }),
    ))
    .await;
    let (st, _, r) = amb
        .pedir(amb.json(Method::POST, "/v1/recebimentos", &carla, Some(Uuid::now_v7()), json!({
            "paciente_id": amb.s.pacientes[SONIA], "atendimento_id": null, "itens": [{ "tuss": "40901114" }], "desconto": null, "forma": "dinheiro" })))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    r
}

#[tokio::test]
async fn recebimento_pago_gera_uma_nota_e_o_estorno_cancela() {
    let amb = Ambiente::novo().await;
    let r = recebimento_em_dinheiro(&amb).await;
    let id = r["id"].as_str().unwrap();
    let carla = amb.entrar("carla.caixa").await;
    let (s404, _, _) = amb
        .pedir(amb.get(&format!("/v1/recebimentos/{id}/nota"), Some(&carla)))
        .await;
    assert_eq!(s404, StatusCode::NOT_FOUND, "antes do outbox");
    processar_outbox(&amb).await;
    processar_outbox(&amb).await;
    let (st, _, n) = amb
        .pedir(amb.get(&format!("/v1/recebimentos/{id}/nota"), Some(&carla)))
        .await;
    assert_eq!(
        (st, n["situacao"].as_str(), n["valor_centavos"].as_i64()),
        (StatusCode::OK, Some("emitida"), Some(38000)),
        "{n}"
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM caixa.nota_fiscal").await,
        1,
        "uma nota por recebimento"
    );
    // Estorno pedido pelo caixa e aprovado pelo financeiro cancela a nota.
    let (_, _, e) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/recebimentos/{id}/estornos"),
            &carla,
            None,
            json!({ "motivo": "cobrado em duplicidade" }),
        ))
        .await;
    let fin = amb.entrar("financeiro.a").await;
    let (sa, _, a) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/estornos/{}/aprovacao", e["id"].as_str().unwrap()),
            &fin,
            None,
            json!({}),
        ))
        .await;
    assert_eq!(sa, StatusCode::OK, "{a}");
    processar_outbox(&amb).await;
    let (_, _, n2) = amb
        .pedir(amb.get(&format!("/v1/recebimentos/{id}/nota"), Some(&carla)))
        .await;
    assert_eq!(n2["situacao"], "cancelada");
    assert_eq!(amb.prefeitura.canceladas.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn sem_prefeitura_a_emissao_fica_pendente_e_recepcao_nao_ve_a_nota() {
    let amb = Ambiente::novo().await;
    let r = recebimento_em_dinheiro(&amb).await;
    let mut estado = amb.estado.clone();
    estado.prefeitura = None;
    let orthanc = vidacor_api::orthanc::Orthanc::novo("http://127.0.0.1:9").unwrap();
    while vidacor_api::outbox::processar_um(&estado, &orthanc)
        .await
        .unwrap()
    {}
    assert_eq!(
        amb.contar("SELECT count(*) FROM caixa.nota_fiscal").await,
        0
    );
    assert_eq!(amb.contar("SELECT count(*) FROM integracao.outbox WHERE tipo = 'nfse.emitir' AND concluido_em IS NULL AND ultimo_erro LIKE '%prefeitura%'").await, 1, "pendente e visível (R7)");
    let ana = amb.entrar("ana.souza").await;
    let (st, _, _) = amb
        .pedir(amb.get(
            &format!("/v1/recebimentos/{}/nota", r["id"].as_str().unwrap()),
            Some(&ana),
        ))
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}
