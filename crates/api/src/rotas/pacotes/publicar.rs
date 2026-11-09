//! Pacote novo e versão nova do pacote (gerência; `Idempotency-Key`). A composição, a ordem do roteiro
//! e o preço são conferidos pelo servidor contra a tabela do particular do primeiro dia da versão.

use super::{carregar, precos_na_data, PacoteDeCheckup};
use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo};
use crate::idempotencia::{self, Reserva};
use crate::lastro::{self, NovoEvento};
use crate::rotas::caixa::proximo;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{Duration, NaiveDate};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::novo_id;
use vidacor_dominio_recepcao::pacote::{conferir, ErroDoPacote, PrecoDoPacote, DESCONTO_MAXIMO_PB};

#[derive(Deserialize, ToSchema)]
pub struct ExamePedidoNoPacote {
    pub tuss: String,
    pub preparo: String,
    /// Por que vem nesta posição do roteiro.
    pub porque: String,
}

#[derive(Deserialize, ToSchema)]
pub struct NovoPacoteDeCheckup {
    pub nome: String,
    pub para_quem: String,
    /// Primeiro dia (a partir de hoje, em até um ano).
    pub vigente_desde: NaiveDate,
    pub preco_centavos: i64,
    pub motivo: String,
    /// Exames na ordem do roteiro.
    pub exames: Vec<ExamePedidoNoPacote>,
}

#[derive(Deserialize, ToSchema)]
pub struct NovaVersaoDoPacoteDeCheckup {
    /// Versão mais nova que a tela mostrou (G2).
    pub regra_versao_vista: i32,
    pub nome: String,
    pub para_quem: String,
    /// Primeiro dia: depois de hoje e depois do início da última versão.
    pub vigente_desde: NaiveDate,
    pub preco_centavos: i64,
    pub motivo: String,
    pub exames: Vec<ExamePedidoNoPacote>,
}

struct Versao<'a> {
    nome: String,
    para_quem: String,
    motivo: String,
    vigente_desde: NaiveDate,
    preco_centavos: i64,
    exames: &'a [ExamePedidoNoPacote],
}

fn invalido(codigo: &str, detalhe: &str) -> ErroApi {
    ErroApi::invalido(codigo, &format!("{detalhe} Nada foi publicado."))
}

fn texto(t: &str, min: usize, max: usize, campo: &str) -> Result<String, ErroApi> {
    let t = t.trim();
    if (min..=max).contains(&t.chars().count()) {
        Ok(t.to_owned())
    } else {
        Err(invalido(
            "pacote-invalido",
            &format!("{campo}: de {min} a {max} letras."),
        ))
    }
}

fn validar<'a>(
    nome: &str,
    para_quem: &str,
    motivo: &str,
    vigente_desde: NaiveDate,
    preco_centavos: i64,
    exames: &'a [ExamePedidoNoPacote],
) -> Result<Versao<'a>, ErroApi> {
    let motivo = motivo.trim();
    if !(5..=300).contains(&motivo.chars().count()) {
        return Err(ErroApi::invalido(
            "motivo-invalido",
            "Escreva o motivo (5 a 300 letras). Nada foi publicado.",
        ));
    }
    for e in exames {
        if e.tuss.len() != 8 || !e.tuss.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalido(
                "pacote-invalido",
                "Código TUSS de 8 algarismos em cada exame.",
            ));
        }
        texto(&e.preparo, 3, 160, &format!("Preparo do exame {}", e.tuss))?;
        texto(
            &e.porque,
            3,
            200,
            &format!("Motivo da ordem do exame {}", e.tuss),
        )?;
    }
    Ok(Versao {
        nome: texto(nome, 2, 120, "Nome do pacote")?,
        para_quem: texto(para_quem, 3, 200, "Para quem é")?,
        motivo: motivo.to_owned(),
        vigente_desde,
        preco_centavos,
        exames,
    })
}

fn erro_do_pacote(e: &ErroDoPacote) -> ErroApi {
    match e {
        ErroDoPacote::QuantidadeDeExames => invalido("pacote-invalido", "O pacote tem de 2 a 10 exames."),
        ErroDoPacote::ExameRepetido => invalido("pacote-invalido", "Cada exame aparece uma vez no pacote."),
        ErroDoPacote::ExameSemPreco(t) => invalido(
            "exame-sem-preco",
            &format!("O exame {t} não tem preço na tabela do particular dessa data: sem ele não dá para calcular o avulso."),
        ),
        ErroDoPacote::PrecoInvalido => invalido("preco-invalido", "Preço do pacote de R$ 0,01 a R$ 100.000,00."),
        ErroDoPacote::NaoSaiMaisBarato => invalido(
            "pacote-nao-sai-mais-barato",
            "O preço do pacote precisa ser menor que a soma avulsa dos exames; senão o orçamento não o aplica.",
        ),
        ErroDoPacote::DescontoAcimaDoLimite => invalido(
            "desconto-acima-do-limite",
            &format!("O desconto do pacote passa do limite de {}% sobre o avulso.", DESCONTO_MAXIMO_PB / 100),
        )
        .com_extra("limite_pontos_base", json!(DESCONTO_MAXIMO_PB)),
    }
}

