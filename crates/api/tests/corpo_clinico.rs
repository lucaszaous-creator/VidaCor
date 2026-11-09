//! Corpo clínico e cabeçalho dos documentos (docs/04: médico novo e mudanças só valem após segunda
//! conferência; docs/18 § 6; DEC-2026-11-08-91 e 92). Rota HTTP real e PostgreSQL real (G4): caminho
//! feliz, erros, repetição, duas requisições simultâneas, limites e acesso indevido. Só dados fictícios.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::{Ambiente, UNIDADE_A, UNIDADE_B};
use serde_json::{json, Value};
use uuid::Uuid;

async fn post(
    amb: &Ambiente,
    login: &str,
    uri: &str,
    chave: Option<Uuid>,
    corpo: Value,
) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.json(Method::POST, uri, &s, chave, corpo))
        .await;
    (st, v)
}

async fn get(amb: &Ambiente, login: &str, uri: &str) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb.pedir(amb.get(uri, Some(&s))).await;
    (st, v)
}

/// Segunda pessoa da gerência na unidade A e uma gerente da unidade B (fictícias).
async fn mais_gerentes(amb: &Ambiente) {
    for (login, nome, unidade) in [
        ("helena.gerencia", "Helena Prado", UNIDADE_A),
        ("gerente.b", "Gerência Norte", UNIDADE_B),
    ] {
        sqlx::query("INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao) VALUES ($1, $2, $3, $4, ARRAY['gerente'], 'seed', $5, 'seed')")
            .bind(Uuid::now_v7()).bind(format!("USR-{login}")).bind(login).bind(nome).bind(unidade)
            .execute(&amb.admin).await.unwrap();
    }
}

fn crm(numero: &str) -> Value {
    json!({ "tipo": "registro", "registro": { "uf": "sp", "numero": numero, "rqe": "7788", "especialidade": "Cardiologia" }, "motivo": "cadastro do corpo clínico" })
}

fn medico(lista: &Value, nome: &str) -> Value {
    lista
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["nome"] == nome)
        .cloned()
        .unwrap_or(Value::Null)
}

