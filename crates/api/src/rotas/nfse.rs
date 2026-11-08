//! NFS-e do recebimento: emissão depois do pagamento e cancelamento depois do estorno aprovado, pelo
//! outbox (G3). Uma nota por recebimento (G2). Descrição sem o nome do exame.

use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::lastro::{self, NovoEvento};
use crate::prefeitura::NotaParaEmitir;
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

fn sistema(unidade_id: Uuid) -> ContextoBanco {
    ContextoBanco {
        usuario_id: None,
        unidade_id,
        perfis: vec!["sistema".into()],
    }
}

/// Trabalhador do outbox: emite a nota do recebimento pago (uma só).
///
/// # Errors
/// Prefeitura não configurada ou fora do ar (o item volta ao outbox com espera).
pub async fn emitir(
    estado: &Estado,
    unidade_id: Uuid,
    recebimento_id: Uuid,
    chave: Uuid,
) -> anyhow::Result<()> {
    let mut tx = banco::transacao(&estado.banco, &sistema(unidade_id)).await?;
    if sqlx::query_scalar!(
        "SELECT id FROM caixa.nota_fiscal WHERE recebimento_id = $1",
        recebimento_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .is_some()
    {
        tx.commit().await?;
        return Ok(());
    }
    let r = sqlx::query!(
        "SELECT r.codigo, r.total_centavos, r.situacao, p.nome, p.cpf FROM caixa.recebimento r JOIN identidade.paciente p ON p.id = r.paciente_id WHERE r.id = $1",
        recebimento_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if r.situacao != "pago" {
        tx.commit().await?;
        return Ok(());
    }
    let prefeitura = estado
        .prefeitura
        .clone()
        .ok_or_else(|| anyhow::anyhow!("prefeitura não configurada"))?;
    let nota = NotaParaEmitir {
        tomador_nome: r.nome,
        tomador_cpf: r.cpf,
        valor_centavos: r.total_centavos,
        descricao: format!("Serviços de diagnóstico por imagem, recibo {}", r.codigo),
    };
    let emitida = prefeitura.emitir(chave, &nota).await?;
    let id = novo_id();
    sqlx::query!(
        "INSERT INTO caixa.nota_fiscal (id, recebimento_id, numero, codigo_verificacao, valor_centavos, situacao, chave_idempotencia, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, 'emitida', $6, 'sistema', $7, $8) ON CONFLICT (recebimento_id) DO NOTHING",
        id, recebimento_id, emitida.numero, emitida.codigo_verificacao, r.total_centavos, chave, unidade_id, chave.to_string()
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "caixa",
            tipo: "nfse_emitida",
            registro_id: recebimento_id,
            registro_codigo: Some(&r.codigo),
            dados: json!({ "numero": emitida.numero }),
            criado_por: None,
            origem: "sistema",
            unidade_id,
            correlacao: &chave.to_string(),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Trabalhador do outbox: cancela a nota do recebimento estornado. Sem nota ainda, não há o que cancelar
/// e a emissão pendente também não sai (o recebimento não está mais pago).
///
/// # Errors
/// Prefeitura não configurada ou fora do ar.
pub async fn cancelar(
    estado: &Estado,
    unidade_id: Uuid,
    recebimento_id: Uuid,
    chave: Uuid,
) -> anyhow::Result<()> {
    let mut tx = banco::transacao(&estado.banco, &sistema(unidade_id)).await?;
    let Some(n) = sqlx::query!(
        "SELECT id, numero, situacao FROM caixa.nota_fiscal WHERE recebimento_id = $1 FOR UPDATE",
        recebimento_id
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        tx.commit().await?;
        return Ok(());
    };
    if n.situacao == "cancelada" {
        tx.commit().await?;
        return Ok(());
    }
    let prefeitura = estado
        .prefeitura
        .clone()
        .ok_or_else(|| anyhow::anyhow!("prefeitura não configurada"))?;
    prefeitura
        .cancelar(chave, &n.numero, "estorno aprovado")
        .await?;
    sqlx::query!(
        "UPDATE caixa.nota_fiscal SET situacao = 'cancelada', cancelada_em = now(), motivo_cancelamento = 'estorno aprovado', versao = versao + 1 WHERE id = $1",
        n.id
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "caixa",
            tipo: "nfse_cancelada",
            registro_id: recebimento_id,
            registro_codigo: Some(&n.numero),
            dados: json!({ "motivo": "estorno aprovado" }),
            criado_por: None,
            origem: "sistema",
            unidade_id,
            correlacao: &chave.to_string(),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct NotaFiscal {
    pub numero: String,
    pub codigo_verificacao: String,
    pub valor_centavos: i64,
    /// emitida ou cancelada.
    pub situacao: String,
    pub emitida_em: DateTime<Utc>,
    pub cancelada_em: Option<DateTime<Utc>>,
}

/// NFS-e do recebimento (404 enquanto não foi emitida).
#[utoipa::path(get, path = "/v1/recebimentos/{id}/nota", tag = "caixa", params(("id" = Uuid, Path)),
    responses((status = 200, body = NotaFiscal), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn nota(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<NotaFiscal>, ErroApi> {
    if !(usuario.pode(Ato::CaixaOperar)
        || usuario.pode(Ato::EstornoAprovar)
        || usuario.pode(Ato::FinanceiroOperar))
    {
        return Err(ErroApi::proibido());
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let n = sqlx::query_as!(
        NotaFiscal,
        "SELECT numero, codigo_verificacao, valor_centavos, situacao, emitida_em, cancelada_em FROM caixa.nota_fiscal WHERE recebimento_id = $1",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("nota-nao-emitida", "A NFS-e ainda não foi emitida. Ela sai sozinha depois do pagamento."))?;
    tx.commit().await?;
    Ok(Json(n))
}