/// Confere exames no catálogo e o preço contra o avulso do primeiro dia (G1).
async fn conferir_preco(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    v: &Versao<'_>,
) -> Result<PrecoDoPacote, ErroApi> {
    let tuss: Vec<String> = v.exames.iter().map(|e| e.tuss.clone()).collect();
    let ativos: BTreeMap<String, bool> = sqlx::query!(
        "SELECT tuss, ativo FROM agenda.procedimento WHERE tuss = ANY($1)",
        &tuss
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|p| (p.tuss, p.ativo))
    .collect();
    if let Some(fora) = tuss.iter().find(|t| ativos.get(*t) != Some(&true)) {
        return Err(invalido(
            "procedimento-fora-do-catalogo",
            &format!("O exame {fora} não está ativo no catálogo da clínica."),
        ));
    }
    let precos = precos_na_data(tx, unidade, v.vigente_desde).await?;
    let itens: Vec<(String, Option<i64>)> = tuss
        .iter()
        .map(|t| (t.clone(), precos.get(t).copied()))
        .collect();
    conferir(&itens, v.preco_centavos).map_err(|e| erro_do_pacote(&e))
}

#[allow(clippy::too_many_arguments)]
async fn gravar(
    tx: &mut sqlx::PgConnection,
    usuario: &Usuario,
    codigo: &str,
    regra_versao: i32,
    v: &Versao<'_>,
    preco: &PrecoDoPacote,
    correlacao: &str,
) -> Result<(), ErroApi> {
    let id = novo_id();
    let tuss: Vec<String> = v.exames.iter().map(|e| e.tuss.clone()).collect();
    sqlx::query!(
        "INSERT INTO financeiro.pacote (id, codigo, nome, regra_versao, tuss, valor_centavos, vigente_desde, para_quem, motivo,
                criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'pacotes', $11, $12)",
        id, codigo, v.nome, regra_versao, &tuss, v.preco_centavos, v.vigente_desde, v.para_quem, v.motivo,
        usuario.id, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito(
            "pacote-concorrente",
            "Outra versão deste pacote foi publicada ao mesmo tempo ou já começa nesse dia. Nada foi publicado. Recarregue e confira.",
        ),
        _ => ErroApi::from(e),
    })?;
    let ordens: Vec<i32> = (1..=i32::try_from(v.exames.len()).unwrap_or(0)).collect();
    let preparos: Vec<String> = v
        .exames
        .iter()
        .map(|e| e.preparo.trim().to_owned())
        .collect();
    let porques: Vec<String> = v
        .exames
        .iter()
        .map(|e| e.porque.trim().to_owned())
        .collect();
    sqlx::query!(
        "INSERT INTO financeiro.pacote_item (pacote_id, ordem, tuss, preparo, porque, unidade_id)
         SELECT $1, o, t, p, q, $6 FROM unnest($2::int[], $3::text[], $4::text[], $5::text[]) AS u(o, t, p, q)",
        id, &ordens, &tuss, &preparos, &porques, usuario.unidade_id
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(&mut *tx, NovoEvento {
        dominio: "pacotes", tipo: "pacote_publicado", registro_id: id, registro_codigo: Some(codigo),
        dados: json!({ "regra_versao": regra_versao, "vigente_desde": v.vigente_desde, "tuss": tuss, "preco_centavos": v.preco_centavos,
                       "avulso_centavos": preco.avulso_centavos, "desconto_pontos_base": preco.desconto_pb, "motivo": v.motivo }),
        criado_por: Some(usuario.id), origem: "pacotes", unidade_id: usuario.unidade_id, correlacao,
    }).await?;
    Ok(())
}

async fn responder(
    tx: &mut sqlx::PgConnection,
    unidade: Uuid,
    codigo: &str,
) -> Result<serde_json::Value, ErroApi> {
    let p: PacoteDeCheckup = carregar(tx, unidade, Some(codigo))
        .await?
        .pop()
        .ok_or_else(|| ErroApi::interno("pacote sumiu"))?;
    serde_json::to_value(p).map_err(ErroApi::interno)
}

