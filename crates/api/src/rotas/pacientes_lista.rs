//! Lista de pacientes (Pacientes-Lista.dc.html): busca por nome, nascimento, CPF, celular ou prontuário,
//! recortes (na clínica hoje, com pendência, cadastro incompleto) com contagem e paginação feita pelo
//! servidor. Como na recepção da Medware e no MediClinic RIS, a lista é o ponto de partida da ficha, do
//! agendamento e da correção de cadastro (DEC-2026-11-09-125). O que aparece passa pelo RLS da sessão;
//! a situação vem do banco, nunca do cliente (G1).

use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::rotas::busca::{classificar, Termo};
use crate::rotas::paciente_visao::mascarar_cpf;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Linhas por página (cabem no painel em 1280×800; o resto rola dentro dele).
pub const POR_PAGINA: i64 = 12;
const RECORTES: [&str; 4] = ["todos", "hoje", "pendencia", "incompleto"];

#[derive(Deserialize, IntoParams)]
pub struct FiltroDaLista {
    /// Nome (3+ letras), nascimento (dd/mm/aaaa), CPF ou celular (só dígitos) ou prontuário (PAC-AAAA-NNNNNN).
    pub q: Option<String>,
    /// todos, hoje (na clínica hoje), pendencia (reserva sem guia a vencer) ou incompleto.
    pub recorte: Option<String>,
    /// Página, a partir de 1.
    pub pagina: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct ExameDaLista {
    pub exame: String,
    pub inicio: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct LinhaDaListaDePacientes {
    pub id: Uuid,
    /// Prontuário (código legível).
    pub codigo: String,
    pub nome: String,
    pub nascimento: NaiveDate,
    /// Operadora do próximo agendamento (ou do último), ou "Particular".
    pub convenio: String,
    pub cpf_mascarado: Option<String>,
    pub celular: Option<String>,
    pub alergias: Vec<String>,
    pub ultimo_exame: Option<ExameDaLista>,
    pub proximo_exame: Option<ExameDaLista>,
    /// na_clinica, pendencia, cadastro_incompleto, possivel_duplicado ou em_dia (a mais urgente).
    pub situacao: String,
    /// O que falta no cadastro (cpf, celular, nome_mae, alergias), para a tela dizer o quê.
    pub falta_no_cadastro: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ContagemDosRecortes {
    pub todos: i64,
    pub hoje: i64,
    pub pendencia: i64,
    pub incompleto: i64,
    pub duplicados: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ListaDePacientes {
    /// Cadastros da unidade (sem filtro).
    pub cadastrados: i64,
    /// Contagem de cada recorte com a busca aplicada.
    pub recortes: ContagemDosRecortes,
    pub pagina: i64,
    pub paginas: i64,
    pub por_pagina: i64,
    /// Total de linhas do recorte com a busca (todas as páginas).
    pub total: i64,
    pub itens: Vec<LinhaDaListaDePacientes>,
}

/// Quem vê a lista: quem abre a ficha do paciente (recepção, gerência, técnico e médico), igual à busca global.
pub(crate) fn ve_pacientes(usuario: &Usuario) -> Result<(), ErroApi> {
    if usuario.pode(crate::atos::Ato::PacienteCadastrar) || usuario.pode(crate::atos::Ato::SalaVer)
    {
        Ok(())
    } else {
        Err(ErroApi::proibido())
    }
}

/// Filtros já classificados (um por tipo de termo; `None` quando não vale).
#[derive(Default)]
struct Filtros {
    nome: Option<String>,
    nascimento: Option<NaiveDate>,
    digitos: Option<String>,
    codigo: Option<String>,
}

fn filtros(q: Option<&str>) -> Result<Filtros, ErroApi> {
    let Some(q) = q.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(Filtros::default());
    };
    let digitos: String = q.chars().filter(char::is_ascii_digit).collect();
    // CPF com ou sem pontuação (11 dígitos) também procura no celular: os dois têm 11 números.
    if digitos.len() == 11 && q.chars().all(|c| c.is_ascii_digit() || " .-()".contains(c)) {
        return Ok(Filtros {
            digitos: Some(digitos),
            ..Filtros::default()
        });
    }
    match classificar(q).map_err(|m| ErroApi::invalido("busca-invalida", m))? {
        Termo::Codigo(c) => Ok(Filtros {
            codigo: Some(c),
            ..Filtros::default()
        }),
        Termo::Nascimento(d) => Ok(Filtros {
            nascimento: Some(d),
            ..Filtros::default()
        }),
        Termo::Celular(c) => Ok(Filtros {
            digitos: Some(c),
            ..Filtros::default()
        }),
        Termo::Nome(n) => Ok(Filtros {
            nome: Some(n),
            ..Filtros::default()
        }),
    }
}

/// Lista paginada de pacientes da unidade com recortes e situação calculada no banco.
#[utoipa::path(get, operation_id = "listar_pacientes", path = "/v1/pacientes/lista", tag = "pacientes", params(FiltroDaLista),
    responses((status = 200, body = ListaDePacientes), (status = 403, body = crate::openapi::ProblemaDoc), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn listar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(f): Query<FiltroDaLista>,
) -> Result<Response, ErroApi> {
    ve_pacientes(&usuario)?;
    let recorte = f.recorte.as_deref().unwrap_or("todos");
    if !RECORTES.contains(&recorte) {
        return Err(ErroApi::invalido(
            "recorte-invalido",
            "Recorte desconhecido. Use todos, hoje, pendência ou incompleto.",
        ));
    }
    let pagina = f.pagina.unwrap_or(1);
    if !(1..=10_000).contains(&pagina) {
        return Err(ErroApi::invalido(
            "pagina-invalida",
            "Página fora do intervalo. Nada foi buscado.",
        ));
    }
    let fl = filtros(f.q.as_deref())?;
    let hoje = tempo::hoje_local();
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let cadastrados = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM identidade.paciente"#)
        .fetch_one(&mut *tx)
        .await?;
    // Uma passada calcula as marcas de cada paciente filtrado; a contagem dos recortes e a página
    // saem da mesma base, então os números da tela e as linhas nunca divergem.
    let c = sqlx::query!(
        r#"WITH base AS (
             SELECT p.id,
                    EXISTS (SELECT 1 FROM recepcao.atendimento a JOIN agenda.agendamento g ON g.id = a.agendamento_id
                             WHERE a.paciente_id = p.id AND (g.inicio AT TIME ZONE 'America/Sao_Paulo')::date = $5
                               AND a.situacao <> 'cancelado') AS hoje,
                    EXISTS (SELECT 1 FROM agenda.agendamento g WHERE g.paciente_id = p.id
                             AND g.situacao = 'reserva_provisoria' AND g.inicio >= now()) AS pendencia,
                    (p.cpf IS NULL OR p.celular IS NULL OR p.nome_mae IS NULL
                     OR (p.alergias IS NULL AND p.nega_alergias IS NULL)) AS incompleto
               FROM identidade.paciente p
              WHERE ($1::text IS NULL OR comum.sem_acento(p.nome) LIKE '%' || comum.sem_acento($1) || '%')
                AND ($2::date IS NULL OR p.nascimento = $2)
                AND ($3::text IS NULL OR p.celular = $3 OR p.cpf = $3)
                AND ($4::text IS NULL OR p.codigo = $4))
           SELECT count(*) AS "todos!", count(*) FILTER (WHERE hoje) AS "hoje!",
                  count(*) FILTER (WHERE pendencia) AS "pendencia!", count(*) FILTER (WHERE incompleto) AS "incompleto!"
             FROM base"#,
        fl.nome, fl.nascimento, fl.digitos, fl.codigo, hoje
    )
    .fetch_one(&mut *tx)
    .await?;
    let duplicados = crate::rotas::pacientes_duplicados::contar_abertos(&mut tx).await?;
    let total = match recorte {
        "hoje" => c.hoje,
        "pendencia" => c.pendencia,
        "incompleto" => c.incompleto,
        _ => c.todos,
    };
    let linhas = sqlx::query!(
        r#"WITH base AS (
             SELECT p.id, p.codigo, p.nome, p.nascimento, p.cpf, p.celular, p.nome_mae,
                    coalesce(p.alergias, '{}') AS "alergias!", p.nega_alergias,
                    EXISTS (SELECT 1 FROM recepcao.atendimento a JOIN agenda.agendamento g ON g.id = a.agendamento_id
                             WHERE a.paciente_id = p.id AND (g.inicio AT TIME ZONE 'America/Sao_Paulo')::date = $5
                               AND a.situacao <> 'cancelado') AS hoje,
                    EXISTS (SELECT 1 FROM agenda.agendamento g WHERE g.paciente_id = p.id
                             AND g.situacao = 'reserva_provisoria' AND g.inicio >= now()) AS pendencia,
                    (p.cpf IS NULL OR p.celular IS NULL OR p.nome_mae IS NULL
                     OR (p.alergias IS NULL AND p.nega_alergias IS NULL)) AS incompleto
               FROM identidade.paciente p
              WHERE ($1::text IS NULL OR comum.sem_acento(p.nome) LIKE '%' || comum.sem_acento($1) || '%')
                AND ($2::date IS NULL OR p.nascimento = $2)
                AND ($3::text IS NULL OR p.celular = $3 OR p.cpf = $3)
                AND ($4::text IS NULL OR p.codigo = $4))
           SELECT b.id, b.codigo, b.nome, b.nascimento, b.cpf, b.celular, b.nome_mae, b."alergias!", b.nega_alergias,
                  b.hoje AS "hoje!", b.pendencia AS "pendencia!", b.incompleto AS "incompleto!",
                  px.exame_nome AS "proximo_exame?", px.inicio AS "proximo_inicio?", px.operadora AS "proximo_operadora?",
                  ul.exame_nome AS "ultimo_exame?", ul.inicio AS "ultimo_inicio?", ul.operadora AS "ultimo_operadora?",
                  EXISTS (SELECT 1 FROM identidade.paciente o
                           WHERE o.id <> b.id AND comum.sem_acento(o.nome) = comum.sem_acento(b.nome)
                             AND (o.nascimento = b.nascimento OR o.celular = b.celular OR o.nome_mae = b.nome_mae
                                  OR date_part('year', o.nascimento) = date_part('year', b.nascimento))
                             AND NOT EXISTS (SELECT 1 FROM identidade.par_duplicado d
                                              WHERE d.paciente_menor = LEAST(o.id, b.id) AND d.paciente_maior = GREATEST(o.id, b.id)
                                                AND d.decisao = 'pessoas_diferentes'
                                                AND NOT EXISTS (SELECT 1 FROM identidade.par_duplicado s WHERE s.substitui = d.id))) AS "duplicado!"
             FROM base b
             LEFT JOIN LATERAL (SELECT g.exame_nome, g.inicio, gu.operadora FROM agenda.agendamento g
                                  LEFT JOIN convenio.guia gu ON gu.id = g.guia_id
                                 WHERE g.paciente_id = b.id AND g.inicio >= now()
                                   AND g.situacao IN ('reserva_provisoria','agendado','confirmado','em_atendimento')
                                 ORDER BY g.inicio LIMIT 1) px ON true
             LEFT JOIN LATERAL (SELECT g.exame_nome, g.inicio, gu.operadora FROM agenda.agendamento g
                                  LEFT JOIN convenio.guia gu ON gu.id = g.guia_id
                                 WHERE g.paciente_id = b.id AND g.situacao IN ('concluido','em_atendimento') AND g.inicio < now()
                                 ORDER BY g.inicio DESC LIMIT 1) ul ON true
            WHERE ($6 = 'todos' OR ($6 = 'hoje' AND b.hoje) OR ($6 = 'pendencia' AND b.pendencia) OR ($6 = 'incompleto' AND b.incompleto))
            ORDER BY b.nome, b.nascimento, b.id
            LIMIT $7 OFFSET $8"#,
        fl.nome, fl.nascimento, fl.digitos, fl.codigo, hoje, recorte, POR_PAGINA, (pagina - 1) * POR_PAGINA
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let itens = linhas
        .into_iter()
        .map(|l| {
            let mut falta = Vec::new();
            if l.cpf.is_none() {
                falta.push("cpf".to_owned());
            }
            if l.celular.is_none() {
                falta.push("celular".to_owned());
            }
            if l.nome_mae.is_none() {
                falta.push("nome_mae".to_owned());
            }
            if l.alergias.is_empty() && l.nega_alergias.is_none() {
                falta.push("alergias".to_owned());
            }
            let situacao = if l.hoje {
                "na_clinica"
            } else if l.pendencia {
                "pendencia"
            } else if l.incompleto {
                "cadastro_incompleto"
            } else if l.duplicado {
                "possivel_duplicado"
            } else {
                "em_dia"
            };
            let convenio = l
                .proximo_operadora
                .clone()
                .or_else(|| l.ultimo_operadora.clone())
                .unwrap_or_else(|| "Particular".to_owned());
            LinhaDaListaDePacientes {
                id: l.id,
                codigo: l.codigo,
                nome: l.nome,
                nascimento: l.nascimento,
                convenio,
                cpf_mascarado: l.cpf.as_deref().map(mascarar_cpf),
                celular: l.celular,
                alergias: l.alergias,
                ultimo_exame: l
                    .ultimo_exame
                    .zip(l.ultimo_inicio)
                    .map(|(exame, inicio)| ExameDaLista { exame, inicio }),
                proximo_exame: l
                    .proximo_exame
                    .zip(l.proximo_inicio)
                    .map(|(exame, inicio)| ExameDaLista { exame, inicio }),
                situacao: situacao.to_owned(),
                falta_no_cadastro: falta,
            }
        })
        .collect();
    let lista = ListaDePacientes {
        cadastrados,
        recortes: ContagemDosRecortes {
            todos: c.todos,
            hoje: c.hoje,
            pendencia: c.pendencia,
            incompleto: c.incompleto,
            duplicados,
        },
        pagina,
        paginas: ((total + POR_PAGINA - 1) / POR_PAGINA).max(1),
        por_pagina: POR_PAGINA,
        total,
        itens,
    };
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(lista)).into_response())
}

#[cfg(test)]
mod testes {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn classifica_cpf_celular_nascimento_codigo_e_nome() {
        assert_eq!(
            filtros(Some("123.456.789-09")).unwrap().digitos.as_deref(),
            Some("12345678909")
        );
        assert_eq!(
            filtros(Some("(48) 99812-4410")).unwrap().digitos.as_deref(),
            Some("48998124410")
        );
        assert!(filtros(Some("12/03/1968")).unwrap().nascimento.is_some());
        assert_eq!(
            filtros(Some("pac-2026-000002")).unwrap().codigo.as_deref(),
            Some("PAC-2026-000002")
        );
        assert_eq!(
            filtros(Some("maria")).unwrap().nome.as_deref(),
            Some("maria")
        );
        assert!(filtros(Some("  ")).unwrap().nome.is_none());
        assert!(filtros(Some("ab")).is_err());
    }
}
