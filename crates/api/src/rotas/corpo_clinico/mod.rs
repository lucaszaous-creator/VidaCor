//! Corpo clínico (Corpo-Clinico.dc.html; docs/04: "médico novo e mudanças só valem após segunda
//! conferência"; docs/18 § 6). A gerência vê os médicos da própria unidade com o registro profissional
//! (CRM, UF, RQE e especialidade) e o que cada um realiza e lauda por modalidade. Toda mudança é um
//! pedido com motivo que só vale quando OUTRA pessoa da gerência confere; a versão nova entra vigente e
//! a anterior deixa de ser (nunca UPDATE destrutivo nem DELETE). Pedir e conferir pedem reautenticação
//! (SEC-AUT-5), como mudar perfil. TI não tem ato aqui. DEC-2026-11-08-91.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Unidades da federação aceitas no CRM.
pub const UFS: [&str; 27] = [
    "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG", "PA", "PB", "PR",
    "PE", "PI", "RJ", "RN", "RS", "RO", "RR", "SC", "SP", "SE", "TO",
];

#[derive(Serialize, Deserialize, ToSchema, Clone, PartialEq, Eq)]
pub struct DadosDoRegistro {
    pub uf: String,
    pub numero: String,
    pub rqe: Option<String>,
    pub especialidade: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema, Clone, PartialEq, Eq)]
