//! Laudo estruturado com classificações (docs/22 E2, docs/25 § 3.3, DEC-2026-11-08-60). Rota real e
//! PostgreSQL real (G4); só o provedor de assinatura é simulado (externo).
//!
//! As frases abaixo ("quando..., então...") ainda não têm ID FC-* aprovado pelo dono do produto e os
//! casos das tabelas aguardam conferência por médico: estão aqui para provar o comportamento
//! implementado, não como aceite.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::{Method, StatusCode};
use comum::Ambiente;
use serde_json::{json, Value};
use uuid::Uuid;

const ANTONIO: &str = "Antônio Carlos Ribeiro";
const NASC: &str = "1954-06-15";

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

/// N1 do mockup: sólido, hipoecoico, 18 mm (TR4, 4 pontos, PAAF a partir de 15 mm).
fn nodulo_tr4() -> Value {
    json!({
        "sistema": "tirads",
        "localizacao": "Lobo direito, terço médio",
        "maior_diametro_mm": 18.0,
        "tirads": {
            "composicao": "solida_ou_quase_solida",
            "ecogenicidade": "hipoecoica",
            "forma": "mais_largo_que_alto",
            "margem": "lisa",
            "focos": []
        }
    })
}

async fn criar(amb: &Ambiente, laudo: Uuid, corpo: Value) -> (StatusCode, Value) {
    pedir(
        amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/achados"),
        corpo,
        Some(Uuid::now_v7()),
    )
    .await
}

