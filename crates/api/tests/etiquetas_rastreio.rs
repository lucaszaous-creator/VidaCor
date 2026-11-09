//! Etiquetas e rastreio (Etiquetas.dc.html, docs/20): painel do dia com contadores e divergências,
//! histórico da etiqueta montado do lastro, resolução da divergência de sala e invalidação com motivo,
//! atômicas (2 requisições simultâneas), sem duplicar ao repetir, sem apagar e com acesso indevido
//! recusado. Bipe real na sala, rota real e PostgreSQL real (G4). Frases FC-ETR-01 a 06 (proposta).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

async fn bipar(amb: &Ambiente, tecnico: &str, sala: i32, etiqueta: &str) -> (StatusCode, Value) {
    let s = amb.entrar(tecnico).await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/salas/{}/bipes", amb.s.salas[&sala]),
            &s,
            Some(Uuid::now_v7()),
            json!({ "codigo_etiqueta": etiqueta }),
        ))
        .await;
    (st, c)
}

async fn painel(amb: &Ambiente, login: &str, consulta: &str) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.get(&format!("/v1/etiquetas{consulta}"), Some(&s)))
        .await;
    (st, v)
}

fn pulseira(atd: &Value) -> String {
    atd["pulseira"]["codigo"].as_str().unwrap().to_owned()
}

fn linha<'a>(p: &'a Value, codigo: &str) -> &'a Value {
    p["etiquetas"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["codigo"] == codigo)
        .unwrap()
}

/// Maria bipada na sala certa (Sala 1) e Juliana, da Sala 2, bipada por engano na Sala 1.
async fn manha(amb: &Ambiente) -> (String, String) {
    let maria = pulseira(&amb.fazer_checkin("Maria Helena Duarte", "1968-03-12").await);
    let juliana = pulseira(
        &amb.fazer_checkin("Juliana Sampaio Rocha", "1993-04-12")
            .await,
    );
    assert_eq!(bipar(amb, "bruno.sales", 1, &maria).await.0, StatusCode::OK);
    let (st, v) = bipar(amb, "bruno.sales", 1, &juliana).await;
    assert_eq!(st, StatusCode::CONFLICT, "{v}");
    (maria, juliana)
}

