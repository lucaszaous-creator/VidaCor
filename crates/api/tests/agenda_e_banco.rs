//! Agenda do dia, RLS direto no banco, lastro imutável e metadados de lastro em toda tabela
//! (REQ-AGD-001, REQ-LAS-001, REQ-LAS-002; docs/18 § 7, docs/21 § 5).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod comum;

use axum::http::StatusCode;
use comum::{Ambiente, UNIDADE_A};

#[tokio::test]
async fn agenda_do_dia_traz_os_dois_identificadores() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (status, resp, corpo) = amb
        .pedir(amb.get(&format!("/v1/agenda?data={}", amb.dia), Some(&ana)))
        .await;
    assert_eq!(status, StatusCode::OK, "{corpo}");
    assert_eq!(resp.headers()["cache-control"], "no-store");
    let itens = corpo["itens"].as_array().unwrap();
    assert_eq!(itens.len(), 4);
    assert_eq!(itens[0]["paciente"]["nome"], "Antônio Carlos Ribeiro");
    assert_eq!(itens[0]["paciente"]["nascimento"], "1954-06-15");
    assert_eq!(itens[0]["sala"]["nome"], "Sala 1");
    assert!(
        itens[0]["inicio"].as_str() < itens[1]["inicio"].as_str(),
        "ordenada por horário"
    );
}

