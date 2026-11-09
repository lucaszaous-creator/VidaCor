/**
 * Conferência em tempo de compilação entre os tipos escritos à mão (que estreitam situações para
 * listas fechadas) e os tipos gerados do `openapi.json` do núcleo (`pnpm gerar`). Se o núcleo
 * renomear, tirar ou mudar o tipo de um campo, o `typecheck` quebra aqui antes de chegar à tela.
 * Valem os tipos da Fase 1, que são os usados pelas telas.
 */
import type { components } from './gerado/openapi';
import type { AutorizacaoDetalhe, AutorizacaoResumo, NovoPedidoAutorizacao, PedidoProtocolo, RespostaDoPortal } from './contrato-autorizacao';
import type { MenuDoObjeto } from './contrato-menu';
import type { ChamadaNoPainel, Painel, PedidoDeChamada } from './contrato-painel';
import type { CatalogoDoOrcamento, NovoOrcamento, Orcamento } from './contrato-orcamento';
import type { IntegracaoVista } from './contrato-configuracoes';
import type { DetalheDaEtiqueta, DivergenciaDeSala, DivergenciaResolvida, EtiquetaInvalidada, EventoDaEtiqueta, LinhaDeEtiqueta, PainelDeEtiquetas, PedidoResolucaoDeSala } from './contrato-etiquetas';
import type { DecisaoRegistrada, DecisaoSobreDuplicado, LinhaDaListaDePacientes, ListaDePacientes, ParDuplicado, VisaoDoPaciente } from './contrato-paciente';
import type { InicioDoDia } from './contrato-inicio';
import type { NovoPreparo, PreparoDoExame, VersaoDoPreparo } from './contrato-preparo';
import type { FluxoDeLaudos } from './contrato-fluxo';
import type {
  CasoDeRevisao,
  DecisaoDePendencia,
  DecisaoDeVersaoDeModelo,
  FraseDeLaudo,
  ItemDaFilaDeRevisao,
  ModeloDeLaudo,
  NovaFraseDeLaudo,
  NovaVersaoDeModelo,
  NovoModeloDeLaudo,
  PainelDeRevisao,
  ParecerDeRevisao,
  RevisaoDesignada,
  RevisoesDoAutor,
  ResultadoDoSorteio,
  UsoDeModelo,
} from './contrato-modelos';
import type { ResultadoDaBusca } from './contrato-busca';
import type { AndamentoNovo, EventoDeSeguranca, NaoConformidade, NovaNaoConformidade, NovoEventoAdverso, PainelDaQualidade, ProtocoloDeQualidade } from './contrato-qualidade';
import type { Manifestacao, NovaManifestacao, PainelDaOuvidoria, ProtocoloDaManifestacao } from './contrato-ouvidoria';
import type { BloqueioDeSala, OfertaDeVaga, PedidoBloqueio, PedidoDeEspera, PedidoInclusao } from './contrato-espera';
import type { AparelhoCadastrado, MudancaDeAparelho, NovoAparelho, PainelDosAparelhos } from './contrato-aparelhos';
import type {
  CabecalhoDaUnidade,
  CabecalhoDosDocumentos,
  CatalogoDeExames,
  DecisaoDoCorpoClinico,
  MedicoDoCorpoClinico,
  MudancaNoProcedimento,
  MudancaRegistradaNoCatalogo,
  NovaVersaoDaTabela,
  NovoCabecalho,
  NovoPedidoDoCorpoClinico,
  NovoProcedimento,
  PedidoDoCorpoClinico,
  ProcedimentoDoCatalogo,
  VersaoDaTabelaDePreco,
  ConsultaCrm,
  NovaConsultaCrm,
  CertificadoDoMedico,
  NovoCertificado,
} from './contrato-cadastros';
import type {
  AlteracaoDoConvenio,
  AuditoriaDoConvenio,
  CatalogoDePacotes,
  ConvenioDetalhe,
  ConvenioResumo,
  DecisaoDaPropostaDeTabela,
  MudancaDePrecoDoConvenio,
  MudancaDoConvenio,
  NovaPropostaDeTabela,
  NovaVersaoDoPacoteDeCheckup,
  NovasRegrasDeAutorizacao,
  NovoConvenio,
  NovoPacoteDeCheckup,
  PacoteDeCheckup,
  PrecosDoConvenio,
  PreviaDeReajuste,
  PropostaDeTabela,
  RegraDaAuditoriaDoConvenio,
  RegraDeAutorizacaoDoExame,
  RegrasDeAutorizacaoDoConvenio,
  TermosDoConvenio,
  VersaoDaTabelaDoConvenio,
  VersaoDoPacoteDeCheckup,
} from './contrato-convenios';
import type { CadeiaDoAtendimento, PedidoDossie } from './contrato-lastro';
import type { Indicadores } from './contrato-indicadores';
import type { ExameExterno, ImportacaoExterna, MudancaDePerfil, NovoUsuario, UsuarioAdministrado } from './contrato-usuarios';
import type { NotaFiscal, NovoTitulo, RepasseDoMedico, RepasseGerado, ResumoFinanceiro, Titulo } from './contrato-financeiro';
import type { ComunicacaoDoPaciente, LembretesEnfileirados } from './contrato-comunicacao';
import type {
  ContaDetalhe,
  ContratoDeConvenio,
  Glosa,
  LoteDetalhe,
  LoteTiss,
  PainelAuditoria,
  PedidoCorrecaoConta,
  PedidoDemonstrativo,
  PedidoLote,
  PedidoRecurso,
  ResultadoConferencia,
  ContaResumo,
} from './contrato-faturamento';
import type { EntradaDeEstoque, ItemDeEstoque, MovimentoFeito, RastreioDoLote } from './contrato-estoque';
import type { AchadoEstruturado, AchadosDoLaudo, DescritoresBirads, DescritoresTirads, NovoAchado } from './contrato-laudo-estruturado';
import type { ConviteEmitido, NovoPaciente, PacienteCadastrado, PossivelDuplicado, SessaoDoPortal, SituacaoDoConvite } from './contrato-portal';
import type { AtendimentoAReceber, EstornoDetalhado, NovoPedidoDesconto, PedidoDesconto } from './contrato-pendencias';
import type { NovoAtendimento, PacienteResumo, Sala, SalaDaUnidade, Sessao } from './contrato-m0';
import type {
  AgendaFase1,
  AtendimentoFase1,
  Caixa,
  CorrecaoCadastro,
  Etiqueta,
  EstudoPendente,
  FilaDaSala,
  Guia,
  ItemDaAgendaFase1,
  ItemDaFilaDeLaudos,
  Laudo,
  LaudoDoPortal,
  LinkCriado,
  PedidoAgendamento,
  PedidoGuia,
  PedidoRecebimento,
  PedidoTriagem,
  Recebimento,
  Reimpressao,
  ResultadoDoBipe,
  Triagem,
} from './contrato-fase1';

