//! Explorador de lastro (docs/21: qualquer código leva à cadeia completa; eventos imutáveis com hash
//! encadeado). Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use comum::Ambiente;
use serde_json::Value;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const NASC: &str = "1954-06-15";

async fn cadeia(amb: &Ambiente, login: &str, codigo: &str) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.get(
            &format!("/v1/lastro/cadeia?codigo={}", urlencoding(codigo)),
            Some(&s),
        ))
        .await;
    (st, v)
}

fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[tokio::test]
async fn qualquer_codigo_leva_a_mesma_cadeia_integra_e_a_leitura_fica_no_lastro() {
    let amb = Ambiente::novo().await;
    let (atd, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let pelo_atendimento = cadeia(&amb, "patricia.lemos", atd["codigo"].as_str().unwrap()).await;
    assert_eq!(pelo_atendimento.0, StatusCode::OK, "{}", pelo_atendimento.1);
    let c = pelo_atendimento.1;
    assert_eq!(
        (
            c["paciente"]["nome"].as_str(),
            c["paciente"]["nascimento"].as_str()
        ),
        (Some(ANTONIO), Some(NASC))
    );
    let tipos: Vec<&str> = c["nos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["tipo"].as_str().unwrap())
        .collect();
    for t in ["agendamento", "atendimento", "etiqueta", "estudo", "laudo"] {
        assert!(tipos.contains(&t), "{t} em {tipos:?}");
    }
    assert_eq!(c["integra"], true);
    assert!(c["eventos"].as_array().unwrap().len() >= 3);
    assert!(
        c["eventos"][0].get("dados").is_none(),
        "o conteúdo do evento não sai"
    );
    let codigo_laudo: String = sqlx::query_scalar("SELECT codigo FROM laudos.laudo WHERE id = $1")
        .bind(laudo)
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    for codigo in [
        atd["accession"].as_str().unwrap(),
        &codigo_laudo,
        atd["pulseira"]["codigo"].as_str().unwrap(),
    ] {
        let (st, outra) = cadeia(&amb, "patricia.lemos", codigo).await;
        assert_eq!(
            (st, &outra["atendimento_codigo"]),
            (StatusCode::OK, &c["atendimento_codigo"]),
            "{codigo}"
        );
    }
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'cadeia_lida'")
            .await,
        4
    );
}

#[tokio::test]
async fn evento_adulterado_aparece_como_nao_integro() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(ANTONIO, NASC).await;
    // Simula alguém com acesso de superusuário alterando um evento por fora da aplicação.
    sqlx::query("ALTER TABLE lastro.evento DISABLE TRIGGER evento_imutavel")
        .execute(&amb.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE lastro.evento SET dados = '{\"adulterado\": true}' WHERE sequencia = (SELECT min(sequencia) FROM lastro.evento WHERE registro_codigo = $1)")
        .bind(atd["codigo"].as_str().unwrap())
        .execute(&amb.admin)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE lastro.evento ENABLE TRIGGER evento_imutavel")
        .execute(&amb.admin)
        .await
        .unwrap();
    let (st, c) = cadeia(&amb, "patricia.lemos", atd["codigo"].as_str().unwrap()).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(c["integra"], false);
    assert!(c["eventos"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["integro"] == false));
}

#[tokio::test]
async fn sem_oraculo_e_so_gerencia_ou_auditor() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(ANTONIO, NASC).await;
    let codigo = atd["codigo"].as_str().unwrap();
    assert_eq!(
        cadeia(&amb, "ana.souza", codigo).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        cadeia(&amb, "medico.a", codigo).await.0,
        StatusCode::FORBIDDEN
    );
    let (s1, e1) = cadeia(&amb, "patricia.lemos", "ATD-2099-999999").await;
    let (s2, e2) = cadeia(&amb, "patricia.lemos", "xx").await;
    assert_eq!(s1, StatusCode::NOT_FOUND);
    assert_eq!(s2, StatusCode::UNPROCESSABLE_ENTITY);
    let _ = (e1, e2);
}
