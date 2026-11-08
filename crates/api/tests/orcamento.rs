//! Orçamento do particular (Orcamento.dc.html): preço e pacote do servidor (G1), desconto com limite
//! e motivo, repetição sem duplicar (G3), cancelamento atômico com motivo (G2) e acesso indevido.
//! Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::{Ambiente, UNIDADE_A};
use serde_json::{json, Value};
use uuid::Uuid;

const MARIA: &str = "Maria Helena Duarte";

async fn com_precos(amb: &Ambiente) {
    let tabela = Uuid::now_v7();
    sqlx::query("INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, origem, unidade_id, correlacao) VALUES ($1, 12, '2020-01-01', 'Particular', 'seed', $2, 'seed')")
        .bind(tabela).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO financeiro.preco (tabela_id, tuss, valor_centavos) VALUES ($1, '40901114', 30000), ($1, '40901300', 30000), ($1, '40901122', 18000)")
        .bind(tabela).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO financeiro.pacote (id, codigo, nome, regra_versao, tuss, valor_centavos, vigente_desde, origem, unidade_id, correlacao)
                 VALUES ($1, 'PAC-CARDIO', 'Pacote Cardio', 3, ARRAY['40901114','40901300'], 54000, '2020-01-01', 'seed', $2, 'seed')")
        .bind(Uuid::now_v7()).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
}

async fn emitir(amb: &Ambiente, login: &str, chave: Uuid, corpo: Value) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, "/v1/orcamentos", &s, Some(chave), corpo))
        .await;
    (st, v)
}

fn pedido(amb: &Ambiente, extra: Value) -> Value {
    let mut b = json!({ "paciente_id": amb.s.pacientes[MARIA], "itens": ["40901114", "40901300"], "pagamento_previsto": "pix" });
    for (k, v) in extra.as_object().unwrap() {
        b[k] = v.clone();
    }
    b
}

