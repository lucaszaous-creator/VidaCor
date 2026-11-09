//! Convênios e tabelas (Convenios-Tabelas.dc.html; DEC-2026-11-09-110 a 114). Rota HTTP real e
//! PostgreSQL real (G4): caminho feliz, erros, repetição, duas requisições simultâneas, limites e
//! acesso indevido. Só dados fictícios.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use chrono::Duration;
use comum::{Ambiente, UNIDADE_A, UNIDADE_B};
use serde_json::{json, Value};
use uuid::Uuid;
use vidacor_api::banco::{transacao, ContextoBanco};

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

fn dia(amb: &Ambiente, dias: i64) -> String {
    (amb.dia + Duration::days(dias)).to_string()
}

const FAT: &str = "rita.faturamento";
const GER: &str = "patricia.lemos";

fn termos() -> Value {
    json!({
        "codigo_prestador": "0004567", "nome_prestador": "VidaCor Imagem Unidade Centro", "cnes": "1234567",
        "regra_versao": "AUD-UNIMED-2026.1", "prazo_envio_dias": 30, "prazo_recurso_dias": 30, "prazo_pagamento_dias": 30,
        "vigencia_inicio": "2021-01-01", "vigencia_fim": "2027-12-31", "reajuste": "IPCA em janeiro",
        "contato_nome": "Fernanda Ruas", "contato_telefone": "(48) 3000-1100", "contato_email": "prestador@operadora.exemplo",
        "autorizacao_por_webservice": true, "ativo": true
    })
}

fn novo(operadora: &str) -> Value {
    json!({ "operadora": operadora, "registro_ans": "34.567-8", "termos": termos(), "motivo": "credenciamento assinado em 2021" })
}

