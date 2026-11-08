//! Laudo impresso e dossiê do lastro em PDF (Typst embutido). Rota HTTP real e PostgreSQL real
//! (G4); só o provedor de assinatura é simulado (externo). Só dados fictícios.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use comum::Ambiente;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;
use vidacor_api::outbox;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const NASC: &str = "1954-06-15";

struct Resposta {
    status: StatusCode,
    cabecalhos: HeaderMap,
    corpo: Vec<u8>,
}

impl Resposta {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.corpo).unwrap_or(Value::Null)
    }

    fn cabecalho(&self, nome: &str) -> &str {
        self.cabecalhos
            .get(nome)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
    }

    /// PDF íntegro: começa com %PDF e o SHA-256 do cabeçalho é o dos bytes recebidos.
    fn conferir_pdf(&self) -> String {
        assert_eq!(self.status, StatusCode::OK, "{}", self.json());
        assert!(self.corpo.starts_with(b"%PDF"), "corpo começa com %PDF");
        assert_eq!(self.cabecalho("content-type"), "application/pdf");
        assert_eq!(self.cabecalho("cache-control"), "no-store");
        let sha: String = Sha256::digest(&self.corpo)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(self.cabecalho("x-documento-sha256"), sha);
        assert!(self.cabecalho("repr-digest").starts_with("sha-256=:"));
        sha
    }
}

async fn bruto(amb: &Ambiente, req: Request<Body>) -> Resposta {
    let r = amb.app.clone().oneshot(req).await.unwrap();
    let (partes, corpo) = r.into_parts();
    Resposta {
        status: partes.status,
        cabecalhos: partes.headers,
        corpo: corpo.collect().await.unwrap().to_bytes().to_vec(),
    }
}

async fn pdf_da_equipe(amb: &Ambiente, login: &str, laudo: Uuid) -> Resposta {
    let s = amb.entrar(login).await;
    bruto(amb, amb.get(&format!("/v1/laudos/{laudo}/pdf"), Some(&s))).await
}

async fn pdf_do_portal(amb: &Ambiente, cookie: &str, laudo: Uuid) -> Resposta {
    let req = Request::builder()
        .uri(format!("/v1/portal/laudos/{laudo}/pdf"))
        .header("cookie", cookie)
        .body(Body::empty())
        .unwrap();
    bruto(amb, req).await
}

async fn medico(amb: &Ambiente, metodo: Method, uri: &str, corpo: Value) -> (StatusCode, Value) {
    let s = amb.entrar("medico.a").await;
    let (st, _, c) = amb
        .pedir(amb.json(metodo, uri, &s, Some(Uuid::now_v7()), corpo))
        .await;
    (st, c)
}

