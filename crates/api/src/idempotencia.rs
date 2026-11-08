//! `Idempotency-Key` nas rotas de escrita (G3, REQ-REC-003). A reserva da chave acontece na mesma
//! transação do negócio: uma segunda requisição com a mesma chave espera a primeira terminar e
//! recebe a mesma resposta; se a primeira falhou, a reserva some junto com o rollback.

use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

pub enum Reserva {
    Nova,
    Repetida { status: u16, corpo: Value },
}

/// # Errors
/// Falha de banco.
pub async fn reservar(
    conexao: &mut PgConnection,
    usuario_id: Uuid,
    rota: &str,
    chave: Uuid,
) -> Result<Reserva, sqlx::Error> {
    let inserida = sqlx::query_scalar!(
        "INSERT INTO integracao.idempotencia (usuario_id, rota, chave) VALUES ($1, $2, $3)
         ON CONFLICT DO NOTHING RETURNING 1",
        usuario_id,
        rota,
        chave
    )
    .fetch_optional(&mut *conexao)
    .await?;
    if inserida.is_some() {
        return Ok(Reserva::Nova);
    }
    let anterior = sqlx::query!(
        "SELECT status, resposta FROM integracao.idempotencia
          WHERE usuario_id = $1 AND rota = $2 AND chave = $3",
        usuario_id,
        rota,
        chave
    )
    .fetch_one(&mut *conexao)
    .await?;
    Ok(Reserva::Repetida {
        status: anterior
            .status
            .and_then(|s| u16::try_from(s).ok())
            .unwrap_or(500),
        corpo: anterior.resposta.unwrap_or(Value::Null),
    })
}

/// Grava a resposta da operação junto com o negócio.
///
/// # Errors
/// Falha de banco.
pub async fn concluir(
    conexao: &mut PgConnection,
    usuario_id: Uuid,
    rota: &str,
    chave: Uuid,
    status: u16,
    corpo: &Value,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE integracao.idempotencia SET status = $4, resposta = $5, concluido_em = now()
          WHERE usuario_id = $1 AND rota = $2 AND chave = $3",
        usuario_id,
        rota,
        chave,
        i32::from(status),
        corpo
    )
    .execute(&mut *conexao)
    .await?;
    Ok(())
}
