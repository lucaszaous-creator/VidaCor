//! Lista de pacientes e possíveis duplicados (Pacientes-Lista.dc.html): busca, recortes e paginação do
//! servidor, situação calculada no banco, decisão sobre duplicado única por par (2 requisições
//! simultâneas), repetição sem duplicar, registro imutável e acesso indevido. Rota real e PostgreSQL
//! real (G4). Frases FC-PAC-01 a 06 (docs/27, proposta aguardando aprovação).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

async fn lista(amb: &Ambiente, login: &str, consulta: &str) -> (StatusCode, Value) {
    let s = amb.entrar(login).await;
    let (st, _, v) = amb
        .pedir(amb.get(&format!("/v1/pacientes/lista{consulta}"), Some(&s)))
        .await;
    (st, v)
}

fn nomes(v: &Value) -> Vec<String> {
    v["itens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["nome"].as_str().unwrap().to_owned())
        .collect()
}

/// Segundo cadastro da Maria Helena com nascimento digitado errado e o mesmo celular (como vem da migração).
async fn cadastro_duplicado(amb: &Ambiente) -> Uuid {
    sqlx::query("UPDATE identidade.paciente SET celular = '48998124410' WHERE id = $1")
        .bind(amb.maria)
        .execute(&amb.admin)
        .await
        .unwrap();
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO identidade.paciente (id, codigo, nome, nascimento, sexo, celular, origem, unidade_id, correlacao)
                 SELECT $1, 'PAC-2026-000790', 'Maria Helena Duarte', DATE '1968-03-21', 'F', '48998124410', 'seed', unidade_id, 'seed'
                   FROM identidade.paciente WHERE id = $2")
        .bind(id)
        .bind(amb.maria)
        .execute(&amb.admin)
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn lista_busca_recortes_e_paginacao_vem_do_servidor() {
    let amb = Ambiente::novo().await;
    // FC-PAC-01: a lista mostra os cadastros da unidade, em ordem de nome, com o total.
    let (st, v) = lista(&amb, "ana.souza", "").await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["cadastrados"], 5);
    assert_eq!(v["recortes"]["todos"], 5);
    assert_eq!(v["por_pagina"], 12);
    assert_eq!(v["paginas"], 1);
    assert_eq!(
        nomes(&v),
        [
            "Antônio Carlos Ribeiro",
            "Juliana Sampaio Rocha",
            "Maria Helena Duarte",
            "Sônia Maria Castro",
            "Vera Lúcia Antunes"
        ]
    );
    // Cadastro da semente sem CPF, celular, mãe nem alergias: incompleto, e a tela sabe o que falta.
    assert_eq!(v["recortes"]["incompleto"], 5);
    let sonia = &v["itens"][3];
    assert_eq!(sonia["situacao"], "cadastro_incompleto");
    assert_eq!(
        sonia["falta_no_cadastro"],
        json!(["cpf", "celular", "nome_mae", "alergias"])
    );
    assert_eq!(sonia["convenio"], "Particular");
    assert!(sonia["proximo_exame"].is_null());

    // FC-PAC-02: busca por nome, nascimento, celular ou CPF e prontuário.
    sqlx::query(
        "UPDATE identidade.paciente SET cpf = '52998224725', celular = '48996401187' WHERE id = $1",
    )
    .bind(amb.antonio)
    .execute(&amb.admin)
    .await
    .unwrap();
    for consulta in [
        "?q=antonio%20carlos",
        "?q=15/06/1954",
        "?q=529.982.247-25",
        "?q=(48)%2099640-1187",
        "?q=PAC-2026-000001",
    ] {
        let (st, v) = lista(&amb, "ana.souza", consulta).await;
        assert_eq!(st, StatusCode::OK, "{consulta}: {v}");
        // "antonio carlos" sem acento acha "Antônio Carlos" (a recepção digita sem acento).
        assert_eq!(nomes(&v), ["Antônio Carlos Ribeiro"], "{consulta}");
        assert_eq!(v["itens"][0]["cpf_mascarado"], "***.***.***-25");
    }
    let (_, v) = lista(&amb, "ana.souza", "?q=Ant%C3%B4nio").await;
    assert_eq!(nomes(&v), ["Antônio Carlos Ribeiro"]);

    // FC-PAC-03: "na clínica hoje" vem do check-in registrado, não do cliente.
    let (_, v) = lista(&amb, "ana.souza", "?recorte=hoje").await;
    assert_eq!(v["total"], 0);
    amb.fazer_checkin("Maria Helena Duarte", "1968-03-12").await;
    let (_, v) = lista(&amb, "ana.souza", "?recorte=hoje").await;
    assert_eq!(nomes(&v), ["Maria Helena Duarte"]);
    assert_eq!(v["itens"][0]["situacao"], "na_clinica");
    assert_eq!(v["recortes"]["hoje"], 1);

    // Limites: recorte desconhecido, página zero ou enorme e busca curta são recusados.
    for consulta in [
        "?recorte=todos-os-outros",
        "?pagina=0",
        "?pagina=-3",
        "?pagina=10001",
        "?q=ab",
    ] {
        let (st, _) = lista(&amb, "ana.souza", consulta).await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{consulta}");
    }
    // Página além da última vem vazia, sem erro e com o total certo.
    let (st, v) = lista(&amb, "ana.souza", "?pagina=3").await;
    assert_eq!(
        (st, v["itens"].as_array().unwrap().len()),
        (StatusCode::OK, 0)
    );
    assert_eq!(v["total"], 5);
}

#[tokio::test]
async fn paginacao_com_mais_de_uma_pagina() {
    let amb = Ambiente::novo().await;
    for n in 0..20 {
        sqlx::query("INSERT INTO identidade.paciente (id, codigo, nome, nascimento, sexo, origem, unidade_id, correlacao)
                     SELECT $1, $2, $3, DATE '1980-01-01' + $4::int, 'F', 'seed', unidade_id, 'seed' FROM identidade.paciente WHERE id = $5")
            .bind(Uuid::now_v7())
            .bind(format!("PAC-2025-{:06}", n + 1))
            .bind(format!("Zélia Ficticia {n:02}"))
            .bind(n)
            .bind(amb.maria)
            .execute(&amb.admin)
            .await
            .unwrap();
    }
    let (_, p1) = lista(&amb, "ana.souza", "?pagina=1").await;
    let (_, p2) = lista(&amb, "ana.souza", "?pagina=2").await;
    assert_eq!(
        (p1["total"].as_i64(), p1["paginas"].as_i64()),
        (Some(25), Some(3))
    );
    assert_eq!(p1["itens"].as_array().unwrap().len(), 12);
    assert_eq!(p2["itens"].as_array().unwrap().len(), 12);
    let mut todos = nomes(&p1);
    todos.extend(nomes(&p2));
    let (_, p3) = lista(&amb, "ana.souza", "?pagina=3").await;
    todos.extend(nomes(&p3));
    let mut unicos = todos.clone();
    unicos.sort();
    unicos.dedup();
    assert_eq!(
        unicos.len(),
        25,
        "nenhuma linha repetida ou perdida entre páginas"
    );
}

#[tokio::test]
async fn acesso_indevido_e_outra_unidade() {
    let amb = Ambiente::novo().await;
    // FC-PAC-04: caixa, financeiro, TI, faturamento e TV não abrem a lista (mínimo necessário).
    for login in [
        "carla.caixa",
        "financeiro.a",
        "ti.a",
        "rita.faturamento",
        "tv.a",
        "estoque.a",
    ] {
        let (st, _) = lista(&amb, login, "").await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    // Técnico e médico veem a lista (abrem a ficha), sem dado clínico na linha.
    for login in ["bruno.sales", "medico.a", "patricia.lemos"] {
        let (st, v) = lista(&amb, login, "").await;
        assert_eq!(st, StatusCode::OK, "{login}");
        assert_eq!(v["cadastrados"], 5);
    }
    // A recepção da outra unidade não vê os pacientes do Centro, nem contados.
    let (st, v) = lista(&amb, "recepcao.b", "?q=Maria").await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        (v["cadastrados"].as_i64(), v["total"].as_i64()),
        (Some(0), Some(0))
    );
    // Sem sessão: 401.
    let (st, _, _) = amb.pedir(amb.get("/v1/pacientes/lista", None)).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn duplicado_decidido_uma_vez_por_par_e_sem_apagar() {
    let amb = Ambiente::novo().await;
    let outra = cadastro_duplicado(&amb).await;
    let ana = amb.entrar("ana.souza").await;
    // FC-PAC-05: o par aparece com o motivo, e a linha da lista avisa.
    let (st, _, pares) = amb
        .pedir(amb.get("/v1/pacientes/pares-duplicados", Some(&ana)))
        .await;
    assert_eq!(st, StatusCode::OK, "{pares}");
    assert_eq!(pares.as_array().unwrap().len(), 1);
    assert_eq!(
        pares[0]["motivo"],
        "Mesmo nome e mesmo celular, nascimento diferente"
    );
    assert_eq!(pares[0]["situacao"], "aberto");
    let (_, v) = lista(&amb, "ana.souza", "?q=Maria%20Helena").await;
    assert_eq!(v["recortes"]["duplicados"], 1);

    // Duas recepções decidindo o mesmo par ao mesmo tempo: só uma decisão vale (G2).
    let pedido =
        |decisao: &str| json!({ "paciente_a": amb.maria, "paciente_b": outra, "decisao": decisao });
    let uri = "/v1/pacientes/pares-duplicados";
    let (a, b) = tokio::join!(
        amb.pedir(amb.json(
            Method::POST,
            uri,
            &ana,
            Some(Uuid::now_v7()),
            pedido("juncao_pedida")
        )),
        amb.pedir(amb.json(
            Method::POST,
            uri,
            &ana,
            Some(Uuid::now_v7()),
            pedido("juncao_pedida")
        ))
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM identidade.par_duplicado")
            .await,
        1
    );
    // Junção pedida: o par continua na lista, aguardando outra pessoa; os cadastros seguem separados.
    let (_, _, pares) = amb
        .pedir(amb.get("/v1/pacientes/pares-duplicados", Some(&ana)))
        .await;
    assert_eq!(pares[0]["situacao"], "juncao_pedida");
    assert_eq!(pares[0]["decidido_por"], "Ana Souza");
    assert_eq!(
        amb.contar("SELECT count(*) FROM identidade.paciente").await,
        6
    );

    // Mudar de ideia sem motivo: recusado; com motivo: nova decisão, a anterior fica.
    let chave = Uuid::now_v7();
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            uri,
            &ana,
            Some(chave),
            pedido("pessoas_diferentes"),
        ))
        .await;
    assert_eq!(st, StatusCode::CONFLICT);
    let mut com_motivo = pedido("pessoas_diferentes");
    com_motivo["motivo"] = json!("Conferido o documento: são mãe e filha com o mesmo nome.");
    let chave = Uuid::now_v7();
    let (st, _, r1) = amb
        .pedir(amb.json(Method::POST, uri, &ana, Some(chave), com_motivo.clone()))
        .await;
    assert_eq!(st, StatusCode::CREATED, "{r1}");
    // Repetir com a mesma chave devolve a mesma resposta e não grava de novo (G3).
    let (st, _, r2) = amb
        .pedir(amb.json(Method::POST, uri, &ana, Some(chave), com_motivo))
        .await;
    assert_eq!((st, &r2["id"]), (StatusCode::CREATED, &r1["id"]));
    assert_eq!(
        amb.contar("SELECT count(*) FROM identidade.par_duplicado")
            .await,
        2
    );
    // Pessoas diferentes: o par sai da lista e da contagem.
    let (_, _, pares) = amb
        .pedir(amb.get("/v1/pacientes/pares-duplicados", Some(&ana)))
        .await;
    assert_eq!(pares.as_array().unwrap().len(), 0);
    let (_, v) = lista(&amb, "ana.souza", "?q=Maria%20Helena").await;
    assert_eq!(v["recortes"]["duplicados"], 0);
    assert!(v["itens"]
        .as_array()
        .unwrap()
        .iter()
        .all(|i| i["situacao"] != "possivel_duplicado"));
    // A decisão não se altera nem se apaga, nem pelo administrador do banco.
    assert!(
        sqlx::query("UPDATE identidade.par_duplicado SET decisao = 'juncao_pedida'")
            .execute(&amb.admin)
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM identidade.par_duplicado")
        .execute(&amb.admin)
        .await
        .is_err());
    // A decisão entra no lastro.
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'par_duplicado_decidido'")
            .await,
        2
    );
}

