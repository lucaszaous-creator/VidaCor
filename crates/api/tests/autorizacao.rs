//! Autorização pedida pelo sistema (docs/22 E4, docs/25 § 3.3): rota HTTP real, PostgreSQL real e a
//! operadora como único ponto simulado (G4). Frases "quando…, então…" sem ID FC-* aprovado ainda
//! (DEC-2026-11-08-80).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use chrono::{Duration, TimeZone, Utc};
use comum::{Ambiente, Sessao, UNIDADE_A};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use vidacor_api::operadora::{OperadoraDesenvolvimento, Roteiro};
use vidacor_dominio_faturamento::autorizacao::StatusAutorizacao;

const ECO: &str = "40901114";
const DOPPLER: &str = "40901300";

fn amanha_as(hora: u32) -> chrono::DateTime<Utc> {
    let dia = vidacor_api::tempo::hoje_local() + Duration::days(1);
    chrono_tz::America::Sao_Paulo
        .from_local_datetime(&dia.and_hms_opt(hora, 0, 0).unwrap())
        .unwrap()
        .with_timezone(&Utc)
}

async fn com_operadora(amb: &mut Ambiente) -> Arc<OperadoraDesenvolvimento> {
    let op = Arc::new(OperadoraDesenvolvimento::default());
    amb.estado.operadora = Some(op.clone());
    amb.app = vidacor_api::aplicacao(amb.estado.clone());
    op
}

/// Contrato Unimed que recebe pedido de autorização pelo webservice.
async fn contrato_unimed(amb: &Ambiente) {
    sqlx::query("INSERT INTO faturamento.contrato (id, codigo, operadora, registro_ans, codigo_prestador, nome_prestador, cnes,
                   regra_versao, prazo_envio_dias, prazo_recurso_dias, autorizacao_por_webservice, origem, unidade_id, correlacao)
                 VALUES ($1, 'CTO-SEED-1', 'Unimed', '123456', '0004567', 'VidaCor Imagem Unidade Centro', '1234567',
                   'AUD-UNIMED-2026.1', 30, 30, true, 'seed', $2, 'seed')")
        .bind(Uuid::now_v7()).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
}

/// Reserva provisória de Antônio amanhã, com eco e Doppler (sem guia ainda).
async fn reserva(amb: &Ambiente, hora: u32) -> Uuid {
    let ana = amb.entrar("ana.souza").await;
    let (st, _, a) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/agendamentos",
            &ana,
            Some(Uuid::now_v7()),
            json!({
                "paciente_id": amb.antonio, "sala_id": amb.sala1, "inicio": amanha_as(hora),
                "procedimentos": [ECO, DOPPLER], "guia_id": null, "reserva_provisoria": true
            }),
        ))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{a}");
    assert_eq!(a["situacao"], "reserva_provisoria");
    Uuid::parse_str(a["id"].as_str().unwrap()).unwrap()
}

fn pedido(agendamento: Uuid, operadora: &str) -> Value {
    json!({
        "agendamento_id": agendamento, "operadora": operadora, "carteirinha": "0 123 4567",
        "solicitante": { "nome": "Dr. Júlio Prado", "conselho": "06", "numero": "52123456", "uf": "33", "cbos": "225125" },
        "indicacao_clinica": "Sopro cardíaco a esclarecer"
    })
}

async fn pedir(
    amb: &Ambiente,
    s: &Sessao,
    metodo: Method,
    uri: &str,
    chave: Option<Uuid>,
    corpo: Value,
) -> (StatusCode, Value) {
    let req = if metodo == Method::GET {
        amb.get(uri, Some(s))
    } else {
        amb.json(metodo, uri, s, chave, corpo)
    };
    let (st, _, c) = amb.pedir(req).await;
    (st, c)
}

async fn processar_outbox(amb: &Ambiente) {
    let orthanc = vidacor_api::orthanc::Orthanc::novo("http://127.0.0.1:9").unwrap();
    while vidacor_api::outbox::processar_um(&amb.estado, &orthanc)
        .await
        .unwrap()
    {}
}

fn roteiro(status: StatusAutorizacao, eco: u16, doppler: u16, motivos: &[&str]) -> Roteiro {
    Roteiro {
        status,
        procedimentos: if eco + doppler == 0 && status != StatusAutorizacao::Parcial {
            vec![]
        } else {
            vec![(ECO.into(), 1, eco), (DOPPLER.into(), 1, doppler)]
        },
        motivos: motivos.iter().map(|m| (*m).to_owned()).collect(),
    }
}

