//! Modelos de laudo e frases rápidas (Modelos-Laudo.dc.html; DEC-2026-11-08-100): modelo pessoal
//! publica direto, o da clínica só com outra pessoa da coordenação médica; editar é nova versão com
//! motivo; nada é apagado; uso no rascunho grava a versão do modelo; acesso indevido recusado. Rota
//! real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::{Ambiente, UNIDADE_B};
use serde_json::{json, Value};
use uuid::Uuid;

const TEXTO: &str = "Técnica: aparelho [aparelho].\nAchados: [achados].\nConclusão: [conclusao].";

async fn pedir(
    amb: &Ambiente,
    login: &str,
    metodo: Method,
    uri: &str,
    corpo: Value,
    chave: Option<Uuid>,
) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let req = if metodo == Method::GET {
        amb.get(uri, Some(&s))
    } else {
        amb.json(metodo, uri, &s, chave, corpo)
    };
    let (st, _, c) = amb.pedir(req).await;
    (st, c)
}

/// Coordenação médica: médico com perfil de coordenação (DEC-2026-11-08-100).
async fn coordenacao(amb: &Ambiente, login: &str) {
    sqlx::query(
        "UPDATE identidade.usuario SET perfis = ARRAY['medico','gerente'] WHERE login = $1",
    )
    .bind(login)
    .execute(&amb.admin)
    .await
    .unwrap();
}

async fn medico_da_unidade_b(amb: &Ambiente) {
    sqlx::query("INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao) VALUES ($1,'USR-medico.b','medico.b','Dra. Norte',ARRAY['medico'],'seed',$2,'seed')")
        .bind(Uuid::now_v7()).bind(UNIDADE_B).execute(&amb.admin).await.unwrap();
}

