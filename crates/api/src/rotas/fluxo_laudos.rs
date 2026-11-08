//! Fluxo de laudos (Fluxo-Laudos.dc.html; docs/04): quadro por etapa, prazo e produtividade por
//! médico para a coordenação. Só metadados (funções `laudos.quadro` e `laudos.retificacoes`, 0051):
//! o texto do laudo nunca sai daqui. Prazo padrão de 24 h desde o recebimento das imagens, a
//! confirmar com a clínica (DEC-2026-11-08-96).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Etapas na ordem do quadro.
pub const ETAPAS: [&str; 5] = [
    "a_laudar",
    "em_retificacao",
    "aguardando_conversa",
    "assinado",
    "entregue",
];

#[derive(Serialize, ToSchema)]
pub struct CartaoDoFluxo {
    pub laudo_id: Uuid,
    pub codigo: String,
    pub etapa: String,
    pub paciente_nome: String,
    pub paciente_nascimento: NaiveDate,
    pub exame: String,
    pub modalidade: String,
    pub medico: Option<String>,
    pub recebido_em: DateTime<Utc>,
    pub prazo_em: DateTime<Utc>,
    pub assinado_em: Option<DateTime<Utc>>,
    pub versao_vigente: Option<i32>,
    pub atrasado: bool,
}

#[derive(Serialize, ToSchema)]
pub struct EtapaDoFluxo {
    pub etapa: String,
    pub quantidade: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ProducaoDoMedico {
    pub medico: String,
    pub assinados_hoje: i64,
    /// Horas do recebimento das imagens à assinatura, nos assinados hoje.
    pub tempo_medio_horas: Option<f64>,
    pub na_fila: i64,
}

#[derive(Serialize, ToSchema)]
pub struct RetificacaoRecente {
    pub laudo_id: Uuid,
    pub codigo: String,
    pub paciente_nome: String,
    pub exame: String,
    pub versao: i32,
    pub assinada_em: Option<DateTime<Utc>>,
    pub motivo: Option<String>,
    pub medico: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct FluxoDeLaudos {
    pub prazo_horas: i64,
    pub etapas: Vec<EtapaDoFluxo>,
    pub no_prazo_percentual: Option<i64>,
    pub atrasados: i64,
    pub vencem_em_2h: i64,
    pub cartoes: Vec<CartaoDoFluxo>,
    pub producao: Vec<ProducaoDoMedico>,
    pub retificacoes: Vec<RetificacaoRecente>,
}

/// Quadro do fluxo de laudos de hoje e o que segue aberto (coordenação).
#[utoipa::path(get, operation_id = "fluxo_de_laudos", path = "/v1/laudos/fluxo", tag = "laudos",
    responses((status = 200, body = FluxoDeLaudos), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn fluxo(State(estado): State<Estado>, usuario: Usuario) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::FluxoLaudosVer)?;
    let (inicio, fim) = tempo::limites_do_dia(tempo::hoje_local())
        .ok_or_else(|| ErroApi::interno("dia sem limites"))?;
    let agora = tempo::agora();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let linhas = sqlx::query!(
        r#"SELECT laudo_id AS "laudo_id!", codigo AS "codigo!", etapa AS "etapa!", paciente_nome AS "paciente_nome!",
                  paciente_nascimento AS "paciente_nascimento!", exame AS "exame!", modalidade AS "modalidade!", medico_nome,
                  recebido_em AS "recebido_em!", prazo_em AS "prazo_em!", assinado_em, versao_vigente, atrasado AS "atrasado!"
             FROM laudos.quadro($1, $2)"#,
        inicio,
        fim
    )
    .fetch_all(&mut *tx)
    .await?;
    let retificacoes = sqlx::query!(
        r#"SELECT laudo_id AS "laudo_id!", codigo AS "codigo!", paciente_nome AS "paciente_nome!", exame AS "exame!",
                  numero AS "numero!", assinada_em, motivo, medico_nome FROM laudos.retificacoes($1)"#,
        agora - Duration::days(7)
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    let cartoes: Vec<CartaoDoFluxo> = linhas
        .into_iter()
        .map(|l| CartaoDoFluxo {
            laudo_id: l.laudo_id,
            codigo: l.codigo,
            etapa: l.etapa,
            paciente_nome: l.paciente_nome,
            paciente_nascimento: l.paciente_nascimento,
            exame: l.exame,
            modalidade: l.modalidade,
            medico: l.medico_nome,
            recebido_em: l.recebido_em,
            prazo_em: l.prazo_em,
            assinado_em: l.assinado_em,
            versao_vigente: l.versao_vigente,
            atrasado: l.atrasado,
        })
        .collect();
    let etapas = ETAPAS
        .iter()
        .map(|e| EtapaDoFluxo {
            etapa: (*e).to_owned(),
            quantidade: cartoes.iter().filter(|c| c.etapa == *e).count() as i64,
        })
        .collect();
    let abertos = |c: &&CartaoDoFluxo| matches!(c.etapa.as_str(), "a_laudar" | "em_retificacao");
    let atrasados = cartoes
        .iter()
        .filter(abertos)
        .filter(|c| c.atrasado)
        .count() as i64;
    let vencem = cartoes
        .iter()
        .filter(abertos)
        .filter(|c| !c.atrasado && c.prazo_em <= agora + Duration::hours(2))
        .count() as i64;
    // No prazo: assinados hoje dentro de 24 h do recebimento.
    let assinados_hoje: Vec<&CartaoDoFluxo> = cartoes
        .iter()
        .filter(|c| c.assinado_em.is_some_and(|a| a >= inicio && a < fim))
        .collect();
    let no_prazo = (!assinados_hoje.is_empty()).then(|| {
        let ok = assinados_hoje
            .iter()
            .filter(|c| c.assinado_em.is_some_and(|a| a <= c.prazo_em))
            .count();
        (ok * 100 / assinados_hoje.len()) as i64
    });
    let mut por_medico: BTreeMap<String, (i64, f64, i64)> = BTreeMap::new();
    for c in &cartoes {
        let nome = c
            .medico
            .clone()
            .unwrap_or_else(|| "Sem médico atribuído".to_owned());
        let e = por_medico.entry(nome).or_insert((0, 0.0, 0));
        if let Some(a) = c.assinado_em.filter(|a| *a >= inicio && *a < fim) {
            e.0 += 1;
            e.1 += (a - c.recebido_em).num_minutes() as f64 / 60.0;
        }
        if matches!(c.etapa.as_str(), "a_laudar" | "em_retificacao") {
            e.2 += 1;
        }
    }
    let producao = por_medico
        .into_iter()
        .map(|(medico, (n, horas, fila))| ProducaoDoMedico {
            medico,
            assinados_hoje: n,
            tempo_medio_horas: (n > 0).then(|| (horas / n as f64 * 10.0).round() / 10.0),
            na_fila: fila,
        })
        .collect();
    let corpo = FluxoDeLaudos {
        prazo_horas: 24,
        etapas,
        no_prazo_percentual: no_prazo,
        atrasados,
        vencem_em_2h: vencem,
        cartoes,
        producao,
        retificacoes: retificacoes
            .into_iter()
            .map(|r| RetificacaoRecente {
                laudo_id: r.laudo_id,
                codigo: r.codigo,
                paciente_nome: r.paciente_nome,
                exame: r.exame,
                versao: r.numero,
                assinada_em: r.assinada_em,
                motivo: r.motivo,
                medico: r.medico_nome,
            })
            .collect(),
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(corpo)).into_response())
}
