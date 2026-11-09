//! Revisão por pares (Revisao-Pares.dc.html; DEC-2026-11-08-101 a 103): laudo assinado vai para
//! segunda leitura de outro médico (nunca o autor, conferido no banco), um revisor por laudo de cada
//! vez, parecer RADPEER com uma decisão só, leitura auditada pelo vínculo de revisão, pendência do
//! autor sem alterar o laudo e acesso indevido recusado. Rota real e PostgreSQL real (G4).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::{Ambiente, UNIDADE_B};
use serde_json::{json, Value};
use uuid::Uuid;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const TEXTO_LAUDO: &str = "Ecocardiograma: septo de 9 mm. Função sistólica preservada.";
const COMENTARIO: &str = "Derrame pericárdico pequeno na janela subcostal, não descrito.";

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

/// Laudo do Antônio escrito e assinado pelo Dr. Paulo (medico.a); Dra. Cecília vira coordenação
/// médica (médico com perfil de coordenação, DEC-2026-11-08-100).
async fn laudo_assinado(amb: &Ambiente) -> Uuid {
    sqlx::query("UPDATE identidade.usuario SET perfis = ARRAY['medico','gerente'] WHERE login = 'cecilia.souto'")
        .execute(&amb.admin)
        .await
        .unwrap();
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, "1954-06-15").await;
    let (st, c) = pedir(
        amb,
        "medico.a",
        Method::PUT,
        &format!("/v1/laudos/{laudo}/rascunho"),
        json!({ "texto": TEXTO_LAUDO, "versao_numero": 1 }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    let (st, c) = pedir(
        amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/assinatura"),
        json!({ "pin": "4821" }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    laudo
}

async fn designar(amb: &Ambiente, laudo: Uuid, revisor: &str, chave: Uuid) -> (StatusCode, Value) {
    let revisor_id = amb.s.usuarios[revisor];
    pedir(
        amb,
        "cecilia.souto",
        Method::POST,
        "/v1/revisoes-por-pares",
        json!({ "laudo_id": laudo, "revisor_id": revisor_id }),
        Some(chave),
    )
    .await
}

#[tokio::test]
async fn revisor_le_com_auditoria_registra_parecer_e_o_laudo_nao_muda() {
    let amb = Ambiente::novo().await;
    let laudo = laudo_assinado(&amb).await;
    // O autor nunca revisa o próprio laudo: nem pela rota, nem direto no banco.
    let (st, e) = designar(&amb, laudo, "medico.a", Uuid::now_v7()).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("revisor-e-autor")),
        "{e}"
    );
    let paulo = amb.s.usuarios["medico.a"];
    let insercao = sqlx::query("INSERT INTO laudos.revisao (id, codigo, laudo_id, revisor_id, modo, criado_por, origem, unidade_id, correlacao) VALUES ($1,'REV-X',$2,$3,'selecao',$3,'teste',$4,'t')")
        .bind(Uuid::now_v7()).bind(laudo).bind(paulo).bind(comum::UNIDADE_A).execute(&amb.admin).await;
    assert!(insercao.is_err(), "o banco aceitou o autor como revisor");

    let chave = Uuid::now_v7();
    let (st, d) = designar(&amb, laudo, "marcos.freire", chave).await;
    assert_eq!(
        (st, d["revisor"].as_str(), d["modo"].as_str()),
        (
            StatusCode::CREATED,
            Some("Dr. Marcos Freire"),
            Some("selecao")
        ),
        "{d}"
    );
    let (st2, d2) = designar(&amb, laudo, "marcos.freire", chave).await;
    assert_eq!(
        (st2, &d2),
        (StatusCode::CREATED, &d),
        "repetir a chave devolve a mesma designação"
    );
    assert_eq!(amb.contar("SELECT count(*) FROM laudos.revisao").await, 1);
    let id = d["id"].as_str().unwrap().to_owned();

    // Fila do revisor sem autor nem paciente; o caso traz os dois identificadores e o texto.
    let (_, fila) = pedir(
        &amb,
        "marcos.freire",
        Method::GET,
        "/v1/revisoes-por-pares/fila",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(fila[0]["situacao"], "designada");
    assert!(
        !fila.to_string().contains("Paulo") && !fila.to_string().contains(ANTONIO),
        "{fila}"
    );
    let (st, caso) = pedir(
        &amb,
        "marcos.freire",
        Method::GET,
        &format!("/v1/revisoes-por-pares/{id}"),
        Value::Null,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{caso}");
    assert_eq!(
        (
            caso["paciente"]["nome"].as_str(),
            caso["paciente"]["nascimento"].as_str()
        ),
        (Some(ANTONIO), Some("1954-06-15"))
    );
    assert_eq!(caso["texto"], TEXTO_LAUDO);
    assert!(caso["autor"].is_null(), "autor oculto até o envio");
    assert_eq!(
        amb.contar("SELECT count(*) FROM auditoria.acesso WHERE recurso = 'laudo' AND resultado = 'permitido' AND justificativa LIKE 'revisao_por_par REV-%'").await,
        1
    );
    // O autor não abre o caso do revisor.
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::GET,
        &format!("/v1/revisoes-por-pares/{id}"),
        Value::Null,
        None,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::FORBIDDEN, Some("revisao-de-outro-medico"))
    );

    // Parecer: classe válida e comentário na discrepância.
    let uri = format!("/v1/revisoes-por-pares/{id}/parecer");
    let (st, e) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &uri,
        json!({ "classe": "4", "comentario": "" }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("classe-invalida"))
    );
    let (st, e) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &uri,
        json!({ "classe": "2b", "comentario": "ver" }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("comentario-obrigatorio")
        )
    );
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &uri,
        json!({ "classe": "1", "comentario": "" }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::FORBIDDEN, Some("revisao-de-outro-medico"))
    );
    let chave = Uuid::now_v7();
    let (st, p) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &uri,
        json!({ "classe": "2b", "comentario": COMENTARIO }),
        Some(chave),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{p}");
    assert_eq!(
        (
            p["situacao"].as_str(),
            p["classe"].as_str(),
            p["autor"].as_str()
        ),
        (Some("concluida"), Some("2b"), Some("Dr. Paulo Medeiros"))
    );
    let (st2, p2) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &uri,
        json!({ "classe": "2b", "comentario": COMENTARIO }),
        Some(chave),
    )
    .await;
    assert_eq!((st2, &p2), (StatusCode::OK, &p));
    let (st, e) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &uri,
        json!({ "classe": "1", "comentario": "" }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("revisao-ja-concluida"))
    );
    assert!(
        sqlx::query("UPDATE laudos.revisao SET classe = '1'")
            .execute(&amb.admin)
            .await
            .is_err(),
        "parecer enviado é imutável"
    );
    assert!(sqlx::query("DELETE FROM laudos.revisao")
        .execute(&amb.admin)
        .await
        .is_err());
    // A revisão não altera o laudo, e o vínculo de revisão acaba com o envio.
    assert_eq!(amb.contar("SELECT count(*) FROM laudos.versao").await, 1);
    let (st, _) = pedir(
        &amb,
        "marcos.freire",
        Method::GET,
        &format!("/v1/laudos/{laudo}"),
        Value::Null,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // O autor vê o parecer (sem saber quem revisou) e a pendência.
    let (_, meus) = pedir(
        &amb,
        "medico.a",
        Method::GET,
        "/v1/revisoes-por-pares/meus-laudos",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(
        (
            meus["revisados"].as_i64(),
            meus["pendentes"].as_i64(),
            meus["concordancia_percentual"].as_i64()
        ),
        (Some(1), Some(1), Some(0))
    );
    assert_eq!(meus["revisoes"][0]["comentario"], COMENTARIO);
    assert_eq!(meus["revisoes"][0]["pendencia"], "aguarda_autor");
    assert!(
        !meus.to_string().contains("Marcos"),
        "o revisor não é revelado ao autor"
    );
    let pend = format!("/v1/revisoes-por-pares/{id}/pendencia");
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &pend,
        json!({ "decisao": "retificado" }),
        None,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("retificacao-nao-encontrada"))
    );
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &pend,
        json!({ "decisao": "mantido" }),
        None,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("motivo-obrigatorio"))
    );
    let (st, _) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &pend,
        json!({ "decisao": "mantido", "motivo": "Não sou o autor" }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // A retificação segue pelo fluxo do laudo (nova versão com motivo); depois o autor marca.
    let (st, r) = pedir(&amb, "medico.a", Method::POST, &format!("/v1/laudos/{laudo}/retificacao"), json!({ "texto": format!("{TEXTO_LAUDO} Pequeno derrame pericárdico."), "motivo": "Revisão por par: derrame não descrito" }), None).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    let (st, d) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &pend,
        json!({ "decisao": "retificado" }),
        None,
    )
    .await;
    assert_eq!(
        (st, d["pendencia"].as_str()),
        (StatusCode::OK, Some("retificado")),
        "{d}"
    );
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &pend,
        json!({ "decisao": "mantido", "motivo": "Mudei de ideia" }),
        None,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("pendencia-ja-decidida"))
    );

    // Painel da coordenação médica: números e discrepância, sem texto do laudo nem comentário.
    let (st, painel) = pedir(
        &amb,
        "cecilia.souto",
        Method::GET,
        "/v1/revisoes-por-pares/painel",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{painel}");
    let paulo = painel["por_medico"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["medico"] == "Dr. Paulo Medeiros")
        .unwrap()
        .clone();
    assert_eq!(
        (paulo["revisados"].as_i64(), paulo["discrepancias"].as_i64()),
        (Some(1), Some(1))
    );
    assert_eq!(painel["discrepancias"][0]["classe"], "2b");
    assert!(
        !painel.to_string().contains("Derrame") && !painel.to_string().contains("septo"),
        "{painel}"
    );
    assert_eq!(amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo IN ('revisao_designada','revisao_concluida','pendencia_de_revisao_decidida')").await, 3);
}

#[tokio::test]
async fn duas_designacoes_simultaneas_deixam_um_so_revisor() {
    let amb = Ambiente::novo().await;
    let laudo = laudo_assinado(&amb).await;
    let ((s1, _), (s2, _)) = tokio::join!(
        designar(&amb, laudo, "marcos.freire", Uuid::now_v7()),
        designar(&amb, laudo, "cecilia.souto", Uuid::now_v7())
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM laudos.revisao WHERE situacao = 'designada'")
            .await,
        1
    );
}

#[tokio::test]
async fn dois_pareceres_simultaneos_so_um_vale() {
    let amb = Ambiente::novo().await;
    let laudo = laudo_assinado(&amb).await;
    let (_, d) = designar(&amb, laudo, "marcos.freire", Uuid::now_v7()).await;
    let uri = format!(
        "/v1/revisoes-por-pares/{}/parecer",
        d["id"].as_str().unwrap()
    );
    let ((s1, _), (s2, _)) = tokio::join!(
        pedir(
            &amb,
            "marcos.freire",
            Method::POST,
            &uri,
            json!({ "classe": "1", "comentario": "" }),
            Some(Uuid::now_v7())
        ),
        pedir(
            &amb,
            "marcos.freire",
            Method::POST,
            &uri,
            json!({ "classe": "3a", "comentario": COMENTARIO }),
            Some(Uuid::now_v7())
        )
    );
    let mut st = [s1, s2];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::CONFLICT]);
    assert_eq!(
        amb.contar("SELECT count(*) FROM laudos.revisao_parecer")
            .await,
        1
    );
}

