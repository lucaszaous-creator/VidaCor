//! Minha agenda (Agenda-Medico.dc.html; DEC-2026-11-09-140 a 143): o dia do médico executante com os
//! pacientes agendados com ele, em grupos (a chegar, esperando, em exame, liberados), os laudos que
//! ele tem para assinar, avisos e a semana. Como no App Agenda da Medware, o médico vê a própria
//! agenda; os dois identificadores (nome e nascimento) vêm em cada linha. O texto do laudo não passa
//! por aqui: o link para laudar só aparece quando o RLS deixa o médico ler aquele laudo.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::rotas::agenda::PacienteResumo;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Prazo do laudo desde a chegada das imagens (o mesmo do fluxo de laudos, DEC-2026-11-08-96).
const PRAZO_LAUDO_HORAS: i64 = 24;

#[derive(Deserialize, IntoParams)]
pub struct FiltroMinhaAgenda {
    /// Dia no formato AAAA-MM-DD (horário de Brasília). Sem data: hoje.
    pub data: Option<NaiveDate>,
}

#[derive(Serialize, ToSchema)]
pub struct TurnoNaSala {
    pub sala: String,
    pub inicio: DateTime<Utc>,
    pub fim: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct ItemDaMinhaAgenda {
    pub agendamento_id: Uuid,
    pub codigo: String,
    pub inicio: DateTime<Utc>,
    pub paciente: PacienteResumo,
    pub exame: String,
    pub sala: String,
    /// a_chegar, esperando, em_exame, liberado ou faltou.
    pub grupo: String,
    pub atendimento_id: Option<Uuid>,
    /// Desde quando o paciente espera (check-in), para o grupo `esperando`.
    pub esperando_desde: Option<DateTime<Utc>>,
    pub senha: Option<String>,
    /// Situação do laudo, sem conteúdo.
    pub laudo_situacao: Option<String>,
    /// Presente só quando o médico pode abrir o laudo (vínculo de cuidado, RLS).
    pub laudo_id: Option<Uuid>,
}

#[derive(Serialize, ToSchema)]
pub struct AvisoParaOMedico {
    /// imagens_recebidas ou laudo_vence.
    pub tipo: String,
    pub texto: String,
    pub quando: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct DiaDaMinhaSemana {
    pub data: NaiveDate,
    pub pacientes: i64,
    /// Ausência aprovada que cobre o dia (motivo), quando houver.
    pub ausencia: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct MinhaAgenda {
    pub data: NaiveDate,
    pub medico: String,
    pub turnos: Vec<TurnoNaSala>,
    pub pacientes: i64,
    pub ja_atendidos: i64,
    pub atendidos_com_laudo_pendente: i64,
    pub esperando: i64,
    pub laudos_para_assinar: i64,
    pub laudos_vencem_hoje: i64,
    pub itens: Vec<ItemDaMinhaAgenda>,
    pub avisos: Vec<AvisoParaOMedico>,
    pub semana: Vec<DiaDaMinhaSemana>,
}

/// Grupo da linha pela situação do agendamento e do atendimento (puro, testado abaixo).
#[must_use]
pub fn grupo(agendamento: &str, atendimento: Option<&str>) -> &'static str {
    match (agendamento, atendimento) {
        ("faltou", _) => "faltou",
        (_, Some("aguardando_sala")) => "esperando",
        (_, Some("em_exame")) => "em_exame",
        (_, Some("exame_concluido")) | ("concluido", _) => "liberado",
        _ => "a_chegar",
    }
}

/// Segunda a sábado da semana do dia (a clínica não abre no domingo).
#[must_use]
pub fn dias_da_semana(dia: NaiveDate) -> Vec<NaiveDate> {
    let segunda = dia - Duration::days(i64::from(dia.weekday().num_days_from_monday()));
    (0..6).map(|i| segunda + Duration::days(i)).collect()
}

fn nome_do_motivo(m: &str) -> &'static str {
    match m {
        "congresso" => "Congresso ou curso",
        "ferias" => "Férias",
        "licenca" => "Licença médica",
        _ => "Compromisso pessoal",
    }
}

/// A agenda do dia do médico da sessão (só o próprio médico; ninguém lê a agenda de outro por aqui).
#[utoipa::path(get, operation_id = "minha_agenda", path = "/v1/minha-agenda", tag = "agenda", params(FiltroMinhaAgenda),
    responses((status = 200, body = MinhaAgenda), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn obter(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(filtro): Query<FiltroMinhaAgenda>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::MinhaAgendaVer)?;
    let dia = filtro.data.unwrap_or_else(tempo::hoje_local);
    let (de, ate) = tempo::limites_do_dia(dia).ok_or_else(|| {
        ErroApi::invalido(
            "data-invalida",
            "A data pedida não é válida. Nada foi alterado. Escolha outro dia.",
        )
    })?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let linhas = sqlx::query!(
        r#"SELECT a.id, a.codigo, a.inicio, a.fim, a.situacao, a.exame_nome, s.nome AS sala,
                  p.id AS paciente_id, p.nome AS paciente_nome, p.nascimento,
                  t.id AS "atendimento_id?", t.situacao AS "atendimento_situacao?", t.criado_em AS "checkin_em?", t.senha,
                  laudos.situacao_do_atendimento(t.id) AS laudo_situacao,
                  (SELECT l.id FROM laudos.laudo l WHERE l.atendimento_id = t.id) AS laudo_id
             FROM agenda.agendamento a
             JOIN agenda.sala s ON s.id = a.sala_id
             JOIN identidade.paciente p ON p.id = a.paciente_id
             LEFT JOIN recepcao.atendimento t ON t.agendamento_id = a.id
            WHERE a.medico_id = $1 AND a.inicio >= $2 AND a.inicio < $3
              AND a.situacao NOT IN ('cancelado','expirada')
            ORDER BY a.inicio
            LIMIT 200"#,
        usuario.id,
        de,
        ate
    )
    .fetch_all(&mut *tx)
    .await?;

    let mut turnos: Vec<TurnoNaSala> = Vec::new();
    for l in &linhas {
        match turnos.iter_mut().find(|t| t.sala == l.sala) {
            Some(t) => {
                t.inicio = t.inicio.min(l.inicio);
                t.fim = t.fim.max(l.fim);
            }
            None => turnos.push(TurnoNaSala {
                sala: l.sala.clone(),
                inicio: l.inicio,
                fim: l.fim,
            }),
        }
    }
    let itens: Vec<ItemDaMinhaAgenda> = linhas
        .into_iter()
        .map(|l| {
            let g = grupo(&l.situacao, l.atendimento_situacao.as_deref());
            ItemDaMinhaAgenda {
                agendamento_id: l.id,
                codigo: l.codigo,
                inicio: l.inicio,
                paciente: PacienteResumo {
                    id: l.paciente_id,
                    nome: l.paciente_nome,
                    nascimento: l.nascimento,
                },
                exame: l.exame_nome,
                sala: l.sala,
                grupo: g.to_owned(),
                atendimento_id: l.atendimento_id,
                esperando_desde: (g == "esperando").then_some(l.checkin_em).flatten(),
                senha: l.senha,
                laudo_situacao: l.laudo_situacao,
                laudo_id: l.laudo_id,
            }
        })
        .collect();
    let contar =
        |g: &str| i64::try_from(itens.iter().filter(|i| i.grupo == g).count()).unwrap_or(0);
    let ja_atendidos = contar("liberado");
    let atendidos_com_laudo_pendente = i64::try_from(
        itens
            .iter()
            .filter(|i| i.grupo == "liberado" && i.laudo_situacao.as_deref() != Some("assinado"))
            .count(),
    )
    .unwrap_or(0);

    // Laudos atribuídos a mim ainda não assinados (o RLS só mostra os que posso ler).
    let laudos = sqlx::query!(
        r#"SELECT l.codigo, p.nome AS paciente, e.recebido_em
             FROM laudos.laudo l
             JOIN exames.estudo e ON e.id = l.estudo_id
             JOIN identidade.paciente p ON p.id = l.paciente_id
            WHERE l.medico_atribuido_id = $1 AND l.situacao IN ('rascunho','retificacao_em_rascunho')
            ORDER BY e.recebido_em LIMIT 200"#,
        usuario.id
    )
    .fetch_all(&mut *tx)
    .await?;
    let prazo = Duration::hours(PRAZO_LAUDO_HORAS);
    let laudos_vencem_hoje = i64::try_from(
        laudos
            .iter()
            .filter(|l| l.recebido_em + prazo < ate)
            .count(),
    )
    .unwrap_or(0);
    let agora = tempo::agora();
    let mut avisos: Vec<AvisoParaOMedico> = laudos
        .iter()
        .filter(|l| {
            l.recebido_em + prazo < ate
                && l.recebido_em + prazo > agora - Duration::hours(PRAZO_LAUDO_HORAS)
        })
        .take(5)
        .map(|l| AvisoParaOMedico {
            tipo: "laudo_vence".into(),
            texto: format!(
                "Laudo de {} vence no prazo de {PRAZO_LAUDO_HORAS} h",
                l.paciente
            ),
            quando: l.recebido_em + prazo,
        })
        .collect();
    let imagens = sqlx::query!(
        r#"SELECT p.nome AS paciente, e.imagens, e.recebido_em
             FROM exames.estudo e
             JOIN recepcao.atendimento t ON t.id = e.atendimento_id
             JOIN agenda.agendamento a ON a.id = t.agendamento_id
             JOIN identidade.paciente p ON p.id = t.paciente_id
            WHERE a.medico_id = $1 AND e.conciliado AND e.recebido_em >= $2 AND e.recebido_em < $3
            ORDER BY e.recebido_em DESC LIMIT 5"#,
        usuario.id,
        de,
        ate
    )
    .fetch_all(&mut *tx)
    .await?;
    avisos.extend(imagens.into_iter().map(|i| AvisoParaOMedico {
        tipo: "imagens_recebidas".into(),
        texto: format!("Imagens de {} chegaram ({} imagens)", i.paciente, i.imagens),
        quando: i.recebido_em,
    }));
    avisos.sort_by_key(|a| std::cmp::Reverse(a.quando));

    let dias = dias_da_semana(dia);
    let mut semana = Vec::with_capacity(dias.len());
    for d in dias {
        let Some((i, f)) = tempo::limites_do_dia(d) else {
            continue;
        };
        let n = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM agenda.agendamento
                WHERE medico_id = $1 AND inicio >= $2 AND inicio < $3 AND situacao NOT IN ('cancelado','expirada')"#,
            usuario.id,
            i,
            f
        )
        .fetch_one(&mut *tx)
        .await?;
        let ausencia = sqlx::query_scalar!(
            "SELECT motivo FROM agenda.ausencia_medico
              WHERE medico_id = $1 AND situacao = 'aprovada' AND tstzrange(inicio, fim) && tstzrange($2, $3)
              ORDER BY inicio LIMIT 1",
            usuario.id,
            i,
            f
        )
        .fetch_optional(&mut *tx)
        .await?;
        semana.push(DiaDaMinhaSemana {
            data: d,
            pacientes: n,
            ausencia: ausencia.map(|m| nome_do_motivo(&m).to_owned()),
        });
    }
    tx.commit().await?;
    let corpo = MinhaAgenda {
        data: dia,
        medico: usuario.nome.clone(),
        turnos,
        pacientes: i64::try_from(itens.iter().filter(|i| i.grupo != "faltou").count()).unwrap_or(0),
        ja_atendidos,
        atendidos_com_laudo_pendente,
        esperando: contar("esperando"),
        laudos_para_assinar: i64::try_from(laudos.len()).unwrap_or(0),
        laudos_vencem_hoje,
        itens,
        avisos,
        semana,
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(corpo)).into_response())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn grupos_pela_situacao() {
        assert_eq!(grupo("confirmado", None), "a_chegar");
        assert_eq!(grupo("agendado", None), "a_chegar");
        assert_eq!(grupo("confirmado", Some("aguardando_sala")), "esperando");
        assert_eq!(grupo("em_atendimento", Some("em_exame")), "em_exame");
        assert_eq!(grupo("em_atendimento", Some("exame_concluido")), "liberado");
        assert_eq!(grupo("concluido", None), "liberado");
        assert_eq!(grupo("faltou", None), "faltou");
    }

    #[test]
    fn semana_de_segunda_a_sabado() {
        let quarta = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap_or_default();
        let dias = dias_da_semana(quarta);
        assert_eq!(dias.len(), 6);
        assert_eq!(
            dias[0],
            NaiveDate::from_ymd_opt(2026, 11, 9).unwrap_or_default()
        );
        assert_eq!(
            dias[5],
            NaiveDate::from_ymd_opt(2026, 11, 14).unwrap_or_default()
        );
    }
}
