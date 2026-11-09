//! Exames e procedimentos e tabela de preços versionada (Exames-Procedimentos.dc.html; docs/04
//! Cadastros base; DEC-2026-11-08-93 e 94). Rota HTTP real e PostgreSQL real (G4): caminho feliz,
//! erros, repetição, duas requisições simultâneas, limites e acesso indevido. Só dados fictícios.

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

/// Tabela v12 do particular, vigente desde o mês passado, e uma gerente da unidade B (fictícias).
async fn preparar(amb: &Ambiente) {
    let tabela = Uuid::now_v7();
    sqlx::query("INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, origem, unidade_id, correlacao) VALUES ($1, 12, current_date - 30, 'Particular 2026', 'seed', $2, 'seed')")
        .bind(tabela).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO financeiro.preco (tabela_id, tuss, valor_centavos) VALUES ($1, '40901114', 38000), ($1, '40901300', 42000)")
        .bind(tabela).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao) VALUES ($1, 'USR-gerente.b', 'gerente.b', 'Gerência Norte', ARRAY['gerente'], 'seed', $2, 'seed')")
        .bind(Uuid::now_v7()).bind(UNIDADE_B).execute(&amb.admin).await.unwrap();
}

fn exame(catalogo: &Value, tuss: &str) -> Value {
    catalogo["procedimentos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["tuss"] == tuss)
        .cloned()
        .unwrap_or(Value::Null)
}

fn amanha(dias: i64) -> String {
    (vidacor_api::tempo::hoje_local() + Duration::days(dias)).to_string()
}

const CATALOGO: &str = "/v1/catalogo/procedimentos";
const TABELAS: &str = "/v1/catalogo/tabelas-de-preco";

#[tokio::test]
async fn quando_a_gerencia_inclui_um_exame_entao_a_recepcao_ve_com_o_preco_e_o_historico() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let (st, c) = get(&amb, "ana.souza", CATALOGO).await;
    assert_eq!(st, StatusCode::OK);
    let eco = exame(&c, "40901114");
    assert_eq!(
        (
            eco["preco_centavos"].as_i64(),
            eco["modalidade"].as_str(),
            eco["versao"].as_i64()
        ),
        (Some(38000), Some("US"), Some(1))
    );
    assert_eq!(c["tabelas"][0]["situacao"], "vigente");

    let chave = Uuid::now_v7();
    let novo = json!({ "tuss": "40901130", "nome": "USG de abdome total", "modalidade": "us", "duracao_min": 30, "requer_contraste": false, "motivo": "exame novo da Sala 2" });
    let (st, p) = post(&amb, "patricia.lemos", CATALOGO, Some(chave), novo.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    assert_eq!(
        (
            p["modalidade"].as_str(),
            p["ativo"].as_bool(),
            p["preco_centavos"].is_null()
        ),
        (Some("US"), Some(true), true)
    );
    let (st, p2) = post(&amb, "patricia.lemos", CATALOGO, Some(chave), novo.clone()).await;
    assert_eq!((st, &p2), (StatusCode::CREATED, &p), "clique duplo");
    let (st, e) = post(&amb, "patricia.lemos", CATALOGO, Some(Uuid::now_v7()), novo).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("tuss-em-uso"))
    );
    let (st, h) = get(&amb, "ana.souza", &format!("{CATALOGO}/40901130/historico")).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        (
            h[0]["tipo"].as_str(),
            h[0]["quem"].as_str(),
            h[0]["antes"].is_null()
        ),
        (Some("incluir"), Some("Patrícia Lemos"), true)
    );
    for (corpo, codigo) in [
        (
            json!({ "tuss": "4090113", "nome": "Exame", "modalidade": "US", "duracao_min": 30, "requer_contraste": false, "motivo": "teste de limite" }),
            "procedimento-invalido",
        ),
        (
            json!({ "tuss": "40901131", "nome": "Ex", "modalidade": "US", "duracao_min": 30, "requer_contraste": false, "motivo": "teste de limite" }),
            "procedimento-invalido",
        ),
        (
            json!({ "tuss": "40901131", "nome": "Exame novo", "modalidade": "U1", "duracao_min": 30, "requer_contraste": false, "motivo": "teste de limite" }),
            "procedimento-invalido",
        ),
        (
            json!({ "tuss": "40901131", "nome": "Exame novo", "modalidade": "US", "duracao_min": 0, "requer_contraste": false, "motivo": "teste de limite" }),
            "procedimento-invalido",
        ),
        (
            json!({ "tuss": "40901131", "nome": "Exame novo", "modalidade": "US", "duracao_min": 245, "requer_contraste": false, "motivo": "teste de limite" }),
            "procedimento-invalido",
        ),
        (
            json!({ "tuss": "40901131", "nome": "Exame novo", "modalidade": "US", "duracao_min": 30, "requer_contraste": false, "motivo": "curt" }),
            "motivo-invalido",
        ),
    ] {
        let (st, e) = post(
            &amb,
            "patricia.lemos",
            CATALOGO,
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
        amb.contar("SELECT count(*) FROM agenda.procedimento WHERE tuss = '40901131'")
            .await,
        0
    );
}

