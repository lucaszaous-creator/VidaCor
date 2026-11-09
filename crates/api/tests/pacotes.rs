//! Check-up e pacotes (Checkup-Pacotes.dc.html; DEC-2026-11-09-115 a 117). Rota HTTP real e
//! PostgreSQL real (G4): caminho feliz, erros, repetição, duas requisições simultâneas, limites e
//! acesso indevido. Só dados fictícios.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use chrono::Duration;
use comum::{Ambiente, UNIDADE_A, UNIDADE_B};
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

async fn get(amb: &Ambiente, login: &str, uri: &str) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb.pedir(amb.get(uri, Some(&s))).await;
    (st, v)
}

const GER: &str = "patricia.lemos";

fn dia(amb: &Ambiente, dias: i64) -> String {
    (amb.dia + Duration::days(dias)).to_string()
}

/// Tabela do particular v12 vigente há 30 dias: carótidas R$ 320,00, eco R$ 380,00, estresse R$ 190,00.
async fn precos(amb: &Ambiente) {
    let t = Uuid::now_v7();
    sqlx::query("INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, origem, unidade_id, correlacao) VALUES ($1, 12, $2, 'Particular 2026', 'seed', $3, 'seed')")
        .bind(t).bind(amb.dia - Duration::days(30)).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO financeiro.preco (tabela_id, tuss, valor_centavos) VALUES ($1, '40901300', 32000), ($1, '40901114', 38000), ($1, '40901220', 19000)")
        .bind(t).execute(&amb.admin).await.unwrap();
}

fn exame(tuss: &str) -> Value {
    json!({ "tuss": tuss, "preparo": "Sem preparo", "porque": "Antes do esforço, em repouso." })
}

fn cardio(amb: &Ambiente, dias: i64, preco: i64) -> Value {
    json!({ "nome": "Check-up cardiológico", "para_quem": "Quem tem fator de risco ou mais de 40 anos", "vigente_desde": dia(amb, dias),
        "preco_centavos": preco, "motivo": "pacote novo da cardiologia",
        "exames": [exame("40901300"), exame("40901114"), exame("40901220")] })
}

