//! Contrato OpenAPI gerado do código (docs/11 § 9.2): fonte da verdade do cliente TypeScript.

use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

/// Erro em Problem Details (RFC 9457).
#[derive(Serialize, ToSchema)]
pub struct ProblemaDoc {
    #[serde(rename = "type")]
    pub tipo: String,
    pub title: String,
    pub status: u16,
    /// Mensagem para a tela: o que aconteceu, o que foi feito e o que fazer agora.
    pub detail: String,
    /// Código estável (ex.: `identificacao-nao-confere`).
    pub codigo: String,
    /// Código de suporte.
    pub trace_id: Option<String>,
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "VidaCor API",
        version = "0.1.0",
        description = "Núcleo VidaCor (docs/29)"
    ),
    paths(
        crate::rotas::saude::saude,
        crate::rotas::sessao::atual,
        crate::rotas::sessao::entrar,
        crate::rotas::sessao::retorno,
        crate::rotas::sessao::sair,
        crate::rotas::agenda::salas,
        crate::rotas::agenda::agenda_do_dia,
        crate::rotas::atendimentos::checkin,
        crate::rotas::atendimentos::obter,
        crate::rotas::sessao::reautenticar,
        crate::rotas::triagem::registrar,
        crate::rotas::triagem::conduta,
        crate::rotas::etiquetas::reimprimir,
        crate::rotas::pacientes::buscar,
        crate::rotas::pacientes::corrigir,
        crate::rotas::sala::bipe,
        crate::rotas::sala::fila,
        crate::rotas::agendamentos::registrar_guia,
        crate::rotas::agendamentos::agendar,
        crate::rotas::agendamentos::confirmar_com_guia,
        crate::rotas::agendamentos::horarios_livres,
        crate::rotas::laudos::fila,
        crate::rotas::laudos::obter,
        crate::rotas::laudos::salvar_rascunho,
        crate::rotas::laudos::assinar,
        crate::rotas::laudos::retificar,
        crate::rotas::laudos::conversar_antes,
        crate::rotas::laudos::liberar,
        crate::rotas::laudos::publicar,
        crate::rotas::laudos::acesso_emergencia,
        crate::rotas::portal::criar_link,
        crate::rotas::portal::meus_laudos,
        crate::rotas::portal::meu_laudo,
        crate::rotas::portal::abrir_link,
        crate::rotas::caixa::abrir,
        crate::rotas::caixa::meu,
        crate::rotas::caixa::fechar,
        crate::rotas::caixa::receber,
        crate::rotas::caixa::obter_recebimento,
        crate::rotas::caixa::autorizar_desconto,
        crate::rotas::caixa::pedir_estorno,
        crate::rotas::caixa::aprovar_estorno,
        crate::rotas::webhooks::pix,
        crate::rotas::eventos::fluxo,
    ),
    components(schemas(ProblemaDoc))
)]
pub struct Documentacao;

/// JSON do contrato, para `just contrato`.
///
/// # Errors
/// Falha de serialização.
pub fn json() -> Result<String, serde_json::Error> {
    Documentacao::openapi().to_pretty_json()
}
