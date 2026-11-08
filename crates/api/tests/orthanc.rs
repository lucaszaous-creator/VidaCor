//! Worklist e conciliação com um Orthanc REAL em contêiner (G4: Orthanc real quando o fluxo usa).
//! REQ-REC-005, REQ-EXA-001 a 003: check-in vira item de worklist pelo outbox; estudo recebido é
//! conciliado pelo accession; estudo sem atendimento fica pendente; repetir não duplica.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use comum::{Ambiente, UNIDADE_A};
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::Duration;
use testcontainers_modules::testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers_modules::testcontainers::runners::SyncRunner;
use testcontainers_modules::testcontainers::{Container, GenericImage, ImageExt};
use uuid::Uuid;
use vidacor_api::orthanc::Orthanc;
use vidacor_api::outbox;

struct Pacs {
    _c: Container<GenericImage>,
    url: String,
}

fn pacs() -> &'static Pacs {
    static P: OnceLock<Pacs> = OnceLock::new();
    P.get_or_init(|| {
        std::thread::spawn(|| {
            // A fixar por versão junto com o compose (SEC-SUP-1) quando houver tag de versão adotada.
            let c = GenericImage::new("orthancteam/orthanc", "latest")
                .with_exposed_port(8042.tcp())
                .with_wait_for(WaitFor::Nothing)
                .with_env_var("ORTHANC__AUTHENTICATION_ENABLED", "false")
                .with_env_var("WORKLISTS_PLUGIN_ENABLED", "true")
                .with_env_var("ORTHANC__WORKLISTS__ENABLE", "true")
                .with_env_var("ORTHANC__WORKLISTS__SAVE_IN_ORTHANC_DATABASE", "true")
                .with_env_var("ORTHANC__STABLE_AGE", "1")
                .start()
                .expect("subir Orthanc");
            let porta = c.get_host_port_ipv4(8042.tcp()).expect("porta");
            let url = format!("http://127.0.0.1:{porta}");
            let rt = tokio::runtime::Runtime::new().expect("runtime");
            let pronto = rt.block_on(async {
                let http = ::reqwest::Client::new();
                for _ in 0..120 {
                    if let Ok(r) = http.get(format!("{url}/system")).send().await {
                        if r.status().is_success() {
                            return true;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                false
            });
            assert!(pronto, "Orthanc não respondeu");
            Pacs { _c: c, url }
        })
        .join()
        .expect("thread do Orthanc")
    })
}

async fn http() -> ::reqwest::Client {
    ::reqwest::Client::new()
}

async fn criar_estudo_no_aparelho(url: &str, accession: &str) {
    // Simula o aparelho enviando uma imagem com o accession recebido pela worklist.
    let corpo = json!({ "Tags": {
        "PatientName": "RIBEIRO^ANTONIO CARLOS", "PatientID": "PAC-2026-000001",
        "AccessionNumber": accession, "Modality": "US", "StudyDescription": "Ecocardiograma"
    }});
    let r = http()
        .await
        .post(format!("{url}/tools/create-dicom"))
        .json(&corpo)
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success(), "create-dicom: {}", r.status());
}

async fn conciliar_ate_estabilizar(amb: &Ambiente, orthanc: &Orthanc, esperado: i64) {
    for _ in 0..40 {
        outbox::conciliar(&amb.estado, orthanc, UNIDADE_A)
            .await
            .unwrap();
        if amb.contar("SELECT count(*) FROM exames.estudo").await >= esperado {
            return;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    panic!("estudo não foi conciliado a tempo");
}

#[tokio::test]
async fn checkin_vira_worklist_e_estudo_recebido_e_conciliado() {
    let p = pacs();
    let amb = Ambiente::novo().await;
    let orthanc = Orthanc::novo(&p.url).unwrap();
    let ana = amb.entrar("ana.souza").await;
    let mut eventos = amb.estado.eventos.subscribe();

    let (status, _, atd) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            "Antônio Carlos Ribeiro",
            "1954-06-15",
        ))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let accession = atd["accession"].as_str().unwrap().to_owned();

    assert!(outbox::processar_um(&amb.estado, &orthanc).await.unwrap());
    assert!(
        !outbox::processar_um(&amb.estado, &orthanc).await.unwrap(),
        "nada pendente depois"
    );
    let erro: Option<String> = sqlx::query_scalar("SELECT ultimo_erro FROM integracao.outbox")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox WHERE concluido_em IS NOT NULL")
            .await,
        1,
        "erro: {erro:?}"
    );

    let lista: Vec<Value> = http()
        .await
        .get(format!("{}/worklists?format=Simplify", p.url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let item = lista
        .iter()
        .find(|w| w["Tags"]["AccessionNumber"] == accession.as_str())
        .expect("worklist criada");
    assert_eq!(item["Tags"]["PatientName"], "RIBEIRO^ANTONIO CARLOS");
    assert_eq!(item["Tags"]["PatientBirthDate"], "19540615");
    assert_eq!(
        item["Tags"]["ScheduledProcedureStepSequence"][0]["ScheduledStationAETitle"],
        "SALA1"
    );

    criar_estudo_no_aparelho(&p.url, &accession).await;
    conciliar_ate_estabilizar(&amb, &orthanc, 1).await;

    let (s, _, detalhe) = amb
        .pedir(amb.get(
            &format!("/v1/atendimentos/{}", atd["id"].as_str().unwrap()),
            Some(&ana),
        ))
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(detalhe["estudo"]["imagens"], 1);
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'estudo_conciliado'")
            .await,
        1
    );

    let evento = tokio::time::timeout(Duration::from_secs(2), eventos.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(evento.tipo, "estudo-recebido");
    assert_eq!(evento.dados["accession"], accession.as_str());

    // Repetir a leitura não duplica estudo nem evento.
    outbox::conciliar(&amb.estado, &orthanc, UNIDADE_A)
        .await
        .unwrap();
    sqlx::query("UPDATE integracao.cursor_orthanc SET ultima_mudanca = 0")
        .execute(&amb.admin)
        .await
        .unwrap();
    outbox::conciliar(&amb.estado, &orthanc, UNIDADE_A)
        .await
        .unwrap();
    assert_eq!(
        amb.contar("SELECT count(*) FROM exames.estudo WHERE conciliado")
            .await,
        1
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'estudo_conciliado'")
            .await,
        1
    );
}

#[tokio::test]
async fn estudo_com_accession_desconhecido_fica_sem_conciliacao() {
    let p = pacs();
    let amb = Ambiente::novo().await;
    let orthanc = Orthanc::novo(&p.url).unwrap();
    let accession = format!("ACC-9{}", &Uuid::now_v7().simple().to_string()[..10]);
    criar_estudo_no_aparelho(&p.url, &accession).await;
    for _ in 0..40 {
        outbox::conciliar(&amb.estado, &orthanc, UNIDADE_A)
            .await
            .unwrap();
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM exames.estudo WHERE accession = $1")
            .bind(&accession)
            .fetch_one(&amb.admin)
            .await
            .unwrap();
        if n == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let (conciliado, atendimento): (bool, Option<Uuid>) =
        sqlx::query_as("SELECT conciliado, atendimento_id FROM exames.estudo WHERE accession = $1")
            .bind(&accession)
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert!(
        !conciliado,
        "imagem sem paciente conciliado nunca vai para laudo"
    );
    assert!(atendimento.is_none());
    assert!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'estudo_sem_conciliacao'")
            .await
            >= 1
    );
}

#[tokio::test]
async fn orthanc_fora_do_ar_deixa_o_item_pendente_com_retentativa() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (status, _, _) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            "Antônio Carlos Ribeiro",
            "1954-06-15",
        ))
        .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "o atendimento não para por causa do PACS (R7)"
    );
    let fora = Orthanc::novo("http://127.0.0.1:9").unwrap();
    assert!(outbox::processar_um(&amb.estado, &fora).await.unwrap());
    let (concluido, erro, tentativas): (
        Option<chrono::DateTime<chrono::Utc>>,
        Option<String>,
        i32,
    ) = sqlx::query_as("SELECT concluido_em, ultimo_erro, tentativas FROM integracao.outbox")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    assert!(concluido.is_none());
    assert!(erro.is_some());
    assert_eq!(tentativas, 1);
    // Em espera (backoff): não é pego de novo de imediato.
    assert!(!outbox::processar_um(&amb.estado, &fora).await.unwrap());
}
