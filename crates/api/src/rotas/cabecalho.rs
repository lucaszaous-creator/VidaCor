//! Cabeçalho impresso nos documentos da unidade (laudo e dossiê em PDF): nome fantasia, razão social,
//! CNPJ, endereço, telefone e responsável técnico (nome e CRM). Configuração versionada (0062): a
//! versão nova entra vigente e a anterior só deixa de ser. Gerência da própria unidade. A tela informa
//! a versão que viu; se outra pessoa publicou antes, nada é gravado (G2). DEC-2026-11-08-92.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::corpo_clinico::UFS;
use crate::sessao::Usuario;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct CabecalhoDaUnidade {
    pub numero: i32,
    pub nome: String,
    pub razao_social: Option<String>,
    pub cnpj: Option<String>,
    pub endereco: Option<String>,
    pub telefone: Option<String>,
    pub site: Option<String>,
    pub responsavel_nome: Option<String>,
    pub responsavel_crm: Option<String>,
    /// Texto impresso no rodapé do PDF.
    pub responsavel_tecnico: Option<String>,
    pub motivo: Option<String>,
    pub publicado_por: Option<String>,
    pub desde: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CabecalhoDosDocumentos {
    /// Sem versão cadastrada, o PDF sai só com "VidaCor Imagem" e o nome da unidade.
    pub vigente: Option<CabecalhoDaUnidade>,
    /// Todas as versões, da mais nova para a mais antiga.
    pub versoes: Vec<CabecalhoDaUnidade>,
}

#[derive(Deserialize, ToSchema)]
pub struct NovoCabecalho {
    /// Versão que a tela mostrou (nenhuma na primeira vez).
    pub numero_atual: Option<i32>,
    pub nome: String,
    pub razao_social: String,
    pub cnpj: String,
    pub endereco: String,
    pub telefone: String,
    pub site: Option<String>,
    pub responsavel_nome: String,
    pub responsavel_crm_uf: String,
    pub responsavel_crm_numero: String,
    /// Obrigatório a partir da segunda versão.
    pub motivo: Option<String>,
}

/// CNPJ com os dígitos verificadores conferidos; devolve no formato 00.000.000/0000-00.
#[must_use]
pub fn cnpj_valido(entrada: &str) -> Option<String> {
    let d: Vec<u32> = entrada
        .chars()
        .filter(|c| !matches!(c, '.' | '/' | '-' | ' '))
        .map(|c| c.to_digit(10))
        .collect::<Option<Vec<u32>>>()?;
    if d.len() != 14 || d.iter().all(|x| *x == d[0]) {
        return None;
    }
    let dv = |n: usize| {
        let pesos: &[u32] = if n == 12 {
            &[5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]
        } else {
            &[6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]
        };
        let soma: u32 = d.iter().zip(pesos).map(|(a, b)| a * b).sum();
        let r = soma % 11;
        if r < 2 {
            0
        } else {
            11 - r
        }
    };
    if dv(12) != d[12] || dv(13) != d[13] {
        return None;
    }
    let s: String = d
        .iter()
        .map(|x| char::from_digit(*x, 10).unwrap_or('0'))
        .collect();
    Some(format!(
        "{}.{}.{}/{}-{}",
        &s[0..2],
        &s[2..5],
        &s[5..8],
        &s[8..12],
        &s[12..14]
    ))
}

fn entre(v: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&v.chars().count())
}

fn site_valido(s: &str) -> bool {
    let (dominio, caminho) = s.split_once('/').unwrap_or((s, ""));
    entre(dominio, 4, 80)
        && dominio
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && caminho.len() <= 60
        && caminho.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'/' | b'_' | b'-')
        })
}

async fn carregar(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
) -> Result<CabecalhoDosDocumentos, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT c.numero, c.nome, c.razao_social, c.cnpj, c.endereco, c.telefone, c.site, c.responsavel_nome,
                  c.responsavel_crm, c.responsavel_tecnico, c.motivo, c.vigente, c.criado_em, u.nome AS "publicado_por?"
             FROM documentos.cabecalho c LEFT JOIN identidade.usuario u ON u.id = c.criado_por
            WHERE c.unidade_id = $1 ORDER BY c.numero DESC LIMIT 100"#,
        unidade
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut vigente = None;
    let mut versoes = Vec::with_capacity(linhas.len());
    for l in linhas {
        let c = CabecalhoDaUnidade {
            numero: l.numero,
            nome: l.nome,
            razao_social: l.razao_social,
            cnpj: l.cnpj,
            endereco: l.endereco,
            telefone: l.telefone,
            site: l.site,
            responsavel_nome: l.responsavel_nome,
            responsavel_crm: l.responsavel_crm,
            responsavel_tecnico: l.responsavel_tecnico,
            motivo: l.motivo,
            publicado_por: l.publicado_por,
            desde: l.criado_em,
        };
        if l.vigente {
            vigente = Some(c.numero);
        }
        versoes.push(c);
    }
    let vigente = vigente.and_then(|n| versoes.iter().find(|v| v.numero == n).cloned());
    Ok(CabecalhoDosDocumentos { vigente, versoes })
}

