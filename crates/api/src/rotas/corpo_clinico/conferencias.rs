//! Conferências do corpo clínico: consulta do CRM no CFM (pelo webservice, quando configurado, ou
//! registrada pela gerência a partir do portal do CFM) e certificado ICP-Brasil do médico (só o
//! certificado público). Aprovar registro de CRM exige consulta regular dos últimos 30 dias
//! (`pedidos::decidir`). DEC-2026-11-09-120 e 113.

use super::{invalido, motivo_valido, um, MedicoDoCorpoClinico, UFS};
use crate::atos::Ato;
use crate::banco;
use crate::certificado;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::Corpo;
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

/// Validade da consulta do CRM para aprovar o registro.
pub const DIAS_DA_CONSULTA: i64 = 30;
/// A tela avisa quando o certificado vence em menos de tantos dias.
pub const DIAS_DE_AVISO_DO_CERTIFICADO: i64 = 30;

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct ConsultaCrm {
    pub id: Uuid,
    pub codigo: String,
    pub uf: String,
    pub numero: String,
    /// webservice_cfm ou portal_cfm.
    pub fonte: String,
    /// regular, suspenso, cassado, cancelado, falecido, transferido, interdicao_cautelar ou nao_encontrado.
    pub situacao: String,
    pub nome_no_conselho: Option<String>,
    pub especialidades: Vec<String>,
    pub resposta_sha256: Option<String>,
    pub observacao: Option<String>,
    pub consultado_em: DateTime<Utc>,
    pub consultado_por: String,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct CertificadoDoMedico {
    pub id: Uuid,
    pub codigo: String,
    pub titular: String,
    pub cpf_mascarado: Option<String>,
    pub emissor: String,
    pub serie: String,
    /// A1, A2, A3, A4 ou outro.
    pub tipo: Option<String>,
    pub valido_de: DateTime<Utc>,
    pub valido_ate: DateTime<Utc>,
    pub sha256: String,
    /// Dias até vencer (negativo: vencido), calculado pelo servidor.
    pub vence_em_dias: i64,
    pub cadastrado_em: DateTime<Utc>,
    pub cadastrado_por: String,
}

#[derive(Deserialize, ToSchema)]
pub struct NovaConsultaCrm {
    pub uf: String,
    pub numero: String,
    /// webservice_cfm (o núcleo consulta) ou portal_cfm (a gerência registra o que viu no portal).
    pub fonte: String,
    /// Só para portal_cfm.
    pub situacao: Option<String>,
    pub nome_no_conselho: Option<String>,
    /// Obrigatória para portal_cfm: onde e como a consulta foi feita.
    pub observacao: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct NovoCertificado {
    /// Conteúdo do arquivo `.cer` (PEM ou DER) em base64. Só o certificado público.
    pub arquivo_base64: String,
    /// Obrigatório quando substitui o certificado vigente.
    pub motivo: Option<String>,
}

fn sem_autoconferencia(usuario: &Usuario, medico: Uuid) -> Result<(), ErroApi> {
    if usuario.id == medico {
        return Err(ErroApi::novo(
            StatusCode::FORBIDDEN,
            "separacao-de-funcoes",
            "Outra pessoa confere",
            "O próprio médico não registra a conferência do seu cadastro. Nada foi registrado.",
        ));
    }
    Ok(())
}

pub(super) async fn ultimas_consultas(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    medico: Option<Uuid>,
    todas: bool,
) -> Result<Vec<(Uuid, ConsultaCrm)>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT c.id, c.codigo, c.medico_id, c.uf, c.numero, c.fonte, c.situacao, c.nome_no_conselho, c.especialidades,
                  c.resposta_sha256, c.observacao, c.criado_em, u.nome AS consultado_por
             FROM corpo.consulta_crm c JOIN identidade.usuario u ON u.id = c.criado_por
            WHERE c.unidade_id = $1 AND ($2::uuid IS NULL OR c.medico_id = $2)
            ORDER BY c.criado_em DESC LIMIT 500"#,
        unidade,
        medico
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut vistos = std::collections::HashSet::new();
    Ok(linhas
        .into_iter()
        .filter(|l| todas || vistos.insert(l.medico_id))
        .map(|l| {
            (
                l.medico_id,
                ConsultaCrm {
                    id: l.id,
                    codigo: l.codigo,
                    uf: l.uf,
                    numero: l.numero,
                    fonte: l.fonte,
                    situacao: l.situacao,
                    nome_no_conselho: l.nome_no_conselho,
                    especialidades: l.especialidades,
                    resposta_sha256: l.resposta_sha256,
                    observacao: l.observacao,
                    consultado_em: l.criado_em,
                    consultado_por: l.consultado_por,
                },
            )
        })
        .collect())
}

pub(super) async fn certificados_vigentes(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    medico: Option<Uuid>,
) -> Result<HashMap<Uuid, CertificadoDoMedico>, ErroApi> {
    let agora = Utc::now();
    let linhas = sqlx::query!(
        r#"SELECT c.id, c.codigo, c.medico_id, c.titular, c.cpf_mascarado, c.emissor, c.serie, c.tipo, c.valido_de, c.valido_ate,
                  c.sha256, c.criado_em, u.nome AS cadastrado_por
             FROM corpo.certificado c JOIN identidade.usuario u ON u.id = c.criado_por
            WHERE c.unidade_id = $1 AND c.vigente AND ($2::uuid IS NULL OR c.medico_id = $2)"#,
        unidade,
        medico
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| {
            (
                l.medico_id,
                CertificadoDoMedico {
                    id: l.id,
                    codigo: l.codigo,
                    titular: l.titular,
                    cpf_mascarado: l.cpf_mascarado,
                    emissor: l.emissor,
                    serie: l.serie,
                    tipo: l.tipo,
                    valido_de: l.valido_de,
                    valido_ate: l.valido_ate,
                    sha256: l.sha256,
                    vence_em_dias: (l.valido_ate - agora).num_days(),
                    cadastrado_em: l.criado_em,
                    cadastrado_por: l.cadastrado_por,
                },
            )
        })
        .collect())
}

