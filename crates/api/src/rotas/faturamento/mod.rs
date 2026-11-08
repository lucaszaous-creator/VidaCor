//! Faturamento TISS com auditoria antes do lote (docs/22 E5, docs/21 § 3.9, DEC-2026-11-08-65).
//!
//! - `conferencia`: apura as contas dos atendimentos com guia consumida, confere pelas regras da
//!   operadora e corrige com versão;
//! - `consultas`: listas e painéis (guias, auditoria, contratos);
//! - `lotes`: lote com número atômico por contrato, arquivo TISS, envio pela porta `Operadora` e
//!   demonstrativo de retorno;
//! - `glosas`: glosas recebidas e recurso com justificativa.
//!
//! As regras e o XML ficam no domínio (`vidacor_dominio_faturamento`); aqui só se lê e grava.

pub mod conferencia;
pub mod consultas;
pub mod contrato;
pub mod glosas;
pub mod lotes;

use crate::erro::ErroApi;
use crate::lastro::{self, NovoEvento};
use crate::tempo;
use chrono::NaiveDate;
use serde_json::json;
use sqlx::PgConnection;
use std::collections::BTreeMap;
use uuid::Uuid;
use vidacor_dominio_faturamento::auditoria::{self, conjunto_padrao, ConjuntoDeRegras};
use vidacor_dominio_faturamento::{
    Centavos, ContextoAuditoria, GuiaFaturavel, ItemAutorizado, ItemExecutado, Pendencia,
    Profissional,
};

/// Situações em que a conta ainda pode ser conferida e corrigida (fora de lote).
pub(crate) const SITUACOES_ABERTAS: [&str; 3] = ["em_conferencia", "pronta", "com_pendencia"];

pub(crate) fn conta_nao_encontrada() -> ErroApi {
    ErroApi::nao_encontrado(
        "conta-nao-encontrada",
        "Esta guia não existe no faturamento desta unidade. Nada foi alterado. Volte à lista de guias.",
    )
}

/// Dados do contrato usados na conferência e no lote.
pub(crate) struct ContratoDaConta {
    pub id: Uuid,
    pub registro_ans: String,
    pub codigo_prestador: String,
    pub nome_prestador: String,
    pub cnes: String,
    pub regra_versao: String,
    pub prazo_envio_dias: i32,
}

pub(crate) async fn contrato_por_id(
    conexao: &mut PgConnection,
    id: Uuid,
) -> Result<Option<ContratoDaConta>, sqlx::Error> {
    sqlx::query_as!(
        ContratoDaConta,
        "SELECT id, registro_ans, codigo_prestador, nome_prestador, cnes, regra_versao, prazo_envio_dias
           FROM faturamento.contrato WHERE id = $1 AND ativo",
        id
    )
    .fetch_optional(conexao)
    .await
}

/// Contrato ativo da unidade para a operadora escrita na guia (sem diferenciar maiúsculas).
pub(crate) async fn contrato_da_operadora(
    conexao: &mut PgConnection,
    unidade: Uuid,
    operadora: &str,
) -> Result<Option<ContratoDaConta>, sqlx::Error> {
    sqlx::query_as!(
        ContratoDaConta,
        "SELECT id, registro_ans, codigo_prestador, nome_prestador, cnes, regra_versao, prazo_envio_dias
           FROM faturamento.contrato
          WHERE unidade_id = $1 AND lower(trim(operadora)) = lower(trim($2)) AND ativo",
        unidade,
        operadora
    )
    .fetch_optional(conexao)
    .await
}

/// Tabela de preços do contrato vigente na data do exame e seus preços por TUSS.
pub(crate) async fn precos_vigentes(
    conexao: &mut PgConnection,
    contrato: Uuid,
    data: NaiveDate,
) -> Result<(Option<Uuid>, BTreeMap<String, Centavos>), sqlx::Error> {
    let tabela = sqlx::query_scalar!(
        "SELECT id FROM faturamento.tabela WHERE contrato_id = $1 AND vigente_desde <= $2
          ORDER BY vigente_desde DESC, versao DESC LIMIT 1",
        contrato,
        data
    )
    .fetch_optional(&mut *conexao)
    .await?;
    let Some(tabela) = tabela else {
        return Ok((None, BTreeMap::new()));
    };
    let precos = sqlx::query!(
        "SELECT tuss, valor_centavos FROM faturamento.preco WHERE tabela_id = $1",
        tabela
    )
    .fetch_all(&mut *conexao)
    .await?
    .into_iter()
    .map(|p| (p.tuss, Centavos(p.valor_centavos)))
    .collect();
    Ok((Some(tabela), precos))
}

