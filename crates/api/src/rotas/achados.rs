//! Laudo estruturado com classificações (docs/22 E2, docs/25 § 3.3, DEC-2026-11-08-60): achados
//! TI-RADS e BI-RADS ligados à versão do laudo. O núcleo calcula categoria, conduta e texto pela
//! regra do domínio e grava a versão da regra (G1); o que a tela manda como categoria é ignorado.
//! Achado só entra e sai no rascunho; laudo assinado é imutável e a correção é pela retificação,
//! que leva os achados para a nova versão (CLAUDE.md, regras clínicas).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::achados_contrato::{
    Achado, AchadosDoLaudo, NovoAchado, RegraVigente, SistemaClassificacao,
};
use crate::rotas::laudos::{self as rotas_laudos, Laudo};
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use sqlx::PgConnection;
use uuid::Uuid;
use vidacor_comum::novo_id;
use vidacor_dominio_laudo::{birads, localizacao, tirads, ErroClassificacao, Medida};

/// Achados ativos por versão do laudo (limite de disponibilidade, SEC-DIS).
pub const LIMITE_ACHADOS: i64 = 20;

/// Resultado do domínio pronto para gravar.
struct Calculado {
    sistema: SistemaClassificacao,
    localizacao: String,
    maior: Option<Medida>,
    descritores: Value,
    pontos: Option<i32>,
    categoria: &'static str,
    categoria_nome: &'static str,
    conduta: String,
    regra_versao: &'static str,
    texto_achado: String,
    texto_conclusao: String,
}

fn invalido(e: &ErroClassificacao) -> ErroApi {
    ErroApi::invalido(e.codigo(), &format!("{e} Nada foi gravado."))
}

/// Aplica a regra do domínio ao pedido (sem banco). O rótulo entra no texto (N1, N2...).
fn calcular(p: &NovoAchado, rotulo: &str) -> Result<Calculado, ErroApi> {
    let local = localizacao(&p.localizacao).map_err(|e| invalido(&e))?;
    let sem_descritores = || {
        ErroApi::invalido(
            "descritores-ausentes",
            "Faltaram os descritores do sistema escolhido. Nada foi gravado. Marque as opções do achado e tente de novo.",
        )
    };
    match p.sistema {
        SistemaClassificacao::Tirads => {
            let d = p.tirads.as_ref().ok_or_else(sem_descritores)?;
            let maior = p
                .maior_diametro_mm
                .ok_or(ErroClassificacao::MedidaInvalida {
                    limite_mm: tirads::LIMITE_MM,
                })
                .and_then(|mm| Medida::de_milimetros(mm, tirads::LIMITE_MM))
                .map_err(|e| invalido(&e))?;
            let (n, r) = tirads::classificar(&d.into(), maior).map_err(|e| invalido(&e))?;
            Ok(Calculado {
                sistema: p.sistema,
                texto_achado: tirads::texto_achado(rotulo, &local, maior, &n, &r),
                texto_conclusao: tirads::texto_conclusao(rotulo, &r),
                localizacao: local,
                maior: Some(maior),
                descritores: serde_json::to_value(&n).map_err(ErroApi::interno)?,
                pontos: Some(i32::from(r.pontos)),
                categoria: r.categoria.sigla(),
                categoria_nome: r.categoria.nome(),
                conduta: r.conduta.texto(),
                regra_versao: r.versao,
            })
        }
        SistemaClassificacao::Birads => {
            let d = p.birads.as_ref().ok_or_else(sem_descritores)?;
            let maior = p
                .maior_diametro_mm
                .map(|mm| Medida::de_milimetros(mm, birads::LIMITE_MM))
                .transpose()
                .map_err(|e| invalido(&e))?;
            let (a, r) = birads::classificar(&d.into(), maior).map_err(|e| invalido(&e))?;
            Ok(Calculado {
                sistema: p.sistema,
                texto_achado: birads::texto_achado(rotulo, &local, maior, &a, &r),
                texto_conclusao: birads::texto_conclusao(rotulo, &r),
                localizacao: local,
                maior,
                descritores: serde_json::to_value(&a).map_err(ErroApi::interno)?,
                pontos: None,
                categoria: r.categoria.sigla(),
                categoria_nome: r.categoria.nome(),
                conduta: format!(
                    "{}; probabilidade de malignidade: {}",
                    r.categoria.conduta(),
                    r.categoria.probabilidade()
                ),
                regra_versao: r.versao,
            })
        }
    }
}

fn rotulo(numero: i32) -> String {
    format!("N{numero}")
}