#[tokio::test]
async fn servidor_calcula_pacote_e_desconto_e_ignora_valor_vindo_da_tela() {
    let amb = Ambiente::novo().await;
    com_precos(&amb).await;
    let chave = Uuid::now_v7();
    // A tela tenta mandar total e preço: são ignorados.
    let corpo = pedido(
        &amb,
        json!({ "pacote": "PAC-CARDIO", "desconto": { "pontos_base": 500, "motivo": "paciente_frequente" }, "total_centavos": 1, "valor_centavos": 1 }),
    );
    let (st, o) = emitir(&amb, "ana.souza", chave, corpo.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{o}");
    assert_eq!(
        (
            o["subtotal_centavos"].as_i64(),
            o["pacote_centavos"].as_i64(),
            o["desconto_centavos"].as_i64(),
            o["total_centavos"].as_i64()
        ),
        (Some(60_000), Some(6_000), Some(2_700), Some(51_300))
    );
    assert_eq!(
        (
            o["pacote_regra_versao"].as_i64(),
            o["tabela_versao"].as_i64(),
            o["situacao"].as_str()
        ),
        (Some(3), Some(12), Some("emitido"))
    );
    // Clique duplo: mesma chave, mesma resposta, um orçamento só.
    let (_, o2) = emitir(&amb, "ana.souza", chave, corpo).await;
    assert_eq!(o2["id"], o["id"]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM financeiro.orcamento")
            .await,
        1
    );
    // Emitido não se altera nem se apaga.
    assert!(sqlx::query(
        "UPDATE financeiro.orcamento SET total_centavos = 1, subtotal_centavos = 7"
    )
    .execute(&amb.admin)
    .await
    .is_err());
    assert!(sqlx::query("DELETE FROM financeiro.orcamento")
        .execute(&amb.admin)
        .await
        .is_err());
    assert!(
        sqlx::query("UPDATE financeiro.orcamento_item SET valor_centavos = 1")
            .execute(&amb.admin)
            .await
            .is_err()
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'orcamento_emitido'")
            .await,
        1
    );
}

#[tokio::test]
async fn limites_e_erros_esperados_nao_emitem_nada() {
    let amb = Ambiente::novo().await;
    com_precos(&amb).await;
    let casos = [
        (
            json!({ "desconto": { "pontos_base": 1001, "motivo": "paciente_frequente" } }),
            StatusCode::FORBIDDEN,
            "desconto-acima-do-limite",
        ),
        (
            json!({ "desconto": { "pontos_base": 500, "motivo": "porque sim" } }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "motivo-invalido",
        ),
        (
            json!({ "desconto": { "pontos_base": 0, "motivo": "paciente_frequente" } }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "desconto-invalido",
        ),
        (
            json!({ "desconto": { "pontos_base": -10, "motivo": "paciente_frequente" } }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "desconto-invalido",
        ),
        (
            json!({ "itens": [] }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "sem-exames",
        ),
        (
            json!({ "itens": ["40901114", "40901114"] }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "exame-repetido",
        ),
        (
            json!({ "itens": ["40901114"], "pacote": "PAC-CARDIO" }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "pacote-nao-cabe",
        ),
        (
            json!({ "pacote": "PAC-INEXISTENTE" }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "pacote-nao-cabe",
        ),
        (
            json!({ "itens": ["40801012"] }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "exame-sem-preco",
        ),
        (
            json!({ "pagamento_previsto": "cheque" }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "forma-invalida",
        ),
        (
            json!({ "paciente_id": Uuid::now_v7() }),
            StatusCode::NOT_FOUND,
            "paciente-nao-encontrado",
        ),
    ];
    for (extra, status, codigo) in casos {
        let (st, v) = emitir(
            &amb,
            "ana.souza",
            Uuid::now_v7(),
            pedido(&amb, extra.clone()),
        )
        .await;
        assert_eq!(
            (st, v["codigo"].as_str()),
            (status, Some(codigo)),
            "{extra} {v}"
        );
    }
    // Exatamente no limite (10%) vale.
    let (st, o) = emitir(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        pedido(
            &amb,
            json!({ "desconto": { "pontos_base": 1000, "motivo": "condicao_social" } }),
        ),
    )
    .await;
    assert_eq!(
        (st, o["total_centavos"].as_i64()),
        (StatusCode::CREATED, Some(54_000))
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM financeiro.orcamento")
            .await,
        1
    );
}

#[tokio::test]
async fn cancelamento_com_motivo_e_atomico_e_dois_ao_mesmo_tempo_so_um_vale() {
    let amb = Ambiente::novo().await;
    com_precos(&amb).await;
    let (_, o) = emitir(&amb, "ana.souza", Uuid::now_v7(), pedido(&amb, json!({}))).await;
    let id = o["id"].as_str().unwrap().to_owned();
    let s = amb.entrar("ana.souza").await;
    let uri = format!("/v1/orcamentos/{id}/cancelamento");
    let (st, v, _) = {
        let (a, _, b) = amb
            .pedir(amb.json(Method::POST, &uri, &s, None, json!({ "motivo": "x" })))
            .await;
        (a, b, ())
    };
    assert_eq!(
        (st, v["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("motivo-invalido"))
    );
    let corpo = json!({ "motivo": "paciente vai fazer pelo convênio" });
    let (r1, r2) = tokio::join!(
        amb.pedir(amb.json(Method::POST, &uri, &s, None, corpo.clone())),
        amb.pedir(amb.json(Method::POST, &uri, &s, None, corpo.clone()))
    );
    let mut st = [r1.0, r2.0];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    let ok = if r1.0 == StatusCode::OK { r1.2 } else { r2.2 };
    assert_eq!(
        (ok["situacao"].as_str(), ok["motivo_cancelamento"].as_str()),
        (Some("cancelado"), Some("paciente vai fazer pelo convênio"))
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'orcamento_cancelado'")
            .await,
        1
    );
}

#[tokio::test]
async fn so_recepcao_e_gerencia_e_cada_unidade_ve_os_seus() {
    let amb = Ambiente::novo().await;
    com_precos(&amb).await;
    let (_, o) = emitir(&amb, "ana.souza", Uuid::now_v7(), pedido(&amb, json!({}))).await;
    for login in [
        "carla.caixa",
        "medico.a",
        "financeiro.a",
        "ti.a",
        "tv.a",
        "bruno.sales",
    ] {
        let (st, _) = emitir(&amb, login, Uuid::now_v7(), pedido(&amb, json!({}))).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
        let s = amb.entrar(login).await;
        let (sl, _, _) = amb.pedir(amb.get("/v1/orcamentos", Some(&s))).await;
        assert_eq!(sl, StatusCode::FORBIDDEN, "{login}");
    }
    let g = amb.entrar("patricia.lemos").await;
    let (sg, _, lista) = amb.pedir(amb.get("/v1/orcamentos", Some(&g))).await;
    assert_eq!(
        (sg, lista.as_array().map(Vec::len)),
        (StatusCode::OK, Some(1))
    );
    // Recepção de outra unidade: não vê nem cancela (RLS).
    let b = amb.entrar("recepcao.b").await;
    let (_, _, lb) = amb.pedir(amb.get("/v1/orcamentos", Some(&b))).await;
    assert_eq!(lb.as_array().map(Vec::len), Some(0));
    let uri = format!("/v1/orcamentos/{}/cancelamento", o["id"].as_str().unwrap());
    let (sc, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &b,
            None,
            json!({ "motivo": "tentativa indevida" }),
        ))
        .await;
    assert_eq!(sc, StatusCode::NOT_FOUND);
    // Catálogo traz o preço do servidor e o limite.
    let s = amb.entrar("ana.souza").await;
    let (sk, _, cat) = amb
        .pedir(amb.get("/v1/orcamentos/catalogo", Some(&s)))
        .await;
    assert_eq!(
        (
            sk,
            cat["limite_desconto_pontos_base"].as_i64(),
            cat["pacotes"][0]["codigo"].as_str()
        ),
        (StatusCode::OK, Some(1000), Some("PAC-CARDIO"))
    );
}
