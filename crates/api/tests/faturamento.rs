//! Faturamento TISS com auditoria antes do lote (docs/22 E5, docs/25 § 3.3): rota HTTP real,
//! PostgreSQL real e a operadora como único ponto simulado (G4). Frases "quando…, então…" sem ID
//! FC-* aprovado ainda (DEC-2026-11-08-65: a aprovar pelo dono do produto).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use chrono::Duration;
use comum::{Ambiente, Sessao, UNIDADE_A};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use vidacor_api::operadora::OperadoraDesenvolvimento;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const NASC_ANTONIO: &str = "1954-06-15";
const MARIA: &str = "Maria Helena Duarte";
const NASC_MARIA: &str = "1968-03-12";
const ECO: &str = "40901114";

/// Contrato Unimed com tabela vigente (ECO a R$ 248,60), como o cadastro de convênios faria.
async fn contrato_unimed(amb: &Ambiente) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO faturamento.contrato (id, codigo, operadora, registro_ans, codigo_prestador, nome_prestador, cnes,
                   regra_versao, prazo_envio_dias, prazo_recurso_dias, origem, unidade_id, correlacao)
                 VALUES ($1, 'CTO-SEED-1', 'Unimed', '123456', '0004567', 'VidaCor Imagem Unidade Centro', '1234567',
                   'AUD-UNIMED-2026.1', 30, 30, 'seed', $2, 'seed')")
        .bind(id).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    tabela(amb, id, 1, 30, 24_860).await;
    id
}