type S = components['schemas'];
type Cabe<A extends B, B> = A;

export type ConfereSala = Cabe<Sala, S['Sala']>;
export type ConfereSalaDaUnidade = Cabe<SalaDaUnidade, S['SalaDaUnidade']>;
export type ConferePaciente = Cabe<PacienteResumo, S['PacienteResumo']>;
export type ConfereSessao = Cabe<Sessao, S['SessaoAtual']>;
export type ConferePedidoCheckin = Cabe<NovoAtendimento, S['PedidoCheckin']>;
export type ConfereItemDaAgenda = Cabe<ItemDaAgendaFase1, S['ItemAgenda']>;
export type ConfereAgenda = Cabe<AgendaFase1, S['AgendaDoDia']>;
export type ConfereAtendimento = Cabe<AtendimentoFase1, S['Atendimento']>;
export type ConfereEtiqueta = Cabe<Etiqueta, S['Etiqueta']>;
export type ConfereReimpressao = Cabe<Reimpressao, S['Reimpressao']>;
export type ConfereCorrecao = Cabe<CorrecaoCadastro, S['Correcao']>;
export type ConfereBipe = Cabe<ResultadoDoBipe, S['ResultadoBipe']>;
export type ConfereFila = Cabe<FilaDaSala, S['FilaDaSala']>;
export type ConfereTriagem = Cabe<Triagem, S['Triagem']>;
export type ConferePedidoTriagem = Cabe<PedidoTriagem, S['PedidoTriagem']>;
export type ConfereGuia = Cabe<Guia, S['Guia']>;
export type ConferePedidoGuia = Cabe<PedidoGuia, S['PedidoGuia']>;
export type ConferePedidoAgendamento = Cabe<PedidoAgendamento, S['PedidoAgendamento']>;
export type ConfereLaudo = Cabe<Laudo, S['Laudo']>;
export type ConfereFilaDeLaudos = Cabe<ItemDaFilaDeLaudos, S['ItemFilaLaudo']>;
export type ConferePortal = Cabe<LaudoDoPortal, S['LaudoDoPortal']>;
export type ConfereLink = Cabe<LinkCriado, S['LinkCriado']>;
export type ConfereEstudoPendente = Cabe<EstudoPendente, S['EstudoPendente']>;
export type ConfereCaixa = Cabe<Caixa, S['Caixa']>;
export type ConfereRecebimento = Cabe<Recebimento, S['Recebimento']>;
export type ConferePedidoRecebimento = Cabe<PedidoRecebimento, S['PedidoRecebimento']>;

