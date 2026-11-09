//! Exames e procedimentos (Exames-Procedimentos.dc.html; docs/04 Cadastros base). O catálogo é da
//! clínica (código TUSS único); o preço é por unidade, numa tabela versionada com data de vigência.
//! A recepção consulta; a gerência inclui procedimento, muda duração, modalidade e ativo (com motivo
//! e histórico imutável) e publica versão nova da tabela de preços, que nunca altera a vigente: começa
//! numa data futura. O servidor decide o preço (G1) e a concorrência é resolvida no banco (G2).
//! DEC-2026-11-08-93 e 94.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Maior preço aceito por procedimento (R$ 100.000,00), contra erro de digitação.
pub const PRECO_MAXIMO_CENTAVOS: i64 = 10_000_000;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProcedimentoDoCatalogo {
    pub tuss: String,
    pub nome: String,
    pub modalidade: Option<String>,
    pub duracao_min: i32,
    pub requer_contraste: bool,
    pub ativo: bool,
    /// Versão do cadastro; a mudança informa a versão que viu (G2).
    pub versao: i32,
    /// Preço do particular na tabela vigente desta unidade (sem preço: não cobra no caixa).
    pub preco_centavos: Option<i64>,
    /// Preço na próxima versão já publicada, se houver.
    pub proximo_preco_centavos: Option<i64>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct VersaoDaTabelaDePreco {
    pub id: Uuid,
    pub versao: i32,
    pub vigente_desde: NaiveDate,
    pub descricao: String,
    pub motivo: Option<String>,
    pub itens: i64,
    pub publicada_por: Option<String>,
    pub publicada_em: DateTime<Utc>,
    /// vigente, futura ou anterior.
    pub situacao: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CatalogoDeExames {
    pub procedimentos: Vec<ProcedimentoDoCatalogo>,
    /// Versões da tabela de preços desta unidade, da mais nova para a mais antiga.
    pub tabelas: Vec<VersaoDaTabelaDePreco>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct EstadoDoProcedimento {
    pub nome: String,
    pub modalidade: Option<String>,
    pub duracao_min: i32,
    pub requer_contraste: bool,
    pub ativo: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MudancaRegistradaNoCatalogo {
    pub codigo: String,
    /// incluir, alterar, desativar ou reativar.
    pub tipo: String,
    pub motivo: String,
    pub versao_do_procedimento: i32,
    pub antes: Option<EstadoDoProcedimento>,
    pub depois: EstadoDoProcedimento,
    pub quem: String,
    pub em: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct NovoProcedimento {
    pub tuss: String,
    pub nome: String,
    pub modalidade: String,
    pub duracao_min: i32,
    pub requer_contraste: bool,
    pub motivo: String,
}

#[derive(Deserialize, ToSchema)]
pub struct MudancaNoProcedimento {
    /// Versão do cadastro que a tela mostrou.
    pub versao: i32,
    pub duracao_min: Option<i32>,
    pub modalidade: Option<String>,
    pub ativo: Option<bool>,
    pub motivo: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PrecoDaVersao {
    pub tuss: String,
    pub valor_centavos: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct NovaVersaoDaTabela {
    /// Primeiro dia da nova versão: depois de hoje e depois do início da última versão.
    pub vigente_desde: NaiveDate,
    pub descricao: String,
    pub motivo: String,
    pub precos: Vec<PrecoDaVersao>,
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

fn modalidade_valida(m: &str) -> Option<String> {
    let m = m.trim().to_uppercase();
    ((2..=4).contains(&m.len()) && m.bytes().all(|b| b.is_ascii_uppercase())).then_some(m)
}

fn tuss_valido(t: &str) -> bool {
    t.len() == 8 && t.bytes().all(|b| b.is_ascii_digit())
}

fn duracao_valida(d: i32) -> bool {
    (5..=240).contains(&d) && d % 5 == 0
}

/// Versões da tabela desta unidade (mais nova primeiro), com a vigente e a próxima.
async fn tabelas(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
) -> Result<(Vec<VersaoDaTabelaDePreco>, Option<Uuid>, Option<Uuid>), ErroApi> {
    let hoje = tempo::hoje_local();
    let linhas = sqlx::query!(
        r#"SELECT t.id, t.versao, t.vigente_desde, t.descricao, t.motivo, t.criado_em, u.nome AS "publicada_por?",
                  (SELECT count(*) FROM financeiro.preco p WHERE p.tabela_id = t.id) AS "itens!"
             FROM financeiro.tabela_preco t LEFT JOIN identidade.usuario u ON u.id = t.criado_por
            WHERE t.unidade_id = $1 ORDER BY t.versao DESC LIMIT 100"#,
        unidade
    )
    .fetch_all(&mut *tx)
    .await?;
    // Mesma regra do caixa e do orçamento: vigente é a maior versão já iniciada.
    let vigente = linhas
        .iter()
        .find(|t| t.vigente_desde <= hoje)
        .map(|t| t.id);
    let futura = linhas.iter().find(|t| t.vigente_desde > hoje).map(|t| t.id);
    let versoes = linhas
        .into_iter()
        .map(|t| VersaoDaTabelaDePreco {
            situacao: if Some(t.id) == vigente {
                "vigente"
            } else if t.vigente_desde > hoje {
                "futura"
            } else {
                "anterior"
            }
            .to_owned(),
            id: t.id,
            versao: t.versao,
            vigente_desde: t.vigente_desde,
            descricao: t.descricao,
            motivo: t.motivo,
            itens: t.itens,
            publicada_por: t.publicada_por,
            publicada_em: t.criado_em,
        })
        .collect();
    Ok((versoes, vigente, futura))
}

async fn carregar(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    tuss: Option<&str>,
) -> Result<CatalogoDeExames, ErroApi> {
    let (tabelas, vigente, futura) = tabelas(tx, unidade).await?;
    let procedimentos = sqlx::query!(
        r#"SELECT pc.tuss, pc.nome, pc.modalidade, pc.duracao_min, pc.requer_contraste, pc.ativo, pc.versao,
                  (SELECT p.valor_centavos FROM financeiro.preco p WHERE p.tabela_id = $1 AND p.tuss = pc.tuss) AS preco,
                  (SELECT p.valor_centavos FROM financeiro.preco p WHERE p.tabela_id = $2 AND p.tuss = pc.tuss) AS proximo
             FROM agenda.procedimento pc
            WHERE ($3::text IS NULL OR pc.tuss = $3)
            ORDER BY pc.ativo DESC, pc.nome LIMIT 1000"#,
        vigente,
        futura,
        tuss
    )
    .fetch_all(&mut *tx)
    .await?;
    Ok(CatalogoDeExames {
        procedimentos: procedimentos
            .into_iter()
            .map(|p| ProcedimentoDoCatalogo {
                tuss: p.tuss,
                nome: p.nome,
                modalidade: p.modalidade,
                duracao_min: p.duracao_min,
                requer_contraste: p.requer_contraste,
                ativo: p.ativo,
                versao: p.versao,
                preco_centavos: p.preco,
                proximo_preco_centavos: p.proximo,
            })
            .collect(),
        tabelas,
    })
}

async fn um(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    tuss: &str,
) -> Result<ProcedimentoDoCatalogo, ErroApi> {
    carregar(tx, unidade, Some(tuss))
        .await?
        .procedimentos
        .pop()
        .ok_or_else(|| {
            ErroApi::nao_encontrado(
                "procedimento-nao-encontrado",
                "Procedimento não encontrado no catálogo. Nada foi alterado.",
            )
        })
}

/// Catálogo de exames com o preço vigente desta unidade e as versões da tabela (recepção e gerência).
#[utoipa::path(get, operation_id = "listar_catalogo_de_exames", path = "/v1/catalogo/procedimentos", tag = "catalogo",
    responses((status = 200, body = CatalogoDeExames), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<CatalogoDeExames>, ErroApi> {
    usuario.exigir(Ato::CatalogoVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let r = carregar(&mut tx, usuario.unidade_id, None).await?;
    tx.commit().await?;
    Ok(Json(r))
}

/// Histórico de mudanças do procedimento, com motivo e quem fez (recepção e gerência).
#[utoipa::path(get, operation_id = "historico_do_procedimento", path = "/v1/catalogo/procedimentos/{tuss}/historico", tag = "catalogo",
    params(("tuss" = String, Path)),
    responses((status = 200, body = Vec<MudancaRegistradaNoCatalogo>), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn historico(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(tuss): Path<String>,
) -> Result<Json<Vec<MudancaRegistradaNoCatalogo>>, ErroApi> {
    usuario.exigir(Ato::CatalogoVer)?;
    if !tuss_valido(&tuss) {
        return Err(ErroApi::nao_encontrado(
            "procedimento-nao-encontrado",
            "Procedimento não encontrado no catálogo.",
        ));
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    um(&mut tx, usuario.unidade_id, &tuss).await?;
    let linhas = sqlx::query!(
        r#"SELECT m.codigo, m.tipo, m.motivo, m.versao_do_procedimento, m.antes, m.depois, u.nome AS quem, m.criado_em
             FROM agenda.procedimento_mudanca m JOIN identidade.usuario u ON u.id = m.criado_por
            WHERE m.tuss = $1 ORDER BY m.versao_do_procedimento DESC LIMIT 200"#,
        tuss
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let mut r = Vec::with_capacity(linhas.len());
    for l in linhas {
        r.push(MudancaRegistradaNoCatalogo {
            codigo: l.codigo,
            tipo: l.tipo,
            motivo: l.motivo,
            versao_do_procedimento: l.versao_do_procedimento,
            antes: l
                .antes
                .map(serde_json::from_value)
                .transpose()
                .map_err(ErroApi::interno)?,
            depois: serde_json::from_value(l.depois).map_err(ErroApi::interno)?,
            quem: l.quem,
            em: l.criado_em,
        });
    }
    Ok(Json(r))
}

pub mod mudancas;
pub mod precos;
