//! Autorização pedida pelo sistema (docs/22 E4): a solicitação SP/SADT e a resposta simulada passam
//! no XSD oficial TISS 4.03.00; a leitura decide a situação pelas quantidades e recusa DTD e entidade.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use chrono::NaiveDate;
use comum::{guia_ok, validar_no_xsd};
use vidacor_dominio_faturamento::autorizacao::{
    ler_resposta, montar_solicitacao, resposta_simulada, ErroResposta, PedidoAutorizacao,
    ProcedimentoSolicitado, RespostaSimulada, StatusAutorizacao,
};

fn quando() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 11, 8)
        .unwrap()
        .and_hms_opt(16, 5, 0)
        .unwrap()
}

fn procedimentos() -> Vec<ProcedimentoSolicitado> {
    vec![
        ProcedimentoSolicitado {
            tuss: "40901114".into(),
            descricao: "Ecocardiograma transtorácico".into(),
            quantidade: 1,
        },
        ProcedimentoSolicitado {
            tuss: "40901300".into(),
            descricao: "Ecodoppler de carótidas".into(),
            quantidade: 1,
        },
    ]
}

fn simulada<'a>(
    status: StatusAutorizacao,
    procs: &'a [(&'a str, u16, u16)],
    motivos: &'a [&'a str],
) -> RespostaSimulada<'a> {
    RespostaSimulada {
        numero_guia_prestador: "AUT-2026-000001",
        registro_ans: "123456",
        codigo_prestador: "0004567",
        carteirinha: "0 123 4567",
        status,
        numero_guia_operadora: Some("UN-77231"),
        senha: Some("UN-88231-X"),
        data_autorizacao: NaiveDate::from_ymd_opt(2026, 11, 8),
        validade: NaiveDate::from_ymd_opt(2026, 12, 8),
        procedimentos: procs,
        motivos,
        quando: quando(),
    }
}

#[test]
fn solicitacao_sp_sadt_passa_no_xsd_oficial() {
    let g = guia_ok();
    let procs = procedimentos();
    let m = montar_solicitacao(&PedidoAutorizacao {
        numero_guia_prestador: "AUT-2026-000001",
        sequencial: 1,
        registro_ans: "123456",
        codigo_prestador: "0004567",
        nome_prestador: "VidaCor Imagem Unidade Centro",
        cnes: "1234567",
        carteirinha: &g.carteirinha,
        solicitante: &g.solicitante,
        data_solicitacao: NaiveDate::from_ymd_opt(2026, 11, 8).unwrap(),
        indicacao_clinica: Some("Investigação de sopro <cardíaco> & dispneia"),
        procedimentos: &procs,
        gerado_em: quando(),
    })
    .unwrap();
    validar_no_xsd(&m.xml, "solicitacao").unwrap();
    let t: String = m.xml.iter().map(|b| char::from(*b)).collect();
    assert!(t.contains("<ans:tipoTransacao>SOLICITACAO_PROCEDIMENTOS</ans:tipoTransacao>"));
    assert!(t.contains("<ans:numeroCarteira>01234567</ans:numeroCarteira>"));
    assert!(t.contains("&lt;cardíaco&gt; &amp; dispneia"));
    assert_eq!(m.sha256.len(), 64);
}

#[test]
fn solicitacao_sem_dados_obrigatorios_nao_e_montada() {
    let mut g = guia_ok();
    g.solicitante.cbos = None;
    let procs = procedimentos();
    let pedido = |g: &vidacor_dominio_faturamento::GuiaFaturavel,
                  procs: &[ProcedimentoSolicitado]| {
        montar_solicitacao(&PedidoAutorizacao {
            numero_guia_prestador: "AUT-2026-000001",
            sequencial: 1,
            registro_ans: "123456",
            codigo_prestador: "0004567",
            nome_prestador: "VidaCor",
            cnes: "1234567",
            carteirinha: &g.carteirinha,
            solicitante: &g.solicitante,
            data_solicitacao: NaiveDate::from_ymd_opt(2026, 11, 8).unwrap(),
            indicacao_clinica: None,
            procedimentos: procs,
            gerado_em: quando(),
        })
        .is_err()
    };
    assert!(pedido(&g, &procs), "solicitante sem CBO");
    assert!(pedido(&guia_ok(), &[]), "sem procedimento");
}

#[test]
fn resposta_autorizada_passa_no_xsd_e_e_lida_com_senha_validade_e_quantidades() {
    let procs = [("40901114", 1, 1), ("40901300", 1, 1)];
    let xml = resposta_simulada(&simulada(StatusAutorizacao::Autorizada, &procs, &[]));
    validar_no_xsd(&xml, "resposta-autorizada").unwrap();
    let r = ler_resposta(&xml).unwrap();
    assert_eq!(r.status, StatusAutorizacao::Autorizada);
    assert_eq!(r.numero_guia_prestador.as_deref(), Some("AUT-2026-000001"));
    assert_eq!(r.numero_guia_operadora.as_deref(), Some("UN-77231"));
    assert_eq!(r.senha.as_deref(), Some("UN-88231-X"));
    assert_eq!(r.validade, NaiveDate::from_ymd_opt(2026, 12, 8));
    assert_eq!(r.procedimentos.len(), 2);
    assert!(r.hash_confere);
}

