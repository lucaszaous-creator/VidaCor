//! Gestão de dose (Gestao-Dose.dc.html; DEC-2026-11-09-144 a 149): exames de TC e raio X com a dose
//! lida do RDSR e a comparação com o NRD da clínica, justificativa da dose acima do NRD, histórico do
//! paciente com a dose efetiva estimada e o agregado por protocolo. Dose por paciente é dado de saúde:
//! o médico vê com vínculo de cuidado (RLS); a gerência vê só o agregado e publica o NRD.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::PacienteResumo;
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use vidacor_comum::novo_id;

const MOTIVOS: [&str; 5] = [
    "imc_alto",
    "repeticao_movimento",
    "fase_extra",
    "implante_metalico",
    "outro",
];
const MODALIDADES_TC: [&str; 1] = ["CT"];

#[derive(Deserialize, IntoParams)]
pub struct FiltroDoses {
    /// Primeiro dia (AAAA-MM-DD). Sem data: 7 dias atrás.
    pub de: Option<NaiveDate>,
    /// Último dia, inclusive (AAAA-MM-DD). Sem data: hoje. Até 92 dias de período.
    pub ate: Option<NaiveDate>,
}

#[derive(Serialize, ToSchema)]
pub struct NrdAplicado {
    /// ctdivol, dlp ou dap.
    pub grandeza: String,
    pub valor: f64,
}

