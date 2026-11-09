//! Comunicação com pacientes (Comunicacao.dc.html, corte do MVP): registro das mensagens com situação e
//! resposta, aviso automático desligado pela gerência não sai e fica registrado, mudança pela versão
//! vista (2 requisições simultâneas, só uma vale), repetir não duplica, versão não se altera e acesso
//! indevido recusado. Rota real e PostgreSQL real; só o WhatsApp é simulado (G4). FC-COM-01 a 05 (proposta).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const MARIA: &str = "Maria Helena Duarte";

async fn processar_outbox(amb: &Ambiente) {
    let orthanc = vidacor_api::orthanc::Orthanc::novo("http://127.0.0.1:9").unwrap();
    while vidacor_api::outbox::processar_um(&amb.estado, &orthanc)
        .await
        .unwrap()
    {}
}

/// Antônio (com celular) e Maria (sem celular) agendados para amanhã; a recepção pede as confirmações.
async fn confirmar_amanha(amb: &Ambiente) {
    sqlx::query("UPDATE identidade.paciente SET celular = '11987654321' WHERE id = $1")
        .bind(amb.s.pacientes[ANTONIO])
        .execute(&amb.admin)
        .await
        .unwrap();
    // Amanhã ao meio-dia no fuso da clínica (sem depender da hora em que o teste roda).
    let dia = vidacor_api::tempo::hoje_local() + chrono::Duration::days(1);
    for (k, p) in [ANTONIO, MARIA].iter().enumerate() {
        sqlx::query("UPDATE agenda.agendamento SET situacao = 'agendado',
                            inicio = ($2::date + time '12:00' + $3 * interval '30 minutes') AT TIME ZONE 'America/Sao_Paulo',
                            fim = ($2::date + time '12:00' + ($3 + 1) * interval '30 minutes') AT TIME ZONE 'America/Sao_Paulo'
                      WHERE id = $1")
            .bind(amb.s.agendamentos[*p])
            .bind(dia)
            .bind(i32::try_from(k).unwrap())
            .execute(&amb.admin)
            .await
            .unwrap();
    }
    let ana = amb.entrar("ana.souza").await;
    let (st, _, l) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/comunicacao/lembretes",
            &ana,
            None,
            json!({ "data": dia }),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{l}");
    processar_outbox(amb).await;
}

async fn painel(amb: &Ambiente, login: &str, consulta: &str) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.get(&format!("/v1/comunicacao{consulta}"), Some(&s)))
        .await;
    (st, v)
}

fn aviso<'a>(p: &'a Value, modelo: &str) -> &'a Value {
    p["automaticas"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["modelo"] == modelo)
        .unwrap()
}

#[tokio::test]
async fn registro_mostra_cada_mensagem_com_situacao_e_dois_identificadores() {
    let amb = Ambiente::novo().await;
    confirmar_amanha(&amb).await;
    // FC-COM-01: a enviada e a que não saiu por falta de celular aparecem, com nome e nascimento.
    let (st, p) = painel(&amb, "ana.souza", "").await;
    assert_eq!(st, StatusCode::OK, "{p}");
    assert_eq!(p["whatsapp_configurado"], true);
    assert_eq!(
        (
            p["hoje"]["enviadas"].as_i64(),
            p["hoje"]["sem_celular"].as_i64()
        ),
        (Some(1), Some(1))
    );
    let por_nome = |nome: &str| -> Value {
        p["mensagens"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["nome"] == nome)
            .cloned()
            .unwrap()
    };
    let a = por_nome(ANTONIO);
    assert_eq!(
        (
            a["situacao"].as_str(),
            a["modelo"].as_str(),
            a["nascimento"].as_str()
        ),
        (Some("enviada"), Some("confirmacao"), Some("1954-06-15"))
    );
    assert_eq!(por_nome(MARIA)["situacao"], "sem_celular");
    // Filtro "não entregues" mostra só a que não saiu.
    let (_, p) = painel(&amb, "ana.souza", "?filtro=nao_entregues").await;
    assert_eq!(p["mensagens"].as_array().unwrap().len(), 1);
    assert_eq!(p["mensagens"][0]["nome"], MARIA);
    let (st, _) = painel(&amb, "ana.souza", "?filtro=qualquer").await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    // Todos os avisos aparecem ligados, sem versão (como vinham funcionando).
    assert_eq!(p["automaticas"].as_array().unwrap().len(), 4);
    assert!(p["automaticas"]
        .as_array()
        .unwrap()
        .iter()
        .all(|a| a["ativo"] == true && a["versao_id"].is_null()));
}

