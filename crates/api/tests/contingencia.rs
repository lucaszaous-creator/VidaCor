//! Contingência e lançamento posterior (docs/27 FC-CTG-01 a 11, docs/23). Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use chrono::Utc;
use comum::{Ambiente, UNIDADE_A};
use serde_json::{json, Value};
use uuid::Uuid;

const HASH_PDF: &str = "3f79bb7b435b05321651daefd374cdc681dc06faa65e374e38337b88ca046dea";

async fn pedir(
    amb: &Ambiente,
    login: &str,
    metodo: Method,
    uri: &str,
    corpo: Value,
) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, c) = amb.pedir(amb.json(metodo, uri, &s, None, corpo)).await;
    (st, c)
}

/// "Há `minutos` minutos", mas sempre dentro do dia de hoje na clínica: logo depois da meia-noite os
/// intervalos encolhem na mesma proporção (a ordem declaração, ficha e recibo se mantém). A faixa de
/// fichas é do dia; sem isso o teste falhava entre 00:00 e 00:41 sem bug do produto (DEC-2026-11-09-133).
fn antes(minutos: i64) -> chrono::DateTime<Utc> {
    let agora = Utc::now();
    let local = agora.with_timezone(&vidacor_api::tempo::FUSO);
    let desde_meia_noite = i64::from(chrono::Timelike::num_seconds_from_midnight(&local));
    let pedido = minutos * 60;
    let teto = 41 * 60;
    let segundos = if desde_meia_noite > teto {
        pedido
    } else {
        pedido * (desde_meia_noite - 1).max(0) / teto
    };
    agora - chrono::Duration::seconds(segundos)
}

async fn kit(amb: &Ambiente) -> Value {
    let (st, k) = pedir(
        amb,
        "patricia.lemos",
        Method::POST,
        "/v1/contingencias/kits",
        json!({}),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{k}");
    k
}

async fn declarar(amb: &Ambiente, chave: Uuid) -> (StatusCode, Value) {
    pedir(amb, "ana.souza", Method::POST, "/v1/contingencias", json!({
        "nivel": "total", "motivo": "servidor local fora do ar", "responsavel_id": amb.s.usuarios["patricia.lemos"],
        "declarada_em": antes(40), "chave": chave
    })).await
}

fn faixa(k: &Value, formulario: &str) -> (i64, i64) {
    let f = k["faixas"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["formulario"] == formulario)
        .unwrap();
    (f["inicio"].as_i64().unwrap(), f["fim"].as_i64().unwrap())
}

fn ficha(amb: &Ambiente, numero: i64, paciente: &str, nascimento: &str) -> Value {
    json!({
        "formulario": "F-ATD", "numero": numero, "agendamento_id": amb.s.agendamentos[paciente], "nome": paciente,
        "nascimento": nascimento, "ocorrido_em": antes(30),
        "atendido_por": amb.s.usuarios["medico.a"], "anexo_sha256": HASH_PDF
    })
}

#[tokio::test]
async fn fc_ctg_11_kit_do_dia_reserva_faixas_sem_sobrepor_e_grava_hash() {
    let amb = Ambiente::novo().await;
    let k1 = kit(&amb).await;
    assert_eq!(k1["modelo_versao"], "KIT-2026.1");
    assert_eq!(k1["hash"].as_str().unwrap().len(), 64);
    assert_eq!(faixa(&k1, "F-ATD"), (1, 100));
    let k2 = kit(&amb).await;
    assert_eq!(k1, k2, "o kit do dia é gerado uma vez");
    // Faixa sobreposta é recusada pelo banco (exclusão por intervalo).
    let sobreposta = sqlx::query("INSERT INTO contingencia.faixa (id, kit_id, formulario, dia, inicio, fim, origem, unidade_id, correlacao) SELECT $1, kit_id, 'F-ATD', dia + 1, 50, 150, 'teste', unidade_id, 'teste' FROM contingencia.faixa LIMIT 1")
        .bind(Uuid::now_v7()).execute(&amb.admin).await;
    assert!(sobreposta.is_err());
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'kit_gerado'")
            .await,
        1
    );
}

