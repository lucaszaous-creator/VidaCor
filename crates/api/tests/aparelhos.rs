//! Aparelhos e integração DICOM (Aparelhos.dc.html): TI e gerência cadastram, desativam e reativam
//! com motivo e histórico imutável; o painel mostra a atividade só em números, sem dado de paciente.
//! Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

async fn cadastrar(amb: &Ambiente, login: &str, chave: Uuid, corpo: Value) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, "/v1/aparelhos", &s, Some(chave), corpo))
        .await;
    (st, v)
}

fn novo(amb: &Ambiente, ae: &str) -> Value {
    json!({ "nome": "Logiq E10", "ae_title": ae, "modalidade": "us", "sala_id": amb.s.salas[&2], "motivo": "aparelho novo da sala 2" })
}

#[tokio::test]
async fn ti_cadastra_com_motivo_repetir_nao_duplica_e_ae_title_e_unico() {
    let amb = Ambiente::novo().await;
    let chave = Uuid::now_v7();
    let (st, a) = cadastrar(&amb, "ti.a", chave, novo(&amb, "logiq_2")).await;
    assert_eq!(st, StatusCode::CREATED, "{a}");
    assert_eq!(
        (
            a["ae_title"].as_str(),
            a["modalidade"].as_str(),
            a["situacao"].as_str()
        ),
        (Some("LOGIQ_2"), Some("US"), Some("conectado"))
    );
    let (_, a2) = cadastrar(&amb, "ti.a", chave, novo(&amb, "logiq_2")).await;
    assert_eq!(a2["id"], a["id"]);
    // Outro aparelho com o mesmo AE Title: recusado.
    let (s3, v3) = cadastrar(
        &amb,
        "patricia.lemos",
        Uuid::now_v7(),
        novo(&amb, "LOGIQ_2"),
    )
    .await;
    assert_eq!(
        (s3, v3["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("ae-title-em-uso"))
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM agenda.aparelho_mudanca WHERE tipo = 'cadastrar'")
            .await,
        1
    );
    // Histórico imutável; aparelho não se apaga.
    assert!(
        sqlx::query("UPDATE agenda.aparelho_mudanca SET motivo = 'outro motivo'")
            .execute(&amb.admin)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM agenda.aparelho WHERE ae_title = 'LOGIQ_2'")
            .execute(&amb.admin)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn dados_invalidos_nao_cadastram_nada() {
    let amb = Ambiente::novo().await;
    let antes = amb.contar("SELECT count(*) FROM agenda.aparelho").await;
    for (corpo, codigo) in [
        (
            json!({ "nome": "X", "ae_title": "OK1", "modalidade": "US", "sala_id": amb.s.salas[&2], "motivo": "motivo válido" }),
            "aparelho-invalido",
        ),
        (
            json!({ "nome": "Logiq", "ae_title": "AE COM ESPACO", "modalidade": "US", "sala_id": amb.s.salas[&2], "motivo": "motivo válido" }),
            "aparelho-invalido",
        ),
        (
            json!({ "nome": "Logiq", "ae_title": "MUITO_LONGO_AE_TITLE", "modalidade": "US", "sala_id": amb.s.salas[&2], "motivo": "motivo válido" }),
            "aparelho-invalido",
        ),
        (
            json!({ "nome": "Logiq", "ae_title": "OK2", "modalidade": "U5", "sala_id": amb.s.salas[&2], "motivo": "motivo válido" }),
            "aparelho-invalido",
        ),
        (
            json!({ "nome": "Logiq", "ae_title": "OK3", "modalidade": "US", "sala_id": amb.s.salas[&2], "motivo": "x" }),
            "motivo-invalido",
        ),
        (
            json!({ "nome": "Logiq", "ae_title": "OK4", "modalidade": "US", "sala_id": Uuid::now_v7(), "motivo": "motivo válido" }),
            "sala-nao-encontrada",
        ),
    ] {
        let (_, v) = cadastrar(&amb, "ti.a", Uuid::now_v7(), corpo.clone()).await;
        assert_eq!(v["codigo"].as_str(), Some(codigo), "{corpo}");
    }
    assert_eq!(
        amb.contar("SELECT count(*) FROM agenda.aparelho").await,
        antes
    );
}

#[tokio::test]
async fn desativar_e_atomico_e_aparelho_desativado_nao_recebe_lista_de_trabalho() {
    let amb = Ambiente::novo().await;
    let ti = amb.entrar("ti.a").await;
    let (_, _, painel) = amb.pedir(amb.get("/v1/aparelhos", Some(&ti))).await;
    let sala1 = painel["aparelhos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["sala_nome"] == "Sala 1")
        .unwrap()
        .clone();
    let uri = format!("/v1/aparelhos/{}/mudancas", sala1["id"].as_str().unwrap());
    let corpo = json!({ "tipo": "desativar", "motivo": "manutenção preventiva do transdutor" });
    let (r1, r2) = tokio::join!(
        amb.pedir(amb.json(Method::POST, &uri, &ti, None, corpo.clone())),
        amb.pedir(amb.json(Method::POST, &uri, &ti, None, corpo.clone()))
    );
    let mut st = [r1.0, r2.0];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    // Sem aparelho ativo, o bipe na sala 1 não envia lista de trabalho.
    let at = amb
        .fazer_checkin("Antônio Carlos Ribeiro", "1954-06-15")
        .await;
    let bruno = amb.entrar("bruno.sales").await;
    let (sb, _, vb) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/salas/{}/bipes", amb.s.salas[&1]),
            &bruno,
            Some(Uuid::now_v7()),
            json!({ "codigo_etiqueta": at["pulseira"]["codigo"] }),
        ))
        .await;
    assert_eq!(
        (sb, vb["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("sala-sem-aparelho")),
        "{vb}"
    );
    let (sr, _, vr) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &ti,
            None,
            json!({ "tipo": "reativar", "motivo": "manutenção concluída" }),
        ))
        .await;
    assert_eq!(
        (sr, vr["ativo"].as_bool(), vr["versao"].as_i64()),
        (StatusCode::OK, Some(true), Some(3))
    );
    assert_eq!(amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo IN ('aparelho_desativar','aparelho_reativar')").await, 2);
}

#[tokio::test]
async fn painel_so_em_numeros_e_so_ti_e_gerencia() {
    let amb = Ambiente::novo().await;
    amb.fazer_checkin("Antônio Carlos Ribeiro", "1954-06-15")
        .await;
    let ti = amb.entrar("ti.a").await;
    let (st, _, p) = amb.pedir(amb.get("/v1/aparelhos", Some(&ti))).await;
    assert_eq!(st, StatusCode::OK, "{p}");
    let texto = p.to_string();
    for proibido in [
        "Antônio",
        "1954",
        "Ecocardiograma",
        "PAC-",
        "paciente_id",
        "atendimento",
    ] {
        assert!(!texto.contains(proibido), "painel vazou {proibido}");
    }
    for login in [
        "ana.souza",
        "bruno.sales",
        "medico.a",
        "carla.caixa",
        "tv.a",
        "financeiro.a",
    ] {
        let s = amb.entrar(login).await;
        let (sl, _, _) = amb.pedir(amb.get("/v1/aparelhos", Some(&s))).await;
        assert_eq!(sl, StatusCode::FORBIDDEN, "{login}");
        let (sc, _) = cadastrar(&amb, login, Uuid::now_v7(), novo(&amb, "NEGADO")).await;
        assert_eq!(sc, StatusCode::FORBIDDEN, "{login}");
    }
}