async fn criar_modelo(amb: &Ambiente, login: &str, tipo: &str, nome: &str) -> Value {
    let (st, m) = pedir(
        amb,
        login,
        Method::POST,
        "/v1/modelos-de-laudo",
        json!({ "tipo": tipo, "nome": nome, "exame": "Ecocardiograma transtorácico", "modalidade": "us", "texto": TEXTO }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{m}");
    m
}

#[tokio::test]
async fn modelo_pessoal_publica_direto_e_editar_gera_versao_com_motivo() {
    let amb = Ambiente::novo().await;
    let chave = Uuid::now_v7();
    let corpo = json!({ "tipo": "pessoal", "nome": "Eco, Dr. Paulo", "exame": "Ecocardiograma transtorácico", "modalidade": "us", "texto": TEXTO });
    let (st, m) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        "/v1/modelos-de-laudo",
        corpo.clone(),
        Some(chave),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{m}");
    assert_eq!(
        (
            m["versao_publicada"].as_i64(),
            m["modalidade"].as_str(),
            m["autor"].as_str()
        ),
        (Some(1), Some("US"), Some("Dr. Paulo Medeiros"))
    );
    assert_eq!(m["pode_editar"], true);
    // Repetir com a mesma chave devolve o mesmo modelo, sem duplicar.
    let (st2, m2) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        "/v1/modelos-de-laudo",
        corpo.clone(),
        Some(chave),
    )
    .await;
    assert_eq!((st2, &m2), (StatusCode::CREATED, &m));
    assert_eq!(amb.contar("SELECT count(*) FROM laudos.modelo").await, 1);
    // Mesmo nome de novo (outra chave): recusado.
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        "/v1/modelos-de-laudo",
        corpo,
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("nome-em-uso")),
        "{e}"
    );

    let id = m["id"].as_str().unwrap();
    let uri = format!("/v1/modelos-de-laudo/{id}/versoes");
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &uri,
        json!({ "texto": "Novo", "motivo": "", "versao_base": 1 }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("motivo-obrigatorio"))
    );
    let (st, v2) = pedir(&amb, "medico.a", Method::POST, &uri, json!({ "texto": "Achados: [achados]. Frase nova.", "motivo": "Nova frase de conclusão", "versao_base": 1 }), Some(Uuid::now_v7())).await;
    assert_eq!(st, StatusCode::CREATED, "{v2}");
    assert_eq!(v2["versao_publicada"].as_i64(), Some(2));
    assert_eq!(v2["versoes"][1]["situacao"], "substituida");
    assert_eq!(v2["versoes"][0]["motivo"], "Nova frase de conclusão");
    // Quem editava a versão 1 recebe 409: a tela nunca grava por cima.
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &uri,
        json!({ "texto": "Outra", "motivo": "Mudança atrasada", "versao_base": 1 }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("versao-desatualizada"))
    );
    // Dado de paciente no modelo é recusado.
    let (st, e) = pedir(&amb, "medico.a", Method::POST, &uri, json!({ "texto": "Paciente CPF 123.456.789-09", "motivo": "teste de limite", "versao_base": 2 }), Some(Uuid::now_v7())).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("modelo-com-dado-de-paciente")
        )
    );
    // Nada é apagado nem reescrito, nem pelo dono do banco.
    assert!(
        sqlx::query("UPDATE laudos.modelo_versao SET texto = 'x' WHERE numero = 1")
            .execute(&amb.admin)
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM laudos.modelo")
        .execute(&amb.admin)
        .await
        .is_err());
    assert!(sqlx::query("DELETE FROM laudos.modelo_versao")
        .execute(&amb.admin)
        .await
        .is_err());

    // Modelo pessoal é só do dono: outro médico não vê nem edita.
    let (st, _) = pedir(
        &amb,
        "cecilia.souto",
        Method::GET,
        &format!("/v1/modelos-de-laudo/{id}"),
        Value::Null,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (_, lista) = pedir(
        &amb,
        "cecilia.souto",
        Method::GET,
        "/v1/modelos-de-laudo",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(lista.as_array().unwrap().len(), 0);
    let (st, _) = pedir(
        &amb,
        "cecilia.souto",
        Method::POST,
        &uri,
        json!({ "texto": "x", "motivo": "tentativa indevida", "versao_base": 2 }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Desativar exige motivo; desativado não recebe versão nova.
    let des = format!("/v1/modelos-de-laudo/{id}/desativacao");
    let (st, _) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &des,
        json!({ "motivo": "" }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, d) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &des,
        json!({ "motivo": "Substituído pelo modelo da clínica" }),
        None,
    )
    .await;
    assert_eq!(
        (st, d["ativo"].as_bool()),
        (StatusCode::OK, Some(false)),
        "{d}"
    );
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &des,
        json!({ "motivo": "De novo, por engano" }),
        None,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("modelo-desativado"))
    );
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &uri,
        json!({ "texto": "x", "motivo": "depois de desativar", "versao_base": 2 }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("modelo-desativado"))
    );
    assert_eq!(amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo IN ('modelo_criado','modelo_versao_publicada','modelo_desativado')").await, 3);
}

