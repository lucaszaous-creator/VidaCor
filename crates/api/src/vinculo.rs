//! Vínculo de cuidado (docs/18 § 6 item 1): o médico acessa dado clínico completo só com vínculo.

use sqlx::PgConnection;
use uuid::Uuid;

/// Dias de validade do vínculo depois do último ato (a confirmar com a clínica, docs/18 § 9).
pub const VALIDADE_DIAS: i32 = 30;

/// Registra ou renova o vínculo do médico com o paciente.
///
/// # Errors
/// Falha de banco.
pub async fn registrar(
    conexao: &mut PgConnection,
    medico_id: Uuid,
    paciente_id: Uuid,
    unidade_id: Uuid,
    motivo: &str,
    correlacao: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO cuidado.vinculo (medico_id, paciente_id, motivo, expira_em, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, now() + make_interval(days => $4), 'nucleo', $5, $6)
         ON CONFLICT (medico_id, paciente_id, motivo)
         DO UPDATE SET expira_em = GREATEST(cuidado.vinculo.expira_em, EXCLUDED.expira_em),
                       versao = cuidado.vinculo.versao + 1",
        medico_id,
        paciente_id,
        motivo,
        VALIDADE_DIAS,
        unidade_id,
        correlacao
    )
    .execute(conexao)
    .await?;
    Ok(())
}

/// O médico tem vínculo ativo ou acesso de emergência com o paciente? (mesma regra do RLS)
///
/// # Errors
/// Falha de banco.
pub async fn pode_ler(conexao: &mut PgConnection, paciente_id: Uuid) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query_scalar!("SELECT cuidado.pode_ler($1)", paciente_id)
            .fetch_one(conexao)
            .await?
            .unwrap_or(false),
    )
}

/// Vínculo de cuidado ativo (sem contar o acesso de emergência).
///
/// # Errors
/// Falha de banco.
pub async fn tem_vinculo(
    conexao: &mut PgConnection,
    paciente_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM cuidado.vinculo WHERE paciente_id = $1
            AND medico_id = NULLIF(current_setting('app.usuario_id', true), '')::uuid AND expira_em > now()) AS \"tem!\"",
        paciente_id
    )
    .fetch_one(conexao)
    .await
}
