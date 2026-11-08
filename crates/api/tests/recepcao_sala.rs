//! Frases aprovadas do docs/27: check-in (FC-CHK), etiquetas (FC-ETQ) e bipe na sala (FC-PUL).
//! Rota HTTP real, PostgreSQL real (G4). Dados fictícios.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::json;
use uuid::Uuid;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const NASC: &str = "1954-06-15";

async fn bipar(
    amb: &Ambiente,
    tecnico: &str,
    sala: i32,
    etiqueta: &str,
) -> (StatusCode, serde_json::Value) {
    let s = amb.entrar(tecnico).await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/salas/{}/bipes", amb.s.salas[&sala]),
            &s,
            Some(Uuid::now_v7()),
            json!({ "codigo_etiqueta": etiqueta }),
        ))
        .await;
    (st, c)
}

#[tokio::test]
async fn fc_chk_01_checkin_emite_senha_e_pulseira_e_registra_as_tres_conferencias() {
    let amb = Ambiente::novo().await;
    let c = amb.fazer_checkin(ANTONIO, NASC).await;
    assert_eq!(c["situacao"], "aguardando_sala");
    assert!(c["senha"].as_str().unwrap().starts_with('A'));
    assert!(c["pulseira"]["codigo"]
        .as_str()
        .unwrap()
        .starts_with("ETQ-"));
    let dados: serde_json::Value =
        sqlx::query_scalar("SELECT dados FROM lastro.evento WHERE tipo = 'checkin_realizado'")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(
        dados["conferencias"],
        json!(["nome", "nascimento", "documento_com_foto"])
    );
}

#[tokio::test]
async fn fc_chk_02_sem_nascimento_ou_sem_documento_bloqueia_e_nada_e_emitido() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    for (confirmacao, falta) in [
        (
            json!({ "nome": ANTONIO, "documento_com_foto": true }),
            "data de nascimento",
        ),
        (
            json!({ "nome": ANTONIO, "nascimento": NASC, "documento_com_foto": false }),
            "documento com foto",
        ),
        (
            json!({ "nascimento": NASC, "documento_com_foto": true }),
            "nome completo",
        ),
    ] {
        let (st, _, c) = amb
            .pedir(amb.json(
                Method::POST,
                "/v1/atendimentos",
                &ana,
                Some(Uuid::now_v7()),
                json!({ "agendamento_id": amb.agendamento_antonio, "confirmacao": confirmacao }),
            ))
            .await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(c["codigo"], "identificacao-incompleta");
        assert!(c["detail"].as_str().unwrap().contains(falta), "{c}");
    }
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.etiqueta").await,
        0
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.atendimento")
            .await,
        0
    );
}

#[tokio::test]
async fn fc_chk_04_homonimo_do_dia_aparece_com_os_dois_nascimentos() {
    let amb = Ambiente::novo().await;
    // Outro Antônio Carlos Ribeiro, nascido em 02/01/1980, também agendado hoje.
    let outro = Uuid::now_v7();
    sqlx::query("INSERT INTO identidade.paciente (id, codigo, nome, nascimento, sexo, origem, unidade_id, correlacao) VALUES ($1,'PAC-2026-000099','Antonio Carlos Ribeiro','1980-01-02','M','seed',$2,'seed')")
        .bind(outro).bind(comum::UNIDADE_A).execute(&amb.admin).await.unwrap();
    sqlx::query("INSERT INTO agenda.agendamento (id, codigo, sala_id, paciente_id, inicio, fim, exame_tuss, exame_nome, situacao, origem, unidade_id, correlacao) VALUES ($1,'AGD-2026-000099',$2,$3,(SELECT inicio FROM agenda.agendamento WHERE codigo='AGD-SEED-000001') + interval '3 hours',(SELECT inicio FROM agenda.agendamento WHERE codigo='AGD-SEED-000001') + interval '210 minutes','40901114','Ecocardiograma','confirmado','seed',$4,'seed')")
        .bind(Uuid::now_v7()).bind(amb.sala1).bind(outro).bind(comum::UNIDADE_A).execute(&amb.admin).await.unwrap();
    let ana = amb.entrar("ana.souza").await;
    let (_, _, agenda) = amb
        .pedir(amb.get(&format!("/v1/agenda?data={}", amb.dia), Some(&ana)))
        .await;
    let itens = agenda["itens"].as_array().unwrap();
    let antonio = itens
        .iter()
        .find(|i| i["paciente"]["nascimento"] == NASC)
        .unwrap();
    assert_eq!(antonio["homonimos"][0]["nascimento"], "1980-01-02");
    let maria = itens
        .iter()
        .find(|i| i["paciente"]["nome"] == "Maria Helena Duarte")
        .unwrap();
    assert_eq!(maria["homonimos"], json!([]));
}

#[tokio::test]
async fn fc_chk_07_perfil_caixa_nao_faz_checkin() {
    let amb = Ambiente::novo().await;
    let carla = amb.entrar("carla.caixa").await;
    let (st, _, _) = amb
        .pedir(amb.checkin(
            &carla,
            Some(Uuid::now_v7()),
            amb.agendamento_antonio,
            ANTONIO,
            NASC,
        ))
        .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}

