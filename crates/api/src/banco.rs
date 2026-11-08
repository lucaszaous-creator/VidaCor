//! Transação com contexto para o RLS (docs/18 § 7): `SET LOCAL` por transação, nunca `SET` de
//! sessão, para o contexto não vazar entre requisições no pool.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ContextoBanco {
    pub usuario_id: Option<Uuid>,
    pub unidade_id: Uuid,
    pub perfis: Vec<String>,
}

impl ContextoBanco {
    /// Contexto dos trabalhos internos do núcleo (outbox, conciliação). Nunca vem de login.
    #[must_use]
    pub fn sistema(unidade_id: Uuid) -> Self {
        Self {
            usuario_id: None,
            unidade_id,
            perfis: vec!["sistema".to_owned()],
        }
    }
}

/// Abre uma transação e declara o contexto do usuário para as políticas RLS.
///
/// # Errors
/// Falha de conexão ou de banco.
pub async fn transacao(
    pool: &PgPool,
    ctx: &ContextoBanco,
) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let usuario = ctx.usuario_id.map(|u| u.to_string()).unwrap_or_default();
    // Perfis só com letras e sublinhado: a vírgula separa a lista lida pelas políticas.
    let perfis: Vec<&str> = ctx
        .perfis
        .iter()
        .map(String::as_str)
        .filter(|p| p.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        .collect();
    sqlx::query(
        "SELECT set_config('app.usuario_id', $1, true), \
                set_config('app.unidade_id', $2, true), \
                set_config('app.perfis', $3, true)",
    )
    .bind(usuario)
    .bind(ctx.unidade_id.to_string())
    .bind(perfis.join(","))
    .execute(&mut *tx)
    .await?;
    Ok(tx)
}
