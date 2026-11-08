//! Exame externo (docs/22 E3): só leitura, nunca vira laudo da VidaCor, identificação original
//! guardada, conferência dos dois identificadores. Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

const VERA: &str = "Vera Lúcia Antunes";

fn importacao(nome: &str, nasc: &str, uid: &str) -> Value {
    json!({ "instituicao": "Hospital São Lucas", "origem": "link", "nome_na_origem": nome, "nascimento_na_origem": nasc, "id_na_origem": "HSL-552190",
            "documento_conferido": true,
            "estudos": [{ "study_uid": uid, "descricao": "TC de crânio", "modalidade": "CT", "data_estudo": "2026-05-10", "imagens": 240, "sha256": "a".repeat(64) }] })
}

async fn importar(amb: &Ambiente, login: &str, chave: Uuid, corpo: Value) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let uri = format!("/v1/pacientes/{}/exames-externos", amb.s.pacientes[VERA]);
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, &uri, &s, Some(chave), corpo))
        .await;
    (st, v)
}

#[tokio::test]
async fn importa_com_identidade_conferida_guarda_o_original_e_e_so_leitura() {
    let amb = Ambiente::novo().await;
    let chave = Uuid::now_v7();
    let corpo = importacao("ANTUNES^VERA^L", "1957-09-18", "1.2.3.4.5");
    let (st, e) = importar(&amb, "ana.souza", chave, corpo.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{e}");
    assert_eq!(
        (e["nome_na_origem"].as_str(), e["somente_leitura"].as_bool()),
        (Some("ANTUNES^VERA^L"), Some(true))
    );
    let (_, e2) = importar(&amb, "ana.souza", chave, corpo.clone()).await;
    assert_eq!(e2["id"], e["id"], "repetir não duplica");
    let (s3, e3) = importar(&amb, "ana.souza", Uuid::now_v7(), corpo).await;
    assert_eq!(
        (s3, e3["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("estudo-ja-importado"))
    );
    // Só leitura e nunca laudo: não há como alterar, e nenhum laudo aponta para ele.
    assert!(sqlx::query("UPDATE exames.externo SET instituicao = 'X'")
        .execute(&amb.admin)
        .await
        .is_err());
    assert!(sqlx::query("DELETE FROM exames.externo_estudo")
        .execute(&amb.admin)
        .await
        .is_err());
    assert_eq!(amb.contar("SELECT count(*) FROM laudos.laudo").await, 0);
    let medico = amb.entrar("medico.a").await;
    let (sl, _, lista) = amb
        .pedir(amb.get(
            &format!("/v1/pacientes/{}/exames-externos", amb.s.pacientes[VERA]),
            Some(&medico),
        ))
        .await;
    assert_eq!(
        (sl, lista[0]["estudos"][0]["imagens"].as_i64()),
        (StatusCode::OK, Some(240))
    );
}

#[tokio::test]
async fn nascimento_diferente_ou_nome_incompativel_nao_entram() {
    let amb = Ambiente::novo().await;
    let (s1, e1) = importar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        importacao("ANTUNES^VERA", "1957-09-19", "1.2.9"),
    )
    .await;
    assert_eq!(
        (s1, e1["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("nascimento-diferente"))
    );
    let (s2, e2) = importar(
        &amb,
        "ana.souza",
        Uuid::now_v7(),
        importacao("SOUZA^VERA", "1957-09-18", "1.2.9"),
    )
    .await;
    assert_eq!(
        (s2, e2["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("nome-incompativel"))
    );
    let mut sem_doc = importacao("ANTUNES^VERA", "1957-09-18", "1.2.9");
    sem_doc["documento_conferido"] = json!(false);
    assert_eq!(
        importar(&amb, "ana.souza", Uuid::now_v7(), sem_doc).await.1["codigo"],
        "documento-nao-conferido"
    );
    let mut sem_hash = importacao("ANTUNES^VERA", "1957-09-18", "1.2.9");
    sem_hash["estudos"][0]["sha256"] = json!("abc");
    assert_eq!(
        importar(&amb, "ana.souza", Uuid::now_v7(), sem_hash)
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(amb.contar("SELECT count(*) FROM exames.externo").await, 0);
}

#[tokio::test]
async fn perfis_e_outra_unidade() {
    let amb = Ambiente::novo().await;
    for login in ["carla.caixa", "medico.a", "financeiro.a", "ti.a"] {
        let (st, _) = importar(
            &amb,
            login,
            Uuid::now_v7(),
            importacao("ANTUNES^VERA", "1957-09-18", "1.2.7"),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    let (st, _) = importar(
        &amb,
        "recepcao.b",
        Uuid::now_v7(),
        importacao("ANTUNES^VERA", "1957-09-18", "1.2.7"),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "outra unidade não acha o paciente"
    );
}