fn laudo_assinado() -> ErroApi {
    ErroApi::conflito(
        "laudo-assinado",
        "Este laudo está assinado e não recebe nem perde achados. Nada foi alterado. Para corrigir, faça uma retificação com motivo: os achados vão para a nova versão.",
    )
}

/// Versão em rascunho do laudo (trava o laudo, na mesma ordem da assinatura, para que achado e
/// assinatura não se cruzem: G2).
async fn rascunho_travado(conexao: &mut PgConnection, laudo: Uuid) -> Result<i32, ErroApi> {
    sqlx::query_scalar!(
        "SELECT id FROM laudos.laudo WHERE id = $1 FOR UPDATE",
        laudo
    )
    .fetch_one(&mut *conexao)
    .await?;
    sqlx::query_scalar!(
        "SELECT numero FROM laudos.versao WHERE laudo_id = $1 AND situacao = 'rascunho'",
        laudo
    )
    .fetch_optional(&mut *conexao)
    .await?
    .ok_or_else(laudo_assinado)
}

async fn carregar_achados(
    conexao: &mut PgConnection,
    laudo: Uuid,
    versao: i32,
    so_id: Option<Uuid>,
) -> Result<Vec<Achado>, ErroApi> {
    let linhas = sqlx::query!(
        r#"SELECT id, codigo, numero, versao_numero, sistema, localizacao, maior_diametro_dmm, descritores,
                  pontos, categoria, categoria_nome, conduta, regra_versao, texto_achado, texto_conclusao, criado_em
             FROM laudos.achado
            WHERE laudo_id = $1 AND versao_numero = $2 AND situacao = 'ativo' AND ($3::uuid IS NULL OR id = $3)
            ORDER BY numero"#,
        laudo,
        versao,
        so_id
    )
    .fetch_all(&mut *conexao)
    .await?;
    linhas
        .into_iter()
        .map(|l| {
            let sistema = match l.sistema.as_str() {
                "tirads" => SistemaClassificacao::Tirads,
                "birads" => SistemaClassificacao::Birads,
                outro => return Err(ErroApi::interno(format!("sistema desconhecido: {outro}"))),
            };
            let (tirads, birads) = match sistema {
                SistemaClassificacao::Tirads => (
                    Some(serde_json::from_value(l.descritores).map_err(ErroApi::interno)?),
                    None,
                ),
                SistemaClassificacao::Birads => (
                    None,
                    Some(serde_json::from_value(l.descritores).map_err(ErroApi::interno)?),
                ),
            };
            Ok(Achado {
                id: l.id,
                codigo: l.codigo,
                numero: l.numero,
                rotulo: rotulo(l.numero),
                versao_numero: l.versao_numero,
                sistema,
                localizacao: l.localizacao,
                maior_diametro_mm: l.maior_diametro_dmm.map(|d| f64::from(d) / 10.0),
                tirads,
                birads,
                pontos: l.pontos,
                categoria: l.categoria,
                categoria_nome: l.categoria_nome,
                conduta: l.conduta,
                regra_versao: l.regra_versao,
                texto_achado: l.texto_achado,
                texto_conclusao: l.texto_conclusao,
                criado_em: l.criado_em,
            })
        })
        .collect()
}

/// Achados da versão em rascunho (ou da vigente, se não houver rascunho).
async fn montar_lista(
    conexao: &mut PgConnection,
    laudo: &Laudo,
    usuario: Uuid,
) -> Result<AchadosDoLaudo, ErroApi> {
    let rascunho = laudo.versoes.iter().find(|v| v.situacao == "rascunho");
    let versao = rascunho
        .map(|v| v.numero)
        .or(laudo.versao_vigente)
        .unwrap_or(1);
    let achados = carregar_achados(conexao, laudo.id, versao, None).await?;
    let juntar = |f: fn(&Achado) -> &str| achados.iter().map(f).collect::<Vec<_>>().join("\n");
    Ok(AchadosDoLaudo {
        laudo_id: laudo.id,
        versao_numero: versao,
        editavel: rascunho.is_some() && laudo.medico_atribuido_id == Some(usuario),
        texto_achados: juntar(|a| &a.texto_achado),
        texto_conclusao: juntar(|a| &a.texto_conclusao),
        achados,
        regras_vigentes: vec![
            RegraVigente {
                sistema: SistemaClassificacao::Tirads,
                versao: tirads::VERSAO.to_owned(),
            },
            RegraVigente {
                sistema: SistemaClassificacao::Birads,
                versao: birads::VERSAO.to_owned(),
            },
        ],
    })
}