#[tokio::test]
async fn aviso_desligado_nao_sai_e_fica_registrado() {
    let amb = Ambiente::novo().await;
    let g = amb.entrar("patricia.lemos").await;
    let uri = "/v1/comunicacao/automaticas/confirmacao";
    // Limite: motivo curto recusado.
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            uri,
            &g,
            Some(Uuid::now_v7()),
            json!({ "ativo": false, "motivo": "x", "versao_vista": null }),
        ))
        .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    // FC-COM-02: a gerência desliga com motivo; repetir com a mesma chave devolve a mesma resposta.
    let chave = Uuid::now_v7();
    let corpo = json!({ "ativo": false, "motivo": "Agenda de amanhã confirmada por telefone durante a troca do número.", "versao_vista": null });
    let (st, _, r1) = amb
        .pedir(amb.json(Method::POST, uri, &g, Some(chave), corpo.clone()))
        .await;
    assert_eq!(st, StatusCode::OK, "{r1}");
    let (st, _, r2) = amb
        .pedir(amb.json(Method::POST, uri, &g, Some(chave), corpo))
        .await;
    assert_eq!((st, &r2["versao_id"]), (StatusCode::OK, &r1["versao_id"]));
    assert_eq!(
        amb.contar("SELECT count(*) FROM comunicacao.envio_automatico")
            .await,
        1
    );
    // Com o aviso desligado, nada sai pelo WhatsApp e cada mensagem fica registrada como desligada.
    confirmar_amanha(&amb).await;
    assert!(amb
        .mensageiro
        .enviadas
        .lock()
        .unwrap()
        .iter()
        .all(|(_, m)| m.modelo != "confirmacao"));
    let (_, p) = painel(&amb, "ana.souza", "").await;
    assert_eq!(p["hoje"]["desligadas"], 2);
    assert_eq!(p["hoje"]["enviadas"], 0);
    let a = aviso(&p, "confirmacao");
    assert_eq!(
        (a["ativo"].as_bool(), a["alterado_por"].as_str()),
        (Some(false), Some("Patrícia Lemos"))
    );
    // A versão não se altera nem se apaga.
    assert!(
        sqlx::query("UPDATE comunicacao.envio_automatico SET ativo = true")
            .execute(&amb.admin)
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM comunicacao.envio_automatico")
        .execute(&amb.admin)
        .await
        .is_err());
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'aviso_automatico_desligado'")
            .await,
        1
    );
}

#[tokio::test]
async fn mudanca_pela_versao_vista_so_uma_vale() {
    let amb = Ambiente::novo().await;
    let g = amb.entrar("patricia.lemos").await;
    let uri = "/v1/comunicacao/automaticas/laudo_pronto";
    let desligar = json!({ "ativo": false, "motivo": "Portal em manutenção programada neste fim de semana.", "versao_vista": null });
    // FC-COM-03: duas mudanças sobre a mesma versão vista, ao mesmo tempo: só uma vale (G2).
    let (a, b) = tokio::join!(
        amb.pedir(amb.json(
            Method::POST,
            uri,
            &g,
            Some(Uuid::now_v7()),
            desligar.clone()
        )),
        amb.pedir(amb.json(
            Method::POST,
            uri,
            &g,
            Some(Uuid::now_v7()),
            desligar.clone()
        ))
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    let vigente = if a.0 == StatusCode::OK { a.2 } else { b.2 };
    // Mudar de novo com a versão antiga (vista antes): recusado; com a vigente: aceito.
    let ligar_antiga = json!({ "ativo": true, "motivo": "Portal voltou a funcionar normalmente.", "versao_vista": null });
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, uri, &g, Some(Uuid::now_v7()), ligar_antiga))
        .await;
    assert_eq!(
        (st, v["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("aviso-mudou"))
    );
    let ligar = json!({ "ativo": true, "motivo": "Portal voltou a funcionar normalmente.", "versao_vista": vigente["versao_id"] });
    let (st, _, _) = amb
        .pedir(amb.json(Method::POST, uri, &g, Some(Uuid::now_v7()), ligar.clone()))
        .await;
    assert_eq!(st, StatusCode::OK);
    // Pedir o estado em que já está: recusado, nada gravado.
    let (_, p) = painel(&amb, "patricia.lemos", "").await;
    let mesmo = json!({ "ativo": true, "motivo": "Garantir que está ligado mesmo.", "versao_vista": aviso(&p, "laudo_pronto")["versao_id"] });
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, uri, &g, Some(Uuid::now_v7()), mesmo))
        .await;
    assert_eq!(
        (st, v["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("aviso-ja-assim"))
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM comunicacao.envio_automatico")
            .await,
        2
    );
}

#[tokio::test]
async fn acesso_indevido_e_aviso_desconhecido() {
    let amb = Ambiente::novo().await;
    // FC-COM-04: só recepção e gerência veem o registro.
    for login in [
        "carla.caixa",
        "bruno.sales",
        "medico.a",
        "financeiro.a",
        "ti.a",
        "tv.a",
    ] {
        assert_eq!(
            painel(&amb, login, "").await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    // FC-COM-05: só a gerência liga ou desliga; a recepção vê mas não muda.
    let corpo = json!({ "ativo": false, "motivo": "Tentativa da recepção de desligar.", "versao_vista": null });
    for login in ["ana.souza", "carla.caixa", "ti.a"] {
        let s = amb.entrar(login).await;
        let (st, _, _) = amb
            .pedir(amb.json(
                Method::POST,
                "/v1/comunicacao/automaticas/confirmacao",
                &s,
                Some(Uuid::now_v7()),
                corpo.clone(),
            ))
            .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    let g = amb.entrar("patricia.lemos").await;
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/comunicacao/automaticas/campanha_marketing",
            &g,
            Some(Uuid::now_v7()),
            corpo,
        ))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // A mudança de uma unidade não vale para a outra (a recepção do Norte vê o aviso ligado).
    let corpo =
        json!({ "ativo": false, "motivo": "Somente na unidade Centro.", "versao_vista": null });
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/comunicacao/automaticas/oferta_vaga",
            &g,
            Some(Uuid::now_v7()),
            corpo,
        ))
        .await;
    assert_eq!(st, StatusCode::OK);
    let (_, p) = painel(&amb, "recepcao.b", "").await;
    assert_eq!(aviso(&p, "oferta_vaga")["ativo"], true);
    let (_, p) = painel(&amb, "ana.souza", "").await;
    assert_eq!(aviso(&p, "oferta_vaga")["ativo"], false);
}