async fn escrever_e_assinar(amb: &Ambiente, laudo: Uuid, texto: &str, versao: i32) {
    let (st, c) = medico(
        amb,
        Method::PUT,
        &format!("/v1/laudos/{laudo}/rascunho"),
        json!({ "texto": texto, "versao_numero": versao }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    let (st, c) = medico(
        amb,
        Method::POST,
        &format!("/v1/laudos/{laudo}/assinatura"),
        json!({ "pin": "4821" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{c}");
}

async fn processar_outbox(amb: &Ambiente) {
    let orthanc = vidacor_api::orthanc::Orthanc::novo("http://127.0.0.1:9").unwrap();
    while outbox::processar_um(&amb.estado, &orthanc).await.unwrap() {}
}

/// CRM fictício do Dr. Paulo (médico do laudo), como o cadastro do corpo clínico faria.
async fn cadastrar_crm(amb: &Ambiente) {
    sqlx::query(
        "INSERT INTO documentos.registro_profissional (id, usuario_id, conselho, uf, numero, rqe, origem, unidade_id, correlacao)
         VALUES ($1, $2, 'CRM', 'XX', '000000', '0000', 'seed', $3, 'seed')",
    )
    .bind(Uuid::now_v7())
    .bind(amb.s.usuarios["medico.a"])
    .bind(comum::UNIDADE_A)
    .execute(&amb.admin)
    .await
    .unwrap();
}

async fn laudo_assinado(amb: &Ambiente) -> Uuid {
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    escrever_e_assinar(amb, laudo, "Ecocardiograma dentro da normalidade.", 1).await;
    laudo
}

#[tokio::test]
async fn laudo_assinado_sai_em_pdf_com_hash_conferido_e_emissao_registrada() {
    let amb = Ambiente::novo().await;
    cadastrar_crm(&amb).await;
    let laudo = laudo_assinado(&amb).await;
    let r = pdf_da_equipe(&amb, "medico.a", laudo).await;
    let sha = r.conferir_pdf();
    let disposicao = r.cabecalho("content-disposition");
    assert!(disposicao.contains("LAU-") && disposicao.contains("-v1.pdf"));
    assert!(
        !disposicao.contains("Ribeiro"),
        "nome do paciente fora do nome do arquivo"
    );
    let emissao = r.cabecalho("x-documento-codigo").to_owned();
    assert!(emissao.starts_with("DOC-"));
    let (tipo, canal, versao, gravado): (String, String, Option<i32>, String) = sqlx::query_as(
        "SELECT tipo, canal, versao_registro, sha256 FROM documentos.emissao WHERE codigo = $1",
    )
    .bind(&emissao)
    .fetch_one(&amb.admin)
    .await
    .unwrap();
    assert_eq!(
        (tipo.as_str(), canal.as_str(), versao, gravado.as_str()),
        ("laudo_impresso", "equipe", Some(1), sha.as_str())
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM auditoria.acesso WHERE recurso = 'laudo.pdf' AND resultado = 'permitido'")
            .await,
        1
    );
    // Mesma versão assinada, mesmo PDF; cada entrega é uma emissão nova.
    let de_novo = pdf_da_equipe(&amb, "medico.a", laudo).await;
    assert_eq!(de_novo.conferir_pdf(), sha);
    assert_eq!(
        amb.contar("SELECT count(*) FROM documentos.emissao").await,
        2
    );
    // A emissão é somente inserção.
    assert!(sqlx::query("UPDATE documentos.emissao SET bytes = 1")
        .execute(&amb.admin)
        .await
        .is_err());
    assert!(sqlx::query("DELETE FROM documentos.emissao")
        .execute(&amb.admin)
        .await
        .is_err());
}

#[tokio::test]
async fn laudo_em_rascunho_nao_imprime() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let r = pdf_da_equipe(&amb, "medico.a", laudo).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.json()["codigo"], "laudo-nao-assinado");
    assert_eq!(
        amb.contar("SELECT count(*) FROM documentos.emissao").await,
        0
    );
}

#[tokio::test]
async fn medico_sem_vinculo_e_outros_perfis_nao_baixam_e_a_tentativa_fica_auditada() {
    let amb = Ambiente::novo().await;
    let laudo = laudo_assinado(&amb).await;
    let r = pdf_da_equipe(&amb, "marcos.freire", laudo).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.json()["codigo"], "sem-vinculo-de-cuidado");
    assert!(!r.corpo.starts_with(b"%PDF"));
    assert_eq!(
        amb.contar("SELECT count(*) FROM auditoria.acesso WHERE recurso = 'laudo.pdf' AND resultado = 'negado'")
            .await,
        1
    );
    for login in ["ana.souza", "ti.a", "patricia.lemos"] {
        let r = pdf_da_equipe(&amb, login, laudo).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{login}");
    }
    let inexistente = pdf_da_equipe(&amb, "medico.a", Uuid::now_v7()).await;
    assert_eq!(inexistente.status, StatusCode::NOT_FOUND);
    assert_eq!(
        amb.contar("SELECT count(*) FROM documentos.emissao").await,
        0
    );
}

#[tokio::test]
async fn retificacao_imprime_a_versao_vigente() {
    let amb = Ambiente::novo().await;
    let laudo = laudo_assinado(&amb).await;
    let v1 = pdf_da_equipe(&amb, "medico.a", laudo).await.conferir_pdf();
    let (st, c) = medico(
        &amb,
        Method::POST,
        &format!("/v1/laudos/{laudo}/retificacao"),
        json!({ "texto": "Ecocardiograma: fração de ejeção corrigida para 64%.", "motivo": "Medida da fração de ejeção corrigida." }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    // Retificação em rascunho: o que imprime ainda é a versão 1 assinada.
    let enquanto = pdf_da_equipe(&amb, "medico.a", laudo).await;
    assert_eq!(enquanto.conferir_pdf(), v1);
    let (st, c) = medico(
        &amb,
        Method::POST,
        &format!("/v1/laudos/{laudo}/assinatura"),
        json!({ "pin": "4821" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    let v2 = pdf_da_equipe(&amb, "medico.a", laudo).await;
    assert_ne!(v2.conferir_pdf(), v1);
    assert!(v2.cabecalho("content-disposition").contains("-v2.pdf"));
    let versoes: Vec<i32> =
        sqlx::query_scalar("SELECT versao_registro FROM documentos.emissao ORDER BY criado_em")
            .fetch_all(&amb.admin)
            .await
            .unwrap();
    assert_eq!(versoes, vec![1, 1, 2]);
}

#[tokio::test]
async fn paciente_baixa_o_proprio_laudo_e_nao_o_de_outro_paciente() {
    let amb = Ambiente::novo().await;
    let laudo_antonio = laudo_assinado(&amb).await;
    sqlx::query("UPDATE agenda.agendamento SET medico_id = $1 WHERE codigo = 'AGD-SEED-000002'")
        .bind(amb.s.usuarios["medico.a"])
        .execute(&amb.admin)
        .await
        .unwrap();
    let (_, laudo_maria) = amb.ate_o_laudo("Maria Helena Duarte", "1968-03-12").await;
    escrever_e_assinar(&amb, laudo_maria, "Laudo de Maria.", 1).await;
    processar_outbox(&amb).await;

    let antonio = amb.entrar_no_portal(ANTONIO).await;
    let proprio = pdf_do_portal(&amb, &antonio, laudo_antonio).await;
    let sha = proprio.conferir_pdf();
    let (canal, quem_tipo, gravado): (String, String, String) =
        sqlx::query_as("SELECT canal, quem_tipo, sha256 FROM documentos.emissao WHERE codigo = $1")
            .bind(proprio.cabecalho("x-documento-codigo"))
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(
        (canal.as_str(), quem_tipo.as_str(), gravado.as_str()),
        ("portal", "paciente", sha.as_str())
    );

    // Maria tenta o laudo de Antônio: a mesma resposta de um laudo que não existe.
    let maria = amb.entrar_no_portal("Maria Helena Duarte").await;
    let alheio = pdf_do_portal(&amb, &maria, laudo_antonio).await;
    let inexistente = pdf_do_portal(&amb, &maria, Uuid::now_v7()).await;
    assert_eq!(
        (alheio.status, inexistente.status),
        (StatusCode::NOT_FOUND, StatusCode::NOT_FOUND)
    );
    assert!(!alheio.corpo.starts_with(b"%PDF"));
    assert_eq!(alheio.json()["codigo"], inexistente.json()["codigo"]);
    assert_eq!(alheio.json()["detail"], inexistente.json()["detail"]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM auditoria.acesso WHERE quem_tipo = 'paciente' AND recurso = 'portal.laudo.pdf' AND resultado = 'negado'")
            .await,
        2
    );
    // O próprio laudo de Maria sai normalmente.
    pdf_do_portal(&amb, &maria, laudo_maria)
        .await
        .conferir_pdf();
    // Sem sessão do portal: nada sai.
    let sem_sessao = pdf_do_portal(&amb, "vc_portal=invalido", laudo_antonio).await;
    assert_eq!(sem_sessao.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn laudo_ainda_nao_publicado_nao_sai_no_portal() {
    let amb = Ambiente::novo().await;
    let laudo = laudo_assinado(&amb).await;
    // Assinado, mas a publicação ainda está na fila (outbox não processado).
    let antonio = amb.entrar_no_portal(ANTONIO).await;
    let r = pdf_do_portal(&amb, &antonio, laudo).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

/// SEC-INJ-4: marcação Typst no nome do paciente e no texto do laudo sai como texto.
#[tokio::test]
async fn caracteres_especiais_no_dado_nao_quebram_nem_injetam_marcacao() {
    let amb = Ambiente::novo().await;
    let nome = r#"Antônio #Carlos *Ribeiro* $x^2$ [teste] \ @ref <r> #panic("injetado")"#;
    sqlx::query("UPDATE identidade.paciente SET nome = $1 WHERE id = $2")
        .bind(nome)
        .bind(amb.s.pacientes[ANTONIO])
        .execute(&amb.admin)
        .await
        .unwrap();
    let ana = amb.entrar("ana.souza").await;
    let (st, _, atd) = amb
        .pedir(amb.checkin(
            &ana,
            Some(Uuid::now_v7()),
            amb.s.agendamentos[ANTONIO],
            nome,
            NASC,
        ))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{atd}");
    let estudo = vidacor_api::orthanc::EstudoOrthanc {
        orthanc_id: format!("{}-0000", &Uuid::now_v7().simple().to_string()[..8]),
        study_uid: format!(
            "1.2.826.0.1.{}",
            Uuid::now_v7().as_u128() % 1_000_000_000_000
        ),
        accession: Some(atd["accession"].as_str().unwrap().to_owned()),
        patient_id: None,
        imagens: 3,
    };
    outbox::registrar_estudo(&amb.estado, comum::UNIDADE_A, &estudo)
        .await
        .unwrap();
    let laudo: Uuid = sqlx::query_scalar("SELECT id FROM laudos.laudo WHERE atendimento_id = $1")
        .bind(Uuid::parse_str(atd["id"].as_str().unwrap()).unwrap())
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    escrever_e_assinar(
        &amb,
        laudo,
        "#set page(width: 1pt)\n#panic(\"injetado\")\n= Título falso\n*sem negrito* _sem itálico_ $a+b$ // não é comentário",
        1,
    )
    .await;
    let r = pdf_da_equipe(&amb, "medico.a", laudo).await;
    r.conferir_pdf();
    assert!(!r.cabecalho("content-disposition").contains('#'));
}

#[tokio::test]
async fn dossie_em_pdf_registra_a_emissao_no_lastro_com_o_hash() {
    let amb = Ambiente::novo().await;
    let (atd, _) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let codigo = atd["codigo"].as_str().unwrap();
    let gerente = amb.entrar("patricia.lemos").await;
    let r = bruto(
        &amb,
        amb.json(
            Method::POST,
            "/v1/lastro/dossie",
            &gerente,
            None,
            json!({ "codigo": atd["accession"] }),
        ),
    )
    .await;
    let sha = r.conferir_pdf();
    let dossie = r.cabecalho("x-documento-codigo").to_owned();
    assert!(dossie.starts_with("DOS-"));
    assert!(r.cabecalho("content-disposition").contains(&dossie));
    let (registro, dados): (Option<String>, Value) = sqlx::query_as(
        "SELECT registro_codigo, dados FROM lastro.evento WHERE tipo = 'dossie_emitido'",
    )
    .fetch_one(&amb.admin)
    .await
    .unwrap();
    assert_eq!(registro.as_deref(), Some(codigo));
    assert_eq!(dados["sha256"], sha);
    assert_eq!(dados["emissao"], dossie);
    assert_eq!(dados["integra"], true);
    let (tipo, gravado): (String, String) =
        sqlx::query_as("SELECT tipo, sha256 FROM documentos.emissao WHERE codigo = $1")
            .bind(&dossie)
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(
        (tipo.as_str(), gravado.as_str()),
        ("dossie_lastro", sha.as_str())
    );
    // A cadeia continua íntegra e mostra a emissão.
    let (st, _, cadeia) = amb
        .pedir(amb.get(
            &format!("/v1/lastro/cadeia?codigo={codigo}"),
            Some(&gerente),
        ))
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(cadeia["integra"], true);
    assert!(cadeia["eventos"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["tipo"] == "dossie_emitido"));
}

#[tokio::test]
async fn dossie_so_para_gerencia_e_auditoria_e_codigo_desconhecido_nao_gera_nada() {
    let amb = Ambiente::novo().await;
    let (atd, _) = amb.ate_o_laudo(ANTONIO, NASC).await;
    for login in ["medico.a", "ana.souza", "ti.a"] {
        let s = amb.entrar(login).await;
        let r = bruto(
            &amb,
            amb.json(
                Method::POST,
                "/v1/lastro/dossie",
                &s,
                None,
                json!({ "codigo": atd["codigo"] }),
            ),
        )
        .await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{login}");
    }
    let gerente = amb.entrar("patricia.lemos").await;
    for (codigo, esperado) in [
        ("ATD-1900-999999", StatusCode::NOT_FOUND),
        ("ab", StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let r = bruto(
            &amb,
            amb.json(
                Method::POST,
                "/v1/lastro/dossie",
                &gerente,
                None,
                json!({ "codigo": codigo }),
            ),
        )
        .await;
        assert_eq!(r.status, esperado, "{codigo}: {}", r.json());
    }
    // Sem o token CSRF a emissão é recusada.
    let sem_csrf = Request::builder()
        .method(Method::POST)
        .uri("/v1/lastro/dossie")
        .header("content-type", "application/json")
        .header("cookie", &gerente.cookie)
        .body(Body::from(json!({ "codigo": atd["codigo"] }).to_string()))
        .unwrap();
    assert_eq!(bruto(&amb, sem_csrf).await.status, StatusCode::FORBIDDEN);
    assert_eq!(
        amb.contar("SELECT count(*) FROM documentos.emissao").await,
        0
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'dossie_emitido'")
            .await,
        0
    );
}