async fn reimprimir(
    amb: &Ambiente,
    login: &str,
    codigo: &str,
    motivo: Option<&str>,
    chave: Uuid,
) -> (StatusCode, serde_json::Value) {
    let s = amb.entrar(login).await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/etiquetas/{codigo}/reimpressao"),
            &s,
            Some(chave),
            json!({ "motivo": motivo }),
        ))
        .await;
    (st, c)
}

#[tokio::test]
async fn fc_etq_01_reimpressao_com_motivo_invalida_a_anterior_e_o_bipe_antigo_bloqueia() {
    let amb = Ambiente::novo().await;
    let c = amb.fazer_checkin(ANTONIO, NASC).await;
    let antiga = c["pulseira"]["codigo"].as_str().unwrap().to_owned();
    let (st, r) = reimprimir(&amb, "ana.souza", &antiga, Some("rasgou"), Uuid::now_v7()).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    assert_ne!(r["nova"]["codigo"], antiga.as_str());
    assert_eq!(r["invalidada"]["situacao"], "invalidada");
    assert_eq!(r["invalidada"]["motivo_invalidacao"], "rasgou");
    let (sb, b) = bipar(&amb, "bruno.sales", 1, &antiga).await;
    assert_eq!(sb, StatusCode::CONFLICT);
    assert_eq!(b["codigo"], "etiqueta-substituida");
}

#[tokio::test]
async fn fc_etq_02_reimpressao_sem_motivo_e_recusada() {
    let amb = Ambiente::novo().await;
    let c = amb.fazer_checkin(ANTONIO, NASC).await;
    let (st, r) = reimprimir(
        &amb,
        "ana.souza",
        c["pulseira"]["codigo"].as_str().unwrap(),
        None,
        Uuid::now_v7(),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r["codigo"], "motivo-obrigatorio");
}

#[tokio::test]
async fn fc_etq_03_impressao_duplicada_ao_mesmo_tempo_gera_uma_etiqueta() {
    let amb = Ambiente::novo().await;
    let c = amb.fazer_checkin(ANTONIO, NASC).await;
    let codigo = c["pulseira"]["codigo"].as_str().unwrap().to_owned();
    let chave = Uuid::now_v7();
    let (a, b) = tokio::join!(
        reimprimir(&amb, "ana.souza", &codigo, Some("rasgou"), chave),
        reimprimir(&amb, "ana.souza", &codigo, Some("rasgou"), chave)
    );
    assert_eq!(a.0, StatusCode::CREATED);
    assert_eq!(b.0, StatusCode::CREATED);
    assert_eq!(a.1, b.1);
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.etiqueta").await,
        2,
        "a original e uma só nova"
    );
}

#[tokio::test]
async fn fc_etq_04_correcao_de_nascimento_invalida_as_etiquetas_e_guarda_a_versao() {
    let amb = Ambiente::novo().await;
    amb.fazer_checkin(ANTONIO, NASC).await;
    let ana = amb.entrar("ana.souza").await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/pacientes/{}/correcao", amb.antonio),
            &ana,
            None,
            json!({ "nascimento": "1955-06-15", "motivo": "documento conferido mostra 1955" }),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    assert_eq!(c["etiquetas_invalidadas"].as_array().unwrap().len(), 1);
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.etiqueta WHERE situacao = 'valida'")
            .await,
        0
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM identidade.paciente_versao")
            .await,
        1
    );
}

#[tokio::test]
async fn correcao_de_cadastro_exige_reautenticacao() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    amb.expirar_reautenticacao().await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/pacientes/{}/correcao", amb.antonio),
            &ana,
            None,
            json!({ "nascimento": "1955-06-15", "motivo": "documento" }),
        ))
        .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    assert_eq!(c["codigo"], "reautenticacao-necessaria");
}

