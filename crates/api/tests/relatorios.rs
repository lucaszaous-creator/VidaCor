//! Relatórios e exportação (docs/27 FC-REL-01 a 07, proposta aguardando aprovação;
//! DEC-2026-11-09-150 a 154): rota HTTP real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::body::Body;
use axum::http::{Method, StatusCode};
use comum::{Ambiente, Sessao};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;

async fn catalogo(amb: &Ambiente, login: &str) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, c) = amb.pedir(amb.get("/v1/relatorios", Some(&s))).await;
    (st, c)
}

/// Exporta e devolve status, cabeçalhos e o corpo bruto (CSV).
async fn exportar(
    amb: &Ambiente,
    s: &Sessao,
    id: &str,
    corpo: Value,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let req = amb.json(
        Method::POST,
        &format!("/v1/relatorios/{id}/exportacao"),
        s,
        None,
        corpo,
    );
    let r = amb.app.clone().oneshot(req).await.unwrap();
    let st = r.status();
    let h = r.headers().clone();
    let b = r.into_body().collect().await.unwrap().to_bytes().to_vec();
    (st, h, b)
}

async fn marcar_falta(amb: &Ambiente, agendamento: Uuid) {
    sqlx::query("UPDATE agenda.agendamento SET situacao = 'faltou' WHERE id = $1")
        .bind(agendamento)
        .execute(&amb.admin)
        .await
        .unwrap();
}

