//! Etiquetas e rastreio (Etiquetas.dc.html, docs/20): painel do dia com contadores, etiquetas com o
//! último evento e as divergências abertas, e o histórico de cada etiqueta montado do lastro. Como a
//! central de etiquetas da Medware (impressão na recepção, bipe na sala), mas com cada passo no lastro
//! (DEC-2026-11-09-128). Só nome, nascimento e códigos: nunca diagnóstico (docs/20).

use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::rotas::pacientes_lista::ve_pacientes;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Deserialize, IntoParams)]
pub struct FiltroDeEtiquetas {
    /// Código da etiqueta (bipado), nome do paciente (3+ letras, sem acento) ou número do exame (ACC).
    pub q: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct PacienteDaEtiqueta {
    pub id: Uuid,
    pub nome: String,
    pub nascimento: NaiveDate,
}

#[derive(Serialize, ToSchema)]
pub struct UltimoEvento {
    pub descricao: String,
    pub em: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct LinhaDeEtiqueta {
    pub codigo: String,
    /// pulseira.
    pub tipo: String,
    /// impressa (aguardando a sala), em_uso (bipada), invalidada ou divergencia (bipada na sala errada, aberta).
    pub estado: String,
    pub paciente: PacienteDaEtiqueta,
    pub exame: String,
    pub sala: String,
    /// Número do exame (accession) que liga etiqueta, aparelho e imagens.
    pub accession: String,
    pub motivo_invalidacao: Option<String>,
    pub substituida_por: Option<String>,
    /// Etiqueta que esta substituiu (reimpressão).
    pub substitui: Option<String>,
    pub ultimo_evento: Option<UltimoEvento>,
}

#[derive(Serialize, ToSchema)]
pub struct DivergenciaDeSala {
    pub id: Uuid,
    pub codigo: String,
    pub etiqueta: String,
    pub paciente: PacienteDaEtiqueta,
    pub exame: String,
    pub sala_bipada: String,
    pub sala_correta: String,
    pub sala_correta_id: Uuid,
    pub aberta_em: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct PainelDeEtiquetas {
    pub dia: NaiveDate,
    pub impressas: i64,
    pub bipadas: i64,
    pub reimpressoes: i64,
    pub invalidadas: i64,
    pub divergencias_abertas: i64,
    pub divergencias: Vec<DivergenciaDeSala>,
    pub etiquetas: Vec<LinhaDeEtiqueta>,
}

#[derive(Serialize, ToSchema)]
pub struct EventoDaEtiqueta {
    /// impressa, invalidada, bipada, bloqueada, resolvida ou outro.
    pub tipo: String,
    pub descricao: String,
    pub em: DateTime<Utc>,
    pub por: Option<String>,
    /// ok (passo normal) ou alerta (bloqueio, invalidação).
    pub nivel: String,
}

#[derive(Serialize, ToSchema)]
pub struct DetalheDaEtiqueta {
    pub etiqueta: LinhaDeEtiqueta,
    pub eventos: Vec<EventoDaEtiqueta>,
}

/// Texto do evento do lastro para a tela (só códigos e salas; o lastro nunca guarda nome nem diagnóstico).
fn descrever(tipo: &str, d: &Value, sala_bipada: Option<&str>) -> (String, String, String) {
    let s = |k: &str| d.get(k).and_then(Value::as_str).unwrap_or("").to_owned();
    match tipo {
        "etiqueta_emitida" => {
            let onde = s("estacao");
            (
                "impressa".into(),
                if onde.is_empty() {
                    "Impressa na recepção".into()
                } else {
                    format!("Impressa no {onde}")
                },
                "ok".into(),
            )
        }
        "etiqueta_reimpressa" => (
            "invalidada".into(),
            format!(
                "Invalidada por reimpressão, motivo: {}; nova etiqueta {}",
                s("motivo"),
                s("nova")
            ),
            "alerta".into(),
        ),
        "etiqueta_invalidada" => (
            "invalidada".into(),
            format!("Invalidada, motivo: {}", s("motivo")),
            "alerta".into(),
        ),
        "pulseira_bipada" => (
            "bipada".into(),
            format!(
                "Bipada na {}, paciente e exame conferem; aparelho {}",
                s("sala"),
                s("aparelho")
            ),
            "ok".into(),
        ),
        "bipe_bloqueado_sala_errada" => (
            "bloqueada".into(),
            format!(
                "Bipada na sala errada{}: exame bloqueado, nada enviado ao aparelho",
                sala_bipada.map(|n| format!(" ({n})")).unwrap_or_default()
            ),
            "alerta".into(),
        ),
        "divergencia_resolvida" => (
            "resolvida".into(),
            format!("Divergência resolvida: {}", s("descricao")),
            "ok".into(),
        ),
        _ => ("outro".into(), "Evento registrado".into(), "ok".into()),
    }
}

/// Painel do dia (hoje, horário da clínica) de quem lida com o paciente.
#[utoipa::path(get, operation_id = "painel_de_etiquetas", path = "/v1/etiquetas", tag = "recepcao", params(FiltroDeEtiquetas),
    responses((status = 200, body = PainelDeEtiquetas), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn painel(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroDeEtiquetas>,
) -> Result<Response, ErroApi> {
    ve_pacientes(&usuario)?;
    let q = f.q.as_deref().map(str::trim).filter(|t| !t.is_empty());
    if q.is_some_and(|t| !(3..=100).contains(&t.chars().count())) {
        return Err(ErroApi::invalido(
            "busca-invalida",
            "Bipe a etiqueta ou digite pelo menos 3 letras do nome ou o código.",
        ));
    }
    let termo = q.map(|t| t.replace(['%', '_'], ""));
    let hoje = tempo::hoje_local();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let c = sqlx::query!(
        r#"SELECT
             (SELECT count(*) FROM recepcao.etiqueta e WHERE (e.criado_em AT TIME ZONE 'America/Sao_Paulo')::date = $1) AS "impressas!",
             (SELECT count(*) FROM lastro.evento v WHERE v.tipo = 'pulseira_bipada' AND (v.criado_em AT TIME ZONE 'America/Sao_Paulo')::date = $1) AS "bipadas!",
             (SELECT count(*) FROM lastro.evento v WHERE v.tipo = 'etiqueta_reimpressa' AND (v.criado_em AT TIME ZONE 'America/Sao_Paulo')::date = $1) AS "reimpressoes!",
             (SELECT count(*) FROM lastro.evento v WHERE v.tipo = 'etiqueta_invalidada' AND (v.criado_em AT TIME ZONE 'America/Sao_Paulo')::date = $1) AS "invalidadas!",
             (SELECT count(*) FROM recepcao.divergencia d WHERE d.tipo = 'pulseira_sala_errada' AND d.situacao = 'aberta') AS "abertas!""#,
        hoje
    )
    .fetch_one(&mut *tx)
    .await?;
    let divergencias = sqlx::query!(
        r#"SELECT d.id, d.codigo, d.dados->>'etiqueta' AS "etiqueta!", d.criado_em, p.id AS paciente_id, p.nome, p.nascimento,
                  ag.exame_nome, sb.nome AS sala_bipada, sc.nome AS sala_correta, sc.id AS sala_correta_id
             FROM recepcao.divergencia d
             JOIN recepcao.atendimento t ON t.id = d.atendimento_id
             JOIN agenda.agendamento ag ON ag.id = t.agendamento_id
             JOIN identidade.paciente p ON p.id = t.paciente_id
             JOIN agenda.sala sb ON sb.id = d.sala_id
             JOIN agenda.sala sc ON sc.id = t.sala_id
            WHERE d.tipo = 'pulseira_sala_errada' AND d.situacao = 'aberta'
            ORDER BY d.criado_em LIMIT 50"#
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|d| DivergenciaDeSala {
        id: d.id,
        codigo: d.codigo,
        etiqueta: d.etiqueta,
        paciente: PacienteDaEtiqueta {
            id: d.paciente_id,
            nome: d.nome,
            nascimento: d.nascimento,
        },
        exame: d.exame_nome,
        sala_bipada: d.sala_bipada,
        sala_correta: d.sala_correta,
        sala_correta_id: d.sala_correta_id,
        aberta_em: d.criado_em,
    })
    .collect();
    let etiquetas = carregar(&mut tx, Some(hoje), termo.as_deref(), None).await?;
    tx.commit().await?;
    let painel = PainelDeEtiquetas {
        dia: hoje,
        impressas: c.impressas,
        bipadas: c.bipadas,
        reimpressoes: c.reimpressoes,
        invalidadas: c.invalidadas,
        divergencias_abertas: c.abertas,
        divergencias,
        etiquetas,
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(painel)).into_response())
}

/// Etiquetas do dia (ou uma, pelo código), com estado e último evento calculados no banco.
pub(crate) async fn carregar(
    tx: &mut sqlx::PgConnection,
    dia: Option<NaiveDate>,
    termo: Option<&str>,
    codigo: Option<&str>,
) -> Result<Vec<LinhaDeEtiqueta>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT e.codigo, e.tipo, e.situacao, e.motivo_invalidacao, e.substituida_por,
                  (SELECT a.codigo FROM recepcao.etiqueta a WHERE a.substituida_por = e.codigo) AS "substitui?",
                  t.situacao AS atendimento_situacao, t.accession, p.id AS paciente_id, p.nome, p.nascimento,
                  ag.exame_nome, s.nome AS sala,
                  EXISTS (SELECT 1 FROM recepcao.divergencia d WHERE d.tipo = 'pulseira_sala_errada' AND d.situacao = 'aberta'
                           AND d.dados->>'etiqueta' = e.codigo) AS "divergencia!",
                  u.tipo AS "ultimo_tipo?", u.dados AS "ultimo_dados?", u.criado_em AS "ultimo_em?", us.nome AS "ultimo_sala?"
             FROM recepcao.etiqueta e
             JOIN recepcao.atendimento t ON t.id = e.atendimento_id
             JOIN agenda.agendamento ag ON ag.id = t.agendamento_id
             JOIN agenda.sala s ON s.id = t.sala_id
             JOIN identidade.paciente p ON p.id = e.paciente_id
             LEFT JOIN LATERAL (SELECT v.tipo, v.dados, v.criado_em FROM lastro.evento v
                                 WHERE (v.registro_id = e.id OR v.dados->>'etiqueta' = e.codigo)
                                   AND v.tipo IN ('etiqueta_emitida','etiqueta_reimpressa','etiqueta_invalidada','pulseira_bipada','bipe_bloqueado_sala_errada','divergencia_resolvida')
                                 ORDER BY v.sequencia DESC LIMIT 1) u ON true
             LEFT JOIN agenda.sala us ON us.id = (u.dados->>'sala_bipada')::uuid
            WHERE ($1::date IS NULL OR (e.criado_em AT TIME ZONE 'America/Sao_Paulo')::date = $1)
              AND ($3::text IS NULL OR e.codigo = $3)
              AND ($2::text IS NULL OR upper(e.codigo) = upper($2) OR t.accession = $2
                   OR comum.sem_acento(p.nome) LIKE '%' || comum.sem_acento($2) || '%')
            ORDER BY e.criado_em DESC LIMIT 200"#,
        dia,
        termo,
        codigo
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| {
            let estado = if l.situacao == "invalidada" {
                "invalidada"
            } else if l.divergencia {
                "divergencia"
            } else if matches!(
                l.atendimento_situacao.as_str(),
                "em_exame" | "exame_concluido"
            ) {
                "em_uso"
            } else {
                "impressa"
            };
            let ultimo_evento = l.ultimo_tipo.as_deref().zip(l.ultimo_em).map(|(t, em)| {
                let (_, descricao, _) = descrever(
                    t,
                    l.ultimo_dados.as_ref().unwrap_or(&Value::Null),
                    l.ultimo_sala.as_deref(),
                );
                UltimoEvento { descricao, em }
            });
            LinhaDeEtiqueta {
                codigo: l.codigo,
                tipo: l.tipo,
                estado: estado.to_owned(),
                paciente: PacienteDaEtiqueta {
                    id: l.paciente_id,
                    nome: l.nome,
                    nascimento: l.nascimento,
                },
                exame: l.exame_nome,
                sala: l.sala,
                accession: l.accession,
                motivo_invalidacao: l.motivo_invalidacao,
                substituida_por: l.substituida_por,
                substitui: l.substitui,
                ultimo_evento,
            }
        })
        .collect())
}