/// Cadastra o convênio pela rota e semeia a tabela v1 (vigente há 30 dias), como a carga inicial faria.
async fn unimed_com_tabela(amb: &Ambiente) -> Uuid {
    let (st, c) = post(
        amb,
        FAT,
        "/v1/convenios",
        Some(Uuid::now_v7()),
        novo("Unimed"),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    let id: Uuid = c["id"].as_str().unwrap().parse().unwrap();
    let t = Uuid::now_v7();
    sqlx::query("INSERT INTO faturamento.tabela (id, codigo, contrato_id, versao, vigente_desde, descricao, origem, unidade_id, correlacao)
                 VALUES ($1, 'TBC-SEED-1', $2, 1, $3, 'CBHPM 5ª ed. mais 10%', 'seed', $4, 'seed')")
        .bind(t).bind(id).bind(amb.dia - Duration::days(30)).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO faturamento.preco (tabela_id, tuss, descricao, valor_centavos, unidade_id) VALUES
                 ($1, '40901114', 'Ecocardiograma transtorácico', 24860, $2), ($1, '41001109', 'TC de abdome com contraste', 61200, $2),
                 ($1, '40801012', 'Raio X de tórax', 6210, $2)")
        .bind(t).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    id
}

async fn gerente_b(amb: &Ambiente) {
    sqlx::query("INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao) VALUES ($1, 'USR-gerente.b', 'gerente.b', 'Gerência Norte', ARRAY['gerente'], 'seed', $2, 'seed')")
        .bind(Uuid::now_v7()).bind(UNIDADE_B).execute(&amb.admin).await.unwrap();
}

fn reajuste(amb: &Ambiente, pb: i64) -> Value {
    json!({ "vigente_desde": dia(amb, 30), "descricao": "Unimed 2027", "documento": "Carta de reajuste 2027", "motivo": "reajuste anual pelo IPCA", "reajuste_pontos_base": pb })
}

#[tokio::test]
async fn quando_o_faturamento_cadastra_e_muda_o_convenio_entao_fica_versao_com_motivo() {
    let amb = Ambiente::novo().await;
    let chave = Uuid::now_v7();
    let (st, c) = post(&amb, FAT, "/v1/convenios", Some(chave), novo("Unimed")).await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    assert_eq!(
        (
            c["registro_ans"].as_str(),
            c["situacao"].as_str(),
            c["versao"].as_i64()
        ),
        (Some("345678"), Some("vigente"), Some(1))
    );
    assert_eq!(c["auditoria"]["versao"], "AUD-UNIMED-2026.1");
    assert_eq!(c["historico"][0]["tipo"], "cadastrar");
    let (st, c2) = post(&amb, FAT, "/v1/convenios", Some(chave), novo("Unimed")).await;
    assert_eq!((st, &c2), (StatusCode::CREATED, &c), "clique duplo");
    let (st, e) = post(
        &amb,
        FAT,
        "/v1/convenios",
        Some(Uuid::now_v7()),
        novo(" unimed "),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("operadora-ja-cadastrada"))
    );
    // A gerência acompanha.
    let (st, lista) = get(&amb, GER, "/v1/convenios").await;
    assert_eq!((st, lista.as_array().unwrap().len()), (StatusCode::OK, 1));

    let id = c["id"].as_str().unwrap();
    let uri = format!("/v1/convenios/{id}/alteracoes");
    let mut t = termos();
    t["vigencia_fim"] = json!(dia(&amb, 40));
    let (st, a) = post(
        &amb,
        FAT,
        &uri,
        None,
        json!({ "versao": 1, "termos": t, "motivo": "aditivo de prorrogação" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{a}");
    assert_eq!(
        (a["versao"].as_i64(), a["situacao"].as_str()),
        (Some(2), Some("renovar"))
    );
    // Quem editou sobre a versão 1 chega tarde.
    let (st, e) = post(
        &amb,
        FAT,
        &uri,
        None,
        json!({ "versao": 1, "termos": termos(), "motivo": "tela desatualizada" }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("versao-mudou"))
    );
    let (st, e) = post(
        &amb,
        FAT,
        &uri,
        None,
        json!({ "versao": 2, "termos": t, "motivo": "nada mudou aqui" }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("nada-a-mudar"))
    );
    t["ativo"] = json!(false);
    let (st, a) = post(
        &amb,
        FAT,
        &uri,
        None,
        json!({ "versao": 2, "termos": t, "motivo": "descredenciamento pedido" }),
    )
    .await;
    assert_eq!(
        (st, a["situacao"].as_str()),
        (StatusCode::OK, Some("inativo"))
    );
    let tipos: Vec<&str> = a["historico"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["tipo"].as_str().unwrap())
        .collect();
    assert_eq!(tipos, vec!["desativar", "alterar", "cadastrar"]);
    assert_eq!(a["historico"][1]["antes"]["vigencia_fim"], "2027-12-31");

    // Limites e erros: nada é gravado.
    let campo = |k: &str, v: Value| {
        let mut n = novo("Bradesco Saúde");
        n["termos"][k] = v;
        n
    };
    let mut ans = novo("Bradesco Saúde");
    ans["registro_ans"] = json!("12345");
    let mut curto = novo("Bradesco Saúde");
    curto["motivo"] = json!("ok");
    for (corpo, codigo) in [
        (campo("cnes", json!("123456")), "convenio-invalido"),
        (
            campo("regra_versao", json!("AUD-INVENTADA-2026.1")),
            "convenio-invalido",
        ),
        (campo("prazo_envio_dias", json!(0)), "convenio-invalido"),
        (campo("prazo_recurso_dias", json!(366)), "convenio-invalido"),
        (
            campo("prazo_pagamento_dias", json!(-1)),
            "convenio-invalido",
        ),
        (
            campo("vigencia_fim", json!("2020-12-31")),
            "convenio-invalido",
        ),
        (
            campo("contato_email", json!("sem-arroba")),
            "convenio-invalido",
        ),
        (
            campo("codigo_prestador", json!("000-4567")),
            "convenio-invalido",
        ),
        (ans, "convenio-invalido"),
        (curto, "motivo-invalido"),
    ] {
        let (st, e) = post(
            &amb,
            FAT,
            "/v1/convenios",
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
        amb.contar("SELECT count(*) FROM faturamento.contrato")
            .await,
        1
    );
    // Histórico imutável, contrato não se apaga, nem pelo dono do banco.
    for sql in [
        "UPDATE faturamento.contrato_mudanca SET motivo = 'trocado'",
        "DELETE FROM faturamento.contrato_mudanca",
        "DELETE FROM faturamento.contrato",
    ] {
        assert!(sqlx::query(sql).execute(&amb.admin).await.is_err(), "{sql}");
    }
}

#[tokio::test]
async fn quando_o_faturamento_propoe_reajuste_entao_so_a_gerencia_aprova_e_a_vigente_nao_muda() {
    let amb = Ambiente::novo().await;
    let id = unimed_com_tabela(&amb).await;
    let (st, p) = get(
        &amb,
        FAT,
        &format!("/v1/convenios/{id}/previa-de-reajuste?pontos_base=520"),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{p}");
    // 248,60 → 261,53; 612,00 → 643,82; 62,10 → 65,33 (o servidor calcula, G1).
    assert_eq!(
        (
            p["afetados"].as_i64(),
            p["total_antes_centavos"].as_i64(),
            p["total_depois_centavos"].as_i64()
        ),
        (Some(3), Some(92_270), Some(97_068))
    );
    assert_eq!(p["maiores"][0]["tuss"], "41001109");

    let chave = Uuid::now_v7();
    let uri = format!("/v1/convenios/{id}/propostas");
    let (st, pr) = post(&amb, FAT, &uri, Some(chave), reajuste(&amb, 520)).await;
    assert_eq!(st, StatusCode::CREATED, "{pr}");
    assert_eq!(
        (
            pr["situacao"].as_str(),
            pr["base_versao"].as_i64(),
            pr["total_depois_centavos"].as_i64()
        ),
        (Some("pendente"), Some(1), Some(97_068))
    );
    let (st, pr2) = post(&amb, FAT, &uri, Some(chave), reajuste(&amb, 520)).await;
    assert_eq!(
        (st, &pr2),
        (StatusCode::CREATED, &pr),
        "repetir não duplica"
    );
    let (st, e) = post(&amb, FAT, &uri, Some(Uuid::now_v7()), reajuste(&amb, 700)).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("proposta-pendente"))
    );

    let decisao = format!(
        "/v1/convenios/propostas/{}/decisao",
        pr["id"].as_str().unwrap()
    );
    // O faturamento não aprova; a gerência precisa confirmar a identidade.
    assert_eq!(
        post(&amb, FAT, &decisao, None, json!({ "aprovar": true }))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let s = amb.entrar(GER).await;
    amb.expirar_reautenticacao().await;
    let (st, _, e) = amb
        .pedir(amb.json(Method::POST, &decisao, &s, None, json!({ "aprovar": true })))
        .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("reautenticacao-necessaria"))
    );
    let (st, ap) = post(&amb, GER, &decisao, None, json!({ "aprovar": true })).await;
    assert_eq!(st, StatusCode::OK, "{ap}");
    assert_eq!(
        (
            ap["situacao"].as_str(),
            ap["tabela_versao"].as_i64(),
            ap["decidido_por"].as_str()
        ),
        (Some("aprovada"), Some(2), Some("Patrícia Lemos"))
    );
    let (st, e) = post(
        &amb,
        GER,
        &decisao,
        None,
        json!({ "aprovar": false, "motivo": "mudei de ideia" }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("proposta-ja-decidida"))
    );

    // A conferência de hoje continua na v1; a v2 começa no futuro, com o preço anterior ao lado.
    let (_, c) = get(&amb, GER, &format!("/v1/convenios/{id}")).await;
    let sit: Vec<(i64, &str)> = c["tabelas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["versao"].as_i64().unwrap(),
                t["situacao"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(sit, vec![(2, "futura"), (1, "vigente")]);
    assert_eq!(c["tabelas"][0]["proposta_por"], "Rita Faturamento");
    assert!(c["proposta_pendente"].is_null());
    let (_, hoje) = get(&amb, FAT, &format!("/v1/convenios/{id}/precos")).await;
    assert_eq!(hoje["versao"], 1);
    let (_, v2) = get(&amb, FAT, &format!("/v1/convenios/{id}/precos?versao=2")).await;
    let eco = v2["itens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["tuss"] == "40901114")
        .unwrap()
        .clone();
    assert_eq!(
        (
            eco["valor_centavos"].as_i64(),
            eco["valor_anterior_centavos"].as_i64()
        ),
        (Some(26_153), Some(24_860))
    );
    // Tabela e proposta não se alteram nem se apagam.
    for sql in [
        "UPDATE faturamento.tabela_proposta_preco SET valor_centavos = 1",
        "DELETE FROM faturamento.tabela_proposta",
        "UPDATE faturamento.preco SET valor_centavos = 1",
        "DELETE FROM faturamento.tabela",
    ] {
        assert!(sqlx::query(sql).execute(&amb.admin).await.is_err(), "{sql}");
    }
}

#[tokio::test]
async fn quando_a_gerencia_recusa_ou_quem_propos_tenta_aprovar_entao_a_tabela_nao_muda() {
    let amb = Ambiente::novo().await;
    let id = unimed_com_tabela(&amb).await;
    sqlx::query("INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao) VALUES ($1, 'USR-dupla', 'dupla', 'Pessoa com dois perfis', ARRAY['faturista','gerente'], 'seed', $2, 'seed')")
        .bind(Uuid::now_v7()).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    let uri = format!("/v1/convenios/{id}/propostas");
    let precos = json!({ "vigente_desde": dia(&amb, 10), "descricao": "Unimed tabela própria", "documento": "Aditivo 03", "motivo": "inclusão de tomografia",
        "precos": [{ "tuss": "40901114", "valor_centavos": 25000 }, { "tuss": "41001010", "valor_centavos": 50000 }] });
    let (st, pr) = post(&amb, "dupla", &uri, Some(Uuid::now_v7()), precos).await;
    assert_eq!(st, StatusCode::CREATED, "{pr}");
    assert_eq!(
        (
            pr["tipo"].as_str(),
            pr["itens"].as_i64(),
            pr["total_antes_centavos"].as_i64()
        ),
        (Some("precos"), Some(2), Some(24_860))
    );
    let decisao = format!(
        "/v1/convenios/propostas/{}/decisao",
        pr["id"].as_str().unwrap()
    );
    let (st, e) = post(&amb, "dupla", &decisao, None, json!({ "aprovar": true })).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::FORBIDDEN, Some("separacao-de-funcoes"))
    );
    let (st, e) = post(&amb, GER, &decisao, None, json!({ "aprovar": false })).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("motivo-invalido"))
    );
    let (st, r) = post(
        &amb,
        GER,
        &decisao,
        None,
        json!({ "aprovar": false, "motivo": "valor da TC acima do contrato" }),
    )
    .await;
    assert_eq!(
        (st, r["situacao"].as_str(), r["tabela_versao"].is_null()),
        (StatusCode::OK, Some("recusada"), true)
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM faturamento.tabela").await,
        1
    );
    // Nem pelo dono do banco quem propôs vira quem decide.
    assert!(
        sqlx::query("UPDATE faturamento.tabela_proposta SET decidido_por = criado_por")
            .execute(&amb.admin)
            .await
            .is_err()
    );
    // Depois da recusa, pode haver proposta nova.
    assert_eq!(
        post(&amb, FAT, &uri, Some(Uuid::now_v7()), reajuste(&amb, 300))
            .await
            .0,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn quando_a_proposta_tem_erro_ou_passa_do_limite_entao_nada_e_proposto() {
    let amb = Ambiente::novo().await;
    let id = unimed_com_tabela(&amb).await;
    let uri = format!("/v1/convenios/{id}/propostas");
    let mut hoje = reajuste(&amb, 500);
    hoje["vigente_desde"] = json!(dia(&amb, 0));
    let mut antes = reajuste(&amb, 500);
    antes["vigente_desde"] = json!(dia(&amb, -1));
    let mut longe = reajuste(&amb, 500);
    longe["vigente_desde"] = json!(dia(&amb, 367));
    let mut os_dois = reajuste(&amb, 500);
    os_dois["precos"] = json!([{ "tuss": "40901114", "valor_centavos": 1 }]);
    let mut nenhum = reajuste(&amb, 500);
    nenhum
        .as_object_mut()
        .unwrap()
        .remove("reajuste_pontos_base");
    let com = |precos: Value| {
        let mut n = nenhum.clone();
        n["precos"] = precos;
        n
    };
    for (corpo, codigo) in [
        (reajuste(&amb, 0), "reajuste-invalido"),
        (reajuste(&amb, -100), "reajuste-invalido"),
        (reajuste(&amb, 3001), "reajuste-invalido"),
        (hoje, "vigencia-invalida"),
        (antes, "vigencia-invalida"),
        (longe, "vigencia-invalida"),
        (os_dois, "proposta-invalida"),
        (nenhum.clone(), "proposta-invalida"),
        (com(json!([])), "proposta-invalida"),
        (
            com(json!([{ "tuss": "4090111", "valor_centavos": 100 }])),
            "proposta-invalida",
        ),
        (
            com(
                json!([{ "tuss": "40901114", "valor_centavos": 100 }, { "tuss": "40901114", "valor_centavos": 200 }]),
            ),
            "proposta-invalida",
        ),
        (
            com(json!([{ "tuss": "40901114", "valor_centavos": 0 }])),
            "preco-invalido",
        ),
        (
            com(json!([{ "tuss": "40901114", "valor_centavos": -5 }])),
            "preco-invalido",
        ),
        (
            com(json!([{ "tuss": "40901114", "valor_centavos": 10_000_001 }])),
            "preco-invalido",
        ),
        (
            com(json!([{ "tuss": "99999999", "valor_centavos": 100 }])),
            "procedimento-fora-do-catalogo",
        ),
    ] {
        let (st, e) = post(&amb, FAT, &uri, Some(Uuid::now_v7()), corpo.clone()).await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo)),
            "{corpo}"
        );
    }
    assert_eq!(
        amb.contar("SELECT count(*) FROM faturamento.tabela_proposta")
            .await,
        0
    );
    // O maior reajuste aceito (30%) entra.
    assert_eq!(
        post(&amb, FAT, &uri, Some(Uuid::now_v7()), reajuste(&amb, 3000))
            .await
            .0,
        StatusCode::CREATED
    );
    // Sem tabela, não há o que reajustar.
    let (_, b) = post(
        &amb,
        FAT,
        "/v1/convenios",
        Some(Uuid::now_v7()),
        novo("Amil"),
    )
    .await;
    let (st, e) = post(
        &amb,
        FAT,
        &format!("/v1/convenios/{}/propostas", b["id"].as_str().unwrap()),
        Some(Uuid::now_v7()),
        reajuste(&amb, 500),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("sem-tabela"))
    );
}