/// Achados estruturados do laudo (médico com vínculo de cuidado ou acesso de emergência). Toda
/// leitura é auditada, como a do laudo.
#[utoipa::path(get, operation_id = "listar_achados_do_laudo", path = "/v1/laudos/{id}/achados", tag = "laudos",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = AchadosDoLaudo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<AchadosDoLaudo>, ErroApi> {
    usuario.exigir(Ato::LaudoLer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let Some(laudo) = rotas_laudos::carregar(&mut tx, id).await? else {
        let erro = rotas_laudos::negado(&mut tx, usuario.id, id).await?;
        tx.commit().await?;
        return Err(erro);
    };
    let emergencia = !crate::vinculo::tem_vinculo(&mut tx, laudo.paciente.id).await?;
    rotas_laudos::auditar(
        &mut tx,
        usuario.id,
        id,
        if emergencia {
            "emergencia"
        } else {
            "permitido"
        },
        None,
    )
    .await?;
    let lista = montar_lista(&mut tx, &laudo, usuario.id).await?;
    tx.commit().await?;
    Ok(Json(lista))
}

/// Registra um achado no rascunho (só o médico atribuído). O servidor calcula pontos, categoria,
/// conduta e texto e grava a versão da regra; categoria vinda da tela é ignorada (G1).
#[utoipa::path(post, operation_id = "criar_achado_do_laudo", path = "/v1/laudos/{id}/achados", tag = "laudos",
    params(("id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)), request_body = NovoAchado,
    responses((status = 201, body = Achado), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn criar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<NovoAchado>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::LaudoEscrever)?;
    // Confere antes de abrir a transação: erro de preenchimento não toca no banco.
    calcular(&p, "N1")?;
    let rota = format!("POST /v1/laudos/{id}/achados");
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
    let laudo = rotas_laudos::do_medico_atribuido(&mut tx, &usuario, id).await?;
    let versao = rascunho_travado(&mut tx, id).await?;
    let contagem = sqlx::query!(
        r#"SELECT count(*) FILTER (WHERE situacao = 'ativo') AS "ativos!", coalesce(max(numero), 0) AS "maior!"
             FROM laudos.achado WHERE laudo_id = $1 AND versao_numero = $2"#,
        id,
        versao
    )
    .fetch_one(&mut *tx)
    .await?;
    if contagem.ativos >= LIMITE_ACHADOS {
        return Err(ErroApi::invalido(
            "limite-de-achados",
            "Este laudo já tem 20 achados estruturados. Nada foi gravado. Descreva os demais no texto do laudo.",
        ));
    }
    let numero = contagem.maior + 1;
    let c = calcular(&p, &rotulo(numero))?;
    let codigo = crate::rotas::caixa::proximo(&mut tx, "ACH").await?;
    let achado = novo_id();
    sqlx::query!(
        "INSERT INTO laudos.achado (id, codigo, laudo_id, versao_numero, numero, sistema, localizacao, maior_diametro_dmm,
                descritores, pontos, categoria, categoria_nome, conduta, regra_versao, texto_achado, texto_conclusao,
                criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, 'consultorio', $18, $19)",
        achado, codigo.como_str(), id, versao, numero, c.sistema.codigo(), c.localizacao,
        c.maior.map(|m| i32::try_from(m.decimos()).unwrap_or(i32::MAX)), c.descritores, c.pontos,
        c.categoria, c.categoria_nome, c.conduta, c.regra_versao, c.texto_achado, c.texto_conclusao,
        usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    // Lastro sem conteúdo clínico (docs/21): ids, códigos, sistema e versão da regra.
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "laudos", tipo: "achado_registrado", registro_id: achado, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "laudo": id, "laudo_codigo": laudo.codigo, "versao": versao, "numero": numero,
                       "sistema": c.sistema.codigo(), "regra_versao": c.regra_versao }),
        criado_por: Some(usuario.id), origem: "consultorio", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let criado = carregar_achados(&mut tx, id, versao, Some(achado))
        .await?
        .pop()
        .ok_or_else(|| ErroApi::interno("achado gravado não foi relido"))?;
    let corpo = serde_json::to_value(&criado).map_err(ErroApi::interno)?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Remove um achado do rascunho (só o médico atribuído). Nada é apagado: o achado fica marcado como
