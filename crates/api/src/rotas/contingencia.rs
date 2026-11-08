//! Modo contingência (docs/23, docs/27 FC-CTG-01 a 11, docs/30 § 6): declaração idempotente, kit do
//! dia com faixas numeradas que não se sobrepõem, lançamento posterior com marca de contingência,
//! divergências abertas em vez de corrigir sozinho, e encerramento só sem pendência.

use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::eventos::EventoTela;
use crate::extratores::{Corpo, Estacao};
use crate::lastro::{self, NovoEvento};
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::America::Sao_Paulo;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgConnection;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel, NomeCompleto};

pub const MODELO_KIT: &str = "KIT-2026.1";
const TAMANHO_FAIXA: i32 = 100;

#[derive(Serialize, ToSchema)]
pub struct Contingencia {
    pub id: Uuid,
    pub codigo: String,
    pub nivel: String,
    pub motivo: String,
    pub declarada_por: Uuid,
    pub responsavel_id: Uuid,
    pub declarada_em: DateTime<Utc>,
    pub situacao: String,
    pub encerrada_em: Option<DateTime<Utc>>,
}

async fn proximo(conexao: &mut PgConnection, prefixo: &str) -> Result<CodigoLegivel, ErroApi> {
    let ano = tempo::ano_local();
    let n = sqlx::query_scalar!("SELECT comum.proximo_numero($1, $2)", prefixo, ano)
        .fetch_one(conexao)
        .await?
        .unwrap_or(0);
    Ok(CodigoLegivel::novo(prefixo, ano, n))
}