/// Confere, na aprovação do registro, se há consulta regular do mesmo CRM nos últimos 30 dias.
pub(super) async fn exigir_consulta_regular(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    medico: Uuid,
    uf: &str,
    numero: &str,
) -> Result<(), ErroApi> {
    let situacao = sqlx::query_scalar!(
        "SELECT situacao FROM corpo.consulta_crm
          WHERE unidade_id = $1 AND medico_id = $2 AND uf = $3 AND numero = $4 AND criado_em >= $5
          ORDER BY criado_em DESC LIMIT 1",
        unidade,
        medico,
        uf,
        numero,
        Utc::now() - Duration::days(DIAS_DA_CONSULTA)
    )
    .fetch_optional(&mut *tx)
    .await?;
    match situacao.as_deref() {
        Some("regular") => Ok(()),
        Some(outra) => Err(ErroApi::conflito(
            "crm-irregular",
            &format!(
                "A última consulta no CFM deste CRM diz \"{}\". O registro não pode ser aprovado. Recuse o pedido com o motivo ou consulte de novo se a situação mudou.",
                outra.replace('_', " ")
            ),
        )),
        None => Err(ErroApi::conflito(
            "crm-sem-consulta",
            "Antes de aprovar, consulte este CRM no CFM (ou registre a consulta feita no portal do CFM). Vale a consulta dos últimos 30 dias. Nada foi alterado.",
        )),
    }
}

/// Consulta do CRM no CFM: pelo webservice (quando configurado) ou registro da consulta feita no
/// portal do CFM pela gerência. Somente inserção; entra no lastro do médico.
#[utoipa::path(post, operation_id = "consultar_crm_no_cfm", path = "/v1/corpo-clinico/{id}/consultas-crm", tag = "corpo-clinico",
    params(("id" = Uuid, Path)), request_body = NovaConsultaCrm,
    responses((status = 201, body = ConsultaCrm), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 422, body = crate::openapi::ProblemaDoc), (status = 502, body = crate::openapi::ProblemaDoc), (status = 503, body = crate::openapi::ProblemaDoc)))]