#[tokio::test]
async fn agenda_de_outra_unidade_vem_vazia() {
    let amb = Ambiente::novo().await;
    let outra = amb.entrar("recepcao.b").await;
    let (status, _, corpo) = amb
        .pedir(amb.get(&format!("/v1/agenda?data={}", amb.dia), Some(&outra)))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corpo["itens"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn agenda_com_data_invalida_responde_400_ou_422() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (status, _, _) = amb
        .pedir(amb.get("/v1/agenda?data=2026-13-40", Some(&ana)))
        .await;
    assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn sessao_atual_traz_csrf_e_unidade() {
    let amb = Ambiente::novo().await;
    let ana = amb.entrar("ana.souza").await;
    let (status, _, corpo) = amb.pedir(amb.get("/v1/sessao", Some(&ana))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corpo["usuario"]["nome"], "Ana Souza");
    assert_eq!(corpo["unidade"]["nome"], "Centro");
    assert_eq!(corpo["csrf"], ana.csrf);
}

#[tokio::test]
async fn rls_sem_contexto_nao_entrega_paciente() {
    let amb = Ambiente::novo().await;
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM identidade.paciente")
        .fetch_one(&amb.app_pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "papel da aplicação sem contexto não vê paciente");
}

#[tokio::test]
async fn rls_com_contexto_autorizado_e_nao_autorizado() {
    let amb = Ambiente::novo().await;
    for (perfis, esperado) in [("recepcao", 5_i64), ("ti", 0), ("financeiro", 0), ("", 0)] {
        let mut tx = amb.app_pool.begin().await.unwrap();
        sqlx::query(
            "SELECT set_config('app.unidade_id', $1, true), set_config('app.perfis', $2, true)",
        )
        .bind(UNIDADE_A.to_string())
        .bind(perfis)
        .execute(&mut *tx)
        .await
        .unwrap();
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM identidade.paciente")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(n, esperado, "perfil {perfis:?}");
        tx.rollback().await.unwrap();
    }
}

#[tokio::test]
async fn contexto_nao_vaza_entre_transacoes_do_pool() {
    let amb = Ambiente::novo().await;
    let mut tx = amb.app_pool.begin().await.unwrap();
    sqlx::query(
        "SELECT set_config('app.unidade_id', $1, true), set_config('app.perfis', 'recepcao', true)",
    )
    .bind(UNIDADE_A.to_string())
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM identidade.paciente")
        .fetch_one(&amb.app_pool)
        .await
        .unwrap();
    assert_eq!(n, 0, "SET LOCAL acaba com a transação");
}

#[tokio::test]
async fn papel_da_aplicacao_nao_tem_bypassrls_nem_superusuario() {
    let amb = Ambiente::novo().await;
    let (bypass, superu): (bool, bool) =
        sqlx::query_as("SELECT rolbypassrls, rolsuper FROM pg_roles WHERE rolname = 'vidacor_app'")
            .fetch_one(&amb.admin)
            .await
            .unwrap();
    assert!(!bypass && !superu);
}

#[tokio::test]
async fn nenhuma_politica_rls_usa_true() {
    let amb = Ambiente::novo().await;
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_policies WHERE trim(coalesce(qual,'')) = 'true' OR trim(coalesce(with_check,'')) = 'true'",
    ).fetch_one(&amb.admin).await.unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn lastro_e_somente_insercao_para_a_aplicacao() {
    let amb = Ambiente::novo().await;
    let atualizar = sqlx::query("UPDATE lastro.evento SET tipo = 'x'")
        .execute(&amb.app_pool)
        .await;
    let apagar = sqlx::query("DELETE FROM lastro.evento")
        .execute(&amb.app_pool)
        .await;
    assert!(atualizar.is_err() && apagar.is_err());
}

#[tokio::test]
async fn eventos_de_lastro_encadeiam_hash() {
    let amb = Ambiente::novo().await;
    for i in 0..3 {
        sqlx::query("INSERT INTO lastro.evento (id, dominio, tipo, registro_id, dados, origem, unidade_id, correlacao) VALUES ($1,'teste','t',$1,'{}','teste',$2,$3)")
            .bind(uuid::Uuid::now_v7()).bind(UNIDADE_A).bind(format!("c{i}")).execute(&amb.admin).await.unwrap();
    }
    let quebras: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM (SELECT hash_anterior, lag(hash) OVER (ORDER BY sequencia) AS anterior FROM lastro.evento) x
          WHERE x.anterior IS NOT NULL AND x.hash_anterior IS DISTINCT FROM x.anterior",
    ).fetch_one(&amb.admin).await.unwrap();
    assert_eq!(quebras, 0);
}

#[tokio::test]
async fn toda_tabela_de_negocio_tem_os_metadados_de_lastro() {
    let amb = Ambiente::novo().await;
    let faltando: Vec<(String, String)> = sqlx::query_as(
        r#"SELECT t.table_schema::text || '.' || t.table_name::text, c.coluna
             FROM information_schema.tables t
             CROSS JOIN unnest(ARRAY['criado_em','criado_por','origem','estacao','unidade_id','versao','correlacao']) AS c(coluna)
            WHERE t.table_schema IN ('identidade','agenda','recepcao','exames')
              AND t.table_type = 'BASE TABLE'
              AND t.table_name NOT IN ('login_pendente')
              AND NOT EXISTS (SELECT 1 FROM information_schema.columns k
                               WHERE k.table_schema = t.table_schema AND k.table_name = t.table_name
                                 AND k.column_name = c.coluna)"#,
    ).fetch_all(&amb.admin).await.unwrap();
    assert!(faltando.is_empty(), "tabelas sem metadados: {faltando:?}");
}

#[tokio::test]
async fn tabelas_com_dado_de_paciente_tem_rls_forcado() {
    let amb = Ambiente::novo().await;
    let sem: Vec<String> = sqlx::query_scalar(
        "SELECT n.nspname || '.' || c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
          WHERE (n.nspname, c.relname) IN (('identidade','paciente'),('agenda','agendamento'),('recepcao','atendimento'),('exames','estudo'),('lastro','evento'))
            AND NOT (c.relrowsecurity AND c.relforcerowsecurity)",
    ).fetch_all(&amb.admin).await.unwrap();
    assert!(sem.is_empty(), "sem RLS forçado: {sem:?}");
}

#[tokio::test]
async fn rota_inexistente_responde_404_em_problem_details() {
    let amb = Ambiente::novo().await;
    let (status, resp, corpo) = amb.pedir(amb.get("/v1/nao-existe", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(resp.headers()["content-type"], "application/problem+json");
    assert_eq!(corpo["codigo"], "rota-inexistente");
}
