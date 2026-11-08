//! Infraestrutura dos testes de integração (G4): PostgreSQL real em contêiner (Testcontainers),
//! um banco novo por teste, migrations reais, papel `vidacor_app` sem BYPASSRLS e a aplicação
//! axum chamada pela rota HTTP. Só dados fictícios (design/dados-ficticios.md).

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::body::Body;
use axum::http::{Method, Request, Response, StatusCode};
use axum::Router;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::OnceLock;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::SyncRunner;
use testcontainers_modules::testcontainers::{Container, ImageExt};
use tower::ServiceExt;
use uuid::Uuid;
use vidacor_api::config::Config;
use vidacor_api::{aplicacao, sessao, Estado};

struct Servidor {
    _conteiner: Container<Postgres>,
    porta: u16,
}

fn servidor() -> &'static Servidor {
    static S: OnceLock<Servidor> = OnceLock::new();
    // Sobe numa thread própria: o executor bloqueante não pode rodar dentro do runtime do teste.
    S.get_or_init(|| {
        std::thread::spawn(|| {
            let conteiner = Postgres::default()
                .with_tag("17-alpine")
                .start()
                .expect("subir PostgreSQL");
            let porta = conteiner.get_host_port_ipv4(5432).expect("porta");
            // Papel é objeto do cluster: criado uma vez aqui, antes dos testes em paralelo, para as
            // migrations de cada banco não disputarem o CREATE/ALTER ROLE.
            let rt = tokio::runtime::Runtime::new().expect("runtime");
            rt.block_on(async {
                let url = format!("postgres://postgres:postgres@127.0.0.1:{porta}/postgres");
                let raiz = PgPoolOptions::new()
                    .max_connections(1)
                    .connect(&url)
                    .await
                    .expect("conectar");
                sqlx::query(
                    "CREATE ROLE vidacor_app LOGIN NOSUPERUSER NOBYPASSRLS PASSWORD 'app_teste'",
                )
                .execute(&raiz)
                .await
                .expect("papel");
                raiz.close().await;
            });
            Servidor {
                _conteiner: conteiner,
                porta,
            }
        })
        .join()
        .expect("thread do contêiner")
    })
}

pub const UNIDADE_A: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00a1);
pub const UNIDADE_B: Uuid = Uuid::from_u128(0x0000_0000_0000_7000_8000_0000_0000_00b1);

pub struct Ambiente {
    pub app: Router,
    pub admin: PgPool,
    pub app_pool: PgPool,
    pub estado: Estado,
    pub sala1: Uuid,
    pub antonio: Uuid,
    pub maria: Uuid,
    pub agendamento_antonio: Uuid,
    pub agendamento_maria: Uuid,
    pub dia: NaiveDate,
}

pub struct Sessao {
    pub cookie: String,
    pub csrf: String,
    pub usuario_id: Uuid,
}

impl Ambiente {
    pub async fn novo() -> Self {
        let porta = servidor().porta;
        let base = format!("postgres://postgres:postgres@127.0.0.1:{porta}");
        let raiz = PgPoolOptions::new()
            .max_connections(1)
            .connect(&format!("{base}/postgres"))
            .await
            .unwrap();
        let nome = format!("t_{}", Uuid::now_v7().simple());
        // Nome gerado pelo próprio teste (UUID), nunca vindo de fora.
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {nome}")))
            .execute(&raiz)
            .await
            .unwrap();
        raiz.close().await;

        let admin = PgPoolOptions::new()
            .max_connections(4)
            .connect(&format!("{base}/{nome}"))
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&admin).await.unwrap();

        let app_pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&format!(
                "postgres://vidacor_app:app_teste@127.0.0.1:{porta}/{nome}"
            ))
            .await
            .unwrap();

        let ids = semear(&admin).await;
        let config = Config {
            endereco: "127.0.0.1:0".parse().unwrap(),
            banco_url: String::new(),
            banco_admin_url: None,
            url_aplicacao: "http://127.0.0.1:5173".into(),
            url_api: "http://127.0.0.1:5173".into(),
            cookie_seguro: true,
            orthanc_url: None,
            unidade_servidor: Some(UNIDADE_A),
            oidc: None,
        };
        let estado = Estado::novo(app_pool.clone(), config, None);
        Self {
            app: aplicacao(estado.clone()),
            admin,
            app_pool,
            estado,
            sala1: ids.0,
            antonio: ids.1,
            maria: ids.2,
            agendamento_antonio: ids.3,
            agendamento_maria: ids.4,
            dia: ids.5,
        }
    }

    /// Sessão de um usuário semeado (pelo login), como faria o retorno do OIDC.
    pub async fn entrar(&self, login: &str) -> Sessao {
        let u: (Uuid, Uuid) =
            sqlx::query_as("SELECT id, unidade_id FROM identidade.usuario WHERE login = $1")
                .bind(login)
                .fetch_one(&self.admin)
                .await
                .unwrap();
        let s = sessao::criar(&self.app_pool, u.0, u.1, "teste")
            .await
            .unwrap();
        Sessao {
            cookie: format!("{}={}", sessao::COOKIE, s.segredo),
            csrf: s.csrf,
            usuario_id: u.0,
        }
    }

    pub async fn pedir(&self, req: Request<Body>) -> (StatusCode, Response<Body>, Value) {
        let resposta = self.app.clone().oneshot(req).await.unwrap();
        let status = resposta.status();
        let (partes, corpo) = resposta.into_parts();
        let bytes = corpo.collect().await.unwrap().to_bytes();
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, Response::from_parts(partes, Body::empty()), json)
    }

    pub fn get(&self, uri: &str, s: Option<&Sessao>) -> Request<Body> {
        let mut b = Request::builder().method(Method::GET).uri(uri);
        if let Some(s) = s {
            b = b.header("cookie", &s.cookie);
        }
        b.body(Body::empty()).unwrap()
    }

    pub fn checkin(
        &self,
        s: &Sessao,
        chave: Option<Uuid>,
        agendamento: Uuid,
        nome: &str,
        nasc: &str,
    ) -> Request<Body> {
        let mut b = Request::builder()
            .method(Method::POST)
            .uri("/v1/atendimentos")
            .header("content-type", "application/json")
            .header("cookie", &s.cookie)
            .header("x-csrf-token", &s.csrf);
        if let Some(c) = chave {
            b = b.header("idempotency-key", c.to_string());
        }
        let corpo = serde_json::json!({
            "agendamento_id": agendamento,
            "confirmacao": { "nome": nome, "nascimento": nasc }
        });
        b.body(Body::from(corpo.to_string())).unwrap()
    }

    pub async fn contar(&self, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql)
            .fetch_one(&self.admin)
            .await
            .unwrap()
    }
}

