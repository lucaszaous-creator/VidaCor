//! Indicadores da gerência (Indicadores.dc.html; docs/25 MVP): só agregados, sem paciente.
//! Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use comum::Ambiente;

const ANTONIO: &str = "Antônio Carlos Ribeiro";

#[tokio::test]
async fn producao_faltas_e_laudo_por_medico_sem_identificar_paciente() {
    let amb = Ambiente::novo().await;
    amb.fazer_checkin(ANTONIO, "1954-06-15").await;
    sqlx::query("UPDATE agenda.agendamento SET situacao = 'faltou', inicio = now() - interval '1 hour', fim = now() - interval '30 minutes' WHERE id = $1")
        .bind(amb.s.agendamentos["Maria Helena Duarte"])
        .execute(&amb.admin)
        .await
        .unwrap();
    let dia = amb.s.dia;
    let patricia = amb.entrar("patricia.lemos").await;
    let (st, _, i) = amb
        .pedir(amb.get(
            &format!("/v1/indicadores?de={dia}&ate={dia}"),
            Some(&patricia),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{i}");
    assert!(i["agendamentos"].as_i64().unwrap() >= 2);
    assert_eq!(i["atendimentos"], 1);
    assert_eq!(i["faltas"], 1);
    assert!(i["taxa_de_faltas"].as_f64().unwrap() > 0.0);
    assert!(i["producao"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["atendimentos"] == 1));
    let texto = i.to_string();
    assert!(
        !texto.contains(ANTONIO) && !texto.contains("1954"),
        "sem paciente: {texto}"
    );
}

#[tokio::test]
async fn periodo_invalido_e_perfis_sem_acesso() {
    let amb = Ambiente::novo().await;
    let patricia = amb.entrar("patricia.lemos").await;
    let (s1, _, _) = amb
        .pedir(amb.get(
            "/v1/indicadores?de=2026-11-10&ate=2026-11-01",
            Some(&patricia),
        ))
        .await;
    assert_eq!(s1, StatusCode::UNPROCESSABLE_ENTITY);
    let (s2, _, _) = amb
        .pedir(amb.get(
            "/v1/indicadores?de=2024-01-01&ate=2026-01-01",
            Some(&patricia),
        ))
        .await;
    assert_eq!(s2, StatusCode::UNPROCESSABLE_ENTITY, "mais de um ano");
    for login in ["ana.souza", "medico.a", "carla.caixa", "ti.a"] {
        let s = amb.entrar(login).await;
        let (st, _, _) = amb
            .pedir(amb.get("/v1/indicadores?de=2026-11-01&ate=2026-11-08", Some(&s)))
            .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    let fin = amb.entrar("financeiro.a").await;
    let (sf, _, _) = amb
        .pedir(amb.get("/v1/indicadores?de=2026-11-01&ate=2026-11-08", Some(&fin)))
        .await;
    assert_eq!(sf, StatusCode::OK);
}