/// Histórico completo de uma etiqueta (qualquer dia), do lastro.
#[utoipa::path(get, operation_id = "detalhe_da_etiqueta", path = "/v1/etiquetas/{codigo}", tag = "recepcao", params(("codigo" = String, Path)),
    responses((status = 200, body = DetalheDaEtiqueta), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn detalhe(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(codigo): Path<String>,
) -> Result<Response, ErroApi> {
    ve_pacientes(&usuario)?;
    let codigo = codigo.trim().to_uppercase();
    if codigo.chars().count() > 40 {
        return Err(ErroApi::nao_encontrado(
            "etiqueta-nao-encontrada",
            "Etiqueta não encontrada nesta unidade.",
        ));
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let etiqueta = carregar(&mut tx, None, None, Some(&codigo))
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| {
            ErroApi::nao_encontrado(
                "etiqueta-nao-encontrada",
                "Etiqueta não encontrada nesta unidade. Confira o código bipado.",
            )
        })?;
    let eventos = sqlx::query!(
        r#"SELECT v.tipo, v.dados, v.criado_em, u.nome AS "por?", sb.nome AS "sala_bipada?"
             FROM lastro.evento v
             JOIN recepcao.etiqueta e ON e.codigo = $1
             LEFT JOIN identidade.usuario u ON u.id = v.criado_por
             LEFT JOIN agenda.sala sb ON sb.id = (v.dados->>'sala_bipada')::uuid
            WHERE (v.registro_id = e.id OR v.dados->>'etiqueta' = e.codigo)
              AND v.tipo IN ('etiqueta_emitida','etiqueta_reimpressa','etiqueta_invalidada','pulseira_bipada','bipe_bloqueado_sala_errada','divergencia_resolvida')
            ORDER BY v.sequencia LIMIT 200"#,
        codigo
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|v| {
        let (tipo, descricao, nivel) = descrever(&v.tipo, &v.dados, v.sala_bipada.as_deref());
        EventoDaEtiqueta {
            tipo,
            descricao,
            em: v.criado_em,
            por: v.por,
            nivel,
        }
    })
    .collect();
    tx.commit().await?;
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(DetalheDaEtiqueta { etiqueta, eventos }),
    )
        .into_response())
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    #[test]
    fn descreve_sem_nome_nem_diagnostico() {
        let (t, d, n) = descrever(
            "etiqueta_reimpressa",
            &json!({ "motivo": "rasgou", "nova": "ETQ-2026-000002" }),
            None,
        );
        assert_eq!((t.as_str(), n.as_str()), ("invalidada", "alerta"));
        assert_eq!(
            d,
            "Invalidada por reimpressão, motivo: rasgou; nova etiqueta ETQ-2026-000002"
        );
        let (_, d, _) = descrever("bipe_bloqueado_sala_errada", &json!({}), Some("Sala 2"));
        assert_eq!(
            d,
            "Bipada na sala errada (Sala 2): exame bloqueado, nada enviado ao aparelho"
        );
        assert_eq!(
            descrever("qualquer", &json!({}), None).1,
            "Evento registrado"
        );
    }
}
