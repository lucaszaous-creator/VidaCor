//! Busca global (Busca-Global.dc.html): pacientes por nome, nascimento, celular ou prontuário e
//! objetos por código legível (agendamento, atendimento, etiqueta, laudo). O que aparece é só o que o
//! perfil já pode ler: cada consulta passa pelo RLS da sessão. Sem contagem de "ocultos" (seria
//! oráculo de enumeração, SEC). Termos curtos ou longos demais são recusados (limites, SEC).

use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Deserialize, IntoParams)]
pub struct TermoDeBusca {
    /// Nome (3+ letras), nascimento (dd/mm/aaaa), celular (10 ou 11 dígitos) ou código (ex.: AGD-2026-000123).
    pub q: String,
}

#[derive(Serialize, ToSchema)]
pub struct ResultadoDaBusca {
    /// paciente, agendamento, atendimento, etiqueta ou laudo.
    pub tipo: String,
    pub id: Uuid,
    pub titulo: String,
    pub detalhe: String,
    /// Caminho da tela que abre o objeto (só identificadores técnicos).
    pub destino: String,
}

/// O que o termo parece ser.
#[derive(Debug, PartialEq, Eq)]
pub enum Termo {
    Codigo(String),
    Nascimento(NaiveDate),
    Celular(String),
    Nome(String),
}

/// Classifica o termo (puro, testado abaixo).
///
/// # Errors
/// Termo vazio, curto (menos de 3 letras) ou longo (mais de 100).
pub fn classificar(q: &str) -> Result<Termo, &'static str> {
    let t = q.trim();
    let n = t.chars().count();
    if !(3..=100).contains(&n) {
        return Err(
            "Digite pelo menos 3 letras, o nascimento (dd/mm/aaaa), o celular ou um código.",
        );
    }
    let maiusculo = t.to_uppercase();
    let partes: Vec<&str> = maiusculo.split('-').collect();
    if partes.len() == 3
        && partes[0].len() == 3
        && partes[0].bytes().all(|b| b.is_ascii_uppercase())
        && partes[1].len() == 4
        && partes[1].bytes().all(|b| b.is_ascii_digit())
        && !partes[2].is_empty()
        && partes[2].len() <= 8
        && partes[2].bytes().all(|b| b.is_ascii_digit())
    {
        return Ok(Termo::Codigo(maiusculo));
    }
    if let Ok(d) = NaiveDate::parse_from_str(t, "%d/%m/%Y") {
        return Ok(Termo::Nascimento(d));
    }
    let digitos: String = t.chars().filter(char::is_ascii_digit).collect();
    if (10..=11).contains(&digitos.len())
        && t.chars().all(|c| c.is_ascii_digit() || " ()-+".contains(c))
    {
        return Ok(Termo::Celular(digitos));
    }
    Ok(Termo::Nome(t.replace(['%', '_'], "")))
}

/// Busca em tudo o que o perfil pode ver (qualquer pessoa da equipe com sessão).
#[utoipa::path(get, operation_id = "busca_global", path = "/v1/busca", tag = "sessao", params(TermoDeBusca),
    responses((status = 200, body = Vec<ResultadoDaBusca>), (status = 422, body = crate::openapi::ProblemaDoc)))]
