//! Check-up e pacotes (Checkup-Pacotes.dc.html; docs/04 Agenda). O pacote é a regra de combinação
//! versionada que o orçamento já usa (`financeiro.pacote`): cada versão tem os exames na ordem do
//! roteiro, o preparo de cada um e o preço. O servidor calcula o avulso pela tabela do particular e
//! confere que o pacote sai mais barato, dentro do limite de desconto (G1). A recepção consulta; a
//! gerência publica versão nova, que começa numa data futura e nunca altera a vigente.
//! DEC-2026-11-09-115 a 117.

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::State;
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_dominio_recepcao::pacote::{avaliar, DESCONTO_MAXIMO_PB};

pub mod publicar;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ExameDoPacoteDeCheckup {
    /// Posição no roteiro do dia (1 é o primeiro).
    pub ordem: i32,
    pub tuss: String,
    pub nome: String,
    pub duracao_min: i32,
    /// Preparo para o paciente (nulo nas versões anteriores ao cadastro de roteiro).
    pub preparo: Option<String>,
    /// Por que vem nesta posição.
    pub porque: Option<String>,
    /// Preço avulso na tabela do particular da data de referência (nulo sem preço).
    pub avulso_centavos: Option<i64>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct VersaoDoPacoteDeCheckup {
    pub id: Uuid,
    pub regra_versao: i32,
    pub nome: String,
    pub para_quem: Option<String>,
    pub vigente_desde: NaiveDate,
    pub vigente_ate: Option<NaiveDate>,
    pub preco_centavos: i64,
    pub motivo: Option<String>,
    pub publicada_por: Option<String>,
    pub publicada_em: DateTime<Utc>,
    /// vigente, futura ou anterior.
    pub situacao: String,
    /// Soma avulsa pela tabela do particular (hoje para a vigente; no início para as outras). Nula
    /// quando algum exame não tem preço.
    pub avulso_centavos: Option<i64>,
    pub economia_centavos: Option<i64>,
    pub desconto_pontos_base: Option<i64>,
    /// O orçamento aplica o pacote (ele sai mais barato que o avulso).
    pub vale_no_orcamento: bool,
    pub duracao_total_min: i32,
    pub exames: Vec<ExameDoPacoteDeCheckup>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PacoteDeCheckup {
    pub codigo: String,
    /// Nome da versão vigente (ou da mais nova).
    pub nome: String,
    /// Versões da mais nova para a mais antiga.
    pub versoes: Vec<VersaoDoPacoteDeCheckup>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CatalogoDePacotes {
    pub pacotes: Vec<PacoteDeCheckup>,
    /// Maior desconto do pacote sobre o avulso, em pontos base.
    pub desconto_maximo_pontos_base: i64,
}

/// Tabela do particular que vale numa data: a maior versão já iniciada (mesma regra do caixa).
pub(crate) async fn precos_na_data(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    data: NaiveDate,
) -> Result<BTreeMap<String, i64>, ErroApi> {
    Ok(sqlx::query!(
        "SELECT p.tuss, p.valor_centavos FROM financeiro.preco p
          WHERE p.tabela_id = (SELECT t.id FROM financeiro.tabela_preco t WHERE t.unidade_id = $1 AND t.vigente_desde <= $2
                                ORDER BY t.versao DESC LIMIT 1)",
        unidade,
        data
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|p| (p.tuss, p.valor_centavos))
    .collect())
}

/// Pacotes da unidade com todas as versões (ou só um código).
pub(crate) async fn carregar(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    codigo: Option<&str>,
) -> Result<Vec<PacoteDeCheckup>, ErroApi> {
    let hoje = tempo::hoje_local();
    let versoes = sqlx::query!(
        r#"SELECT k.id, k.codigo, k.nome, k.regra_versao, k.para_quem, k.vigente_desde, k.vigente_ate, k.valor_centavos,
                  k.motivo, k.criado_em, u.nome AS "publicada_por?"
             FROM financeiro.pacote k LEFT JOIN identidade.usuario u ON u.id = k.criado_por
            WHERE k.unidade_id = $1 AND ($2::text IS NULL OR k.codigo = $2)
            ORDER BY k.codigo, k.regra_versao DESC LIMIT 500"#,
        unidade,
        codigo
    )
    .fetch_all(&mut *tx)
    .await?;
    let ids: Vec<Uuid> = versoes.iter().map(|v| v.id).collect();
    let exames = sqlx::query!(
        r#"SELECT k.id AS pacote_id, x.ordem::int AS "ordem!", x.tuss AS "tuss!", pc.nome, pc.duracao_min, i.preparo AS "preparo?", i.porque AS "porque?"
             FROM financeiro.pacote k
             CROSS JOIN LATERAL unnest(k.tuss) WITH ORDINALITY AS x(tuss, ordem)
             JOIN agenda.procedimento pc ON pc.tuss = x.tuss
             LEFT JOIN financeiro.pacote_item i ON i.pacote_id = k.id AND i.tuss = x.tuss
            WHERE k.id = ANY($1)
            ORDER BY k.id, coalesce(i.ordem, x.ordem::int)"#,
        &ids
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut tabelas: BTreeMap<NaiveDate, BTreeMap<String, i64>> = BTreeMap::new();
    let mut por_codigo: BTreeMap<String, PacoteDeCheckup> = BTreeMap::new();
    // Vigente de cada código: a maior versão já iniciada e não encerrada (mesma regra do orçamento).
    let mut vigentes: BTreeMap<String, Uuid> = BTreeMap::new();
    for v in &versoes {
        if v.vigente_desde <= hoje && v.vigente_ate.is_none_or(|a| a >= hoje) {
            vigentes.entry(v.codigo.clone()).or_insert(v.id);
        }
    }
    for v in versoes {
        let vigente = vigentes.get(&v.codigo) == Some(&v.id);
        // A vigente vale pelo preço avulso de hoje; a futura e as anteriores, pelo do seu início.
        let referencia = if vigente { hoje } else { v.vigente_desde };
        if let std::collections::btree_map::Entry::Vacant(e) = tabelas.entry(referencia) {
            e.insert(precos_na_data(tx, unidade, referencia).await?);
        }
        let precos = tabelas.get(&referencia);
        let lista: Vec<ExameDoPacoteDeCheckup> = exames
            .iter()
            .filter(|e| e.pacote_id == v.id)
            .enumerate()
            .map(|(k, e)| ExameDoPacoteDeCheckup {
                ordem: i32::try_from(k + 1).unwrap_or(e.ordem),
                tuss: e.tuss.clone(),
                nome: e.nome.clone(),
                duracao_min: e.duracao_min,
                preparo: e.preparo.clone(),
                porque: e.porque.clone(),
                avulso_centavos: precos.and_then(|p| p.get(&e.tuss).copied()),
            })
            .collect();
        let avulso: Option<i64> = lista.iter().map(|e| e.avulso_centavos).sum();
        let economia = avulso.and_then(|a| avaliar(a, v.valor_centavos));
        let situacao = if vigente {
            "vigente"
        } else if v.vigente_desde > hoje {
            "futura"
        } else {
            "anterior"
        };
        let versao = VersaoDoPacoteDeCheckup {
            id: v.id,
            regra_versao: v.regra_versao,
            nome: v.nome.clone(),
            para_quem: v.para_quem,
            vigente_desde: v.vigente_desde,
            vigente_ate: v.vigente_ate,
            preco_centavos: v.valor_centavos,
            motivo: v.motivo,
            publicada_por: v.publicada_por,
            publicada_em: v.criado_em,
            situacao: situacao.to_owned(),
            avulso_centavos: avulso,
            economia_centavos: economia.as_ref().map(|e| e.economia_centavos),
            desconto_pontos_base: economia.as_ref().map(|e| e.desconto_pb),
            vale_no_orcamento: economia.is_some(),
            duracao_total_min: lista.iter().map(|e| e.duracao_min).sum(),
            exames: lista,
        };
        let p = por_codigo
            .entry(v.codigo.clone())
            .or_insert_with(|| PacoteDeCheckup {
                codigo: v.codigo.clone(),
                nome: v.nome.clone(),
                versoes: Vec::new(),
            });
        if vigente {
            p.nome = v.nome;
        }
        p.versoes.push(versao);
    }
    let mut r: Vec<PacoteDeCheckup> = por_codigo.into_values().collect();
    r.sort_by(|a, b| a.nome.cmp(&b.nome));
    Ok(r)
}

/// Pacotes e check-ups da unidade, com o preço do pacote, o avulso calculado pelo servidor e as
/// versões (recepção e gerência).
#[utoipa::path(get, operation_id = "listar_pacotes_de_checkup", path = "/v1/pacotes", tag = "pacotes",
    responses((status = 200, body = CatalogoDePacotes), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<CatalogoDePacotes>, ErroApi> {
    usuario.exigir(Ato::PacotesVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let pacotes = carregar(&mut tx, usuario.unidade_id, None).await?;
    tx.commit().await?;
    Ok(Json(CatalogoDePacotes {
        pacotes,
        desconto_maximo_pontos_base: DESCONTO_MAXIMO_PB,
    }))
}
