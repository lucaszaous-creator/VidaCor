//! Rotas `/v1` (docs/29 § 6.2). Toda rota declara o ato exigido no próprio handler.

pub mod a_receber;
pub mod achados;
pub mod achados_contrato;
pub mod agenda;
pub mod agendamentos;
pub mod aparelhos;
pub mod atendimentos;
pub mod ausencias;
pub mod autorizacoes;
pub mod autorizacoes_contrato;
pub mod autorizacoes_portal;
pub mod autorizacoes_resposta;
pub mod bloqueios;
pub mod busca;
pub mod cabecalho;
pub mod cadastro;
pub mod caixa;
pub mod cancelamento;
pub mod catalogo;
pub mod chamada;
pub mod comunicacao;
pub mod configuracoes;
pub mod contingencia;
pub mod convenios;
pub mod corpo_clinico;
pub mod documentos;
pub mod dose;
pub mod equipe;
pub mod estoque;
pub mod estornos;
pub mod estudos;
pub mod etiquetas;
pub mod eventos;
pub mod externo;
pub mod faturamento;
pub mod financeiro;
pub mod fluxo_laudos;
pub mod frases_laudo;
pub mod indicadores;
pub mod inicio;
pub mod lastro_explorador;
pub mod laudos;
pub mod lista_espera;
pub mod menu;
pub mod minha_agenda;
pub mod modelos_laudo;
pub mod nfse;
pub mod nrd;
pub mod ofertas;
pub mod orcamentos;
pub mod ouvidoria;
pub mod paciente_visao;
pub mod pacientes;
pub mod pacotes;
pub mod pendencias;
pub mod portal;
pub mod portal_acesso;
pub mod preparo;
pub mod qualidade;
pub mod repasse;
pub mod respostas;
pub mod revisao_coordenacao;
pub mod revisao_pares;
pub mod sala;
pub mod saude;
pub mod sessao;
pub mod triagem;
pub mod usuarios;
pub mod vagas;
pub mod visualizacao;
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
        .route(
            "/v1/pacientes",
            get(pacientes::buscar).post(cadastro::cadastrar),
        )
        .route(
            "/v1/pacientes/possiveis-duplicados",
            get(cadastro::duplicados),
        )
        .route("/v1/pacientes/{id}/correcao", post(pacientes::corrigir))
        .route("/v1/pacientes/{id}/visao", get(paciente_visao::visao))
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
        .route("/v1/laudos/{id}/pdf", get(documentos::laudo::laudo_pdf))
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
        .route(
            "/v1/laudos/{id}/achados",
            get(achados::listar).post(achados::criar),
        )
        .route(
            "/v1/laudos/{id}/achados/{achado_id}/remocao",
            post(achados::remover),
        )
        .route("/v1/laudos/{id}/links", post(portal::criar_link))
        .route(
            "/v1/pacientes/{id}/convites-portal",
            post(portal_acesso::convidar),
        )
        .route("/v1/portal/convites/{token}", get(portal_acesso::situacao))
        .route(
            "/v1/portal/convites/{token}/aceite",
            post(portal_acesso::aceitar),
        )
        .route(
            "/v1/portal/sessao",
            get(portal_acesso::atual).post(portal_acesso::entrar),
        )
        .route("/v1/portal/sessao/encerramento", post(portal_acesso::sair))
        .route("/v1/portal/laudos", get(portal::meus_laudos))
        .route("/v1/portal/laudos/{id}", get(portal::meu_laudo))
        .route(
            "/v1/portal/laudos/{id}/pdf",
            get(documentos::laudo::portal_laudo_pdf),
        )
        .route("/v1/links/{token}", get(portal::abrir_link))
        .route("/v1/caixas", post(caixa::abrir))
        .route("/v1/caixas/meu", get(caixa::meu))
        .route("/v1/caixa/a-receber", get(a_receber::a_receber))
        .route("/v1/caixas/{id}/fechamento", post(caixa::fechar))
        .route("/v1/recebimentos", post(caixa::receber))
        .route("/v1/recebimentos/{id}", get(caixa::obter_recebimento))
        .route("/v1/recebimentos/{id}/nota", get(nfse::nota))
        .route("/v1/recebimentos/{id}/estornos", post(caixa::pedir_estorno))
        .route("/v1/autorizacoes-desconto", post(caixa::autorizar_desconto))
        .route("/v1/estornos/{id}/aprovacao", post(caixa::aprovar_estorno))
        .route("/v1/estoque/itens", get(estoque::itens))
        .route("/v1/estoque/entradas", post(estoque::entrada))
        .route("/v1/estoque/lotes/{id}/rastreio", get(estoque::rastreio))
        .route("/v1/atendimentos/{id}/contraste", post(estoque::contraste))
        .route("/v1/lastro/cadeia", get(lastro_explorador::cadeia))
        .route("/v1/lastro/dossie", post(documentos::dossie::dossie))
        .route(
            "/v1/agendamentos/{id}/comprovante",
            get(documentos::atendimento::comprovante),
        )
        .route(
            "/v1/atendimentos/{id}/declaracao",
            get(documentos::atendimento::declaracao),
        )
        .route(
            "/v1/caixa/recebimentos/{id}/recibo",
            get(documentos::atendimento::recibo),
        )
        .route("/v1/preparos", get(preparo::listar))
        .route(
            "/v1/preparos/{tuss}",
            get(preparo::versoes).post(preparo::publicar),
        )
        .route("/v1/indicadores", get(indicadores::indicadores))
        .route("/v1/usuarios", get(usuarios::listar).post(usuarios::criar))
        .route(
            "/v1/pacientes/{id}/exames-externos",
            get(externo::listar).post(externo::importar),
        )
        .route("/v1/atendimentos/{id}/chamadas", post(chamada::chamar))
        .route("/v1/painel-de-chamada", get(chamada::painel))
        .route("/v1/inicio", get(inicio::inicio))
        .route("/v1/busca", get(busca::buscar))
        .route("/v1/qualidade", get(qualidade::painel))
        .route("/v1/qualidade/nao-conformidades", post(qualidade::abrir_nc))
        .route(
            "/v1/qualidade/nao-conformidades/{id}/andamento",
            post(qualidade::andamento),
        )
        .route("/v1/qualidade/eventos", post(qualidade::notificar))
        .route(
            "/v1/qualidade/eventos/{id}/situacao",
            post(qualidade::situacao_evento),
        )
        .route(
            "/v1/ouvidoria",
            get(ouvidoria::painel).post(ouvidoria::registrar),
        )
        .route("/v1/ouvidoria/{id}/resposta", post(ouvidoria::responder))
        .route("/v1/laudos/fluxo", get(fluxo_laudos::fluxo))
        .route(
            "/v1/modelos-de-laudo",
            get(modelos_laudo::listar).post(modelos_laudo::criar),
        )
        .route("/v1/modelos-de-laudo/{id}", get(modelos_laudo::obter))
        .route(
            "/v1/modelos-de-laudo/{id}/versoes",
            post(modelos_laudo::nova_versao),
        )
        .route(
            "/v1/modelos-de-laudo/{id}/versoes/{numero}/decisao",
            post(modelos_laudo::decidir),
        )
        .route(
            "/v1/modelos-de-laudo/{id}/desativacao",
            post(modelos_laudo::desativar),
        )
        .route("/v1/laudos/{id}/usos-de-modelo", post(modelos_laudo::usar))
        .route(
            "/v1/frases-de-laudo",
            get(frases_laudo::listar).post(frases_laudo::criar),
        )
        .route(
            "/v1/frases-de-laudo/{id}/desativacao",
            post(frases_laudo::desativar),
        )
        .route(
            "/v1/revisoes-por-pares",
            post(revisao_coordenacao::designar),
        )
        .route(
            "/v1/revisoes-por-pares/sorteio",
            post(revisao_coordenacao::sortear),
        )
        .route(
            "/v1/revisoes-por-pares/painel",
            get(revisao_coordenacao::painel),
        )
        .route("/v1/revisoes-por-pares/fila", get(revisao_pares::fila))
        .route(
            "/v1/revisoes-por-pares/meus-laudos",
            get(revisao_pares::meus_laudos),
        )
        .route("/v1/revisoes-por-pares/{id}", get(revisao_pares::obter))
        .route(
            "/v1/revisoes-por-pares/{id}/parecer",
            post(revisao_pares::parecer),
        )
        .route(
            "/v1/revisoes-por-pares/{id}/conflito",
            post(revisao_pares::conflito),
        )
        .route(
            "/v1/revisoes-por-pares/{id}/pendencia",
            post(revisao_pares::decidir_pendencia),
        )
        .route(
            "/v1/configuracoes/integracoes",
            get(configuracoes::integracoes),
        )
        .route(
            "/v1/agendamentos/{id}/cancelamento",
            post(cancelamento::cancelar),
        )
        .route("/v1/estudos/{id}/visualizacao", get(visualizacao::abrir))
        .route(
            "/v1/bloqueios",
            get(bloqueios::listar).post(bloqueios::criar),
        )
        .route("/v1/bloqueios/{id}/retirada", post(bloqueios::retirar))
        .route(
            "/v1/lista-espera",
            get(lista_espera::listar).post(lista_espera::incluir),
        )
        .route("/v1/lista-espera/{id}/saida", post(lista_espera::sair))
        .route("/v1/ofertas", get(ofertas::listar).post(ofertas::oferecer))
        .route("/v1/ofertas/{id}/aceite", post(ofertas::aceitar))
        .route("/v1/ofertas/{id}/recusa", post(ofertas::recusar))
        .route(
            "/v1/aparelhos",
            get(aparelhos::listar).post(aparelhos::cadastrar),
        )
        .route("/v1/aparelhos/{id}/mudancas", post(aparelhos::mudar))
        .route("/v1/corpo-clinico", get(corpo_clinico::listar))
        .route(
            "/v1/corpo-clinico/{id}/consultas-crm",
            get(corpo_clinico::conferencias::consultas)
                .post(corpo_clinico::conferencias::consultar),
        )
        .route(
            "/v1/corpo-clinico/{id}/certificados",
            post(corpo_clinico::conferencias::cadastrar_certificado),
        )
        .route(
            "/v1/corpo-clinico/{id}/historico",
            get(corpo_clinico::historico),
        )
        .route(
            "/v1/corpo-clinico/{id}/pedidos",
            post(corpo_clinico::pedidos::pedir),
        )
        .route(
            "/v1/corpo-clinico/pedidos/{id}/decisao",
            post(corpo_clinico::pedidos::decidir),
        )
        .route(
            "/v1/catalogo/procedimentos",
            get(catalogo::listar).post(catalogo::mudancas::incluir),
        )
        .route(
            "/v1/catalogo/procedimentos/{tuss}/historico",
            get(catalogo::historico),
        )
        .route(
            "/v1/catalogo/procedimentos/{tuss}/mudancas",
            post(catalogo::mudancas::mudar),
        )
        .route(
            "/v1/catalogo/tabelas-de-preco",
            post(catalogo::precos::publicar_tabela),
        )
        .route(
            "/v1/configuracoes/cabecalho",
            get(cabecalho::ver).post(cabecalho::publicar),
        )
        .route(
            "/v1/convenios",
            get(convenios::listar).post(convenios::contratos::cadastrar),
        )
        .route(
            "/v1/convenios/propostas/{id}/decisao",
            post(convenios::decisao::decidir),
        )
        .route("/v1/convenios/{id}", get(convenios::obter))
        .route(
            "/v1/convenios/{id}/alteracoes",
            post(convenios::contratos::alterar),
        )
        .route("/v1/convenios/{id}/precos", get(convenios::tabelas::precos))
        .route(
            "/v1/convenios/{id}/previa-de-reajuste",
            get(convenios::tabelas::previa),
        )
        .route(
            "/v1/convenios/{id}/propostas",
            post(convenios::tabelas::propor),
        )
        .route(
            "/v1/convenios/{id}/regras-de-autorizacao",
            post(convenios::regras::publicar),
        )
        .route(
            "/v1/pacotes",
            get(pacotes::listar).post(pacotes::publicar::criar),
        )
        .route(
            "/v1/pacotes/{codigo}/versoes",
            post(pacotes::publicar::nova_versao),
        )
        .route("/v1/orcamentos/catalogo", get(orcamentos::catalogo))
        .route(
            "/v1/orcamentos",
            get(orcamentos::listar).post(orcamentos::emitir),
        )
        .route(
            "/v1/orcamentos/{id}/cancelamento",
            post(orcamentos::cancelar),
        )
        .route("/v1/usuarios/{id}/mudancas", post(usuarios::mudar))
        .route("/v1/usuarios/pedidos/{id}/decisao", post(usuarios::decidir))
        .route(
            "/v1/financeiro/titulos",
            get(financeiro::titulos).post(financeiro::lancar),
        )
        .route(
            "/v1/financeiro/titulos/{id}/aprovacoes",
            post(financeiro::aprovar),
        )
        .route(
            "/v1/financeiro/titulos/{id}/baixa",
            post(financeiro::baixar),
        )
        .route(
            "/v1/financeiro/extratos",
            post(financeiro::importar_extrato),
        )
        .route("/v1/financeiro/resumo", get(financeiro::resumo))
        .route("/v1/financeiro/contas", get(financeiro::contas))
        .route("/v1/financeiro/repasse", get(repasse::previa))
        .route(
            "/v1/financeiro/repasse/{competencia}/titulos",
            post(repasse::gerar),
        )
        .route("/v1/financeiro/regras-repasse", post(repasse::nova_regra))
        .route("/v1/estornos", get(estornos::listar))
        .route("/v1/menu/agendamento/{id}", get(menu::agendamento))
        .route("/v1/menu/atendimento/{id}", get(menu::atendimento))
        .route("/v1/menu/laudo/{id}", get(menu::laudo))
        .route("/v1/estornos/{id}", get(estornos::obter))
        .route("/v1/estornos/{id}/recusa", post(estornos::recusar))
        .route(
            "/v1/pedidos-desconto",
            post(pendencias::pedir_desconto).get(pendencias::listar_pedidos),
        )
        .route("/v1/pedidos-desconto/{id}", get(pendencias::obter_pedido))
        .route(
            "/v1/pedidos-desconto/{id}/recusa",
            post(pendencias::recusar_desconto),
        )
        .route(
            "/v1/pedidos-desconto/{id}/cancelamento",
            post(pendencias::cancelar_desconto),
        )
        .route("/v1/webhooks/pix", post(webhooks::pix))
        .route("/v1/webhooks/whatsapp", post(webhooks::whatsapp))
        .route("/v1/comunicacao/lembretes", post(comunicacao::lembretes))
        .route(
            "/v1/pacientes/{id}/comunicacao",
            get(comunicacao::do_paciente).post(comunicacao::preferir),
        )
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
        .route(
            "/v1/faturamento/conferencias",
            post(faturamento::conferencia::conferir),
        )
        .route(
            "/v1/faturamento/contas",
            get(faturamento::consultas::listar),
        )
        .route(
            "/v1/faturamento/contas/{id}",
            get(faturamento::consultas::obter),
        )
        .route(
            "/v1/faturamento/contas/{id}/correcao",
            post(faturamento::conferencia::corrigir),
        )
        .route(
            "/v1/faturamento/auditoria",
            get(faturamento::consultas::auditoria),
        )
        .route(
            "/v1/faturamento/contratos",
            get(faturamento::consultas::contratos),
        )
        .route(
            "/v1/faturamento/lotes",
            get(faturamento::lotes::listar).post(faturamento::lotes::gerar),
        )
        .route("/v1/faturamento/lotes/{id}", get(faturamento::lotes::obter))
        .route(
            "/v1/faturamento/lotes/{id}/arquivo",
            get(faturamento::lotes::arquivo),
        )
        .route(
            "/v1/faturamento/lotes/{id}/envio",
            post(faturamento::lotes::enviar),
        )
        .route(
            "/v1/faturamento/lotes/{id}/demonstrativo",
            post(faturamento::demonstrativo::demonstrativo),
        )
        .route("/v1/faturamento/glosas", get(faturamento::glosas::listar))
        .route(
            "/v1/faturamento/glosas/{id}/recurso",
            post(faturamento::glosas::recorrer),
        )
        .route(
            "/v1/autorizacoes",
            get(autorizacoes::listar).post(autorizacoes::pedir),
        )
        .route("/v1/autorizacoes/{id}", get(autorizacoes::obter))
        .route(
            "/v1/autorizacoes/{id}/protocolo",
            post(autorizacoes_portal::protocolo),
        )
        .route(
            "/v1/autorizacoes/{id}/resposta",
            post(autorizacoes_portal::resposta_do_portal),
        )
        .route(
            "/v1/autorizacoes/{id}/consulta",
            post(autorizacoes::consultar),
        )
        // Agente 2: minha agenda e ausência do médico (DEC-2026-11-09-140 a 143).
        .route("/v1/minha-agenda", get(minha_agenda::obter))
        .route("/v1/minha-agenda/ausencias", post(ausencias::pedir))
        .route("/v1/ausencias", get(ausencias::listar))
        .route("/v1/ausencias/{id}/decisao", post(ausencias::decidir))
        .route("/v1/ausencias/{id}/cancelamento", post(ausencias::cancelar))
        // Agente 2: gestão de dose (DEC-2026-11-09-144 a 149).
        .route("/v1/doses", get(dose::listar))
        .route("/v1/doses/protocolos", get(dose::protocolos))
        .route("/v1/doses/pacientes/{id}", get(dose::historico))
        .route("/v1/doses/{id}/justificativa", post(dose::justificar))
        .route("/v1/nrd", get(nrd::nrd))
        .route("/v1/nrd/{tuss}", post(nrd::publicar_nrd))
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