#[tokio::test]
async fn quando_a_gerencia_muda_duracao_modalidade_ou_ativo_entao_fica_versao_com_motivo() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let uri = format!("{CATALOGO}/40901114/mudancas");
    let (st, p) = post(
        &amb,
        "patricia.lemos",
        &uri,
        None,
        json!({ "versao": 1, "duracao_min": 40, "motivo": "média real de 40 minutos" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{p}");
    assert_eq!(
        (p["duracao_min"].as_i64(), p["versao"].as_i64()),
        (Some(40), Some(2))
    );
    // Quem editou sobre a versão 1 chega tarde: nada muda.
    let (st, e) = post(
        &amb,
        "patricia.lemos",
        &uri,
        None,
        json!({ "versao": 1, "modalidade": "DX", "motivo": "tela desatualizada" }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("versao-mudou"))
    );
    let (st, p) = post(
        &amb,
        "patricia.lemos",
        &uri,
        None,
        json!({ "versao": 2, "ativo": false, "motivo": "aparelho em manutenção prolongada" }),
    )
    .await;
    assert_eq!((st, p["ativo"].as_bool()), (StatusCode::OK, Some(false)));
    let (_, h) = get(
        &amb,
        "patricia.lemos",
        &format!("{CATALOGO}/40901114/historico"),
    )
    .await;
    let h = h.as_array().unwrap();
    assert_eq!(
        h.iter()
            .map(|m| m["tipo"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["desativar", "alterar"]
    );
    assert_eq!(
        (
            h[1]["antes"]["duracao_min"].as_i64(),
            h[1]["depois"]["duracao_min"].as_i64()
        ),
        (Some(30), Some(40))
    );
    for (corpo, st_esperado, codigo) in [
        (
            json!({ "versao": 3, "ativo": false, "motivo": "já está desativado" }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "nada-a-mudar",
        ),
        (
            json!({ "versao": 3, "duracao_min": 7, "motivo": "duração quebrada" }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "procedimento-invalido",
        ),
        (
            json!({ "versao": 3, "duracao_min": -5, "motivo": "duração negativa" }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "procedimento-invalido",
        ),
        (
            json!({ "versao": 3, "ativo": true, "motivo": "ok" }),
            StatusCode::UNPROCESSABLE_ENTITY,
            "motivo-invalido",
        ),
    ] {
        let (st, e) = post(&amb, "patricia.lemos", &uri, None, corpo.clone()).await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (st_esperado, Some(codigo)),
            "{corpo}"
        );
    }
    let (st, _) = post(
        &amb,
        "patricia.lemos",
        &format!("{CATALOGO}/99999999/mudancas"),
        None,
        json!({ "versao": 1, "ativo": false, "motivo": "não existe no catálogo" }),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Procedimento não se apaga, nem pelo dono do banco; histórico é imutável.
    for sql in [
        "DELETE FROM agenda.procedimento WHERE tuss = '40901114'",
        "UPDATE agenda.procedimento_mudanca SET motivo = 'trocado'",
        "DELETE FROM agenda.procedimento_mudanca",
    ] {
        assert!(sqlx::query(sql).execute(&amb.admin).await.is_err(), "{sql}");
    }
}

#[tokio::test]
async fn quando_duas_mudancas_chegam_juntas_sobre_a_mesma_versao_entao_so_uma_entra() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let uri = format!("{CATALOGO}/40901300/mudancas");
    let ((s1, _), (s2, _)) = tokio::join!(
        post(
            &amb,
            "patricia.lemos",
            &uri,
            None,
            json!({ "versao": 1, "duracao_min": 45, "motivo": "ajuste da agenda" })
        ),
        post(
            &amb,
            "patricia.lemos",
            &uri,
            None,
            json!({ "versao": 1, "duracao_min": 20, "motivo": "ajuste da agenda" })
        ),
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM agenda.procedimento_mudanca WHERE tuss = '40901300'")
            .await,
        1
    );
    // Duas inclusões do mesmo TUSS ao mesmo tempo: só uma.
    let novo = json!({ "tuss": "40901140", "nome": "USG de rins e vias urinárias", "modalidade": "US", "duracao_min": 30, "requer_contraste": false, "motivo": "exame novo da Sala 1" });
    let ((s1, _), (s2, _)) = tokio::join!(
        post(
            &amb,
            "patricia.lemos",
            CATALOGO,
            Some(Uuid::now_v7()),
            novo.clone()
        ),
        post(
            &amb,
            "patricia.lemos",
            CATALOGO,
            Some(Uuid::now_v7()),
            novo.clone()
        ),
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
}

fn tabela(vigente_desde: &str, precos: Value) -> Value {
    json!({ "vigente_desde": vigente_desde, "descricao": "Particular 2027", "motivo": "reajuste anual aprovado", "precos": precos })
}

#[tokio::test]
async fn quando_a_gerencia_publica_tabela_nova_entao_a_vigente_nao_muda_ate_a_data() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let chave = Uuid::now_v7();
    let corpo = tabela(
        &amanha(1),
        json!([{ "tuss": "40901114", "valor_centavos": 40000 }, { "tuss": "40901300", "valor_centavos": 45000 }]),
    );
    let (st, t) = post(&amb, "patricia.lemos", TABELAS, Some(chave), corpo.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    assert_eq!(
        (
            t["versao"].as_i64(),
            t["situacao"].as_str(),
            t["itens"].as_i64()
        ),
        (Some(13), Some("futura"), Some(2))
    );
    let (st, t2) = post(&amb, "patricia.lemos", TABELAS, Some(chave), corpo).await;
    assert_eq!((st, &t2), (StatusCode::CREATED, &t), "repetir não duplica");
    let (_, c) = get(&amb, "ana.souza", CATALOGO).await;
    let eco = exame(&c, "40901114");
    assert_eq!(
        (
            eco["preco_centavos"].as_i64(),
            eco["proximo_preco_centavos"].as_i64()
        ),
        (Some(38000), Some(40000)),
        "a vigente não mudou"
    );
    // O caixa e o orçamento usam a mesma regra: vigente é a maior versão já iniciada.
    let vigente: i32 = sqlx::query_scalar("SELECT versao FROM financeiro.tabela_preco WHERE unidade_id = $1 AND vigente_desde <= (now() AT TIME ZONE 'America/Sao_Paulo')::date ORDER BY versao DESC LIMIT 1")
        .bind(UNIDADE_A).fetch_one(&amb.admin).await.unwrap();
    assert_eq!(vigente, 12);
    // A tabela é versionada: ninguém altera nem apaga preço.
    for sql in [
        "UPDATE financeiro.preco SET valor_centavos = 1",
        "DELETE FROM financeiro.preco",
        "UPDATE financeiro.tabela_preco SET vigente_desde = (now() AT TIME ZONE 'America/Sao_Paulo')::date",
        "DELETE FROM financeiro.tabela_preco",
    ] {
        assert!(sqlx::query(sql).execute(&amb.admin).await.is_err(), "{sql}");
    }
}

#[tokio::test]
async fn quando_a_tabela_tem_erro_ou_passa_do_limite_entao_nada_e_publicado() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    sqlx::query("UPDATE agenda.procedimento SET ativo = false WHERE tuss = '41001010'")
        .execute(&amb.admin)
        .await
        .unwrap();
    let um = |tuss: &str, v: i64| json!([{ "tuss": tuss, "valor_centavos": v }]);
    for (corpo, codigo) in [
        (
            tabela(&amanha(0), um("40901114", 40000)),
            "vigencia-invalida",
        ),
        (
            tabela(&amanha(-1), um("40901114", 40000)),
            "vigencia-invalida",
        ),
        (
            tabela(&amanha(367), um("40901114", 40000)),
            "vigencia-invalida",
        ),
        (tabela(&amanha(1), json!([])), "tabela-invalida"),
        (tabela(&amanha(1), um("40901114", 0)), "preco-invalido"),
        (tabela(&amanha(1), um("40901114", -100)), "preco-invalido"),
        (
            tabela(&amanha(1), um("40901114", 10_000_001)),
            "preco-invalido",
        ),
        (
            tabela(
                &amanha(1),
                json!([{ "tuss": "40901114", "valor_centavos": 1 }, { "tuss": "40901114", "valor_centavos": 2 }]),
            ),
            "tabela-invalida",
        ),
        (
            tabela(&amanha(1), um("99999999", 100)),
            "procedimento-fora-do-catalogo",
        ),
        (
            tabela(&amanha(1), um("41001010", 100)),
            "procedimento-fora-do-catalogo",
        ),
    ] {
        let (st, e) = post(
            &amb,
            "patricia.lemos",
            TABELAS,
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
    // O limite superior de preço é aceito.
    let (st, t) = post(
        &amb,
        "patricia.lemos",
        TABELAS,
        Some(Uuid::now_v7()),
        tabela(&amanha(10), um("40901114", 10_000_000)),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    // Versão nova não começa antes (nem no mesmo dia) da última publicada.
    let (st, e) = post(
        &amb,
        "patricia.lemos",
        TABELAS,
        Some(Uuid::now_v7()),
        tabela(&amanha(5), um("40901114", 40000)),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("vigencia-invalida"))
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM financeiro.tabela_preco")
            .await,
        2
    );
}

#[tokio::test]
async fn quando_duas_versoes_com_a_mesma_vigencia_chegam_juntas_entao_so_uma_entra() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    let corpo = tabela(
        &amanha(3),
        json!([{ "tuss": "40901114", "valor_centavos": 41000 }]),
    );
    let ((s1, _), (s2, e2)) = tokio::join!(
        post(
            &amb,
            "patricia.lemos",
            TABELAS,
            Some(Uuid::now_v7()),
            corpo.clone()
        ),
        post(
            &amb,
            "patricia.lemos",
            TABELAS,
            Some(Uuid::now_v7()),
            corpo.clone()
        ),
    );
    let criadas = [s1, s2]
        .iter()
        .filter(|s| **s == StatusCode::CREATED)
        .count();
    assert_eq!(criadas, 1, "{s1} {s2} {e2}");
    assert!(
        [s1, s2].contains(&StatusCode::CONFLICT)
            || [s1, s2].contains(&StatusCode::UNPROCESSABLE_ENTITY)
    );
    assert_eq!(
        amb.contar(
            "SELECT count(*) FROM financeiro.tabela_preco WHERE vigente_desde > (now() AT TIME ZONE 'America/Sao_Paulo')::date"
        )
        .await,
        1
    );
    assert_eq!(amb.contar("SELECT count(*) FROM financeiro.preco").await, 3);
}

#[tokio::test]
async fn quando_quem_nao_e_da_gerencia_ou_de_outra_unidade_tenta_entao_e_recusado() {
    let amb = Ambiente::novo().await;
    preparar(&amb).await;
    // TI, médico, caixa e TV não consultam o catálogo.
    for login in ["ti.a", "medico.a", "carla.caixa", "tv.a"] {
        assert_eq!(
            get(&amb, login, CATALOGO).await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    // A recepção consulta, mas não inclui, não muda e não publica preço.
    for (uri, corpo) in [
        (
            CATALOGO.to_owned(),
            json!({ "tuss": "40901150", "nome": "Exame indevido", "modalidade": "US", "duracao_min": 30, "requer_contraste": false, "motivo": "tentativa indevida" }),
        ),
        (
            format!("{CATALOGO}/40901114/mudancas"),
            json!({ "versao": 1, "ativo": false, "motivo": "tentativa indevida" }),
        ),
        (
            TABELAS.to_owned(),
            tabela(
                &amanha(1),
                json!([{ "tuss": "40901114", "valor_centavos": 1 }]),
            ),
        ),
    ] {
        for login in ["ana.souza", "recepcao.b", "financeiro.a"] {
            let (st, _) = post(&amb, login, &uri, Some(Uuid::now_v7()), corpo.clone()).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "{login} {uri}");
        }
    }
    // Gerência de outra unidade: vê o catálogo da clínica, mas não os preços desta unidade.
    let (st, c) = get(&amb, "gerente.b", CATALOGO).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        (
            exame(&c, "40901114")["preco_centavos"].is_null(),
            c["tabelas"].as_array().unwrap().len()
        ),
        (true, 0)
    );
    // A tabela que ela publica é da unidade B e não muda os preços da A.
    let (st, t) = post(
        &amb,
        "gerente.b",
        TABELAS,
        Some(Uuid::now_v7()),
        tabela(
            &amanha(1),
            json!([{ "tuss": "40901114", "valor_centavos": 50000 }]),
        ),
    )
    .await;
    assert_eq!((st, t["versao"].as_i64()), (StatusCode::CREATED, Some(1)));
    let (_, c) = get(&amb, "patricia.lemos", CATALOGO).await;
    assert_eq!(exame(&c, "40901114")["proximo_preco_centavos"], Value::Null);
    // Pelo banco, o papel da aplicação sem gerência não grava preço.
    let ana = amb.s.usuarios["ana.souza"];
    let mut tx = amb.app_pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.usuario_id', $1, true), set_config('app.unidade_id', $2, true), set_config('app.perfis', 'recepcao', true)")
        .bind(ana.to_string()).bind(UNIDADE_A.to_string()).execute(&mut *tx).await.unwrap();
    let r = sqlx::query("INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, motivo, origem, unidade_id, correlacao) VALUES ($1, 99, current_date + 90, 'Indevida', 'tentativa indevida', 'teste', $2, 'teste')")
        .bind(Uuid::now_v7()).bind(UNIDADE_A).execute(&mut *tx).await;
    assert!(r.is_err());
}