#[tokio::test]
async fn painel_do_dia_mostra_contadores_estado_e_divergencia_aberta() {
    let amb = Ambiente::novo().await;
    let (maria, juliana) = manha(&amb).await;
    // FC-ETR-01: contadores e estado de cada etiqueta vêm do banco e do lastro.
    let (st, p) = painel(&amb, "ana.souza", "").await;
    assert_eq!(st, StatusCode::OK, "{p}");
    assert_eq!(
        (
            p["impressas"].as_i64(),
            p["bipadas"].as_i64(),
            p["divergencias_abertas"].as_i64()
        ),
        (Some(2), Some(1), Some(1))
    );
    let m = linha(&p, &maria);
    assert_eq!(m["estado"], "em_uso");
    assert_eq!(m["paciente"]["nome"], "Maria Helena Duarte");
    assert_eq!(m["paciente"]["nascimento"], "1968-03-12");
    assert!(m["ultimo_evento"]["descricao"]
        .as_str()
        .unwrap()
        .starts_with("Bipada na Sala 1"));
    let j = linha(&p, &juliana);
    assert_eq!(j["estado"], "divergencia");
    assert_eq!(j["sala"], "Sala 2");
    // FC-ETR-02: a divergência aberta diz a sala bipada e a certa, com os dois identificadores.
    let d = &p["divergencias"][0];
    assert_eq!(d["etiqueta"], juliana);
    assert_eq!(
        (d["sala_bipada"].as_str(), d["sala_correta"].as_str()),
        (Some("Sala 1"), Some("Sala 2"))
    );
    assert_eq!(d["paciente"]["nome"], "Juliana Sampaio Rocha");
    // Histórico da etiqueta montado do lastro, na ordem.
    let ana = amb.entrar("ana.souza").await;
    let (st, _, h) = amb
        .pedir(amb.get(&format!("/v1/etiquetas/{juliana}"), Some(&ana)))
        .await;
    assert_eq!(st, StatusCode::OK, "{h}");
    let tipos: Vec<&str> = h["eventos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["tipo"].as_str().unwrap())
        .collect();
    assert_eq!(tipos, ["impressa", "bloqueada"]);
    assert_eq!(
        h["eventos"][1]["descricao"],
        "Bipada na sala errada (Sala 1): exame bloqueado, nada enviado ao aparelho"
    );
    assert_eq!(h["eventos"][1]["por"], "Bruno Sales");
    // Busca por código bipado (minúsculo), por nome sem acento e pelo número do exame.
    for q in [juliana.to_lowercase(), "juliana sampaio".to_owned()] {
        let (_, v) = painel(&amb, "ana.souza", &format!("?q={}", q.replace(' ', "%20"))).await;
        assert_eq!(v["etiquetas"].as_array().unwrap().len(), 1, "{q}");
    }
    let (st, _) = painel(&amb, "ana.souza", "?q=ab").await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn divergencia_resolvida_uma_vez_e_sem_apagar() {
    let amb = Ambiente::novo().await;
    let (_, juliana) = manha(&amb).await;
    let (_, p) = painel(&amb, "bruno.sales", "").await;
    let id = p["divergencias"][0]["id"].as_str().unwrap().to_owned();
    let uri = format!("/v1/divergencias-de-sala/{id}/resolucao");
    let bruno = amb.entrar("bruno.sales").await;
    // Limite: "outro" sem explicação é recusado.
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &bruno,
            Some(Uuid::now_v7()),
            json!({ "resolucao": "outro" }),
        ))
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    // FC-ETR-03: duas resoluções ao mesmo tempo, só uma vale (G2).
    let ana = amb.entrar("ana.souza").await;
    let corpo = json!({ "resolucao": "encaminhado" });
    let chave = Uuid::now_v7();
    let (a, b) = tokio::join!(
        amb.pedir(amb.json(Method::POST, &uri, &bruno, Some(chave), corpo.clone())),
        amb.pedir(amb.json(
            Method::POST,
            &uri,
            &ana,
            Some(Uuid::now_v7()),
            corpo.clone()
        ))
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    // Repetir com a mesma chave devolve a mesma resposta (G3), sem novo evento.
    let (st, _, r) = amb
        .pedir(amb.json(Method::POST, &uri, &bruno, Some(chave), corpo))
        .await;
    if a.0 == StatusCode::OK {
        assert_eq!(
            (st, r["descricao"].as_str()),
            (StatusCode::OK, Some("paciente encaminhado à Sala 2"))
        );
    }
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'divergencia_resolvida'")
            .await,
        1
    );
    let (_, p) = painel(&amb, "ana.souza", "").await;
    assert_eq!(p["divergencias_abertas"], 0);
    assert_eq!(linha(&p, &juliana)["estado"], "impressa");
    let (_, _, h) = amb
        .pedir(amb.get(&format!("/v1/etiquetas/{juliana}"), Some(&ana)))
        .await;
    assert_eq!(
        h["eventos"].as_array().unwrap().last().unwrap()["descricao"],
        "Divergência resolvida: paciente encaminhado à Sala 2"
    );
    // Resolvida não volta a abrir nem se apaga, nem pelo administrador do banco.
    assert!(
        sqlx::query("UPDATE recepcao.divergencia SET situacao = 'aberta'")
            .execute(&amb.admin)
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM recepcao.divergencia")
        .execute(&amb.admin)
        .await
        .is_err());
    // A pulseira de Juliana continua valendo na sala certa? A Sala 2 não tem técnico lotado nesta
    // semente; o bipe certo da Sala 2 é coberto em recepcao_sala.rs. Aqui basta: a etiqueta segue válida.
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.etiqueta WHERE situacao = 'valida'")
            .await,
        2
    );
}

