import type { QueryClient } from '@tanstack/react-query';
import {
  createRootRouteWithContext,
  createRoute,
  createRouter,
  Outlet,
  redirect,
  type RouterHistory,
} from '@tanstack/react-router';
import { ErroDaApi } from '@vidacor/api-client';
import { Casca } from './casca/Casca';
import { telaInicial } from './casca/menu';
import { chaves, consultaSessao } from './consultas';
import type { Dependencias } from './contexto';
import { TelaNovoAgendamento } from './telas/agendamento/TelaNovoAgendamento';
import { TelaAgenda } from './telas/agenda/TelaAgenda';
import { TelaCadastroPaciente } from './telas/cadastro/TelaCadastroPaciente';
import { TelaAutorizacoes } from './telas/autorizacoes/TelaAutorizacoes';
import { TelaEstoque } from './telas/estoque/TelaEstoque';
import { TelaAuditoriaFaturamento } from './telas/faturamento/TelaAuditoriaFaturamento';
import { TelaFaturamento, type AbaDoFaturamento } from './telas/faturamento/TelaFaturamento';
import { TelaLastro } from './telas/lastro/TelaLastro';
import { TelaIndicadores } from './telas/indicadores/TelaIndicadores';
import { TelaFinanceiro } from './telas/financeiro/TelaFinanceiro';
import { TelaUsuarios } from './telas/usuarios/TelaUsuarios';
import { TelaAparelhos } from './telas/aparelhos/TelaAparelhos';
import { TelaFluxoDeLaudos } from './telas/fluxo/TelaFluxoDeLaudos';
import { TelaCorpoClinico } from './telas/corpo-clinico/TelaCorpoClinico';
import { TelaExames } from './telas/exames/TelaExames';
import { TelaPreparo } from './telas/preparo/TelaPreparo';
import { TelaModelosDeLaudo } from './telas/modelos/TelaModelosDeLaudo';
import { TelaRevisaoPorPares } from './telas/revisao/TelaRevisaoPorPares';
import { TelaConvenios } from './telas/convenios/TelaConvenios';
import { TelaPacotes } from './telas/pacotes/TelaPacotes';
import { TelaEtiquetas } from './telas/etiquetas/TelaEtiquetas';
import { TelaComunicacao } from './telas/comunicacao/TelaComunicacao';
import { TelaInicio } from './telas/inicio/TelaInicio';
import { TelaOuvidoria } from './telas/ouvidoria/TelaOuvidoria';
import { TelaQualidade } from './telas/qualidade/TelaQualidade';
import { TelaPaciente } from './telas/paciente/TelaPaciente';
import { TelaConfiguracoes } from './telas/configuracoes/TelaConfiguracoes';
import { TelaListaEspera } from './telas/espera/TelaListaEspera';
import { TelaOrcamento } from './telas/orcamento/TelaOrcamento';
import { TelaPainelDeChamada } from './telas/painel/TelaPainelDeChamada';
import { TelaExameExterno } from './telas/externo/TelaExameExterno';
import { TelaConvite } from './telas/portal/TelaConvite';
import { TelaEntrarNoPortal } from './telas/portal/TelaEntrarNoPortal';
import { TelaPendencias } from './telas/pendencias/TelaPendencias';
import { TelaCaixa } from './telas/caixa/TelaCaixa';
import { TelaContingencia } from './telas/contingencia/TelaContingencia';
import { TelaEstudos } from './telas/estudos/TelaEstudos';
import { TelaFilaDeLaudos } from './telas/laudos/TelaFilaDeLaudos';
import { TelaLaudo } from './telas/laudos/TelaLaudo';
import { CascaDoPortal } from './telas/portal/CascaDoPortal';
import { TelaLaudoDoPortal } from './telas/portal/TelaLaudoDoPortal';
import { TelaLinkCompartilhado } from './telas/portal/TelaLinkCompartilhado';
import { TelaPortal } from './telas/portal/TelaPortal';
import { TelaRecepcao } from './telas/recepcao/TelaRecepcao';
import { TelaSala } from './telas/sala/TelaSala';
import { TelaEntrar } from './telas/TelaEntrar';
import { TelaReautenticado } from './telas/TelaReautenticado';
import { TelaTriagem } from './telas/triagem/TelaTriagem';

export interface ContextoDoRoteador {
  queryClient: QueryClient;
  dependencias: Dependencias;
}

export interface BuscaDaAgenda {
  /** Dia mostrado, `AAAA-MM-DD`. Só data e sala vão na URL: nada de dado de paciente. */
  data?: string;
  sala?: string;
}