// Pendências da gerência no caixa.
export type ConferePedidoDesconto = Cabe<PedidoDesconto, S['PedidoDesconto']>;
export type ConfereNovoPedidoDesconto = Cabe<NovoPedidoDesconto, S['NovoPedidoDesconto']>;
export type ConfereEstornoDetalhado = Cabe<EstornoDetalhado, S['EstornoDetalhado']>;
export type ConfereMenuDoObjeto = Cabe<MenuDoObjeto, S['MenuDoObjeto']>;
export type ConfereConviteEmitido = Cabe<ConviteEmitido, S['ConviteEmitido']>;
export type ConfereSituacaoDoConvite = Cabe<SituacaoDoConvite, S['SituacaoDoConvite']>;
export type ConfereSessaoDoPortal = Cabe<SessaoDoPortal, S['SessaoDoPortal']>;
export type ConferePossivelDuplicado = Cabe<PossivelDuplicado, S['PossivelDuplicado']>;
export type ConfereNovoPaciente = Cabe<NovoPaciente, S['NovoPaciente']>;
export type ConferePacienteCadastrado = Cabe<PacienteCadastrado, S['PacienteCadastrado']>;
export type ConfereAtendimentoAReceber = Cabe<AtendimentoAReceber, S['AtendimentoAReceber']>;
export type ConfereItemDeEstoque = Cabe<ItemDeEstoque, S['ItemDeEstoque']>;
export type ConfereEntradaDeEstoque = Cabe<EntradaDeEstoque, S['EntradaDeEstoque']>;
export type ConfereMovimentoFeito = Cabe<MovimentoFeito, S['MovimentoFeito']>;
export type ConfereRastreioDoLote = Cabe<RastreioDoLote, S['RastreioDoLote']>;
export type ConfereCadeiaDoAtendimento = Cabe<CadeiaDoAtendimento, S['CadeiaDoAtendimento']>;
export type ConferePedidoDossie = Cabe<PedidoDossie, S['PedidoDossie']>;

