//! Usuários e perfis (Usuarios-Perfis.dc.html; docs/18). TI e gerência administram contas da própria
//! unidade. Perfil clínico novo e reativação esperam a conferência de uma segunda pessoa; retirar
//! perfil e desativar valem na hora (e derrubam a sessão, que confere `ativo` e os perfis a cada
//! pedido). Separação de funções: TI não acumula perfil clínico; caixa não acumula financeiro.
//! Mudar permissão pede reautenticação (SEC-AUT-5). Nada é apagado.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

/// Perfis conhecidos (docs/18 § 3). Perfil desconhecido é recusado (negar por padrão).
pub const PERFIS: [&str; 11] = [
    "recepcao",
    "caixa",
    "tecnico",
    "enfermagem",
    "medico",
    "faturista",
    "financeiro",
    "gerente",
    "estoque",
    "auditor",
    "ti",
];
/// Perfis com dado clínico: conceder exige segunda pessoa.
pub const CLINICOS: [&str; 4] = ["medico", "tecnico", "enfermagem", "auditor"];

/// Combinação proibida pela separação de funções (docs/18 § 6), com o motivo para a tela.
#[must_use]
pub fn conflito(perfis: &[String]) -> Option<&'static str> {
    let tem = |p: &str| perfis.iter().any(|x| x == p);
    if tem("ti") && CLINICOS.iter().any(|c| tem(c)) {
        return Some("A TI não acumula perfil com dado clínico.");
    }
    if tem("caixa") && tem("financeiro") {
        return Some("Quem recebe no caixa não concilia no financeiro.");
    }
    None
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UsuarioAdministrado {
    pub id: Uuid,
    pub codigo: String,
    pub login: String,
    pub nome: String,
    pub perfis: Vec<String>,
    pub ativo: bool,
    pub ultimo_acesso: Option<DateTime<Utc>>,
    /// Pedido aguardando a segunda pessoa (tipo e perfil).
    pub pendente: Option<PedidoPendente>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PedidoPendente {
    pub id: Uuid,
    pub tipo: String,
    pub perfil: Option<String>,
    pub motivo: String,
    pub pedido_por: String,
}

async fn listar_todos(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    id: Option<Uuid>,
) -> Result<Vec<UsuarioAdministrado>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT u.id, u.codigo, u.login, u.nome, u.perfis, u.ativo,
                  (SELECT max(s.criado_em) FROM identidade.sessao s WHERE s.usuario_id = u.id) AS ultimo,
                  p.id AS "pid?", p.tipo AS "ptipo?", p.perfil AS "pperfil?", p.motivo AS "pmotivo?", q.nome AS "ppor?"
           FROM identidade.usuario u
           LEFT JOIN identidade.pedido_perfil p ON p.usuario_id = u.id AND p.situacao = 'aguardando'
           LEFT JOIN identidade.usuario q ON q.id = p.pedido_por
           WHERE u.unidade_id = $1 AND ($2::uuid IS NULL OR u.id = $2) ORDER BY u.nome LIMIT 500"#,
        unidade,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| UsuarioAdministrado {
            pendente: match (l.pid, l.ptipo, l.pmotivo, l.ppor) {
                (Some(id), Some(tipo), Some(motivo), Some(por)) => Some(PedidoPendente {
                    id,
                    tipo,
                    perfil: l.pperfil,
                    motivo,
                    pedido_por: por,
                }),
                _ => None,
            },
            id: l.id,
            codigo: l.codigo,
            login: l.login,
            nome: l.nome,
            perfis: l.perfis,
            ativo: l.ativo,
            ultimo_acesso: l.ultimo,
        })
        .collect())
}

async fn um(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    id: Uuid,
) -> Result<UsuarioAdministrado, ErroApi> {
    listar_todos(tx, unidade, Some(id))
        .await?
        .pop()
        .ok_or_else(|| {
            ErroApi::nao_encontrado(
                "usuario-nao-encontrado",
                "Usuário não encontrado nesta unidade.",
            )
        })
}

