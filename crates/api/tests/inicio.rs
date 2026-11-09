//! Início (Main.dc.html): números do dia por perfil, laudos só como contagem, caixa só para a
//! gerência, outra unidade não soma. Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use comum::Ambiente;

#[tokio::test]
async fn inicio_conta_o_dia_e_esconde_o_caixa_de_quem_nao_e_gerencia() {
    let amb = Ambiente::novo().await;
    amb.fazer_checkin("Antônio Carlos Ribeiro", "1954-06-15")
        .await;
    amb.ate_o_laudo("Maria Helena Duarte", "1968-03-12").await;
    let ana = amb.entrar("ana.souza").await;
    let (st, resp, v) = amb.pedir(amb.get("/v1/inicio", Some(&ana))).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(resp.headers()["cache-control"], "no-store");
    let agendados: i64 = amb.contar("SELECT count(*) FROM agenda.agendamento WHERE situacao NOT IN ('cancelado','expirada') AND (inicio AT TIME ZONE 'America/Sao_Paulo')::date = (now() AT TIME ZONE 'America/Sao_Paulo')::date").await;
    assert_eq!(v["exames_agendados"].as_i64(), Some(agendados));
    assert!(v["aguardando_agora"].as_i64().unwrap() >= 1, "{v}");
    assert_eq!(
        v["laudos_a_liberar"].as_i64(),
        Some(1),
        "o laudo em rascunho conta, sem conteúdo"
    );
    assert!(v["caixa"].is_null(), "recepção não vê o caixa do dia");
    assert!(
        !v.to_string().contains("texto"),
        "nada de conteúdo de laudo"
    );
    assert!(v["ocupacao"]
        .as_array()
        .unwrap()
        .iter()
        .all(|o| o["percentual"].as_i64().unwrap() <= 100));
    let g = amb.entrar("patricia.lemos").await;
    let (_, _, vg) = amb.pedir(amb.get("/v1/inicio", Some(&g))).await;
    assert_eq!(vg["caixa"]["recebido_centavos"].as_i64(), Some(0));
    for login in [
        "carla.caixa",
        "medico.a",
        "bruno.sales",
        "ti.a",
        "tv.a",
        "financeiro.a",
    ] {
        let s = amb.entrar(login).await;
        assert_eq!(
            amb.pedir(amb.get("/v1/inicio", Some(&s))).await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    let b = amb.entrar("recepcao.b").await;
    let (_, _, vb) = amb.pedir(amb.get("/v1/inicio", Some(&b))).await;
    assert_eq!(
        (
            vb["exames_agendados"].as_i64(),
            vb["laudos_a_liberar"].as_i64()
        ),
        (Some(0), Some(0)),
        "outra unidade não soma"
    );
}