pub async fn consultar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(medico): Path<Uuid>,
    Corpo(n): Corpo<NovaConsultaCrm>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::CorpoClinicoAdministrar)?;
    sem_autoconferencia(&usuario, medico)?;
    let uf = n.uf.trim().to_uppercase();
    let numero = n.numero.trim().to_owned();
    if !UFS.contains(&uf.as_str())
        || numero.is_empty()
        || numero.len() > 10
        || !numero.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalido(
            "Confira a UF e o número do CRM (só algarismos, até 10). Nada foi registrado.",
        ));
    }
    let limpo = |v: &Option<String>| {
        v.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    // Confere que o médico existe na unidade antes de qualquer chamada externa.
    {
        let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
        um(&mut tx, usuario.unidade_id, medico).await?;
        tx.commit().await?;
    }
    let (situacao, nome, especialidades, sha, observacao) = match n.fonte.as_str() {
        "webservice_cfm" => {
            let Some(cfm) = estado.cfm.as_ref() else {
                return Err(ErroApi::novo(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "cfm-nao-configurado",
                    "Consulta ao CFM indisponível",
                    "A consulta automática ao CFM ainda não está configurada. Consulte no portal do CFM e registre o que viu (fonte: portal do CFM).",
                ));
            };
            let r = cfm.consultar(&uf, &numero).await.map_err(|e| {
                tracing::warn!(erro = %e, "consulta ao CFM falhou");
                ErroApi::novo(
                    StatusCode::BAD_GATEWAY,
                    "cfm-indisponivel",
                    "CFM não respondeu",
                    "O CFM não respondeu agora. Nada foi registrado. Tente de novo em instantes ou consulte no portal do CFM.",
                )
            })?;
            if !crate::cfm::SITUACOES.contains(&r.situacao.as_str()) {
                return Err(ErroApi::interno("situação desconhecida devolvida pelo CFM"));
            }
            (
                r.situacao,
                r.nome,
                r.especialidades,
                Some(r.resposta_sha256),
                None,
            )
        }
        "portal_cfm" => {
            let situacao = n.situacao.clone().unwrap_or_default();
            if !crate::cfm::SITUACOES.contains(&situacao.as_str()) {
                return Err(invalido(
                    "Escolha a situação que o portal do CFM mostrou. Nada foi registrado.",
                ));
            }
            let obs = limpo(&n.observacao)
                .filter(|o| (5..=300).contains(&o.chars().count()))
                .ok_or_else(|| {
                    invalido("Diga onde e como consultou (5 a 300 letras), ex.: portal do CFM, busca por CRM e UF. Nada foi registrado.")
                })?;
            (
                situacao,
                limpo(&n.nome_no_conselho),
                Vec::new(),
                None,
                Some(obs),
            )
        }
        _ => return Err(invalido("Fonte da consulta: webservice_cfm ou portal_cfm.")),
    };
    let nome = nome.filter(|s| (3..=160).contains(&s.chars().count()));
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let id = novo_id();
    let codigo = proximo(&mut tx, "CCF").await?;
    sqlx::query!(
        "INSERT INTO corpo.consulta_crm (id, codigo, medico_id, uf, numero, fonte, situacao, nome_no_conselho, especialidades, resposta_sha256, observacao, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'corpo_clinico', $13, $14)",
        id, codigo.como_str(), medico, uf, numero, n.fonte, situacao, nome, &especialidades, sha, observacao, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "corpo_clinico",
            tipo: "crm_consultado",
            registro_id: medico,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "uf": uf, "numero": numero, "fonte": n.fonte, "situacao": situacao, "resposta_sha256": sha, "por": usuario.id }),
            criado_por: Some(usuario.id),
            origem: "corpo_clinico",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let c = ultimas_consultas(&mut tx, usuario.unidade_id, Some(medico), false)
        .await?
        .into_iter()
        .map(|(_, c)| c)
        .next()
        .ok_or_else(|| ErroApi::interno("consulta registrada sem leitura"))?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(c)).into_response())
}

