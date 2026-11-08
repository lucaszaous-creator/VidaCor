//! Abrir imagens do estudo (Visualizador.dc.html): só estudo conciliado, médico com vínculo ou técnico
//! lotado na sala, cada tentativa auditada. Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use comum::{Ambiente, UNIDADE_A};
use uuid::Uuid;

async fn abrir(amb: &Ambiente, login: &str, estudo: Uuid) -> (StatusCode, serde_json::Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.get(&format!("/v1/estudos/{estudo}/visualizacao"), Some(&s)))
        .await;
    (st, v)
}

#[tokio::test]
async fn medico_com_vinculo_e_tecnico_da_sala_abrem_os_outros_nao_e_tudo_e_auditado() {
    let amb = Ambiente::novo().await;
    let (atd, _) = amb
        .ate_o_laudo("Antônio Carlos Ribeiro", "1954-06-15")
        .await;
    let estudo: Uuid = sqlx::query_scalar("SELECT id FROM exames.estudo WHERE atendimento_id = $1")
        .bind(Uuid::parse_str(atd["id"].as_str().unwrap()).unwrap())
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    let (st, v) = abrir(&amb, "medico.a", estudo).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let url = v["url"].as_str().unwrap();
    assert!(
        url.starts_with("http://127.0.0.1:3000/viewer?StudyInstanceUIDs=1.2.826."),
        "{url}"
    );
    assert!(
        !url.contains("Ant") && !url.contains("1954"),
        "nada de dado pessoal na URL"
    );
    assert_eq!(abrir(&amb, "bruno.sales", estudo).await.0, StatusCode::OK);
    for login in ["cecilia.souto", "livia.costa"] {
        let (s, e) = abrir(&amb, login, estudo).await;
        assert_eq!(
            (s, e["codigo"].as_str()),
            (StatusCode::FORBIDDEN, Some("sem-vinculo")),
            "{login}"
        );
    }
    for login in ["ana.souza", "patricia.lemos", "ti.a", "carla.caixa", "tv.a"] {
        assert_eq!(
            abrir(&amb, login, estudo).await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    assert_eq!(amb.contar("SELECT count(*) FROM auditoria.acesso WHERE recurso = 'estudo.imagens' AND resultado = 'permitido'").await, 2);
    assert_eq!(amb.contar("SELECT count(*) FROM auditoria.acesso WHERE recurso = 'estudo.imagens' AND resultado = 'negado'").await, 2);
}

#[tokio::test]
async fn estudo_sem_paciente_conciliado_nunca_e_aberto() {
    let amb = Ambiente::novo().await;
    let estudo = vidacor_api::orthanc::EstudoOrthanc {
        orthanc_id: "abcd1234-0000".to_owned(),
        study_uid: "1.2.826.0.1.999".to_owned(),
        accession: None,
        patient_id: None,
        imagens: 3,
    };
    vidacor_api::outbox::registrar_estudo(&amb.estado, UNIDADE_A, &estudo)
        .await
        .unwrap();
    let id: Uuid =
        sqlx::query_scalar("SELECT id FROM exames.estudo WHERE study_uid = '1.2.826.0.1.999'")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    let (st, v) = abrir(&amb, "medico.a", id).await;
    assert_eq!(
        (st, v["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("estudo-sem-paciente"))
    );
    let (sn, _) = abrir(&amb, "medico.a", Uuid::now_v7()).await;
    assert_eq!(sn, StatusCode::NOT_FOUND);
}