/// Usuários da unidade com perfis e pedidos aguardando (TI e gerência).
#[utoipa::path(get, operation_id = "listar_usuarios", path = "/v1/usuarios", tag = "usuarios", responses((status = 200, body = Vec<UsuarioAdministrado>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<UsuarioAdministrado>>, ErroApi> {
    usuario.exigir(Ato::UsuarioAdministrar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = listar_todos(&mut tx, usuario.unidade_id, None).await?;
    tx.commit().await?;
    Ok(Json(r))
}

#[derive(Deserialize, ToSchema)]
pub struct NovoUsuario {
    /// O mesmo login do provedor de identidade (Keycloak).
    pub login: String,
    pub nome: String,
    pub perfis: Vec<String>,
    pub motivo: String,
}

fn validar_perfis(perfis: &[String]) -> Result<(), ErroApi> {
    if perfis.is_empty() || perfis.len() > 4 || perfis.iter().any(|p| !PERFIS.contains(&p.as_str()))
    {
        return Err(ErroApi::invalido(
            "perfil-invalido",
            "Escolha de 1 a 4 perfis da lista. Nada foi alterado.",
        ));
    }
    if let Some(m) = conflito(perfis) {
        return Err(ErroApi::novo(
            StatusCode::UNPROCESSABLE_ENTITY,
            "separacao-de-funcoes",
            "Separação de funções",
            &format!("{m} Nada foi alterado."),
        ));
    }
    Ok(())
}

struct NovoPedido<'a> {
    alvo: Uuid,
    tipo: &'a str,
    perfil: Option<&'a str>,
    motivo: &'a str,
    aguarda: bool,
}

async fn registrar_pedido(
    tx: &mut sqlx::PgConnection,
    usuario: &Usuario,
    p: NovoPedido<'_>,
    correlacao: &str,
) -> Result<Uuid, ErroApi> {
    let NovoPedido {
        alvo,
        tipo,
        perfil,
        motivo,
        aguarda,
    } = p;
    let id = novo_id();
    let codigo = proximo(tx, "PPF").await?;
    sqlx::query!(
        "INSERT INTO identidade.pedido_perfil (id, codigo, usuario_id, tipo, perfil, motivo, situacao, pedido_por, decidido_por, decidido_em, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $8, 'usuarios', $11, $12)",
        id, codigo.como_str(), alvo, tipo, perfil, motivo, if aguarda { "aguardando" } else { "aplicado" }, usuario.id,
        (!aguarda).then_some(usuario.id), (!aguarda).then(Utc::now), usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("pedido-aguardando", "Já há uma mudança deste usuário aguardando conferência. Decida essa antes. Nada foi alterado."),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(tx, NovoEvento {
        dominio: "identidade", tipo: if aguarda { "perfil_pedido" } else { "perfil_alterado" }, registro_id: alvo, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "tipo": tipo, "perfil": perfil, "por": usuario.id, "reautenticado": true }), criado_por: Some(usuario.id), origem: "usuarios",
        unidade_id: usuario.unidade_id, correlacao,
    }).await?;
    Ok(id)
}