pub struct DadosDaHabilitacao {
    /// Modalidades que realiza na sala (US, CT...).
    pub realiza: Vec<String>,
    /// Modalidades que lauda.
    pub lauda: Vec<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RegistroVigente {
    pub conselho: String,
    pub uf: String,
    pub numero: String,
    pub rqe: Option<String>,
    pub especialidade: Option<String>,
    pub desde: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct HabilitacaoVigente {
    pub realiza: Vec<String>,
    pub lauda: Vec<String>,
    pub desde: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PedidoDoCorpoClinico {
    pub id: Uuid,
    pub codigo: String,
    /// registro ou habilitacao.
    pub tipo: String,
    /// aguardando, aplicado ou recusado.
    pub situacao: String,
    pub registro: Option<DadosDoRegistro>,
    pub habilitacao: Option<DadosDaHabilitacao>,
    pub motivo: String,
    pub pedido_por: String,
    pub pedido_por_id: Uuid,
    pub pedido_em: DateTime<Utc>,
    pub decidido_por: Option<String>,
    pub decidido_em: Option<DateTime<Utc>>,
    pub motivo_decisao: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MedicoDoCorpoClinico {
    pub id: Uuid,
    pub codigo: String,
    pub nome: String,
    pub ativo: bool,
    /// ativo, aguardando_conferencia, sem_registro ou inativo.
    pub situacao: String,
    pub registro: Option<RegistroVigente>,
    pub habilitacao: Option<HabilitacaoVigente>,
    pub pendentes: Vec<PedidoDoCorpoClinico>,
    /// Última consulta do CRM no CFM (qualquer fonte).
    pub consulta_crm: Option<conferencias::ConsultaCrm>,
    /// Certificado ICP-Brasil vigente.
    pub certificado: Option<conferencias::CertificadoDoMedico>,
}

#[derive(Deserialize, ToSchema)]
pub struct NovoPedidoDoCorpoClinico {
    /// registro ou habilitacao.
    pub tipo: String,
    pub registro: Option<DadosDoRegistro>,
    pub habilitacao: Option<DadosDaHabilitacao>,
    pub motivo: String,
}

#[derive(Deserialize, ToSchema)]
pub struct DecisaoDoCorpoClinico {
    pub aprovar: bool,
    pub motivo: String,
}

fn invalido(detalhe: &str) -> ErroApi {
    ErroApi::invalido("corpo-clinico-invalido", detalhe)
}

fn motivo_valido(m: &str, nada: &str) -> Result<String, ErroApi> {
    let m = m.trim();
    if (5..=300).contains(&m.chars().count()) {
        Ok(m.to_owned())
    } else {
        Err(ErroApi::invalido(
            "motivo-invalido",
            &format!("Escreva o motivo (5 a 300 letras). {nada}"),
        ))
    }
}

fn so_digitos(v: &str, max: usize) -> bool {
    !v.is_empty() && v.len() <= max && v.bytes().all(|b| b.is_ascii_digit())
}

/// Confere e normaliza o registro (o servidor decide; G1).
fn validar_registro(r: &DadosDoRegistro) -> Result<DadosDoRegistro, ErroApi> {
    let uf = r.uf.trim().to_uppercase();
    let numero = r.numero.trim().to_owned();
    let rqe = r
        .rqe
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let especialidade = r
        .especialidade
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    if !UFS.contains(&uf.as_str()) || !so_digitos(&numero, 10) {
        return Err(invalido(
            "Confira o CRM: UF válida e número só com algarismos (até 10). Nada foi pedido.",
        ));
    }
    if rqe.as_deref().is_some_and(|q| !so_digitos(q, 10)) {
        return Err(invalido(
            "O RQE tem só algarismos (até 10). Nada foi pedido.",
        ));
    }
    if especialidade
        .as_deref()
        .is_some_and(|e| !(3..=80).contains(&e.chars().count()))
    {
        return Err(invalido(
            "A especialidade tem de 3 a 80 letras. Nada foi pedido.",
        ));
    }
    if rqe.is_some() && especialidade.is_none() {
        return Err(invalido(
            "O RQE é de uma especialidade: informe qual. Nada foi pedido.",
        ));
    }
    Ok(DadosDoRegistro {
        uf,
        numero,
        rqe,
        especialidade,
    })
}

fn modalidades(v: &[String]) -> Result<Vec<String>, ErroApi> {
    let mut saida: Vec<String> = Vec::new();
    for m in v {
        let m = m.trim().to_uppercase();
        if !(2..=4).contains(&m.len()) || !m.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(invalido(
                "Modalidade tem 2 a 4 letras (US, CT, DX). Nada foi pedido.",
            ));
        }
        if !saida.contains(&m) {
            saida.push(m);
        }
    }
    if saida.len() > 12 {
        return Err(invalido("No máximo 12 modalidades. Nada foi pedido."));
    }
    saida.sort();
    Ok(saida)
}

fn situacao(ativo: bool, tem_registro: bool, pendente: bool) -> &'static str {
    if !ativo {
        "inativo"
    } else if pendente {
        "aguardando_conferencia"
    } else if tem_registro {
        "ativo"
    } else {
        "sem_registro"
    }
}

async fn pedidos(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    medico: Option<Uuid>,
    so_aguardando: bool,
) -> Result<Vec<(Uuid, PedidoDoCorpoClinico)>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT p.id, p.codigo, p.medico_id, p.tipo, p.situacao, p.dados, p.motivo, p.pedido_por, q.nome AS pedido_por_nome,
                  p.criado_em, d.nome AS "decidido_por_nome?", p.decidido_em, p.motivo_decisao
             FROM corpo.pedido p
             JOIN identidade.usuario q ON q.id = p.pedido_por
             LEFT JOIN identidade.usuario d ON d.id = p.decidido_por
            WHERE p.unidade_id = $1 AND ($2::uuid IS NULL OR p.medico_id = $2) AND (NOT $3 OR p.situacao = 'aguardando')
            ORDER BY p.criado_em DESC LIMIT 200"#,
        unidade,
        medico,
        so_aguardando
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| {
            let registro = (l.tipo == "registro")
                .then(|| serde_json::from_value(l.dados.clone()).ok())
                .flatten();
            let habilitacao = (l.tipo == "habilitacao")
                .then(|| serde_json::from_value(l.dados).ok())
                .flatten();
            (
                l.medico_id,
                PedidoDoCorpoClinico {
                    id: l.id,
                    codigo: l.codigo,
                    tipo: l.tipo,
                    situacao: l.situacao,
                    registro,
                    habilitacao,
                    motivo: l.motivo,
                    pedido_por: l.pedido_por_nome,
                    pedido_por_id: l.pedido_por,
                    pedido_em: l.criado_em,
                    decidido_por: l.decidido_por_nome,
                    decidido_em: l.decidido_em,
                    motivo_decisao: l.motivo_decisao,
                },
            )
        })
        .collect())
}