#[tokio::test]
async fn decisao_recusa_par_invalido_e_perfil_errado() {
    let amb = Ambiente::novo().await;
    let outra = cadastro_duplicado(&amb).await;
    let uri = "/v1/pacientes/pares-duplicados";
    // FC-PAC-06: técnico e médico veem a lista, mas não decidem nem leem os pares.
    for login in ["bruno.sales", "medico.a", "carla.caixa"] {
        let s = amb.entrar(login).await;
        let (st, _, _) = amb
            .pedir(amb.json(Method::POST, uri, &s, Some(Uuid::now_v7()), json!({ "paciente_a": amb.maria, "paciente_b": outra, "decisao": "pessoas_diferentes" })))
            .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
        let (st, _, _) = amb.pedir(amb.get(uri, Some(&s))).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    let ana = amb.entrar("ana.souza").await;
    let casos = [
        (
            json!({ "paciente_a": amb.maria, "paciente_b": outra, "decisao": "juntar_agora" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({ "paciente_a": amb.maria, "paciente_b": amb.maria, "decisao": "pessoas_diferentes" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        // Cadastros de nomes diferentes não são duplicado: ninguém marca par qualquer pela rota.
        (
            json!({ "paciente_a": amb.maria, "paciente_b": amb.antonio, "decisao": "pessoas_diferentes" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({ "paciente_a": amb.maria, "paciente_b": Uuid::now_v7(), "decisao": "pessoas_diferentes" }),
            StatusCode::NOT_FOUND,
        ),
        (
            json!({ "paciente_a": amb.maria, "paciente_b": outra, "decisao": "pessoas_diferentes", "motivo": "ab" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ];
    for (corpo, esperado) in casos {
        let (st, _, v) = amb
            .pedir(amb.json(Method::POST, uri, &ana, Some(Uuid::now_v7()), corpo.clone()))
            .await;
        assert_eq!(st, esperado, "{corpo}: {v}");
    }
    // A recepção da outra unidade não decide sobre cadastros do Centro (não os enxerga).
    let rb = amb.entrar("recepcao.b").await;
    let (st, _, _) = amb
        .pedir(amb.json(Method::POST, uri, &rb, Some(Uuid::now_v7()), json!({ "paciente_a": amb.maria, "paciente_b": outra, "decisao": "pessoas_diferentes" })))
        .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert_eq!(
        amb.contar("SELECT count(*) FROM identidade.par_duplicado")
            .await,
        0
    );
}

#[tokio::test]
async fn duas_mudancas_de_decisao_ao_mesmo_tempo_so_uma_vale() {
    let amb = Ambiente::novo().await;
    let outra = cadastro_duplicado(&amb).await;
    let ana = amb.entrar("ana.souza").await;
    let uri = "/v1/pacientes/pares-duplicados";
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            uri,
            &ana,
            Some(Uuid::now_v7()),
            json!({ "paciente_a": outra, "paciente_b": amb.maria, "decisao": "juncao_pedida" }),
        ))
        .await;
    assert_eq!(st, StatusCode::CREATED);
    let mudar = json!({ "paciente_a": amb.maria, "paciente_b": outra, "decisao": "pessoas_diferentes", "motivo": "Conferido o documento com foto das duas." });
    let g = amb.entrar("patricia.lemos").await;
    let (a, b) = tokio::join!(
        amb.pedir(amb.json(Method::POST, uri, &ana, Some(Uuid::now_v7()), mudar.clone())),
        amb.pedir(amb.json(Method::POST, uri, &g, Some(Uuid::now_v7()), mudar.clone()))
    );
    let mut st = [a.0, b.0];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM identidade.par_duplicado")
            .await,
        2
    );
}

#[tokio::test]
async fn menu_de_contexto_do_paciente_vem_do_servidor_pelo_perfil() {
    let amb = Ambiente::novo().await;
    let uri = format!("/v1/menu/paciente/{}", amb.maria);
    let ids = |v: &Value| -> Vec<String> {
        v["acoes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["id"].as_str().unwrap().to_owned())
            .collect()
    };
    let ana = amb.entrar("ana.souza").await;
    let (st, _, v) = amb.pedir(amb.get(&uri, Some(&ana))).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(ids(&v), ["abrir", "agendar", "copiar"]);
    assert_eq!(v["cabecalho"]["titulo"], "Maria Helena Duarte");
    assert_eq!(
        v["cabecalho"]["subtitulo"],
        "12/03/1968, prontuário PAC-2026-000002"
    );
    // O médico abre a ficha, mas agendar não aparece (perfil nunca terá).
    let m = amb.entrar("medico.a").await;
    let (_, _, v) = amb.pedir(amb.get(&uri, Some(&m))).await;
    assert_eq!(ids(&v), ["abrir", "copiar"]);
    let c = amb.entrar("carla.caixa").await;
    assert_eq!(
        amb.pedir(amb.get(&uri, Some(&c))).await.0,
        StatusCode::FORBIDDEN
    );
    let rb = amb.entrar("recepcao.b").await;
    assert_eq!(
        amb.pedir(amb.get(&uri, Some(&rb))).await.0,
        StatusCode::NOT_FOUND
    );
}
