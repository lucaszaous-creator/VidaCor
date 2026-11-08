//! Menu de contexto vindo do servidor (docs/19 § 4 e § 8): ações por perfil e por estado, motivo no
//! indisponível e acesso indevido. Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use comum::Ambiente;
use serde_json::Value;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const NASC: &str = "1954-06-15";

fn acao<'a>(menu: &'a Value, id: &str) -> Option<&'a Value> {
    menu["acoes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id)
}

#[tokio::test]
async fn checkin_aparece_disponivel_e_depois_do_checkin_fica_apagado_com_motivo() {
    let amb = Ambiente::novo().await;
    let uri = format!("/v1/menu/agendamento/{}", amb.s.agendamentos[ANTONIO]);
    let ana = amb.entrar("ana.souza").await;
    let (st, _, m) = amb.pedir(amb.get(&uri, Some(&ana))).await;
    assert_eq!(st, StatusCode::OK, "{m}");
    assert_eq!(m["cabecalho"]["titulo"], ANTONIO);
    assert!(m["cabecalho"]["subtitulo"]
        .as_str()
        .unwrap()
        .starts_with("Nascimento 15/06/1954"));
    assert_eq!(acao(&m, "checkin").unwrap()["estado"], "disponivel");
    assert_eq!(
        acao(&m, "reimprimir").unwrap()["motivo"],
        "Faça o check-in antes"
    );

    let atd = amb.fazer_checkin(ANTONIO, NASC).await;
    let (_, _, depois) = amb.pedir(amb.get(&uri, Some(&ana))).await;
    let checkin = acao(&depois, "checkin").unwrap();
    assert_eq!(checkin["estado"], "indisponivel");
    assert_eq!(
        checkin["motivo"],
        format!("Check-in já feito ({})", atd["codigo"].as_str().unwrap())
    );
    assert_eq!(checkin["destino"], Value::Null);
    assert_eq!(acao(&depois, "reimprimir").unwrap()["estado"], "disponivel");
    assert_eq!(acao(&depois, "reimprimir").unwrap()["perigo"], true);
}

#[tokio::test]
async fn caixa_nao_ve_checkin_e_ti_nao_tem_menu() {
    let amb = Ambiente::novo().await;
    let uri = format!("/v1/menu/agendamento/{}", amb.s.agendamentos[ANTONIO]);
    let carla = amb.entrar("carla.caixa").await;
    let (st, _, m) = amb.pedir(amb.get(&uri, Some(&carla))).await;
    assert_eq!(st, StatusCode::OK);
    assert!(
        acao(&m, "checkin").is_none(),
        "o perfil nunca terá: não aparece"
    );
    assert_eq!(
        acao(&m, "receber").unwrap()["motivo"],
        "Faça o check-in antes"
    );
    let ti = amb.entrar("ti.a").await;
    let (s403, _, _) = amb.pedir(amb.get(&uri, Some(&ti))).await;
    assert_eq!(s403, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn outra_unidade_nao_descobre_o_agendamento_nem_o_laudo() {
    let amb = Ambiente::novo().await;
    let recepcao_b = amb.entrar("recepcao.b").await;
    let (st, _, _) = amb
        .pedir(amb.get(
            &format!("/v1/menu/agendamento/{}", amb.s.agendamentos[ANTONIO]),
            Some(&recepcao_b),
        ))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "ter o ID não dá acesso (RLS)");
    let (_, laudo_id) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let outro = amb.entrar("marcos.freire").await;
    let (s, _, m) = amb
        .pedir(amb.get(&format!("/v1/menu/laudo/{}", laudo_id), Some(&outro)))
        .await;
    assert!(
        s == StatusCode::NOT_FOUND || acao(&m, "assinar").unwrap()["estado"] == "indisponivel",
        "{s} {m}"
    );
}

#[tokio::test]
async fn laudo_em_rascunho_assina_e_ainda_nao_compartilha() {
    let amb = Ambiente::novo().await;
    let (_, laudo_id) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let medico = amb.entrar("medico.a").await;
    let (st, _, m) = amb
        .pedir(amb.get(&format!("/v1/menu/laudo/{}", laudo_id), Some(&medico)))
        .await;
    assert_eq!(st, StatusCode::OK, "{m}");
    assert_eq!(acao(&m, "assinar").unwrap()["estado"], "disponivel");
    assert_eq!(
        acao(&m, "link").unwrap()["motivo"],
        "Laudo ainda não assinado"
    );
    assert_eq!(
        acao(&m, "abrir").unwrap()["destino"],
        format!("/laudos/{}", laudo_id)
    );
    let ana = amb.entrar("ana.souza").await;
    let (s403, _, _) = amb
        .pedir(amb.get(&format!("/v1/menu/laudo/{}", laudo_id), Some(&ana)))
        .await;
    assert_eq!(s403, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tecnico_abre_o_menu_pelo_atendimento_da_fila_da_sala() {
    let amb = Ambiente::novo().await;
    let atd = amb.fazer_checkin(ANTONIO, NASC).await;
    let bruno = amb.entrar("bruno.sales").await;
    let (st, _, m) = amb
        .pedir(amb.get(
            &format!("/v1/menu/atendimento/{}", atd["id"].as_str().unwrap()),
            Some(&bruno),
        ))
        .await;
    assert_eq!(st, StatusCode::OK, "{m}");
    assert_eq!(m["cabecalho"]["titulo"], ANTONIO);
    assert_eq!(acao(&m, "sala").unwrap()["estado"], "disponivel");
    assert!(acao(&m, "checkin").is_none());
}