async fn carregar(conexao: &mut PgConnection, id: Uuid) -> Result<Option<Contingencia>, ErroApi> {
    Ok(sqlx::query_as!(
        Contingencia,
        "SELECT id, codigo, nivel, motivo, declarada_por, responsavel_id, declarada_em, situacao, encerrada_em FROM contingencia.contingencia WHERE id = $1",
        id
    )
    .fetch_optional(conexao)
    .await?)
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoDeclaracao {
    /// externo, setor ou total.
    pub nivel: String,
    pub motivo: String,
    pub responsavel_id: Uuid,
    /// Hora em que a contingência foi declarada (pode ter sido sem servidor).
    pub declarada_em: DateTime<Utc>,
    /// UUID gerado na estação; repetir não cria outra (FC-CTG-02).
    pub chave: Uuid,
}

/// Declara a contingência. A mesma chave sobe uma vez só, com a hora original.
#[utoipa::path(post, path = "/v1/contingencias", tag = "contingencia", request_body = PedidoDeclaracao,
    responses((status = 201, body = Contingencia), (status = 200, body = Contingencia), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn declarar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    Corpo(p): Corpo<PedidoDeclaracao>,
) -> Result<(StatusCode, Json<Contingencia>), ErroApi> {
    usuario.exigir(Ato::ContingenciaDeclarar)?;
    if !["externo", "setor", "total"].contains(&p.nivel.as_str())
        || p.motivo.trim().chars().count() < 5
    {
        return Err(ErroApi::invalido(
            "declaracao-invalida",
            "Informe o nível (externo, setor ou total) e o motivo. Nada foi declarado.",
        ));
    }
    let agora = Utc::now();
    if p.declarada_em > agora + Duration::minutes(5) || p.declarada_em < agora - Duration::days(3) {
        return Err(ErroApi::invalido("hora-invalida", "A hora da declaração está no futuro ou há mais de 3 dias. Nada foi declarado. Confira o relógio da estação."));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if let Some(id) = sqlx::query_scalar!(
        "SELECT id FROM contingencia.contingencia WHERE chave = $1",
        p.chave
    )
    .fetch_optional(&mut *tx)
    .await?
    {
        let c = carregar(&mut tx, id)
            .await?
            .ok_or_else(|| ErroApi::interno("contingência sumiu"))?;
        tx.commit().await?;
        return Ok((StatusCode::OK, Json(c)));
    }
    let codigo = proximo(&mut tx, "CTG").await?;
    let id = novo_id();
    let inserido = sqlx::query!(
        "INSERT INTO contingencia.contingencia (id, codigo, nivel, motivo, declarada_por, responsavel_id, declarada_em, chave, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $5, 'balcao', $9, $10, $11)
         ON CONFLICT (chave) DO NOTHING",
        id, codigo.como_str(), p.nivel, p.motivo.trim(), usuario.id, p.responsavel_id, p.declarada_em, p.chave, estacao, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await;
    match inserido {
        Err(sqlx::Error::Database(d)) if d.constraint() == Some("contingencia_uma_aberta") => {
            return Err(ErroApi::conflito("contingencia-ja-aberta", "Já há uma contingência aberta nesta unidade. Nada foi declarado de novo. Use a contingência aberta."));
        }
        Err(e) => return Err(e.into()),
        Ok(r) if r.rows_affected() == 0 => {
            // Corrida com a mesma chave: devolve a que ficou.
            tx.rollback().await?;
            let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
            let existente = sqlx::query_scalar!(
                "SELECT id FROM contingencia.contingencia WHERE chave = $1",
                p.chave
            )
            .fetch_one(&mut *tx)
            .await?;
            let c = carregar(&mut tx, existente)
                .await?
                .ok_or_else(|| ErroApi::interno("contingência sumiu"))?;
            tx.commit().await?;
            return Ok((StatusCode::OK, Json(c)));
        }
        Ok(_) => {}
    }
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "contingencia", tipo: "contingencia_declarada", registro_id: id, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "nivel": p.nivel, "motivo": p.motivo.trim(), "responsavel": p.responsavel_id, "declarada_em": p.declarada_em, "estacao": estacao }),
        criado_por: Some(usuario.id), origem: "balcao", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    let c = carregar(&mut tx, id)
        .await?
        .ok_or_else(|| ErroApi::interno("contingência sumiu"))?;
    tx.commit().await?;
    // Todas as estações e a coordenação são avisadas (FC-CTG-01).
    let _ = estado.eventos.send(EventoTela {
        unidade_id: usuario.unidade_id,
        tipo: "contingencia-declarada",
        perfis: &[
            "recepcao",
            "caixa",
            "tecnico",
            "medico",
            "gerente",
            "faturista",
            "financeiro",
        ],
        dados: json!({ "codigo": c.codigo, "nivel": c.nivel }),
    });
    Ok((StatusCode::CREATED, Json(c)))
}

/// Contingência aberta na unidade, se houver.
#[utoipa::path(get, path = "/v1/contingencias/aberta", tag = "contingencia", responses((status = 200, body = Option<Contingencia>)))]
pub async fn aberta(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<Json<Option<Contingencia>>, ErroApi> {
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let id =
        sqlx::query_scalar!("SELECT id FROM contingencia.contingencia WHERE situacao = 'aberta'")
            .fetch_optional(&mut *tx)
            .await?;
    let c = match id {
        Some(id) => carregar(&mut tx, id).await?,
        None => None,
    };
    tx.commit().await?;
    Ok(Json(c))
}

#[derive(Serialize, ToSchema)]
pub struct Faixa {
    pub formulario: String,
    pub inicio: i32,
    pub fim: i32,
}

#[derive(Serialize, ToSchema)]
pub struct Kit {
    pub codigo: String,
    pub dia: NaiveDate,
    pub modelo_versao: String,
    pub hash: String,
    pub faixas: Vec<Faixa>,
}

/// Gera o kit do dia (agenda por sala para papel) e reserva as faixas, uma vez por dia (FC-CTG-11).
///
/// # Errors
/// Falha de banco.
pub async fn gerar_kit(
    estado: &Estado,
    unidade_id: Uuid,
    dia: NaiveDate,
    usuario: Option<Uuid>,
) -> Result<Kit, ErroApi> {
    let mut tx = banco::transacao(&estado.banco, &ContextoBanco::sistema(unidade_id)).await?;
    // Uma geração por vez por unidade: a reserva das faixas é decidida no banco.
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtext('kit-contingencia'), hashtext($1))",
        unidade_id.to_string()
    )
    .execute(&mut *tx)
    .await?;
    let existente = sqlx::query!("SELECT id, codigo, modelo_versao, hash FROM contingencia.kit WHERE unidade_id = $1 AND dia = $2", unidade_id, dia)
        .fetch_optional(&mut *tx)
        .await?;
    let (kit_id, codigo, hash) = if let Some(k) = existente {
        (k.id, k.codigo, k.hash)
    } else {
        let (inicio, fim) =
            tempo::limites_do_dia(dia).ok_or_else(|| ErroApi::interno("dia inválido"))?;
        let agenda = sqlx::query!(
            r#"SELECT a.codigo, a.inicio, s.nome AS sala, p.nome, p.nascimento, a.exame_nome
                 FROM agenda.agendamento a JOIN agenda.sala s ON s.id = a.sala_id JOIN identidade.paciente p ON p.id = a.paciente_id
                WHERE a.unidade_id = $1 AND a.inicio >= $2 AND a.inicio < $3 AND a.situacao NOT IN ('cancelado','expirada')
                ORDER BY s.codigo, a.inicio"#,
            unidade_id, inicio, fim
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut conteudo = format!("VidaCor - kit de contingencia {dia} - modelo {MODELO_KIT}\n");
        for l in &agenda {
            conteudo.push_str(&format!(
                "{} | {} | {} | {} | {} | {}\n",
                l.inicio.with_timezone(&Sao_Paulo).format("%H:%M"),
                l.sala,
                l.nome,
                l.nascimento.format("%d/%m/%Y"),
                l.exame_nome,
                l.codigo
            ));
        }
        let hash: String = Sha256::digest(conteudo.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let codigo = proximo(&mut tx, "KIT").await?;
        let id = novo_id();
        sqlx::query!(
            "INSERT INTO contingencia.kit (id, codigo, dia, modelo_versao, conteudo, hash, criado_por, origem, unidade_id, correlacao)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'nucleo', $8, 'kit')",
            id, codigo.como_str(), dia, MODELO_KIT, conteudo, hash, usuario, unidade_id
        )
        .execute(&mut *tx)
        .await?;
        for formulario in ["F-ATD", "F-REC"] {
            let ultimo = sqlx::query_scalar!("SELECT COALESCE(MAX(fim), 0) AS \"m!\" FROM contingencia.faixa WHERE unidade_id = $1 AND formulario = $2", unidade_id, formulario)
                .fetch_one(&mut *tx)
                .await?;
            sqlx::query!(
                "INSERT INTO contingencia.faixa (id, kit_id, formulario, dia, inicio, fim, criado_por, origem, unidade_id, correlacao)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 'nucleo', $8, 'kit')",
                novo_id(), id, formulario, dia, ultimo + 1, ultimo + TAMANHO_FAIXA, usuario, unidade_id
            )
            .execute(&mut *tx)
            .await?;
        }
        lastro::registrar(&mut tx, NovoEvento {
            dominio: "contingencia", tipo: "kit_gerado", registro_id: id, registro_codigo: Some(codigo.como_str()),
            dados: json!({ "dia": dia, "modelo_versao": MODELO_KIT, "hash": hash, "agendamentos": agenda.len() }),
            criado_por: usuario, origem: "nucleo", unidade_id, correlacao: "kit",
        }).await?;
        (id, codigo.to_string(), hash)
    };
    let faixas = sqlx::query_as!(Faixa, "SELECT formulario, inicio, fim FROM contingencia.faixa WHERE kit_id = $1 ORDER BY formulario", kit_id)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Kit {
        codigo,
        dia,
        modelo_versao: MODELO_KIT.into(),
        hash,
        faixas,
    })
}

/// Gera (ou devolve) o kit de contingência de hoje.
#[utoipa::path(post, path = "/v1/contingencias/kits", tag = "contingencia", responses((status = 201, body = Kit), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn kit(
    State(estado): State<Estado>,
    usuario: Usuario,
) -> Result<(StatusCode, Json<Kit>), ErroApi> {
    usuario.exigir(Ato::ContingenciaKit)?;
    let k = gerar_kit(
        &estado,
        usuario.unidade_id,
        tempo::hoje_local(),
        Some(usuario.id),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(k)))
}