/// Registra a consulta do CRM feita no portal do CFM (gerente A), exigida para aprovar o registro.
async fn consultar_no_cfm(amb: &Ambiente, medico_id: Uuid, numero: &str, situacao: &str) {
    let (st, c) = post(
        amb,
        "patricia.lemos",
        &format!("/v1/corpo-clinico/{medico_id}/consultas-crm"),
        None,
        json!({ "uf": "SP", "numero": numero, "fonte": "portal_cfm", "situacao": situacao, "observacao": "portal do CFM, busca por CRM e UF" }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
}

/// Pede (gerente A) e confere (Helena) um pedido; devolve o médico depois da conferência.
async fn pedir_e_conferir(amb: &Ambiente, medico_id: Uuid, corpo: Value) -> Value {
    let (st, m) = post(
        amb,
        "patricia.lemos",
        &format!("/v1/corpo-clinico/{medico_id}/pedidos"),
        Some(Uuid::now_v7()),
        corpo,
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{m}");
    let pedido = m["pendentes"][0]["id"].as_str().unwrap().to_owned();
    if let Some(numero) = m["pendentes"][0]["registro"]["numero"].as_str() {
        consultar_no_cfm(amb, medico_id, numero, "regular").await;
    }
    let (st, d) = post(
        amb,
        "helena.gerencia",
        &format!("/v1/corpo-clinico/pedidos/{pedido}/decisao"),
        None,
        json!({ "aprovar": true, "motivo": "conferido com o documento do conselho" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{d}");
    d
}

#[tokio::test]
async fn quando_a_gerencia_cadastra_o_crm_entao_so_vale_depois_da_segunda_conferencia() {
    let amb = Ambiente::novo().await;
    mais_gerentes(&amb).await;
    let paulo = amb.s.usuarios["medico.a"];
    let (st, lista) = get(&amb, "patricia.lemos", "/v1/corpo-clinico").await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        lista.as_array().unwrap().len(),
        3,
        "só os médicos da unidade"
    );
    assert_eq!(
        medico(&lista, "Dr. Paulo Medeiros")["situacao"],
        "sem_registro"
    );

    let chave = Uuid::now_v7();
    let uri = format!("/v1/corpo-clinico/{paulo}/pedidos");
    let (st, m) = post(&amb, "patricia.lemos", &uri, Some(chave), crm("123456")).await;
    assert_eq!(st, StatusCode::CREATED, "{m}");
    assert_eq!(
        (m["situacao"].as_str(), m["registro"].is_null()),
        (Some("aguardando_conferencia"), true)
    );
    assert_eq!(m["pendentes"][0]["registro"]["uf"], "SP", "UF normalizada");
    // Clique duplo: mesma chave, mesma resposta, um pedido só.
    let (st2, m2) = post(&amb, "patricia.lemos", &uri, Some(chave), crm("123456")).await;
    assert_eq!((st2, &m2), (StatusCode::CREATED, &m));
    assert_eq!(amb.contar("SELECT count(*) FROM corpo.pedido").await, 1);

    let pedido = m["pendentes"][0]["id"].as_str().unwrap().to_owned();
    let decisao_uri = format!("/v1/corpo-clinico/pedidos/{pedido}/decisao");
    let aprova = json!({ "aprovar": true, "motivo": "conferido no CFM" });
    // Quem pediu não confere.
    let (st, e) = post(&amb, "patricia.lemos", &decisao_uri, None, aprova.clone()).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::FORBIDDEN, Some("separacao-de-funcoes"))
    );
    // O próprio médico não tem o ato.
    let (st, _) = post(&amb, "medico.a", &decisao_uri, None, aprova.clone()).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Sem consulta no CFM, ou com situação irregular, o registro não é aprovado (DEC-2026-11-09-120).
    let (st, e) = post(&amb, "helena.gerencia", &decisao_uri, None, aprova.clone()).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("crm-sem-consulta"))
    );
    consultar_no_cfm(&amb, paulo, "123456", "suspenso").await;
    let (st, e) = post(&amb, "helena.gerencia", &decisao_uri, None, aprova.clone()).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("crm-irregular"))
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM corpo.pedido WHERE situacao = 'aguardando'")
            .await,
        1
    );
    consultar_no_cfm(&amb, paulo, "123456", "regular").await;
    let (st, d) = post(&amb, "helena.gerencia", &decisao_uri, None, aprova.clone()).await;
    assert_eq!(st, StatusCode::OK, "{d}");
    assert_eq!(
        (
            d["situacao"].as_str(),
            d["registro"]["numero"].as_str(),
            d["registro"]["rqe"].as_str(),
            d["registro"]["especialidade"].as_str()
        ),
        (
            Some("ativo"),
            Some("123456"),
            Some("7788"),
            Some("Cardiologia")
        )
    );
    let (st, _) = post(&amb, "helena.gerencia", &decisao_uri, None, aprova).await;
    assert_eq!(st, StatusCode::CONFLICT, "já decidido");

    // Correção: versão nova com motivo, a anterior sai de vigente e continua guardada.
    let corrigido = pedir_e_conferir(&amb, paulo, crm("123457")).await;
    assert_eq!(corrigido["registro"]["numero"], "123457");
    assert_eq!(
        amb.contar("SELECT count(*) FROM documentos.registro_profissional")
            .await,
        2,
        "nada é apagado"
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM documentos.registro_profissional WHERE vigente")
            .await,
        1
    );
    let (st, h) = get(
        &amb,
        "patricia.lemos",
        &format!("/v1/corpo-clinico/{paulo}/historico"),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let h = h.as_array().unwrap();
    assert_eq!(h.len(), 2);
    assert!(h
        .iter()
        .all(|p| p["situacao"] == "aplicado" && p["decidido_por"] == "Helena Prado"));
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE dominio = 'corpo_clinico' AND tipo <> 'crm_consultado'")
            .await,
        4
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE dominio = 'corpo_clinico' AND tipo = 'crm_consultado'")
            .await,
        3,
        "cada consulta ao CFM entra no lastro"
    );
}

