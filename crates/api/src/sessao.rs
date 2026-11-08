//! Sessão em cookie HttpOnly (SEC-AUT-4): o navegador guarda só um segredo aleatório; o banco
//! guarda o hash. Escrita exige o cabeçalho `X-CSRF-Token` igual ao da sessão.

use crate::atos::Ato;
use crate::banco::ContextoBanco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header, Method};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use subtle::ConstantTimeEq;
use uuid::Uuid;

pub const COOKIE: &str = "vc_sessao";
pub const CABECALHO_CSRF: &str = "x-csrf-token";
/// Sessão curta de balcão (docs/11 § 8). Renovação fica para a próxima entrega.
pub const DURACAO_HORAS: i64 = 10;

#[derive(Debug, Clone)]
pub struct Usuario {
    pub id: Uuid,
    pub nome: String,
    pub perfis: Vec<String>,
    pub unidade_id: Uuid,
    pub sessao_id: Uuid,
    pub csrf: String,
    /// Último login com senha e 2FA nesta sessão (SEC-AUT-5).
    pub reautenticado_em: chrono::DateTime<chrono::Utc>,
}

/// Validade da reautenticação para atos sensíveis.
pub const JANELA_REAUTENTICACAO_MIN: i64 = 5;

impl Usuario {
    /// Exige login recente (senha e 2FA) para atos sensíveis: assinar, estornar, autorizar
    /// desconto, conduta clínica, acesso de emergência, corrigir cadastro (SEC-AUT-5).
    ///
    /// # Errors
    /// 401 `reautenticacao-necessaria`.
    pub fn exigir_reautenticacao(&self) -> Result<(), ErroApi> {
        let limite = chrono::Utc::now() - chrono::Duration::minutes(JANELA_REAUTENTICACAO_MIN);
        if self.reautenticado_em >= limite {
            Ok(())
        } else {
            Err(ErroApi::novo(
                axum::http::StatusCode::UNAUTHORIZED,
                "reautenticacao-necessaria",
                "Confirme sua identidade",
                "Este ato exige que você confirme sua senha e o segundo fator. Nada foi alterado. Confirme e o pedido será refeito.",
            ))
        }
    }

    #[must_use]
    pub fn tem_perfil(&self, perfil: &str) -> bool {
        self.perfis.iter().any(|p| p == perfil)
    }

    /// # Errors
    /// [`ErroApi::proibido`] se nenhum perfil do usuário concede o ato.
    pub fn exigir(&self, ato: Ato) -> Result<(), ErroApi> {
        if self.perfis.iter().any(|p| ato.concedido_a(p)) {
            Ok(())
        } else {
            Err(ErroApi::proibido())
        }
    }

    #[must_use]
    pub fn pode(&self, ato: Ato) -> bool {
        self.exigir(ato).is_ok()
    }

    #[must_use]
    pub fn contexto(&self) -> ContextoBanco {
        ContextoBanco {
            usuario_id: Some(self.id),
            unidade_id: self.unidade_id,
            perfis: self.perfis.clone(),
        }
    }
}

fn aleatorio(bytes: usize) -> String {
    let mut buf = vec![0_u8; bytes];
    rand::rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn hash(segredo: &str) -> Vec<u8> {
    Sha256::digest(segredo.as_bytes()).to_vec()
}

pub struct SessaoNova {
    pub segredo: String,
    pub csrf: String,
}

/// Cria a sessão de um usuário já autenticado (pelo OIDC).
///
/// # Errors
/// Falha de banco.
pub async fn criar(
    banco: &PgPool,
    usuario_id: Uuid,
    unidade_id: Uuid,
    correlacao: &str,
) -> Result<SessaoNova, sqlx::Error> {
    let segredo = aleatorio(32);
    let csrf = aleatorio(24);
    sqlx::query!(
        "INSERT INTO identidade.sessao
           (id, usuario_id, segredo_hash, csrf, expira_em, origem, unidade_id, correlacao, criado_por)
         VALUES ($1, $2, $3, $4, now() + make_interval(hours => $5), 'login', $6, $7, $2)",
        vidacor_comum::novo_id(),
        usuario_id,
        hash(&segredo),
        csrf,
        i32::try_from(DURACAO_HORAS).unwrap_or(10),
        unidade_id,
        correlacao
    )
    .execute(banco)
    .await?;
    Ok(SessaoNova { segredo, csrf })
}

/// Valor do cabeçalho `Set-Cookie` da sessão.
#[must_use]
pub fn cookie(segredo: &str, seguro: bool) -> String {
    let seguro = if seguro { "; Secure" } else { "" };
    format!(
        "{COOKIE}={segredo}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{seguro}",
        DURACAO_HORAS * 3600
    )
}

#[must_use]
pub fn cookie_apagado(seguro: bool) -> String {
    let seguro = if seguro { "; Secure" } else { "" };
    format!("{COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{seguro}")
}

fn ler_cookie(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .map(str::trim)
        .find_map(|par| par.strip_prefix(&format!("{COOKIE}=")).map(str::to_owned))
        .filter(|v| !v.is_empty() && v.len() <= 128)
}

/// Revoga a sessão (logout).
///
/// # Errors
/// Falha de banco.
pub async fn revogar(banco: &PgPool, sessao_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE identidade.sessao SET revogada_em = now() WHERE id = $1 AND revogada_em IS NULL",
        sessao_id
    )
    .execute(banco)
    .await?;
    Ok(())
}

impl FromRequestParts<Estado> for Usuario {
    type Rejection = ErroApi;

    async fn from_request_parts(
        parts: &mut Parts,
        estado: &Estado,
    ) -> Result<Self, Self::Rejection> {
        let segredo = ler_cookie(parts).ok_or_else(ErroApi::nao_autenticado)?;
        let linha = sqlx::query!(
            r#"SELECT s.id AS sessao_id, s.csrf, s.reautenticado_em, u.id, u.nome, u.perfis, u.unidade_id
                 FROM identidade.sessao s
                 JOIN identidade.usuario u ON u.id = s.usuario_id
                WHERE s.segredo_hash = $1 AND s.revogada_em IS NULL
                  AND s.expira_em > now() AND u.ativo"#,
            hash(&segredo)
        )
        .fetch_optional(&estado.banco)
        .await?
        .ok_or_else(ErroApi::nao_autenticado)?;

        let leitura = matches!(parts.method, Method::GET | Method::HEAD | Method::OPTIONS);
        if !leitura {
            let enviado = parts
                .headers
                .get(CABECALHO_CSRF)
                .map(|v| v.as_bytes().to_vec())
                .unwrap_or_default();
            if !bool::from(enviado.ct_eq(linha.csrf.as_bytes())) {
                return Err(ErroApi::csrf());
            }
        }

        Ok(Self {
            id: linha.id,
            nome: linha.nome,
            perfis: linha.perfis,
            unidade_id: linha.unidade_id,
            sessao_id: linha.sessao_id,
            csrf: linha.csrf,
            reautenticado_em: linha.reautenticado_em,
        })
    }
}
