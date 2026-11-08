//! Sala de exame: fila e bipe da pulseira (docs/27 FC-PUL, FC-TRI-03 e 06; docs/20).
//! A worklist só vai ao aparelho depois do bipe da pulseira certa na sala certa.

use crate::atos::Ato;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::extratores::{ChaveIdempotencia, Corpo, Estacao};
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::{PacienteResumo, Sala};
use crate::rotas::atendimentos::{self, Atendimento};
use crate::rotas::triagem::exigir_lotacao;
use crate::sessao::Usuario;
use crate::{banco, tempo};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use vidacor_comum::{novo_id, CodigoLegivel};

#[derive(Deserialize, ToSchema)]
pub struct PedidoBipe {
    pub codigo_etiqueta: String,
}

#[derive(Serialize, ToSchema)]
pub struct Aparelho {
    pub nome: String,
    pub ae_title: String,
}

#[derive(Serialize, ToSchema)]
pub struct ResultadoBipe {
    pub atendimento: Atendimento,
    pub aparelho: Aparelho,
    /// enviada (primeiro bipe) ou ja_enviada (bipe repetido, sem segundo item de worklist).
    pub worklist: String,
}

/// Bipe da pulseira na sala: confere etiqueta, sala, triagem e envia a worklist ao aparelho.
#[utoipa::path(post, path = "/v1/salas/{sala_id}/bipes", tag = "sala", params(("sala_id" = Uuid, Path), ("Idempotency-Key" = Uuid, Header)),
    request_body = PedidoBipe,
    responses((status = 200, body = ResultadoBipe), (status = 403, body = crate::openapi::ProblemaDoc),
              (status = 404, body = crate::openapi::ProblemaDoc),
              (status = 409, body = crate::openapi::ProblemaDoc, description = "etiqueta-substituida, paciente-de-outra-sala, conduta-medica-pendente, sala-sem-aparelho"),
              (status = 422, body = crate::openapi::ProblemaDoc, description = "triagem-ausente")))]