#[tokio::test]
async fn quando_o_pedido_tem_erro_ou_passa_do_limite_entao_nada_e_pedido() {
    let amb = Ambiente::novo().await;
    mais_gerentes(&amb).await;
    let paulo = amb.s.usuarios["medico.a"];
    let uri = format!("/v1/corpo-clinico/{paulo}/pedidos");
    for (corpo, codigo) in [
        (
            json!({ "tipo": "registro", "registro": { "uf": "XX", "numero": "123" }, "motivo": "cadastro novo" }),
            "corpo-clinico-invalido",
        ),
        (
            json!({ "tipo": "registro", "registro": { "uf": "SP", "numero": "12a" }, "motivo": "cadastro novo" }),
            "corpo-clinico-invalido",
        ),
        (
            json!({ "tipo": "registro", "registro": { "uf": "SP", "numero": "12345678901" }, "motivo": "cadastro novo" }),
            "corpo-clinico-invalido",
        ),
        (
            json!({ "tipo": "registro", "registro": { "uf": "SP", "numero": "" }, "motivo": "cadastro novo" }),
            "corpo-clinico-invalido",
        ),
        (
            json!({ "tipo": "registro", "registro": { "uf": "SP", "numero": "123", "rqe": "55" }, "motivo": "cadastro novo" }),
            "corpo-clinico-invalido",
        ),
        (
            json!({ "tipo": "registro", "registro": { "uf": "SP", "numero": "123" }, "motivo": "curt" }),
            "motivo-invalido",
        ),
        (
            json!({ "tipo": "registro", "registro": { "uf": "SP", "numero": "123" }, "motivo": "    " }),
            "motivo-invalido",
        ),
        (
            json!({ "tipo": "registro", "motivo": "sem os dados do registro" }),
            "corpo-clinico-invalido",
        ),
        (
            json!({ "tipo": "perfil", "motivo": "tipo que não existe" }),
            "corpo-clinico-invalido",
        ),
        (
            json!({ "tipo": "habilitacao", "habilitacao": { "realiza": ["ULTRA"], "lauda": [] }, "motivo": "modalidade errada" }),
            "corpo-clinico-invalido",
        ),
    ] {
        let (st, e) = post(
            &amb,
            "patricia.lemos",
            &uri,
            Some(Uuid::now_v7()),
            corpo.clone(),
        )
        .await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo)),
            "{corpo}"
        );
    }
    // Sem a chave que evita duplicidade.
    let (st, e) = post(&amb, "patricia.lemos", &uri, None, crm("123")).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::BAD_REQUEST, Some("idempotency-key-ausente"))
    );
    // Usuário que não é médico, ou que não existe.
    let ana = amb.s.usuarios["ana.souza"];
    for alvo in [ana, Uuid::now_v7()] {
        let (st, _) = post(
            &amb,
            "patricia.lemos",
            &format!("/v1/corpo-clinico/{alvo}/pedidos"),
            Some(Uuid::now_v7()),
            crm("123"),
        )
        .await;
        assert_eq!(st, StatusCode::NOT_FOUND);
    }
    assert_eq!(amb.contar("SELECT count(*) FROM corpo.pedido").await, 0);

    // Recusa: o cadastro não muda e o pedido fica no histórico com o motivo.
    let (st, m) = post(
        &amb,
        "patricia.lemos",
        &uri,
        Some(Uuid::now_v7()),
        crm("999"),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    let pedido = m["pendentes"][0]["id"].as_str().unwrap().to_owned();
    // Um pedido do mesmo tipo aguardando por vez.
    let (st, e) = post(
        &amb,
        "patricia.lemos",
        &uri,
        Some(Uuid::now_v7()),
        crm("998"),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("pedido-aguardando"))
    );
    let (st, _) = post(
        &amb,
        "helena.gerencia",
        &format!("/v1/corpo-clinico/pedidos/{pedido}/decisao"),
        None,
        json!({ "aprovar": true, "motivo": "ok" }),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "decisão sem motivo");
    let (st, d) = post(
        &amb,
        "helena.gerencia",
        &format!("/v1/corpo-clinico/pedidos/{pedido}/decisao"),
        None,
        json!({ "aprovar": false, "motivo": "número não confere com o CFM" }),
    )
    .await;
    assert_eq!(
        (st, d["registro"].is_null(), d["situacao"].as_str()),
        (StatusCode::OK, true, Some("sem_registro"))
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM corpo.pedido WHERE situacao = 'recusado'")
            .await,
        1
    );

    // Pedido igual ao vigente não é pedido.
    pedir_e_conferir(&amb, paulo, crm("123456")).await;
    let (st, e) = post(
        &amb,
        "patricia.lemos",
        &uri,
        Some(Uuid::now_v7()),
        crm("123456"),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("nada-a-mudar"))
    );
    // O mesmo CRM não vai para outro médico da unidade.
    let cecilia = amb.s.usuarios["cecilia.souto"];
    let (st, e) = post(
        &amb,
        "patricia.lemos",
        &format!("/v1/corpo-clinico/{cecilia}/pedidos"),
        Some(Uuid::now_v7()),
        crm("123456"),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("crm-em-uso"))
    );
}