#[tokio::test]
async fn quando_duas_propostas_ou_duas_decisoes_chegam_juntas_entao_so_uma_vale() {
    let amb = Ambiente::novo().await;
    let id = unimed_com_tabela(&amb).await;
    let uri = format!("/v1/convenios/{id}/propostas");
    let ((s1, a), (s2, b)) = tokio::join!(
        post(&amb, FAT, &uri, Some(Uuid::now_v7()), reajuste(&amb, 500)),
        post(&amb, FAT, &uri, Some(Uuid::now_v7()), reajuste(&amb, 600)),
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT], "{a} {b}");
    let pid: Uuid = sqlx::query_scalar("SELECT id FROM faturamento.tabela_proposta")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    let decisao = format!("/v1/convenios/propostas/{pid}/decisao");
    let ((s1, _), (s2, _)) = tokio::join!(
        post(&amb, GER, &decisao, None, json!({ "aprovar": true })),
        post(
            &amb,
            GER,
            &decisao,
            None,
            json!({ "aprovar": false, "motivo": "conferir a carta antes" })
        ),
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    assert!(amb.contar("SELECT count(*) FROM faturamento.tabela").await <= 2);
    assert_eq!(
        amb.contar("SELECT count(*) FROM faturamento.tabela_proposta WHERE situacao = 'pendente'")
            .await,
        0
    );
}