/// Conjunto de regras da versão gravada no contrato (sem contrato: o padrão, que bloqueia).
pub(crate) fn regras_do_contrato(
    contrato: Option<&ContratoDaConta>,
) -> Result<&'static ConjuntoDeRegras, ErroApi> {
    match contrato {
        None => Ok(conjunto_padrao()),
        Some(c) => auditoria::conjunto(&c.regra_versao).ok_or_else(|| {
            ErroApi::interno(format!(
                "regra de auditoria desconhecida no contrato: {}",
                c.regra_versao
            ))
        }),
    }
}

/// Conta montada como guia do domínio, com o contrato (se houver).
pub(crate) struct ContaMontada {
    pub guia: GuiaFaturavel,
    pub contrato: Option<ContratoDaConta>,
    pub situacao: String,
    pub em_lote: bool,
    pub codigo: String,
}

/// Lê a conta, os itens, a autorização da guia e a situação do laudo (sem conteúdo clínico: só a
/// palavra da situação, pela função própria do laudo).
pub(crate) async fn montar_conta(
    conexao: &mut PgConnection,
    conta: Uuid,
    travar: bool,
) -> Result<Option<ContaMontada>, ErroApi> {
    if travar {
        sqlx::query_scalar!(
            "SELECT id FROM faturamento.conta WHERE id = $1 FOR UPDATE",
            conta
        )
        .fetch_optional(&mut *conexao)
        .await?;
    }
    let c = sqlx::query!(
        "SELECT id, codigo, atendimento_id, guia_id, contrato_id, operadora, unidade_id, data_execucao,
                numero_guia_operadora, senha, data_autorizacao, validade_senha, carteirinha,
                solicitante_nome, solicitante_conselho, solicitante_numero, solicitante_uf, solicitante_cbos,
                cid, assinatura_beneficiario, situacao, lote_id
           FROM faturamento.conta WHERE id = $1",
        conta
    )
    .fetch_optional(&mut *conexao)
    .await?;
    let Some(c) = c else { return Ok(None) };
    let contrato = match c.contrato_id {
        Some(id) => contrato_por_id(conexao, id).await?,
        None => contrato_da_operadora(conexao, c.unidade_id, &c.operadora).await?,
    };
    let itens = sqlx::query!(
        "SELECT tuss, descricao, quantidade, valor_unitario_centavos FROM faturamento.conta_item
          WHERE conta_id = $1 ORDER BY sequencial",
        conta
    )
    .fetch_all(&mut *conexao)
    .await?;
    let autorizados = sqlx::query!(
        "SELECT tuss, autorizado FROM convenio.guia_procedimento WHERE guia_id = $1 ORDER BY tuss",
        c.guia_id
    )
    .fetch_all(&mut *conexao)
    .await?;
    let laudo = sqlx::query_scalar!(
        "SELECT laudos.situacao_do_atendimento($1)",
        c.atendimento_id
    )
    .fetch_one(&mut *conexao)
    .await?;
    let quantidade = |q: i32| u16::try_from(q).unwrap_or(u16::MAX);
    let guia = GuiaFaturavel {
        numero_guia_prestador: c.codigo.clone(),
        registro_ans: contrato.as_ref().map(|k| k.registro_ans.clone()),
        codigo_prestador: contrato.as_ref().map(|k| k.codigo_prestador.clone()),
        cnes: contrato.as_ref().map(|k| k.cnes.clone()),
        numero_guia_operadora: c.numero_guia_operadora,
        senha: c.senha,
        data_autorizacao: c.data_autorizacao,
        validade_senha: c.validade_senha,
        carteirinha: c.carteirinha,
        solicitante: Profissional {
            nome: c.solicitante_nome,
            conselho: c.solicitante_conselho,
            numero: c.solicitante_numero,
            uf: c.solicitante_uf,
            cbos: c.solicitante_cbos,
        },
        data_execucao: c.data_execucao,
        executados: itens
            .into_iter()
            .map(|i| ItemExecutado {
                tuss: i.tuss,
                descricao: i.descricao,
                quantidade: quantidade(i.quantidade),
                valor_unitario: i.valor_unitario_centavos.map(Centavos),
            })
            .collect(),
        autorizados: autorizados
            .into_iter()
            .map(|a| ItemAutorizado {
                tuss: a.tuss,
                quantidade: quantidade(a.autorizado),
            })
            .collect(),
        // Laudo vigente assinado (a retificação em rascunho mantém a versão assinada vigente).
        laudo_assinado: matches!(
            laudo.as_deref(),
            Some("assinado" | "retificacao_em_rascunho")
        ),
        cid: c.cid,
        assinatura_beneficiario: c.assinatura_beneficiario,
    };
    Ok(Some(ContaMontada {
        guia,
        contrato,
        situacao: c.situacao,
        em_lote: c.lote_id.is_some(),
        codigo: c.codigo,
    }))
}

