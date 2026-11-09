//! Gestão de dose (docs/27 FC-DOS-01 a 07, proposta aguardando aprovação; DEC-2026-11-09-144 a 149):
//! o aparelho manda imagem e RDSR ao Orthanc REAL (contêiner), o núcleo concilia, lê a dose, compara
//! ao NRD e o médico com vínculo justifica. Rota HTTP real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::{Ambiente, UNIDADE_A};
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::Duration;
use testcontainers_modules::testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers_modules::testcontainers::runners::SyncRunner;
use testcontainers_modules::testcontainers::{Container, GenericImage, ImageExt};
use uuid::Uuid;
use vidacor_api::orthanc::Orthanc;

const TC_ABDOME: &str = "41001109";
const VERA: &str = "Vera Lúcia Antunes";

struct Pacs {
    _c: Container<GenericImage>,
    url: String,
}

fn pacs() -> &'static Pacs {
    static P: OnceLock<Pacs> = OnceLock::new();
    P.get_or_init(|| {
        std::thread::spawn(|| {
            let c = GenericImage::new("orthancteam/orthanc", "latest")
                .with_exposed_port(8042.tcp())
                .with_wait_for(WaitFor::Nothing)
                .with_env_var("ORTHANC__AUTHENTICATION_ENABLED", "false")
                .with_env_var("ORTHANC__STABLE_AGE", "1")
                .with_label("org.testcontainers.managed-by", "testcontainers")
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

fn medida(codigo: &str, valor: &str, unidade: &str) -> Value {
    json!({
        "ValueType": "NUM", "RelationshipType": "CONTAINS",
        "ConceptNameCodeSequence": [{ "CodeValue": codigo, "CodingSchemeDesignator": "DCM", "CodeMeaning": codigo }],
        "MeasuredValueSequence": [{ "NumericValue": valor,
            "MeasurementUnitsCodeSequence": [{ "CodeValue": unidade, "CodingSchemeDesignator": "UCUM", "CodeMeaning": unidade }] }]
    })
}

/// O tomógrafo manda uma imagem e o RDSR (X-Ray Radiation Dose SR) do mesmo estudo.
async fn tomografo_envia(url: &str, accession: &str, ctdivol: &str, dlp: &str) {
    let http = ::reqwest::Client::new();
    let r: Value = http
        .post(format!("{url}/tools/create-dicom"))
        .json(&json!({ "Tags": {
            "PatientName": "ANTUNES^VERA LUCIA", "PatientID": "PAC-2026-000004",
            "AccessionNumber": accession, "Modality": "CT", "StudyDescription": "TC de abdome"
        }}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let instancia = r["ID"].as_str().unwrap().to_owned();
    let estudo: Value = http
        .get(format!("{url}/instances/{instancia}/study"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let estudo_id = estudo["ID"].as_str().unwrap();
    let sr = http
        .post(format!("{url}/tools/create-dicom"))
        .json(&json!({ "Parent": estudo_id, "Tags": {
            "Modality": "SR", "SOPClassUID": "1.2.840.10008.5.1.4.1.1.88.67", "ValueType": "CONTAINER",
            "ConceptNameCodeSequence": [{ "CodeValue": "113701", "CodingSchemeDesignator": "DCM", "CodeMeaning": "X-Ray Radiation Dose Report" }],
            "ContentSequence": [
                { "ValueType": "CONTAINER", "RelationshipType": "CONTAINS",
                  "ConceptNameCodeSequence": [{ "CodeValue": "113811", "CodingSchemeDesignator": "DCM", "CodeMeaning": "CT Accumulated Dose Data" }],
                  "ContentSequence": [medida("113813", dlp, "mGy.cm")] },
                { "ValueType": "CONTAINER", "RelationshipType": "CONTAINS",
                  "ConceptNameCodeSequence": [{ "CodeValue": "113819", "CodingSchemeDesignator": "DCM", "CodeMeaning": "CT Acquisition" }],
                  "ContentSequence": [{ "ValueType": "CONTAINER", "RelationshipType": "CONTAINS",
                      "ConceptNameCodeSequence": [{ "CodeValue": "113829", "CodingSchemeDesignator": "DCM", "CodeMeaning": "CT Dose" }],
                      "ContentSequence": [medida("113830", ctdivol, "mGy"), medida("113838", dlp, "mGy.cm")] }] }
            ]
        }}))
        .send()
        .await
        .unwrap();
    assert!(
        sr.status().is_success(),
        "RDSR: {}",
        sr.text().await.unwrap()
    );
}

async fn publicar_nrd(amb: &Ambiente, login: &str, corpo: Value) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/nrd/{TC_ABDOME}"),
            &s,
            Some(Uuid::now_v7()),
            corpo,
        ))
        .await;
    (st, c)
}

/// Do check-in à dose gravada: estudo de TC da Vera com o RDSR, conciliado e lido.
async fn ate_a_dose(amb: &Ambiente, ctdivol: &str, dlp: &str) -> Orthanc {
    let p = pacs();
    let orthanc = Orthanc::novo(&p.url).unwrap();
    // O Orthanc é compartilhado pelos testes em paralelo e cada teste tem um banco novo: o contador do
    // accession começa num ponto próprio, como numa clínica real o accession nunca se repete.
    let inicio = i64::from(u32::try_from(Uuid::now_v7().as_u128() % 900_000).unwrap()) + 1000;
    sqlx::query("INSERT INTO comum.contador (prefixo, ano, valor) VALUES ('ACC', $1, $2) ON CONFLICT (prefixo, ano) DO UPDATE SET valor = EXCLUDED.valor")
        .bind(vidacor_api::tempo::ano_local())
        .bind(inicio)
        .execute(&amb.admin)
        .await
        .unwrap();
    let atd = amb.fazer_checkin(VERA, "1957-09-18").await;
    let accession = atd["accession"].as_str().unwrap().to_owned();
    tomografo_envia(&p.url, &accession, ctdivol, dlp).await;
    for _ in 0..40 {
        vidacor_api::outbox::conciliar(&amb.estado, &orthanc, UNIDADE_A)
            .await
            .unwrap();
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM exames.estudo WHERE conciliado AND accession = $1",
        )
        .bind(&accession)
        .fetch_one(&amb.admin)
        .await
        .unwrap();
        if n == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let gravadas = vidacor_api::dose::ler_pendentes(&amb.estado, &orthanc, UNIDADE_A)
        .await
        .unwrap();
    assert_eq!(gravadas, 1, "dose não lida");
    orthanc
}

#[tokio::test]
async fn fc_dos_01_rdsr_do_aparelho_vira_dose_comparada_ao_nrd() {
    let amb = Ambiente::novo().await;
    let (st, n) = publicar_nrd(
        &amb,
        "patricia.lemos",
        json!({ "grandeza": "ctdivol", "valor": 12.0, "fator_dose_efetiva": 0.015 }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{n}");
    let orthanc = ate_a_dose(&amb, "14.2", "520").await;
    // A Dra. Cecília (médica do agendamento, com vínculo) vê a dose acima do NRD.
    let cecilia = amb.entrar("cecilia.souto").await;
    let (st, _, d) = amb.pedir(amb.get("/v1/doses", Some(&cecilia))).await;
    assert_eq!(st, StatusCode::OK, "{d}");
    let e = &d["exames"][0];
    assert_eq!(e["paciente"]["nome"], VERA);
    assert_eq!(e["paciente"]["nascimento"], "1957-09-18");
    assert_eq!(e["ctdivol_mgy"], 14.2);
    assert_eq!(e["dlp_mgycm"], 520.0);
    assert_eq!(e["nrd"]["valor"], 12.0);
    assert_eq!(e["acima_do_nrd"], true);
    assert_eq!(d["sem_justificativa"], 1);
    // Ler de novo não duplica (uma dose por estudo, G3).
    assert_eq!(
        vidacor_api::dose::ler_pendentes(&amb.estado, &orthanc, UNIDADE_A)
            .await
            .unwrap(),
        0
    );
    assert_eq!(amb.contar("SELECT count(*) FROM exames.dose").await, 1);
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'dose_lida'")
            .await,
        1
    );
    // A origem do valor fica guardada: o objeto RDSR e o SHA-256 das tags lidas.
    assert_eq!(
        amb.contar("SELECT count(*) FROM exames.dose WHERE rdsr_sop_uid <> '' AND length(rdsr_sha256) = 64").await,
        1
    );
    // Histórico com a dose efetiva estimada (520 x 0,015 = 7,8 mSv).
    let pid = amb.s.pacientes[VERA];
    let (st, _, h) = amb
        .pedir(amb.get(&format!("/v1/doses/pacientes/{pid}"), Some(&cecilia)))
        .await;
    assert_eq!(st, StatusCode::OK, "{h}");
    assert_eq!(h["dose_efetiva_12_meses_msv"], 7.8);
}

#[tokio::test]
async fn fc_dos_02_sem_vinculo_o_medico_nao_ve_a_dose_e_a_gerencia_so_ve_o_agregado() {
    let amb = Ambiente::novo().await;
    publicar_nrd(
        &amb,
        "patricia.lemos",
        json!({ "grandeza": "ctdivol", "valor": 12.0 }),
    )
    .await;
    ate_a_dose(&amb, "14.2", "520").await;
    let paulo = amb.entrar("medico.a").await;
    let (st, _, d) = amb.pedir(amb.get("/v1/doses", Some(&paulo))).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(d["exames"].as_array().unwrap().len(), 0, "{d}");
    let pid = amb.s.pacientes[VERA];
    let (st, _, _) = amb
        .pedir(amb.get(&format!("/v1/doses/pacientes/{pid}"), Some(&paulo)))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let patricia = amb.entrar("patricia.lemos").await;
    let (st, _, _) = amb.pedir(amb.get("/v1/doses", Some(&patricia))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _, p) = amb
        .pedir(amb.get("/v1/doses/protocolos", Some(&patricia)))
        .await;
    assert_eq!(st, StatusCode::OK, "{p}");
    assert_eq!(p[0]["exame_tuss"], TC_ABDOME);
    assert_eq!(p[0]["exames"], 1);
    assert_eq!(p[0]["acima"], 1);
    assert_eq!(p[0]["percentual_do_nrd"], 118);
    assert!(p[0].get("paciente").is_none());
    for login in ["ana.souza", "bruno.sales", "carla.caixa", "ti.a"] {
        let s = amb.entrar(login).await;
        let (st, _, _) = amb.pedir(amb.get("/v1/doses/protocolos", Some(&s))).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
}

#[tokio::test]
async fn fc_dos_03_justificativa_uma_vez_so_e_so_por_quem_tem_vinculo() {
    let amb = Ambiente::novo().await;
    publicar_nrd(
        &amb,
        "patricia.lemos",
        json!({ "grandeza": "ctdivol", "valor": 12.0 }),
    )
    .await;
    ate_a_dose(&amb, "14.2", "520").await;
    let dose: Uuid = sqlx::query_scalar("SELECT id FROM exames.dose")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    let uri = format!("/v1/doses/{dose}/justificativa");
    let cecilia = amb.entrar("cecilia.souto").await;
    let paulo = amb.entrar("medico.a").await;
    let (st, _, e) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &paulo,
            Some(Uuid::now_v7()),
            json!({ "motivo": "imc_alto" }),
        ))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "{e}");
    let (st, _, e) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &cecilia,
            Some(Uuid::now_v7()),
            json!({ "motivo": "outro" }),
        ))
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(e["codigo"], "justificativa-invalida");
    let chave = Uuid::now_v7();
    let corpo =
        json!({ "motivo": "fase_extra", "detalhe": "Fase tardia pedida pelo radiologista" });
    let (s1, _, a) = amb
        .pedir(amb.json(Method::POST, &uri, &cecilia, Some(chave), corpo.clone()))
        .await;
    let (s2, _, b) = amb
        .pedir(amb.json(Method::POST, &uri, &cecilia, Some(chave), corpo.clone()))
        .await;
    assert_eq!((s1, s2), (StatusCode::CREATED, StatusCode::CREATED), "{a}");
    assert_eq!(a, b);
    let (st, _, e) = amb
        .pedir(amb.json(Method::POST, &uri, &cecilia, Some(Uuid::now_v7()), corpo))
        .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(e["codigo"], "dose-ja-justificada");
    let (_, _, d) = amb.pedir(amb.get("/v1/doses", Some(&cecilia))).await;
    assert_eq!(d["exames"][0]["justificativa"]["motivo"], "fase_extra");
    assert_eq!(d["exames"][0]["justificativa"]["por"], "Dra. Cecília Souto");
    assert_eq!(d["sem_justificativa"], 0);
}

#[tokio::test]
async fn fc_dos_04_dose_dentro_do_nrd_nao_pede_justificativa() {
    let amb = Ambiente::novo().await;
    publicar_nrd(
        &amb,
        "patricia.lemos",
        json!({ "grandeza": "ctdivol", "valor": 15.0 }),
    )
    .await;
    ate_a_dose(&amb, "9.8", "470").await;
    let dose: Uuid = sqlx::query_scalar("SELECT id FROM exames.dose")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    let cecilia = amb.entrar("cecilia.souto").await;
    let (st, _, e) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/doses/{dose}/justificativa"),
            &cecilia,
            Some(Uuid::now_v7()),
            json!({ "motivo": "imc_alto" }),
        ))
        .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(e["codigo"], "dose-dentro-do-nrd");
}