async fn criar(amb: &Ambiente) -> String {
    let (st, p) = post(
        amb,
        GER,
        "/v1/pacotes",
        Some(Uuid::now_v7()),
        cardio(amb, 0, 69_000),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    p["codigo"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn quando_a_gerencia_cria_o_pacote_entao_o_servidor_calcula_o_avulso_e_o_orcamento_usa() {
    let amb = Ambiente::novo().await;
    precos(&amb).await;
    let chave = Uuid::now_v7();
    // A tela tenta mandar avulso e economia: são ignorados.
    let mut corpo = cardio(&amb, 0, 69_000);
    corpo["avulso_centavos"] = json!(1);
    let (st, p) = post(&amb, GER, "/v1/pacotes", Some(chave), corpo.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    let v = &p["versoes"][0];
    assert_eq!(
        (
            v["situacao"].as_str(),
            v["avulso_centavos"].as_i64(),
            v["economia_centavos"].as_i64(),
            v["desconto_pontos_base"].as_i64(),
            v["vale_no_orcamento"].as_bool()
        ),
        (
            Some("vigente"),
            Some(89_000),
            Some(20_000),
            Some(2_247),
            Some(true)
        )
    );
    let ordem: Vec<&str> = v["exames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["tuss"].as_str().unwrap())
        .collect();
    assert_eq!(ordem, vec!["40901300", "40901114", "40901220"]);
    assert_eq!(v["exames"][1]["preparo"], "Sem preparo");
    let (st, p2) = post(&amb, GER, "/v1/pacotes", Some(chave), corpo).await;
    assert_eq!((st, &p2), (StatusCode::CREATED, &p), "clique duplo");
    // A recepção consulta para orientar o paciente.
    let (st, c) = get(&amb, "ana.souza", "/v1/pacotes").await;
    assert_eq!(
        (
            st,
            c["pacotes"].as_array().unwrap().len(),
            c["desconto_maximo_pontos_base"].as_i64()
        ),
        (StatusCode::OK, 1, Some(4_000))
    );
    // O orçamento aplica o pacote calculado pelo servidor.
    let codigo = p["codigo"].as_str().unwrap();
    let corpo = json!({ "paciente_id": amb.s.pacientes["Maria Helena Duarte"], "itens": ["40901300", "40901114", "40901220"], "pacote": codigo, "pagamento_previsto": "pix" });
    let (st, o) = post(
        &amb,
        "ana.souza",
        "/v1/orcamentos",
        Some(Uuid::now_v7()),
        corpo,
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{o}");
    assert_eq!(
        (o["pacote_centavos"].as_i64(), o["total_centavos"].as_i64()),
        (Some(20_000), Some(69_000))
    );
}

#[tokio::test]
async fn quando_a_gerencia_publica_versao_nova_entao_a_vigente_nao_muda_ate_a_data() {
    let amb = Ambiente::novo().await;
    precos(&amb).await;
    let codigo = criar(&amb).await;
    let uri = format!("/v1/pacotes/{codigo}/versoes");
    let mut nova = cardio(&amb, 10, 72_000);
    nova["regra_versao_vista"] = json!(1);
    let (st, p) = post(&amb, GER, &uri, Some(Uuid::now_v7()), nova.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    let sit: Vec<(i64, &str)> = p["versoes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["regra_versao"].as_i64().unwrap(),
                v["situacao"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(sit, vec![(2, "futura"), (1, "vigente")]);
    // O orçamento de hoje continua com a versão 1.
    let v: i32 = sqlx::query_scalar("SELECT regra_versao FROM financeiro.pacote WHERE codigo = $1 AND vigente_desde <= $2 ORDER BY regra_versao DESC LIMIT 1")
        .bind(&codigo).bind(amb.dia).fetch_one(&amb.admin).await.unwrap();
    assert_eq!(v, 1);
    // Quem editou sobre a versão 1 chega tarde; versão nova não começa antes da última.
    let (st, e) = post(&amb, GER, &uri, Some(Uuid::now_v7()), nova.clone()).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("versao-mudou"))
    );
    for dias in [5, 10, 0, -1, 367] {
        let mut n = cardio(&amb, dias, 70_000);
        n["regra_versao_vista"] = json!(2);
        let (st, e) = post(&amb, GER, &uri, Some(Uuid::now_v7()), n).await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some("vigencia-invalida")),
            "{dias}"
        );
    }
    assert_eq!(
        post(
            &amb,
            GER,
            "/v1/pacotes/PCT-2026-999999/versoes",
            Some(Uuid::now_v7()),
            nova
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // Versão não se altera nem se apaga, nem pelo dono do banco.
    for sql in [
        "UPDATE financeiro.pacote SET valor_centavos = 1",
        "DELETE FROM financeiro.pacote",
        "UPDATE financeiro.pacote_item SET preparo = 'trocado'",
        "DELETE FROM financeiro.pacote_item",
    ] {
        assert!(sqlx::query(sql).execute(&amb.admin).await.is_err(), "{sql}");
    }
}

#[tokio::test]
async fn quando_o_pacote_tem_erro_ou_passa_do_limite_entao_nada_e_publicado() {
    let amb = Ambiente::novo().await;
    precos(&amb).await;
    let com_exames = |exames: Value| {
        let mut n = cardio(&amb, 0, 30_000);
        n["exames"] = exames;
        n
    };
    let onze: Vec<Value> = (0..11).map(|_| exame("40901114")).collect();
    let mut curto = cardio(&amb, 0, 69_000);
    curto["motivo"] = json!("ok");
    let mut sem_preparo = cardio(&amb, 0, 69_000);
    sem_preparo["exames"][0]["preparo"] = json!("");
    for (corpo, codigo) in [
        (com_exames(json!([exame("40901114")])), "pacote-invalido"),
        (com_exames(json!(onze)), "pacote-invalido"),
        (
            com_exames(json!([exame("40901114"), exame("40901114")])),
            "pacote-invalido",
        ),
        (
            com_exames(json!([exame("40901114"), exame("41001010")])),
            "exame-sem-preco",
        ),
        (
            com_exames(json!([exame("40901114"), exame("99999999")])),
            "procedimento-fora-do-catalogo",
        ),
        (cardio(&amb, 0, 89_000), "pacote-nao-sai-mais-barato"),
        (cardio(&amb, 0, 95_000), "pacote-nao-sai-mais-barato"),
        (cardio(&amb, 0, 53_399), "desconto-acima-do-limite"),
        (cardio(&amb, 0, 0), "preco-invalido"),
        (cardio(&amb, 0, -100), "preco-invalido"),
        (cardio(&amb, -1, 69_000), "vigencia-invalida"),
        (curto, "motivo-invalido"),
        (sem_preparo, "pacote-invalido"),
    ] {
        let (st, e) = post(
            &amb,
            GER,
            "/v1/pacotes",
            Some(Uuid::now_v7()),
            corpo.clone(),
        )
        .await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo)),
            "{corpo}"
        );
    }
    assert_eq!(
        amb.contar("SELECT count(*) FROM financeiro.pacote").await,
        0
    );
    // O maior desconto aceito (40%: R$ 534,00 sobre R$ 890,00) entra.
    assert_eq!(
        post(
            &amb,
            GER,
            "/v1/pacotes",
            Some(Uuid::now_v7()),
            cardio(&amb, 0, 53_400)
        )
        .await
        .0,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn quando_duas_versoes_chegam_juntas_entao_so_uma_entra() {
    let amb = Ambiente::novo().await;
    precos(&amb).await;
    let codigo = criar(&amb).await;
    let uri = format!("/v1/pacotes/{codigo}/versoes");
    let mut a = cardio(&amb, 7, 70_000);
    a["regra_versao_vista"] = json!(1);
    let mut b = cardio(&amb, 8, 71_000);
    b["regra_versao_vista"] = json!(1);
    let ((s1, _), (s2, _)) = tokio::join!(
        post(&amb, GER, &uri, Some(Uuid::now_v7()), a),
        post(&amb, GER, &uri, Some(Uuid::now_v7()), b),
    );
    assert_eq!(
        [s1, s2]
            .iter()
            .filter(|s| **s == StatusCode::CREATED)
            .count(),
        1,
        "{s1} {s2}"
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM financeiro.pacote").await,
        2
    );
}

#[tokio::test]
async fn quando_quem_nao_pode_ou_de_outra_unidade_tenta_entao_e_recusado() {
    let amb = Ambiente::novo().await;
    precos(&amb).await;
    let codigo = criar(&amb).await;
    sqlx::query("INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao) VALUES ($1, 'USR-gerente.b', 'gerente.b', 'Gerência Norte', ARRAY['gerente'], 'seed', $2, 'seed')")
        .bind(Uuid::now_v7()).bind(UNIDADE_B).execute(&amb.admin).await.unwrap();
    for login in [
        "carla.caixa",
        "medico.a",
        "ti.a",
        "tv.a",
        "rita.faturamento",
    ] {
        assert_eq!(
            get(&amb, login, "/v1/pacotes").await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    let mut nova = cardio(&amb, 10, 70_000);
    nova["regra_versao_vista"] = json!(1);
    // A recepção consulta, mas não cria nem muda o preço.
    assert_eq!(
        post(
            &amb,
            "ana.souza",
            "/v1/pacotes",
            Some(Uuid::now_v7()),
            cardio(&amb, 0, 60_000)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(
            &amb,
            "ana.souza",
            &format!("/v1/pacotes/{codigo}/versoes"),
            Some(Uuid::now_v7()),
            nova.clone()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Outra unidade não vê nem publica versão do pacote da unidade A.
    assert_eq!(
        get(&amb, "gerente.b", "/v1/pacotes").await.1["pacotes"],
        json!([])
    );
    assert_eq!(
        post(
            &amb,
            "gerente.b",
            &format!("/v1/pacotes/{codigo}/versoes"),
            Some(Uuid::now_v7()),
            nova
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM financeiro.pacote").await,
        1
    );
}
