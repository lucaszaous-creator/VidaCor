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

/// Segredo do webhook do PSP de teste.
pub const SEGREDO_PIX: &str = "segredo-de-teste-do-webhook";

pub const SEGREDO_WHATSAPP: &str = "segredo-de-teste-do-app-whatsapp";
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
    pub s: Semente,
    pub psp: std::sync::Arc<vidacor_api::pix::PspDesenvolvimento>,
    pub correio: std::sync::Arc<vidacor_api::correio::CorreioEmMemoria>,
    pub mensageiro: std::sync::Arc<vidacor_api::mensageiro::MensageiroEmMemoria>,
    pub prefeitura: std::sync::Arc<vidacor_api::prefeitura::PrefeituraEmMemoria>,
}

pub struct Sessao {
    pub cookie: String,
    pub csrf: String,
    pub usuario_id: Uuid,
}

impl Ambiente {
    pub async fn novo() -> Self {
        // Logs do núcleo aparecem na saída do teste que falhar (diagnóstico sem dado real).
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter("warn,vidacor_api=info")
            .try_init();
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
            pix_webhook_segredo: Some(SEGREDO_PIX.to_owned()),
            convite_segredo: Some("segredo-de-teste-do-convite-do-portal-0001".to_owned()),
            whatsapp_segredo: Some(SEGREDO_WHATSAPP.to_owned()),
        };
        let mut estado = Estado::novo(app_pool.clone(), config, None);
        // Provedor externo de assinatura: único ponto simulado (G4); sem valor legal.
        estado.assinador = Some(std::sync::Arc::new(
            vidacor_api::assinatura::AssinadorDesenvolvimento,
        ));
        let psp = std::sync::Arc::new(vidacor_api::pix::PspDesenvolvimento::default());
        estado.psp = Some(psp.clone());
        // E-mail: provedor externo simulado em memória (G4).
        let correio = std::sync::Arc::new(vidacor_api::correio::CorreioEmMemoria::default());
        estado.correio = Some(correio.clone());
        let mensageiro =
            std::sync::Arc::new(vidacor_api::mensageiro::MensageiroEmMemoria::default());
        estado.mensageiro = Some(mensageiro.clone());
        let prefeitura =
            std::sync::Arc::new(vidacor_api::prefeitura::PrefeituraEmMemoria::default());
        estado.prefeitura = Some(prefeitura.clone());
        Self {
            app: aplicacao(estado.clone()),
            admin,
            app_pool,
            estado,
            sala1: ids.salas[&1],
            antonio: ids.pacientes["Antônio Carlos Ribeiro"],
            maria: ids.pacientes["Maria Helena Duarte"],
            agendamento_antonio: ids.agendamentos["Antônio Carlos Ribeiro"],
            agendamento_maria: ids.agendamentos["Maria Helena Duarte"],
            dia: ids.dia,
            s: ids,
            psp,
            correio,
            mensageiro,
            prefeitura,
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
            "confirmacao": { "nome": nome, "nascimento": nasc, "documento_com_foto": true }
        });
        b.body(Body::from(corpo.to_string())).unwrap()
    }

    /// Pedido JSON genérico com sessão, CSRF e (opcional) Idempotency-Key.
    pub fn json(
        &self,
        metodo: Method,
        uri: &str,
        s: &Sessao,
        chave: Option<Uuid>,
        corpo: Value,
    ) -> Request<Body> {
        let mut b = Request::builder()
            .method(metodo)
            .uri(uri)
            .header("content-type", "application/json")
            .header("cookie", &s.cookie)
            .header("x-csrf-token", &s.csrf);
        if let Some(c) = chave {
            b = b.header("idempotency-key", c.to_string());
        }
        b.body(Body::from(corpo.to_string())).unwrap()
    }

    /// Check-in feito pela recepção (atalho para testes de etapas seguintes).
    pub async fn fazer_checkin(&self, paciente: &str, nasc: &str) -> Value {
        let ana = self.entrar("ana.souza").await;
        let ag = self.s.agendamentos[paciente];
        let (status, _, corpo) = self
            .pedir(self.checkin(&ana, Some(Uuid::now_v7()), ag, paciente, nasc))
            .await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "check-in de {paciente}: {corpo}"
        );
        corpo
    }

    /// Envelhece a reautenticação de todas as sessões (para testar o 401 de reautenticação).
    pub async fn expirar_reautenticacao(&self) {
        sqlx::query(
            "UPDATE identidade.sessao SET reautenticado_em = now() - interval '10 minutes'",
        )
        .execute(&self.admin)
        .await
        .unwrap();
    }

    /// Leva um paciente do check-in ao estudo conciliado (o laudo nasce em rascunho, atribuído ao médico).
    pub async fn ate_o_laudo(&self, paciente: &str, nasc: &str) -> (Value, Uuid) {
        let atd = self.fazer_checkin(paciente, nasc).await;
        let accession = atd["accession"].as_str().unwrap().to_owned();
        let estudo = vidacor_api::orthanc::EstudoOrthanc {
            orthanc_id: format!("{}-0000", &Uuid::now_v7().simple().to_string()[..8]),
            study_uid: format!(
                "1.2.826.0.1.{}",
                Uuid::now_v7().as_u128() % 1_000_000_000_000
            ),
            accession: Some(accession),
            patient_id: None,
            imagens: 12,
        };
        vidacor_api::outbox::registrar_estudo(&self.estado, UNIDADE_A, &estudo)
            .await
            .unwrap();
        let laudo: Uuid =
            sqlx::query_scalar("SELECT id FROM laudos.laudo WHERE atendimento_id = $1")
                .bind(Uuid::parse_str(atd["id"].as_str().unwrap()).unwrap())
                .fetch_one(&self.admin)
                .await
                .unwrap();
        (atd, laudo)
    }

    /// Sessão do portal para um paciente (cria a conta se não existir).
    pub async fn entrar_no_portal(&self, paciente: &str) -> String {
        let pid = self.s.pacientes[paciente];
        let conta: Uuid = sqlx::query_scalar(
            "INSERT INTO portal.conta (id, paciente_id, login, origem, unidade_id, correlacao) VALUES ($1, $2, $3, 'seed', $4, 'seed')
             ON CONFLICT (paciente_id) DO UPDATE SET ativa = true RETURNING id")
            .bind(Uuid::now_v7()).bind(pid).bind(format!("p-{pid}")).bind(UNIDADE_A).fetch_one(&self.admin).await.unwrap();
        let segredo = vidacor_api::portal::criar_sessao(&self.app_pool, conta)
            .await
            .unwrap();
        format!("vc_portal={segredo}")
    }

    pub async fn contar(&self, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql)
            .fetch_one(&self.admin)
            .await
            .unwrap()
    }
}

