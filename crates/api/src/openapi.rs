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
        crate::rotas::achados::listar,
        crate::rotas::achados::criar,
        crate::rotas::achados::remover,
        crate::rotas::portal::criar_link,
        crate::rotas::portal::meus_laudos,
        crate::rotas::portal::meu_laudo,
        crate::rotas::portal::abrir_link,
        crate::rotas::caixa::abrir,
        crate::rotas::caixa::meu,
        crate::rotas::a_receber::a_receber,
        crate::rotas::caixa::fechar,
        crate::rotas::caixa::receber,
        crate::rotas::caixa::obter_recebimento,
        crate::rotas::caixa::autorizar_desconto,
        crate::rotas::caixa::pedir_estorno,
        crate::rotas::caixa::aprovar_estorno,
        crate::rotas::pendencias::pedir_desconto,
        crate::rotas::pendencias::listar_pedidos,
        crate::rotas::pendencias::obter_pedido,
        crate::rotas::pendencias::recusar_desconto,
        crate::rotas::pendencias::cancelar_desconto,
        crate::rotas::estornos::listar,
        crate::rotas::estornos::obter,
        crate::rotas::estornos::recusar,
        crate::rotas::menu::agendamento,
        crate::rotas::menu::atendimento,
        crate::rotas::menu::laudo,
        crate::rotas::portal_acesso::convidar,
        crate::rotas::cadastro::cadastrar,
        crate::rotas::estoque::itens,
        crate::rotas::lastro_explorador::cadeia,
        crate::rotas::indicadores::indicadores,
        crate::rotas::comunicacao::lembretes,
        crate::rotas::financeiro::titulos,
        crate::rotas::financeiro::lancar,
        crate::rotas::financeiro::aprovar,
        crate::rotas::financeiro::baixar,
        crate::rotas::financeiro::importar_extrato,
        crate::rotas::financeiro::resumo,
        crate::rotas::financeiro::contas,
        crate::rotas::repasse::previa,
        crate::rotas::repasse::gerar,
        crate::rotas::repasse::nova_regra,
        crate::rotas::comunicacao::do_paciente,
        crate::rotas::comunicacao::preferir,
        crate::rotas::webhooks::whatsapp,
        crate::rotas::estoque::entrada,
        crate::rotas::estoque::contraste,
        crate::rotas::estoque::rastreio,
        crate::rotas::cadastro::duplicados,
        crate::rotas::portal_acesso::situacao,
        crate::rotas::portal_acesso::aceitar,
        crate::rotas::portal_acesso::entrar,
        crate::rotas::portal_acesso::atual,
        crate::rotas::portal_acesso::sair,
        crate::rotas::webhooks::pix,
        crate::rotas::contingencia::declarar,
        crate::rotas::contingencia::aberta,
        crate::rotas::contingencia::kit,
        crate::rotas::contingencia::lancar_ficha,
        crate::rotas::contingencia::lancar_recibo,
        crate::rotas::contingencia::situar_numeros,
        crate::rotas::contingencia::encerrar,
        crate::rotas::contingencia::resolver_divergencia,
        crate::rotas::estudos::pendentes,
        crate::rotas::estudos::conciliar,
        crate::rotas::etiquetas::nova_pulseira,
        crate::rotas::equipe::listar,
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

#[cfg(test)]
mod testes {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    /// O cliente TypeScript é gerado por operationId: repetido, uma rota some do cliente.
    #[test]
    fn operation_ids_sao_unicos() {
        let doc: serde_json::Value = serde_json::from_str(&super::json().unwrap()).unwrap();
        let mut vistos = std::collections::BTreeSet::new();
        for (caminho, metodos) in doc["paths"].as_object().unwrap() {
            for (metodo, op) in metodos.as_object().unwrap() {
                let id = op["operationId"].as_str().unwrap_or_default().to_owned();
                assert!(
                    vistos.insert(id.clone()),
                    "operationId repetido: {id} em {metodo} {caminho}"
                );
            }
        }
    }
}