/// Cria o usuário. Com perfil clínico, ele nasce inativo e espera a segunda pessoa.
#[utoipa::path(post, operation_id = "criar_usuario", path = "/v1/usuarios", tag = "usuarios", request_body = NovoUsuario, params(("Idempotency-Key" = Uuid, Header)),
    responses((status = 201, body = UsuarioAdministrado), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn criar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoUsuario>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::UsuarioAdministrar)?;
    usuario.exigir_reautenticacao()?;
    validar_perfis(&n.perfis)?;
    let login = n.login.trim().to_lowercase();
    if !(3..=60).contains(&login.len())
        || !login
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        || n.nome.trim().len() < 3
        || n.motivo.trim().len() < 5
    {
        return Err(ErroApi::invalido(
            "usuario-invalido",
            "Informe login (letras, números, . _ -), nome completo e motivo. Nada foi criado.",
        ));
    }
    let clinico = n.perfis.iter().any(|p| CLINICOS.contains(&p.as_str()));
    const ROTA: &str = "POST /v1/usuarios";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, ROTA, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let id = novo_id();
    sqlx::query!(
        "INSERT INTO identidade.usuario (id, codigo, login, nome, perfis, ativo, criado_por, origem, unidade_id, correlacao) VALUES ($1, $2, $3, $4, $5, $6, $7, 'usuarios', $8, $9)",
        id, format!("USR-{login}"), login, n.nome.trim(), &n.perfis, !clinico, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("login-em-uso", "Este login já existe. Nada foi criado."),
        _ => ErroApi::from(e),
    })?;
    let pedido = NovoPedido {
        alvo: id,
        tipo: "criar",
        perfil: None,
        motivo: n.motivo.trim(),
        aguarda: clinico,
    };
    registrar_pedido(&mut tx, &usuario, pedido, &correlacao).await?;
    let corpo = serde_json::to_value(um(&mut tx, usuario.unidade_id, id).await?)
        .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

#[derive(Deserialize, ToSchema)]
pub struct MudancaDePerfil {
    /// conceder, retirar, desativar ou reativar.
    pub tipo: String,
    pub perfil: Option<String>,
    pub motivo: String,
}

/// Pede uma mudança. Conceder perfil clínico e reativar esperam a segunda pessoa; o resto vale já.
#[utoipa::path(post, operation_id = "mudar_perfil", path = "/v1/usuarios/{id}/mudancas", tag = "usuarios", params(("id" = Uuid, Path)), request_body = MudancaDePerfil,
    responses((status = 200, body = UsuarioAdministrado), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn mudar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(m): Corpo<MudancaDePerfil>,
) -> Result<Json<UsuarioAdministrado>, ErroApi> {
    usuario.exigir(Ato::UsuarioAdministrar)?;
    if id == usuario.id {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "separacao-de-funcoes",
            "Própria conta",
            "Ninguém muda os próprios perfis. Peça a outra pessoa. Nada foi alterado.",
        ));
    }
    usuario.exigir_reautenticacao()?;
    if m.motivo.trim().chars().count() < 5 {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Diga o motivo da mudança. Nada foi alterado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let atual = um(&mut tx, usuario.unidade_id, id).await?;
    let mut perfis = atual.perfis.clone();
    let perfil = m.perfil.as_deref();
    let aguarda = match (m.tipo.as_str(), perfil) {
        ("conceder", Some(p)) if PERFIS.contains(&p) && !perfis.iter().any(|x| x == p) => {
            perfis.push(p.to_owned());
            validar_perfis(&perfis)?;
            CLINICOS.contains(&p)
        }
        ("retirar", Some(p)) if perfis.iter().any(|x| x == p) && perfis.len() > 1 => false,
        ("desativar", None) if atual.ativo => false,
        ("reativar", None) if !atual.ativo => true,
        _ => {
            return Err(ErroApi::invalido(
                "mudanca-invalida",
                "Esta mudança não se aplica a este usuário agora. Nada foi alterado.",
            ))
        }
    };
    registrar_pedido(
        &mut tx,
        &usuario,
        NovoPedido {
            alvo: id,
            tipo: &m.tipo,
            perfil,
            motivo: m.motivo.trim(),
            aguarda,
        },
        &correlacao,
    )
    .await?;
    if !aguarda {
        aplicar(&mut tx, id, &m.tipo, perfil).await?;
    }
    let r = um(&mut tx, usuario.unidade_id, id).await?;
    tx.commit().await?;
    Ok(Json(r))
}