/// Confere a conta (já travada pelo chamador ou travada aqui), grava as pendências com a versão da
/// regra e a situação. Conta em lote não muda. Devolve as pendências abertas.
pub(crate) async fn auditar_conta(
    conexao: &mut PgConnection,
    conta: Uuid,
    usuario: Uuid,
    correlacao: &str,
) -> Result<Vec<Pendencia>, ErroApi> {
    let Some(m) = montar_conta(conexao, conta, true).await? else {
        return Err(conta_nao_encontrada());
    };
    if m.em_lote {
        return Ok(Vec::new());
    }
    let regras = regras_do_contrato(m.contrato.as_ref())?;
    let (_, precos) = match &m.contrato {
        Some(k) => precos_vigentes(conexao, k.id, m.guia.data_execucao).await?,
        None => (None, BTreeMap::new()),
    };
    let ctx = ContextoAuditoria {
        hoje: tempo::hoje_local(),
        prazo_envio_dias: m
            .contrato
            .as_ref()
            .and_then(|k| u16::try_from(k.prazo_envio_dias).ok()),
        precos_vigentes: precos,
    };
    let pendencias = vidacor_dominio_faturamento::auditar(&m.guia, &ctx, regras);
    let pronta = auditoria::pronta(&pendencias);
    sqlx::query!(
        "UPDATE faturamento.pendencia SET situacao = 'superada', superada_em = now()
          WHERE conta_id = $1 AND situacao = 'aberta'",
        conta
    )
    .execute(&mut *conexao)
    .await?;
    for p in &pendencias {
        sqlx::query!(
            "INSERT INTO faturamento.pendencia (id, conta_id, regra, regra_versao, efeito, origem_regra, titulo, detalhe,
                    o_que_fazer, criado_por, origem, unidade_id, correlacao)
             SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'faturamento', unidade_id, $11
               FROM faturamento.conta WHERE id = $2",
            vidacor_comum::novo_id(), conta, p.regra.codigo(), regras.versao, p.efeito.codigo(), p.origem,
            p.titulo, p.detalhe.chars().take(500).collect::<String>(), p.o_que_fazer, usuario, correlacao
        )
        .execute(&mut *conexao)
        .await?;
    }
    let situacao = if pronta { "pronta" } else { "com_pendencia" };
    let unidade = sqlx::query_scalar!(
        "UPDATE faturamento.conta SET situacao = $2, regra_versao = $3, auditada_em = now(), auditada_por = $4,
                contrato_id = coalesce(contrato_id, $5), versao = versao + 1
          WHERE id = $1 RETURNING unidade_id",
        conta,
        situacao,
        regras.versao,
        usuario,
        m.contrato.as_ref().map(|k| k.id)
    )
    .fetch_one(&mut *conexao)
    .await?;
    lastro::registrar(
        conexao,
        NovoEvento {
            dominio: "faturamento",
            tipo: "conta_conferida",
            registro_id: conta,
            registro_codigo: Some(&m.codigo),
            dados: json!({
                "regra_versao": regras.versao,
                "situacao": situacao,
                "anterior": m.situacao,
                "regras": pendencias.iter().map(|p| p.regra.codigo()).collect::<Vec<_>>(),
            }),
            criado_por: Some(usuario),
            origem: "faturamento",
            unidade_id: unidade,
            correlacao,
        },
    )
    .await?;
    Ok(pendencias)
}
