//! O que o caixa tem a receber no dia (BUG-2026-001). O caixa não lê a agenda (o exame pedido é N3,
//! docs/18 § 2 e § 4): o servidor lê em contexto de sistema e devolve só o necessário para cobrar,
//! paciente (N1), horário (N0) e os procedimentos pela tabela de preços (N2, como no recibo).

use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::rotas::agenda::PacienteResumo;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProcedimentoACobrar {
    pub tuss: String,
    /// Descrição da tabela de preços (a mesma do recibo).
    pub descricao: String,
    /// Nulo quando o procedimento não está na tabela vigente.
    pub valor_centavos: Option<i64>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AtendimentoAReceber {
    pub atendimento_id: Uuid,
    pub codigo: String,
    pub paciente: PacienteResumo,
    pub inicio: DateTime<Utc>,
    pub procedimentos: Vec<ProcedimentoACobrar>,
    /// Já há recebimento pago ou aguardando para este atendimento.
    pub recebido: bool,
}

#[derive(Deserialize, IntoParams)]
pub struct DiaDoCaixa {
    /// AAAA-MM-DD; padrão: hoje.
    pub data: Option<NaiveDate>,
}

/// Atendimentos do dia com check-in, para receber (só o caixa).
#[utoipa::path(get, path = "/v1/caixa/a-receber", tag = "caixa", params(DiaDoCaixa),
    responses((status = 200, body = Vec<AtendimentoAReceber>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn a_receber(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(q): Query<DiaDoCaixa>,
) -> Result<Json<Vec<AtendimentoAReceber>>, ErroApi> {
    usuario.exigir(Ato::CaixaOperar)?;
    let dia = q.data.unwrap_or_else(tempo::hoje_local);
    let (de, ate) = tempo::limites_do_dia(dia)
        .ok_or_else(|| ErroApi::invalido("data-invalida", "Data inválida."))?;
    // Leitura em contexto de sistema da mesma unidade; a resposta não leva o nome do exame da agenda.
    let sistema = ContextoBanco {
        usuario_id: Some(usuario.id),
        unidade_id: usuario.unidade_id,
        perfis: vec!["sistema".into()],
    };
    let mut tx = banco::transacao(&estado.banco, &sistema).await?;
    let linhas = sqlx::query!(
        r#"SELECT a.id, a.codigo, g.inicio, p.id AS paciente_id, p.nome, p.nascimento,
                  coalesce((SELECT array_agg(ap.tuss ORDER BY ap.tuss) FROM agenda.agendamento_procedimento ap WHERE ap.agendamento_id = g.id),
                           ARRAY[g.exame_tuss]) AS "tuss!",
                  EXISTS (SELECT 1 FROM caixa.recebimento r WHERE r.atendimento_id = a.id AND r.situacao IN ('pago','aguardando_pagamento')) AS "recebido!"
           FROM recepcao.atendimento a
           JOIN agenda.agendamento g ON g.id = a.agendamento_id
           JOIN identidade.paciente p ON p.id = a.paciente_id
           WHERE a.unidade_id = $1 AND g.inicio >= $2 AND g.inicio < $3 AND a.situacao <> 'cancelado'
           ORDER BY g.inicio"#,
        usuario.unidade_id,
        de,
        ate
    )
    .fetch_all(&mut *tx)
    .await?;
    let tabela = sqlx::query_scalar!(
        "SELECT id FROM financeiro.tabela_preco WHERE unidade_id = $1 AND vigente_desde <= $2 ORDER BY versao DESC LIMIT 1",
        usuario.unidade_id,
        dia
    )
    .fetch_optional(&mut *tx)
    .await?;
    let precos = sqlx::query!(
        "SELECT pc.tuss, pc.nome, pr.valor_centavos AS \"valor?\" FROM agenda.procedimento pc
           LEFT JOIN financeiro.preco pr ON pr.tuss = pc.tuss AND pr.tabela_id = $1",
        tabela
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        linhas
            .into_iter()
            .map(|l| AtendimentoAReceber {
                atendimento_id: l.id,
                codigo: l.codigo,
                paciente: PacienteResumo {
                    id: l.paciente_id,
                    nome: l.nome,
                    nascimento: l.nascimento,
                },
                inicio: l.inicio,
                recebido: l.recebido,
                procedimentos: l
                    .tuss
                    .into_iter()
                    .map(|t| {
                        let p = precos.iter().find(|p| p.tuss == t);
                        ProcedimentoACobrar {
                            descricao: p
                                .map_or_else(|| format!("Procedimento {t}"), |p| p.nome.clone()),
                            valor_centavos: p.and_then(|p| p.valor),
                            tuss: t,
                        }
                    })
                    .collect(),
            })
            .collect(),
    ))
}