#[tokio::test]
async fn modelo_da_clinica_so_publica_com_aprovacao_de_outra_pessoa_da_coordenacao() {
    let amb = Ambiente::novo().await;
    coordenacao(&amb, "cecilia.souto").await;
    coordenacao(&amb, "marcos.freire").await;
    let m = criar_modelo(&amb, "medico.a", "clinica", "TC de crânio sem contraste").await;
    assert_eq!(
        (
            m["versao_publicada"].as_i64(),
            m["proposta_em_aberto"].as_i64(),
            m["autor"].as_str()
        ),
        (None, Some(1), Some("Clínica"))
    );
    let id = m["id"].as_str().unwrap();
    let decidir = |n: i32| format!("/v1/modelos-de-laudo/{id}/versoes/{n}/decisao");
    // Gerência sem perfil médico e médico sem coordenação não aprovam.
    for login in ["patricia.lemos", "medico.a"] {
        let (st, _) = pedir(
            &amb,
            login,
            Method::POST,
            &decidir(1),
            json!({ "aprovar": true }),
            None,
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    let (st, a) = pedir(
        &amb,
        "cecilia.souto",
        Method::POST,
        &decidir(1),
        json!({ "aprovar": true }),
        None,
    )
    .await;
    assert_eq!(
        (st, a["versao_publicada"].as_i64()),
        (StatusCode::OK, Some(1)),
        "{a}"
    );
    assert_eq!(a["versoes"][0]["decidido_por"], "Dra. Cecília Souto");
    let (st, e) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &decidir(1),
        json!({ "aprovar": true }),
        None,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("proposta-ja-decidida"))
    );

    // Quem propõe não aprova: nem pela rota, nem direto no banco.
    let uri = format!("/v1/modelos-de-laudo/{id}/versoes");
    let (st, p2) = pedir(&amb, "cecilia.souto", Method::POST, &uri, json!({ "texto": "Técnica: [aparelho]. Conclusão: [conclusao]. Limitações do método.", "motivo": "Inclui limitação do método", "versao_base": 1 }), Some(Uuid::now_v7())).await;
    assert_eq!(
        (
            st,
            p2["proposta_em_aberto"].as_i64(),
            p2["versao_publicada"].as_i64()
        ),
        (StatusCode::CREATED, Some(2), Some(1)),
        "{p2}"
    );
    assert_eq!(p2["pode_decidir"], false);
    let (st, e) = pedir(
        &amb,
        "cecilia.souto",
        Method::POST,
        &decidir(2),
        json!({ "aprovar": true }),
        None,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::FORBIDDEN, Some("quem-propoe-nao-aprova"))
    );
    let cecilia = amb.s.usuarios["cecilia.souto"];
    assert!(sqlx::query("UPDATE laudos.modelo_versao SET situacao = 'publicada', decidido_por = $1, decidido_em = now() WHERE numero = 2")
        .bind(cecilia).execute(&amb.admin).await.is_err());
    // Recusar exige motivo; a versão 1 continua valendo.
    let (st, e) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &decidir(2),
        json!({ "aprovar": false }),
        None,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("motivo-obrigatorio"))
    );
    let (st, r) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &decidir(2),
        json!({ "aprovar": false, "motivo": "Texto da limitação precisa de referência" }),
        None,
    )
    .await;
    assert_eq!(
        (
            st,
            r["versao_publicada"].as_i64(),
            r["versoes"][0]["situacao"].as_str()
        ),
        (StatusCode::OK, Some(1), Some("recusada")),
        "{r}"
    );

    // Duas propostas ao mesmo tempo: só uma fica em aberto (G2).
    let ((s1, _), (s2, _)) = tokio::join!(
        pedir(
            &amb,
            "medico.a",
            Method::POST,
            &uri,
            json!({ "texto": "Proposta A [x]", "motivo": "Proposta de ajuste A", "versao_base": 1 }),
            Some(Uuid::now_v7())
        ),
        pedir(
            &amb,
            "marcos.freire",
            Method::POST,
            &uri,
            json!({ "texto": "Proposta B [x]", "motivo": "Proposta de ajuste B", "versao_base": 1 }),
            Some(Uuid::now_v7())
        )
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM laudos.modelo_versao WHERE situacao = 'proposta'")
            .await,
        1
    );

    // Modelo da clínica sai de uso só pela coordenação médica.
    let des = format!("/v1/modelos-de-laudo/{id}/desativacao");
    let (st, _) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &des,
        json!({ "motivo": "Não uso mais este modelo" }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &des,
        json!({ "motivo": "Protocolo novo de TC" }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
}

#[tokio::test]
async fn uso_do_modelo_no_rascunho_grava_a_versao_e_so_o_medico_atribuido_usa() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb
        .ate_o_laudo("Antônio Carlos Ribeiro", "1954-06-15")
        .await;
    let m = criar_modelo(&amb, "medico.a", "pessoal", "Eco de rotina").await;
    let id = m["id"].as_str().unwrap();
    let uri = format!("/v1/laudos/{laudo}/usos-de-modelo");
    let chave = Uuid::now_v7();
    let corpo = json!({ "modelo_id": id, "numero": 1 });
    let (st, u) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &uri,
        corpo.clone(),
        Some(chave),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{u}");
    assert_eq!(
        (u["texto"].as_str(), u["laudo_versao"].as_i64()),
        (Some(TEXTO), Some(1))
    );
    let (st2, u2) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &uri,
        corpo.clone(),
        Some(chave),
    )
    .await;
    assert_eq!((st2, &u2), (StatusCode::CREATED, &u));
    assert_eq!(
        amb.contar("SELECT count(*) FROM laudos.uso_modelo").await,
        1
    );
    let (_, lido) = pedir(
        &amb,
        "medico.a",
        Method::GET,
        &format!("/v1/modelos-de-laudo/{id}"),
        Value::Null,
        None,
    )
    .await;
    assert_eq!(lido["usos"].as_i64(), Some(1));
    // Sem chave: 400; versão que não existe: 409.
    let s = amb.entrar("medico.a").await;
    let (st, _, _) = amb
        .pedir(amb.json(Method::POST, &uri, &s, None, corpo.clone()))
        .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &uri,
        json!({ "modelo_id": id, "numero": 9 }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("versao-de-modelo-indisponivel"))
    );
    // Outro médico (sem vínculo) e outros perfis não usam modelo no laudo de outro.
    let (st, _) = pedir(
        &amb,
        "cecilia.souto",
        Method::POST,
        &uri,
        corpo.clone(),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    for login in ["ana.souza", "ti.a", "carla.caixa", "patricia.lemos"] {
        let (st, _) = pedir(
            &amb,
            login,
            Method::POST,
            &uri,
            corpo.clone(),
            Some(Uuid::now_v7()),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    // Laudo assinado: o modelo não entra (correção só por retificação com motivo).
    pedir(
        &amb,
        "medico.a",
        Method::PUT,
        &format!("/v1/laudos/{laudo}/rascunho"),
        json!({ "texto": TEXTO, "versao_numero": 1 }),
        None,
    )
    .await;
    let (st, a) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/assinatura"),
        json!({ "pin": "4821" }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{a}");
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &uri,
        corpo,
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("laudo-sem-rascunho"))
    );
    assert!(sqlx::query("DELETE FROM laudos.uso_modelo")
        .execute(&amb.admin)
        .await
        .is_err());
}

#[tokio::test]
async fn frases_rapidas_pessoais_e_da_clinica_com_versao_por_substituicao() {
    let amb = Ambiente::novo().await;
    coordenacao(&amb, "cecilia.souto").await;
    let nova = |tipo: &str, atalho: &str, texto: &str| json!({ "tipo": tipo, "atalho": atalho, "texto": texto });
    let (st, f) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        "/v1/frases-de-laudo",
        nova(
            "pessoal",
            ".retorno",
            "Sugere-se controle conforme critério do médico assistente.",
        ),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, f["dono"].as_str()),
        (StatusCode::CREATED, Some("Dr. Paulo Medeiros")),
        "{f}"
    );
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        "/v1/frases-de-laudo",
        nova("pessoal", ".retorno", "Outra frase"),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("atalho-em-uso"))
    );
    for atalho in ["normal", ".N", ".a", ".com espaço"] {
        let (st, e) = pedir(
            &amb,
            "medico.a",
            Method::POST,
            "/v1/frases-de-laudo",
            nova("pessoal", atalho, "Frase válida"),
            Some(Uuid::now_v7()),
        )
        .await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some("atalho-invalido")),
            "{atalho}"
        );
    }
    // Frase da clínica: só a coordenação médica.
    let (st, _) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        "/v1/frases-de-laudo",
        nova(
            "clinica",
            ".normal",
            "Exame dentro dos limites da normalidade.",
        ),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, c) = pedir(
        &amb,
        "cecilia.souto",
        Method::POST,
        "/v1/frases-de-laudo",
        nova(
            "clinica",
            ".normal",
            "Exame dentro dos limites da normalidade.",
        ),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, c["dono"].as_str()),
        (StatusCode::CREATED, Some("Clínica"))
    );
    // Editar = nova frase que substitui a anterior, com motivo.
    let fid = f["id"].as_str().unwrap();
    let mut edicao = nova(
        "pessoal",
        ".retorno",
        "Sugere-se controle ecográfico conforme critério do médico assistente.",
    );
    edicao["substitui_id"] = json!(fid);
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        "/v1/frases-de-laudo",
        edicao.clone(),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("motivo-obrigatorio"))
    );
    edicao["motivo"] = json!("Deixa claro que é controle ecográfico");
    let (st, f2) = pedir(
        &amb,
        "cecilia.souto",
        Method::POST,
        "/v1/frases-de-laudo",
        edicao.clone(),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "frase pessoal de outro médico não é editável"
    );
    let (st, f2b) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        "/v1/frases-de-laudo",
        edicao,
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, f2b["substitui_codigo"].as_str()),
        (StatusCode::CREATED, f["codigo"].as_str()),
        "{f2} {f2b}"
    );
    let (_, lista) = pedir(
        &amb,
        "medico.a",
        Method::GET,
        "/v1/frases-de-laudo",
        Value::Null,
        None,
    )
    .await;
    let atalhos: Vec<&str> = lista
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["atalho"].as_str().unwrap())
        .collect();
    assert_eq!(atalhos, [".normal", ".retorno"]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM laudos.frase").await,
        3,
        "a versão anterior continua guardada"
    );
    // Outro médico vê só as da clínica; a frase pessoal não aparece.
    let (_, lista) = pedir(
        &amb,
        "marcos.freire",
        Method::GET,
        "/v1/frases-de-laudo",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(lista.as_array().unwrap().len(), 1);
    // Desativar com motivo.
    let f2id = f2b["id"].as_str().unwrap();
    let (st, _) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/frases-de-laudo/{f2id}/desativacao"),
        json!({ "motivo": "Não uso mais" }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(sqlx::query("DELETE FROM laudos.frase")
        .execute(&amb.admin)
        .await
        .is_err());
}

#[tokio::test]
async fn recepcao_ti_caixa_gerencia_e_outra_unidade_nao_acessam_modelos() {
    let amb = Ambiente::novo().await;
    medico_da_unidade_b(&amb).await;
    let m = criar_modelo(&amb, "medico.a", "clinica", "Doppler de carótidas").await;
    let id = m["id"].as_str().unwrap();
    for login in [
        "ana.souza",
        "ti.a",
        "carla.caixa",
        "financeiro.a",
        "patricia.lemos",
        "tv.a",
    ] {
        for uri in ["/v1/modelos-de-laudo", "/v1/frases-de-laudo"] {
            let (st, _) = pedir(&amb, login, Method::GET, uri, Value::Null, None).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "{login} {uri}");
        }
        let (st, _) = pedir(&amb, login, Method::POST, "/v1/modelos-de-laudo", json!({ "tipo": "pessoal", "nome": "Indevido", "exame": "Qualquer", "modalidade": "US", "texto": "x" }), Some(Uuid::now_v7())).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    // Médico de outra unidade não vê o modelo da clínica da unidade Centro.
    let (_, lista) = pedir(
        &amb,
        "medico.b",
        Method::GET,
        "/v1/modelos-de-laudo",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(lista.as_array().unwrap().len(), 0);
    let (st, _) = pedir(
        &amb,
        "medico.b",
        Method::GET,
        &format!("/v1/modelos-de-laudo/{id}"),
        Value::Null,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // O médico da clínica vê o da clínica, mas não o aprova.
    let (_, lista) = pedir(
        &amb,
        "marcos.freire",
        Method::GET,
        "/v1/modelos-de-laudo",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(lista[0]["pode_decidir"], false);
}