/// Consultas do CRM do médico, da mais nova para a mais antiga (gerência).
#[utoipa::path(get, operation_id = "consultas_crm_do_medico", path = "/v1/corpo-clinico/{id}/consultas-crm", tag = "corpo-clinico",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Vec<ConsultaCrm>), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn consultas(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(medico): Path<Uuid>,
) -> Result<Json<Vec<ConsultaCrm>>, ErroApi> {
    usuario.exigir(Ato::CorpoClinicoAdministrar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    um(&mut tx, usuario.unidade_id, medico).await?;
    let r = ultimas_consultas(&mut tx, usuario.unidade_id, Some(medico), true)
        .await?
        .into_iter()
        .map(|(_, c)| c)
        .collect();
    tx.commit().await?;
    Ok(Json(r))
}

fn recusa(codigo: &str, detalhe: &str) -> ErroApi {
    ErroApi::invalido(codigo, detalhe)
}

/// Cadastra o certificado ICP-Brasil do médico (gerência; reautenticação). Só o certificado
/// público; recusa chave privada, certificado fora da ICP-Brasil, vencido ou de outra pessoa.
/// Substituir exige motivo; o anterior fica guardado.
#[utoipa::path(post, operation_id = "cadastrar_certificado_do_medico", path = "/v1/corpo-clinico/{id}/certificados", tag = "corpo-clinico",
    params(("id" = Uuid, Path)), request_body = NovoCertificado,
    responses((status = 201, body = MedicoDoCorpoClinico), (status = 401, body = crate::openapi::ProblemaDoc), (status = 403, body = crate::openapi::ProblemaDoc),
        (status = 404, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn cadastrar_certificado(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(medico): Path<Uuid>,
    Corpo(n): Corpo<NovoCertificado>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::CorpoClinicoAdministrar)?;
    sem_autoconferencia(&usuario, medico)?;
    usuario.exigir_reautenticacao()?;
    if n.arquivo_base64.len() > certificado::TAMANHO_MAXIMO * 2 {
        return Err(recusa(
            "certificado-invalido",
            "O arquivo do certificado tem até 16 KB. Nada foi cadastrado.",
        ));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(n.arquivo_base64.trim())
        .map_err(|_| recusa("certificado-invalido", "Não foi possível ler o arquivo. Envie o certificado público (.cer). Nada foi cadastrado."))?;
    let c = certificado::ler(&bytes).map_err(|m| {
        recusa(
            "certificado-invalido",
            &format!("Certificado recusado: {m}. Nada foi cadastrado."),
        )
    })?;
    if !c.icp_brasil {
        return Err(recusa("certificado-nao-icp", "Este certificado não é da ICP-Brasil (não traz política ICP-Brasil). Nada foi cadastrado."));
    }
    let agora = Utc::now();
    if c.valido_ate <= agora || c.valido_de > agora {
        return Err(recusa(
            "certificado-fora-da-validade",
            "Este certificado está vencido ou ainda não começou a valer. Nada foi cadastrado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let m = um(&mut tx, usuario.unidade_id, medico).await?;
    if !certificado::nome_confere(&c.titular, &m.nome) {
        return Err(recusa(
            "certificado-de-outra-pessoa",
            &format!("O titular do certificado ({}) não confere com o nome do cadastro ({}). Nada foi cadastrado. Confira se o arquivo é deste médico.", c.titular, m.nome),
        ));
    }
    let atual = sqlx::query!(
        "SELECT id, sha256 FROM corpo.certificado WHERE medico_id = $1 AND vigente AND unidade_id = $2 FOR UPDATE",
        medico,
        usuario.unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let motivo = match &atual {
        Some(a) if a.sha256 == c.sha256 => {
            return Err(ErroApi::conflito(
                "certificado-ja-cadastrado",
                "Este certificado já é o vigente deste médico. Nada foi alterado.",
            ));
        }
        Some(_) => Some(motivo_valido(
            n.motivo.as_deref().unwrap_or_default(),
            "Nada foi cadastrado.",
        )?),
        None => n
            .motivo
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| motivo_valido(s, "Nada foi cadastrado."))
            .transpose()?,
    };
    if let Some(a) = &atual {
        sqlx::query!("UPDATE corpo.certificado SET vigente = false, versao = versao + 1 WHERE id = $1 AND vigente", a.id)
            .execute(&mut *tx)
            .await?;
    }
    let id = novo_id();
    let codigo = proximo(&mut tx, "CER").await?;
    sqlx::query!(
        "INSERT INTO corpo.certificado (id, codigo, medico_id, titular, cpf_mascarado, emissor, serie, tipo, valido_de, valido_ate, sha256, motivo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'corpo_clinico', $14, $15)",
        id, codigo.como_str(), medico, c.titular, c.cpf_mascarado, c.emissor.chars().take(500).collect::<String>(), c.serie.chars().take(200).collect::<String>(),
        c.tipo, c.valido_de, c.valido_ate, c.sha256, motivo, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito(
            "certificado-mudou",
            "Outra pessoa cadastrou um certificado deste médico agora. Nada foi alterado. Recarregue e confira.",
        ),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "corpo_clinico",
            tipo: "certificado_cadastrado",
            registro_id: medico,
            registro_codigo: Some(codigo.como_str()),
            dados: json!({ "sha256": c.sha256, "tipo": c.tipo, "valido_ate": c.valido_ate, "substitui": atual.map(|a| a.id), "motivo": motivo, "por": usuario.id, "reautenticado": true }),
            criado_por: Some(usuario.id),
            origem: "corpo_clinico",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let r = um(&mut tx, usuario.unidade_id, medico).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(r)).into_response())
}
