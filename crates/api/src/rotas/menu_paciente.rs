//! Menu de contexto do paciente na lista (docs/19 § 7 e § 8): abrir a ficha, agendar e copiar nome e
//! nascimento. A lista vem do servidor pelo perfil; executar continua conferindo a permissão na rota
//! de destino (G1). Arquivo próprio para não crescer `menu.rs` (G5).

use crate::atos::Ato;
use crate::banco;
use crate::erro::ErroApi;
use crate::estado::Estado;
use crate::rotas::menu::{AcaoDoMenu, CabecalhoDoMenu, MenuDoObjeto};
use crate::rotas::pacientes_lista::ve_pacientes;
use crate::sessao::Usuario;
use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

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

/// Ações do paciente para o usuário (só o que o perfil pode; o resto não aparece).
#[must_use]
pub fn acoes_do_paciente(u: &Usuario, id: Uuid) -> Vec<AcaoDoMenu> {
    let mut v = vec![acao(
        "abrir",
        "Abrir ficha",
        Some(format!("/pacientes?id={id}")),
    )];
    if u.pode(Ato::AgendaAgendar) {
        v.push(acao(
            "agendar",
            "Agendar exame",
            Some("/agendamentos/novo".to_owned()),
        ));
    }
    v.push(acao("copiar", "Copiar nome e nascimento", None));
    v
}

/// Menu de contexto de um paciente (lista de pacientes).
#[utoipa::path(get, path = "/v1/menu/paciente/{id}", tag = "menu", params(("id" = Uuid, Path)),
    responses((status = 200, body = MenuDoObjeto), (status = 403, body = crate::openapi::ProblemaDoc), (status = 404, body = crate::openapi::ProblemaDoc)))]
pub async fn paciente(
    State(estado): State<Estado>,
    usuario: Usuario,
    Path(id): Path<Uuid>,
) -> Result<Json<MenuDoObjeto>, ErroApi> {
    ve_pacientes(&usuario)?;
    let mut tx = banco::transacao(&estado.banco, &usuario.contexto()).await?;
    let p = sqlx::query!(
        "SELECT nome, nascimento, codigo FROM identidade.paciente WHERE id = $1",
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ErroApi::nao_encontrado(
            "paciente-nao-encontrado",
            "Paciente não encontrado nesta unidade.",
        )
    })?;
    tx.commit().await?;
    Ok(Json(MenuDoObjeto {
        cabecalho: CabecalhoDoMenu {
            titulo: p.nome,
            subtitulo: format!(
                "{}, prontuário {}",
                p.nascimento.format("%d/%m/%Y"),
                p.codigo
            ),
        },
        acoes: acoes_do_paciente(&usuario, id),
    }))
}
