//! Financeiro essencial e repasse (docs/25 MVP; Financeiro.dc.html). Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::{Ambiente, UNIDADE_A};
use serde_json::{json, Value};
use uuid::Uuid;

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

async fn conta(amb: &Ambiente) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO financeiro.conta_bancaria (id, codigo, nome, saldo_inicial_centavos, origem, unidade_id, correlacao) VALUES ($1, 'CTA-1', 'Conta movimento', 100000, 'seed', $2, 'seed')")
        .bind(id).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    id
}

fn titulo(valor: i64) -> Value {
    json!({ "tipo": "pagar", "descricao": "Contraste iodado, NF 1203", "contraparte": "Distribuidora Fictícia", "categoria": "fornecedor", "valor_centavos": valor, "vencimento": "2026-11-20" })
}

#[tokio::test]
async fn acima_de_10_mil_duas_aprovacoes_de_outras_pessoas_e_baixa_unica() {
    let amb = Ambiente::novo().await;
    let c = conta(&amb).await;
    let chave = Uuid::now_v7();
    let (st, t) = post(
        &amb,
        "financeiro.a",
        "/v1/financeiro/titulos",
        Some(chave),
        titulo(1_500_000),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    assert_eq!(t["aprovacoes_exigidas"], 2);
    let (_, t2) = post(
        &amb,
        "financeiro.a",
        "/v1/financeiro/titulos",
        Some(chave),
        titulo(1_500_000),
    )
    .await;
    assert_eq!(t2["id"], t["id"], "repetir não duplica");
    let id = t["id"].as_str().unwrap();
    let (s0, e0) = post(
        &amb,
        "financeiro.a",
        &format!("/v1/financeiro/titulos/{id}/aprovacoes"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(
        (s0, e0["codigo"].as_str()),
        (StatusCode::FORBIDDEN, Some("separacao-de-funcoes"))
    );
    let (s1, _) = post(
        &amb,
        "patricia.lemos",
        &format!("/v1/financeiro/titulos/{id}/aprovacoes"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(s1, StatusCode::OK);
    let (s2, e2) = post(
        &amb,
        "patricia.lemos",
        &format!("/v1/financeiro/titulos/{id}/aprovacoes"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(
        (s2, e2["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("ja-aprovado-por-voce"))
    );
    let baixa = json!({ "conta_id": c, "data": "2026-11-20" });
    let (s3, e3) = post(
        &amb,
        "financeiro.a",
        &format!("/v1/financeiro/titulos/{id}/baixa"),
        None,
        baixa.clone(),
    )
    .await;
    assert_eq!(
        (s3, e3["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("aprovacao-pendente"))
    );
    sqlx::query("INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao) VALUES ($1, 'USR-fin2', 'financeiro.b', 'Outra do Financeiro', ARRAY['financeiro'], 'seed', $2, 'seed')")
        .bind(Uuid::now_v7()).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    assert_eq!(
        post(
            &amb,
            "financeiro.b",
            &format!("/v1/financeiro/titulos/{id}/aprovacoes"),
            None,
            json!({})
        )
        .await
        .0,
        StatusCode::OK
    );
    let uri_baixa = format!("/v1/financeiro/titulos/{id}/baixa");
    let (a, b) = tokio::join!(
        post(&amb, "financeiro.a", &uri_baixa, None, baixa.clone()),
        post(&amb, "financeiro.a", &uri_baixa, None, baixa.clone()),
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(
        st,
        [StatusCode::OK, StatusCode::CONFLICT],
        "duas baixas ao mesmo tempo: uma vale"
    );
    assert_eq!(amb.contar("SELECT count(*) FROM financeiro.baixa").await, 1);
}

#[tokio::test]
async fn extrato_importado_nao_duplica_e_conciliacao_confere_valor_e_sinal() {
    let amb = Ambiente::novo().await;
    let c = conta(&amb).await;
    let linhas = json!({ "conta_id": c, "linhas": [
        { "id_banco": "B-1", "data": "2026-11-05", "valor_centavos": 9_621_000, "descricao": "TED UNIMED LOTE 0148" },
        { "id_banco": "B-2", "data": "2026-11-06", "valor_centavos": -50_000, "descricao": "PAGTO FORNECEDOR" } ] });
    let (_, i1) = post(
        &amb,
        "financeiro.a",
        "/v1/financeiro/extratos",
        None,
        linhas.clone(),
    )
    .await;
    let (_, i2) = post(
        &amb,
        "financeiro.a",
        "/v1/financeiro/extratos",
        None,
        linhas,
    )
    .await;
    assert_eq!(
        (
            i1["novas"].as_i64(),
            i2["novas"].as_i64(),
            i2["repetidas"].as_i64()
        ),
        (Some(2), Some(0), Some(2))
    );
    let (_, t) = post(&amb, "financeiro.a", "/v1/financeiro/titulos", Some(Uuid::now_v7()), json!({ "tipo": "receber", "descricao": "Lote 0148", "contraparte": "Unimed", "categoria": "operadora", "valor_centavos": 9_621_000, "vencimento": "2026-11-14" })).await;
    assert_eq!(
        t["aprovacoes_exigidas"], 0,
        "a receber não precisa de aprovação"
    );
    let id = t["id"].as_str().unwrap();
    let saida: Uuid =
        sqlx::query_scalar("SELECT id FROM financeiro.extrato_linha WHERE id_banco = 'B-2'")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    let entrada: Uuid =
        sqlx::query_scalar("SELECT id FROM financeiro.extrato_linha WHERE id_banco = 'B-1'")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    let (s1, e1) = post(
        &amb,
        "financeiro.a",
        &format!("/v1/financeiro/titulos/{id}/baixa"),
        None,
        json!({ "conta_id": c, "data": "2026-11-05", "extrato_linha_id": saida }),
    )
    .await;
    assert_eq!(
        (s1, e1["codigo"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("extrato-nao-confere")
        )
    );
    let (s2, t2) = post(
        &amb,
        "financeiro.a",
        &format!("/v1/financeiro/titulos/{id}/baixa"),
        None,
        json!({ "conta_id": c, "data": "2026-11-05", "extrato_linha_id": entrada }),
    )
    .await;
    assert_eq!(
        (s2, t2["situacao"].as_str()),
        (StatusCode::OK, Some("pago"))
    );
    let fin = amb.entrar("financeiro.a").await;
    let (_, _, r) = amb
        .pedir(amb.get("/v1/financeiro/resumo", Some(&fin)))
        .await;
    assert_eq!(r["saldo_em_contas_centavos"], 100_000 + 9_621_000 - 50_000);
    assert_eq!(r["extrato_sem_conciliacao"], 1);
}

#[tokio::test]
async fn repasse_pela_regra_vigente_gera_um_titulo_por_medico() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb
        .ate_o_laudo("Antônio Carlos Ribeiro", "1954-06-15")
        .await;
    let tabela = Uuid::now_v7();
    sqlx::query("INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, origem, unidade_id, correlacao) VALUES ($1, 1, '2020-01-01', 'Tabela', 'seed', $2, 'seed')")
        .bind(tabela).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO financeiro.preco (tabela_id, tuss, valor_centavos) SELECT $1, exame_tuss, 40000 FROM agenda.agendamento GROUP BY exame_tuss")
        .bind(tabela).execute(&amb.admin).await.unwrap();
    // Assinatura no mês passado, para a competência estar fechada.
    let s = amb.entrar("medico.a").await;
    amb.pedir(amb.json(
        Method::PUT,
        &format!("/v1/laudos/{laudo}/rascunho"),
        &s,
        None,
        json!({ "texto": "Normal.", "versao_numero": 1 }),
    ))
    .await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/laudos/{laudo}/assinatura"),
            &s,
            Some(Uuid::now_v7()),
            json!({ "pin": "4821" }),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    sqlx::query("ALTER TABLE laudos.versao DISABLE TRIGGER USER")
        .execute(&amb.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE laudos.versao SET assinada_em = date_trunc('month', now()) - interval '10 days' WHERE laudo_id = $1").bind(laudo).execute(&amb.admin).await.unwrap();
    sqlx::query("ALTER TABLE laudos.versao ENABLE TRIGGER USER")
        .execute(&amb.admin)
        .await
        .unwrap();
    let competencia: String = sqlx::query_scalar(
        "SELECT to_char(date_trunc('month', now()) - interval '10 days', 'YYYY-MM')",
    )
    .fetch_one(&amb.admin)
    .await
    .unwrap();
    let medico = amb.s.usuarios["medico.a"];
    let (s0, g0) = post(
        &amb,
        "financeiro.a",
        &format!("/v1/financeiro/repasse/{competencia}/titulos"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(s0, StatusCode::OK);
    assert_eq!(
        g0["sem_regra"],
        json!(["Dr. Paulo Medeiros"]),
        "sem regra, nada é calculado por suposição"
    );
    let (sr, r) = post(
        &amb,
        "patricia.lemos",
        "/v1/financeiro/regras-repasse",
        None,
        json!({ "medico_id": medico, "percentual_pb": 3000, "vigente_desde": "2020-01-01" }),
    )
    .await;
    assert_eq!((sr, r["versao"].as_i64()), (StatusCode::CREATED, Some(1)));
    let fin = amb.entrar("financeiro.a").await;
    let (_, _, p) = amb
        .pedir(amb.get(
            &format!("/v1/financeiro/repasse?competencia={competencia}"),
            Some(&fin),
        ))
        .await;
    assert_eq!(
        (
            p[0]["laudos"].as_i64(),
            p[0]["base_centavos"].as_i64(),
            p[0]["valor_centavos"].as_i64()
        ),
        (Some(1), Some(40000), Some(12000))
    );
    let (_, g1) = post(
        &amb,
        "financeiro.a",
        &format!("/v1/financeiro/repasse/{competencia}/titulos"),
        None,
        json!({}),
    )
    .await;
    let (_, g2) = post(
        &amb,
        "financeiro.a",
        &format!("/v1/financeiro/repasse/{competencia}/titulos"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(
        (
            g1["titulos_criados"].as_i64(),
            g2["titulos_criados"].as_i64(),
            g2["ja_existiam"].as_i64()
        ),
        (Some(1), Some(0), Some(1))
    );
}

#[tokio::test]
async fn perfis_sem_acesso_e_limites() {
    let amb = Ambiente::novo().await;
    for login in ["ana.souza", "carla.caixa", "medico.a", "ti.a"] {
        let s = amb.entrar(login).await;
        let (st, _, _) = amb.pedir(amb.get("/v1/financeiro/titulos", Some(&s))).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    assert_eq!(
        post(
            &amb,
            "financeiro.a",
            "/v1/financeiro/titulos",
            Some(Uuid::now_v7()),
            titulo(0)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post(
            &amb,
            "financeiro.a",
            "/v1/financeiro/titulos",
            Some(Uuid::now_v7()),
            titulo(-5)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post(
            &amb,
            "financeiro.a",
            "/v1/financeiro/repasse/2026-13/titulos",
            None,
            json!({})
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mes: String = sqlx::query_scalar("SELECT to_char(now(), 'YYYY-MM')")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    assert_eq!(
        post(
            &amb,
            "financeiro.a",
            &format!("/v1/financeiro/repasse/{mes}/titulos"),
            None,
            json!({})
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY,
        "competência aberta"
    );
}