const DATA = /^\d{4}-\d{2}-\d{2}$/;

export function lerBuscaDaAgenda(busca: Record<string, unknown>): BuscaDaAgenda {
  const resultado: BuscaDaAgenda = {};
  if (typeof busca.data === 'string' && DATA.test(busca.data)) {
    resultado.data = busca.data;
  }
  if (typeof busca.sala === 'string' && busca.sala !== '') {
    resultado.sala = busca.sala;
  }
  return resultado;
}

/** Só identificadores técnicos (UUID) e números vão na URL: nada de dado de paciente. */
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function lerUuid(valor: unknown): string | undefined {
  return typeof valor === 'string' && UUID.test(valor) ? valor : undefined;
}

export interface BuscaDaSala {
  sala?: string;
}

export function lerBuscaDaSala(busca: Record<string, unknown>): BuscaDaSala {
  return typeof busca.sala === 'string' && busca.sala !== '' ? { sala: busca.sala } : {};
}


export interface BuscaDasPendencias {
  aba?: 'descontos' | 'estornos';
  id?: string;
}

export function lerBuscaDasPendencias(busca: Record<string, unknown>): BuscaDasPendencias {
  const r: BuscaDasPendencias = {};
  if (busca.aba === 'descontos' || busca.aba === 'estornos') r.aba = busca.aba;
  const id = lerUuid(busca.id);
  if (id !== undefined) r.id = id;
  return r;
}

export interface BuscaDoEstorno {
  estorno?: string;
}

export function lerBuscaDoEstorno(busca: Record<string, unknown>): BuscaDoEstorno {
  const estorno = lerUuid(busca.estorno);
  return estorno === undefined ? {} : { estorno };
}

export interface BuscaDoFaturamento {
  aba?: AbaDoFaturamento;
  id?: string;
}

export function lerBuscaDoFaturamento(busca: Record<string, unknown>): BuscaDoFaturamento {
  const r: BuscaDoFaturamento = {};
  if (busca.aba === 'guias' || busca.aba === 'lotes' || busca.aba === 'glosas') r.aba = busca.aba;
  const id = lerUuid(busca.id);
  if (id !== undefined) r.id = id;
  return r;
}

export function lerBuscaDasAutorizacoes(busca: Record<string, unknown>): { id?: string } {
  const id = lerUuid(busca.id);
  return id === undefined ? {} : { id };
}

const raiz = createRootRouteWithContext<ContextoDoRoteador>()({ component: Outlet });

const rotaEntrar = createRoute({ getParentRoute: () => raiz, path: '/entrar', component: TelaEntrar });
const rotaReautenticado = createRoute({ getParentRoute: () => raiz, path: '/reautenticado', component: TelaReautenticado });

async function exigirSessao({ context }: { context: ContextoDoRoteador }) {
  try {
    await context.queryClient.query(consultaSessao(context.dependencias.api));
  } catch (erro) {
    // Sem sessão vai para a entrada. Outros erros (servidor fora) a tela mostra no lugar.
    if (erro instanceof ErroDaApi && erro.status === 401) {
      // eslint-disable-next-line @typescript-eslint/only-throw-error -- padrão do TanStack Router
      throw redirect({ to: '/entrar' });
    }
  }
}

/** TV de chamada em tela cheia, sem menu (Telao.dc.html; conta do dispositivo TV, docs/18). */
const rotaPainel = createRoute({ getParentRoute: () => raiz, path: '/telao', beforeLoad: exigirSessao, component: TelaPainelDeChamada });

const rotaApp = createRoute({
  getParentRoute: () => raiz,
  id: 'app',
  beforeLoad: exigirSessao,
  component: Casca,
});