// Laudo estruturado (E2).
export type ConfereNovoAchado = Cabe<NovoAchado, S['NovoAchado']>;
export type ConfereAchado = Cabe<AchadoEstruturado, S['Achado']>;
export type ConfereAchadosDoLaudo = Cabe<AchadosDoLaudo, S['AchadosDoLaudo']>;
export type ConfereDescritoresTirads = Cabe<DescritoresTirads, S['DescritoresTirads']>;
export type ConfereDescritoresBirads = Cabe<DescritoresBirads, S['DescritoresBirads']>;
// Ao contrário também: o corpo gerado cabe no escrito à mão (nada do servidor ficou de fora da tela).
export type ConfereNovoAchadoVolta = Cabe<S['NovoAchado'], NovoAchado>;
export type ConfereIndicadores = Cabe<Indicadores, S['Indicadores']>;
export type ConfereComunicacaoDoPaciente = Cabe<ComunicacaoDoPaciente, S['ComunicacaoDoPaciente']>;
export type ConfereLembretes = Cabe<LembretesEnfileirados, S['LembretesEnfileirados']>;
export type ConfereTitulo = Cabe<Titulo, S['Titulo']>;
export type ConfereNovoTitulo = Cabe<NovoTitulo, S['NovoTitulo']>;
export type ConfereResumoFinanceiro = Cabe<ResumoFinanceiro, S['ResumoFinanceiro']>;
export type ConfereRepasseDoMedico = Cabe<RepasseDoMedico, S['RepasseDoMedico']>;
export type ConfereRepasseGerado = Cabe<RepasseGerado, S['RepasseGerado']>;
export type ConfereUsuarioAdministrado = Cabe<UsuarioAdministrado, S['UsuarioAdministrado']>;
export type ConfereNovoUsuario = Cabe<NovoUsuario, S['NovoUsuario']>;
export type ConfereMudancaDePerfil = Cabe<MudancaDePerfil, S['MudancaDePerfil']>;
export type ConfereNotaFiscal = Cabe<NotaFiscal, S['NotaFiscal']>;

// Faturamento TISS (E5).
export type ConfereContaResumo = Cabe<ContaResumo, S['ContaResumo']>;
export type ConfereContaDetalhe = Cabe<ContaDetalhe, S['ContaDetalhe']>;
export type ConfereGlosa = Cabe<Glosa, S['Glosa']>;
export type ConfereLote = Cabe<LoteTiss, S['Lote']>;
export type ConfereLoteDetalhe = Cabe<LoteDetalhe, S['LoteDetalhe']>;
export type ConferePainelAuditoria = Cabe<PainelAuditoria, S['PainelAuditoria']>;
export type ConfereContrato = Cabe<ContratoDeConvenio, S['Contrato']>;
export type ConfereResultadoConferencia = Cabe<ResultadoConferencia, S['ResultadoConferencia']>;
export type ConferePedidoLote = Cabe<PedidoLote, S['PedidoLote']>;
export type ConferePedidoCorrecaoConta = Cabe<PedidoCorrecaoConta, S['PedidoCorrecaoConta']>;
export type ConferePedidoDemonstrativo = Cabe<PedidoDemonstrativo, S['PedidoDemonstrativo']>;
export type ConferePedidoRecurso = Cabe<PedidoRecurso, S['PedidoRecurso']>;
// Ao contrário: o corpo gerado cabe no escrito à mão.
export type ConferePedidoCorrecaoContaVolta = Cabe<S['PedidoCorrecaoConta'], PedidoCorrecaoConta>;
export type ConferePedidoDemonstrativoVolta = Cabe<S['PedidoDemonstrativo'], PedidoDemonstrativo>;
export type ConfereExameExterno = Cabe<ExameExterno, S['ExameExterno']>;
export type ConfereImportacaoExterna = Cabe<ImportacaoExterna, S['ImportacaoExterna']>;
export type ConfereChamadaNoPainel = Cabe<ChamadaNoPainel, S['ChamadaNoPainel']>;
export type ConferePainel = Cabe<Painel, S['Painel']>;
export type ConferePedidoDeChamada = Cabe<PedidoDeChamada, S['PedidoDeChamada']>;