#[tokio::test]
async fn quando_o_faturamento_publica_regras_de_autorizacao_entao_fica_versao_inteira() {
    let amb = Ambiente::novo().await;
    let id = unimed_com_tabela(&amb).await;
    let uri = format!("/v1/convenios/{id}/regras-de-autorizacao");
    let regras = |vista: Value| {
        json!({ "versao_vista": vista, "motivo": "manual do prestador 2026", "itens": [
        { "tuss": "41001109", "exige_senha": true, "validade_dias": 30, "documentos": "Pedido médico com CID e relatório clínico" },
        { "tuss": "40801012", "exige_senha": false, "validade_dias": null, "documentos": "Pedido médico e carteirinha" }] })
    };
    let (st, r) = post(&amb, FAT, &uri, None, regras(Value::Null)).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    assert_eq!(
        (r["versao_regras"].as_i64(), r["itens"][0]["tuss"].as_str()),
        (Some(1), Some("41001109"))
    );
    let (st, e) = post(&amb, FAT, &uri, None, regras(Value::Null)).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("versao-mudou"))
    );
    assert_eq!(
        post(&amb, FAT, &uri, None, regras(json!(1))).await.0,
        StatusCode::CREATED
    );
    let (_, c) = get(&amb, GER, &format!("/v1/convenios/{id}")).await;
    assert_eq!(c["regras_autorizacao"]["versao_regras"], 2);
    let item = |senha: bool, val: Value, tuss: &str| json!({ "versao_vista": 2, "motivo": "teste de limite", "itens": [{ "tuss": tuss, "exige_senha": senha, "validade_dias": val, "documentos": "Pedido médico" }] });
    for (corpo, codigo) in [
        (item(true, Value::Null, "41001109"), "regras-invalidas"),
        (item(true, json!(0), "41001109"), "regras-invalidas"),
        (item(true, json!(366), "41001109"), "regras-invalidas"),
        (item(false, json!(30), "41001109"), "regras-invalidas"),
        (
            item(false, Value::Null, "99999999"),
            "procedimento-fora-do-catalogo",
        ),
        (
            json!({ "versao_vista": 2, "motivo": "lista vazia", "itens": [] }),
            "regras-invalidas",
        ),
    ] {
        let (st, e) = post(&amb, FAT, &uri, None, corpo.clone()).await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo)),
            "{corpo}"
        );
    }
    assert!(
        sqlx::query("DELETE FROM faturamento.regra_autorizacao_item")
            .execute(&amb.admin)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn quando_quem_nao_pode_ou_de_outra_unidade_tenta_entao_e_recusado() {
    let amb = Ambiente::novo().await;
    let id = unimed_com_tabela(&amb).await;
    gerente_b(&amb).await;
    for login in [
        "ana.souza",
        "carla.caixa",
        "medico.a",
        "ti.a",
        "financeiro.a",
        "tv.a",
    ] {
        assert_eq!(
            get(&amb, login, "/v1/convenios").await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    // A gerência lê, mas não cadastra, não muda, não propõe nem publica regras.
    for (uri, corpo) in [
        ("/v1/convenios".to_owned(), novo("Amil")),
        (
            format!("/v1/convenios/{id}/alteracoes"),
            json!({ "versao": 1, "termos": termos(), "motivo": "tentativa indevida" }),
        ),
        (format!("/v1/convenios/{id}/propostas"), reajuste(&amb, 500)),
        (
            format!("/v1/convenios/{id}/regras-de-autorizacao"),
            json!({ "versao_vista": null, "motivo": "tentativa indevida", "itens": [] }),
        ),
    ] {
        assert_eq!(
            post(&amb, GER, &uri, Some(Uuid::now_v7()), corpo).await.0,
            StatusCode::FORBIDDEN,
            "{uri}"
        );
    }
    // Outra unidade não vê nem mexe no convênio da unidade A (ter o ID não dá direito).
    assert_eq!(
        get(&amb, "faturista.b", &format!("/v1/convenios/{id}"))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(&amb, "faturista.b", &format!("/v1/convenios/{id}/precos"))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(get(&amb, "faturista.b", "/v1/convenios").await.1, json!([]));
    assert_eq!(
        post(
            &amb,
            "faturista.b",
            &format!("/v1/convenios/{id}/propostas"),
            Some(Uuid::now_v7()),
            reajuste(&amb, 500)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (_, pr) = post(
        &amb,
        FAT,
        &format!("/v1/convenios/{id}/propostas"),
        Some(Uuid::now_v7()),
        reajuste(&amb, 500),
    )
    .await;
    let decisao = format!(
        "/v1/convenios/propostas/{}/decisao",
        pr["id"].as_str().unwrap()
    );
    assert_eq!(
        post(
            &amb,
            "gerente.b",
            &decisao,
            None,
            json!({ "aprovar": true })
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // No banco, o faturamento não muda o contrato sem histórico nem cria versão de tabela por fora.
    let rita: Uuid = sqlx::query_scalar("SELECT id FROM identidade.usuario WHERE login = $1")
        .bind(FAT)
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    let ctx = ContextoBanco {
        usuario_id: Some(rita),
        unidade_id: UNIDADE_A,
        perfis: vec!["faturista".into()],
    };
    let mut tx = transacao(&amb.app_pool, &ctx).await.unwrap();
    assert!(sqlx::query(
        "UPDATE faturamento.contrato SET reajuste = 'mudança sem histórico' WHERE id = $1"
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .is_err());
    drop(tx);
    let mut tx = transacao(&amb.app_pool, &ctx).await.unwrap();
    assert!(sqlx::query("INSERT INTO faturamento.tabela (id, codigo, contrato_id, versao, vigente_desde, descricao, origem, unidade_id, correlacao) VALUES ($1, 'TBC-X', $2, 9, $4, 'por fora', 'x', $3, 'x')")
        .bind(Uuid::now_v7()).bind(id).bind(UNIDADE_A).bind(amb.dia + Duration::days(90)).execute(&mut *tx).await.is_err());
}