async fn aplicar(
    tx: &mut sqlx::PgConnection,
    id: Uuid,
    tipo: &str,
    perfil: Option<&str>,
) -> Result<(), ErroApi> {
    match (tipo, perfil) {
        ("conceder", Some(p)) => sqlx::query!("UPDATE identidade.usuario SET perfis = array_append(perfis, $2), versao = versao + 1 WHERE id = $1 AND NOT ($2 = ANY(perfis))", id, p).execute(&mut *tx).await?,
        ("retirar", Some(p)) => sqlx::query!("UPDATE identidade.usuario SET perfis = array_remove(perfis, $2), versao = versao + 1 WHERE id = $1", id, p).execute(&mut *tx).await?,
        ("desativar", _) => {
            // Desativar derruba as sessões abertas na hora.
            sqlx::query!("UPDATE identidade.sessao SET revogada_em = now() WHERE usuario_id = $1 AND revogada_em IS NULL", id).execute(&mut *tx).await?;
            sqlx::query!("UPDATE identidade.usuario SET ativo = false, versao = versao + 1 WHERE id = $1", id).execute(&mut *tx).await?
        }
        ("reativar" | "criar", _) => sqlx::query!("UPDATE identidade.usuario SET ativo = true, versao = versao + 1 WHERE id = $1", id).execute(&mut *tx).await?,
        _ => return Err(ErroApi::interno("tipo de mudança desconhecido")),
    };
    Ok(())
}

#[derive(Deserialize, ToSchema)]
pub struct DecisaoDePerfil {
    pub aprovar: bool,
    pub motivo: String,
}

/// A segunda pessoa confere o pedido (gerência; quem pediu não decide; reautenticação).
#[utoipa::path(post, operation_id = "decidir_perfil", path = "/v1/usuarios/pedidos/{id}/decisao", tag = "usuarios", params(("id" = Uuid, Path)), request_body = DecisaoDePerfil,
    responses((status = 200, body = UsuarioAdministrado), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn decidir(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(pedido): Path<Uuid>,
    Corpo(d): Corpo<DecisaoDePerfil>,
) -> Result<Json<UsuarioAdministrado>, ErroApi> {
    usuario.exigir(Ato::PerfilConferir)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let p = sqlx::query!("SELECT codigo, usuario_id, tipo, perfil, pedido_por, situacao FROM identidade.pedido_perfil WHERE id = $1 FOR UPDATE", pedido)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ErroApi::nao_encontrado("pedido-nao-encontrado", "Pedido não encontrado."))?;
    if p.pedido_por == usuario.id || p.usuario_id == usuario.id {
        return Err(ErroApi::novo(StatusCode::FORBIDDEN, "separacao-de-funcoes", "Segunda pessoa", "A conferência é de outra pessoa: quem pediu, e o próprio usuário, não decidem. Nada foi alterado."));
    }
    usuario.exigir_reautenticacao()?;
    if p.situacao != "aguardando" {
        return Err(ErroApi::conflito(
            "pedido-ja-decidido",
            "Este pedido já foi decidido. Nada foi alterado.",
        ));
    }
    if d.motivo.trim().chars().count() < 5 {
        return Err(ErroApi::invalido(
            "motivo-obrigatorio",
            "Diga o motivo da decisão. Nada foi alterado.",
        ));
    }
    sqlx::query!(
        "UPDATE identidade.pedido_perfil SET situacao = $2, decidido_por = $3, decidido_em = now(), versao = versao + 1 WHERE id = $1 AND situacao = 'aguardando'",
        pedido, if d.aprovar { "aplicado" } else { "recusado" }, usuario.id
    )
    .execute(&mut *tx)
    .await?;
    if d.aprovar {
        aplicar(&mut tx, p.usuario_id, &p.tipo, p.perfil.as_deref()).await?;
    }
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "identidade", tipo: if d.aprovar { "perfil_conferido" } else { "perfil_recusado" }, registro_id: p.usuario_id, registro_codigo: Some(&p.codigo),
        dados: json!({ "tipo": p.tipo, "perfil": p.perfil, "pedido_por": p.pedido_por, "decidido_por": usuario.id, "motivo": d.motivo.trim(), "reautenticado": true }),
        criado_por: Some(usuario.id), origem: "usuarios", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let r = um(&mut tx, usuario.unidade_id, p.usuario_id).await?;
    tx.commit().await?;
    Ok(Json(r))
}