#[tokio::test]
async fn invalidacao_com_motivo_atomica_e_bipe_depois_bloqueia() {
    let amb = Ambiente::novo().await;
    let maria = pulseira(&amb.fazer_checkin("Maria Helena Duarte", "1968-03-12").await);
    let uri = format!("/v1/etiquetas/{maria}/invalidacao");
    let ana = amb.entrar("ana.souza").await;
    // Limite: motivo curto; técnico não invalida (só a recepção, como a reimpressão).
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &ana,
            Some(Uuid::now_v7()),
            json!({ "motivo": "x" }),
        ))
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let bruno = amb.entrar("bruno.sales").await;
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &bruno,
            Some(Uuid::now_v7()),
            json!({ "motivo": "paciente desistiu do exame" }),
        ))
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // FC-ETR-04: duas invalidações ao mesmo tempo, só uma tem efeito (G2).
    let corpo = json!({ "motivo": "paciente desistiu do exame" });
    let (a, b) = tokio::join!(
        amb.pedir(amb.json(
            Method::POST,
            &uri,
            &ana,
            Some(Uuid::now_v7()),
            corpo.clone()
        )),
        amb.pedir(amb.json(
            Method::POST,
            &uri,
            &ana,
            Some(Uuid::now_v7()),
            corpo.clone()
        ))
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'etiqueta_invalidada'")
            .await,
        1
    );
    // Bipada depois, a sala bloqueia (docs/20).
    let (st, v) = bipar(&amb, "bruno.sales", 1, &maria).await;
    assert_eq!(
        (st, v["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("etiqueta-substituida"))
    );
    let (_, p) = painel(&amb, "ana.souza", "").await;
    let m = linha(&p, &maria);
    assert_eq!(
        (m["estado"].as_str(), m["motivo_invalidacao"].as_str()),
        (Some("invalidada"), Some("paciente desistiu do exame"))
    );
    assert_eq!(p["invalidadas"], 1);
    // Invalidada não volta a valer nem se apaga.
    assert!(
        sqlx::query("UPDATE recepcao.etiqueta SET situacao = 'valida'")
            .execute(&amb.admin)
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM recepcao.etiqueta")
        .execute(&amb.admin)
        .await
        .is_err());
    // Etiqueta que não existe: 404.
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/etiquetas/ETQ-2026-999999/invalidacao",
            &ana,
            Some(Uuid::now_v7()),
            corpo,
        ))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn reimpressao_aparece_no_historico_das_duas_etiquetas() {
    let amb = Ambiente::novo().await;
    let maria = pulseira(&amb.fazer_checkin("Maria Helena Duarte", "1968-03-12").await);
    let ana = amb.entrar("ana.souza").await;
    let (st, _, r) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/etiquetas/{maria}/reimpressao"),
            &ana,
            Some(Uuid::now_v7()),
            json!({ "motivo": "rasgou" }),
        ))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    let nova = r["nova"]["codigo"].as_str().unwrap();
    // FC-ETR-05: a nova diz qual substitui; a antiga diz o motivo e a nova.
    let (_, p) = painel(&amb, "ana.souza", "").await;
    assert_eq!(linha(&p, nova)["substitui"], maria);
    assert_eq!(linha(&p, &maria)["substituida_por"], nova);
    assert_eq!(p["reimpressoes"], 1);
    let (_, _, h) = amb
        .pedir(amb.get(&format!("/v1/etiquetas/{maria}"), Some(&ana)))
        .await;
    assert_eq!(
        h["eventos"][1]["descricao"],
        format!("Invalidada por reimpressão, motivo: rasgou; nova etiqueta {nova}")
    );
}

#[tokio::test]
async fn acesso_indevido_e_outra_unidade() {
    let amb = Ambiente::novo().await;
    let (maria, _) = manha(&amb).await;
    // FC-ETR-06: caixa, financeiro, TI, faturamento e TV não abrem o painel nem o histórico.
    for login in [
        "carla.caixa",
        "financeiro.a",
        "ti.a",
        "rita.faturamento",
        "tv.a",
    ] {
        assert_eq!(
            painel(&amb, login, "").await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
        let s = amb.entrar(login).await;
        let (st, _, _) = amb
            .pedir(amb.get(&format!("/v1/etiquetas/{maria}"), Some(&s)))
            .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    // Caixa e médico não resolvem divergência.
    let (_, p) = painel(&amb, "ana.souza", "").await;
    let uri = format!(
        "/v1/divergencias-de-sala/{}/resolucao",
        p["divergencias"][0]["id"].as_str().unwrap()
    );
    for login in ["carla.caixa", "medico.a"] {
        let s = amb.entrar(login).await;
        let (st, _, _) = amb
            .pedir(amb.json(
                Method::POST,
                &uri,
                &s,
                Some(Uuid::now_v7()),
                json!({ "resolucao": "encaminhado" }),
            ))
            .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    // A recepção da outra unidade não vê as etiquetas do Centro nem resolve a divergência delas.
    let (st, p) = painel(&amb, "recepcao.b", "").await;
    assert_eq!(
        (
            st,
            p["etiquetas"].as_array().unwrap().len(),
            p["divergencias_abertas"].as_i64()
        ),
        (StatusCode::OK, 0, Some(0))
    );
    let rb = amb.entrar("recepcao.b").await;
    let (st, _, _) = amb
        .pedir(amb.get(&format!("/v1/etiquetas/{maria}"), Some(&rb)))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &rb,
            Some(Uuid::now_v7()),
            json!({ "resolucao": "encaminhado" }),
        ))
        .await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/etiquetas/{maria}/invalidacao"),
            &rb,
            Some(Uuid::now_v7()),
            json!({ "motivo": "tentativa de outra unidade" }),
        ))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.divergencia WHERE situacao = 'aberta'")
            .await,
        1
    );
}