const rotaAparelhos = createRoute({ getParentRoute: () => rotaApp, path: '/aparelhos', component: TelaAparelhos });
const rotaCorpoClinico = createRoute({ getParentRoute: () => rotaApp, path: '/corpo-clinico', component: TelaCorpoClinico });
const rotaExames = createRoute({ getParentRoute: () => rotaApp, path: '/exames', component: TelaExames });
const rotaPreparo = createRoute({ getParentRoute: () => rotaApp, path: '/preparo', component: TelaPreparo });
const rotaConvenios = createRoute({ getParentRoute: () => rotaApp, path: '/convenios', component: TelaConvenios });
const rotaPacotes = createRoute({ getParentRoute: () => rotaApp, path: '/pacotes', component: TelaPacotes });
const rotaEtiquetas = createRoute({ getParentRoute: () => rotaApp, path: '/etiquetas', component: TelaEtiquetas });
const rotaComunicacao = createRoute({ getParentRoute: () => rotaApp, path: '/comunicacao', component: TelaComunicacao });
const rotaPaciente = createRoute({
  getParentRoute: () => rotaApp,
  path: '/pacientes',
  validateSearch: (b: Record<string, unknown>): { id?: string } => {
    const id = lerUuid(b.id);
    return id === undefined ? {} : { id };
  },
  component: TelaPaciente,
});
const rotaQualidade = createRoute({ getParentRoute: () => rotaApp, path: '/qualidade', component: TelaQualidade });
const rotaOuvidoria = createRoute({ getParentRoute: () => rotaApp, path: '/ouvidoria', component: TelaOuvidoria });
const rotaFluxoDeLaudos = createRoute({ getParentRoute: () => rotaApp, path: '/fluxo-de-laudos', component: TelaFluxoDeLaudos });
const rotaModelosDeLaudo = createRoute({ getParentRoute: () => rotaApp, path: '/modelos-de-laudo', component: TelaModelosDeLaudo });
const rotaRevisaoPorPares = createRoute({ getParentRoute: () => rotaApp, path: '/revisao-por-pares', component: TelaRevisaoPorPares });
const rotaPaginaInicial = createRoute({ getParentRoute: () => rotaApp, path: '/inicio', component: TelaInicio });
const rotaConfiguracoes = createRoute({ getParentRoute: () => rotaApp, path: '/configuracoes', component: TelaConfiguracoes });
const rotaListaEspera = createRoute({ getParentRoute: () => rotaApp, path: '/lista-espera', component: TelaListaEspera });
const rotaOrcamento = createRoute({ getParentRoute: () => rotaApp, path: '/orcamentos', component: TelaOrcamento });

const rotaInicio = createRoute({
  getParentRoute: () => rotaApp,
  path: '/',
  beforeLoad: ({ context }) => {
    const sessao = context.queryClient.getQueryData<{ usuario: { perfis: string[] } }>(chaves.sessao);
    // eslint-disable-next-line @typescript-eslint/only-throw-error -- padrão do TanStack Router
    throw redirect({ to: telaInicial(sessao?.usuario.perfis ?? []) });
  },
});

const rotaAgenda = createRoute({
  getParentRoute: () => rotaApp,
  path: '/agenda',
  validateSearch: lerBuscaDaAgenda,
  component: TelaAgenda,
});

const rotaRecepcao = createRoute({ getParentRoute: () => rotaApp, path: '/recepcao', validateSearch: lerBuscaDaAgenda, component: TelaRecepcao });
const rotaNovoAgendamento = createRoute({ getParentRoute: () => rotaApp, path: '/agendamentos/novo', component: TelaNovoAgendamento });
const rotaNovoPaciente = createRoute({ getParentRoute: () => rotaApp, path: '/pacientes/novo', component: TelaCadastroPaciente });
const rotaEstoque = createRoute({ getParentRoute: () => rotaApp, path: '/estoque', component: TelaEstoque });
const rotaLastro = createRoute({ getParentRoute: () => rotaApp, path: '/lastro', component: TelaLastro });
const rotaAutorizacoes = createRoute({ getParentRoute: () => rotaApp, path: '/autorizacoes', validateSearch: lerBuscaDasAutorizacoes, component: TelaAutorizacoes });
const rotaIndicadores = createRoute({ getParentRoute: () => rotaApp, path: '/indicadores', component: TelaIndicadores });
const rotaFinanceiro = createRoute({ getParentRoute: () => rotaApp, path: '/financeiro', component: TelaFinanceiro });
const rotaUsuarios = createRoute({ getParentRoute: () => rotaApp, path: '/usuarios', component: TelaUsuarios });
const rotaExameExterno = createRoute({ getParentRoute: () => rotaApp, path: '/exames-externos', component: TelaExameExterno });
const rotaFaturamento = createRoute({ getParentRoute: () => rotaApp, path: '/faturamento', validateSearch: lerBuscaDoFaturamento, component: TelaFaturamento });
const rotaAuditoriaFaturamento = createRoute({ getParentRoute: () => rotaApp, path: '/faturamento/auditoria', component: TelaAuditoriaFaturamento });
const rotaSala = createRoute({ getParentRoute: () => rotaApp, path: '/sala', validateSearch: lerBuscaDaSala, component: TelaSala });
const rotaTriagem = createRoute({ getParentRoute: () => rotaApp, path: '/triagem/$atendimentoId', component: TelaTriagem });
const rotaFilaDeLaudos = createRoute({ getParentRoute: () => rotaApp, path: '/laudos', component: TelaFilaDeLaudos });
const rotaLaudo = createRoute({ getParentRoute: () => rotaApp, path: '/laudos/$laudoId', component: TelaLaudo });
const rotaCaixa = createRoute({ getParentRoute: () => rotaApp, path: '/caixa', validateSearch: lerBuscaDaAgenda, component: TelaCaixa });
const rotaPendencias = createRoute({ getParentRoute: () => rotaApp, path: '/caixa/pendencias', validateSearch: lerBuscaDasPendencias, component: TelaPendencias });
// Endereços antigos (link do chat) levam às pendências: o pedido agora vem da fila do servidor.
const rotaAutorizacao = createRoute({
  getParentRoute: () => rotaApp,
  path: '/caixa/autorizacao-desconto',
  beforeLoad: () => {
    // eslint-disable-next-line @typescript-eslint/only-throw-error -- padrão do TanStack Router
    throw redirect({ to: '/caixa/pendencias', search: { aba: 'descontos' } });
  },
});
const rotaEstornos = createRoute({
  getParentRoute: () => rotaApp,
  path: '/caixa/estornos',
  validateSearch: lerBuscaDoEstorno,
  beforeLoad: ({ search }) => {
    // eslint-disable-next-line @typescript-eslint/only-throw-error -- padrão do TanStack Router
    throw redirect({ to: '/caixa/pendencias', search: { aba: 'estornos', ...(search.estorno === undefined ? {} : { id: search.estorno }) } });
  },
});
const rotaEstudos = createRoute({ getParentRoute: () => rotaApp, path: '/estudos', validateSearch: lerBuscaDaAgenda, component: TelaEstudos });
const rotaContingencia = createRoute({ getParentRoute: () => rotaApp, path: '/contingencia', component: TelaContingencia });