pub async fn buscar(
    State(estado): State<Estado>,
    usuario: Usuario,
    Query(b): Query<TermoDeBusca>,
) -> Result<Response, ErroApi> {
    // A conta da TV de chamada fica em área pública e não busca nada (docs/18).
    if usuario.tem_perfil("tv") {
        return Err(ErroApi::proibido());
    }
    let termo = classificar(&b.q).map_err(|m| ErroApi::invalido("busca-invalida", m))?;
    // Mínimo necessário: paciente só aparece para quem abre a tela do paciente. O caixa lê a
    // identificação pelo RLS (0015) para receber, mas pela tela dele, não pela busca.
    let ve_paciente = usuario.pode(crate::atos::Ato::PacienteCadastrar)
        || usuario.pode(crate::atos::Ato::SalaVer);
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let mut r: Vec<ResultadoDaBusca> = Vec::new();
    let paciente = |id: Uuid, nome: String, nasc: NaiveDate, codigo: String| ResultadoDaBusca {
        tipo: "paciente".into(),
        id,
        titulo: nome,
        detalhe: format!(
            "nascimento {}, prontuário {codigo}",
            nasc.format("%d/%m/%Y")
        ),
        destino: format!("/pacientes?id={id}"),
    };
    match &termo {
        Termo::Nome(_) | Termo::Nascimento(_) | Termo::Celular(_) if !ve_paciente => {}
        Termo::Nome(nome) => {
            for p in sqlx::query!("SELECT id, nome, nascimento, codigo FROM identidade.paciente WHERE nome ILIKE '%' || $1 || '%' ORDER BY nome LIMIT 10", nome)
                .fetch_all(&mut *tx)
                .await?
            {
                r.push(paciente(p.id, p.nome, p.nascimento, p.codigo));
            }
        }
        Termo::Nascimento(d) => {
            for p in sqlx::query!("SELECT id, nome, nascimento, codigo FROM identidade.paciente WHERE nascimento = $1 ORDER BY nome LIMIT 10", d)
                .fetch_all(&mut *tx)
                .await?
            {
                r.push(paciente(p.id, p.nome, p.nascimento, p.codigo));
            }
        }
        Termo::Celular(c) => {
            for p in sqlx::query!("SELECT id, nome, nascimento, codigo FROM identidade.paciente WHERE celular = $1 ORDER BY nome LIMIT 10", c)
                .fetch_all(&mut *tx)
                .await?
            {
                r.push(paciente(p.id, p.nome, p.nascimento, p.codigo));
            }
        }
        Termo::Codigo(c) => {
            if let Some(p) = sqlx::query!("SELECT id, nome, nascimento, codigo FROM identidade.paciente WHERE codigo = $1 AND $2", c, ve_paciente).fetch_optional(&mut *tx).await? {
                r.push(paciente(p.id, p.nome, p.nascimento, p.codigo));
            }
            if let Some(a) = sqlx::query!(
                "SELECT a.id, a.codigo, a.exame_nome, a.inicio, p.nome FROM agenda.agendamento a JOIN identidade.paciente p ON p.id = a.paciente_id WHERE a.codigo = $1",
                c
            )
            .fetch_optional(&mut *tx)
            .await?
            {
                r.push(ResultadoDaBusca {
                    tipo: "agendamento".into(),
                    id: a.id,
                    titulo: format!("{} ({})", a.codigo, a.nome),
                    detalhe: format!("{}, {}", a.exame_nome, a.inicio.with_timezone(&crate::tempo::FUSO).format("%d/%m/%Y %H:%M")),
                    destino: format!("/agenda?data={}", a.inicio.with_timezone(&crate::tempo::FUSO).format("%Y-%m-%d")),
                });
            }
            if let Some(t) = sqlx::query!(
                r#"SELECT t.id, t.codigo, t.criado_em, p.nome FROM recepcao.atendimento t JOIN identidade.paciente p ON p.id = t.paciente_id
                    WHERE t.codigo = $1 OR t.id = (SELECT e.atendimento_id FROM recepcao.etiqueta e WHERE e.codigo = $1)"#,
                c
            )
            .fetch_optional(&mut *tx)
            .await?
            {
                r.push(ResultadoDaBusca {
                    tipo: if c.starts_with("ETQ-") { "etiqueta" } else { "atendimento" }.into(),
                    id: t.id,
                    titulo: format!("{} ({})", t.codigo, t.nome),
                    detalhe: format!("atendimento de {}", t.criado_em.with_timezone(&crate::tempo::FUSO).format("%d/%m/%Y")),
                    destino: format!("/recepcao?data={}", t.criado_em.with_timezone(&crate::tempo::FUSO).format("%Y-%m-%d")),
                });
            }
            // Laudo: o RLS só deixa o médico com vínculo ver; para os demais, nada aparece.
            if let Some(l) = sqlx::query!("SELECT id, codigo, situacao FROM laudos.laudo WHERE codigo = $1", c).fetch_optional(&mut *tx).await? {
                r.push(ResultadoDaBusca { tipo: "laudo".into(), id: l.id, titulo: l.codigo, detalhe: format!("laudo, {}", l.situacao), destino: format!("/laudos/{}", l.id) });
            }
        }
    }
    tx.commit().await?;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(r)).into_response())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn classifica_codigo_data_celular_e_nome() {
        assert_eq!(
            classificar("agd-2026-000123"),
            Ok(Termo::Codigo("AGD-2026-000123".into()))
        );
        assert_eq!(
            classificar("12/03/1968"),
            Ok(Termo::Nascimento(
                NaiveDate::from_ymd_opt(1968, 3, 12).unwrap()
            ))
        );
        assert_eq!(
            classificar("(21) 90000-0017"),
            Ok(Termo::Celular("21900000017".into()))
        );
        assert_eq!(classificar("Maria"), Ok(Termo::Nome("Maria".into())));
        assert_eq!(classificar("50%_"), Ok(Termo::Nome("50".into())));
        assert!(classificar("ab").is_err());
        assert!(classificar(&"x".repeat(101)).is_err());
    }
}