fn ids(c: &Value) -> Vec<String> {
    c.as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn fc_rel_01_cada_perfil_ve_so_os_seus_relatorios() {
    let amb = Ambiente::novo().await;
    let (_, g) = catalogo(&amb, "patricia.lemos").await;
    assert_eq!(ids(&g).len(), 7);
    let (_, f) = catalogo(&amb, "rita.faturamento").await;
    assert_eq!(ids(&f), ["producao_convenio", "glosas"]);
    let (_, fin) = catalogo(&amb, "financeiro.a").await;
    assert_eq!(ids(&fin), ["financeiro"]);
    let (_, e) = catalogo(&amb, "estoque.a").await;
    assert_eq!(ids(&e), ["estoque"]);
    for login in [
        "ana.souza",
        "carla.caixa",
        "medico.a",
        "bruno.sales",
        "ti.a",
        "tv.a",
    ] {
        let (st, _) = catalogo(&amb, login).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    // Relatório fora do perfil: 403 na prévia e na exportação; inexistente: 404.
    let rita = amb.entrar("rita.faturamento").await;
    let (st, _, _) = amb
        .pedir(amb.get("/v1/relatorios/faltas", Some(&rita)))
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _, _) = exportar(&amb, &rita, "financeiro", json!({})).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _, _) = amb
        .pedir(amb.get("/v1/relatorios/inexistente", Some(&rita)))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn fc_rel_02_faltas_saem_com_codigo_nunca_com_nome() {
    let amb = Ambiente::novo().await;
    marcar_falta(&amb, amb.agendamento_maria).await;
    let s = amb.entrar("patricia.lemos").await;
    let (st, _, p) = amb.pedir(amb.get("/v1/relatorios/faltas", Some(&s))).await;
    assert_eq!(st, StatusCode::OK, "{p}");
    assert_eq!(p["dado_de_saude"], true);
    assert_eq!(p["linhas"][0]["a"], "PAC-2026-000002");
    assert_eq!(p["linhas"][0]["b"], "TUSS 40901114");
    assert_eq!(p["linhas"][0]["valor"], 1);
    assert!(!p.to_string().contains("Maria"));
    assert!(!p.to_string().contains("Ecocardiograma"));
}

#[tokio::test]
async fn fc_rel_03_exportar_pede_reautenticacao_e_nada_e_registrado_sem_ela() {
    let amb = Ambiente::novo().await;
    let s = amb.entrar("patricia.lemos").await;
    amb.expirar_reautenticacao().await;
    let (st, _, b) = exportar(&amb, &s, "producao_sala", json!({})).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let e: Value = serde_json::from_slice(&b).unwrap();
    assert_eq!(e["codigo"], "reautenticacao-necessaria");
    assert_eq!(
        amb.contar("SELECT count(*) FROM relatorios.exportacao")
            .await,
        0
    );
}

#[tokio::test]
async fn fc_rel_04_exportacao_gera_csv_com_sha256_registrado_e_lastro() {
    let amb = Ambiente::novo().await;
    marcar_falta(&amb, amb.agendamento_maria).await;
    let s = amb.entrar("patricia.lemos").await;
    let (st, h, b) = exportar(&amb, &s, "faltas", json!({})).await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(h["content-type"], "text/csv; charset=utf-8");
    assert_eq!(h["cache-control"], "no-store");
    let texto = String::from_utf8(b.clone()).unwrap();
    assert!(
        texto.starts_with("\u{feff}Paciente;Exame;Faltas\r\n"),
        "{texto}"
    );
    assert!(texto.contains("PAC-2026-000002;TUSS 40901114;1\r\n"));
    assert!(!texto.contains("Maria"));
    let sha: String = Sha256::digest(&b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect();
    assert_eq!(h["x-documento-sha256"].to_str().unwrap(), sha);
    let codigo = h["x-documento-codigo"].to_str().unwrap().to_owned();
    assert!(codigo.starts_with("EXP-"));
    let disp = h["content-disposition"].to_str().unwrap();
    assert!(disp.contains(&codigo) && !disp.contains("Maria"));
    let (conteudo, gravado): (String, String) =
        sqlx::query_as("SELECT conteudo, sha256 FROM relatorios.exportacao WHERE codigo = $1")
            .bind(&codigo)
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(conteudo, "pseudonimizado");
    assert_eq!(gravado, sha);
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'relatorio_exportado'")
            .await,
        1
    );
    // Cada pedido é uma emissão nova e honesta (código e registro próprios), como o dossiê.
    let (_, h2, _) = exportar(&amb, &s, "faltas", json!({})).await;
    assert_ne!(h2["x-documento-codigo"], h["x-documento-codigo"]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM relatorios.exportacao")
            .await,
        2
    );
}

#[tokio::test]
async fn fc_rel_05_historico_cada_um_ve_as_suas_e_a_gerencia_as_da_unidade() {
    let amb = Ambiente::novo().await;
    let rita = amb.entrar("rita.faturamento").await;
    let (st, _, _) = exportar(&amb, &rita, "glosas", json!({})).await;
    assert_eq!(st, StatusCode::OK);
    let fin = amb.entrar("financeiro.a").await;
    let (st, _, _) = exportar(&amb, &fin, "financeiro", json!({})).await;
    assert_eq!(st, StatusCode::OK);
    let (_, _, h) = amb
        .pedir(amb.get("/v1/relatorios/exportacoes", Some(&rita)))
        .await;
    assert_eq!(h.as_array().unwrap().len(), 1, "{h}");
    assert_eq!(h[0]["titulo"], "Glosas");
    assert_eq!(h[0]["quem"], "Rita Faturamento");
    let patricia = amb.entrar("patricia.lemos").await;
    let (_, _, h) = amb
        .pedir(amb.get("/v1/relatorios/exportacoes", Some(&patricia)))
        .await;
    assert_eq!(h.as_array().unwrap().len(), 2);
    // Faturamento de outra unidade não vê as exportações desta.
    let norte = amb.entrar("faturista.b").await;
    let (_, _, h) = amb
        .pedir(amb.get("/v1/relatorios/exportacoes", Some(&norte)))
        .await;
    assert_eq!(h.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn fc_rel_06_producao_por_sala_mostra_as_salas_da_unidade() {
    let amb = Ambiente::novo().await;
    let s = amb.entrar("patricia.lemos").await;
    let (st, _, p) = amb
        .pedir(amb.get("/v1/relatorios/producao_sala", Some(&s)))
        .await;
    assert_eq!(st, StatusCode::OK, "{p}");
    let salas: Vec<&str> = p["linhas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["a"].as_str().unwrap())
        .collect();
    assert_eq!(salas, ["Sala 1", "Sala 2", "Sala 6"]);
    assert_eq!(p["colunas"], json!(["Sala", "Ocupação", "Exames"]));
    // Os demais relatórios do catálogo rodam com o contexto da gerência (o RLS decide o que entra).
    for id in [
        "producao_medico",
        "producao_convenio",
        "glosas",
        "financeiro",
        "estoque",
    ] {
        let (st, _, p) = amb
            .pedir(amb.get(&format!("/v1/relatorios/{id}"), Some(&s)))
            .await;
        assert_eq!(st, StatusCode::OK, "{id}: {p}");
        assert!(p["linhas"].is_array(), "{id}");
    }
}

#[tokio::test]
async fn fc_rel_07_periodo_invalido_e_sem_login() {
    let amb = Ambiente::novo().await;
    let s = amb.entrar("patricia.lemos").await;
    for q in [
        "de=2026-11-09&ate=2026-11-01",
        "de=2024-01-01&ate=2026-11-01",
    ] {
        let (st, _, e) = amb
            .pedir(amb.get(&format!("/v1/relatorios/producao_sala?{q}"), Some(&s)))
            .await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{q}");
        assert_eq!(e["codigo"], "periodo-invalido");
    }
    let (st, _, _) = exportar(
        &amb,
        &s,
        "producao_sala",
        json!({ "de": "2026-11-09", "ate": "2026-11-01" }),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, _, _) = amb.pedir(amb.get("/v1/relatorios", None)).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let r = axum::http::Request::builder()
        .method(Method::POST)
        .uri("/v1/relatorios/producao_sala/exportacao")
        .header("content-type", "application/json")
        .header("cookie", &s.cookie)
        .body(Body::from("{}"))
        .unwrap();
    let (st, _, _) = amb.pedir(r).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "sem CSRF");
}