async fn contingencia_aberta(
    conexao: &mut PgConnection,
    id: Uuid,
) -> Result<(Contingencia, NaiveDate), ErroApi> {
    let c = carregar(conexao, id).await?.ok_or_else(|| {
        ErroApi::nao_encontrado(
            "contingencia-nao-encontrada",
            "Contingência não encontrada nesta unidade.",
        )
    })?;
    if c.situacao != "aberta" {
        return Err(ErroApi::conflito(
            "contingencia-encerrada",
            "Esta contingência já foi encerrada. Nada foi lançado.",
        ));
    }
    let dia = c.declarada_em.with_timezone(&Sao_Paulo).date_naive();
    Ok((c, dia))
}

async fn faixa_do_numero(
    conexao: &mut PgConnection,
    formulario: &str,
    numero: i32,
    dia: NaiveDate,
) -> Result<Uuid, ErroApi> {
    sqlx::query_scalar!(
        "SELECT id FROM contingencia.faixa WHERE formulario = $1 AND dia = $2 AND $3 BETWEEN inicio AND fim",
        formulario, dia, numero
    )
    .fetch_optional(conexao)
    .await?
    .ok_or_else(|| ErroApi::invalido("numero-fora-da-faixa", &format!("A ficha {formulario} {numero:04} não pertence à faixa reservada para o dia. Nada foi lançado. Confira o número impresso.")))
}