#[tokio::test]
async fn fc_ctg_01_e_02_declaracao_unica_com_a_hora_original() {
    let amb = Ambiente::novo().await;
    let chave = Uuid::now_v7();
    let mut eventos = amb.estado.eventos.subscribe();
    let (s1, c1) = declarar(&amb, chave).await;
    assert_eq!(s1, StatusCode::CREATED, "{c1}");
    assert_eq!(c1["situacao"], "aberta");
    assert_eq!(eventos.recv().await.unwrap().tipo, "contingencia-declarada");
    let (s2, c2) = declarar(&amb, chave).await;
    let (s3, c3) = declarar(&amb, chave).await;
    assert_eq!((s2, s3), (StatusCode::OK, StatusCode::OK));
    assert_eq!(c1["declarada_em"], c2["declarada_em"]);
    assert_eq!(c1["id"], c3["id"]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM contingencia.contingencia")
            .await,
        1
    );
}

#[tokio::test]
async fn fc_ctg_03_ficha_lancada_leva_a_marca_de_contingencia() {
    let amb = Ambiente::novo().await;
    let k = kit(&amb).await;
    let (_, c) = declarar(&amb, Uuid::now_v7()).await;
    let (n, _) = faixa(&k, "F-ATD");
    let (st, a) = pedir(
        &amb,
        "ana.souza",
        Method::POST,
        &format!("/v1/contingencias/{}/fichas", c["id"].as_str().unwrap()),
        ficha(&amb, n + 11, "Maria Helena Duarte", "1968-03-12"),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{a}");
    assert_eq!(a["origem"], "contingencia");
    assert_eq!(a["registrado_em_contingencia"], true);
    let (formulario, hash, origem): (String, String, String) =
        sqlx::query_as("SELECT formulario, anexo_sha256, origem FROM recepcao.atendimento")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(formulario, format!("F-ATD {:04}", n + 11));
    assert_eq!(hash, HASH_PDF);
    assert_eq!(origem, "contingencia");
    let dados: Value =
        sqlx::query_scalar("SELECT dados FROM lastro.evento WHERE tipo = 'atendimento_lancado'")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(dados["registrado_em_contingencia"], true);
    assert!(
        dados["ocorrido_em"].is_string()
            && dados["lancado_por"].is_string()
            && dados["atendido_por"].is_string()
    );
}

#[tokio::test]
async fn fc_ctg_04_mesma_ficha_em_duas_estacoes_gera_um_registro() {
    let amb = Ambiente::novo().await;
    let k = kit(&amb).await;
    let (_, c) = declarar(&amb, Uuid::now_v7()).await;
    let (n, _) = faixa(&k, "F-ATD");
    let uri = format!("/v1/contingencias/{}/fichas", c["id"].as_str().unwrap());
    let corpo = ficha(&amb, n + 11, "Maria Helena Duarte", "1968-03-12");
    let ((s1, _), (s2, _)) = tokio::join!(
        pedir(&amb, "ana.souza", Method::POST, &uri, corpo.clone()),
        pedir(&amb, "ana.souza", Method::POST, &uri, corpo)
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.atendimento")
            .await,
        1
    );
    let (_, de_novo) = pedir(
        &amb,
        "ana.souza",
        Method::POST,
        &uri,
        ficha(&amb, n + 11, "Maria Helena Duarte", "1968-03-12"),
    )
    .await;
    assert_eq!(de_novo["codigo"], "ficha-ja-lancada");
    assert!(de_novo["atendimento_codigo"]
        .as_str()
        .unwrap()
        .starts_with("ATD-"));
}

#[tokio::test]
async fn fc_ctg_05_ficha_fora_da_faixa_e_recusada() {
    let amb = Ambiente::novo().await;
    let k = kit(&amb).await;
    let (_, c) = declarar(&amb, Uuid::now_v7()).await;
    let (_, fim) = faixa(&k, "F-ATD");
    let (st, r) = pedir(
        &amb,
        "ana.souza",
        Method::POST,
        &format!("/v1/contingencias/{}/fichas", c["id"].as_str().unwrap()),
        ficha(&amb, fim + 112, "Maria Helena Duarte", "1968-03-12"),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r["codigo"], "numero-fora-da-faixa");
}

#[tokio::test]
async fn fc_ctg_06_nascimento_diferente_no_papel_abre_divergencia_sem_mexer_no_cadastro() {
    let amb = Ambiente::novo().await;
    let k = kit(&amb).await;
    let (_, c) = declarar(&amb, Uuid::now_v7()).await;
    let (n, _) = faixa(&k, "F-ATD");
    let (st, r) = pedir(
        &amb,
        "ana.souza",
        Method::POST,
        &format!("/v1/contingencias/{}/fichas", c["id"].as_str().unwrap()),
        ficha(&amb, n + 13, "Juliana Sampaio Rocha", "1992-04-12"),
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED, "{r}");
    assert_eq!(r["codigo"], "divergencia-aberta");
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.atendimento")
            .await,
        0
    );
    let nasc: chrono::NaiveDate = sqlx::query_scalar(
        "SELECT nascimento FROM identidade.paciente WHERE nome = 'Juliana Sampaio Rocha'",
    )
    .fetch_one(&amb.admin)
    .await
    .unwrap();
    assert_eq!(nasc.to_string(), "1993-04-12", "nenhum cadastro é alterado");
    assert_eq!(amb.contar("SELECT count(*) FROM recepcao.divergencia WHERE tipo = 'identificacao' AND situacao = 'aberta'").await, 1);
}