async fn tabela(amb: &Ambiente, contrato: Uuid, versao: i32, dias_atras: i64, eco: i64) {
    let t = Uuid::now_v7();
    let desde = amb.dia - Duration::days(dias_atras);
    sqlx::query("INSERT INTO faturamento.tabela (id, codigo, contrato_id, versao, vigente_desde, descricao, origem, unidade_id, correlacao)
                 VALUES ($1, $2, $3, $4, $5, 'CBHPM 5ª ed. menos 10%', 'seed', $6, 'seed')")
        .bind(t).bind(format!("TBO-SEED-{versao}")).bind(contrato).bind(versao).bind(desde).bind(UNIDADE_A)
        .execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO faturamento.preco (tabela_id, tuss, descricao, valor_centavos, unidade_id) VALUES ($1, $2, 'Ecocardiograma transtorácico', $3, $4)")
        .bind(t).bind(ECO).bind(eco).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
}

/// Guia autorizada registrada pela recepção e consumida pelo agendamento de hoje do paciente.
async fn guia_consumida(amb: &Ambiente, paciente: &str, numero: &str) -> Uuid {
    let ana = amb.entrar("ana.souza").await;
    let validade = amb.dia + Duration::days(10);
    let (st, _, g) = amb
        .pedir(amb.json(Method::POST, "/v1/guias", &ana, Some(Uuid::now_v7()), json!({
            "operadora": "Unimed", "numero": numero, "carteirinha": "0 123 4567", "senha": "UN-88231-X",
            "validade": validade, "paciente_id": amb.s.pacientes[paciente], "solicitante": "Dr. Júlio Prado",
            "procedimentos": [{ "tuss": ECO, "quantidade": 1 }]
        })))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{g}");
    let guia = Uuid::parse_str(g["id"].as_str().unwrap()).unwrap();
    let ag = amb.s.agendamentos[paciente];
    sqlx::query(
        "UPDATE convenio.guia SET situacao = 'consumida', agendamento_id = $2 WHERE id = $1",
    )
    .bind(guia)
    .bind(ag)
    .execute(&amb.admin)
    .await
    .unwrap();
    sqlx::query("UPDATE agenda.agendamento SET guia_id = $1 WHERE id = $2")
        .bind(guia)
        .bind(ag)
        .execute(&amb.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE convenio.guia_procedimento SET saldo = 0 WHERE guia_id = $1")
        .bind(guia)
        .execute(&amb.admin)
        .await
        .unwrap();
    guia
}

/// Exame feito e laudo assinado pelo Dr. Paulo (o agendamento da Maria passa a ser dele antes).
async fn exame_com_laudo(amb: &Ambiente, paciente: &str, nasc: &str, assinar: bool) {
    sqlx::query("UPDATE agenda.agendamento SET medico_id = $2 WHERE id = $1")
        .bind(amb.s.agendamentos[paciente])
        .bind(amb.s.usuarios["medico.a"])
        .execute(&amb.admin)
        .await
        .unwrap();
    let (_, laudo) = amb.ate_o_laudo(paciente, nasc).await;
    if !assinar {
        return;
    }
    let m = amb.entrar("medico.a").await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::PUT,
            &format!("/v1/laudos/{laudo}/rascunho"),
            &m,
            None,
            json!({ "texto": "Exame dentro da normalidade.", "versao_numero": 1 }),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/laudos/{laudo}/assinatura"),
            &m,
            Some(Uuid::now_v7()),
            json!({ "pin": "4821" }),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{c}");
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

async fn conferir(amb: &Ambiente, s: &Sessao) -> Value {
    let (st, r) = pedir(
        amb,
        s,
        Method::POST,
        "/v1/faturamento/conferencias",
        Some(Uuid::now_v7()),
        json!({}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    r
}

async fn conta_de(amb: &Ambiente, paciente: &str) -> Uuid {
    sqlx::query_scalar("SELECT id FROM faturamento.conta WHERE paciente_id = $1")
        .bind(amb.s.pacientes[paciente])
        .fetch_one(&amb.admin)
        .await
        .unwrap()
}

async fn completar_solicitante(amb: &Ambiente, s: &Sessao, conta: Uuid) -> Value {
    let (st, d) = pedir(amb, s, Method::POST, &format!("/v1/faturamento/contas/{conta}/correcao"), Some(Uuid::now_v7()),
        json!({ "motivo": "CRM do pedido médico", "solicitante": { "conselho": "06", "numero": "52123456", "uf": "33", "cbos": "225125" } })).await;
    assert_eq!(st, StatusCode::OK, "{d}");
    d
}

fn regras(d: &Value) -> Vec<&str> {
    d["pendencias"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["regra"].as_str().unwrap())
        .collect()
}

async fn processar_outbox(amb: &Ambiente) {
    let orthanc = vidacor_api::orthanc::Orthanc::novo("http://127.0.0.1:9").unwrap();
    while vidacor_api::outbox::processar_um(&amb.estado, &orthanc)
        .await
        .unwrap()
    {}
}

/// Antônio pronto para o lote: contrato, guia consumida, laudo assinado e solicitante completo.
async fn antonio_pronto(amb: &Ambiente, rita: &Sessao) -> (Uuid, Uuid) {
    let contrato = contrato_unimed(amb).await;
    guia_consumida(amb, ANTONIO, "UN-55120").await;
    exame_com_laudo(amb, ANTONIO, NASC_ANTONIO, true).await;
    conferir(amb, rita).await;
    let conta = conta_de(amb, ANTONIO).await;
    let d = completar_solicitante(amb, rita, conta).await;
    assert_eq!(d["resumo"]["situacao"], "pronta", "{d}");
    (contrato, conta)
}

#[tokio::test]
async fn quando_a_guia_passa_na_auditoria_entao_vai_no_lote_tiss_valido_e_e_enviada_uma_vez() {
    let mut amb = Ambiente::novo().await;
    let operadora = Arc::new(OperadoraDesenvolvimento::default());
    amb.estado.operadora = Some(operadora.clone());
    amb.app = vidacor_api::aplicacao(amb.estado.clone());
    let rita = amb.entrar("rita.faturamento").await;
    let contrato = contrato_unimed(&amb).await;
    guia_consumida(&amb, ANTONIO, "UN-55120").await;
    exame_com_laudo(&amb, ANTONIO, NASC_ANTONIO, true).await;

    let r = conferir(&amb, &rita).await;
    assert_eq!(
        r,
        json!({ "novas": 1, "conferidas": 1, "prontas": 0, "com_pendencia": 1 })
    );
    let conta = conta_de(&amb, ANTONIO).await;
    let (_, d) = pedir(
        &amb,
        &rita,
        Method::GET,
        &format!("/v1/faturamento/contas/{conta}"),
        None,
        Value::Null,
    )
    .await;
    // A guia da recepção só traz o nome do solicitante: o TISS exige conselho, número, UF e CBO.
    assert_eq!(regras(&d), vec!["solicitante-incompleto"]);
    assert_eq!(d["pendencias"][0]["regra_versao"], "AUD-UNIMED-2026.1");
    assert_eq!(
        d["resumo"]["paciente"],
        json!({ "nome": ANTONIO, "nascimento": NASC_ANTONIO })
    );
    assert_eq!(d["resumo"]["valor_total_centavos"], 24_860);

    let d = completar_solicitante(&amb, &rita, conta).await;
    assert_eq!(d["resumo"]["situacao"], "pronta");
    assert_eq!(d["historico"][0]["motivo"], "CRM do pedido médico");
    assert_eq!(d["historico"][0]["versao"], 2, "versão anterior guardada");
    let anterior: Value =
        sqlx::query_scalar("SELECT anterior FROM faturamento.conta_historico WHERE conta_id = $1")
            .bind(conta)
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(
        anterior["solicitante_conselho"],
        Value::Null,
        "o estado anterior fica no histórico"
    );

    let (st, l) = pedir(
        &amb,
        &rita,
        Method::POST,
        "/v1/faturamento/lotes",
        Some(Uuid::now_v7()),
        json!({ "contrato_id": contrato }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{l}");
    assert_eq!(l["lote"]["numero"], 1);
    assert_eq!(l["lote"]["versao_tiss"], "4.03.00");
    assert_eq!(l["lote"]["quantidade_guias"], 1);
    assert_eq!(l["contas"][0]["situacao"], "em_lote");
    let lote = Uuid::parse_str(l["lote"]["id"].as_str().unwrap()).unwrap();

    // O arquivo guardado passa no XSD oficial e confere com o SHA-256 do lastro.
    let (arquivo, sha): (Vec<u8>, String) =
        sqlx::query_as("SELECT arquivo, sha256 FROM faturamento.lote WHERE id = $1")
            .bind(lote)
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    let caminho = std::env::temp_dir().join(format!("lote-{lote}.xml"));
    std::fs::write(&caminho, &arquivo).unwrap();
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
    use sha2::Digest as _;
    assert_eq!(
        sha,
        sha2::Sha256::digest(&arquivo)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let texto: String = arquivo.iter().map(|b| char::from(*b)).collect();
    assert!(texto.contains("<ans:numeroGuiaPrestador>FAT-"));
    assert!(texto.contains("<ans:numeroCarteira>01234567</ans:numeroCarteira>"));
    assert!(
        !texto.contains(ANTONIO),
        "o TISS 4 não leva o nome do beneficiário"
    );

    // Envio: pedir duas vezes cria um envio só; a operadora recebe a mesma chave.
    for _ in 0..2 {
        let (st, e) = pedir(
            &amb,
            &rita,
            Method::POST,
            &format!("/v1/faturamento/lotes/{lote}/envio"),
            None,
            Value::Null,
        )
        .await;
        assert_eq!(st, StatusCode::ACCEPTED, "{e}");
    }
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox WHERE tipo = 'tiss.enviar_lote'")
            .await,
        1
    );
    processar_outbox(&amb).await;
    let (_, l) = pedir(
        &amb,
        &rita,
        Method::GET,
        &format!("/v1/faturamento/lotes/{lote}"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(l["lote"]["situacao"], "enviado");
    assert_eq!(l["lote"]["protocolo"], "DEV-00000001");
    assert_eq!(l["lote"]["envio_pendente"], Value::Null);
    assert_eq!(l["contas"][0]["situacao"], "enviada");
    // Retentativa do mesmo envio (ex.: o trabalhador caiu antes de gravar) não gera outro protocolo.
    vidacor_api::rotas::faturamento::lotes::enviar_pelo_outbox(
        &amb.estado,
        UNIDADE_A,
        lote,
        Uuid::now_v7(),
    )
    .await
    .unwrap();
    assert_eq!(operadora.protocolos_emitidos(), 1);

    // Demonstrativo com glosa e recurso dentro do prazo.
    let (st, dm) = pedir(&amb, &rita, Method::POST, &format!("/v1/faturamento/lotes/{lote}/demonstrativo"), Some(Uuid::now_v7()),
        json!({ "recebido_em": amb.dia, "contas": [{ "conta_id": conta, "valor_pago_centavos": 18_000,
                "glosas": [{ "codigo_glosa": "1801", "motivo": "Procedimento sem relatório clínico", "valor_centavos": 6_860 }] }] })).await;
    assert_eq!(st, StatusCode::CREATED, "{dm}");
    assert_eq!(dm["lote"]["situacao"], "demonstrativo_recebido");
    assert_eq!(dm["lote"]["valor_glosado_centavos"], 6_860);
    assert_eq!(dm["contas"][0]["situacao"], "paga_com_glosa");
    let (_, gl) = pedir(
        &amb,
        &rita,
        Method::GET,
        "/v1/faturamento/glosas",
        None,
        Value::Null,
    )
    .await;
    let glosa = gl[0]["id"].as_str().unwrap().to_owned();
    assert_eq!(gl[0]["prazo_recurso"], json!(amb.dia + Duration::days(30)));
    let chave = Uuid::now_v7();
    let corpo =
        json!({ "justificativa": "Relatório clínico anexado ao pedido médico, conforme manual." });
    let (st, g1) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/faturamento/glosas/{glosa}/recurso"),
        Some(chave),
        corpo.clone(),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{g1}");
    assert_eq!(g1["situacao"], "em_recurso");
    let (st, g2) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/faturamento/glosas/{glosa}/recurso"),
        Some(chave),
        corpo.clone(),
    )
    .await;
    assert_eq!(
        (st, &g2),
        (StatusCode::CREATED, &g1),
        "mesma chave, mesma resposta"
    );
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/faturamento/glosas/{glosa}/recurso"),
        Some(Uuid::now_v7()),
        corpo,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("recurso-ja-registrado"))
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM faturamento.recurso").await,
        1
    );
    // O envio do recurso fica pendente e visível no outbox (provedor ainda não feito, R7).
    processar_outbox(&amb).await;
    let pendente: Option<String> = sqlx::query_scalar("SELECT ultimo_erro FROM integracao.outbox WHERE tipo = 'tiss.recurso_glosa' AND concluido_em IS NULL")
        .fetch_one(&amb.admin).await.unwrap();
    assert!(pendente.unwrap().contains("ainda não configurado"));

    let tipos: Vec<String> = sqlx::query_scalar(
        "SELECT tipo FROM lastro.evento WHERE dominio = 'faturamento' ORDER BY sequencia",
    )
    .fetch_all(&amb.admin)
    .await
    .unwrap();
    for t in [
        "conta_apurada",
        "conta_conferida",
        "conta_corrigida",
        "lote_gerado",
        "lote_envio_pedido",
        "lote_enviado",
        "demonstrativo_registrado",
        "recurso_registrado",
    ] {
        assert!(tipos.iter().any(|x| x == t), "lastro sem {t}: {tipos:?}");
    }
}

#[tokio::test]
async fn guia_com_erro_bloqueante_fica_fora_do_lote_e_a_regra_fica_gravada() {
    let amb = Ambiente::novo().await;
    let rita = amb.entrar("rita.faturamento").await;
    let (contrato, antonio) = antonio_pronto(&amb, &rita).await;
    // Maria: laudo não assinado e senha vencida antes do exame.
    let guia = guia_consumida(&amb, MARIA, "UN-55121").await;
    sqlx::query("UPDATE convenio.guia SET validade = $2 WHERE id = $1")
        .bind(guia)
        .bind(amb.dia - Duration::days(1))
        .execute(&amb.admin)
        .await
        .unwrap();
    exame_com_laudo(&amb, MARIA, NASC_MARIA, false).await;
    let r = conferir(&amb, &rita).await;
    assert_eq!(r["novas"], 1);
    let maria = conta_de(&amb, MARIA).await;
    completar_solicitante(&amb, &rita, maria).await;
    let (_, d) = pedir(
        &amb,
        &rita,
        Method::GET,
        &format!("/v1/faturamento/contas/{maria}"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(regras(&d), vec!["senha-vencida", "laudo-nao-assinado"]);
    assert_eq!(d["resumo"]["situacao"], "com_pendencia");

    let (_, painel) = pedir(
        &amb,
        &rita,
        Method::GET,
        "/v1/faturamento/auditoria",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(
        painel["resumo"],
        json!({ "conferidas": 2, "prontas": 1, "corrigir_antes": 0, "bloqueadas": 1 })
    );
    assert_eq!(painel["grupos"][0]["contas"][0]["paciente"]["nome"], MARIA);
    let unimed = painel["conjuntos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["versao"] == "AUD-UNIMED-2026.1")
        .unwrap();
    assert_eq!(unimed["operadoras"], json!(["Unimed"]));

    let (st, l) = pedir(
        &amb,
        &rita,
        Method::POST,
        "/v1/faturamento/lotes",
        Some(Uuid::now_v7()),
        json!({ "contrato_id": contrato }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{l}");
    let ids: Vec<&str> = l["contas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![antonio.to_string().as_str()],
        "só a guia sem pendência entra"
    );
    let (_, d) = pedir(
        &amb,
        &rita,
        Method::GET,
        &format!("/v1/faturamento/contas/{maria}"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(d["resumo"]["situacao"], "com_pendencia");
    assert_eq!(d["resumo"]["lote_codigo"], Value::Null);
}

#[tokio::test]
async fn quando_a_guia_pronta_perde_a_condicao_entao_a_conferencia_do_lote_a_tira() {
    let amb = Ambiente::novo().await;
    let rita = amb.entrar("rita.faturamento").await;
    let (contrato, conta) = antonio_pronto(&amb, &rita).await;
    // Reajuste retroativo depois da conferência: o lote confere de novo e a guia fica de fora.
    tabela(&amb, contrato, 2, 10, 25_900).await;
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        "/v1/faturamento/lotes",
        Some(Uuid::now_v7()),
        json!({ "contrato_id": contrato }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("nenhuma-guia-pronta"))
    );
    assert_eq!(amb.contar("SELECT count(*) FROM faturamento.lote").await, 0);
    assert_eq!(
        amb.contar("SELECT ultimo_lote FROM faturamento.contrato")
            .await,
        0,
        "número não é gasto"
    );
    conferir(&amb, &rita).await;
    let (_, d) = pedir(
        &amb,
        &rita,
        Method::GET,
        &format!("/v1/faturamento/contas/{conta}"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(regras(&d), vec!["valor-diferente-da-tabela"]);
    assert_eq!(
        d["pendencias"][0]["detalhe"],
        "40901114: cobrado R$ 248,60, tabela atual R$ 259,00."
    );
    let (st, d) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/faturamento/contas/{conta}/correcao"),
        Some(Uuid::now_v7()),
        json!({ "motivo": "Reajuste da tabela Unimed", "reaplicar_tabela": true }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{d}");
    assert_eq!(d["resumo"]["situacao"], "pronta");
    assert_eq!(d["resumo"]["valor_total_centavos"], 25_900);
}

#[tokio::test]
async fn dois_gerar_lote_simultaneos_dao_um_lote_so_e_a_guia_nao_vai_em_dois_lotes() {
    let amb = Ambiente::novo().await;
    let rita = amb.entrar("rita.faturamento").await;
    let (contrato, conta) = antonio_pronto(&amb, &rita).await;
    let corpo = json!({ "contrato_id": contrato });
    let (a, b) = tokio::join!(
        pedir(
            &amb,
            &rita,
            Method::POST,
            "/v1/faturamento/lotes",
            Some(Uuid::now_v7()),
            corpo.clone()
        ),
        pedir(
            &amb,
            &rita,
            Method::POST,
            "/v1/faturamento/lotes",
            Some(Uuid::now_v7()),
            corpo.clone()
        ),
    );
    let mut status = [a.0, b.0];
    status.sort();
    assert_eq!(
        status,
        [StatusCode::CREATED, StatusCode::CONFLICT],
        "{} / {}",
        a.1,
        b.1
    );
    assert_eq!(amb.contar("SELECT count(*) FROM faturamento.lote").await, 1);
    assert_eq!(
        amb.contar("SELECT ultimo_lote FROM faturamento.contrato")
            .await,
        1
    );
    // Nem por fora da aplicação: a conta já está em um lote (constraint única) e conta em lote não muda.
    let lote: Uuid = sqlx::query_scalar("SELECT id FROM faturamento.lote")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    let dup = sqlx::query("INSERT INTO faturamento.lote_item (lote_id, conta_id, sequencial, unidade_id) VALUES ($1, $2, 2, $3)")
        .bind(lote).bind(conta).bind(UNIDADE_A).execute(&amb.admin).await;
    assert!(dup
        .unwrap_err()
        .to_string()
        .contains("lote_item_conta_id_key"));
    let volta = sqlx::query(
        "UPDATE faturamento.conta SET situacao = 'pronta', lote_id = NULL WHERE id = $1",
    )
    .bind(conta)
    .execute(&amb.admin)
    .await;
    assert!(volta
        .unwrap_err()
        .to_string()
        .contains("conta em lote não muda"));
    let arq = sqlx::query("UPDATE faturamento.lote SET arquivo = 'x' WHERE id = $1")
        .bind(lote)
        .execute(&amb.admin)
        .await;
    assert!(arq.unwrap_err().to_string().contains("imutáveis"));
    let apagar = sqlx::query("DELETE FROM faturamento.conta WHERE id = $1")
        .bind(conta)
        .execute(&amb.admin)
        .await;
    assert!(apagar.is_err(), "nada é apagado");
}

#[tokio::test]
async fn repetir_conferencia_e_lote_nao_duplica() {
    let amb = Ambiente::novo().await;
    let rita = amb.entrar("rita.faturamento").await;
    let (contrato, _) = antonio_pronto(&amb, &rita).await;
    let chave = Uuid::now_v7();
    let (s1, r1) = pedir(
        &amb,
        &rita,
        Method::POST,
        "/v1/faturamento/conferencias",
        Some(chave),
        json!({}),
    )
    .await;
    let (s2, r2) = pedir(
        &amb,
        &rita,
        Method::POST,
        "/v1/faturamento/conferencias",
        Some(chave),
        json!({}),
    )
    .await;
    assert_eq!((s1, &r1), (s2, &r2));
    // Duas conferências simultâneas com chaves diferentes não criam conta a mais (atendimento único).
    guia_consumida(&amb, MARIA, "UN-55121").await;
    exame_com_laudo(&amb, MARIA, NASC_MARIA, true).await;
    let (a, b) = tokio::join!(conferir(&amb, &rita), conferir(&amb, &rita));
    assert_eq!(
        a["novas"].as_i64().unwrap() + b["novas"].as_i64().unwrap(),
        1
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM faturamento.conta").await,
        2
    );

    let chave = Uuid::now_v7();
    let corpo = json!({ "contrato_id": contrato });
    let (s1, l1) = pedir(
        &amb,
        &rita,
        Method::POST,
        "/v1/faturamento/lotes",
        Some(chave),
        corpo.clone(),
    )
    .await;
    let (s2, l2) = pedir(
        &amb,
        &rita,
        Method::POST,
        "/v1/faturamento/lotes",
        Some(chave),
        corpo,
    )
    .await;
    assert_eq!(s1, StatusCode::CREATED, "{l1}");
    assert_eq!(
        (s2, l2["lote"]["id"].clone()),
        (StatusCode::CREATED, l1["lote"]["id"].clone())
    );
    assert_eq!(amb.contar("SELECT count(*) FROM faturamento.lote").await, 1);
    // Sem operadora configurada, o envio fica pendente e visível no lote (R7).
    let lote = l1["lote"]["id"].as_str().unwrap();
    pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/faturamento/lotes/{lote}/envio"),
        None,
        Value::Null,
    )
    .await;
    processar_outbox(&amb).await;
    let (_, l) = pedir(
        &amb,
        &rita,
        Method::GET,
        &format!("/v1/faturamento/lotes/{lote}"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(l["lote"]["situacao"], "aguardando_envio");
    assert_eq!(l["lote"]["envio_pendente"]["tentativas"], 1);
    assert!(l["lote"]["envio_pendente"]["ultimo_erro"]
        .as_str()
        .unwrap()
        .contains("não configurado"));
}

#[tokio::test]
async fn perfis_indevidos_e_outra_unidade_nao_faturam() {
    let amb = Ambiente::novo().await;
    let rita = amb.entrar("rita.faturamento").await;
    let (contrato, conta) = antonio_pronto(&amb, &rita).await;
    for login in [
        "ana.souza",
        "carla.caixa",
        "medico.a",
        "financeiro.a",
        "ti.a",
        "patricia.lemos",
        "bruno.sales",
    ] {
        let s = amb.entrar(login).await;
        for (uri, corpo) in [
            ("/v1/faturamento/conferencias".to_owned(), json!({})),
            (
                "/v1/faturamento/lotes".to_owned(),
                json!({ "contrato_id": contrato }),
            ),
            (
                format!("/v1/faturamento/contas/{conta}/correcao"),
                json!({ "motivo": "teste de acesso", "cid": "R51" }),
            ),
            (
                format!("/v1/faturamento/glosas/{}/recurso", Uuid::now_v7()),
                json!({ "justificativa": "teste de acesso indevido" }),
            ),
        ] {
            let (st, _) = pedir(&amb, &s, Method::POST, &uri, Some(Uuid::now_v7()), corpo).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "{login} {uri}");
        }
        let (st, _) = pedir(
            &amb,
            &s,
            Method::GET,
            "/v1/faturamento/contas",
            None,
            Value::Null,
        )
        .await;
        let esperado = if login == "patricia.lemos" {
            StatusCode::OK
        } else {
            StatusCode::FORBIDDEN
        };
        assert_eq!(st, esperado, "{login} lê as guias");
    }
    // Faturista de outra unidade: não vê nem altera (RLS por unidade).
    let norte = amb.entrar("faturista.b").await;
    let (_, lista) = pedir(
        &amb,
        &norte,
        Method::GET,
        "/v1/faturamento/contas",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(lista, json!([]));
    let (st, _) = pedir(
        &amb,
        &norte,
        Method::GET,
        &format!("/v1/faturamento/contas/{conta}"),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = pedir(
        &amb,
        &norte,
        Method::POST,
        &format!("/v1/faturamento/contas/{conta}/correcao"),
        Some(Uuid::now_v7()),
        json!({ "motivo": "tentativa de outra unidade", "cid": "R51" }),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = pedir(
        &amb,
        &norte,
        Method::POST,
        "/v1/faturamento/lotes",
        Some(Uuid::now_v7()),
        json!({ "contrato_id": contrato }),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // RLS direto: recepção e médico da mesma unidade não leem o faturamento.
    for perfil in ["recepcao", "medico", "financeiro", "caixa"] {
        let mut tx = amb.app_pool.begin().await.unwrap();
        sqlx::query(
            "SELECT set_config('app.unidade_id', $1, true), set_config('app.perfis', $2, true)",
        )
        .bind(UNIDADE_A.to_string())
        .bind(perfil)
        .execute(&mut *tx)
        .await
        .unwrap();
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM faturamento.conta")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(n, 0, "{perfil}");
    }
}

#[tokio::test]
async fn limites_da_correcao_do_demonstrativo_e_do_recurso() {
    let mut amb = Ambiente::novo().await;
    amb.estado.operadora = Some(Arc::new(OperadoraDesenvolvimento::default()));
    amb.app = vidacor_api::aplicacao(amb.estado.clone());
    let rita = amb.entrar("rita.faturamento").await;
    let (contrato, conta) = antonio_pronto(&amb, &rita).await;
    let corrigir = format!("/v1/faturamento/contas/{conta}/correcao");
    for (corpo, codigo) in [
        (json!({ "motivo": "abc", "cid": "R51" }), "motivo-invalido"),
        (
            json!({ "motivo": "pedido médico", "cid": "R5.1" }),
            "cid-invalido",
        ),
        (
            json!({ "motivo": "pedido médico", "solicitante": { "conselho": "CRM", "numero": "1", "uf": "RJ", "cbos": "225125" } }),
            "solicitante-invalido",
        ),
        (json!({ "motivo": "pedido médico" }), "correcao-vazia"),
    ] {
        let (st, e) = pedir(
            &amb,
            &rita,
            Method::POST,
            &corrigir,
            Some(Uuid::now_v7()),
            corpo,
        )
        .await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo))
        );
    }
    let (_, l) = pedir(
        &amb,
        &rita,
        Method::POST,
        "/v1/faturamento/lotes",
        Some(Uuid::now_v7()),
        json!({ "contrato_id": contrato }),
    )
    .await;
    let lote = l["lote"]["id"].as_str().unwrap().to_owned();
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        &corrigir,
        Some(Uuid::now_v7()),
        json!({ "motivo": "pedido médico", "cid": "R51" }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("conta-em-lote"))
    );
    let demo = format!("/v1/faturamento/lotes/{lote}/demonstrativo");
    let retorno = |pago: i64, glosas: Value| json!({ "recebido_em": amb.dia, "contas": [{ "conta_id": conta, "valor_pago_centavos": pago, "glosas": glosas }] });
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        &demo,
        Some(Uuid::now_v7()),
        retorno(24_860, json!([])),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("lote-nao-enviado"))
    );
    pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/faturamento/lotes/{lote}/envio"),
        None,
        Value::Null,
    )
    .await;
    processar_outbox(&amb).await;
    for (corpo, codigo) in [
        (retorno(20_000, json!([])), "valores-nao-fecham"),
        (
            retorno(
                -1,
                json!([{ "codigo_glosa": "1801", "motivo": "teste", "valor_centavos": 24_861 }]),
            ),
            "retorno-invalido",
        ),
        (
            retorno(
                0,
                json!([{ "codigo_glosa": "18", "motivo": "Sem senha", "valor_centavos": 24_860 }]),
            ),
            "glosa-invalida",
        ),
        (
            retorno(
                0,
                json!([{ "codigo_glosa": "1801", "motivo": "Sem senha", "valor_centavos": 0 }]),
            ),
            "glosa-invalida",
        ),
        (
            json!({ "recebido_em": amb.dia + Duration::days(1), "contas": [] }),
            "data-futura",
        ),
        (
            json!({ "recebido_em": amb.dia, "contas": [] }),
            "demonstrativo-incompleto",
        ),
    ] {
        let (st, e) = pedir(
            &amb,
            &rita,
            Method::POST,
            &demo,
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
    let (st, d) = pedir(&amb, &rita, Method::POST, &demo, Some(Uuid::now_v7()),
        retorno(0, json!([{ "codigo_glosa": "1714", "motivo": "Guia sem assinatura", "valor_centavos": 24_860 }]))).await;
    assert_eq!(st, StatusCode::CREATED, "{d}");
    assert_eq!(d["contas"][0]["situacao"], "glosada");
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        &demo,
        Some(Uuid::now_v7()),
        retorno(24_860, json!([])),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("demonstrativo-ja-registrado"))
    );

    let glosa: Uuid = sqlx::query_scalar("SELECT id FROM faturamento.glosa")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    let recurso = format!("/v1/faturamento/glosas/{glosa}/recurso");
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        &recurso,
        Some(Uuid::now_v7()),
        json!({ "justificativa": "curta" }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("justificativa-invalida")
        )
    );
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        &recurso,
        Some(Uuid::now_v7()),
        json!({ "justificativa": "x".repeat(501) }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("justificativa-invalida")
        )
    );
    sqlx::query("UPDATE faturamento.glosa SET prazo_recurso = $1")
        .bind(amb.dia - Duration::days(1))
        .execute(&amb.admin)
        .await
        .unwrap();
    let (st, e) = pedir(
        &amb,
        &rita,
        Method::POST,
        &recurso,
        Some(Uuid::now_v7()),
        json!({ "justificativa": "Assinatura colhida no portal do paciente." }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("prazo-de-recurso-vencido")
        )
    );
    // Dois recursos simultâneos com chaves diferentes: só um entra (um recurso por glosa).
    sqlx::query("UPDATE faturamento.glosa SET prazo_recurso = $1")
        .bind(amb.dia)
        .execute(&amb.admin)
        .await
        .unwrap();
    let corpo = json!({ "justificativa": "Assinatura colhida no portal do paciente." });
    let (a, b) = tokio::join!(
        pedir(
            &amb,
            &rita,
            Method::POST,
            &recurso,
            Some(Uuid::now_v7()),
            corpo.clone()
        ),
        pedir(
            &amb,
            &rita,
            Method::POST,
            &recurso,
            Some(Uuid::now_v7()),
            corpo
        ),
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM faturamento.recurso").await,
        1
    );
}

#[tokio::test]
async fn sem_contrato_a_guia_fica_bloqueada_e_nao_ha_lote() {
    let amb = Ambiente::novo().await;
    let rita = amb.entrar("rita.faturamento").await;
    guia_consumida(&amb, ANTONIO, "UN-55120").await;
    exame_com_laudo(&amb, ANTONIO, NASC_ANTONIO, true).await;
    conferir(&amb, &rita).await;
    let conta = conta_de(&amb, ANTONIO).await;
    let (_, d) = pedir(
        &amb,
        &rita,
        Method::GET,
        &format!("/v1/faturamento/contas/{conta}"),
        None,
        Value::Null,
    )
    .await;
    let r = regras(&d);
    assert!(
        r.contains(&"operadora-sem-contrato") && r.contains(&"procedimento-sem-preco"),
        "{r:?}"
    );
    assert_eq!(d["pendencias"][0]["regra_versao"], "AUD-TISS-2026.1");
    let (_, k) = pedir(
        &amb,
        &rita,
        Method::GET,
        "/v1/faturamento/contratos",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(k, json!([]));
    // Contrato cadastrado depois: a correção reaplica a tabela e a guia fica pronta.
    contrato_unimed(&amb).await;
    completar_solicitante(&amb, &rita, conta).await;
    let (st, d) = pedir(
        &amb,
        &rita,
        Method::POST,
        &format!("/v1/faturamento/contas/{conta}/correcao"),
        Some(Uuid::now_v7()),
        json!({ "motivo": "Contrato Unimed cadastrado", "reaplicar_tabela": true }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{d}");
    assert_eq!(d["resumo"]["situacao"], "pronta", "{d}");
    assert_eq!(d["resumo"]["regra_versao"], "AUD-UNIMED-2026.1");
}