async fn abrir_divergencia(
    conexao: &mut PgConnection,
    usuario: &Usuario,
    contingencia: Uuid,
    tipo: &str,
    dados: Value,
    correlacao: &str,
) -> Result<String, ErroApi> {
    let codigo = proximo(conexao, "DIV").await?;
    sqlx::query!(
        "INSERT INTO recepcao.divergencia (id, codigo, tipo, dados, contingencia_id, criado_por, origem, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'contingencia', $7, $8)",
        novo_id(), codigo.como_str(), tipo, dados, contingencia, usuario.id, usuario.unidade_id, correlacao
    )
    .execute(conexao)
    .await?;
    Ok(codigo.to_string())
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoFicha {
    /// F-ATD.
    pub formulario: String,
    pub numero: i32,
    pub agendamento_id: Uuid,
    pub nome: String,
    pub nascimento: NaiveDate,
    pub ocorrido_em: DateTime<Utc>,
    pub atendido_por: Uuid,
    /// SHA-256 do PDF digitalizado da ficha.
    pub anexo_sha256: String,
}

/// Lança a ficha de atendimento feita em papel (FC-CTG-03 a 06).
#[utoipa::path(post, path = "/v1/contingencias/{id}/fichas", tag = "contingencia", params(("id" = Uuid, Path)), request_body = PedidoFicha,
    responses((status = 201, body = crate::rotas::respostas::FichaLancada), (status = 202, description = "divergencia-aberta", body = crate::rotas::respostas::DivergenciaAberta), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn lancar_ficha(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoFicha>,
) -> Result<(StatusCode, Json<Value>), ErroApi> {
    usuario.exigir(Ato::ContingenciaLancar)?;
    if p.formulario != "F-ATD"
        || !(p.anexo_sha256.len() == 64
            && p.anexo_sha256
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()))
    {
        return Err(ErroApi::invalido(
            "ficha-invalida",
            "Informe a ficha F-ATD com o PDF digitalizado (hash SHA-256). Nada foi lançado.",
        ));
    }
    let nome = NomeCompleto::novo(&p.nome).map_err(|_| {
        ErroApi::invalido(
            "nome-incompleto",
            "Informe o nome completo como está no papel. Nada foi lançado.",
        )
    })?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let (c, dia) = contingencia_aberta(&mut tx, id).await?;
    let faixa = faixa_do_numero(&mut tx, &p.formulario, p.numero, dia).await?;
    let ficha = format!("{} {:04}", p.formulario, p.numero);
    // O número é a trava: duas estações com a mesma ficha, só uma lança (FC-CTG-04).
    let reservado = sqlx::query!(
        "INSERT INTO contingencia.numero (unidade_id, formulario, numero, faixa_id, contingencia_id, situacao, criado_por, estacao, correlacao)
         VALUES ($1, $2, $3, $4, $5, 'usado', $6, $7, $8) ON CONFLICT DO NOTHING",
        usuario.unidade_id, p.formulario, p.numero, faixa, c.id, usuario.id, estacao, correlacao
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if reservado == 0 {
        let atd = sqlx::query_scalar!(
            "SELECT t.codigo FROM contingencia.numero n LEFT JOIN recepcao.atendimento t ON t.id = n.registro_id WHERE n.unidade_id = $1 AND n.formulario = $2 AND n.numero = $3",
            usuario.unidade_id, p.formulario, p.numero
        )
        .fetch_one(&mut *tx)
        .await?;
        return Err(ErroApi::conflito(
            "ficha-ja-lancada",
            &format!("A ficha {ficha} já foi lançada. Nada foi lançado de novo."),
        )
        .com_extra("atendimento_codigo", json!(atd)));
    }
    let ag = sqlx::query!(
        r#"SELECT a.id, a.codigo, a.situacao, a.sala_id, a.unidade_id, p.id AS paciente_id, p.nome, p.nascimento
             FROM agenda.agendamento a JOIN identidade.paciente p ON p.id = a.paciente_id WHERE a.id = $1 FOR UPDATE OF a"#,
        p.agendamento_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("agendamento-nao-encontrado", "Agendamento da ficha não encontrado. Nada foi lançado."))?;
    let confere_nome = NomeCompleto::novo(&ag.nome)
        .map(|n| n.confere_com(&nome))
        .unwrap_or(false);
    if !confere_nome || ag.nascimento != p.nascimento {
        // Nunca corrige nem une cadastro sozinho: abre divergência de identificação (FC-CTG-06).
        let div = abrir_divergencia(&mut tx, &usuario, c.id, "identificacao",
            json!({ "ficha": ficha, "agendamento": ag.codigo, "nascimento_papel": p.nascimento, "nome_confere": confere_nome }), &correlacao).await?;
        sqlx::query!("UPDATE contingencia.numero SET situacao = 'pendente' WHERE unidade_id = $1 AND formulario = $2 AND numero = $3", usuario.unidade_id, p.formulario, p.numero)
            .execute(&mut *tx)
            .await?;
        lastro::registrar(
            &mut tx,
            NovoEvento {
                dominio: "contingencia",
                tipo: "divergencia_identificacao",
                registro_id: c.id,
                registro_codigo: Some(&c.codigo),
                dados: json!({ "ficha": ficha, "divergencia": div }),
                criado_por: Some(usuario.id),
                origem: "contingencia",
                unidade_id: usuario.unidade_id,
                correlacao: &correlacao,
            },
        )
        .await?;
        tx.commit().await?;
        return Ok((
            StatusCode::ACCEPTED,
            Json(crate::rotas::respostas::valor(&crate::rotas::respostas::DivergenciaAberta { codigo: "divergencia-aberta".into(), divergencia: div, detail: "O nome ou o nascimento do papel não conferem com o cadastro. Nenhum cadastro foi alterado e a ficha ficou pendente. A divergência foi aberta para a coordenação resolver.".into() })?),
        ));
    }
    let ano = tempo::ano_local();
    let n_atd = sqlx::query_scalar!("SELECT comum.proximo_numero('ATD', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);
    let n_acc = sqlx::query_scalar!("SELECT comum.proximo_numero('ACC', $1)", ano)
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);
    let atd = novo_id();
    let codigo = CodigoLegivel::novo("ATD", ano, n_atd);
    let accession = CodigoLegivel::novo("ACC", ano, n_acc);
    sqlx::query!(
        "INSERT INTO recepcao.atendimento (id, codigo, accession, agendamento_id, paciente_id, sala_id, situacao, documento_conferido,
                contingencia_id, formulario, ocorrido_em, atendido_por, anexo_sha256, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, 'exame_concluido', false, $7, $8, $9, $10, $11, $12, 'contingencia', $13, $14, $15)",
        atd, codigo.como_str(), accession.como_str(), ag.id, ag.paciente_id, ag.sala_id,
        c.id, ficha, p.ocorrido_em, p.atendido_por, p.anexo_sha256, usuario.id, estacao, ag.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => ErroApi::conflito("checkin-ja-realizado", "Este agendamento já tem atendimento. Nada foi lançado."),
        _ => ErroApi::from(e),
    })?;
    sqlx::query!("UPDATE agenda.agendamento SET situacao = 'em_atendimento', versao = versao + 1 WHERE id = $1 AND situacao IN ('agendado','confirmado')", ag.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("UPDATE contingencia.numero SET registro_id = $4 WHERE unidade_id = $1 AND formulario = $2 AND numero = $3", usuario.unidade_id, p.formulario, p.numero, atd)
        .execute(&mut *tx)
        .await?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "contingencia", tipo: "atendimento_lancado", registro_id: atd, registro_codigo: Some(codigo.como_str()),
        dados: json!({ "registrado_em_contingencia": true, "contingencia": c.codigo, "ficha": ficha, "ocorrido_em": p.ocorrido_em,
                       "atendido_por": p.atendido_por, "lancado_por": usuario.id, "anexo_sha256": p.anexo_sha256 }),
        criado_por: Some(usuario.id), origem: "contingencia", unidade_id: ag.unidade_id, correlacao: &correlacao,
    }).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(crate::rotas::respostas::valor(
            &crate::rotas::respostas::FichaLancada {
                id: atd,
                codigo: codigo.to_string(),
                origem: "contingencia".into(),
                contingencia: c.codigo,
                formulario: ficha,
                ocorrido_em: p.ocorrido_em,
                registrado_em_contingencia: true,
            },
        )?),
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoRecibo {
    pub numero: i32,
    pub paciente_id: Uuid,
    pub tuss: String,
    pub valor_papel_centavos: i64,
    pub ocorrido_em: DateTime<Utc>,
}