#[tokio::test]
async fn fc_ctg_07_valor_do_papel_diferente_grava_o_recalculado_e_abre_divergencia() {
    let amb = Ambiente::novo().await;
    let tabela = Uuid::now_v7();
    sqlx::query("INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, origem, unidade_id, correlacao) VALUES ($1, 12, current_date - 1, 'Particular', 'seed', $2, 'seed')")
        .bind(tabela).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO financeiro.preco (tabela_id, tuss, valor_centavos) VALUES ($1, '40901114', 38000)").bind(tabela).execute(&amb.admin).await.unwrap();
    let k = kit(&amb).await;
    let (_, c) = declarar(&amb, Uuid::now_v7()).await;
    let (n, _) = faixa(&k, "F-REC");
    let (st, r) = pedir(&amb, "ana.souza", Method::POST, &format!("/v1/contingencias/{}/recibos", c["id"].as_str().unwrap()), json!({
        "numero": n + 6, "paciente_id": amb.s.pacientes["Sônia Maria Castro"], "tuss": "40901114",
        "valor_papel_centavos": 30000, "ocorrido_em": antes(20)
    })).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    assert_eq!(r["valor_recalculado_centavos"], 38000);
    assert!(r["divergencia"].is_string());
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.divergencia WHERE tipo = 'preco'")
            .await,
        1
    );
}

#[tokio::test]
async fn fc_ctg_08_estudo_de_contingencia_vai_para_fila_propria_e_so_entra_no_laudo_conciliado() {
    let amb = Ambiente::novo().await;
    let atd = amb
        .fazer_checkin("Antônio Carlos Ribeiro", "1954-06-15")
        .await;
    let estudo = vidacor_api::orthanc::EstudoOrthanc {
        orthanc_id: "abcd-0414".into(),
        study_uid: "1.2.826.0.1.414".into(),
        accession: Some(atd["accession"].as_str().unwrap().into()),
        patient_id: Some("C-0414".into()),
        imagens: 8,
    };
    vidacor_api::outbox::registrar_estudo(&amb.estado, UNIDADE_A, &estudo)
        .await
        .unwrap();
    assert_eq!(amb.contar("SELECT count(*) FROM exames.estudo WHERE pendencia = 'conciliacao_contingencia' AND NOT conciliado").await, 1);
    assert_eq!(
        amb.contar("SELECT count(*) FROM laudos.laudo").await,
        0,
        "não vai direto ao laudo"
    );
    let id: Uuid = sqlx::query_scalar("SELECT id FROM exames.estudo")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    let uri = format!("/v1/estudos/{id}/conciliacao");
    let (errado, e) = pedir(&amb, "medico.a", Method::POST, &uri, json!({ "atendimento_id": atd["id"], "nome": "Antônio Carlos Ribeiro", "nascimento": "1954-06-16" })).await;
    assert_eq!(errado, StatusCode::UNPROCESSABLE_ENTITY, "{e}");
    let (ok, c) = pedir(&amb, "medico.a", Method::POST, &uri, json!({ "atendimento_id": atd["id"], "nome": "Antônio Carlos Ribeiro", "nascimento": "1954-06-15" })).await;
    assert_eq!(ok, StatusCode::OK, "{c}");
    assert_eq!(amb.contar("SELECT count(*) FROM laudos.laudo").await, 1);
}