pub async fn bipe(
    State(estado): State<Estado>,
    usuario: Usuario,
    Estacao(estacao): Estacao,
    ChaveIdempotencia(_chave): ChaveIdempotencia,
    Path(sala_id): Path<Uuid>,
    Corpo(p): Corpo<PedidoBipe>,
) -> Result<Response, ErroApi> {
    usuario.exigir(Ato::SalaBipar)?;
    let codigo = p.codigo_etiqueta.trim().to_uppercase();
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    exigir_lotacao(&mut tx, usuario.id, sala_id).await?;

    let etq = sqlx::query!(
        "SELECT id, codigo, situacao, substituida_por, atendimento_id FROM recepcao.etiqueta WHERE codigo = $1",
        codigo
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("etiqueta-nao-encontrada", "Esta pulseira não foi encontrada nesta unidade. O exame não foi iniciado. Confira a leitura ou procure a recepção."))?;
    if etq.situacao != "valida" {
        return Err(ErroApi::conflito(
            "etiqueta-substituida",
            &format!(
                "Esta pulseira foi substituída{}. O exame não foi iniciado. Use a pulseira válida do paciente ou peça a reimpressão na recepção.",
                etq.substituida_por.map(|c| format!(" pela {c}")).unwrap_or_default()
            ),
        ));
    }

    let a = sqlx::query!(
        r#"SELECT t.id, t.codigo, t.sala_id, t.situacao, t.unidade_id, s.nome AS sala_nome, ag.requer_contraste
             FROM recepcao.atendimento t
             JOIN agenda.sala s ON s.id = t.sala_id
             JOIN agenda.agendamento ag ON ag.id = t.agendamento_id
            WHERE t.id = $1 FOR UPDATE OF t"#,
        etq.atendimento_id
    )
    .fetch_one(&mut *tx)
    .await?;

    if a.sala_id != sala_id {
        // Pulseira de outro paciente/sala: bloqueia, não envia worklist e abre divergência (FC-PUL-02).
        let ano = tempo::ano_local();
        let n = sqlx::query_scalar!("SELECT comum.proximo_numero('DIV', $1)", ano)
            .fetch_one(&mut *tx)
            .await?
            .ok_or_else(|| ErroApi::interno("contador DIV sem valor"))?;
        let div_id = novo_id();
        let div_codigo = CodigoLegivel::novo("DIV", ano, n);
        sqlx::query!(
            "INSERT INTO recepcao.divergencia (id, codigo, tipo, atendimento_id, sala_id, dados, criado_por, origem, estacao, unidade_id, correlacao)
             VALUES ($1, $2, 'pulseira_sala_errada', $3, $4, $5, $6, 'sala', $7, $8, $9)",
            div_id,
            div_codigo.como_str(),
            a.id,
            sala_id,
            json!({ "etiqueta": etq.codigo, "sala_bipada": sala_id, "sala_do_atendimento": a.sala_id }),
            usuario.id,
            estacao,
            a.unidade_id,
            correlacao
        )
        .execute(&mut *tx)
        .await?;
        lastro::registrar(&mut tx, NovoEvento {
            dominio: "sala", tipo: "bipe_bloqueado_sala_errada", registro_id: div_id,
            registro_codigo: Some(div_codigo.como_str()),
            dados: json!({ "etiqueta": etq.codigo, "atendimento": a.codigo, "sala_bipada": sala_id, "estacao": estacao }),
            criado_por: Some(usuario.id), origem: "sala", unidade_id: a.unidade_id, correlacao: &correlacao,
        }).await?;
        tx.commit().await?;
        return Ok(ErroApi::conflito(
            "paciente-de-outra-sala",
            &format!("Esta pulseira é de paciente agendado na {}. O exame foi bloqueado nesta sala e nada foi enviado ao aparelho. Encaminhe o paciente à {}.", a.sala_nome, a.sala_nome),
        )
        .com_extra("sala_correta", json!({ "id": a.sala_id, "nome": a.sala_nome }))
        .com_extra("divergencia", json!(div_codigo.como_str()))
        .into_response());
    }

    if a.requer_contraste {
        match crate::rotas::triagem::carregar(&mut tx, a.id).await? {
            None => return Err(ErroApi::invalido(
                "triagem-ausente",
                "Exame com contraste sem triagem registrada. O exame não foi iniciado. Faça a triagem de segurança antes.",
            )),
            Some(t) if !t.exame_liberado => return Err(ErroApi::conflito(
                "conduta-medica-pendente",
                "A triagem pede aprovação médica e a conduta ainda não foi registrada. O exame não foi iniciado e nada foi enviado ao aparelho. Aguarde a conduta do médico.",
            )),
            Some(_) => {}
        }
    }

    let aparelho = sqlx::query!(
        "SELECT id, nome, ae_title FROM agenda.aparelho WHERE sala_id = $1 AND ativo ORDER BY codigo LIMIT 1",
        sala_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::conflito("sala-sem-aparelho", "Esta sala não tem aparelho cadastrado. O exame não foi iniciado. Avise a TI."))?;

    let enviada = sqlx::query_scalar!(
        "INSERT INTO integracao.outbox (id, tipo, chave_idempotencia, carga, unidade_id, correlacao, referencia)
         VALUES ($1, 'worklist.criar', $2, $3, $4, $5, $6)
         ON CONFLICT (tipo, referencia) WHERE referencia IS NOT NULL DO NOTHING
         RETURNING 1",
        novo_id(),
        novo_id(),
        json!({ "atendimento_id": a.id, "aparelho_id": aparelho.id }),
        a.unidade_id,
        correlacao,
        a.id
    )
    .fetch_optional(&mut *tx)
    .await?
    .is_some();

    sqlx::query!(
        "UPDATE recepcao.atendimento SET situacao = 'em_exame', versao = versao + 1
          WHERE id = $1 AND situacao = 'aguardando_sala'",
        a.id
    )
    .execute(&mut *tx)
    .await?;
    lastro::registrar(&mut tx, NovoEvento {
        dominio: "sala", tipo: "pulseira_bipada", registro_id: a.id, registro_codigo: Some(&a.codigo),
        dados: json!({ "etiqueta": etq.codigo, "sala": a.sala_nome, "aparelho": aparelho.nome, "worklist": if enviada { "enviada" } else { "ja_enviada" }, "estacao": estacao }),
        criado_por: Some(usuario.id), origem: "sala", unidade_id: a.unidade_id, correlacao: &correlacao,
    }).await?;
    let atendimento = atendimentos::carregar(&mut tx, a.id)
        .await?
        .ok_or_else(|| ErroApi::interno("atendimento sumiu"))?;
    tx.commit().await?;
    estado.outbox_aviso.notify_one();
    Ok((
        StatusCode::OK,
        Json(ResultadoBipe {
            atendimento,
            aparelho: Aparelho {
                nome: aparelho.nome,
                ae_title: aparelho.ae_title,
            },
            worklist: if enviada {
                "enviada".into()
            } else {
                "ja_enviada".into()
            },
        }),
    )
        .into_response())
}