async fn assinar(amb: &Ambiente, laudo: Uuid) -> Value {
    let (st, c) = pedir(
        amb,
        "medico.a",
        Method::PUT,
        &format!("/v1/laudos/{laudo}/rascunho"),
        json!({ "texto": "Tireoide com nódulo descrito abaixo.", "versao_numero": 1 }),
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
    c
}

async fn achados_no_banco(amb: &Ambiente) -> i64 {
    amb.contar("SELECT count(*) FROM laudos.achado").await
}

/// Quando o médico atribuído registra um nódulo, então o servidor calcula pontos, categoria e
/// conduta, grava a versão da regra e deixa lastro sem conteúdo clínico.
#[tokio::test]
async fn achado_tirads_calculado_pelo_servidor_com_versao_da_regra() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let (st, a) = criar(&amb, laudo, nodulo_tr4()).await;
    assert_eq!(st, StatusCode::CREATED, "{a}");
    assert_eq!(a["rotulo"], "N1");
    assert_eq!(a["pontos"], 4);
    assert_eq!(a["categoria"], "TR4");
    assert_eq!(a["categoria_nome"], "moderadamente suspeito");
    assert_eq!(a["conduta"], "PAAF indicada (a partir de 15 mm)");
    assert_eq!(a["regra_versao"], "TIRADS-ACR-2017.1");
    assert_eq!(a["versao_numero"], 1);
    assert_eq!(a["maior_diametro_mm"], 18.0);
    assert_eq!(a["tirads"]["focos"], json!(["nenhum_ou_cauda_de_cometa"]));
    assert!(a["codigo"].as_str().unwrap().starts_with("ACH-"), "{a}");

    let (st, lista) = pedir(
        &amb,
        "medico.a",
        Method::GET,
        &format!("/v1/laudos/{laudo}/achados"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{lista}");
    assert_eq!(lista["editavel"], true);
    assert_eq!(lista["achados"].as_array().unwrap().len(), 1);
    assert!(lista["texto_achados"]
        .as_str()
        .unwrap()
        .starts_with("N1: no lobo direito"));
    assert_eq!(
        lista["texto_conclusao"],
        "N1: ACR TI-RADS TR4, moderadamente suspeito; PAAF indicada (a partir de 15 mm)."
    );

    let (regra, dados): (String, Value) = sqlx::query_as(
        "SELECT a.regra_versao, e.dados FROM laudos.achado a JOIN lastro.evento e ON e.registro_id = a.id AND e.tipo = 'achado_registrado'",
    )
    .fetch_one(&amb.admin)
    .await
    .unwrap();
    assert_eq!(regra, "TIRADS-ACR-2017.1");
    assert_eq!(dados["regra_versao"], "TIRADS-ACR-2017.1");
    assert!(
        dados.get("categoria").is_none(),
        "lastro sem conteúdo clínico: {dados}"
    );
    // A leitura dos achados é auditada como a do laudo.
    assert_eq!(
        amb.contar("SELECT count(*) FROM auditoria.acesso WHERE resultado = 'permitido'")
            .await,
        1
    );
}

/// Quando a tela manda categoria, pontos e conduta, então o servidor ignora e calcula pelos descritores (G1).
#[tokio::test]
async fn categoria_enviada_pela_tela_e_ignorada() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let mut corpo = json!({
        "sistema": "tirads",
        "localizacao": "Istmo",
        "maior_diametro_mm": 12.0,
        "tirads": {
            "composicao": "solida_ou_quase_solida",
            "ecogenicidade": "muito_hipoecoica",
            "forma": "mais_alto_que_largo",
            "margem": "extensao_extratireoidiana",
            "focos": ["focos_puntiformes"]
        }
    });
    corpo["categoria"] = json!("TR1");
    corpo["pontos"] = json!(0);
    corpo["conduta"] = json!("sem seguimento");
    corpo["regra_versao"] = json!("TIRADS-INVENTADA.9");
    let (st, a) = criar(&amb, laudo, corpo).await;
    assert_eq!(st, StatusCode::CREATED, "{a}");
    assert_eq!(
        (a["categoria"].as_str(), a["pontos"].as_i64()),
        (Some("TR5"), Some(14))
    );
    assert_eq!(a["regra_versao"], "TIRADS-ACR-2017.1");
    let (cat, pts): (String, i32) = sqlx::query_as("SELECT categoria, pontos FROM laudos.achado")
        .fetch_one(&amb.admin)
        .await
        .unwrap();
    assert_eq!((cat.as_str(), pts), ("TR5", 14));
}

/// Quando o médico escolhe uma categoria BI-RADS incoerente com a massa, então o servidor recusa.
#[tokio::test]
async fn birads_confere_coerencia_da_categoria() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let massa = json!({
        "forma": "irregular", "orientacao": "nao_paralela", "margens": ["espiculada"],
        "padrao_ecogenico": "hipoecoico", "posterior": "sombra"
    });
    let corpo = |categoria: &str, biopsia: bool| {
        json!({
            "sistema": "birads",
            "localizacao": "Quadrante superolateral, 2 horas",
            "maior_diametro_mm": 14.0,
            "birads": { "lado": "esquerda", "categoria_atribuida": categoria, "biopsia_maligna_confirmada": biopsia, "massa": massa }
        })
    };
    let (st, e) = criar(&amb, laudo, corpo("6", false)).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{e}");
    assert_eq!(e["codigo"], "birads-6-sem-biopsia");
    let (st, e) = criar(&amb, laudo, corpo("3", false)).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{e}");
    assert_eq!(e["codigo"], "categoria-incoerente");
    assert_eq!(achados_no_banco(&amb).await, 0, "recusa não grava nada");

    let (st, a) = criar(&amb, laudo, corpo("4C", false)).await;
    assert_eq!(st, StatusCode::CREATED, "{a}");
    assert_eq!(a["categoria"], "4C");
    assert_eq!(a["pontos"], Value::Null);
    assert_eq!(a["regra_versao"], "BIRADS-ACR-2013-US.1");
    assert!(a["conduta"]
        .as_str()
        .unwrap()
        .starts_with("diagnóstico tecidual (biópsia)"));
    assert_eq!(a["birads"]["massa"]["margens"], json!(["espiculada"]));
}