/// Cria o pacote com a primeira versão (gerência; `Idempotency-Key`).
#[utoipa::path(post, operation_id = "criar_pacote_de_checkup", path = "/v1/pacotes", tag = "pacotes",
    params(("Idempotency-Key" = Uuid, Header)), request_body = NovoPacoteDeCheckup,
    responses((status = 201, body = PacoteDeCheckup), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn criar(
    State(estado): State<Estado>,
    usuario: Usuario,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovoPacoteDeCheckup>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::PacotesAdministrar)?;
    let v = validar(
        &n.nome,
        &n.para_quem,
        &n.motivo,
        n.vigente_desde,
        n.preco_centavos,
        &n.exames,
    )?;
    let hoje = tempo::hoje_local();
    if v.vigente_desde < hoje || v.vigente_desde > hoje + Duration::days(366) {
        return Err(invalido(
            "vigencia-invalida",
            "O pacote novo começa hoje ou numa data em até um ano.",
        ));
    }
    const ROTA: &str = "POST /v1/pacotes";
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
    let preco = conferir_preco(&mut tx, usuario.unidade_id, &v).await?;
    let codigo = proximo(&mut tx, "PCT").await?;
    gravar(
        &mut tx,
        &usuario,
        codigo.como_str(),
        1,
        &v,
        &preco,
        &correlacao,
    )
    .await?;
    let corpo = responder(&mut tx, usuario.unidade_id, codigo.como_str()).await?;
    idempotencia::concluir(&mut tx, usuario.id, ROTA, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}

/// Publica versão nova do pacote (gerência; `Idempotency-Key`). A vigente não muda: a nova começa
/// depois de hoje e depois do início da última versão. Duas versões ao mesmo tempo: só uma entra.
#[utoipa::path(post, operation_id = "nova_versao_do_pacote_de_checkup", path = "/v1/pacotes/{codigo}/versoes", tag = "pacotes",
    params(("codigo" = String, Path), ("Idempotency-Key" = Uuid, Header)), request_body = NovaVersaoDoPacoteDeCheckup,
    responses((status = 201, body = PacoteDeCheckup), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc),
        (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn nova_versao(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(codigo): Path<String>,
    ChaveIdempotencia(chave): ChaveIdempotencia,
    Corpo(n): Corpo<NovaVersaoDoPacoteDeCheckup>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::PacotesAdministrar)?;
    let nao_encontrado = || {
        ErroApi::nao_encontrado(
            "pacote-nao-encontrado",
            "Pacote não encontrado nesta unidade. Nada foi publicado.",
        )
    };
    if !(3..=30).contains(&codigo.len())
        || !codigo
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(nao_encontrado());
    }
    let v = validar(
        &n.nome,
        &n.para_quem,
        &n.motivo,
        n.vigente_desde,
        n.preco_centavos,
        &n.exames,
    )?;
    let hoje = tempo::hoje_local();
    if v.vigente_desde <= hoje || v.vigente_desde > hoje + Duration::days(366) {
        return Err(invalido(
            "vigencia-invalida",
            "A versão nova começa a partir de amanhã e em até um ano: o pacote vigente não muda no meio do dia.",
        ));
    }
    let rota = format!("POST /v1/pacotes/{codigo}/versoes");
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
    let ultima = sqlx::query!(
        "SELECT max(regra_versao) AS versao, max(vigente_desde) AS desde FROM financeiro.pacote WHERE unidade_id = $1 AND codigo = $2",
        usuario.unidade_id,
        codigo
    )
    .fetch_one(&mut *tx)
    .await?;
    let atual = ultima.versao.ok_or_else(nao_encontrado)?;
    if atual != n.regra_versao_vista {
        return Err(ErroApi::conflito(
            "versao-mudou",
            "Outra pessoa publicou versão deste pacote enquanto você editava. Nada foi publicado. Recarregue e confira antes de repetir.",
        ));
    }
    if let Some(d) = ultima.desde.filter(|d| v.vigente_desde <= *d) {
        return Err(invalido(
            "vigencia-invalida",
            &format!(
                "A versão nova precisa começar depois de {} (início da última versão).",
                d.format("%d/%m/%Y")
            ),
        ));
    }
    let preco = conferir_preco(&mut tx, usuario.unidade_id, &v).await?;
    gravar(
        &mut tx,
        &usuario,
        &codigo,
        atual + 1,
        &v,
        &preco,
        &correlacao,
    )
    .await?;
    let corpo = responder(&mut tx, usuario.unidade_id, &codigo).await?;
    idempotencia::concluir(&mut tx, usuario.id, &rota, chave, 201, &corpo).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(corpo)).into_response())
}