#[tokio::test]
async fn fc_etq_05_perfil_financeiro_nao_reimprime() {
    let amb = Ambiente::novo().await;
    let c = amb.fazer_checkin(ANTONIO, NASC).await;
    let (st, _) = reimprimir(
        &amb,
        "financeiro.a",
        c["pulseira"]["codigo"].as_str().unwrap(),
        Some("rasgou"),
        Uuid::now_v7(),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn fc_pul_01_pulseira_certa_libera_a_worklist_e_registra_o_bipe() {
    let amb = Ambiente::novo().await;
    let c = amb.fazer_checkin(ANTONIO, NASC).await;
    let (st, b) = bipar(
        &amb,
        "bruno.sales",
        1,
        c["pulseira"]["codigo"].as_str().unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{b}");
    assert_eq!(b["worklist"], "enviada");
    assert_eq!(b["aparelho"]["nome"], "Vivid E95");
    assert_eq!(b["atendimento"]["situacao"], "em_exame");
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox WHERE tipo = 'worklist.criar'")
            .await,
        1
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'pulseira_bipada'")
            .await,
        1
    );
}

#[tokio::test]
async fn fc_pul_02_pulseira_de_outra_sala_bloqueia_e_abre_divergencia() {
    let amb = Ambiente::novo().await;
    let c = amb
        .fazer_checkin("Juliana Sampaio Rocha", "1993-04-12")
        .await;
    let (st, b) = bipar(
        &amb,
        "bruno.sales",
        1,
        c["pulseira"]["codigo"].as_str().unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(b["codigo"], "paciente-de-outra-sala");
    assert_eq!(b["sala_correta"]["nome"], "Sala 2");
    assert!(b["detail"].as_str().unwrap().contains("Sala 2"));
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox").await,
        0
    );
    assert_eq!(amb.contar("SELECT count(*) FROM recepcao.divergencia WHERE tipo = 'pulseira_sala_errada' AND situacao = 'aberta'").await, 1);
}

#[tokio::test]
async fn fc_pul_04_bipe_repetido_nao_envia_a_worklist_duas_vezes() {
    let amb = Ambiente::novo().await;
    let c = amb.fazer_checkin(ANTONIO, NASC).await;
    let etq = c["pulseira"]["codigo"].as_str().unwrap().to_owned();
    let ((s1, b1), (s2, b2)) = tokio::join!(
        bipar(&amb, "bruno.sales", 1, &etq),
        bipar(&amb, "bruno.sales", 1, &etq)
    );
    assert_eq!((s1, s2), (StatusCode::OK, StatusCode::OK));
    let mut w = [
        b1["worklist"].as_str().unwrap().to_owned(),
        b2["worklist"].as_str().unwrap().to_owned(),
    ];
    w.sort();
    assert_eq!(w, ["enviada", "ja_enviada"]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox").await,
        1
    );
}

#[tokio::test]
async fn fc_pul_05_tecnico_de_outra_sala_nao_libera() {
    let amb = Ambiente::novo().await;
    let c = amb.fazer_checkin(ANTONIO, NASC).await;
    let (st, b) = bipar(
        &amb,
        "ricardo.lopes",
        1,
        c["pulseira"]["codigo"].as_str().unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    assert_eq!(b["codigo"], "fora-da-sala");
    assert_eq!(
        amb.contar("SELECT count(*) FROM integracao.outbox").await,
        0
    );
}

#[tokio::test]
async fn fila_da_sala_mostra_os_atendimentos_com_dois_identificadores() {
    let amb = Ambiente::novo().await;
    amb.fazer_checkin(ANTONIO, NASC).await;
    let bruno = amb.entrar("bruno.sales").await;
    let (st, _, f) = amb
        .pedir(amb.get(&format!("/v1/salas/{}/fila", amb.sala1), Some(&bruno)))
        .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(f["itens"][0]["paciente"]["nascimento"], NASC);
    let ricardo = amb.entrar("ricardo.lopes").await;
    let (st2, _, _) = amb
        .pedir(amb.get(&format!("/v1/salas/{}/fila", amb.sala1), Some(&ricardo)))
        .await;
    assert_eq!(st2, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn depois_da_correcao_de_cadastro_a_recepcao_emite_pulseira_nova() {
    let amb = Ambiente::novo().await;
    let c = amb.fazer_checkin(ANTONIO, NASC).await;
    let ana = amb.entrar("ana.souza").await;
    let uri = format!("/v1/atendimentos/{}/pulseira", c["id"].as_str().unwrap());
    let (ja, _, j) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &ana,
            Some(Uuid::now_v7()),
            json!({ "motivo": "teste" }),
        ))
        .await;
    assert_eq!(ja, StatusCode::CONFLICT, "{j}");
    amb.pedir(amb.json(
        Method::POST,
        &format!("/v1/pacientes/{}/correcao", amb.antonio),
        &ana,
        None,
        json!({ "nascimento": "1955-06-15", "motivo": "documento conferido mostra 1955" }),
    ))
    .await;
    let chave = Uuid::now_v7();
    let (st, _, nova) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &ana,
            Some(chave),
            json!({ "motivo": "cadastro corrigido" }),
        ))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{nova}");
    assert_eq!(nova["situacao"], "valida");
    let (st2, _, repetida) = amb
        .pedir(amb.json(
            Method::POST,
            &uri,
            &ana,
            Some(chave),
            json!({ "motivo": "cadastro corrigido" }),
        ))
        .await;
    assert_eq!(st2, StatusCode::CREATED);
    assert_eq!(nova, repetida);
    assert_eq!(
        amb.contar("SELECT count(*) FROM recepcao.etiqueta WHERE situacao = 'valida'")
            .await,
        1
    );
}

#[tokio::test]
async fn equipe_lista_nome_e_perfis_da_propria_unidade() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (st, _, e) = amb.pedir(amb.get("/v1/equipe", Some(&ana))).await;
    assert_eq!(st, StatusCode::OK);
    let nomes: Vec<&str> = e
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["nome"].as_str().unwrap())
        .collect();
    assert!(nomes.contains(&"Patrícia Lemos"));
    assert!(
        !nomes.contains(&"Recepção Norte"),
        "outra unidade não aparece"
    );
}