/// Quando o laudo está assinado, então não recebe nem perde achado; a retificação leva os achados
/// para a nova versão e os originais continuam imutáveis, até para o dono do banco.
#[tokio::test]
async fn laudo_assinado_recusa_achado_e_retificacao_leva_os_achados() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let (_, a) = criar(&amb, laudo, nodulo_tr4()).await;
    let achado = a["id"].as_str().unwrap().to_owned();
    let assinado = assinar(&amb, laudo).await;

    // O hash assinado inclui os achados (não é o hash só do texto).
    let so_texto = vidacor_api::laudos::hash_documento(
        assinado["codigo"].as_str().unwrap(),
        1,
        ANTONIO,
        NASC,
        assinado["exame"].as_str().unwrap(),
        "Tireoide com nódulo descrito abaixo.",
    );
    assert_ne!(
        assinado["versoes"][0]["hash_documento"].as_str().unwrap(),
        so_texto
    );

    let (st, e) = criar(&amb, laudo, nodulo_tr4()).await;
    assert_eq!(st, StatusCode::CONFLICT, "{e}");
    assert_eq!(e["codigo"], "laudo-assinado");
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/achados/{achado}/remocao"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{e}");
    let (st, lista) = pedir(
        &amb,
        "medico.a",
        Method::GET,
        &format!("/v1/laudos/{laudo}/achados"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        (
            lista["editavel"].as_bool(),
            lista["achados"].as_array().unwrap().len()
        ),
        (Some(false), 1)
    );

    // Nem o dono do banco altera ou apaga achado de versão assinada (gatilho).
    for sql in [
        "UPDATE laudos.achado SET categoria = 'TR1'",
        "UPDATE laudos.achado SET situacao = 'removido', removido_em = now(), removido_por = criado_por",
        "DELETE FROM laudos.achado",
    ] {
        assert!(sqlx::query(sql).execute(&amb.admin).await.is_err(), "{sql}");
    }

    let (st, r) = pedir(&amb, "medico.a", Method::POST, &format!("/v1/laudos/{laudo}/retificacao"),
        json!({ "texto": "Tireoide com nódulo descrito abaixo (medida revisada).", "motivo": "medida do nódulo revisada" }), None).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    let (_, lista) = pedir(
        &amb,
        "medico.a",
        Method::GET,
        &format!("/v1/laudos/{laudo}/achados"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(lista["versao_numero"], 2);
    assert_eq!(lista["editavel"], true);
    assert_eq!(lista["achados"][0]["categoria"], "TR4");
    assert_ne!(lista["achados"][0]["id"].as_str().unwrap(), achado);
    let copiado_de: Option<Uuid> =
        sqlx::query_scalar("SELECT copiado_de FROM laudos.achado WHERE versao_numero = 2")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert_eq!(copiado_de.unwrap().to_string(), achado);
    // Na versão em retificação o achado pode ser removido; o da versão 1 continua lá, ativo.
    let novo = lista["achados"][0]["id"].as_str().unwrap();
    let (st, l2) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/achados/{novo}/remocao"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{l2}");
    assert_eq!(
        amb.contar(
            "SELECT count(*) FROM laudos.achado WHERE versao_numero = 1 AND situacao = 'ativo'"
        )
        .await,
        1
    );
}

/// Quando outro perfil ou um médico sem vínculo tenta ler ou registrar achado, então é recusado e
/// nada é gravado; com acesso de emergência, o médico lê mas não escreve no laudo de outro.
#[tokio::test]
async fn acesso_indevido_recusado() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    criar(&amb, laudo, nodulo_tr4()).await;
    let uri = format!("/v1/laudos/{laudo}/achados");

    for login in [
        "ana.souza",
        "bruno.sales",
        "patricia.lemos",
        "ti.a",
        "carla.caixa",
    ] {
        let (st, e) = pedir(
            &amb,
            login,
            Method::POST,
            &uri,
            nodulo_tr4(),
            Some(Uuid::now_v7()),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}: {e}");
        let (st, _) = pedir(&amb, login, Method::GET, &uri, json!({}), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{login}");
    }
    let (st, e) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &uri,
        nodulo_tr4(),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::FORBIDDEN, Some("sem-vinculo-de-cuidado"))
    );
    let (st, e) = pedir(&amb, "marcos.freire", Method::GET, &uri, json!({}), None).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::FORBIDDEN, Some("sem-vinculo-de-cuidado"))
    );
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::GET,
        &format!("/v1/laudos/{}/achados", Uuid::now_v7()),
        json!({}),
        None,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::NOT_FOUND, Some("laudo-nao-encontrado"))
    );
    assert_eq!(achados_no_banco(&amb).await, 1);
    assert_eq!(
        amb.contar("SELECT count(*) FROM auditoria.acesso WHERE resultado = 'negado'")
            .await,
        2
    );

    // RLS barra mesmo se a rota errar: o médico sem vínculo não vê achado nenhum.
    let mut tx = amb.app_pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.unidade_id', $1, true), set_config('app.perfis', 'medico', true), set_config('app.usuario_id', $2, true)")
        .bind(comum::UNIDADE_A.to_string()).bind(amb.s.usuarios["marcos.freire"].to_string()).execute(&mut *tx).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM laudos.achado")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(n, 0);
    tx.rollback().await.unwrap();

    // Com acesso de emergência ele lê, mas o laudo é de outro médico: não registra nem remove.
    let (st, e) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &format!("/v1/laudos/{laudo}/acesso-emergencia"),
        json!({ "justificativa": "paciente em atendimento de urgência no pronto-socorro" }),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{e}");
    let (st, lista) = pedir(&amb, "marcos.freire", Method::GET, &uri, json!({}), None).await;
    assert_eq!(st, StatusCode::OK, "{lista}");
    assert_eq!(lista["editavel"], false);
    let (st, e) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &uri,
        nodulo_tr4(),
        Some(Uuid::now_v7()),
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::FORBIDDEN, Some("laudo-de-outro-medico"))
    );
    let achado = lista["achados"][0]["id"].as_str().unwrap();
    let (st, _) = pedir(
        &amb,
        "marcos.freire",
        Method::POST,
        &format!("{uri}/{achado}/remocao"),
        json!({}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    assert_eq!(
        amb.contar("SELECT count(*) FROM laudos.achado WHERE situacao = 'ativo'")
            .await,
        1
    );
}

/// Quando o mesmo pedido chega duas vezes (clique duplo, ao mesmo tempo ou não), então só um achado
/// é gravado e a resposta é a mesma; pedidos diferentes simultâneos ganham números diferentes.
#[tokio::test]
async fn repeticao_com_a_mesma_chave_nao_duplica() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let uri = format!("/v1/laudos/{laudo}/achados");
    let chave = Uuid::now_v7();
    let (s1, a1) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &uri,
        nodulo_tr4(),
        Some(chave),
    )
    .await;
    let (s2, a2) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &uri,
        nodulo_tr4(),
        Some(chave),
    )
    .await;
    assert_eq!((s1, s2), (StatusCode::CREATED, StatusCode::CREATED));
    assert_eq!(a1, a2);
    assert_eq!(achados_no_banco(&amb).await, 1);

    // Duas requisições simultâneas com a mesma chave: uma só tem efeito.
    let chave = Uuid::now_v7();
    let (r1, r2) = tokio::join!(
        pedir(
            &amb,
            "medico.a",
            Method::POST,
            &uri,
            nodulo_tr4(),
            Some(chave)
        ),
        pedir(
            &amb,
            "medico.a",
            Method::POST,
            &uri,
            nodulo_tr4(),
            Some(chave)
        ),
    );
    assert_eq!(r1.1["id"], r2.1["id"], "{r1:?} {r2:?}");
    assert_eq!(achados_no_banco(&amb).await, 2);

    // Duas simultâneas com chaves diferentes: dois achados, números distintos (sem colisão).
    let (r3, r4) = tokio::join!(
        pedir(
            &amb,
            "medico.a",
            Method::POST,
            &uri,
            nodulo_tr4(),
            Some(Uuid::now_v7())
        ),
        pedir(
            &amb,
            "medico.a",
            Method::POST,
            &uri,
            nodulo_tr4(),
            Some(Uuid::now_v7())
        ),
    );
    assert_eq!(
        (r3.0, r4.0),
        (StatusCode::CREATED, StatusCode::CREATED),
        "{r3:?} {r4:?}"
    );
    let mut numeros = vec![
        r3.1["numero"].as_i64().unwrap(),
        r4.1["numero"].as_i64().unwrap(),
    ];
    numeros.sort_unstable();
    assert_eq!(numeros, vec![3, 4]);

    // Sem a chave, nada é gravado.
    let s = amb.entrar("medico.a").await;
    let (st, _, e) = amb
        .pedir(amb.json(Method::POST, &uri, &s, None, nodulo_tr4()))
        .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::BAD_REQUEST, Some("idempotency-key-ausente"))
    );
    assert_eq!(achados_no_banco(&amb).await, 4);
}