/// Lança o recibo manual: grava o valor recalculado pela tabela da data e abre divergência se diferir (FC-CTG-07).
#[utoipa::path(post, path = "/v1/contingencias/{id}/recibos", tag = "contingencia", params(("id" = Uuid, Path)), request_body = PedidoRecibo,
    responses((status = 201, body = crate::rotas::respostas::ReciboLancado), (status = 409, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn lancar_recibo(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoRecibo>,
) -> Result<(StatusCode, Json<Value>), ErroApi> {
    usuario.exigir(Ato::ContingenciaLancar)?;
    if p.valor_papel_centavos < 0 {
        return Err(ErroApi::invalido(
            "valor-invalido",
            "O valor do papel não pode ser negativo. Nada foi lançado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let (c, dia) = contingencia_aberta(&mut tx, id).await?;
    let faixa = faixa_do_numero(&mut tx, "F-REC", p.numero, dia).await?;
    let data = p.ocorrido_em.with_timezone(&Sao_Paulo).date_naive();
    let preco = sqlx::query!(
        "SELECT t.versao, pr.valor_centavos FROM financeiro.tabela_preco t JOIN financeiro.preco pr ON pr.tabela_id = t.id
          WHERE t.unidade_id = $1 AND t.vigente_desde <= $2 AND pr.tuss = $3 ORDER BY t.versao DESC LIMIT 1",
        usuario.unidade_id, data, p.tuss
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::invalido("procedimento-sem-preco", "Não há preço vigente na data para este procedimento. Nada foi lançado."))?;
    let reservado = sqlx::query!(
        "INSERT INTO contingencia.numero (unidade_id, formulario, numero, faixa_id, contingencia_id, situacao, criado_por, estacao, correlacao)
         VALUES ($1, 'F-REC', $2, $3, $4, 'usado', $5, $6, $7) ON CONFLICT DO NOTHING",
        usuario.unidade_id, p.numero, faixa, c.id, usuario.id, estacao, correlacao
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if reservado == 0 {
        return Err(ErroApi::conflito(
            "ficha-ja-lancada",
            &format!(
                "O recibo F-REC {:04} já foi lançado. Nada foi lançado de novo.",
                p.numero
            ),
        ));
    }
    let recibo = novo_id();
    sqlx::query!(
        "INSERT INTO contingencia.recibo (id, numero, contingencia_id, paciente_id, tuss, valor_papel_centavos, valor_recalculado_centavos, tabela_versao, ocorrido_em, criado_por, origem, estacao, unidade_id, correlacao)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'contingencia', $11, $12, $13)",
        recibo, p.numero, c.id, p.paciente_id, p.tuss, p.valor_papel_centavos, preco.valor_centavos, preco.versao, p.ocorrido_em, usuario.id, estacao, usuario.unidade_id, correlacao
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("UPDATE contingencia.numero SET registro_id = $3 WHERE unidade_id = $1 AND formulario = 'F-REC' AND numero = $2", usuario.unidade_id, p.numero, recibo)
        .execute(&mut *tx)
        .await?;
    let divergencia = if p.valor_papel_centavos == preco.valor_centavos {
        None
    } else {
        Some(abrir_divergencia(&mut tx, &usuario, c.id, "preco",
            json!({ "recibo": format!("F-REC {:04}", p.numero), "valor_papel_centavos": p.valor_papel_centavos, "valor_recalculado_centavos": preco.valor_centavos, "tabela_versao": preco.versao }),
            &correlacao).await?)
    };
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "contingencia", tipo: "recibo_lancado", registro_id: recibo, registro_codigo: None,
        dados: json!({ "registrado_em_contingencia": true, "recibo": p.numero, "valor_recalculado_centavos": preco.valor_centavos, "tabela_versao": preco.versao, "divergencia": divergencia }),
        criado_por: Some(usuario.id), origem: "contingencia", unidade_id: usuario.unidade_id, correlacao: &correlacao,
    }).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(crate::rotas::respostas::valor(
            &crate::rotas::respostas::ReciboLancado {
                valor_recalculado_centavos: preco.valor_centavos,
                tabela_versao: preco.versao,
                divergencia,
            },
        )?),
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoNumeros {
    pub formulario: String,
    pub de: i32,
    pub ate: i32,
    /// devolvido ou anulado.
    pub situacao: String,
}

/// Dá situação aos números não usados da faixa (devolvidos ou anulados), para o encerramento.
#[utoipa::path(post, path = "/v1/contingencias/{id}/numeros", tag = "contingencia", params(("id" = Uuid, Path)), request_body = PedidoNumeros,
    responses((status = 200, body = crate::rotas::respostas::NumerosSituados), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn situar_numeros(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoNumeros>,
) -> Result<Json<Value>, ErroApi> {
    usuario.exigir(Ato::ContingenciaLancar)?;
    if !["devolvido", "anulado"].contains(&p.situacao.as_str())
        || p.de > p.ate
        || p.ate - p.de > 1000
    {
        return Err(ErroApi::invalido(
            "numeros-invalidos",
            "Informe uma faixa válida e a situação (devolvido ou anulado). Nada foi alterado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let (c, dia) = contingencia_aberta(&mut tx, id).await?;
    let faixa = faixa_do_numero(&mut tx, &p.formulario, p.de, dia).await?;
    faixa_do_numero(&mut tx, &p.formulario, p.ate, dia).await?;
    let n = sqlx::query!(
        "INSERT INTO contingencia.numero (unidade_id, formulario, numero, faixa_id, contingencia_id, situacao, criado_por, correlacao)
         SELECT $1, $2, g, $3, $4, $5, $6, $7 FROM generate_series($8::int, $9::int) g
         ON CONFLICT DO NOTHING",
        usuario.unidade_id, p.formulario, faixa, c.id, p.situacao, usuario.id, correlacao, p.de, p.ate
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    Ok(Json(crate::rotas::respostas::valor(
        &crate::rotas::respostas::NumerosSituados {
            numeros_situados: n,
        },
    )?))
}

#[derive(Deserialize, ToSchema)]
pub struct PedidoResolucao {
    pub resolucao: String,
}

/// Resolve uma divergência (coordenação), com o que foi decidido.
#[utoipa::path(post, path = "/v1/divergencias/{id}/resolucao", tag = "contingencia", params(("id" = Uuid, Path)), request_body = PedidoResolucao,
    responses((status = 200, body = crate::rotas::respostas::DivergenciaResolvida), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn resolver_divergencia(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
    Corpo(p): Corpo<PedidoResolucao>,
) -> Result<Json<Value>, ErroApi> {
    usuario.exigir(Ato::ContingenciaEncerrar)?;
    if p.resolucao.trim().chars().count() < 5 {
        return Err(ErroApi::invalido(
            "resolucao-obrigatoria",
            "Descreva como a divergência foi resolvida. Nada foi alterado.",
        ));
    }
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let d = sqlx::query!(
        "UPDATE recepcao.divergencia SET situacao = 'resolvida', dados = dados || jsonb_build_object('resolucao', $2::text, 'resolvida_por', $3::uuid), versao = versao + 1
          WHERE id = $1 AND situacao = 'aberta' RETURNING codigo, dados",
        id, p.resolucao.trim(), usuario.id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::conflito("divergencia-indisponivel", "Divergência não encontrada ou já resolvida. Nada foi alterado."))?;
    // Ficha pendente da divergência resolvida: o número fica anulado (um lançamento correto usa a ficha certa).
    if let Some(ficha) = d.dados["ficha"].as_str() {
        if let Some((formulario, numero)) = ficha.split_once(' ') {
            if let Ok(n) = numero.parse::<i32>() {
                sqlx::query!("UPDATE contingencia.numero SET situacao = 'anulado' WHERE unidade_id = $1 AND formulario = $2 AND numero = $3 AND situacao = 'pendente'",
                    usuario.unidade_id, formulario, n)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "recepcao",
            tipo: "divergencia_resolvida",
            registro_id: id,
            registro_codigo: Some(&d.codigo),
            dados: json!({ "resolucao": p.resolucao.trim() }),
            criado_por: Some(usuario.id),
            origem: "coordenacao",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Json(crate::rotas::respostas::valor(
        &crate::rotas::respostas::DivergenciaResolvida {
            codigo: d.codigo,
            situacao: "resolvida".into(),
        },
    )?))
}

/// Encerra a contingência só sem pendência: ficha pendente, divergência aberta ou número sem situação (FC-CTG-09).
#[utoipa::path(post, path = "/v1/contingencias/{id}/encerramento", tag = "contingencia", params(("id" = Uuid, Path)),
    responses((status = 200, body = Contingencia), (status = 403, body = crate::openapi::ProblemaDoc), (status = 409, body = crate::openapi::ProblemaDoc)))]
pub async fn encerrar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<Contingencia>, ErroApi> {
    usuario.exigir(Ato::ContingenciaEncerrar)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let (c, dia) = contingencia_aberta(&mut tx, id).await?;
    sqlx::query!(
        "SELECT id FROM contingencia.contingencia WHERE id = $1 FOR UPDATE",
        c.id
    )
    .fetch_one(&mut *tx)
    .await?;
    let mut pendencias: Vec<String> = Vec::new();
    for d in sqlx::query!("SELECT codigo, tipo FROM recepcao.divergencia WHERE contingencia_id = $1 AND situacao = 'aberta' ORDER BY codigo", c.id)
        .fetch_all(&mut *tx)
        .await?
    {
        pendencias.push(format!("Divergência {} ({}) aberta", d.codigo, d.tipo.replace('_', " ")));
    }
    for n in sqlx::query!("SELECT formulario, numero FROM contingencia.numero WHERE contingencia_id = $1 AND situacao = 'pendente' ORDER BY formulario, numero", c.id)
        .fetch_all(&mut *tx)
        .await?
    {
        pendencias.push(format!("Ficha {} {:04} pendente", n.formulario, n.numero));
    }
    for f in sqlx::query!(
        r#"SELECT f.formulario, count(*) FILTER (WHERE n.numero IS NULL) AS "sem_situacao!"
             FROM contingencia.faixa f
             CROSS JOIN LATERAL generate_series(f.inicio, f.fim) g
             LEFT JOIN contingencia.numero n ON n.unidade_id = f.unidade_id AND n.formulario = f.formulario AND n.numero = g
            WHERE f.dia = $1 GROUP BY f.formulario ORDER BY f.formulario"#,
        dia
    )
    .fetch_all(&mut *tx)
    .await?
    {
        if f.sem_situacao > 0 {
            pendencias.push(format!("{} números da faixa {} sem situação (usado, anulado ou devolvido)", f.sem_situacao, f.formulario));
        }
    }
    if !pendencias.is_empty() {
        return Err(ErroApi::conflito("pendencias", "A contingência não pode ser encerrada: há pendências. Nada foi alterado. Resolva a lista abaixo e tente de novo.")
            .com_extra("pendencias", json!(pendencias)));
    }
    sqlx::query!("UPDATE contingencia.contingencia SET situacao = 'encerrada', encerrada_em = now(), encerrada_por = $2, versao = versao + 1 WHERE id = $1", c.id, usuario.id)
        .execute(&mut *tx)
        .await?;
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "contingencia",
            tipo: "contingencia_encerrada",
            registro_id: c.id,
            registro_codigo: Some(&c.codigo),
            dados: json!({}),
            criado_por: Some(usuario.id),
            origem: "coordenacao",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    let fechada = carregar(&mut tx, c.id)
        .await?
        .ok_or_else(|| ErroApi::interno("contingência sumiu"))?;
    tx.commit().await?;
    Ok(Json(fechada))
}