#[derive(Serialize, ToSchema)]
pub struct JustificativaDeDose {
    /// imc_alto, repeticao_movimento, fase_extra, implante_metalico ou outro.
    pub motivo: String,
    pub detalhe: Option<String>,
    pub por: String,
    pub em: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct ExameComDose {
    pub dose_id: Uuid,
    pub codigo: String,
    pub paciente: PacienteResumo,
    /// Menor de 18 anos na data do exame (atenção redobrada à dose).
    pub menor_de_idade: bool,
    pub exame: String,
    pub modalidade: String,
    pub aparelho: Option<String>,
    pub realizado_em: DateTime<Utc>,
    pub ctdivol_mgy: Option<f64>,
    pub dlp_mgycm: Option<f64>,
    pub dap_gycm2: Option<f64>,
    pub nrd: Option<NrdAplicado>,
    pub valor_comparado: Option<f64>,
    pub acima_do_nrd: bool,
    pub justificativa: Option<JustificativaDeDose>,
}

#[derive(Serialize, ToSchema)]
pub struct ExamesComDose {
    pub de: NaiveDate,
    pub ate: NaiveDate,
    pub exames: Vec<ExameComDose>,
    pub acima_do_nrd: i64,
    pub sem_justificativa: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ExameDoHistorico {
    pub exame: String,
    pub aparelho: Option<String>,
    pub realizado_em: DateTime<Utc>,
    pub dlp_mgycm: Option<f64>,
    pub dap_gycm2: Option<f64>,
    /// Estimativa (mSv) pelo fator do NRD vigente do exame; ausente sem fator cadastrado.
    pub dose_efetiva_msv: Option<f64>,
}

#[derive(Serialize, ToSchema)]
pub struct HistoricoDeDose {
    pub paciente: PacienteResumo,
    pub exames: Vec<ExameDoHistorico>,
    /// Soma das estimativas dos últimos 12 meses nesta clínica (não é dosimetria individual).
    pub dose_efetiva_12_meses_msv: Option<f64>,
    pub exames_sem_estimativa: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ProtocoloDeDose {
    pub exame_tuss: String,
    pub exame: String,
    pub aparelho: String,
    pub grandeza: String,
    pub exames: i64,
    pub mediana: Option<f64>,
    pub nrd: Option<f64>,
    /// Mediana em percentual do NRD.
    pub percentual_do_nrd: Option<i64>,
    pub acima: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct NovaJustificativa {
    /// imc_alto, repeticao_movimento, fase_extra, implante_metalico ou outro.
    pub motivo: String,
    /// Obrigatório em "outro" (5 a 500 caracteres).
    pub detalhe: Option<String>,
}

fn periodo(
    f: &FiltroDoses,
) -> Result<(NaiveDate, NaiveDate, DateTime<Utc>, DateTime<Utc>), ErroApi> {
    let hoje = tempo::hoje_local();
    let ate = f.ate.unwrap_or(hoje);
    let de = f.de.unwrap_or(ate - Duration::days(6));
    let invalido = || {
        ErroApi::invalido(
            "periodo-invalido",
            "O período vai de 1 a 92 dias, com o início antes do fim. Nada foi consultado. Ajuste as datas.",
        )
    };
    if de > ate || (ate - de).num_days() > 91 {
        return Err(invalido());
    }
    let (i, _) = tempo::limites_do_dia(de).ok_or_else(invalido)?;
    let (_, f2) = tempo::limites_do_dia(ate).ok_or_else(invalido)?;
    Ok((de, ate, i, f2))
}

fn menor(nascimento: NaiveDate, em: DateTime<Utc>) -> bool {
    let dia = em.with_timezone(&tempo::FUSO).date_naive();
    nascimento
        .checked_add_months(chrono::Months::new(18 * 12))
        .is_some_and(|maior| maior > dia)
}

/// Dose efetiva estimada pelo fator do exame (puro): DLP na TC, DAP no raio X.
#[must_use]
pub fn dose_efetiva(
    modalidade: &str,
    dlp: Option<f64>,
    dap: Option<f64>,
    fator: Option<f64>,
) -> Option<f64> {
    let k = fator?;
    let base = if MODALIDADES_TC.contains(&modalidade) {
        dlp
    } else {
        dap
    }?;
    Some((base * k * 100.0).round() / 100.0)
}

/// Exames com dose no período (médico; só os pacientes com vínculo de cuidado, pelo RLS).
#[utoipa::path(get, operation_id = "exames_com_dose", path = "/v1/doses", tag = "exames", params(FiltroDoses),
    responses((status = 200, body = ExamesComDose), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroDoses>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::DoseVer)?;
    let (de, ate, i, fim) = periodo(&f)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let linhas = sqlx::query!(
        r#"SELECT d.id, d.codigo, d.modalidade, d.aparelho, d.ctdivol_mgy, d.dlp_mgycm, d.dap_gycm2, d.valor_comparado,
                  d.acima_do_nrd, e.recebido_em, p.id AS paciente_id, p.nome, p.nascimento,
                  coalesce(c.nome, d.exame_tuss) AS "exame!", n.grandeza AS "grandeza?", n.valor AS "nrd_valor?",
                  j.motivo AS "j_motivo?", j.detalhe AS j_detalhe, ju.nome AS "j_por?", j.criado_em AS "j_em?"
             FROM exames.dose d
             JOIN exames.estudo e ON e.id = d.estudo_id
             JOIN identidade.paciente p ON p.id = d.paciente_id
             LEFT JOIN agenda.procedimento c ON c.tuss = d.exame_tuss
             LEFT JOIN exames.nrd n ON n.id = d.nrd_id
             LEFT JOIN exames.dose_justificativa j ON j.dose_id = d.id
             LEFT JOIN identidade.usuario ju ON ju.id = j.criado_por
            WHERE e.recebido_em >= $1 AND e.recebido_em < $2
            ORDER BY d.acima_do_nrd DESC, e.recebido_em DESC
            LIMIT 500"#,
        i,
        fim
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let exames: Vec<ExameComDose> = linhas
        .into_iter()
        .map(|l| ExameComDose {
            dose_id: l.id,
            codigo: l.codigo,
            menor_de_idade: menor(l.nascimento, l.recebido_em),
            paciente: PacienteResumo {
                id: l.paciente_id,
                nome: l.nome,
                nascimento: l.nascimento,
            },
            exame: l.exame,
            modalidade: l.modalidade,
            aparelho: l.aparelho,
            realizado_em: l.recebido_em,
            ctdivol_mgy: l.ctdivol_mgy,
            dlp_mgycm: l.dlp_mgycm,
            dap_gycm2: l.dap_gycm2,
            nrd: l
                .grandeza
                .zip(l.nrd_valor)
                .map(|(grandeza, valor)| NrdAplicado { grandeza, valor }),
            valor_comparado: l.valor_comparado,
            acima_do_nrd: l.acima_do_nrd,
            justificativa: match (l.j_motivo, l.j_por, l.j_em) {
                (Some(motivo), Some(por), Some(em)) => Some(JustificativaDeDose {
                    motivo,
                    detalhe: l.j_detalhe,
                    por,
                    em,
                }),
                _ => None,
            },
        })
        .collect();
    let acima = i64::try_from(exames.iter().filter(|e| e.acima_do_nrd).count()).unwrap_or(0);
    let sem = i64::try_from(
        exames
            .iter()
            .filter(|e| e.acima_do_nrd && e.justificativa.is_none())
            .count(),
    )
    .unwrap_or(0);
    let corpo = ExamesComDose {
        de,
        ate,
        exames,
        acima_do_nrd: acima,
        sem_justificativa: sem,
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(corpo)).into_response())
}

fn paciente_nao_encontrado() -> ErroApi {
    ErroApi::nao_encontrado(
        "paciente-nao-encontrado",
        "Paciente não encontrado ou sem vínculo de atendimento com você. Nada foi mostrado.",
    )
}

/// Histórico de dose do paciente nos últimos 12 meses (médico com vínculo de cuidado).
#[utoipa::path(get, operation_id = "historico_de_dose", path = "/v1/doses/pacientes/{id}", tag = "exames",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = HistoricoDeDose), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn historico(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::DoseVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    // Sem vínculo: mesma resposta de inexistente (sem oráculo de enumeração).
    if !crate::vinculo::pode_ler(&mut tx, id).await? {
        return Err(paciente_nao_encontrado());
    }
    let p = sqlx::query_as!(
        PacienteResumo,
        "SELECT id, nome, nascimento FROM identidade.paciente WHERE id = $1",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(paciente_nao_encontrado)?;
    let linhas = sqlx::query!(
        r#"SELECT d.modalidade, d.aparelho, d.dlp_mgycm, d.dap_gycm2, e.recebido_em,
                  coalesce(c.nome, d.exame_tuss) AS "exame!",
                  (SELECT n.fator_dose_efetiva FROM exames.nrd n WHERE n.exame_tuss = d.exame_tuss AND n.vigente) AS fator
             FROM exames.dose d
             JOIN exames.estudo e ON e.id = d.estudo_id
             LEFT JOIN agenda.procedimento c ON c.tuss = d.exame_tuss
            WHERE d.paciente_id = $1 AND e.recebido_em >= now() - interval '12 months'
            ORDER BY e.recebido_em DESC LIMIT 100"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO auditoria.acesso (id, quem, quem_tipo, recurso, recurso_id, resultado, correlacao)
         VALUES ($1, $2, 'usuario', 'paciente.dose', $3, 'permitido', $4)",
        novo_id(), usuario.id, id, crate::trace::atual().unwrap_or_default()
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let exames: Vec<ExameDoHistorico> = linhas
        .into_iter()
        .map(|l| ExameDoHistorico {
            dose_efetiva_msv: dose_efetiva(&l.modalidade, l.dlp_mgycm, l.dap_gycm2, l.fator),
            exame: l.exame,
            aparelho: l.aparelho,
            realizado_em: l.recebido_em,
            dlp_mgycm: l.dlp_mgycm,
            dap_gycm2: l.dap_gycm2,
        })
        .collect();
    let sem = i64::try_from(
        exames
            .iter()
            .filter(|e| e.dose_efetiva_msv.is_none())
            .count(),
    )
    .unwrap_or(0);
    let total: Option<f64> = exames
        .iter()
        .filter_map(|e| e.dose_efetiva_msv)
        .reduce(|a, b| a + b)
        .map(|t| (t * 100.0).round() / 100.0);
    let corpo = HistoricoDeDose {
        paciente: p,
        exames,
        dose_efetiva_12_meses_msv: total,
        exames_sem_estimativa: sem,
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(corpo)).into_response())
}

/// Dose por protocolo no período, sem paciente (gerência e médicos).
#[utoipa::path(get, operation_id = "dose_por_protocolo", path = "/v1/doses/protocolos", tag = "exames", params(FiltroDoses),
    responses((status = 200, body = Vec<ProtocoloDeDose>), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn protocolos(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroDoses>,
) -> Result<Json<Vec<ProtocoloDeDose>>, ErroApi> {
    usuario.exigir(Ato::DoseAgregadoVer)?;
    let (_, _, i, fim) = periodo(&f)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = sqlx::query!(
        r#"SELECT exame_tuss AS "exame_tuss!", exame AS "exame!", aparelho AS "aparelho!", grandeza AS "grandeza!",
                  exames AS "exames!", mediana, nrd, acima AS "acima!"
             FROM exames.dose_por_protocolo($1, $2)"#,
        i,
        fim
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(
        r.into_iter()
            .map(|l| ProtocoloDeDose {
                percentual_do_nrd: match (l.mediana, l.nrd) {
                    #[allow(clippy::cast_possible_truncation)]
                    (Some(m), Some(n)) if n > 0.0 => Some((m / n * 100.0).round() as i64),
                    _ => None,
                },
                exame_tuss: l.exame_tuss,
                exame: l.exame,
                aparelho: l.aparelho,
                grandeza: l.grandeza,
                exames: l.exames,
                mediana: l.mediana.map(|m| (m * 100.0).round() / 100.0),
                nrd: l.nrd,
                acima: l.acima,
            })
            .collect(),
    ))
}

fn dose_nao_encontrada() -> ErroApi {
    ErroApi::nao_encontrado(
        "dose-nao-encontrada",
        "Exame com dose não encontrado ou sem vínculo de atendimento com você. Nada foi registrado.",
    )
}

/// Justificativa de dose acima do NRD (médico com vínculo; `Idempotency-Key`). Fica no lastro do
/// estudo e entra na revisão periódica da proteção radiológica.
#[utoipa::path(post, operation_id = "justificar_dose", path = "/v1/doses/{id}/justificativa", tag = "exames",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = NovaJustificativa,
    responses((status = 201, body = JustificativaDeDose), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn justificar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovaJustificativa>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::DoseJustificar)?;
    let detalhe = n
        .detalhe
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(str::to_owned);
    if !MOTIVOS.contains(&n.motivo.as_str())
        || detalhe
            .as_ref()
            .is_some_and(|d| !(5..=500).contains(&d.chars().count()))
        || (n.motivo == "outro" && detalhe.is_none())
    {
        return Err(ErroApi::invalido(
            "justificativa-invalida",
            "Escolha o motivo; em \"Outro\", descreva em 5 a 500 caracteres. Nada foi registrado.",
        ));
    }
    let rota = format!("POST /v1/doses/{id}/justificativa");
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, &rota, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let dose = sqlx::query!(
        "SELECT codigo, acima_do_nrd FROM exames.dose WHERE id = $1",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(dose_nao_encontrada)?;
    if !dose.acima_do_nrd {
        return Err(ErroApi::conflito(
            "dose-dentro-do-nrd",
            "Esta dose está dentro do NRD e não precisa de justificativa. Nada foi registrado.",
        ));
    }
    let jid = novo_id();
    let codigo = proximo(&mut tx, "JUS").await?;
    sqlx::query!(
        "INSERT INTO exames.dose_justificativa (id, codigo, dose_id, motivo, detalhe, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'gestao_de_dose', $7, $8)",
        jid, codigo.como_str(), id, n.motivo, detalhe, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito(
            "dose-ja-justificada",
            "Esta dose já foi justificada por outra pessoa. Nada foi registrado. Recarregue para ver a justificativa.",
        ),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "exames",
            tipo: "dose_justificada",
            registro_id: jid,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "dose_id": id, "dose": dose.codigo, "motivo": n.motivo }),
            criado_por: Some(usuario.id),
            origem: "gestao_de_dose",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(JustificativaDeDose {
        motivo: n.motivo,
        detalhe,
        por: usuario.nome.clone(),
        em: tempo::agora(),
    })
    .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn dose_efetiva_pela_modalidade() {
        assert_eq!(
            dose_efetiva("CT", Some(520.0), None, Some(0.014)),
            Some(7.28)
        );
        assert_eq!(dose_efetiva("DX", None, Some(0.9), Some(0.2)), Some(0.18));
        assert_eq!(dose_efetiva("CT", Some(520.0), None, None), None);
        assert_eq!(dose_efetiva("DX", Some(520.0), None, Some(0.2)), None);
    }

    #[test]
    fn menor_de_idade_na_data_do_exame() {
        let em = Utc::now();
        let hoje = em.with_timezone(&tempo::FUSO).date_naive();
        assert!(menor(hoje - Duration::days(365 * 10), em));
        assert!(!menor(hoje - Duration::days(365 * 30), em));
    }
}