/// Quando o achado é removido do rascunho, então sai da lista mas não é apagado; repetir não muda nada.
#[tokio::test]
async fn remocao_no_rascunho_nao_apaga() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let (_, a) = criar(&amb, laudo, nodulo_tr4()).await;
    let uri = format!(
        "/v1/laudos/{laudo}/achados/{}/remocao",
        a["id"].as_str().unwrap()
    );
    let (st, l1) = pedir(&amb, "medico.a", Method::POST, &uri, json!({}), None).await;
    assert_eq!(st, StatusCode::OK, "{l1}");
    assert_eq!(l1["achados"], json!([]));
    let (st, l2) = pedir(&amb, "medico.a", Method::POST, &uri, json!({}), None).await;
    assert_eq!((st, &l1), (StatusCode::OK, &l2));
    assert_eq!(amb.contar("SELECT count(*) FROM laudos.achado WHERE situacao = 'removido' AND removido_por IS NOT NULL").await, 1);
    assert_eq!(
        amb.contar("SELECT count(*) FROM lastro.evento WHERE tipo = 'achado_removido'")
            .await,
        1
    );
    let (st, e) = pedir(
        &amb,
        "medico.a",
        Method::POST,
        &format!("/v1/laudos/{laudo}/achados/{}/remocao", Uuid::now_v7()),
        json!({}),
        None,
    )
    .await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::NOT_FOUND, Some("achado-nao-encontrado"))
    );
    // O próximo achado ganha o número seguinte (N2): o removido continua ocupando o N1.
    let (_, b) = criar(&amb, laudo, nodulo_tr4()).await;
    assert_eq!(b["rotulo"], "N2");
}