#[tokio::test]
async fn quando_o_que_o_medico_realiza_e_lauda_muda_entao_vale_a_versao_conferida() {
    let amb = Ambiente::novo().await;
    mais_gerentes(&amb).await;
    let cecilia = amb.s.usuarios["cecilia.souto"];
    let d = pedir_e_conferir(
        &amb,
        cecilia,
        json!({ "tipo": "habilitacao", "habilitacao": { "realiza": ["us", " ct", "US"], "lauda": ["CT", "DX", "US"] }, "motivo": "passou a laudar raio X" }),
    )
    .await;
    assert_eq!(d["habilitacao"]["realiza"], json!(["CT", "US"]));
    assert_eq!(d["habilitacao"]["lauda"], json!(["CT", "DX", "US"]));
    let d = pedir_e_conferir(
        &amb,
        cecilia,
        json!({ "tipo": "habilitacao", "habilitacao": { "realiza": ["US"], "lauda": ["US"] }, "motivo": "afastada da tomografia" }),
    )
    .await;
    assert_eq!(d["habilitacao"]["lauda"], json!(["US"]));
    assert_eq!(
        amb.contar("SELECT count(*) FROM corpo.habilitacao WHERE NOT vigente")
            .await,
        1
    );
}

#[tokio::test]
async fn quando_duas_conferencias_ou_dois_pedidos_chegam_juntos_entao_so_um_tem_efeito() {
    let amb = Ambiente::novo().await;
    mais_gerentes(&amb).await;
    let paulo = amb.s.usuarios["medico.a"];
    let uri = format!("/v1/corpo-clinico/{paulo}/pedidos");
    let ((s1, _), (s2, _)) = tokio::join!(
        post(
            &amb,
            "patricia.lemos",
            &uri,
            Some(Uuid::now_v7()),
            crm("111")
        ),
        post(
            &amb,
            "patricia.lemos",
            &uri,
            Some(Uuid::now_v7()),
            crm("222")
        ),
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
    let pedido: Uuid = sqlx::query_scalar("SELECT id FROM corpo.pedido")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    let numero: String = sqlx::query_scalar("SELECT dados->>'numero' FROM corpo.pedido")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    consultar_no_cfm(&amb, paulo, &numero, "regular").await;
    let decisao = format!("/v1/corpo-clinico/pedidos/{pedido}/decisao");
    let corpo = json!({ "aprovar": true, "motivo": "conferido no CFM" });
    let ((s1, _), (s2, _)) = tokio::join!(
        post(&amb, "helena.gerencia", &decisao, None, corpo.clone()),
        post(&amb, "helena.gerencia", &decisao, None, corpo.clone()),
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM documentos.registro_profissional")
            .await,
        1
    );
}

#[tokio::test]
async fn quando_quem_nao_e_da_gerencia_ou_de_outra_unidade_tenta_entao_e_recusado() {
    let amb = Ambiente::novo().await;
    mais_gerentes(&amb).await;
    let paulo = amb.s.usuarios["medico.a"];
    // Recepção, TI (sem dado clínico), médico e caixa não veem nem pedem.
    for login in ["ana.souza", "ti.a", "medico.a", "carla.caixa", "recepcao.b"] {
        let (st, _) = get(&amb, login, "/v1/corpo-clinico").await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
        let (st, _) = post(
            &amb,
            login,
            &format!("/v1/corpo-clinico/{paulo}/pedidos"),
            Some(Uuid::now_v7()),
            crm("123"),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    // Gerência de outra unidade não vê nem pede nem confere os médicos desta.
    let (st, lista) = get(&amb, "gerente.b", "/v1/corpo-clinico").await;
    assert_eq!((st, lista), (StatusCode::OK, json!([])));
    let (st, _) = post(
        &amb,
        "gerente.b",
        &format!("/v1/corpo-clinico/{paulo}/pedidos"),
        Some(Uuid::now_v7()),
        crm("123"),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = get(
        &amb,
        "gerente.b",
        &format!("/v1/corpo-clinico/{paulo}/historico"),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (_, m) = post(
        &amb,
        "patricia.lemos",
        &format!("/v1/corpo-clinico/{paulo}/pedidos"),
        Some(Uuid::now_v7()),
        crm("123"),
    )
    .await;
    let pedido = m["pendentes"][0]["id"].as_str().unwrap().to_owned();
    let (st, _) = post(
        &amb,
        "gerente.b",
        &format!("/v1/corpo-clinico/pedidos/{pedido}/decisao"),
        None,
        json!({ "aprovar": true, "motivo": "tentativa de outra unidade" }),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Reautenticação vencida: pedir e conferir esperam a senha de novo.
    let s = amb.entrar("helena.gerencia").await;
    amb.expirar_reautenticacao().await;
    let (st, _, e) = amb
        .pedir(amb.json(
            Method::POST,
            &format!("/v1/corpo-clinico/pedidos/{pedido}/decisao"),
            &s,
            None,
            json!({ "aprovar": true, "motivo": "conferido no CFM" }),
        ))
        .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("reautenticacao-necessaria"))
    );
    assert_eq!(
        amb.contar("SELECT count(*) FROM corpo.pedido WHERE situacao = 'aguardando'")
            .await,
        1
    );
}

#[tokio::test]
async fn quando_alguem_tenta_pular_a_conferencia_pelo_banco_entao_o_banco_barra() {
    let amb = Ambiente::novo().await;
    mais_gerentes(&amb).await;
    let paulo = amb.s.usuarios["medico.a"];
    let patricia = amb.s.usuarios["patricia.lemos"];
    // Papel da aplicação com contexto de gerente: versão sem pedido conferido não entra.
    let mut tx = amb.app_pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.usuario_id', $1, true), set_config('app.unidade_id', $2, true), set_config('app.perfis', 'gerente', true)")
        .bind(patricia.to_string()).bind(UNIDADE_A.to_string()).execute(&mut *tx).await.unwrap();
    let r = sqlx::query("INSERT INTO documentos.registro_profissional (id, usuario_id, conselho, uf, numero, origem, unidade_id, correlacao) VALUES ($1, $2, 'CRM', 'SP', '1', 'teste', $3, 'teste')")
        .bind(Uuid::now_v7()).bind(paulo).bind(UNIDADE_A).execute(&mut *tx).await;
    assert!(r.is_err(), "sem segunda conferência");
    drop(tx);
    let d = pedir_e_conferir(&amb, paulo, crm("123456")).await;
    assert_eq!(d["situacao"], "ativo");
    // Nem o dono do banco altera ou apaga a versão ou o pedido.
    for sql in [
        "UPDATE documentos.registro_profissional SET numero = '1'",
        "DELETE FROM documentos.registro_profissional",
        "UPDATE corpo.pedido SET motivo = 'trocado depois'",
        "DELETE FROM corpo.pedido",
    ] {
        assert!(sqlx::query(sql).execute(&amb.admin).await.is_err(), "{sql}");
    }
}

fn cabecalho(numero_atual: Option<i32>, cnpj: &str, motivo: Option<&str>) -> Value {
    json!({
        "numero_atual": numero_atual, "nome": "VidaCor Imagem Centro", "razao_social": "VidaCor Diagnóstico por Imagem Ltda.",
        "cnpj": cnpj, "endereco": "Rua Fictícia, 100, Centro", "telefone": "(11) 3000-0000", "site": "vidacorimagem.com.br",
        "responsavel_nome": "Dra. Cecília Souto", "responsavel_crm_uf": "SP", "responsavel_crm_numero": "123456", "motivo": motivo
    })
}

#[tokio::test]
async fn quando_a_gerencia_publica_o_cabecalho_entao_vira_versao_nova_e_a_anterior_fica_guardada() {
    let amb = Ambiente::novo().await;
    mais_gerentes(&amb).await;
    const URI: &str = "/v1/configuracoes/cabecalho";
    let (st, c) = get(&amb, "patricia.lemos", URI).await;
    assert_eq!((st, c["vigente"].is_null()), (StatusCode::OK, true));
    let chave = Uuid::now_v7();
    let primeiro = cabecalho(None, "11222333000181", None);
    let (st, c) = post(&amb, "patricia.lemos", URI, Some(chave), primeiro.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    assert_eq!(
        (
            c["vigente"]["numero"].as_i64(),
            c["vigente"]["cnpj"].as_str(),
            c["vigente"]["responsavel_tecnico"].as_str()
        ),
        (
            Some(1),
            Some("11.222.333/0001-81"),
            Some("Dra. Cecília Souto, CRM-SP 123456")
        )
    );
    let (st, c2) = post(&amb, "patricia.lemos", URI, Some(chave), primeiro.clone()).await;
    assert_eq!((st, &c2), (StatusCode::CREATED, &c), "repetir não duplica");
    // Quem viu "nenhuma versão" e publica depois da primeira: nada é gravado.
    let (st, e) = post(&amb, "helena.gerencia", URI, Some(Uuid::now_v7()), primeiro).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("cabecalho-mudou"))
    );
    for (corpo, codigo) in [
        (
            cabecalho(Some(1), "11222333000181", None),
            "motivo-invalido",
        ),
        (
            cabecalho(Some(1), "11.222.333/0001-82", Some("CNPJ corrigido")),
            "cabecalho-invalido",
        ),
        (
            cabecalho(Some(1), "00000000000000", Some("CNPJ zerado")),
            "cabecalho-invalido",
        ),
    ] {
        let (st, e) = post(&amb, "patricia.lemos", URI, Some(Uuid::now_v7()), corpo).await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo))
        );
    }
    // Duas publicações sobre a mesma versão ao mesmo tempo: só uma entra.
    let nova = cabecalho(Some(1), "11222333000181", Some("telefone novo da unidade"));
    let ((s1, _), (s2, _)) = tokio::join!(
        post(
            &amb,
            "patricia.lemos",
            URI,
            Some(Uuid::now_v7()),
            nova.clone()
        ),
        post(
            &amb,
            "helena.gerencia",
            URI,
            Some(Uuid::now_v7()),
            nova.clone()
        ),
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
    let (_, c) = get(&amb, "patricia.lemos", URI).await;
    assert_eq!(
        (
            c["vigente"]["numero"].as_i64(),
            c["versoes"].as_array().unwrap().len()
        ),
        (Some(2), 2)
    );
    assert!(sqlx::query("DELETE FROM documentos.cabecalho")
        .execute(&amb.admin)
        .await
        .is_err());
    // Só a gerência; outra unidade não vê o cabeçalho desta.
    for login in ["ana.souza", "ti.a", "medico.a"] {
        assert_eq!(
            get(&amb, login, URI).await.0,
            StatusCode::FORBIDDEN,
            "{login}"
        );
        let (st, _) = post(
            &amb,
            login,
            URI,
            Some(Uuid::now_v7()),
            cabecalho(Some(2), "11222333000181", Some("tentativa indevida")),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    let (st, c) = get(&amb, "gerente.b", URI).await;
    assert_eq!(
        (
            st,
            c["vigente"].is_null(),
            c["versoes"].as_array().unwrap().len()
        ),
        (StatusCode::OK, true, 0)
    );
}
