//! Explorador de lastro (Lastro.dc.html, docs/21: "qualquer código leva à cadeia completa"). A partir
//! de um código (atendimento, accession, agendamento, etiqueta, guia, triagem, laudo, estudo ou
//! StudyUID, recebimento, estorno), o servidor acha o atendimento e devolve os registros ligados e os
//! eventos imutáveis deles, conferindo o hash de cada evento. Os dados do evento não saem (só o
//! tipo): o explorador mostra o caminho, não o conteúdo. A leitura fica no lastro.

use crate::atos::Ato;
use crate::banco::{self, ContextoBanco};
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::lastro::{self, NovoEvento};
use crate::rotas::agenda::PacienteResumo;
use crate::sessao::Usuario;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Deserialize, IntoParams)]
pub struct BuscaDeLastro {
    /// Qualquer código do atendimento (ATD, accession, AGD, ETQ, guia, TRI, LAU, EST, StudyUID, REC, EST de estorno).
    pub codigo: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct NoDaCadeia {
    /// agendamento, guia, atendimento, etiqueta, triagem, estudo, laudo, recebimento, estorno, contraste.
    pub tipo: String,
    pub id: Uuid,
    pub codigo: String,
    pub situacao: Option<String>,
    pub criado_em: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct EventoDaCadeia {
    pub sequencia: i64,
    pub dominio: String,
    pub tipo: String,
    pub registro_codigo: Option<String>,
    pub criado_em: DateTime<Utc>,
    pub origem: String,
    /// Primeiros 12 caracteres do hash (hex).
    pub hash: String,
    /// O hash confere com o conteúdo e com o evento anterior.
    pub integro: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CadeiaDoAtendimento {
    pub paciente: PacienteResumo,
    pub atendimento_codigo: String,
    pub nos: Vec<NoDaCadeia>,
    pub eventos: Vec<EventoDaCadeia>,
    /// Todos os eventos da cadeia conferem.
    pub integra: bool,
}

/// Código digitado na busca: completo e de tamanho razoável.
pub(crate) fn validar_codigo(codigo: &str) -> Result<&str, ErroApi> {
    let codigo = codigo.trim();
    if codigo.len() < 3 || codigo.len() > 80 {
        return Err(ErroApi::invalido(
            "codigo-invalido",
            "Digite um código completo (mínimo 3 caracteres). Nada foi buscado.",
        ));
    }
    Ok(codigo)
}

/// Contexto em que a cadeia é montada: a cadeia cruza domínios que o perfil não lê um a um
/// (estudo, laudo, caixa), então o servidor monta em contexto de sistema da MESMA unidade e devolve
/// só código, situação e tipo de evento.
pub(crate) fn contexto_da_cadeia(usuario: &Usuario) -> ContextoBanco {
    ContextoBanco {
        usuario_id: Some(usuario.id),
        unidade_id: usuario.unidade_id,
        perfis: vec!["sistema".into()],
    }
}

/// A cadeia montada, com o hash completo de cada evento (o JSON da tela leva só o começo).
pub(crate) struct CadeiaMontada {
    pub atendimento_id: Uuid,
    pub cadeia: CadeiaDoAtendimento,
    pub hashes: Vec<String>,
}

/// Monta a cadeia do atendimento achado pelo código, só da unidade do contexto.
pub(crate) async fn montar(
    conexao: &mut PgConnection,
    unidade_id: Uuid,
    codigo: &str,
) -> Result<CadeiaMontada, ErroApi> {
    let atendimento = sqlx::query_scalar!(
        r#"SELECT a.id AS "id!" FROM recepcao.atendimento a WHERE a.unidade_id = $2 AND (a.codigo = $1 OR a.accession = $1)
           UNION SELECT a.id FROM recepcao.atendimento a JOIN agenda.agendamento g ON g.id = a.agendamento_id WHERE a.unidade_id = $2 AND g.codigo = $1
           UNION SELECT a.id FROM recepcao.atendimento a JOIN convenio.guia gu ON gu.agendamento_id = a.agendamento_id WHERE a.unidade_id = $2 AND gu.numero = $1
           UNION SELECT e.atendimento_id FROM recepcao.etiqueta e WHERE e.unidade_id = $2 AND e.codigo = $1
           UNION SELECT t.atendimento_id FROM triagem.triagem t WHERE t.unidade_id = $2 AND t.codigo = $1
           UNION SELECT s.atendimento_id FROM exames.estudo s WHERE s.unidade_id = $2 AND s.atendimento_id IS NOT NULL AND (s.codigo = $1 OR s.study_uid = $1)
           UNION SELECT l.atendimento_id FROM laudos.laudo l WHERE l.unidade_id = $2 AND l.codigo = $1
           UNION SELECT r.atendimento_id FROM caixa.recebimento r WHERE r.unidade_id = $2 AND r.atendimento_id IS NOT NULL AND r.codigo = $1
           UNION SELECT r.atendimento_id FROM caixa.estorno x JOIN caixa.recebimento r ON r.id = x.recebimento_id WHERE x.unidade_id = $2 AND r.atendimento_id IS NOT NULL AND x.codigo = $1
           LIMIT 2"#,
        codigo,
        unidade_id
    )
    .fetch_all(&mut *conexao)
    .await?;
    // Código desconhecido ou de outra unidade: a mesma resposta (sem oráculo de enumeração).
    let [atendimento] = atendimento.as_slice() else {
        return Err(ErroApi::nao_encontrado(
            "codigo-sem-cadeia",
            "Nenhum atendimento desta unidade tem este código.",
        ));
    };
    let a = sqlx::query!(
        "SELECT a.codigo, a.situacao, a.criado_em, a.agendamento_id, p.id AS paciente_id, p.nome, p.nascimento,
                g.codigo AS ag_codigo, g.situacao AS ag_situacao, g.criado_em AS ag_criado
           FROM recepcao.atendimento a JOIN identidade.paciente p ON p.id = a.paciente_id JOIN agenda.agendamento g ON g.id = a.agendamento_id
          WHERE a.id = $1",
        atendimento
    )
    .fetch_one(&mut *conexao)
    .await?;
    let mut nos = vec![
        NoDaCadeia {
            tipo: "agendamento".into(),
            id: a.agendamento_id,
            codigo: a.ag_codigo,
            situacao: Some(a.ag_situacao),
            criado_em: a.ag_criado,
        },
        NoDaCadeia {
            tipo: "atendimento".into(),
            id: *atendimento,
            codigo: a.codigo.clone(),
            situacao: Some(a.situacao),
            criado_em: a.criado_em,
        },
    ];
    let ligados = sqlx::query!(
        r#"SELECT 'guia' AS "tipo!", id AS "id!", numero AS "codigo!", situacao, criado_em AS "criado_em!" FROM convenio.guia WHERE agendamento_id = $2
           UNION ALL SELECT 'etiqueta', id, codigo, situacao, criado_em FROM recepcao.etiqueta WHERE atendimento_id = $1
           UNION ALL SELECT 'triagem', id, codigo, resultado, criado_em FROM triagem.triagem WHERE atendimento_id = $1
           UNION ALL SELECT 'estudo', id, codigo, NULL, criado_em FROM exames.estudo WHERE atendimento_id = $1
           UNION ALL SELECT 'laudo', id, codigo, situacao, criado_em FROM laudos.laudo WHERE atendimento_id = $1
           UNION ALL SELECT 'recebimento', id, codigo, situacao, criado_em FROM caixa.recebimento WHERE atendimento_id = $1
           UNION ALL SELECT 'estorno', x.id, x.codigo, x.situacao, x.criado_em FROM caixa.estorno x JOIN caixa.recebimento r ON r.id = x.recebimento_id WHERE r.atendimento_id = $1
           UNION ALL SELECT 'contraste', m.id, l.numero, NULL, m.criado_em FROM estoque.movimento m JOIN estoque.lote l ON l.id = m.lote_id WHERE m.atendimento_id = $1
           ORDER BY 5"#,
        atendimento,
        a.agendamento_id
    )
    .fetch_all(&mut *conexao)
    .await?;
    nos.extend(ligados.into_iter().map(|l| NoDaCadeia {
        tipo: l.tipo,
        id: l.id,
        codigo: l.codigo,
        situacao: l.situacao,
        criado_em: l.criado_em,
    }));
    let ids: Vec<Uuid> = nos.iter().map(|n| n.id).collect();
    // Conferência do hash: o mesmo cálculo do gatilho lastro.encadear (migration 0001).
    let eventos = sqlx::query!(
        r#"SELECT e.sequencia, e.dominio, e.tipo, e.registro_codigo, e.criado_em, e.origem, encode(e.hash, 'hex') AS "hash!",
                  e.hash = digest(coalesce(encode(e.hash_anterior, 'hex'), '') || '|' || e.id::text || '|' || e.dominio || '|' ||
                     e.tipo || '|' || e.registro_id::text || '|' || e.dados::text || '|' ||
                     to_char(e.criado_em AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US'), 'sha256')
                  AND (e.hash_anterior IS NOT DISTINCT FROM (SELECT p.hash FROM lastro.evento p WHERE p.sequencia < e.sequencia ORDER BY p.sequencia DESC LIMIT 1))
                  AS "integro!"
           FROM lastro.evento e WHERE e.registro_id = ANY($1) ORDER BY e.sequencia LIMIT 500"#,
        &ids
    )
    .fetch_all(&mut *conexao)
    .await?;
    let hashes: Vec<String> = eventos.iter().map(|e| e.hash.clone()).collect();
    let eventos: Vec<EventoDaCadeia> = eventos
        .into_iter()
        .map(|e| EventoDaCadeia {
            sequencia: e.sequencia,
            dominio: e.dominio,
            tipo: e.tipo,
            registro_codigo: e.registro_codigo,
            criado_em: e.criado_em,
            origem: e.origem,
            hash: e.hash.chars().take(12).collect(),
            integro: e.integro,
        })
        .collect();
    let integra = eventos.iter().all(|e| e.integro);
    Ok(CadeiaMontada {
        atendimento_id: *atendimento,
        cadeia: CadeiaDoAtendimento {
            paciente: PacienteResumo {
                id: a.paciente_id,
                nome: a.nome,
                nascimento: a.nascimento,
            },
            atendimento_codigo: a.codigo,
            nos,
            eventos,
            integra,
        },
        hashes,
    })
}

/// Cadeia completa a partir de qualquer código (gerência e auditor).
#[utoipa::path(get, path = "/v1/lastro/cadeia", tag = "lastro", params(BuscaDeLastro),
    responses((status = 200, body = CadeiaDoAtendimento), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn cadeia(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(b): Query<BuscaDeLastro>,
) -> Result<Json<CadeiaDoAtendimento>, ErroApi> {
    usuario.exigir(Ato::LastroExplorar)?;
    let codigo = validar_codigo(&b.codigo)?;
    let correlacao = crate::trace::atual().unwrap_or_default();
    let mut tx = banco::transacao(&estado.banco, &contexto_da_cadeia(&usuario)).await?;
    let montada = montar(&mut tx, usuario.unidade_id, codigo).await?;
    // Quem abriu a cadeia, de quem e por qual código (docs/21: leituras ficam registradas).
    lastro::registrar(
        &mut tx,
        NovoEvento {
            dominio: "lastro",
            tipo: "cadeia_lida",
            registro_id: montada.atendimento_id,
            registro_codigo: Some(&montada.cadeia.atendimento_codigo),
            dados: json!({ "por": usuario.id, "perfis": usuario.perfis }),
            criado_por: Some(usuario.id),
            origem: "lastro",
            unidade_id: usuario.unidade_id,
            correlacao: &correlacao,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Json(montada.cadeia))
}