/// Quando os dados vêm fora dos limites ou incompletos, então nada é gravado e a mensagem diz o que fazer.
#[tokio::test]
async fn limites_e_dados_invalidos() {
    let amb = Ambiente::novo().await;
    let (_, laudo) = amb.ate_o_laudo(ANTONIO, NASC).await;
    let com = |campo: &str, valor: Value| {
        let mut c = nodulo_tr4();
        c[campo] = valor;
        c
    };
    let mut foco_desconhecido = nodulo_tr4();
    foco_desconhecido["tirads"]["focos"] = json!(["brilhante"]);
    let casos = [
        (com("maior_diametro_mm", json!(0)), "medida-invalida"),
        (com("maior_diametro_mm", json!(-3)), "medida-invalida"),
        (com("maior_diametro_mm", json!(100.1)), "medida-invalida"),
        (com("maior_diametro_mm", Value::Null), "medida-invalida"),
        (com("localizacao", json!("   ")), "localizacao-invalida"),
        (
            com("localizacao", json!("x".repeat(121))),
            "localizacao-invalida",
        ),
        (com("tirads", Value::Null), "descritores-ausentes"),
        (com("sistema", json!("lirads")), "corpo-invalido"),
        (foco_desconhecido, "corpo-invalido"),
    ];
    for (corpo, codigo) in casos {
        let (st, e) = criar(&amb, laudo, corpo.clone()).await;
        assert_eq!(
            (st, e["codigo"].as_str()),
            (StatusCode::UNPROCESSABLE_ENTITY, Some(codigo)),
            "{corpo}"
        );
        assert!(
            e["detail"].as_str().unwrap().contains("Nada foi gravado"),
            "{e}"
        );
    }
    assert_eq!(achados_no_banco(&amb).await, 0);

    // Limites exatos aceitos: 0,1 mm e 100 mm.
    assert_eq!(
        criar(&amb, laudo, com("maior_diametro_mm", json!(0.1)))
            .await
            .0,
        StatusCode::CREATED
    );
    assert_eq!(
        criar(&amb, laudo, com("maior_diametro_mm", json!(100)))
            .await
            .0,
        StatusCode::CREATED
    );
    // Até 20 achados ativos por versão; o 21º é recusado.
    for _ in 0..18 {
        assert_eq!(
            criar(&amb, laudo, nodulo_tr4()).await.0,
            StatusCode::CREATED
        );
    }
    let (st, e) = criar(&amb, laudo, nodulo_tr4()).await;
    assert_eq!(
        (st, e["codigo"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("limite-de-achados"))
    );
    assert_eq!(achados_no_banco(&amb).await, 20);
}
