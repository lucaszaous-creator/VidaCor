//! Preparo do exame e documentos do atendimento (comprovante com preparo, declaração de
//! comparecimento e recibo). Rota HTTP real e PostgreSQL real (G4); nada é simulado. Só dados
//! fictícios.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{HeaderMap, Method, StatusCode};
use comum::{Ambiente, UNIDADE_A};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const NASC: &str = "1954-06-15";
const SONIA: &str = "Sônia Maria Castro";
const ECO: &str = "40901114";

struct Resposta {
    status: StatusCode,
    cabecalhos: HeaderMap,
    corpo: Vec<u8>,
}

impl Resposta {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.corpo).unwrap_or(Value::Null)
    }

    fn codigo(&self) -> String {
        self.json()["codigo"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }

    fn cabecalho(&self, nome: &str) -> &str {
        self.cabecalhos
            .get(nome)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
    }

    /// PDF íntegro: começa com %PDF, SHA-256 do cabeçalho igual ao dos bytes, nome do arquivo só com
    /// códigos. Devolve (sha, código da emissão).
    fn conferir_pdf(&self, nome_do_paciente: &str) -> (String, String) {
        assert_eq!(self.status, StatusCode::OK, "{}", self.json());
        assert!(self.corpo.starts_with(b"%PDF"));
        assert_eq!(self.cabecalho("content-type"), "application/pdf");
        assert_eq!(self.cabecalho("cache-control"), "no-store");
        let sha: String = Sha256::digest(&self.corpo)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(self.cabecalho("x-documento-sha256"), sha);
        let disposicao = self.cabecalho("content-disposition");
        for parte in nome_do_paciente.split(' ') {
            assert!(
                !disposicao.contains(parte),
                "nome fora do arquivo: {disposicao}"
            );
        }
        (sha, self.cabecalho("x-documento-codigo").to_owned())
    }
}

async fn baixar(amb: &Ambiente, login: &str, uri: &str) -> Resposta {
    let s = amb.entrar(login).await;
    let r = amb
        .app
        .clone()
        .oneshot(amb.get(uri, Some(&s)))
        .await
        .unwrap();
    let (partes, corpo) = r.into_parts();
    Resposta {
        status: partes.status,
        cabecalhos: partes.headers,
        corpo: corpo.collect().await.unwrap().to_bytes().to_vec(),
    }
}

async fn publicar(amb: &Ambiente, login: &str, chave: Uuid, corpo: Value) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/preparos/{ECO}"),
            &s,
            Some(chave),
            corpo,
        ))
        .await;
    (st, v)
}

async fn emissao_registrada(amb: &Ambiente, codigo: &str) -> (String, String) {
    sqlx::query_as::<_, (String, String)>(
        "SELECT tipo, sha256 FROM documentos.emissao WHERE codigo = $1",
    )
    .bind(codigo)
    .fetch_one(&amb.admin)
    .await
    .unwrap()
}