async fn detalhe(amb: &Ambiente, s: &Sessao, id: &str) -> Value {
    let (st, d) = pedir(
        amb,
        s,
        Method::GET,
        &format!("/v1/autorizacoes/{id}"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{d}");
    d
}

async fn situacao_do_agendamento(amb: &Ambiente, id: Uuid) -> String {
    sqlx::query_scalar("SELECT situacao FROM agenda.agendamento WHERE id = $1")
        .bind(id)
        .fetch_one(&amb.admin)
        .await
        .unwrap()
}

#[tokio::test]
async fn quando_a_operadora_autoriza_entao_a_guia_e_preenchida_sem_digitacao_e_a_reserva_confirmada(
) {
    let mut amb = Ambiente::novo().await;
    let op = com_operadora(&mut amb).await;
    op.programar(vec![roteiro(StatusAutorizacao::Autorizada, 1, 1, &[])]);
    contrato_unimed(&amb).await;
    let ag = reserva(&amb, 10).await;
    let ana = amb.entrar("ana.souza").await;
    let (st, p) = pedir(
        &amb,
        &ana,
        Method::POST,
        "/v1/autorizacoes",
        Some(Uuid::now_v7()),
        pedido(ag, "Unimed"),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    assert_eq!(p["resumo"]["canal"], "webservice");
    assert_eq!(p["resumo"]["situacao"], "aguardando_envio");
    assert_eq!(
        p["resumo"]["paciente"],
        json!({ "nome": "Antônio Carlos Ribeiro", "nascimento": "1954-06-15" })
    );
    let id = p["resumo"]["id"].as_str().unwrap().to_owned();

    // A mensagem enviada passa no XSD oficial e o envio sai com a chave do pedido.
    let (xml, chave): (Vec<u8>, Uuid) = sqlx::query_as(
        "SELECT solicitacao, chave_envio FROM convenio.autorizacao WHERE id = $1::uuid",
    )
    .bind(&id)
    .fetch_one(&amb.admin)
    .await
    .unwrap();
    let caminho = std::env::temp_dir().join(format!("autorizacao-{id}.xml"));
    std::fs::write(&caminho, &xml).unwrap();
    let xsd = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../dominio-faturamento/xsd/tiss-4.03.00/tissV4_03_00.xsd"
    );
    let v = std::process::Command::new("xmllint")
        .args(["--noout", "--nonet", "--schema", xsd])
        .arg(&caminho)
        .output()
        .expect("xmllint");
    let _ = std::fs::remove_file(&caminho);
    assert!(v.status.success(), "{}", String::from_utf8_lossy(&v.stderr));
    let chave_outbox: Uuid = sqlx::query_scalar("SELECT chave_idempotencia FROM integracao.outbox WHERE tipo = 'tiss.solicitar_autorizacao'")
        .fetch_one(&amb.admin).await.unwrap();
    assert_eq!(chave_outbox, chave);

    processar_outbox(&amb).await;
    let d = detalhe(&amb, &ana, &id).await;
    assert_eq!(d["resumo"]["situacao"], "autorizada", "{d}");
    let guia = &d["guia"];
    assert_eq!(
        guia["senha"],
        format!(
            "DEV-{}",
            p["resumo"]["codigo"]
                .as_str()
                .unwrap()
                .replace(['A', 'U', 'T', '-'], "")
        )
    );
    assert_eq!(
        guia["validade"],
        json!(vidacor_api::tempo::hoje_local() + Duration::days(30))
    );
    assert_eq!(
        guia["situacao"], "consumida",
        "a reserva foi confirmada com a guia"
    );
    assert_eq!(situacao_do_agendamento(&amb, ag).await, "confirmado");
    assert_eq!(
        d["itens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["autorizado"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 1]
    );
    // Resposta gravada como recebida: bruta, com o SHA-256 que confere.
    let (bruta, sha): (Vec<u8>, String) =
        sqlx::query_as("SELECT bruta, sha256 FROM convenio.autorizacao_resposta")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    use sha2::Digest as _;
    assert_eq!(
        sha,
        sha2::Sha256::digest(&bruta)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    assert_eq!(d["respostas"][0]["hash_confere"], true);
    let saldos: Vec<i32> =
        sqlx::query_scalar("SELECT saldo FROM convenio.guia_procedimento ORDER BY tuss")
            .fetch_all(&amb.admin)
            .await
            .unwrap();
    assert_eq!(saldos, vec![0, 0]);
    // A resposta é imutável.
    assert!(
        sqlx::query("UPDATE convenio.autorizacao_resposta SET aplicada = false")
            .execute(&amb.admin)
            .await
            .is_err()
    );
    let tipos: Vec<String> = sqlx::query_scalar(
        "SELECT tipo FROM lastro.evento WHERE dominio = 'convenio' ORDER BY sequencia",
    )
    .fetch_all(&amb.admin)
    .await
    .unwrap();
    assert!(
        tipos.contains(&"autorizacao_pedida".to_owned())
            && tipos.contains(&"autorizacao_respondida".to_owned()),
        "{tipos:?}"
    );
    // Retentativa do envio (trabalhador caiu antes de gravar) não muda nada nem pede outra senha.
    vidacor_api::rotas::autorizacoes_resposta::enviar_pelo_outbox(
        &amb.estado,
        UNIDADE_A,
        Uuid::parse_str(&id).unwrap(),
        chave,
    )
    .await
    .unwrap();
    assert_eq!(op.solicitacoes_recebidas(), 1);
}

#[tokio::test]
async fn negada_e_parcial_ficam_visiveis_com_o_motivo() {
    let mut amb = Ambiente::novo().await;
    let op = com_operadora(&mut amb).await;
    op.programar(vec![
        roteiro(StatusAutorizacao::Negada, 0, 0, &["1423"]),
        roteiro(StatusAutorizacao::Autorizada, 1, 0, &[]),
    ]);
    contrato_unimed(&amb).await;
    let ag = reserva(&amb, 10).await;
    let ana = amb.entrar("ana.souza").await;
    let (_, p) = pedir(
        &amb,
        &ana,
        Method::POST,
        "/v1/autorizacoes",
        Some(Uuid::now_v7()),
        pedido(ag, "Unimed"),
    )
    .await;
    processar_outbox(&amb).await;
    let (_, lista) = pedir(
        &amb,
        &ana,
        Method::GET,
        "/v1/autorizacoes?situacao=negada",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(lista[0]["id"], p["resumo"]["id"]);
    assert_eq!(lista[0]["motivo"], "Código(s) da operadora: 1423");
    assert_eq!(
        situacao_do_agendamento(&amb, ag).await,
        "reserva_provisoria"
    );
    assert_eq!(amb.contar("SELECT count(*) FROM convenio.guia").await, 0);

    // Depois da negativa, um pedido novo é aceito; a operadora autoriza só o eco: parcial.
    let (st, p2) = pedir(
        &amb,
        &ana,
        Method::POST,
        "/v1/autorizacoes",
        Some(Uuid::now_v7()),
        pedido(ag, "Unimed"),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{p2}");
    processar_outbox(&amb).await;
    let d = detalhe(&amb, &ana, p2["resumo"]["id"].as_str().unwrap()).await;
    assert_eq!(d["resumo"]["situacao"], "parcial");
    assert_eq!(
        d["resumo"]["motivo"],
        "Autorizado a menos: 40901300 (0 de 1)"
    );
    assert_eq!(
        d["guia"]["situacao"], "disponivel",
        "parcial não confirma a reserva sozinha"
    );
    let autorizados: Vec<(String, i32)> =
        sqlx::query_as("SELECT tuss, autorizado FROM convenio.guia_procedimento")
            .fetch_all(&amb.admin)
            .await
            .unwrap();
    assert_eq!(autorizados, vec![(ECO.to_owned(), 1)]);
    assert_eq!(
        situacao_do_agendamento(&amb, ag).await,
        "reserva_provisoria"
    );
}

#[tokio::test]
async fn em_analise_e_consultada_de_novo_e_a_resposta_repetida_nao_duplica() {
    let mut amb = Ambiente::novo().await;
    let op = com_operadora(&mut amb).await;
    op.programar(vec![
        roteiro(StatusAutorizacao::EmAnalise, 0, 0, &[]),
        roteiro(StatusAutorizacao::EmAnalise, 0, 0, &[]),
        roteiro(StatusAutorizacao::Autorizada, 1, 1, &[]),
    ]);
    contrato_unimed(&amb).await;
    let ag = reserva(&amb, 10).await;
    let ana = amb.entrar("ana.souza").await;
    let (_, p) = pedir(
        &amb,
        &ana,
        Method::POST,
        "/v1/autorizacoes",
        Some(Uuid::now_v7()),
        pedido(ag, "Unimed"),
    )
    .await;
    let id = p["resumo"]["id"].as_str().unwrap().to_owned();
    processar_outbox(&amb).await;
    let d = detalhe(&amb, &ana, &id).await;
    assert_eq!(d["resumo"]["situacao"], "em_analise");
    assert_eq!(
        d["pendente_no_outbox"]["tentativas"], 0,
        "consulta agendada com espera"
    );
    // "Consultar agora": a operadora segue analisando e responde o mesmo documento.
    let (st, _) = pedir(
        &amb,
        &ana,
        Method::POST,
        &format!("/v1/autorizacoes/{id}/consulta"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED);
    processar_outbox(&amb).await;
    let d = detalhe(&amb, &ana, &id).await;
    assert_eq!(d["resumo"]["situacao"], "em_analise");
    assert_eq!(
        d["respostas"].as_array().unwrap().len(),
        1,
        "resposta repetida não duplica"
    );
    assert!(d["pendente_no_outbox"]["ultimo_erro"]
        .as_str()
        .unwrap()
        .contains("ainda está analisando"));
    let (espera, tentativas): (bool, i32) = sqlx::query_as("SELECT proximo_em > now(), tentativas FROM integracao.outbox WHERE tipo = 'tiss.consultar_autorizacao'")
        .fetch_one(&amb.admin).await.unwrap();
    assert!(
        espera && tentativas == 1,
        "nova consulta com espera crescente"
    );
    // Na consulta seguinte a operadora autoriza: guia preenchida e reserva confirmada.
    pedir(
        &amb,
        &ana,
        Method::POST,
        &format!("/v1/autorizacoes/{id}/consulta"),
        None,
        Value::Null,
    )
    .await;
    processar_outbox(&amb).await;
    let d = detalhe(&amb, &ana, &id).await;
    assert_eq!(d["resumo"]["situacao"], "autorizada", "{d}");
    assert_eq!(d["pendente_no_outbox"], Value::Null);
    assert_eq!(situacao_do_agendamento(&amb, ag).await, "confirmado");
    assert_eq!(op.consultas.load(std::sync::atomic::Ordering::SeqCst), 2);
    // Nem por fora: a mesma resposta não entra duas vezes.
    let dup = sqlx::query("INSERT INTO convenio.autorizacao_resposta (id, autorizacao_id, origem_resposta, bruta, sha256, situacao_lida, aplicada, origem, unidade_id, correlacao)
                           SELECT $1, autorizacao_id, origem_resposta, bruta, sha256, situacao_lida, false, 'teste', unidade_id, 'teste' FROM convenio.autorizacao_resposta LIMIT 1")
        .bind(Uuid::now_v7()).execute(&amb.admin).await;
    assert!(dup
        .unwrap_err()
        .to_string()
        .contains("autorizacao_resposta_autorizacao_id_sha256_key"));
}

#[tokio::test]
async fn dois_pedidos_simultaneos_para_o_mesmo_agendamento_dao_um_so() {
    let amb = Ambiente::novo().await;
    contrato_unimed(&amb).await;
    let ag = reserva(&amb, 10).await;
    let ana = amb.entrar("ana.souza").await;
    let rita = amb.entrar("rita.faturamento").await;
    let (a, b) = tokio::join!(
        pedir(
            &amb,
            &ana,
            Method::POST,
            "/v1/autorizacoes",
            Some(Uuid::now_v7()),
            pedido(ag, "Unimed")
        ),
        pedir(
            &amb,
            &rita,
            Method::POST,
            "/v1/autorizacoes",
            Some(Uuid::now_v7()),
            pedido(ag, "Unimed")
        ),
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(
        st,
        [StatusCode::CREATED, StatusCode::CONFLICT],
        "{} / {}",
        a.1,
        b.1
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM convenio.autorizacao")
            .await,
        1
    );
    assert_eq!(
        amb.contar(
            "SELECT count(*) FROM integracao.outbox WHERE tipo = 'tiss.solicitar_autorizacao'"
        )
        .await,
        1
    );
    // Mesma chave: mesma resposta, sem pedido novo.
    let chave = Uuid::now_v7();
    let ag2 = reserva(&amb, 11).await;
    let (s1, r1) = pedir(
        &amb,
        &ana,
        Method::POST,
        "/v1/autorizacoes",
        Some(chave),
        pedido(ag2, "Unimed"),
    )
    .await;
    let (s2, r2) = pedir(
        &amb,
        &ana,
        Method::POST,
        "/v1/autorizacoes",
        Some(chave),
        pedido(ag2, "Unimed"),
    )
    .await;
    assert_eq!((s1, &r1), (s2, &r2));
    assert_eq!(
        amb.contar("SELECT count(*) FROM convenio.autorizacao")
            .await,
        2
    );
    // Sem webservice configurado no núcleo, o envio fica pendente e visível (R7).
    processar_outbox(&amb).await;
    let d = detalhe(&amb, &ana, r1["resumo"]["id"].as_str().unwrap()).await;
    assert_eq!(d["resumo"]["situacao"], "aguardando_envio");
    assert!(d["pendente_no_outbox"]["ultimo_erro"]
        .as_str()
        .unwrap()
        .contains("não configurado"));
}

#[tokio::test]
async fn operadora_sem_webservice_pedido_pelo_portal_com_protocolo_e_resposta_registrada() {
    let amb = Ambiente::novo().await;
    let ag = reserva(&amb, 10).await;
    let rita = amb.entrar("rita.faturamento").await;
    let (st, p) = pedir(
        &amb,
        &rita,
        Method::POST,
        "/v1/autorizacoes",
        Some(Uuid::now_v7()),
        pedido(ag, "Petrobras"),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    assert_eq!(p["resumo"]["canal"], "portal");
    assert_eq!(p["resumo"]["situacao"], "aguardando_portal");
    let id = p["resumo"]["id"].as_str().unwrap().to_owned();
    let resposta = json!({ "situacao": "autorizada", "numero_guia_operadora": "PB-55120", "senha": "PB-10377",
        "data_autorizacao": vidacor_api::tempo::hoje_local(), "validade": vidacor_api::tempo::hoje_local() + Duration::days(30),
        "procedimentos": [{ "tuss": ECO, "autorizado": 1 }, { "tuss": DOPPLER, "autorizado": 1 }] });
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/autorizacoes/{id}/resposta"),
        Some(Uuid::now_v7()),
        resposta.clone(),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("sem-protocolo"))
    );
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/autorizacoes/{id}/protocolo"),
        None,
        json!({ "protocolo": "PB 2026<script>" }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("protocolo-invalido"))
    );
    let (st, d) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/autorizacoes/{id}/protocolo"),
        None,
        json!({ "protocolo": "PB-2026-8812" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{d}");
    assert_eq!(d["resumo"]["situacao"], "enviada");
    for (corpo, codigo) in [
        (
            json!({ "situacao": "autorizada", "procedimentos": [{ "tuss": ECO, "autorizado": 1 }] }),
            "resposta-invalida",
        ),
        (json!({ "situacao": "negada" }), "resposta-invalida"),
        (json!({ "situacao": "talvez" }), "resposta-invalida"),
        (
            json!({ "situacao": "autorizada", "senha": "PB-1", "validade": vidacor_api::tempo::hoje_local(), "procedimentos": [{ "tuss": "41001109", "autorizado": 1 }] }),
            "procedimento-fora-do-pedido",
        ),
    ] {
        let (st, e) = pedir(
            &amb,
            &rita,
            Method::POST,
            &format!("/v1/autorizacoes/{id}/resposta"),
            Some(Uuid::now_v7()),
            corpo,
        )
        .await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo)),
            "{e}"
        );
    }
    let chave = Uuid::now_v7();
    let (st, d) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/autorizacoes/{id}/resposta"),
        Some(chave),
        resposta.clone(),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{d}");
    assert_eq!(d["resumo"]["situacao"], "autorizada");
    assert_eq!(d["guia"]["numero"], "PB-55120");
    assert_eq!(d["respostas"][0]["origem"], "portal");
    assert_eq!(situacao_do_agendamento(&amb, ag).await, "confirmado");
    let (st2, d2) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/autorizacoes/{id}/resposta"),
        Some(chave),
        resposta.clone(),
    )
    .await;
    assert_eq!((st2, &d2), (st, &d), "mesma chave, mesma resposta");
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/autorizacoes/{id}/resposta"),
        Some(Uuid::now_v7()),
        resposta,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("autorizacao-respondida"))
    );
    assert_eq!(amb.contar("SELECT count(*) FROM convenio.guia").await, 1);
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/autorizacoes/{id}/consulta"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("consulta-fora-de-hora"))
    );
}

#[tokio::test]
async fn perfis_indevidos_outra_unidade_e_limites() {
    let amb = Ambiente::novo().await;
    contrato_unimed(&amb).await;
    let ag = reserva(&amb, 10).await;
    for login in [
        "carla.caixa",
        "medico.a",
        "bruno.sales",
        "financeiro.a",
        "ti.a",
        "patricia.lemos",
    ] {
        let s = amb.entrar(login).await;
        let (st, _) = pedir(
            &amb,
            &s,
            Method::POST,
            "/v1/autorizacoes",
            Some(Uuid::now_v7()),
            pedido(ag, "Unimed"),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
        let (st, _) = pedir(&amb, &s, Method::GET, "/v1/autorizacoes", None, Value::Null).await;
        assert_eq!(
            st,
            if login == "patricia.lemos" {
                StatusCode::OK
            } else {
                StatusCode::FORBIDDEN
            },
            "{login} lê"
        );
    }
    let ana = amb.entrar("ana.souza").await;
    let (_, p) = pedir(
        &amb,
        &ana,
        Method::POST,
        "/v1/autorizacoes",
        Some(Uuid::now_v7()),
        pedido(ag, "Unimed"),
    )
    .await;
    let id = p["resumo"]["id"].as_str().unwrap().to_owned();
    // Outra unidade: não vê o pedido nem pede para o agendamento daqui.
    let norte = amb.entrar("recepcao.b").await;
    let (_, l) = pedir(
        &amb,
        &norte,
        Method::GET,
        "/v1/autorizacoes",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(l, json!([]));
    let (st, _) = pedir(
        &amb,
        &norte,
        Method::GET,
        &format!("/v1/autorizacoes/{id}"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = pedir(
        &amb,
        &norte,
        Method::POST,
        "/v1/autorizacoes",
        Some(Uuid::now_v7()),
        pedido(ag, "Unimed"),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // RLS direto: caixa, médico e financeiro não leem pedidos de autorização.
    for perfil in ["caixa", "medico", "financeiro", "ti"] {
        let mut tx = amb.app_pool.begin().await.unwrap();
        sqlx::query(
            "SELECT set_config('app.unidade_id', $1, true), set_config('app.perfis', $2, true)",
        )
        .bind(UNIDADE_A.to_string())
        .bind(perfil)
        .execute(&mut *tx)
        .await
        .unwrap();
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM convenio.autorizacao")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(n, 0, "{perfil}");
    }
    // Limites e erros de preenchimento.
    let ag2 = reserva(&amb, 11).await;
    let mut ruim = pedido(ag2, "Unimed");
    ruim["solicitante"]["uf"] = json!("RJ");
    let mut carteira = pedido(ag2, "Unimed");
    carteira["carteirinha"] = json!("123<456>");
    let mut longa = pedido(ag2, "Unimed");
    longa["indicacao_clinica"] = json!("x".repeat(501));
    for (corpo, st_esperado, codigo) in [
        (
            ruim,
            StatusCode::UNPROCESSABLE_ENTITY,
            "solicitante-invalido",
        ),
        (
            carteira,
            StatusCode::UNPROCESSABLE_ENTITY,
            "carteirinha-invalida",
        ),
        (longa, StatusCode::UNPROCESSABLE_ENTITY, "indicacao-longa"),
        (
            pedido(Uuid::now_v7(), "Unimed"),
            StatusCode::NOT_FOUND,
            "agendamento-nao-encontrado",
        ),
        (
            pedido(ag, "Unimed"),
            StatusCode::CONFLICT,
            "autorizacao-em-andamento",
        ),
    ] {
        let (st, e) = pedir(
            &amb,
            &ana,
            Method::POST,
            "/v1/autorizacoes",
            Some(Uuid::now_v7()),
            corpo,
        )
        .await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (st_esperado, Some(codigo)),
            "{e}"
        );
    }
    sqlx::query(
        "UPDATE agenda.agendamento SET expira_em = now() - interval '1 minute' WHERE id = $1",
    )
    .bind(ag2)
    .execute(&amb.admin)
    .await
    .unwrap();
    let (st, e) = pedir(
        &amb,
        &ana,
        Method::POST,
        "/v1/autorizacoes",
        Some(Uuid::now_v7()),
        pedido(ag2, "Unimed"),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("agendamento-encerrado"))
    );
    // Pedido do webservice não aceita resposta digitada.
    let (st, e) = pedir(
        &amb,
        &ana,
        Method::POST,
        &format!("/v1/autorizacoes/{id}/resposta"),
        Some(Uuid::now_v7()),
        json!({ "situacao": "negada", "motivo": "teste" }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("resposta-pelo-webservice"))
    );
}