// Autorização pedida pelo sistema (E4).
export type ConfereAutorizacaoResumo = Cabe<AutorizacaoResumo, S['AutorizacaoResumo']>;
export type ConfereAutorizacaoDetalhe = Cabe<AutorizacaoDetalhe, S['AutorizacaoDetalhe']>;
export type ConfereNovoPedidoAutorizacao = Cabe<NovoPedidoAutorizacao, S['NovoPedidoAutorizacao']>;
export type ConfereNovoPedidoAutorizacaoVolta = Cabe<S['NovoPedidoAutorizacao'], NovoPedidoAutorizacao>;
export type ConferePedidoProtocolo = Cabe<PedidoProtocolo, S['PedidoProtocolo']>;
export type ConfereRespostaDoPortal = Cabe<RespostaDoPortal, S['RespostaDoPortal']>;
export type ConfereCatalogoDoOrcamento = Cabe<CatalogoDoOrcamento, S['CatalogoDoOrcamento']>;
export type ConfereOrcamento = Cabe<Orcamento, S['Orcamento']>;
export type ConfereNovoOrcamento = Cabe<NovoOrcamento, S['NovoOrcamento']>;
export type ConfereAparelhoCadastrado = Cabe<AparelhoCadastrado, S['AparelhoCadastrado']>;
export type ConferePainelDosAparelhos = Cabe<PainelDosAparelhos, S['PainelDosAparelhos']>;
export type ConfereNovoAparelho = Cabe<NovoAparelho, S['NovoAparelho']>;
export type ConfereMudancaDeAparelho = Cabe<MudancaDeAparelho, S['MudancaDeAparelho']>;
export type ConferePedidoDeEspera = Cabe<PedidoDeEspera, S['PedidoDeEspera']>;
export type ConferePedidoInclusao = Cabe<PedidoInclusao, S['PedidoInclusao']>;
export type ConfereOfertaDeVaga = Cabe<OfertaDeVaga, S['Oferta']>;
export type ConferePedidoBloqueio = Cabe<PedidoBloqueio, S['PedidoBloqueio']>;
export type ConfereBloqueioDeSala = Cabe<BloqueioDeSala, S['Bloqueio']>;
export type ConfereIntegracaoVista = Cabe<IntegracaoVista, S['IntegracaoVista']>;
export type ConfereVisaoDoPaciente = Cabe<VisaoDoPaciente, S['VisaoDoPaciente']>;
export type ConfereListaDePacientes = Cabe<ListaDePacientes, S['ListaDePacientes']>;
export type ConfereLinhaDaListaDePacientes = Cabe<LinhaDaListaDePacientes, S['LinhaDaListaDePacientes']>;
export type ConfereParDuplicado = Cabe<ParDuplicado, S['ParDuplicado']>;
export type ConfereDecisaoSobreDuplicado = Cabe<DecisaoSobreDuplicado, S['DecisaoSobreDuplicado']>;
export type ConfereDecisaoRegistrada = Cabe<DecisaoRegistrada, S['DecisaoRegistrada']>;
export type ConferePainelDeEtiquetas = Cabe<PainelDeEtiquetas, S['PainelDeEtiquetas']>;
export type ConfereLinhaDeEtiqueta = Cabe<LinhaDeEtiqueta, S['LinhaDeEtiqueta']>;
export type ConfereDivergenciaDeSala = Cabe<DivergenciaDeSala, S['DivergenciaDeSala']>;
export type ConfereDetalheDaEtiqueta = Cabe<DetalheDaEtiqueta, S['DetalheDaEtiqueta']>;
export type ConfereEventoDaEtiqueta = Cabe<EventoDaEtiqueta, S['EventoDaEtiqueta']>;
export type ConfereEtiquetaInvalidada = Cabe<EtiquetaInvalidada, S['EtiquetaInvalidada']>;
export type ConferePedidoResolucaoDeSala = Cabe<PedidoResolucaoDeSala, S['PedidoResolucao']>;
export type ConfereDivergenciaResolvida = Cabe<DivergenciaResolvida, S['DivergenciaResolvida']>;
export type ConfereInicioDoDia = Cabe<InicioDoDia, S['InicioDoDia']>;
export type ConfereFluxoDeLaudos = Cabe<FluxoDeLaudos, S['FluxoDeLaudos']>;
export type ConfereResultadoDaBusca = Cabe<ResultadoDaBusca, S['ResultadoDaBusca']>;
export type ConfereNovaManifestacao = Cabe<NovaManifestacao, S['NovaManifestacao']>;
export type ConfereProtocoloDaManifestacao = Cabe<ProtocoloDaManifestacao, S['ProtocoloDaManifestacao']>;
export type ConfereManifestacao = Cabe<Manifestacao, S['Manifestacao']>;
export type ConferePainelDaOuvidoria = Cabe<PainelDaOuvidoria, S['PainelDaOuvidoria']>;