/// Cabeçalho vigente e versões anteriores (gerência).
#[utoipa::path(get, operation_id = "ver_cabecalho_dos_documentos", path = "/v1/configuracoes/cabecalho", tag = "configuracoes",
    responses((status = 200, body = CabecalhoDosDocumentos), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn ver(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<CabecalhoDosDocumentos>, ErroApi> {
    usuario.exigir(Ato::CabecalhoAdministrar)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = carregar(&mut tx, usuario.unidade_id).await?;
    tx.commit().await?;
    Ok(Json(r))
}

/// Publica a versão nova do cabeçalho (gerência; `Idempotency-Key`). Vale para os próximos PDFs.
#[utoipa::path(post, operation_id = "publicar_cabecalho_dos_documentos", path = "/v1/configuracoes/cabecalho", tag = "configuracoes",
    params(("Idempotency-Key" = Uuid, Header)), request_body = NovoCabecalho,
    responses((status = 201, body = CabecalhoDosDocumentos), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn publicar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoCabecalho>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::CabecalhoAdministrar)?;
    let invalido =
        |d: &str| ErroApi::invalido("cabecalho-invalido", &format!("{d} Nada foi publicado."));
    let nome = n.nome.trim();
    let razao = n.razao_social.trim();
    let endereco = n.endereco.trim();
    let telefone = n.telefone.trim();
    let responsavel = n.responsavel_nome.trim();
    let uf = n.responsavel_crm_uf.trim().to_uppercase();
    let crm = n.responsavel_crm_numero.trim();
    let site = n
        .site
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());
    let motivo = n
        .motivo
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    if !entre(nome, 2, 120) || !entre(razao, 2, 160) || !entre(endereco, 5, 200) {
        return Err(invalido("Confira o nome fantasia (2 a 120 letras), a razão social (2 a 160) e o endereço (5 a 200)."));
    }
    let Some(cnpj) = cnpj_valido(&n.cnpj) else {
        return Err(invalido(
            "O CNPJ não confere (14 algarismos com os dígitos verificadores).",
        ));
    };
    if !(8..=20).contains(&telefone.len())
        || !telefone
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'(' | b')' | b' ' | b'+' | b'-'))
    {
        return Err(invalido(
            "Confira o telefone (8 a 20 caracteres: algarismos, parênteses, espaço, + e -).",
        ));
    }
    if site.as_deref().is_some_and(|s| !site_valido(s)) {
        return Err(invalido(
            "Confira o site (ex.: vidacorimagem.com.br), sem http.",
        ));
    }
    if !entre(responsavel, 5, 120)
        || !UFS.contains(&uf.as_str())
        || crm.is_empty()
        || crm.len() > 10
        || !crm.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalido(
            "Confira o responsável técnico: nome (5 a 120 letras), UF e número do CRM.",
        ));
    }
    if motivo.as_deref().is_some_and(|m| !entre(m, 5, 300))
        || (n.numero_atual.is_some() && motivo.is_none())
    {
        return Err(ErroApi::invalido(
            "motivo-invalido",
            "Escreva o motivo da mudança (5 a 300 letras). Nada foi publicado.",
        ));
    }
    let responsavel_crm = format!("CRM-{uf} {crm}");
    let texto_impresso = format!("{responsavel}, {responsavel_crm}");
    if texto_impresso.chars().count() > 160 {
        return Err(invalido(
            "O nome do responsável técnico ficou longo demais para o rodapé.",
        ));
    }
    const ROTA: &str = "POST /v1/configuracoes/cabecalho";
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Reserva::Repetida { status, corpo } =
        idempotencia::reservar(&mut tx, usuario.id, ROTA, chave).await?
    {
        tx.commit().await?;
        return Ok((
            StatusCode::from_u16(status).unwrap_or(StatusCode::OK),
            Json(corpo),
        )
            .into_response());
    }
    let mudou = || {
        ErroApi::conflito(
            "cabecalho-mudou",
            "Outra pessoa publicou uma versão do cabeçalho enquanto você editava. Nada foi publicado. Recarregue e confira antes de repetir.",
        )
    };
    let numero = match n.numero_atual {
        Some(atual) => {
            sqlx::query_scalar!(
                "UPDATE documentos.cabecalho SET vigente = false, versao = versao + 1
                  WHERE unidade_id = $1 AND vigente AND numero = $2 RETURNING numero",
                usuario.unidade_id,
                atual
            )
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(mudou)?
                + 1
        }
        None => 1,
    };
    sqlx::query!(
        "INSERT INTO documentos.cabecalho (id, numero, nome, razao_social, cnpj, endereco, telefone, site, responsavel_nome, responsavel_crm,
                                           responsavel_tecnico, motivo, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'configuracoes', $14, $15)",
        novo_id(), numero, nome, razao, cnpj, endereco, telefone, site, responsavel, responsavel_crm, texto_impresso, motivo,
        usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => mudou(),
        _ => ErroApi::from(e),
    })?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "documentos",
            tipo: "cabecalho_publicado",
            registro_id: usuario.unidade_id,
            registro_codigo: None,
            dados: json!({ "numero": numero, "motivo": motivo, "cnpj": cnpj }),
            criado_por: Some(usuario.id),
            origem: "configuracoes",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let corpo = serde_json::to_value(carregar(&mut tx, usuario.unidade_id).await?)
        .map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cnpj_confere_os_digitos_verificadores() {
        // CNPJ fictício de exemplo com dígitos verificadores válidos.
        assert_eq!(
            cnpj_valido("11222333000181").as_deref(),
            Some("11.222.333/0001-81")
        );
        assert_eq!(
            cnpj_valido("11.222.333/0001-81").as_deref(),
            Some("11.222.333/0001-81")
        );
        assert_eq!(cnpj_valido("11.222.333/0001-82"), None);
        assert_eq!(cnpj_valido("11111111111111"), None);
        assert_eq!(cnpj_valido("1122233300018"), None);
        assert_eq!(cnpj_valido("11a22333000181"), None);
    }

    #[test]
    fn site_sem_esquema_nem_maiusculas() {
        assert!(site_valido("vidacorimagem.com.br"));
        assert!(site_valido("vidacor.com.br/laudos"));
        assert!(!site_valido("https://vidacor.com.br"));
        assert!(!site_valido("a.b"));
    }
}