async fn carregar(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    id: Option<Uuid>,
) -> Result<Vec<MedicoDoCorpoClinico>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT u.id, u.codigo, u.nome, u.ativo,
                  r.conselho AS "conselho?", r.uf AS "uf?", r.numero AS "numero?", r.rqe AS "rqe?",
                  r.especialidade AS "especialidade?", r.criado_em AS "registro_desde?",
                  h.realiza AS "realiza?", h.lauda AS "lauda?", h.criado_em AS "habilitacao_desde?"
             FROM identidade.usuario u
             LEFT JOIN documentos.registro_profissional r ON r.usuario_id = u.id AND r.conselho = 'CRM' AND r.vigente
             LEFT JOIN corpo.habilitacao h ON h.usuario_id = u.id AND h.vigente
            WHERE u.unidade_id = $1 AND 'medico' = ANY(u.perfis) AND ($2::uuid IS NULL OR u.id = $2)
            ORDER BY u.nome LIMIT 500"#,
        unidade,
        id
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut pendentes: HashMap<Uuid, Vec<PedidoDoCorpoClinico>> = HashMap::new();
    for (medico, p) in pedidos(tx, unidade, id, true).await? {
        pendentes.entry(medico).or_default().push(p);
    }
    let mut consultas: HashMap<Uuid, conferencias::ConsultaCrm> =
        conferencias::ultimas_consultas(tx, unidade, id, false)
            .await?
            .into_iter()
            .collect();
    let mut certificados = conferencias::certificados_vigentes(tx, unidade, id).await?;
    Ok(linhas
        .into_iter()
        .map(|l| {
            let registro = match (l.conselho, l.uf, l.numero, l.registro_desde) {
                (Some(conselho), Some(uf), Some(numero), Some(desde)) => Some(RegistroVigente {
                    conselho,
                    uf,
                    numero,
                    rqe: l.rqe,
                    especialidade: l.especialidade,
                    desde,
                }),
                _ => None,
            };
            let habilitacao = match (l.realiza, l.lauda, l.habilitacao_desde) {
                (Some(realiza), Some(lauda), Some(desde)) => Some(HabilitacaoVigente {
                    realiza,
                    lauda,
                    desde,
                }),
                _ => None,
            };
            let pendentes = pendentes.remove(&l.id).unwrap_or_default();
            MedicoDoCorpoClinico {
                situacao: situacao(l.ativo, registro.is_some(), !pendentes.is_empty()).to_owned(),
                id: l.id,
                codigo: l.codigo,
                nome: l.nome,
                ativo: l.ativo,
                registro,
                habilitacao,
                pendentes,
                consulta_crm: consultas.remove(&l.id),
                certificado: certificados.remove(&l.id),
            }
        })
        .collect())
}

async fn um(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    id: Uuid,
) -> Result<MedicoDoCorpoClinico, ErroApi> {
    carregar(tx, unidade, Some(id)).await?.pop().ok_or_else(|| {
        ErroApi::nao_encontrado(
            "medico-nao-encontrado",
            "Médico não encontrado nesta unidade. Nada foi alterado.",
        )
    })
}

/// Médicos da unidade com registro, habilitação e pedidos aguardando conferência (gerência).
#[utoipa::path(get, operation_id = "listar_corpo_clinico", path = "/v1/corpo-clinico", tag = "corpo-clinico",
    responses((status = 200, body = Vec<MedicoDoCorpoClinico>), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Vec<MedicoDoCorpoClinico>>, ErroApi> {
    usuario.exigir(Ato::CorpoClinicoAdministrar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = carregar(&mut tx, usuario.unidade_id, None).await?;
    tx.commit().await?;
    Ok(Json(r))
}

/// Histórico de pedidos do médico, com quem pediu, quem conferiu e os motivos (gerência).
#[utoipa::path(get, operation_id = "historico_do_medico", path = "/v1/corpo-clinico/{id}/historico", tag = "corpo-clinico", params(("id" = Uuid, Path)),
    responses((status = 200, body = Vec<PedidoDoCorpoClinico>), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn historico(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<PedidoDoCorpoClinico>>, ErroApi> {
    usuario.exigir(Ato::CorpoClinicoAdministrar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    um(&mut tx, usuario.unidade_id, id).await?;
    let r = pedidos(&mut tx, usuario.unidade_id, Some(id), false)
        .await?
        .into_iter()
        .map(|(_, p)| p)
        .collect();
    tx.commit().await?;
    Ok(Json(r))
}

pub mod conferencias;
pub mod pedidos;
