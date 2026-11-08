//! Busca global (Busca-Global.dc.html): acha por nome, nascimento, celular e código; cada perfil vê só
//! o que já pode ler (RLS); outra unidade não aparece; sem oráculo. Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use comum::Ambiente;
use serde_json::Value;

async fn buscar(amb: &Ambiente, login: &str, q: &str) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let uri = format!("/v1/busca?q={}", urlencoding(q));
    let (st, _, v) = amb.pedir(amb.get(&uri, Some(&s))).await;
    (st, v)
}

fn urlencoding(q: &str) -> String {
    q.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b == b'-' {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[tokio::test]
async fn acha_por_nome_nascimento_celular_e_codigo_conforme_o_perfil() {
    let amb = Ambiente::novo().await;
    let atd = amb
        .fazer_checkin("Antônio Carlos Ribeiro", "1954-06-15")
        .await;
    sqlx::query(
        "UPDATE identidade.paciente SET celular = '21900000017' WHERE nome = 'Maria Helena Duarte'",
    )
    .execute(&amb.admin)
    .await
    .unwrap();
    let (st, v) = buscar(&amb, "ana.souza", "Antônio").await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v[0]["tipo"], "paciente");
    assert!(v[0]["destino"]
        .as_str()
        .unwrap()
        .starts_with("/pacientes?id="));
    let (_, v) = buscar(&amb, "ana.souza", "15/06/1954").await;
    assert_eq!(v[0]["titulo"], "Antônio Carlos Ribeiro");
    let (_, v) = buscar(&amb, "ana.souza", "(21) 90000-0017").await;
    assert_eq!(v[0]["titulo"], "Maria Helena Duarte");
    let etq = atd["pulseira"]["codigo"].as_str().unwrap().to_lowercase();
    let (_, v) = buscar(&amb, "ana.souza", &etq).await;
    assert_eq!(v[0]["tipo"], "etiqueta", "{v}");
    // Caixa recebe pela tela própria: a busca não traz paciente para ele (mínimo necessário) e não diz que existe.
    let (sc, vc) = buscar(&amb, "carla.caixa", "Antônio").await;
    assert_eq!((sc, vc.as_array().map(Vec::len)), (StatusCode::OK, Some(0)));
    // Outra unidade não vê.
    let (_, vb) = buscar(&amb, "recepcao.b", "Antônio").await;
    assert_eq!(vb.as_array().map(Vec::len), Some(0));
    // TV não busca; termo curto é recusado.
    assert_eq!(
        buscar(&amb, "tv.a", "Antônio").await.0,
        StatusCode::FORBIDDEN
    );
    let (s2, v2) = buscar(&amb, "ana.souza", "an").await;
    assert_eq!(
        (s2, v2["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("busca-invalida"))
    );
}