/// Dados fictícios (design/dados-ficticios.md): unidades Centro (A) e Norte (B), usuários por
/// perfil, Salas 1, 2 e 6 com aparelhos, lotação dos técnicos e agendamentos de hoje:
/// Antônio Carlos Ribeiro (Sala 1, Dr. Paulo), Maria Helena Duarte (Sala 1), Juliana Sampaio Rocha
/// (Sala 2) e Vera Lúcia Antunes (Sala 6, TC com contraste, Dra. Cecília).
async fn semear(admin: &PgPool) -> Semente {
    let dia = vidacor_api::tempo::hoje_local();
    let meio_dia = Utc.from_utc_datetime(&dia.and_hms_opt(12, 0, 0).unwrap());
    let mut tx = admin.begin().await.unwrap();
    for (id, codigo, nome) in [
        (UNIDADE_A, "UNI-CENTRO", "Centro"),
        (UNIDADE_B, "UNI-NORTE", "Norte"),
    ] {
        sqlx::query("INSERT INTO identidade.unidade (id, codigo, nome, origem, unidade_id, correlacao) VALUES ($1,$2,$3,'seed',$1,'seed')")
            .bind(id).bind(codigo).bind(nome).execute(&mut *tx).await.unwrap();
    }
    let mut usuarios = std::collections::HashMap::new();
    for (login, nome, perfil, unidade) in [
        ("ana.souza", "Ana Souza", "recepcao", UNIDADE_A),
        ("bruno.sales", "Bruno Sales", "tecnico", UNIDADE_A),
        ("livia.costa", "Lívia Costa", "tecnico", UNIDADE_A),
        ("ricardo.lopes", "Ricardo Lopes", "tecnico", UNIDADE_A),
        ("medico.a", "Dr. Paulo Medeiros", "medico", UNIDADE_A),
        ("cecilia.souto", "Dra. Cecília Souto", "medico", UNIDADE_A),
        ("marcos.freire", "Dr. Marcos Freire", "medico", UNIDADE_A),
        ("carla.caixa", "Carla Mendes", "caixa", UNIDADE_A),
        ("joao.caixa", "João Prado", "caixa", UNIDADE_A),
        ("estoque.a", "Marcos Almoxarifado", "estoque", UNIDADE_A),
        ("patricia.lemos", "Patrícia Lemos", "gerente", UNIDADE_A),
        ("financeiro.a", "Equipe Financeiro", "financeiro", UNIDADE_A),
        ("ti.a", "Equipe TI", "ti", UNIDADE_A),
        ("recepcao.b", "Recepção Norte", "recepcao", UNIDADE_B),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, origem, unidade_id, correlacao) VALUES ($1,$2,$3,$4,ARRAY[$5],'seed',$6,'seed')")
            .bind(id).bind(format!("USR-{login}")).bind(login).bind(nome).bind(perfil).bind(unidade)
            .execute(&mut *tx).await.unwrap();
        usuarios.insert(login, id);
    }
    let mut salas = std::collections::HashMap::new();
    for (n, ae, modalidade, aparelho) in [
        (1, "SALA1", "US", "Vivid E95"),
        (2, "SALA2", "US", "Voluson E10"),
        (6, "SALA6", "CT", "Revolution CT"),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO agenda.sala (id, codigo, nome, ae_title, modalidade, origem, unidade_id, correlacao) VALUES ($1,$2,$3,$4,$5,'seed',$6,'seed')")
            .bind(id).bind(format!("SALA-{n}")).bind(format!("Sala {n}")).bind(ae).bind(modalidade).bind(UNIDADE_A)
            .execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO agenda.aparelho (id, codigo, nome, ae_title, modalidade, sala_id, origem, unidade_id, correlacao) VALUES ($1,$2,$3,$4,$5,$6,'seed',$7,'seed')")
            .bind(Uuid::now_v7()).bind(format!("APA-{n}")).bind(aparelho).bind(format!("AP{n}")).bind(modalidade).bind(id).bind(UNIDADE_A)
            .execute(&mut *tx).await.unwrap();
        salas.insert(n, id);
    }
    for (login, sala) in [("bruno.sales", 1), ("livia.costa", 6), ("ricardo.lopes", 6)] {
        sqlx::query("INSERT INTO identidade.lotacao (usuario_id, sala_id, origem, unidade_id, correlacao) VALUES ($1,$2,'seed',$3,'seed')")
            .bind(usuarios[login]).bind(salas[&sala]).bind(UNIDADE_A).execute(&mut *tx).await.unwrap();
    }
    let mut pacientes = std::collections::HashMap::new();
    for (codigo, nome, nasc, sexo) in [
        (
            "PAC-2026-000001",
            "Antônio Carlos Ribeiro",
            "1954-06-15",
            "M",
        ),
        ("PAC-2026-000002", "Maria Helena Duarte", "1968-03-12", "F"),
        (
            "PAC-2026-000003",
            "Juliana Sampaio Rocha",
            "1993-04-12",
            "F",
        ),
        ("PAC-2026-000004", "Vera Lúcia Antunes", "1957-09-18", "F"),
        ("PAC-2026-000005", "Sônia Maria Castro", "1962-02-03", "F"),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO identidade.paciente (id, codigo, nome, nascimento, sexo, origem, unidade_id, correlacao) VALUES ($1,$2,$3,$4::date,$5,'seed',$6,'seed')")
            .bind(id).bind(codigo).bind(nome).bind(nasc).bind(sexo).bind(UNIDADE_A).execute(&mut *tx).await.unwrap();
        pacientes.insert(nome, id);
    }
    // Códigos da semente ocupam PAC-2026-00000N: o contador do cadastro começa depois (migration 0020).
    sqlx::query("INSERT INTO comum.contador (prefixo, ano, valor) VALUES ('PAC', 2026, 100) ON CONFLICT (prefixo, ano) DO UPDATE SET valor = GREATEST(comum.contador.valor, 100)")
        .execute(&mut *tx).await.unwrap();
    let mut agendamentos = std::collections::HashMap::new();
    for (codigo, paciente, sala, minutos, tuss, exame, contraste, medico) in [
        (
            "AGD-SEED-000001",
            "Antônio Carlos Ribeiro",
            1,
            0,
            "40901114",
            "Ecocardiograma transtorácico",
            false,
            Some("medico.a"),
        ),
        (
            "AGD-SEED-000002",
            "Maria Helena Duarte",
            1,
            30,
            "40901114",
            "Ecocardiograma transtorácico",
            false,
            None,
        ),
        (
            "AGD-SEED-000003",
            "Juliana Sampaio Rocha",
            2,
            60,
            "40901203",
            "USG obstétrica",
            false,
            None,
        ),
        (
            "AGD-SEED-000004",
            "Vera Lúcia Antunes",
            6,
            90,
            "41001109",
            "TC de abdome com contraste",
            true,
            Some("cecilia.souto"),
        ),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO agenda.agendamento (id, codigo, sala_id, paciente_id, inicio, fim, exame_tuss, exame_nome, situacao, requer_contraste, medico_id, origem, unidade_id, correlacao) VALUES ($1,$2,$3,$4,$5,$5 + interval '30 minutes',$6,$7,'confirmado',$8,$9,'seed',$10,'seed')")
            .bind(id).bind(codigo).bind(salas[&sala]).bind(pacientes[paciente]).bind(meio_dia + Duration::minutes(minutos))
            .bind(tuss).bind(exame).bind(contraste).bind(medico.map(|m| usuarios[m])).bind(UNIDADE_A)
            .execute(&mut *tx).await.unwrap();
        agendamentos.insert(paciente, id);
    }
    tx.commit().await.unwrap();
    Semente {
        dia,
        usuarios: usuarios
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
        salas,
        pacientes: pacientes
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
        agendamentos: agendamentos
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    }
}

pub struct Semente {
    pub dia: NaiveDate,
    pub usuarios: std::collections::HashMap<String, Uuid>,
    pub salas: std::collections::HashMap<i32, Uuid>,
    pub pacientes: std::collections::HashMap<String, Uuid>,
    pub agendamentos: std::collections::HashMap<String, Uuid>,
}
