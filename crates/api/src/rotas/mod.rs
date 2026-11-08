//! Rotas `/v1` (docs/29 § 6.2). Toda rota declara o ato exigido no próprio handler.

pub mod agenda;
pub mod agendamentos;
pub mod atendimentos;
pub mod caixa;
pub mod contingencia;
pub mod equipe;
pub mod estudos;
pub mod etiquetas;
pub mod eventos;
pub mod laudos;
pub mod pacientes;
pub mod portal;
pub mod respostas;
pub mod sala;
pub mod saude;
pub mod sessao;
pub mod triagem;
pub mod webhooks;

use crate::estado::Estado;
use axum::routing::{get, post};
use axum::Router;

pub fn rotas() -> Router<Estado> {
    Router::new()
        .route("/v1/saude", get(saude::saude))
        .route("/v1/sessao", get(sessao::atual))
        .route("/v1/sessao/entrar", get(sessao::entrar))
        .route("/v1/sessao/retorno", get(sessao::retorno))
        .route("/v1/sessao/sair", post(sessao::sair))
        .route("/v1/sessao/reautenticar", get(sessao::reautenticar))
        .route("/v1/salas", get(agenda::salas))
        .route("/v1/agenda", get(agenda::agenda_do_dia))
        .route("/v1/atendimentos", post(atendimentos::checkin))
        .route("/v1/atendimentos/{id}", get(atendimentos::obter))
        .route("/v1/atendimentos/{id}/triagem", post(triagem::registrar))
        .route(
            "/v1/atendimentos/{id}/triagem/conduta",
            post(triagem::conduta),
        )
        .route(
            "/v1/etiquetas/{codigo}/reimpressao",
            post(etiquetas::reimprimir),
        )
        .route("/v1/pacientes", get(pacientes::buscar))
        .route("/v1/pacientes/{id}/correcao", post(pacientes::corrigir))
        .route("/v1/salas/{sala_id}/bipes", post(sala::bipe))
        .route("/v1/salas/{sala_id}/fila", get(sala::fila))
        .route("/v1/guias", post(agendamentos::registrar_guia))
        .route("/v1/agendamentos", post(agendamentos::agendar))
        .route(
            "/v1/agendamentos/{id}/guia",
            post(agendamentos::confirmar_com_guia),
        )
        .route(
            "/v1/salas/{sala_id}/horarios-livres",
            get(agendamentos::horarios_livres),
        )
        .route("/v1/laudos/fila", get(laudos::fila))
        .route("/v1/laudos/{id}", get(laudos::obter))
        .route(
            "/v1/laudos/{id}/rascunho",
            axum::routing::put(laudos::salvar_rascunho),
        )
        .route("/v1/laudos/{id}/assinatura", post(laudos::assinar))
        .route("/v1/laudos/{id}/retificacao", post(laudos::retificar))
        .route(
            "/v1/laudos/{id}/conversar-antes",
            post(laudos::conversar_antes),
        )
        .route("/v1/laudos/{id}/liberacao", post(laudos::liberar))
        .route("/v1/laudos/{id}/publicacao", post(laudos::publicar))
        .route(
            "/v1/laudos/{id}/acesso-emergencia",
            post(laudos::acesso_emergencia),
        )
        .route("/v1/laudos/{id}/links", post(portal::criar_link))
        .route("/v1/portal/laudos", get(portal::meus_laudos))
        .route("/v1/portal/laudos/{id}", get(portal::meu_laudo))
        .route("/v1/links/{token}", get(portal::abrir_link))
        .route("/v1/caixas", post(caixa::abrir))
        .route("/v1/caixas/meu", get(caixa::meu))
        .route("/v1/caixas/{id}/fechamento", post(caixa::fechar))
        .route("/v1/recebimentos", post(caixa::receber))
        .route("/v1/recebimentos/{id}", get(caixa::obter_recebimento))
        .route("/v1/recebimentos/{id}/estornos", post(caixa::pedir_estorno))
        .route("/v1/autorizacoes-desconto", post(caixa::autorizar_desconto))
        .route("/v1/estornos/{id}/aprovacao", post(caixa::aprovar_estorno))
        .route("/v1/webhooks/pix", post(webhooks::pix))
        .route("/v1/contingencias", post(contingencia::declarar))
        .route("/v1/contingencias/aberta", get(contingencia::aberta))
        .route("/v1/contingencias/kits", post(contingencia::kit))
        .route(
            "/v1/contingencias/{id}/fichas",
            post(contingencia::lancar_ficha),
        )
        .route(
            "/v1/contingencias/{id}/recibos",
            post(contingencia::lancar_recibo),
        )
        .route(
            "/v1/contingencias/{id}/numeros",
            post(contingencia::situar_numeros),
        )
        .route(
            "/v1/contingencias/{id}/encerramento",
            post(contingencia::encerrar),
        )
        .route(
            "/v1/divergencias/{id}/resolucao",
            post(contingencia::resolver_divergencia),
        )
        .route("/v1/estudos/pendentes", get(estudos::pendentes))
        .route("/v1/estudos/{id}/conciliacao", post(estudos::conciliar))
        .route(
            "/v1/atendimentos/{id}/pulseira",
            post(etiquetas::nova_pulseira),
        )
        .route("/v1/equipe", get(equipe::listar))
        .route("/v1/eventos", get(eventos::fluxo))
        .fallback(nao_existe)
}

async fn nao_existe() -> crate::erro::ErroApi {
    crate::erro::ErroApi::nao_encontrado(
        "rota-inexistente",
        "Este endereço não existe no VidaCor. Nada foi feito. Volte para a tela inicial.",
    )
}