#[test]
fn autorizado_a_menos_e_parcial_e_negada_traz_os_motivos() {
    let procs = [("40901114", 1, 1), ("40901300", 1, 0)];
    let xml = resposta_simulada(&simulada(StatusAutorizacao::Autorizada, &procs, &[]));
    assert_eq!(
        ler_resposta(&xml).unwrap().status,
        StatusAutorizacao::Parcial,
        "o servidor decide pela quantidade"
    );
    let xml = resposta_simulada(&RespostaSimulada {
        senha: None,
        validade: None,
        numero_guia_operadora: None,
        ..simulada(StatusAutorizacao::Negada, &[], &["1423"])
    });
    validar_no_xsd(&xml, "resposta-negada").unwrap();
    let r = ler_resposta(&xml).unwrap();
    assert_eq!(r.status, StatusAutorizacao::Negada);
    assert_eq!(r.motivos, vec!["1423".to_owned()]);
    let xml = resposta_simulada(&RespostaSimulada {
        senha: None,
        validade: None,
        ..simulada(StatusAutorizacao::EmAnalise, &[], &[])
    });
    let r = ler_resposta(&xml).unwrap();
    assert_eq!(r.status, StatusAutorizacao::EmAnalise);
    assert!(!r.status.final_());
}

#[test]
fn autorizada_sem_senha_ou_sem_procedimento_e_incompleta() {
    let procs = [("40901114", 1, 1)];
    let xml = resposta_simulada(&RespostaSimulada {
        senha: None,
        ..simulada(StatusAutorizacao::Autorizada, &procs, &[])
    });
    assert_eq!(ler_resposta(&xml), Err(ErroResposta::Incompleta("a senha")));
    let zero = [("40901114", 1, 0)];
    let xml = resposta_simulada(&simulada(StatusAutorizacao::Autorizada, &zero, &[]));
    assert_eq!(
        ler_resposta(&xml),
        Err(ErroResposta::Incompleta("procedimento autorizado"))
    );
}

#[test]
fn mensagem_de_erro_da_operadora_e_recusa() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?><ans:mensagemTISS xmlns:ans="http://www.ans.gov.br/padroes/tiss/schemas"><ans:cabecalho><ans:identificacaoTransacao><ans:tipoTransacao>RESPOSTA_SOLICITACAO</ans:tipoTransacao><ans:sequencialTransacao>1</ans:sequencialTransacao><ans:dataRegistroTransacao>2026-11-08</ans:dataRegistroTransacao><ans:horaRegistroTransacao>10:00:00</ans:horaRegistroTransacao></ans:identificacaoTransacao><ans:origem><ans:registroANS>123456</ans:registroANS></ans:origem><ans:destino><ans:identificacaoPrestador><ans:codigoPrestadorNaOperadora>0004567</ans:codigoPrestadorNaOperadora></ans:identificacaoPrestador></ans:destino><ans:Padrao>4.03.00</ans:Padrao></ans:cabecalho><ans:operadoraParaPrestador><ans:autorizacaoServicos><ans:mensagemErro><ans:codigoGlosa>1001</ans:codigoGlosa><ans:descricaoGlosa>Carteirinha inválida</ans:descricaoGlosa></ans:mensagemErro></ans:autorizacaoServicos></ans:operadoraParaPrestador><ans:epilogo><ans:hash>00</ans:hash></ans:epilogo></ans:mensagemTISS>"#.as_bytes();
    validar_no_xsd(xml, "mensagem-erro").unwrap();
    let r = ler_resposta(xml).unwrap();
    assert_eq!(r.status, StatusAutorizacao::Recusada);
    assert_eq!(r.motivos, vec!["1001".to_owned()]);
    assert_eq!(r.observacao.as_deref(), Some("Carteirinha inválida"));
    assert!(!r.hash_confere, "hash errado é informado");
}

#[test]
fn dtd_entidade_tamanho_e_outra_mensagem_sao_recusados() {
    let xxe = br#"<?xml version="1.0"?><!DOCTYPE x [<!ENTITY e SYSTEM "file:///etc/passwd">]><ans:mensagemTISS xmlns:ans="http://www.ans.gov.br/padroes/tiss/schemas">&e;</ans:mensagemTISS>"#;
    assert!(matches!(ler_resposta(xxe), Err(ErroResposta::Ilegivel(_))));
    let grande = vec![b' '; 512 * 1024 + 1];
    assert_eq!(ler_resposta(&grande), Err(ErroResposta::Grande));
    assert_eq!(
        ler_resposta(b"<outro/>"),
        Err(ErroResposta::NaoEhAutorizacao)
    );
    assert!(matches!(
        ler_resposta(b"\xff\xfe"),
        Err(ErroResposta::Ilegivel(_))
    ));
    // Resposta alterada depois de assinada pelo hash: lida, mas com o hash marcado.
    let procs = [("40901114", 1, 1)];
    let xml = resposta_simulada(&simulada(StatusAutorizacao::Autorizada, &procs, &[]));
    let alterada: Vec<u8> = String::from_utf8_lossy(&xml)
        .replace("UN-88231-X", "UN-00000-0")
        .into_bytes();
    let r = ler_resposta(&alterada).unwrap();
    assert!(!r.hash_confere);
}
