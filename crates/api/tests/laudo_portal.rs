//! Laudo, portal e acesso indevido (docs/27 FC-LAU-01 a 10 e FC-ACE-01 a 08). Rota real e
//! PostgreSQL real (G4); só o provedor de assinatura é simulado (externo).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;
use vidacor_api::outbox;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const NASC: &str = "1954-06-15";

async fn medico(
    amb: &Ambiente,
    login: &str,
    metodo: Method,
    uri: &str,
    corpo: Value,
    chave: Option<Uuid>,
) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let req = if metodo == Method::GET {
        amb.get(uri, Some(&s))
    } else {
        amb.json(metodo, uri, &s, chave, corpo)
    };
    let (st, _, c) = amb.pedir(req).await;
    (st, c)
}

async fn escrever_e_assinar(amb: &Ambiente, laudo: Uuid, texto: &str) -> (StatusCode, Value) {
    let (st, c) = medico(
        amb,
        "medico.a",
        Method::PUT,
        &format!("/v1/laudos/{laudo}/rascunho"),
        json!({ "texto": texto, "versao_numero": 1 }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    medico(
        amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/assinatura"),
        json!({ "pin": "4821" }),
        Some(Uuid::now_v7()),
    )
    .await
}

async fn processar_outbox(amb: &Ambiente) {
    let orthanc = vidacor_api::orthanc::Orthanc::novo("http://127.0.0.1:9").unwrap();
    while outbox::processar_um(&amb.estado, &orthanc).await.unwrap() {}
}

fn portal(amb: &Ambiente, cookie: &str, uri: &str) -> Request<Body> {
    let _ = amb;
    Request::builder()
        .uri(uri)
        .header("cookie", cookie)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn fc_lau_01_assinatura_grava_versao_1_com_certificado_carimbo_e_hash() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let (st, l) = escrever_e_assinar(&amb, laudo, "Ecocardiograma dentro da normalidade.").await;
    assert_eq!(st, StatusCode::OK, "{l}");
    assert_eq!(l["situacao"], "assinado");
    assert_eq!(l["versoes"][0]["situacao"], "assinada");
    assert_eq!(
        l["versoes"][0]["hash_documento"].as_str().unwrap().len(),
        64
    );
    let (cert, carimbo): (String, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT certificado, carimbo_tempo FROM laudos.versao")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert!(
        cert.contains("SEM VALOR LEGAL"),
        "assinador de desenvolvimento se identifica"
    );
    assert!(carimbo.is_some());
}

#[tokio::test]
async fn assinatura_exige_reautenticacao_e_pin_valido() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    medico(
        &amb,
        "medico.a",
        Method::PUT,
        &format!("/v1/laudos/{laudo}/rascunho"),
        json!({ "texto": "Normal.", "versao_numero": 1 }),
        None,
    )
    .await;
    let (st, c) = medico(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/assinatura"),
        json!({ "pin": "0000" }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(c["codigo"], "pin-recusado");
    let paulo = amb.entrar("medico.a").await;
    amb.expirar_reautenticacao().await;
    let (st2, _, c2) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/laudos/{laudo}/assinatura"),
            &paulo,
            Some(Uuid::now_v7()),
            json!({ "pin": "4821" }),
        ))
        .await;
    assert_eq!(st2, StatusCode::UNAUTHORIZED);
    assert_eq!(c2["codigo"], "reautenticacao-necessaria");
}

#[tokio::test]
async fn fc_lau_02_rascunho_nao_vai_ao_portal() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let (st, c) = medico(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/publicacao"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(c["codigo"], "laudo-nao-assinado");
    assert_eq!(
        amb.contar("SELECT count(*) FROM portal.publicacao").await,
        0
    );
}

#[tokio::test]
async fn fc_lau_03_assinado_vai_ao_portal_uma_vez_com_um_aviso() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    escrever_e_assinar(&amb, laudo, "Normal.").await;
    processar_outbox(&amb).await;
    // O evento é processado de novo: nada duplica.
    vidacor_api::laudos::publicar(&amb.estado, laudo, comum::UNIDADE_A)
        .await
        .unwrap();
    assert_eq!(
        amb.contar("SELECT count(*) FROM portal.publicacao").await,
        1
    );
    assert_eq!(amb.contar("SELECT count(*) FROM portal.aviso").await, 1);
    let cookie = amb.entrar_no_portal(ANTONIO).await;
    let (st, _, lista) = amb.pedir(portal(&amb, &cookie, "/v1/portal/laudos")).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(lista.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn fc_lau_04_laudo_assinado_nao_pode_ser_alterado() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    escrever_e_assinar(&amb, laudo, "Texto original assinado.").await;
    let (st, c) = medico(
        &amb,
        "medico.a",
        Method::PUT,
        &format!("/v1/laudos/{laudo}/rascunho"),
        json!({ "texto": "alterado", "versao_numero": 1 }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(c["codigo"], "laudo-assinado");
    // Nem direto no banco: o gatilho barra a alteração da versão assinada.
    let direto = sqlx::query("UPDATE laudos.versao SET texto = 'x' WHERE numero = 1")
        .execute(&amb.admin)
        .await;
    assert!(direto.is_err());
    let texto: String = sqlx::query_scalar("SELECT texto FROM laudos.versao WHERE numero = 1")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    assert_eq!(texto, "Texto original assinado.");
}

#[tokio::test]
async fn fc_lau_05_retificacao_cria_versao_2_e_o_portal_mostra_a_retificada() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    escrever_e_assinar(&amb, laudo, "Átrio esquerdo 52 mm.").await;
    processar_outbox(&amb).await;
    let (st, l) = medico(&amb, "medico.a", Method::POST, &format!("/v1/laudos/{laudo}/retificacao"),
        json!({ "texto": "Átrio esquerdo 38 mm.", "motivo": "medida do átrio esquerdo digitada errada" }), None).await;
    assert_eq!(st, StatusCode::CREATED, "{l}");
    let (sa, a) = medico(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/assinatura"),
        json!({ "pin": "4821" }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(sa, StatusCode::OK, "{a}");
    assert_eq!(a["versao_vigente"], 2);
    assert_eq!(a["versoes"][0]["situacao"], "substituida");
    assert_eq!(
        a["versoes"][1]["motivo"],
        "medida do átrio esquerdo digitada errada"
    );
    processar_outbox(&amb).await;
    let cookie = amb.entrar_no_portal(ANTONIO).await;
    let (_, _, lista) = amb.pedir(portal(&amb, &cookie, "/v1/portal/laudos")).await;
    assert_eq!(lista.as_array().unwrap().len(), 1);
    assert_eq!(lista[0]["retificado"], true);
    assert_eq!(lista[0]["texto"], "Átrio esquerdo 38 mm.");
}

#[tokio::test]
async fn fc_lau_06_correcao_sem_motivo_e_recusada() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    escrever_e_assinar(&amb, laudo, "Normal.").await;
    let (st, c) = medico(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/retificacao"),
        json!({ "texto": "Outro" }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(c["codigo"], "motivo-obrigatorio");
}

#[tokio::test]
async fn fc_lau_07_duas_assinaturas_simultaneas_geram_uma_versao_vigente() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    medico(
        &amb,
        "medico.a",
        Method::PUT,
        &format!("/v1/laudos/{laudo}/rascunho"),
        json!({ "texto": "Normal.", "versao_numero": 1 }),
        None,
    )
    .await;
    let uri = format!("/v1/laudos/{laudo}/assinatura");
    let ((s1, _), (s2, _)) = tokio::join!(
        medico(
            &amb,
            "medico.a",
            Method::POST,
            &uri,
            json!({ "pin": "4821" }),
            Some(Uuid::now_v7())
        ),
        medico(
            &amb,
            "medico.a",
            Method::POST,
            &uri,
            json!({ "pin": "4821" }),
            Some(Uuid::now_v7())
        )
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM laudos.versao WHERE situacao = 'assinada'")
            .await,
        1
    );
}

#[tokio::test]
async fn fc_lau_08_conversar_antes_segura_ate_a_liberacao() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let (st, _) = medico(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/conversar-antes"),
        json!({ "marcado": true }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    escrever_e_assinar(&amb, laudo, "Achado que exige conversa.").await;
    processar_outbox(&amb).await;
    assert_eq!(
        amb.contar("SELECT count(*) FROM portal.publicacao").await,
        0
    );
    let (sl, _) = medico(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/liberacao"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(sl, StatusCode::OK);
    processar_outbox(&amb).await;
    assert_eq!(
        amb.contar("SELECT count(*) FROM portal.publicacao").await,
        1
    );
    assert_eq!(
        amb.contar(
            "SELECT count(*) FROM laudos.laudo WHERE liberado_apos_conversa_por IS NOT NULL"
        )
        .await,
        1
    );
}

#[tokio::test]
async fn fc_lau_09_estudo_sem_conciliacao_nao_entra_na_fila() {
    let amb = Ambiente::novo().await;
    let estudo = vidacor_api::orthanc::EstudoOrthanc {
        orthanc_id: "abcd-0001".into(),
        study_uid: "1.2.3.4.5".into(),
        accession: Some("ACC-9999-999999".into()),
        imagens: 3,
    };
    outbox::registrar_estudo(&amb.estado, comum::UNIDADE_A, &estudo)
        .await
        .unwrap();
    assert_eq!(amb.contar("SELECT count(*) FROM laudos.laudo").await, 0);
    let (st, fila) = medico(
        &amb,
        "medico.a",
        Method::GET,
        "/v1/laudos/fila",
        json!({}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(fila.as_array().unwrap().len(), 0);
    // Com o estudo conciliado, o laudo aparece só na fila do médico atribuído.
    amb.ate_o_laudo(ANTONIO, NASC).await;
    let (_, fila2) = medico(
        &amb,
        "medico.a",
        Method::GET,
        "/v1/laudos/fila",
        json!({}),
        None,
    )
    .await;
    assert_eq!(fila2.as_array().unwrap().len(), 1);
    let (_, fila3) = medico(
        &amb,
        "marcos.freire",
        Method::GET,
        "/v1/laudos/fila",
        json!({}),
        None,
    )
    .await;
    assert_eq!(fila3.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn fc_lau_10_medico_sem_atribuicao_nao_assina() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    medico(
        &amb,
        "medico.a",
        Method::PUT,
        &format!("/v1/laudos/{laudo}/rascunho"),
        json!({ "texto": "Normal.", "versao_numero": 1 }),
        None,
    )
    .await;
    let (st, _) = medico(
        &amb,
        "marcos.freire",
        Method::POST,
        &format!("/v1/laudos/{laudo}/assinatura"),
        json!({ "pin": "4821" }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    assert_eq!(
        amb.contar("SELECT count(*) FROM laudos.versao WHERE situacao = 'assinada'")
            .await,
        0
    );
}

async fn dois_laudos_publicados(amb: &Ambiente) -> (Uuid, Uuid) {
    let (_, a) = amb.ate_o_laudo(ANTONIO, NASC).await;
    escrever_e_assinar(amb, a, "Laudo de Antônio.").await;
    sqlx::query("UPDATE agenda.agendamento SET medico_id = $1 WHERE codigo = 'AGD-SEED-000002'")
        .bind(amb.s.usuarios["medico.a"])
        .execute(&amb.admin)
        .await
        .unwrap();
    let (_, m) = amb.ate_o_laudo("Maria Helena Duarte", "1968-03-12").await;
    escrever_e_assinar(amb, m, "Laudo de Maria.").await;
    processar_outbox(amb).await;
    (a, m)
}

#[tokio::test]
async fn fc_ace_01_e_02_paciente_nao_ve_laudo_de_outro_e_nao_descobre_se_existe() {
    let amb = Ambiente::novo().await;
    let (laudo_antonio, _) = dois_laudos_publicados(&amb).await;
    let maria = amb.entrar_no_portal("Maria Helena Duarte").await;
    let (s1, _, c1) = amb
        .pedir(portal(
            &amb,
            &maria,
            &format!("/v1/portal/laudos/{laudo_antonio}"),
        ))
        .await;
    let (s2, _, c2) = amb
        .pedir(portal(
            &amb,
            &maria,
            &format!("/v1/portal/laudos/{}", Uuid::now_v7()),
        ))
        .await;
    assert_eq!((s1, s2), (StatusCode::NOT_FOUND, StatusCode::NOT_FOUND));
    assert_eq!(c1["codigo"], c2["codigo"]);
    assert_eq!(c1["detail"], c2["detail"]);
    assert_eq!(amb.contar("SELECT count(*) FROM auditoria.acesso WHERE quem_tipo = 'paciente' AND resultado = 'negado'").await, 2);
    let (s3, _, _) = amb.pedir(portal(&amb, &maria, "/v1/portal/laudos")).await;
    assert_eq!(s3, StatusCode::OK);
}

#[tokio::test]
async fn fc_ace_03_e_04_sem_vinculo_403_e_acesso_de_emergencia_auditado() {
    let amb = Ambiente::novo().await;
    let (laudo, _) = dois_laudos_publicados(&amb).await;
    let (st, c) = medico(
        &amb,
        "marcos.freire",
        Method::GET,
        &format!("/v1/laudos/{laudo}"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    assert_eq!(c["codigo"], "sem-vinculo-de-cuidado");
    let (se, e) = medico(
        &amb,
        "marcos.freire",
        Method::POST,
        &format!("/v1/laudos/{laudo}/acesso-emergencia"),
        json!({ "justificativa": "paciente em atendimento de urgência no pronto-socorro" }),
        None,
    )
    .await;
    assert_eq!(se, StatusCode::CREATED, "{e}");
    let (sl, l) = medico(
        &amb,
        "marcos.freire",
        Method::GET,
        &format!("/v1/laudos/{laudo}"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(sl, StatusCode::OK, "{l}");
    assert_eq!(
        amb.contar("SELECT count(*) FROM auditoria.acesso WHERE resultado = 'emergencia'")
            .await,
        2
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox WHERE tipo = 'aviso.acesso_emergencia'")
            .await,
        1
    );
    assert_eq!(amb.contar("SELECT count(*) FROM cuidado.acesso_emergencia WHERE revisar_ate > now() + interval '47 hours'").await, 1);
}

#[tokio::test]
async fn acesso_de_emergencia_exige_justificativa_de_20_caracteres() {
    let amb = Ambiente::novo().await;
    let (laudo, _) = dois_laudos_publicados(&amb).await;
    let (st, c) = medico(
        &amb,
        "marcos.freire",
        Method::POST,
        &format!("/v1/laudos/{laudo}/acesso-emergencia"),
        json!({ "justificativa": "urgente" }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(c["codigo"], "justificativa-curta");
}

#[tokio::test]
async fn fc_ace_05_rls_barra_mesmo_se_a_rota_errar() {
    let amb = Ambiente::novo().await;
    dois_laudos_publicados(&amb).await;
    let mut tx = amb.app_pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.unidade_id', $1, true), set_config('app.perfis', 'medico', true), set_config('app.usuario_id', $2, true)")
        .bind(comum::UNIDADE_A.to_string()).bind(amb.s.usuarios["marcos.freire"].to_string()).execute(&mut *tx).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM laudos.laudo")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let v: i64 = sqlx::query_scalar("SELECT count(*) FROM laudos.versao")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!((n, v), (0, 0));
}

#[tokio::test]
async fn fc_ace_06_recepcao_nao_le_laudo_e_ve_so_a_situacao() {
    let amb = Ambiente::novo().await;
    let (atd, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let (st, _) = medico(
        &amb,
        "ana.souza",
        Method::GET,
        &format!("/v1/laudos/{laudo}"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (sa, a) = medico(
        &amb,
        "ana.souza",
        Method::GET,
        &format!("/v1/atendimentos/{}", atd["id"].as_str().unwrap()),
        json!({}),
        None,
    )
    .await;
    assert_eq!(sa, StatusCode::OK);
    assert_eq!(a["laudo_situacao"], "rascunho");
    assert!(a.get("texto").is_none());
}

#[tokio::test]
async fn fc_ace_07_ti_nao_le_dado_clinico() {
    let amb = Ambiente::novo().await;
    let (atd, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    for uri in [
        format!("/v1/laudos/{laudo}"),
        "/v1/laudos/fila".to_owned(),
        format!("/v1/atendimentos/{}", atd["id"].as_str().unwrap()),
    ] {
        let (st, _) = medico(&amb, "ti.a", Method::GET, &uri, json!({}), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{uri}");
    }
}

#[tokio::test]
async fn fc_ace_08_link_expirado_nao_abre() {
    let amb = Ambiente::novo().await;
    let (laudo, _) = dois_laudos_publicados(&amb).await;
    let (st, link) = medico(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/links"),
        json!({ "horas": 24 }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{link}");
    let token = link["token"].as_str().unwrap();
    let (sv, _, v) = amb
        .pedir(
            Request::builder()
                .uri(format!("/v1/links/{token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(sv, StatusCode::OK);
    assert_eq!(v["texto"], "Laudo de Antônio.");
    sqlx::query("UPDATE portal.link SET expira_em = now() - interval '1 day'")
        .execute(&amb.admin)
        .await
        .unwrap();
    let (se, _, e) = amb
        .pedir(
            Request::builder()
                .uri(format!("/v1/links/{token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(se, StatusCode::GONE);
    assert_eq!(e["codigo"], "link-expirado");
    assert!(e.get("texto").is_none());
}