#[tokio::test]
async fn sorteio_nunca_designa_o_autor_respeita_limites_e_o_conflito_declarado() {
    let amb = Ambiente::novo().await;
    let laudo = laudo_assinado(&amb).await;
    let sortear = |q: i32, d: i32| json!({ "quantidade": q, "dias": d });
    for (q, d) in [(0, 30), (51, 30), (5, 0), (5, 91), (-1, 30)] {
        let (st, e) = pedir(
            &amb,
            "cecilia.souto",
            Method::POST,
            "/v1/revisoes-por-pares/sorteio",
            sortear(q, d),
            Some(Uuid::now_v7()),
        )
        .await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some("sorteio-invalido")),
            "{q} {d}"
        );
    }
    let chave = Uuid::now_v7();
    let (st, s) = pedir(
        &amb,
        "cecilia.souto",
        Method::POST,
        "/v1/revisoes-por-pares/sorteio",
        sortear(5, 30),
        Some(chave),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{s}");
    assert_eq!(
        (
            s["candidatos"].as_i64(),
            s["designadas"].as_array().unwrap().len()
        ),
        (Some(1), 1)
    );
    let primeiro = s["designadas"][0].clone();
    assert_ne!(
        primeiro["revisor"], "Dr. Paulo Medeiros",
        "o autor não revisa"
    );
    let (_, s2) = pedir(
        &amb,
        "cecilia.souto",
        Method::POST,
        "/v1/revisoes-por-pares/sorteio",
        sortear(5, 30),
        Some(chave),
    )
    .await;
    assert_eq!(s2, s, "repetir a chave não sorteia de novo");
    let (_, s3) = pedir(
        &amb,
        "cecilia.souto",
        Method::POST,
        "/v1/revisoes-por-pares/sorteio",
        sortear(5, 30),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        s3["candidatos"].as_i64(),
        Some(0),
        "laudo com revisor designado não volta ao sorteio"
    );

    // O revisor sorteado declara conflito: o caso volta e vai para outro, nunca o autor.
    let revisor_login = if primeiro["revisor"] == "Dr. Marcos Freire" {
        "marcos.freire"
    } else {
        "cecilia.souto"
    };
    let id = primeiro["id"].as_str().unwrap();
    let (st, _) = pedir(
        &amb,
        revisor_login,
        Method::POST,
        &format!("/v1/revisoes-por-pares/{id}/conflito"),
        json!({ "motivo": "" }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, c) = pedir(
        &amb,
        revisor_login,
        Method::POST,
        &format!("/v1/revisoes-por-pares/{id}/conflito"),
        json!({ "motivo": "Atendi este paciente no consultório" }),
        None,
    )
    .await;
    assert_eq!(
        (st, c["situacao"].as_str()),
        (StatusCode::OK, Some("conflito_declarado")),
        "{c}"
    );
    let (st, e) = pedir(
        &amb,
        revisor_login,
        Method::GET,
        &format!("/v1/revisoes-por-pares/{id}"),
        Value::Null,
        None,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("revisao-encerrada"))
    );
    let (st, e) = designar(&amb, laudo, revisor_login, Uuid::now_v7()).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::CONFLICT, Some("revisor-declarou-conflito"))
    );
    let (_, painel) = pedir(
        &amb,
        "cecilia.souto",
        Method::GET,
        "/v1/revisoes-por-pares/painel",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(painel["conflitos"].as_array().unwrap().len(), 1);
    let (_, s4) = pedir(
        &amb,
        "cecilia.souto",
        Method::POST,
        "/v1/revisoes-por-pares/sorteio",
        sortear(5, 30),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(s4["designadas"].as_array().unwrap().len(), 1, "{s4}");
    let novo = s4["designadas"][0]["revisor"].as_str().unwrap();
    assert!(
        novo != "Dr. Paulo Medeiros" && novo != primeiro["revisor"].as_str().unwrap(),
        "{novo}"
    );
    let (_, painel) = pedir(
        &amb,
        "cecilia.souto",
        Method::GET,
        "/v1/revisoes-por-pares/painel",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(
        painel["conflitos"].as_array().unwrap().len(),
        0,
        "conflito redesignado sai da lista"
    );
}

#[tokio::test]
async fn recepcao_ti_caixa_gerencia_autor_e_outra_unidade_nao_acessam() {
    let amb = Ambiente::novo().await;
    let laudo = laudo_assinado(&amb).await;
    sqlx::query("INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao) VALUES ($1,'USR-medico.b','medico.b','Dra. Norte',ARRAY['medico'],'seed',$2,'seed')")
        .bind(Uuid::now_v7()).bind(UNIDADE_B).execute(&amb.admin).await.unwrap();
    let (_, d) = designar(&amb, laudo, "marcos.freire", Uuid::now_v7()).await;
    let id = d["id"].as_str().unwrap();
    for login in [
        "ana.souza",
        "ti.a",
        "carla.caixa",
        "financeiro.a",
        "tv.a",
        "rita.faturamento",
    ] {
        for uri in [
            "/v1/revisoes-por-pares/fila",
            "/v1/revisoes-por-pares/painel",
            "/v1/revisoes-por-pares/meus-laudos",
            &format!("/v1/revisoes-por-pares/{id}"),
        ] {
            let (st, _) = pedir(&amb, login, Method::GET, uri, Value::Null, None).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "{login} {uri}");
        }
        let (st, _) = pedir(
            &amb,
            login,
            Method::POST,
            "/v1/revisoes-por-pares/sorteio",
            json!({ "quantidade": 1, "dias": 30 }),
            Some(Uuid::now_v7()),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    // Gerência sem perfil médico e médico sem coordenação não sorteiam nem veem o painel.
    for login in ["patricia.lemos", "medico.a"] {
        let (st, _) = pedir(
            &amb,
            login,
            Method::GET,
            "/v1/revisoes-por-pares/painel",
            Value::Null,
            None,
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
        let (st, _) = pedir(
            &amb,
            login,
            Method::POST,
            "/v1/revisoes-por-pares",
            json!({ "laudo_id": laudo, "revisor_id": amb.s.usuarios["marcos.freire"] }),
            Some(Uuid::now_v7()),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    // Médico de outra unidade: a revisão não existe para ele e não pode ser revisor aqui.
    let (st, _) = pedir(
        &amb,
        "medico.b",
        Method::GET,
        &format!("/v1/revisoes-por-pares/{id}"),
        Value::Null,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = pedir(
        &amb,
        "medico.b",
        Method::POST,
        &format!("/v1/revisoes-por-pares/{id}/parecer"),
        json!({ "classe": "1", "comentario": "" }),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let medico_b: Uuid =
        sqlx::query_scalar("SELECT id FROM identidade.usuario WHERE login = 'medico.b'")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    let (_, maria) = amb.ate_o_laudo("Maria Helena Duarte", "1968-03-12").await;
    for (laudo_id, revisor, codigo) in [
        (laudo, medico_b, "revisor-invalido"),
        (laudo, amb.s.usuarios["ana.souza"], "revisor-invalido"),
        (maria, amb.s.usuarios["marcos.freire"], "laudo-nao-assinado"),
        (
            Uuid::now_v7(),
            amb.s.usuarios["marcos.freire"],
            "laudo-nao-encontrado",
        ),
    ] {
        let (_, e) = pedir(
            &amb,
            "cecilia.souto",
            Method::POST,
            "/v1/revisoes-por-pares",
            json!({ "laudo_id": laudo_id, "revisor_id": revisor }),
            Some(Uuid::now_v7()),
        )
        .await;
        assert_eq!(e["codigo"].as_str(), Some(codigo), "{e}");
    }
    // Sem a chave de idempotência, nada é designado.
    let s = amb.entrar("cecilia.souto").await;
    let (st, _, _) = amb
        .pedir(amb.json(
            Method::POST,
            "/v1/revisoes-por-pares/sorteio",
            &s,
            None,
            json!({ "quantidade": 1, "dias": 30 }),
        ))
        .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}