/** Portal do paciente: sessão de paciente separada da equipe (docs/30 § 4), mesma aplicação e design system. */
const rotaPortal = createRoute({ getParentRoute: () => raiz, path: '/portal', component: CascaDoPortal });
const rotaPortalInicio = createRoute({ getParentRoute: () => rotaPortal, path: '/', component: TelaPortal });
const rotaPortalLaudo = createRoute({ getParentRoute: () => rotaPortal, path: '/laudos/$laudoId', component: TelaLaudoDoPortal });
const rotaPortalLink = createRoute({ getParentRoute: () => rotaPortal, path: '/link/$token', component: TelaLinkCompartilhado });
const rotaPortalEntrar = createRoute({ getParentRoute: () => rotaPortal, path: '/entrar', component: TelaEntrarNoPortal });
const rotaPortalConvite = createRoute({ getParentRoute: () => rotaPortal, path: '/convite/$token', component: TelaConvite });

const arvore = raiz.addChildren([
  rotaEntrar,
  rotaReautenticado,
  rotaPainel,
  rotaPortal.addChildren([rotaPortalInicio, rotaPortalLaudo, rotaPortalLink, rotaPortalEntrar, rotaPortalConvite]),
  rotaApp.addChildren([
    rotaInicio,
    rotaOrcamento,
    rotaListaEspera,
    rotaConfiguracoes,
    rotaPaciente,
    rotaPaginaInicial,
    rotaFluxoDeLaudos,
    rotaOuvidoria,
    rotaQualidade,
    rotaModelosDeLaudo,
    rotaRevisaoPorPares,
    rotaAparelhos,
    rotaCorpoClinico,
    rotaExames,
    rotaPreparo,
    rotaConvenios,
    rotaPacotes,
    rotaEtiquetas,
    rotaComunicacao,
    rotaAgenda,
    rotaRecepcao,
    rotaNovoAgendamento,
    rotaSala,
    rotaTriagem,
    rotaFilaDeLaudos,
    rotaLaudo,
    rotaCaixa,
    rotaPendencias,
    rotaNovoPaciente,
    rotaEstoque,
    rotaLastro,
    rotaAutorizacoes,
    rotaIndicadores,
    rotaFinanceiro,
    rotaUsuarios,
    rotaExameExterno,
    rotaFaturamento,
    rotaAuditoriaFaturamento,
    rotaAutorizacao,
    rotaEstornos,
    rotaContingencia,
    rotaEstudos,
  ]),
]);

export function criarRoteador(contexto: ContextoDoRoteador, history?: RouterHistory) {
  return createRouter({
    routeTree: arvore,
    context: contexto,
    defaultPreload: false,
    ...(history === undefined ? {} : { history }),
  });
}

declare module '@tanstack/react-router' {
  interface Register {
    router: ReturnType<typeof criarRoteador>;
  }
}
