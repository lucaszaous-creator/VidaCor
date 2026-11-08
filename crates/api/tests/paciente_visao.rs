//! Paciente 360 (Paciente.dc.html): visão geral com identificação, próximos e histórico; CPF
//! mascarado; leitura auditada; outra unidade e perfis sem atendimento não veem. Rota real e
//! PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use comum::Ambiente;

#[tokio::test]
async fn visao_traz_identificacao_mascarada_e_agendamentos_e_fica_auditada() {
    let amb = Ambiente::novo().await;
    let id = amb.s.pacientes["Antônio Carlos Ribeiro"];
    sqlx::query(
        "UPDATE identidade.paciente SET cpf = '52998224725', celular = '21900000017' WHERE id = $1",
    )
    .bind(id)
    .execute(&amb.admin)
    .await
    .unwrap();
    let ana = amb.entrar("ana.souza").await;
    let (st, resp, v) = amb
        .pedir(amb.get(&format!("/v1/pacientes/{id}/visao"), Some(&ana)))
        .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(resp.headers()["cache-control"], "no-store");
    assert_eq!(v["identificacao"]["nome"], "Antônio Carlos Ribeiro");
    assert_eq!(v["identificacao"]["cpf_mascarado"], "***.***.***-25");
    assert!(
        !v.to_string().contains("52998224725"),
        "CPF inteiro não sai"
    );
    let total = v["proximos"].as_array().unwrap().len() + v["historico"].as_array().unwrap().len();
    assert!(total >= 1, "o agendamento de hoje aparece: {v}");
    assert_eq!(
        amb.contar("SELECT count(*) FROM auditoria.acesso WHERE recurso = 'paciente.visao'")
            .await,
        1
    );
    // Médico e técnico também veem; perfis sem atendimento e outra unidade, não.
    for login in ["medico.a", "bruno.sales", "patricia.lemos"] {
        let s = amb.entrar(login).await;
        assert_eq!(
            amb.pedir(amb.get(&format!("/v1/pacientes/{id}/visao"), Some(&s)))
                .await
                .0,
            StatusCode::OK,
            "{login}"
        );
    }
    for login in [
        "carla.caixa",
        "financeiro.a",
        "ti.a",
        "tv.a",
        "rita.faturamento",
    ] {
        let s = amb.entrar(login).await;
        assert_eq!(
            amb.pedir(amb.get(&format!("/v1/pacientes/{id}/visao"), Some(&s)))
                .await
                .0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    let b = amb.entrar("recepcao.b").await;
    let (sb, _, vb) = amb
        .pedir(amb.get(&format!("/v1/pacientes/{id}/visao"), Some(&b)))
        .await;
    assert_eq!(
        (sb, vb["codigo"].as_str()),
        (StatusCode::NOT_FOUND, Some("paciente-nao-encontrado"))
    );
}