// Cadastros base: corpo clínico, catálogo de exames com tabela de preços e cabeçalho dos documentos.
export type ConfereMedicoDoCorpoClinico = Cabe<MedicoDoCorpoClinico, S['MedicoDoCorpoClinico']>;
export type ConferePedidoDoCorpoClinico = Cabe<PedidoDoCorpoClinico, S['PedidoDoCorpoClinico']>;
export type ConfereNovoPedidoDoCorpoClinico = Cabe<NovoPedidoDoCorpoClinico, S['NovoPedidoDoCorpoClinico']>;
export type ConfereDecisaoDoCorpoClinico = Cabe<DecisaoDoCorpoClinico, S['DecisaoDoCorpoClinico']>;
export type ConfereCatalogoDeExames = Cabe<CatalogoDeExames, S['CatalogoDeExames']>;
export type ConfereProcedimentoDoCatalogo = Cabe<ProcedimentoDoCatalogo, S['ProcedimentoDoCatalogo']>;
export type ConfereVersaoDaTabelaDePreco = Cabe<VersaoDaTabelaDePreco, S['VersaoDaTabelaDePreco']>;
export type ConfereMudancaRegistradaNoCatalogo = Cabe<MudancaRegistradaNoCatalogo, S['MudancaRegistradaNoCatalogo']>;
export type ConfereNovoProcedimento = Cabe<NovoProcedimento, S['NovoProcedimento']>;
export type ConfereMudancaNoProcedimento = Cabe<MudancaNoProcedimento, S['MudancaNoProcedimento']>;
export type ConfereNovaVersaoDaTabela = Cabe<NovaVersaoDaTabela, S['NovaVersaoDaTabela']>;
export type ConfereCabecalhoDaUnidade = Cabe<CabecalhoDaUnidade, S['CabecalhoDaUnidade']>;
export type ConfereCabecalhoDosDocumentos = Cabe<CabecalhoDosDocumentos, S['CabecalhoDosDocumentos']>;
export type ConfereNovoCabecalho = Cabe<NovoCabecalho, S['NovoCabecalho']>;
export type ConfereNovaNaoConformidade = Cabe<NovaNaoConformidade, S['NovaNaoConformidade']>;
export type ConfereNovoEventoAdverso = Cabe<NovoEventoAdverso, S['NovoEventoAdverso']>;
export type ConfereProtocoloDeQualidade = Cabe<ProtocoloDeQualidade, S['ProtocoloDeQualidade']>;
export type ConfereNaoConformidade = Cabe<NaoConformidade, S['NaoConformidade']>;
export type ConfereEventoDeSeguranca = Cabe<EventoDeSeguranca, S['EventoDeSeguranca']>;
export type ConferePainelDaQualidade = Cabe<PainelDaQualidade, S['PainelDaQualidade']>;
export type ConfereAndamentoNovo = Cabe<AndamentoNovo, S['AndamentoNovo']>;
export type ConfereModeloDeLaudo = Cabe<ModeloDeLaudo, S['ModeloDeLaudo']>;
export type ConfereNovoModeloDeLaudo = Cabe<NovoModeloDeLaudo, S['NovoModeloDeLaudo']>;
export type ConfereNovaVersaoDeModelo = Cabe<NovaVersaoDeModelo, S['NovaVersaoDeModelo']>;
export type ConfereDecisaoDeVersaoDeModelo = Cabe<DecisaoDeVersaoDeModelo, S['DecisaoDeVersaoDeModelo']>;
export type ConfereUsoDeModelo = Cabe<UsoDeModelo, S['UsoDeModelo']>;
export type ConfereFraseDeLaudo = Cabe<FraseDeLaudo, S['FraseDeLaudo']>;
export type ConfereNovaFraseDeLaudo = Cabe<NovaFraseDeLaudo, S['NovaFraseDeLaudo']>;
export type ConfereItemDaFilaDeRevisao = Cabe<ItemDaFilaDeRevisao, S['ItemDaFilaDeRevisao']>;
export type ConfereCasoDeRevisao = Cabe<CasoDeRevisao, S['CasoDeRevisao']>;
export type ConfereParecerDeRevisao = Cabe<ParecerDeRevisao, S['ParecerDeRevisao']>;
export type ConfereRevisoesDoAutor = Cabe<RevisoesDoAutor, S['RevisoesDoAutor']>;
export type ConfereDecisaoDePendencia = Cabe<DecisaoDePendencia, S['DecisaoDePendencia']>;
export type ConfereRevisaoDesignada = Cabe<RevisaoDesignada, S['RevisaoDesignada']>;
export type ConfereResultadoDoSorteio = Cabe<ResultadoDoSorteio, S['ResultadoDoSorteio']>;
export type ConferePainelDeRevisao = Cabe<PainelDeRevisao, S['PainelDeRevisao']>;
export type ConferePedidoSorteio = Cabe<{ quantidade: number; dias: number }, S['PedidoSorteio']>;
export type ConferePedidoDesignacao = Cabe<{ laudo_id: string; revisor_id: string }, S['PedidoDesignacao']>;
export type ConferePedidoUsoDeModelo = Cabe<{ modelo_id: string; numero: number }, S['PedidoUsoDeModelo']>;