#[tokio::test]
async fn preparo_versionado_com_motivo_e_duas_edicoes_ao_mesmo_tempo_so_uma_vale() {
    let amb = Ambiente::novo().await;
    let v1 = json!({ "versao_vista": null, "passos": ["Venha com roupa confortável.", "Traga o pedido médico."], "chegar_antes_min": 15 });
    let chave = Uuid::now_v7();
    let (st, a) = publicar(&amb, "patricia.lemos", chave, v1.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{a}");
    assert_eq!(
        (a["numero"].as_i64(), a["vigente"].as_bool()),
        (Some(1), Some(true))
    );
    // Repetir a mesma chave devolve a mesma resposta e não cria outra versão (G3).
    let (st2, a2) = publicar(&amb, "patricia.lemos", chave, v1.clone()).await;
    assert_eq!(
        (st2, a2["id"].clone()),
        (StatusCode::CREATED, a["id"].clone())
    );

    // Recepção, caixa, médico e TI não publicam.
    for login in ["ana.souza", "carla.caixa", "medico.a", "ti.a"] {
        assert_eq!(
            publicar(&amb, login, Uuid::now_v7(), v1.clone()).await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    // Limites: sem passos, passo curto, antecedência fora de 0 a 120, segunda versão sem motivo.
    for (corpo, codigo) in [
        (
            json!({ "versao_vista": 1, "passos": [], "chegar_antes_min": 15, "motivo": "Revisão do texto" }),
            "preparo-invalido",
        ),
        (
            json!({ "versao_vista": 1, "passos": ["ab"], "chegar_antes_min": 15, "motivo": "Revisão do texto" }),
            "preparo-invalido",
        ),
        (
            json!({ "versao_vista": 1, "passos": ["Passo válido"], "chegar_antes_min": 121, "motivo": "Revisão do texto" }),
            "preparo-invalido",
        ),
        (
            json!({ "versao_vista": 1, "passos": ["Passo válido"], "chegar_antes_min": 15 }),
            "motivo-obrigatorio",
        ),
    ] {
        let (st, e) = publicar(&amb, "patricia.lemos", Uuid::now_v7(), corpo).await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo))
        );
    }
    // Versão vista desatualizada: 409 e nada muda.
    let (st, e) = publicar(
        &amb,
        "patricia.lemos",
        Uuid::now_v7(),
        json!({ "versao_vista": null, "passos": ["Outro texto"], "chegar_antes_min": 10 }),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("preparo-mudou"))
    );

    // Duas edições simultâneas sobre a versão 1: só uma entra (G2).
    let v2 = |texto: &str| json!({ "versao_vista": 1, "passos": [texto], "chegar_antes_min": 30, "motivo": "Orientação revisada pelo responsável técnico" });
    let (r1, r2) = tokio::join!(
        publicar(
            &amb,
            "patricia.lemos",
            Uuid::now_v7(),
            v2("Jejum de 4 horas.")
        ),
        publicar(
            &amb,
            "patricia.lemos",
            Uuid::now_v7(),
            v2("Jejum de 6 horas.")
        ),
    );
    let mut status = [r1.0, r2.0];
    status.sort();
    assert_eq!(
        status,
        [StatusCode::CREATED, StatusCode::CONFLICT],
        "{} {}",
        r1.1,
        r2.1
    );
    let vigentes: i64 = amb
        .contar("SELECT count(*) FROM agenda.preparo WHERE vigente")
        .await;
    assert_eq!(vigentes, 1);

    // A recepção lê a lista; a versão 1 continua guardada, não vigente.
    let ana = amb.entrar("ana.souza").await;
    let (st, _, lista) = amb.pedir(amb.get("/v1/preparos", Some(&ana))).await;
    assert_eq!(st, StatusCode::OK);
    let eco = lista
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["tuss"] == ECO)
        .unwrap();
    assert_eq!(eco["vigente"]["numero"].as_i64(), Some(2));
    assert_eq!(eco["vigente"]["chegar_antes_min"].as_i64(), Some(30));
    let (_, _, versoes) = amb
        .pedir(amb.get(&format!("/v1/preparos/{ECO}"), Some(&ana)))
        .await;
    let versoes = versoes.as_array().unwrap();
    assert_eq!(versoes.len(), 2);
    assert_eq!(versoes[1]["vigente"].as_bool(), Some(false));
    assert_eq!(versoes[1]["passos"].as_array().unwrap().len(), 2);
    // Caixa não lê; exame fora do catálogo: 404.
    let carla = amb.entrar("carla.caixa").await;
    assert_eq!(
        amb.pedir(amb.get("/v1/preparos", Some(&carla))).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        amb.pedir(amb.get("/v1/preparos/99999999", Some(&ana)))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    // Versão guardada nunca é apagada nem reescrita, nem pelo dono do banco.
    assert!(sqlx::query("DELETE FROM agenda.preparo")
        .execute(&amb.admin)
        .await
        .is_err());
    assert!(
        sqlx::query("UPDATE agenda.preparo SET passos = ARRAY['trocado']")
            .execute(&amb.admin)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn comprovante_e_declaracao_registram_a_emissao_e_respeitam_perfil_e_unidade() {
    let amb = Ambiente::novo().await;
    let agendamento = amb.s.agendamentos[ANTONIO];
    let uri = format!("/v1/agendamentos/{agendamento}/comprovante");
    // Sem preparo cadastrado o comprovante sai assim mesmo (o documento diz que não há orientação).
    let r = baixar(&amb, "ana.souza", &uri).await;
    let (sha, codigo) = r.conferir_pdf(ANTONIO);
    assert!(codigo.starts_with("CPV-"), "{codigo}");
    assert_eq!(
        emissao_registrada(&amb, &codigo).await,
        ("comprovante_agendamento".into(), sha)
    );
    let eventos: i64 = amb
        .contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'comprovante_emitido'")
        .await;
    assert_eq!(eventos, 1);
    // A gerência também emite; caixa, técnico, médico e TI não; outra unidade não descobre (404).
    baixar(&amb, "patricia.lemos", &uri)
        .await
        .conferir_pdf(ANTONIO);
    for login in ["carla.caixa", "bruno.sales", "medico.a", "ti.a"] {
        assert_eq!(
            baixar(&amb, login, &uri).await.status,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    let outra = baixar(&amb, "recepcao.b", &uri).await;
    assert_eq!(
        (outra.status, outra.codigo()),
        (StatusCode::NOT_FOUND, "agendamento-nao-encontrado".into())
    );
    let inexistente = baixar(
        &amb,
        "ana.souza",
        &format!("/v1/agendamentos/{}/comprovante", Uuid::now_v7()),
    )
    .await;
    assert_eq!(inexistente.status, StatusCode::NOT_FOUND);

    // Declaração: só depois do check-in; cancelado não tem.
    let atd = amb.fazer_checkin(ANTONIO, NASC).await;
    let atd_id = atd["id"].as_str().unwrap();
    let uri = format!("/v1/atendimentos/{atd_id}/declaracao");
    let r = baixar(&amb, "ana.souza", &uri).await;
    let (sha, codigo) = r.conferir_pdf(ANTONIO);
    assert!(codigo.starts_with("DCL-"), "{codigo}");
    assert_eq!(
        emissao_registrada(&amb, &codigo).await,
        ("declaracao_comparecimento".into(), sha)
    );
    for login in ["carla.caixa", "bruno.sales", "ti.a"] {
        assert_eq!(
            baixar(&amb, login, &uri).await.status,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    assert_eq!(
        baixar(&amb, "recepcao.b", &uri).await.status,
        StatusCode::NOT_FOUND
    );
    sqlx::query("UPDATE recepcao.atendimento SET situacao = 'cancelado' WHERE id = $1::uuid")
        .bind(atd_id)
        .execute(&amb.admin)
        .await
        .unwrap();
    let r = baixar(&amb, "ana.souza", &uri).await;
    assert_eq!(
        (r.status, r.codigo()),
        (StatusCode::CONFLICT, "atendimento-cancelado".into())
    );

    // Agendamento cancelado não tem comprovante.
    let maria = amb.s.agendamentos["Maria Helena Duarte"];
    sqlx::query("UPDATE agenda.agendamento SET situacao = 'cancelado', motivo = 'Paciente pediu para desmarcar' WHERE id = $1")
        .bind(maria)
        .execute(&amb.admin)
        .await
        .unwrap();
    let r = baixar(
        &amb,
        "ana.souza",
        &format!("/v1/agendamentos/{maria}/comprovante"),
    )
    .await;
    assert_eq!(
        (r.status, r.codigo()),
        (StatusCode::CONFLICT, "agendamento-encerrado".into())
    );
    // Emissão é somente inserção.
    assert!(sqlx::query("DELETE FROM documentos.emissao")
        .execute(&amb.admin)
        .await
        .is_err());
}

#[tokio::test]
async fn recibo_so_do_recebimento_pago_e_so_pelo_caixa() {
    let amb = Ambiente::novo().await;
    let tabela = Uuid::now_v7();
    sqlx::query("INSERT INTO financeiro.tabela_preco (id, versao, vigente_desde, descricao, origem, unidade_id, correlacao) VALUES ($1, 12, (now() AT TIME ZONE 'America/Sao_Paulo')::date - 1, 'Particular 2026', 'seed', $2, 'seed')")
        .bind(tabela).bind(UNIDADE_A).execute(&amb.admin).await.unwrap();
    sqlx::query(
        "INSERT INTO financeiro.preco (tabela_id, tuss, valor_centavos) VALUES ($1, $2, 38000)",
    )
    .bind(tabela)
    .bind(ECO)
    .execute(&amb.admin)
    .await
    .unwrap();
    let carla = amb.entrar("carla.caixa").await;
    let (st, _, c) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/caixas",
            &carla,
            None,
            json!({ "fundo_centavos": 20000 }),
        ))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    let receber = |forma: &'static str| {
        let corpo = json!({ "paciente_id": amb.s.pacientes[SONIA], "atendimento_id": null,
            "itens": [{ "tuss": ECO, "valor_centavos": 1 }], "desconto": null, "forma": forma });
        amb.json(
            Method::POST,
            "/v1/recebimentos",
            &carla,
            Some(Uuid::now_v7()),
            corpo,
        )
    };
    let (st, _, pago) = amb.pedir(receber("dinheiro")).await;
    assert_eq!(
        (st, pago["situacao"].as_str()),
        (StatusCode::CREATED, Some("pago")),
        "{pago}"
    );
    let uri = format!(
        "/v1/caixa/recebimentos/{}/recibo",
        pago["id"].as_str().unwrap()
    );
    let r = baixar(&amb, "carla.caixa", &uri).await;
    let (sha, codigo) = r.conferir_pdf(SONIA);
    assert!(codigo.starts_with("RCB-"), "{codigo}");
    assert_eq!(
        emissao_registrada(&amb, &codigo).await,
        ("recibo".into(), sha)
    );
    for login in ["ana.souza", "medico.a", "ti.a", "patricia.lemos"] {
        assert_eq!(
            baixar(&amb, login, &uri).await.status,
            StatusCode::FORBIDDEN,
            "{login}"
        );
    }
    // PIX ainda não confirmado: sem recibo.
    let (st, _, pix) = amb.pedir(receber("pix")).await;
    assert_eq!(st, StatusCode::CREATED, "{pix}");
    let r = baixar(
        &amb,
        "carla.caixa",
        &format!(
            "/v1/caixa/recebimentos/{}/recibo",
            pix["id"].as_str().unwrap()
        ),
    )
    .await;
    assert_eq!(
        (r.status, r.codigo()),
        (StatusCode::CONFLICT, "recebimento-nao-pago".into())
    );
    let r = baixar(
        &amb,
        "carla.caixa",
        &format!("/v1/caixa/recebimentos/{}/recibo", Uuid::now_v7()),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}
