//! Menu de contexto (docs/19 § 8): a lista de ações de um objeto vem do servidor, calculada pelo
//! perfil (ato) e pelo estado do objeto. Ação que o perfil nunca terá não vem; indisponível vem com
//! motivo. Cada ação leva a tela que chama a MESMA rota da ação fora do menu (`destino`, só com
//! identificadores técnicos). Executar continua conferindo permissão na rota de destino (G1).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::sessao::Usuario;
use crate::tempo;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema, Debug, PartialEq, Eq)]
pub struct AcaoDoMenu {
    pub id: String,
    pub rotulo: String,
    /// disponivel ou indisponivel.
    pub estado: String,
    /// Obrigatório quando indisponível.
    pub motivo: Option<String>,
    /// Destrutiva: vai para o fim, em vermelho.
    pub perigo: bool,
    /// Caminho da tela que faz a ação (relativo, só com UUID). Sem destino: ação local da tela
    /// (ex.: copiar nome e nascimento).
    pub destino: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CabecalhoDoMenu {
    pub titulo: String,
    pub subtitulo: String,
}

#[derive(Serialize, ToSchema)]
pub struct MenuDoObjeto {
    pub cabecalho: CabecalhoDoMenu,
    pub acoes: Vec<AcaoDoMenu>,
}

fn acao(id: &str, rotulo: &str, destino: Option<String>) -> AcaoDoMenu {
    AcaoDoMenu {
        id: id.into(),
        rotulo: rotulo.into(),
        estado: "disponivel".into(),
        motivo: None,
        perigo: false,
        destino,
    }
}

impl AcaoDoMenu {
    fn indisponivel_se(mut self, motivo: Option<String>) -> Self {
        if let Some(m) = motivo {
            self.estado = "indisponivel".into();
            self.motivo = Some(m);
            self.destino = None;
        }
        self
    }
    fn perigosa(mut self) -> Self {
        self.perigo = true;
        self
    }
}

fn nascimento(d: chrono::NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

/// Estado do agendamento que decide o menu (separado da consulta para ser testável).
pub struct EstadoDoAgendamento {
    pub id: Uuid,
    pub situacao: String,
    pub dia: chrono::NaiveDate,
    pub requer_contraste: bool,
    pub sala_id: Uuid,
    pub atendimento: Option<(Uuid, String)>,
}

/// Ações do agendamento para o usuário (catálogo do docs/19 § 7, só o que já existe no sistema).
#[must_use]
pub fn acoes_do_agendamento(
    u: &Usuario,
    a: &EstadoDoAgendamento,
    hoje: chrono::NaiveDate,
) -> Vec<AcaoDoMenu> {
    let mut v = Vec::new();
    let sem_checkin = a
        .atendimento
        .is_none()
        .then(|| "Faça o check-in antes".to_owned());
    let fechado = matches!(a.situacao.as_str(), "cancelado" | "faltou" | "expirada");
    if u.pode(Ato::RecepcaoCheckin) || u.pode(Ato::AgendaAgendar) {
        v.push(acao(
            "abrir",
            "Abrir na recepção",
            Some(format!("/recepcao?agendamento={}", a.id)),
        ));
    }
    if u.pode(Ato::RecepcaoCheckin) {
        let motivo = if let Some((_, codigo)) = &a.atendimento {
            Some(format!("Check-in já feito ({codigo})"))
        } else if fechado || a.situacao == "reserva_provisoria" {
            Some(format!("Agendamento {}", a.situacao.replace('_', " ")))
        } else if a.dia != hoje {
            Some("Check-in só no dia do exame".to_owned())
        } else {
            None
        };
        v.push(
            acao(
                "checkin",
                "Fazer check-in",
                Some(format!("/recepcao?agendamento={}", a.id)),
            )
            .indisponivel_se(motivo),
        );
    }
    if a.requer_contraste && (u.pode(Ato::TriagemRegistrar) || u.pode(Ato::TriagemConduta)) {
        let destino = a
            .atendimento
            .as_ref()
            .map(|(id, _)| format!("/triagem/{id}"));
        v.push(
            acao("triagem", "Abrir triagem de contraste", destino)
                .indisponivel_se(sem_checkin.clone()),
        );
    }
    if u.pode(Ato::SalaVer) {
        v.push(
            acao(
                "sala",
                "Abrir na sala",
                Some(format!("/sala?sala={}", a.sala_id)),
            )
            .indisponivel_se(sem_checkin.clone()),
        );
    }
    if u.pode(Ato::CaixaOperar) {
        v.push(
            acao(
                "receber",
                "Receber no caixa",
                Some(format!("/caixa?data={}", a.dia)),
            )
            .indisponivel_se(sem_checkin.clone()),
        );
    }
    v.push(acao("copiar", "Copiar nome e nascimento", None));
    if u.pode(Ato::EtiquetaReimprimir) {
        // Reimprimir invalida a pulseira anterior (docs/20): destrutiva, com motivo na tela.
        v.push(
            acao(
                "reimprimir",
                "Reimprimir pulseira",
                Some(format!("/recepcao?agendamento={}", a.id)),
            )
            .indisponivel_se(sem_checkin)
            .perigosa(),
        );
    }
    v
}

/// Menu de contexto de um agendamento (agenda, recepção).
#[utoipa::path(get, path = "/v1/menu/agendamento/{id}", tag = "menu", params(("id" = Uuid, Path)),
    responses((status = 200, body = MenuDoObjeto), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn agendamento(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<MenuDoObjeto>, ErroApi> {
    menu_do_agendamento(&estado, &usuario, Some(id), None).await
}

/// Menu de contexto de um atendimento (fila da sala): as mesmas ações do agendamento dele.
#[utoipa::path(get, path = "/v1/menu/atendimento/{id}", tag = "menu", params(("id" = Uuid, Path)),
    responses((status = 200, body = MenuDoObjeto), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn atendimento(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<MenuDoObjeto>, ErroApi> {
    menu_do_agendamento(&estado, &usuario, None, Some(id)).await
}

async fn menu_do_agendamento(
    estado: &Estado,
    usuario: &Usuario,
    agendamento: Option<Uuid>,
    atendimento: Option<Uuid>,
) -> Result<Json<MenuDoObjeto>, ErroApi> {
    if !(usuario.pode(Ato::AgendaVer)
        || usuario.pode(Ato::RecepcaoVer)
        || usuario.pode(Ato::SalaVer))
    {
        return Err(ErroApi::proibido());
    }
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let l = sqlx::query!(
        r#"SELECT g.id, g.situacao, g.inicio, g.requer_contraste, g.sala_id, g.exame_nome, p.nome, p.nascimento,
                  a.id AS "atendimento_id?", a.codigo AS "atendimento_codigo?"
           FROM agenda.agendamento g
           JOIN identidade.paciente p ON p.id = g.paciente_id
           LEFT JOIN recepcao.atendimento a ON a.agendamento_id = g.id
           WHERE ($1::uuid IS NOT NULL AND g.id = $1) OR ($2::uuid IS NOT NULL AND a.id = $2)"#,
        agendamento,
        atendimento
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("agendamento-nao-encontrado", "Agendamento não encontrado."))?;
    tx.commit().await?;
    let e = EstadoDoAgendamento {
        id: l.id,
        situacao: l.situacao,
        dia: l.inicio.with_timezone(&tempo::FUSO).date_naive(),
        requer_contraste: l.requer_contraste,
        sala_id: l.sala_id,
        atendimento: l.atendimento_id.zip(l.atendimento_codigo),
    };
    Ok(Json(MenuDoObjeto {
        cabecalho: CabecalhoDoMenu {
            titulo: l.nome,
            subtitulo: format!(
                "Nascimento {}, {}, {}",
                nascimento(l.nascimento),
                l.inicio.with_timezone(&tempo::FUSO).format("%H:%M"),
                l.exame_nome
            ),
        },
        acoes: acoes_do_agendamento(usuario, &e, tempo::hoje_local()),
    }))
}

/// Ações do laudo na fila (docs/19 § 7: laudo na fila e laudo assinado; sem desfazer assinatura).
#[must_use]
pub fn acoes_do_laudo(
    u: &Usuario,
    id: Uuid,
    situacao: &str,
    atribuido: Option<Uuid>,
) -> Vec<AcaoDoMenu> {
    if !u.pode(Ato::LaudoLer) {
        return Vec::new();
    }
    let abrir = Some(format!("/laudos/{id}"));
    let assinado = situacao == "assinado";
    let de_outro = atribuido
        .is_some_and(|m| m != u.id)
        .then(|| "Laudo atribuído a outro médico".to_owned());
    let mut v = vec![acao("abrir", "Abrir no laudo", abrir.clone())];
    // Laudo impresso: só versão assinada (com retificação em rascunho, imprime a vigente).
    v.push(
        acao("pdf", "Baixar PDF", abrir.clone()).indisponivel_se(
            (situacao == "rascunho").then(|| "Laudo ainda não assinado".to_owned()),
        ),
    );
    if u.pode(Ato::LaudoEscrever) {
        v.push(
            acao("assinar", "Assinar", abrir.clone()).indisponivel_se(if assinado {
                Some("Laudo já assinado".to_owned())
            } else {
                de_outro.clone()
            }),
        );
        v.push(
            acao("link", "Compartilhar link seguro", abrir.clone())
                .indisponivel_se((!assinado).then(|| "Laudo ainda não assinado".to_owned())),
        );
        v.push(
            acao("retificar", "Retificar (nova versão)", abrir).indisponivel_se(if assinado {
                de_outro
            } else {
                Some("Só laudo assinado é retificado".to_owned())
            }),
        );
    }
    v.push(acao("copiar", "Copiar nome e nascimento", None));
    v
}

/// Menu de contexto de um laudo (fila de laudos).
#[utoipa::path(get, path = "/v1/menu/laudo/{id}", tag = "menu", params(("id" = Uuid, Path)),
    responses((status = 200, body = MenuDoObjeto), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn laudo(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<MenuDoObjeto>, ErroApi> {
    usuario.exigir(Ato::LaudoLer)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let l = sqlx::query!(
        r#"SELECT l.id, l.codigo, l.situacao, l.medico_atribuido_id, p.nome, p.nascimento
           FROM laudos.laudo l JOIN identidade.paciente p ON p.id = l.paciente_id WHERE l.id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ErroApi::nao_encontrado("laudo-nao-encontrado", "Laudo não encontrado."))?;
    tx.commit().await?;
    Ok(Json(MenuDoObjeto {
        cabecalho: CabecalhoDoMenu {
            titulo: l.nome,
            subtitulo: format!("Nascimento {}, {}", nascimento(l.nascimento), l.codigo),
        },
        acoes: acoes_do_laudo(&usuario, l.id, &l.situacao, l.medico_atribuido_id),
    }))
}

#[cfg(test)]
mod testes {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn usuario(perfil: &str) -> Usuario {
        Usuario {
            id: Uuid::nil(),
            nome: "Teste".into(),
            perfis: vec![perfil.into()],
            unidade_id: Uuid::nil(),
            sessao_id: Uuid::nil(),
            csrf: String::new(),
            reautenticado_em: chrono::Utc::now(),
        }
    }
    fn hoje() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 11, 8).unwrap()
    }
    fn agendado(atendimento: bool) -> EstadoDoAgendamento {
        EstadoDoAgendamento {
            id: Uuid::nil(),
            situacao: "confirmado".into(),
            dia: hoje(),
            requer_contraste: true,
            sala_id: Uuid::nil(),
            atendimento: atendimento.then(|| (Uuid::nil(), "ATD-2026-000001".into())),
        }
    }
    fn ids(v: &[AcaoDoMenu]) -> Vec<&str> {
        v.iter().map(|a| a.id.as_str()).collect()
    }

    #[test]
    fn acao_que_o_perfil_nunca_tera_nao_aparece() {
        let caixa = acoes_do_agendamento(&usuario("caixa"), &agendado(true), hoje());
        assert!(!ids(&caixa).contains(&"checkin"));
        assert!(!ids(&caixa).contains(&"reimprimir"));
        assert!(ids(&caixa).contains(&"receber"));
        assert!(acoes_do_laudo(&usuario("recepcao"), Uuid::nil(), "assinado", None).is_empty());
        assert!(
            acoes_do_agendamento(&usuario("ti"), &agendado(true), hoje())
                .iter()
                .all(|a| a.id == "copiar")
        );
    }

    #[test]
    fn indisponivel_vem_com_motivo_e_sem_destino() {
        let r = acoes_do_agendamento(&usuario("recepcao"), &agendado(true), hoje());
        let checkin = r.iter().find(|a| a.id == "checkin").unwrap();
        assert_eq!(
            (
                checkin.estado.as_str(),
                checkin.motivo.as_deref(),
                &checkin.destino
            ),
            (
                "indisponivel",
                Some("Check-in já feito (ATD-2026-000001)"),
                &None
            )
        );
        let amanha = acoes_do_agendamento(
            &usuario("recepcao"),
            &EstadoDoAgendamento {
                dia: hoje().succ_opt().unwrap(),
                ..agendado(false)
            },
            hoje(),
        );
        assert_eq!(
            amanha
                .iter()
                .find(|a| a.id == "checkin")
                .unwrap()
                .motivo
                .as_deref(),
            Some("Check-in só no dia do exame")
        );
        let tec = acoes_do_agendamento(&usuario("tecnico"), &agendado(false), hoje());
        assert_eq!(
            tec.iter()
                .find(|a| a.id == "triagem")
                .unwrap()
                .motivo
                .as_deref(),
            Some("Faça o check-in antes")
        );
    }

    #[test]
    fn reimprimir_e_destrutiva_e_vai_no_fim() {
        let r = acoes_do_agendamento(&usuario("recepcao"), &agendado(true), hoje());
        let ultima = r.last().unwrap();
        assert_eq!((ultima.id.as_str(), ultima.perigo), ("reimprimir", true));
    }

    #[test]
    fn pdf_do_laudo_so_depois_de_assinado() {
        let estado = |situacao: &str| {
            let r = acoes_do_laudo(&usuario("medico"), Uuid::nil(), situacao, None);
            let pdf = r.iter().find(|a| a.id == "pdf").unwrap();
            (pdf.estado.clone(), pdf.motivo.clone())
        };
        assert_eq!(
            estado("rascunho"),
            (
                "indisponivel".to_owned(),
                Some("Laudo ainda não assinado".to_owned())
            )
        );
        assert_eq!(estado("assinado"), ("disponivel".to_owned(), None));
        assert_eq!(
            estado("retificacao_em_rascunho"),
            ("disponivel".to_owned(), None)
        );
    }

    #[test]
    fn laudo_assinado_nao_assina_de_novo_e_so_retifica() {
        let r = acoes_do_laudo(
            &usuario("medico"),
            Uuid::nil(),
            "assinado",
            Some(Uuid::nil()),
        );
        let assinar = r.iter().find(|a| a.id == "assinar").unwrap();
        assert_eq!(assinar.motivo.as_deref(), Some("Laudo já assinado"));
        assert_eq!(
            r.iter().find(|a| a.id == "retificar").unwrap().estado,
            "disponivel"
        );
        let rascunho_de_outro = acoes_do_laudo(
            &usuario("medico"),
            Uuid::nil(),
            "rascunho",
            Some(Uuid::max()),
        );
        assert_eq!(
            rascunho_de_outro
                .iter()
                .find(|a| a.id == "assinar")
                .unwrap()
                .motivo
                .as_deref(),
            Some("Laudo atribuído a outro médico")
        );
        assert_eq!(
            rascunho_de_outro
                .iter()
                .find(|a| a.id == "link")
                .unwrap()
                .motivo
                .as_deref(),
            Some("Laudo ainda não assinado")
        );
    }

    #[test]
    fn destino_so_leva_identificador_tecnico() {
        for a in acoes_do_agendamento(&usuario("recepcao"), &agendado(true), hoje()) {
            if let Some(d) = a.destino {
                assert!(d.starts_with('/') && !d.contains("Teste"), "{d}");
            }
        }
    }
}