#[tokio::test]
async fn fc_ctg_09_encerramento_bloqueado_com_pendencia_e_liberado_depois() {
    let amb = Ambiente::novo().await;
    let k = kit(&amb).await;
    let (_, c) = declarar(&amb, Uuid::now_v7()).await;
    let id = c["id"].as_str().unwrap();
    let (n, fim) = faixa(&k, "F-ATD");
    let (rn, rfim) = faixa(&k, "F-REC");
    let (_, div) = pedir(
        &amb,
        "ana.souza",
        Method::POST,
        &format!("/v1/contingencias/{id}/fichas"),
        ficha(&amb, n + 13, "Juliana Sampaio Rocha", "1992-04-12"),
    )
    .await;
    let (st, p) = pedir(
        &amb,
        "patricia.lemos",
        Method::POST,
        &format!("/v1/contingencias/{id}/encerramento"),
        json!({}),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(p["codigo"], "pendencias");
    let lista = p["pendencias"].as_array().unwrap();
    assert!(lista
        .iter()
        .any(|x| x.as_str().unwrap().contains("Divergência")));
    assert!(lista
        .iter()
        .any(|x| x.as_str().unwrap().contains("sem situação")));
    // Resolve a divergência e devolve os números não usados.
    let div_id: Uuid = sqlx::query_scalar("SELECT id FROM recepcao.divergencia WHERE codigo = $1")
        .bind(div["divergencia"].as_str().unwrap())
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    let (sr, _) = pedir(
        &amb,
        "patricia.lemos",
        Method::POST,
        &format!("/v1/divergencias/{div_id}/resolucao"),
        json!({ "resolucao": "nascimento do papel errado; ficha anulada e relançada" }),
    )
    .await;
    assert_eq!(sr, StatusCode::OK);
    for (form, de, ate) in [("F-ATD", n, fim), ("F-REC", rn, rfim)] {
        let (sd, _) = pedir(
            &amb,
            "ana.souza",
            Method::POST,
            &format!("/v1/contingencias/{id}/numeros"),
            json!({ "formulario": form, "de": de, "ate": ate, "situacao": "devolvido" }),
        )
        .await;
        assert_eq!(sd, StatusCode::OK);
    }
    let (se, e) = pedir(
        &amb,
        "patricia.lemos",
        Method::POST,
        &format!("/v1/contingencias/{id}/encerramento"),
        json!({}),
    )
    .await;
    assert_eq!(se, StatusCode::OK, "{e}");
    assert_eq!(e["situacao"], "encerrada");
}

#[tokio::test]
async fn fc_ctg_10_perfil_caixa_nao_declara_nem_encerra() {
    let amb = Ambiente::novo().await;
    let (s1, _) = pedir(&amb, "carla.caixa", Method::POST, "/v1/contingencias", json!({
        "nivel": "total", "motivo": "servidor fora", "responsavel_id": amb.s.usuarios["patricia.lemos"], "declarada_em": Utc::now(), "chave": Uuid::now_v7()
    })).await;
    assert_eq!(s1, StatusCode::FORBIDDEN);
    let (_, c) = declarar(&amb, Uuid::now_v7()).await;
    let (s2, _) = pedir(
        &amb,
        "carla.caixa",
        Method::POST,
        &format!(
            "/v1/contingencias/{}/encerramento",
            c["id"].as_str().unwrap()
        ),
        json!({}),
    )
    .await;
    assert_eq!(s2, StatusCode::FORBIDDEN);
}
