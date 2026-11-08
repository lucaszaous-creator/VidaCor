//! Fluxo de laudos (Fluxo-Laudos.dc.html): quadro por etapa e prazo só com metadados para a
//! coordenação; o texto do laudo nunca sai; outros perfis e o banco recusam. Rota real e PostgreSQL
//! real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use comum::Ambiente;

#[tokio::test]
async fn coordenacao_ve_etapas_e_prazos_sem_texto_e_os_demais_nao() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo("Maria Helena Duarte", "1968-03-12").await;
    // Texto marcador no rascunho: não pode aparecer no quadro.
    sqlx::query("UPDATE laudos.versao SET texto = 'TEXTO-CLINICO-SIGILOSO' WHERE laudo_id = $1")
        .bind(laudo)
        .execute(&amb.admin)
        .await
        .ok();
    // Estudo recebido há 30 h: o laudo aberto está atrasado (prazo de 24 h).
    sqlx::query("UPDATE exames.estudo SET recebido_em = now() - interval '30 hours' WHERE id = (SELECT estudo_id FROM laudos.laudo WHERE id = $1)").bind(laudo).execute(&amb.admin).await.unwrap();
    let g = amb.entrar("patricia.lemos").await;
    let (st, resp, v) = amb.pedir(amb.get("/v1/laudos/fluxo", Some(&g))).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(resp.headers()["cache-control"], "no-store");
    assert!(
        !v.to_string().contains("TEXTO-CLINICO-SIGILOSO"),
        "texto de laudo vazou"
    );
    let cartao = v["cartoes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["laudo_id"] == laudo.to_string())
        .unwrap()
        .clone();
    assert_eq!(
        (cartao["etapa"].as_str(), cartao["atrasado"].as_bool()),
        (Some("a_laudar"), Some(true))
    );
    assert_eq!(cartao["paciente_nome"], "Maria Helena Duarte");
    assert_eq!(v["atrasados"].as_i64(), Some(1));
    let a_laudar = v["etapas"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["etapa"] == "a_laudar")
        .unwrap()["quantidade"]
        .as_i64();
    assert_eq!(a_laudar, Some(1));
    for login in [
        "medico.a",
        "ana.souza",
        "ti.a",
        "carla.caixa",
        "financeiro.a",
        "tv.a",
    ] {
        let s = amb.entrar(login).await;
        assert_eq!(
            amb.pedir(amb.get("/v1/laudos/fluxo", Some(&s))).await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    // Recepção de outra unidade também não abre o quadro.
    let b = amb.entrar("recepcao.b").await;
    assert_eq!(
        amb.pedir(amb.get("/v1/laudos/fluxo", Some(&b))).await.0,
        StatusCode::FORBIDDEN
    );
}