#[tokio::test]
async fn fc_dos_05_nrd_so_a_gerencia_publica_com_grandeza_certa_e_versao() {
    let amb = Ambiente::novo().await;
    let (st, _) = publicar_nrd(
        &amb,
        "cecilia.souto",
        json!({ "grandeza": "ctdivol", "valor": 12.0 }),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, e) = publicar_nrd(
        &amb,
        "patricia.lemos",
        json!({ "grandeza": "dap", "valor": 1.2 }),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(e["codigo"], "nrd-invalido");
    for valor in [0.0, -1.0, 200_000.0] {
        let (st, _) = publicar_nrd(
            &amb,
            "patricia.lemos",
            json!({ "grandeza": "ctdivol", "valor": valor }),
        )
        .await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{valor}");
    }
    let (st, _) = publicar_nrd(
        &amb,
        "patricia.lemos",
        json!({ "grandeza": "ctdivol", "valor": 12.0 }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    // Segunda versão: exige a versão vista e o motivo; a anterior fica guardada.
    let (st, e) = publicar_nrd(
        &amb,
        "patricia.lemos",
        json!({ "versao_vista": 1, "grandeza": "ctdivol", "valor": 14.0 }),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(e["codigo"], "motivo-obrigatorio");
    let (st, e) = publicar_nrd(
        &amb,
        "patricia.lemos",
        json!({ "grandeza": "ctdivol", "valor": 14.0, "motivo": "Revisão anual da física médica" }),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{e}");
    let (st, n) = publicar_nrd(&amb, "patricia.lemos", json!({ "versao_vista": 1, "grandeza": "ctdivol", "valor": 14.0, "motivo": "Revisão anual da física médica" })).await;
    assert_eq!(st, StatusCode::CREATED, "{n}");
    assert_eq!(n["numero"], 2);
    assert_eq!(amb.contar("SELECT count(*) FROM exames.nrd").await, 2);
    assert_eq!(
        amb.contar("SELECT count(*) FROM exames.nrd WHERE vigente")
            .await,
        1
    );
}

#[tokio::test]
async fn fc_dos_06_duas_publicacoes_simultaneas_do_nrd_so_uma_entra() {
    let amb = Ambiente::novo().await;
    let (a, b) = tokio::join!(
        publicar_nrd(
            &amb,
            "patricia.lemos",
            json!({ "grandeza": "ctdivol", "valor": 12.0 })
        ),
        publicar_nrd(
            &amb,
            "patricia.lemos",
            json!({ "grandeza": "dlp", "valor": 700.0 })
        )
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(
        st,
        [StatusCode::CREATED, StatusCode::CONFLICT],
        "{} {}",
        a.1,
        b.1
    );
    assert_eq!(amb.contar("SELECT count(*) FROM exames.nrd").await, 1);
}

#[tokio::test]
async fn fc_dos_07_periodo_invalido_e_sem_login() {
    let amb = Ambiente::novo().await;
    let cecilia = amb.entrar("cecilia.souto").await;
    let (st, _, e) = amb
        .pedir(amb.get("/v1/doses?de=2026-01-01&ate=2026-12-31", Some(&cecilia)))
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(e["codigo"], "periodo-invalido");
    let (st, _, _) = amb
        .pedir(amb.get("/v1/doses?de=2026-11-09&ate=2026-11-01", Some(&cecilia)))
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, _, _) = amb.pedir(amb.get("/v1/doses", None)).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}
