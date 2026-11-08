//! Regras do laudo que rodam no núcleo (sem rota): criação do laudo quando o estudo é conciliado e
//! publicação idempotente no portal (docs/27 FC-LAU-03, 09; CLAUDE.md: laudo só vai ao portal
//! depois de assinado).

use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::lastro::{self, NovoEvento};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgConnection;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};

/// Hash do documento do laudo: SHA-256 da forma canônica (paciente, exame, versão e texto).
/// O PDF com Typst entra depois e passará a ser o documento assinado (docs/11 § 4).
#[must_use]
pub fn hash_documento(
    laudo_codigo: &str,
    numero: i32,
    paciente: &str,
    nascimento: &str,
    exame: &str,
    texto: &str,
) -> String {
    let canonico = format!(
        "VIDACOR-LAUDO-1\n{laudo_codigo}\nv{numero}\n{paciente}\n{nascimento}\n{exame}\n{texto}"
    );
    hex(&Sha256::digest(canonico.as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Cria o laudo em rascunho para um estudo conciliado, atribuído ao médico do agendamento, e grava
/// o vínculo de cuidado. Só estudo conciliado chega aqui (FC-LAU-09). Repetir não duplica.
///
/// # Errors
/// Falha de banco.
pub async fn criar_para_estudo(
    conexao: &mut PgConnection,
    estudo_id: Uuid,
    atendimento_id: Uuid,
    unidade_id: Uuid,
    correlacao: &str,
) -> Result<(), sqlx::Error> {
    let Some(a) = sqlx::query!(
        "SELECT t.paciente_id, ag.medico_id FROM recepcao.atendimento t JOIN agenda.agendamento ag ON ag.id = t.agendamento_id WHERE t.id = $1",
        atendimento_id
    )
    .fetch_optional(&mut *conexao)
    .await?
    else {
        return Ok(());
    };
    let ano = crate::tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero('LAU', $1)", ano)
        .fetch_one(&mut *conexao)
        .await?
        .unwrap_or(0);
    let codigo = CodigoLegivel::novo("LAU", ano, n);
    let id = novo_id();
    let criado = sqlx::query_scalar!(
        "INSERT INTO laudos.laudo (id, codigo, atendimento_id, estudo_id, paciente_id, medico_atribuido_id, situacao, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'rascunho', 'nucleo', $7, $8)
         ON CONFLICT (atendimento_id) DO NOTHING RETURNING id",
        id, codigo.como_str(), atendimento_id, estudo_id, a.paciente_id, a.medico_id, unidade_id, correlacao
    )
    .fetch_optional(&mut *conexao)
    .await?;
    if criado.is_none() {
        return Ok(());
    }
    sqlx::query!(
        "INSERT INTO laudos.versao (id, laudo_id, numero, texto, situacao, origem, unidade_id, correlacao)
         VALUES ($1, $2, 1, '', 'rascunho', 'nucleo', $3, $4)",
        novo_id(), id, unidade_id, correlacao
    )
    .execute(&mut *conexao)
    .await?;
    if let Some(medico) = a.medico_id {
        crate::vinculo::registrar(
            conexao,
            medico,
            a.paciente_id,
            unidade_id,
            "laudo_atribuido",
            correlacao,
        )
        .await?;
    }
    lastro::registrar(
        conexao,
        NovoEvento {
            dominio: "laudos",
            tipo: "laudo_criado",
            registro_id: id,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "estudo": estudo_id, "atribuido": a.medico_id }),
            criado_por: None,
            origem: "nucleo",
            unidade_id,
            correlacao,
        },
    )
    .await?;
    Ok(())
}

/// Publica no portal a versão vigente de um laudo assinado. Repetir não duplica a publicação nem o
/// aviso (FC-LAU-03). Rascunho nunca é publicado (FC-LAU-02); "conversar antes" segura até a liberação.
///
/// # Errors
/// 409 `laudo-nao-assinado` ou `aguarda-conversa`; falha de banco.
pub async fn publicar(estado: &Estado, laudo_id: Uuid, unidade_id: Uuid) -> Result<bool, ErroApi> {
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    let l = sqlx::query!(
        r#"SELECT l.id, l.codigo, l.situacao, l.versao_vigente, l.paciente_id, l.conversar_antes,
                  l.liberado_apos_conversa_em, ag.exame_nome
             FROM laudos.laudo l JOIN recepcao.atendimento t ON t.id = l.atendimento_id
             JOIN agenda.agendamento ag ON ag.id = t.agendamento_id
            WHERE l.id = $1 FOR UPDATE OF l"#,
        laudo_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("laudo-nao-encontrado", "Laudo não encontrado."))?;
    let Some(numero) = l.versao_vigente else {
        return Err(ErroApi::conflito(
            "laudo-nao-assinado",
            "O laudo ainda não foi assinado. Nada foi publicado no portal.",
        ));
    };
    if l.conversar_antes && l.liberado_apos_conversa_em.is_none() {
        return Err(ErroApi::conflito(
            "aguarda-conversa",
            "O laudo está marcado para conversar antes de liberar. Nada foi publicado.",
        ));
    }
    let v = sqlx::query!(
        "SELECT numero, texto, hash_documento FROM laudos.versao WHERE laudo_id = $1 AND numero = $2 AND situacao = 'assinada'",
        l.id, numero
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::conflito("laudo-nao-assinado", "A versão vigente não está assinada. Nada foi publicado."))?;
    sqlx::query!("UPDATE portal.publicacao SET vigente = false WHERE laudo_id = $1 AND versao_numero <> $2 AND vigente", l.id, numero)
        .execute(&mut *tx)
        .await?;
    let publicacao = sqlx::query_scalar!(
        "INSERT INTO portal.publicacao (id, laudo_id, versao_numero, paciente_id, exame, texto, retificado, hash_documento, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'nucleo', $9, 'publicacao')
         ON CONFLICT (laudo_id, versao_numero) DO NOTHING RETURNING id",
        novo_id(), l.id, numero, l.paciente_id, l.exame_nome, v.texto, numero > 1, v.hash_documento.unwrap_or_default(), unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let nova = publicacao.is_some();
    if let Some(pid) = publicacao {
        sqlx::query!("INSERT INTO portal.aviso (id, publicacao_id, canal) VALUES ($1, $2, 'whatsapp') ON CONFLICT (publicacao_id) DO NOTHING", novo_id(), pid)
            .execute(&mut *tx)
            .await?;
        lastro::registrar(
            &mut tx,
            NovoEvento {
                dominio: "portal",
                tipo: "laudo_publicado",
                registro_id: l.id,
                registro_codigo: Some(&l.codigo),
                dados: json!({ "versao": numero, "retificado": numero > 1 }),
                criado_por: None,
                origem: "nucleo",
                unidade_id,
                correlacao: "publicacao",
            },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(nova)
}