#[derive(Serialize, ToSchema)]
pub struct ItemFila {
    pub atendimento_id: Uuid,
    pub codigo: String,
    pub paciente: PacienteResumo,
    pub exame: String,
    pub inicio: DateTime<Utc>,
    pub situacao: String,
    pub senha: Option<String>,
    pub requer_contraste: bool,
    pub triagem_resultado: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct FilaDaSala {
    pub sala: Sala,
    pub itens: Vec<ItemFila>,
}

/// Fila do dia da sala (técnico lotado, médico, gerente).
#[utoipa::path(get, operation_id = "fila_da_sala", path = "/v1/salas/{sala_id}/fila", tag = "sala", params(("sala_id" = Uuid, Path)),
    responses((status = 200, body = FilaDaSala), (status = 403, body = crate::openapi::ProblemaDoc)))]
pub async fn fila(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(sala_id): Path<Uuid>,
) -> Result<Json<FilaDaSala>, ErroApi> {
    usuario.exigir(Ato::SalaVer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    if usuario.tem_perfil("tecnico")
        && !usuario.tem_perfil("gerente")
        && !usuario.tem_perfil("medico")
    {
        exigir_lotacao(&mut tx, usuario.id, sala_id).await?;
    }
    let sala = sqlx::query_as!(
        Sala,
        "SELECT id, nome FROM agenda.sala WHERE id = $1 AND unidade_id = $2",
        sala_id,
        usuario.unidade_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado("sala-nao-encontrada", "Esta sala não existe nesta unidade.")
    })?;
    let (inicio, fim) = tempo::limites_do_dia(tempo::hoje_local())
        .ok_or_else(|| ErroApi::interno("dia inválido"))?;
    let linhas = sqlx::query!(
        r#"SELECT t.id, t.codigo, t.situacao, t.senha, a.inicio, a.exame_nome, a.requer_contraste,
                  p.id AS paciente_id, p.nome, p.nascimento, tr.resultado AS "triagem?"
             FROM recepcao.atendimento t
             JOIN agenda.agendamento a ON a.id = t.agendamento_id
             JOIN identidade.paciente p ON p.id = t.paciente_id
             LEFT JOIN triagem.triagem tr ON tr.atendimento_id = t.id
            WHERE t.sala_id = $1 AND a.inicio >= $2 AND a.inicio < $3
            ORDER BY a.inicio LIMIT 200"#,
        sala_id,
        inicio,
        fim
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(FilaDaSala {
        sala,
        itens: linhas
            .into_iter()
            .map(|l| ItemFila {
                atendimento_id: l.id,
                codigo: l.codigo,
                paciente: PacienteResumo {
                    id: l.paciente_id,
                    nome: l.nome,
                    nascimento: l.nascimento,
                },
                exame: l.exame_nome,
                inicio: l.inicio,
                situacao: l.situacao,
                senha: l.senha,
                requer_contraste: l.requer_contraste,
                triagem_resultado: l.triagem,
            })
            .collect(),
    }))
}