// Preparo do exame (versões com motivo).
export type ConfereVersaoDoPreparo = Cabe<VersaoDoPreparo, S['VersaoDoPreparo']>;
export type ConferePreparoDoExame = Cabe<PreparoDoExame, S['PreparoDoExame']>;
export type ConfereNovoPreparo = Cabe<NovoPreparo, S['NovoPreparo']>;

// Corpo clínico: consulta do CRM no CFM e certificado ICP-Brasil.
export type ConfereConsultaCrm = Cabe<ConsultaCrm, S['ConsultaCrm']>;
export type ConfereNovaConsultaCrm = Cabe<NovaConsultaCrm, S['NovaConsultaCrm']>;
export type ConfereCertificadoDoMedico = Cabe<CertificadoDoMedico, S['CertificadoDoMedico']>;
export type ConfereNovoCertificado = Cabe<NovoCertificado, S['NovoCertificado']>;

// Convênios e tabelas; check-up e pacotes.
export type ConfereTermosDoConvenio = Cabe<TermosDoConvenio, S['TermosDoConvenio']>;
export type ConfereConvenioResumo = Cabe<ConvenioResumo, S['ConvenioResumo']>;
export type ConfereVersaoDaTabelaDoConvenio = Cabe<VersaoDaTabelaDoConvenio, S['VersaoDaTabelaDoConvenio']>;
export type ConfereMudancaDePrecoDoConvenio = Cabe<MudancaDePrecoDoConvenio, S['MudancaDePrecoDoConvenio']>;
export type ConferePropostaDeTabela = Cabe<PropostaDeTabela, S['PropostaDeTabela']>;
export type ConfereRegraDeAutorizacaoDoExame = Cabe<RegraDeAutorizacaoDoExame, S['RegraDeAutorizacaoDoExame']>;
export type ConfereRegrasDeAutorizacaoDoConvenio = Cabe<RegrasDeAutorizacaoDoConvenio, S['RegrasDeAutorizacaoDoConvenio']>;
export type ConfereRegraDaAuditoriaDoConvenio = Cabe<RegraDaAuditoriaDoConvenio, S['RegraDaAuditoriaDoConvenio']>;
export type ConfereAuditoriaDoConvenio = Cabe<AuditoriaDoConvenio, S['AuditoriaDoConvenio']>;
export type ConfereMudancaDoConvenio = Cabe<MudancaDoConvenio, S['MudancaDoConvenio']>;
export type ConfereConvenioDetalhe = Cabe<ConvenioDetalhe, S['ConvenioDetalhe']>;
export type ConfereNovoConvenio = Cabe<NovoConvenio, S['NovoConvenio']>;
export type ConfereAlteracaoDoConvenio = Cabe<AlteracaoDoConvenio, S['AlteracaoDoConvenio']>;
export type ConferePrecosDoConvenio = Cabe<PrecosDoConvenio, S['PrecosDoConvenio']>;
export type ConferePreviaDeReajuste = Cabe<PreviaDeReajuste, S['PreviaDeReajuste']>;
export type ConfereNovaPropostaDeTabela = Cabe<NovaPropostaDeTabela, S['NovaPropostaDeTabela']>;
export type ConfereDecisaoDaPropostaDeTabela = Cabe<DecisaoDaPropostaDeTabela, S['DecisaoDaPropostaDeTabela']>;
export type ConfereNovasRegrasDeAutorizacao = Cabe<NovasRegrasDeAutorizacao, S['NovasRegrasDeAutorizacao']>;
export type ConfereVersaoDoPacoteDeCheckup = Cabe<VersaoDoPacoteDeCheckup, S['VersaoDoPacoteDeCheckup']>;
export type ConferePacoteDeCheckup = Cabe<PacoteDeCheckup, S['PacoteDeCheckup']>;
export type ConfereCatalogoDePacotes = Cabe<CatalogoDePacotes, S['CatalogoDePacotes']>;
export type ConfereNovoPacoteDeCheckup = Cabe<NovoPacoteDeCheckup, S['NovoPacoteDeCheckup']>;
export type ConfereNovaVersaoDoPacoteDeCheckup = Cabe<NovaVersaoDoPacoteDeCheckup, S['NovaVersaoDoPacoteDeCheckup']>;
export type ConfereNovoConvenioVolta = Cabe<S['NovoConvenio'], NovoConvenio>;
export type ConfereAlteracaoDoConvenioVolta = Cabe<S['AlteracaoDoConvenio'], AlteracaoDoConvenio>;
export type ConfereNovaPropostaDeTabelaVolta = Cabe<S['NovaPropostaDeTabela'], NovaPropostaDeTabela>;
export type ConfereDecisaoDaPropostaDeTabelaVolta = Cabe<S['DecisaoDaPropostaDeTabela'], DecisaoDaPropostaDeTabela>;
export type ConfereNovasRegrasDeAutorizacaoVolta = Cabe<S['NovasRegrasDeAutorizacao'], NovasRegrasDeAutorizacao>;
export type ConfereNovoPacoteDeCheckupVolta = Cabe<S['NovoPacoteDeCheckup'], NovoPacoteDeCheckup>;
export type ConfereNovaVersaoDoPacoteDeCheckupVolta = Cabe<S['NovaVersaoDoPacoteDeCheckup'], NovaVersaoDoPacoteDeCheckup>;
