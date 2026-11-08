//! Eventos de lastro imutáveis com hash encadeado (docs/21). Os dados do evento guardam ids e
//! códigos, nunca nome ou conteúdo clínico.

use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

pub struct NovoEvento<'a> {
    pub dominio: &'a str,
    pub tipo: &'a str,
    pub registro_id: Uuid,
    pub registro_codigo: Option<&'a str>,
    pub dados: Value,
    pub criado_por: Option<Uuid>,
    pub origem: &'a str,
    pub unidade_id: Uuid,
    pub correlacao: &'a str,
}

/// # Errors
/// Falha de banco.
pub async fn registrar(conexao: &mut PgConnection, e: NovoEvento<'_>) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO lastro.evento
           (id, dominio, tipo, registro_id, registro_codigo, dados, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        vidacor_comum::novo_id(),
        e.dominio,
        e.tipo,
        e.registro_id,
        e.registro_codigo,
        e.dados,
        e.criado_por,
        e.origem,
        e.unidade_id,
        e.correlacao
    )
    .execute(conexao)
    .await?;
    Ok(())
}