/// removido, com quem e quando. Repetir não muda nada e devolve a mesma lista.
#[utoipa::path(post, operation_id = "remover_achado_do_laudo", path = "/v1/laudos/{id}/achados/{achado_id}/remocao", tag = "laudos",
    params(("id" = Uuid, Path), ("achado_id" = Uuid, Path)),
    responses((status = 200, body = AchadosDoLaudo), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn remover(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path((id, achado_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<AchadosDoLaudo>, ErroApi> {
    usuario.exigir(Ato::LaudoEscrever)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let laudo = rotas_laudos::do_medico_atribuido(&mut tx, &usuario, id).await?;
    let atual = sqlx::query!(
        "SELECT codigo, versao_numero, situacao FROM laudos.achado WHERE id = $1 AND laudo_id = $2",
        achado_id,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "achado-nao-encontrado",
            "Este achado não existe neste laudo. Nada foi alterado.",
        )
    })?;
    let versao = rascunho_travado(&mut tx, id).await?;
    if atual.versao_numero != versao {
        return Err(laudo_assinado());
    }
    let removeu = sqlx::query!(
        "UPDATE laudos.achado SET situacao = 'removido', removido_em = now(), removido_por = $2, versao = versao + 1
          WHERE id = $1 AND situacao = 'ativo'",
        achado_id,
        usuario.id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if removeu == 1 {
        lastro::registrar(
            &mut tx,
            NovoEvento {
                dominio: "laudos",
                tipo: "achado_removido",
                registro_id: achado_id,
                registro_codigo: Some(&atual.codigo),
                dados: json!({ "laudo": id, "laudo_codigo": laudo.codigo, "versao": versao }),
                criado_por: Some(usuario.id),
                origem: "consultorio",
                unidade_id: usuario.unidade_id,
                correlacao: &correlacao,
            },
        )
        .await?;
    }
    let lista = montar_lista(&mut tx, &laudo, usuario.id).await?;
    tx.commit().await?;
    Ok(Json(lista))
}

/// Forma canônica dos achados ativos de uma versão, para entrar no hash do documento assinado.
/// Vazia quando não há achado (o hash fica igual ao do laudo só com texto).
///
/// # Errors
/// Falha de banco.
pub(crate) async fn canonico_para_assinatura(
    conexao: &mut PgConnection,
    laudo: Uuid,
    versao: i32,
) -> Result<String, sqlx::Error> {
    let linhas = sqlx::query!(
        r#"SELECT numero, sistema, regra_versao, categoria, pontos, maior_diametro_dmm, descritores::text AS "descritores!",
                  localizacao, texto_achado, texto_conclusao
             FROM laudos.achado WHERE laudo_id = $1 AND versao_numero = $2 AND situacao = 'ativo' ORDER BY numero"#,
        laudo,
        versao
    )
    .fetch_all(conexao)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| {
            format!(
                "N{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                l.numero,
                l.sistema,
                l.regra_versao,
                l.categoria,
                l.pontos.map(|p| p.to_string()).unwrap_or_default(),
                l.maior_diametro_dmm
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                l.descritores,
                l.localizacao,
                l.texto_achado,
                l.texto_conclusao
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Na retificação, os achados ativos da versão vigente passam para a nova versão em rascunho, com
/// código novo e ligação ao original (`copiado_de`). Os originais continuam imutáveis.
///
/// # Errors
/// Falha de banco.
pub(crate) async fn copiar_para_retificacao(
    conexao: &mut PgConnection,
    laudo: Uuid,
    de: i32,
    para: i32,
    usuario: &Usuario,
    correlacao: &str,
) -> Result<i64, ErroApi> {
    let originais = sqlx::query!(
        "SELECT id, numero FROM laudos.achado WHERE laudo_id = $1 AND versao_numero = $2 AND situacao = 'ativo' ORDER BY numero",
        laudo,
        de
    )
    .fetch_all(&mut *conexao)
    .await?;
    let mut copiados = 0;
    for o in originais {
        let codigo = crate::rotas::caixa::proximo(conexao, "ACH").await?;
        sqlx::query!(
            "INSERT INTO laudos.achado (id, codigo, laudo_id, versao_numero, numero, sistema, localizacao, maior_diametro_dmm,
                    descritores, pontos, categoria, categoria_nome, conduta, regra_versao, texto_achado, texto_conclusao,
                    copiado_de, criado_por, origem, unidade_id, correlacao)
             SELECT $1, $2, laudo_id, $3, numero, sistema, localizacao, maior_diametro_dmm,
                    descritores, pontos, categoria, categoria_nome, conduta, regra_versao, texto_achado, texto_conclusao,
                    id, $4, 'consultorio', unidade_id, $5
               FROM laudos.achado WHERE id = $6",
            novo_id(), codigo.como_str(), para, usuario.id, correlacao, o.id
        )
        .execute(&mut *conexao)
        .await?;
        copiados += 1;
    }
    Ok(copiados)
}