/// Dados fictícios: unidades Centro (A) e Norte (B), usuários por perfil, Sala 1, pacientes
/// Antônio Carlos Ribeiro (15/06/1954) e Maria Helena Duarte (12/03/1968), agendados hoje.
async fn semear(admin: &PgPool) -> (Uuid, Uuid, Uuid, Uuid, Uuid, NaiveDate) {
    let dia = vidacor_api::tempo::hoje_local();
    let meio_dia = Utc.from_utc_datetime(&dia.and_hms_opt(12, 0, 0).unwrap());
    let sala1 = Uuid::now_v7();
    let antonio = Uuid::now_v7();
    let maria = Uuid::now_v7();
    let ag_antonio = Uuid::now_v7();
    let ag_maria = Uuid::now_v7();
    let mut tx = admin.begin().await.unwrap();
    for (id, codigo, nome) in [
        (UNIDADE_A, "UNI-CENTRO", "Centro"),
        (UNIDADE_B, "UNI-NORTE", "Norte"),
    ] {
        sqlx::query("INSERT INTO identidade.unidade (id, codigo, nome, origem, unidade_id, correlacao) VALUES ($1,$2,$3,'seed',$1,'seed')")
            .bind(id).bind(codigo).bind(nome).execute(&mut *tx).await.unwrap();
    }
    for (login, nome, perfil, unidade) in [
        ("ana.souza", "Ana Souza", "recepcao", UNIDADE_A),
        ("bruno.sales", "Bruno Sales", "tecnico", UNIDADE_A),
        ("medico.a", "Dr. Paulo Mendes", "medico", UNIDADE_A),
        ("ti.a", "Equipe TI", "ti", UNIDADE_A),
        ("recepcao.b", "Carla Mendes", "recepcao", UNIDADE_B),
    ] {
        sqlx::query("INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao) VALUES ($1,$2,$3,$4,ARRAY[$5],'seed',$6,'seed')")
            .bind(Uuid::now_v7()).bind(format!("USR-{login}")).bind(login).bind(nome).bind(perfil).bind(unidade)
            .execute(&mut *tx).await.unwrap();
    }
    sqlx::query("INSERT INTO agenda.sala (id, codigo, nome, ae_title, origem, unidade_id, correlacao) VALUES ($1,'SALA-1','Sala 1','SALA1','seed',$2,'seed')")
        .bind(sala1).bind(UNIDADE_A).execute(&mut *tx).await.unwrap();
    for (id, codigo, nome, nasc) in [
        (
            antonio,
            "PAC-2026-000001",
            "Antônio Carlos Ribeiro",
            "1954-06-15",
        ),
        (
            maria,
            "PAC-2026-000002",
            "Maria Helena Duarte",
            "1968-03-12",
        ),
    ] {
        sqlx::query("INSERT INTO identidade.paciente (id, codigo, nome, nascimento, origem, unidade_id, correlacao) VALUES ($1,$2,$3,$4::date,'seed',$5,'seed')")
            .bind(id).bind(codigo).bind(nome).bind(nasc).bind(UNIDADE_A).execute(&mut *tx).await.unwrap();
    }
    for (id, codigo, paciente, inicio) in [
        (ag_antonio, "AGD-2026-000001", antonio, meio_dia),
        (
            ag_maria,
            "AGD-2026-000002",
            maria,
            meio_dia + Duration::minutes(30),
        ),
    ] {
        sqlx::query("INSERT INTO agenda.agendamento (id, codigo, sala_id, paciente_id, inicio, fim, exame_tuss, exame_nome, situacao, origem, unidade_id, correlacao) VALUES ($1,$2,$3,$4,$5,$5 + interval '30 minutes','40901114','Ecocardiograma transtorácico','confirmado','seed',$6,'seed')")
            .bind(id).bind(codigo).bind(sala1).bind(paciente).bind(inicio).bind(UNIDADE_A).execute(&mut *tx).await.unwrap();
    }
    tx.commit().await.unwrap();
    (sala1, antonio, maria, ag_antonio, ag_maria, dia)
}
