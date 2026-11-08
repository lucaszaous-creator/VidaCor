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
import { TelaAutorizacaoDesconto } from './telas/caixa/TelaAutorizacaoDesconto';
import { TelaCaixa } from './telas/caixa/TelaCaixa';
import { TelaEstornos } from './telas/caixa/TelaEstornos';
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

export interface BuscaDaAutorizacao {
  paciente?: string;
  solicitante?: string;
  pontos?: number;
}

export function lerBuscaDaAutorizacao(busca: Record<string, unknown>): BuscaDaAutorizacao {
  const r: BuscaDaAutorizacao = {};
  const paciente = lerUuid(busca.paciente);
  const solicitante = lerUuid(busca.solicitante);
  if (paciente !== undefined) r.paciente = paciente;
  if (solicitante !== undefined) r.solicitante = solicitante;
  if (typeof busca.pontos === 'number' && Number.isInteger(busca.pontos) && busca.pontos > 0 && busca.pontos <= 10_000) {
    r.pontos = busca.pontos;
  }
  return r;
}

export interface BuscaDoEstorno {
  estorno?: string;
}

export function lerBuscaDoEstorno(busca: Record<string, unknown>): BuscaDoEstorno {
  const estorno = lerUuid(busca.estorno);
  return estorno === undefined ? {} : { estorno };
}

const raiz = createRootRouteWithContext<ContextoDoRoteador>()({ component: Outlet });

const rotaEntrar = createRoute({ getParentRoute: () => raiz, path: '/entrar', component: TelaEntrar });

const rotaApp = createRoute({
  getParentRoute: () => raiz,
  id: 'app',
  beforeLoad: async ({ context }) => {
    try {
      await context.queryClient.query(consultaSessao(context.dependencias.api));
    } catch (erro) {
      // Sem sessão vai para a entrada. Outros erros (servidor fora) a casca mostra no lugar.
      if (erro instanceof ErroDaApi && erro.status === 401) {
        // eslint-disable-next-line @typescript-eslint/only-throw-error -- padrão do TanStack Router
        throw redirect({ to: '/entrar' });
      }
    }
  },
  component: Casca,
});

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
const rotaSala = createRoute({ getParentRoute: () => rotaApp, path: '/sala', validateSearch: lerBuscaDaSala, component: TelaSala });
const rotaTriagem = createRoute({ getParentRoute: () => rotaApp, path: '/triagem/$atendimentoId', component: TelaTriagem });
const rotaFilaDeLaudos = createRoute({ getParentRoute: () => rotaApp, path: '/laudos', component: TelaFilaDeLaudos });
const rotaLaudo = createRoute({ getParentRoute: () => rotaApp, path: '/laudos/$laudoId', component: TelaLaudo });
const rotaCaixa = createRoute({ getParentRoute: () => rotaApp, path: '/caixa', validateSearch: lerBuscaDaAgenda, component: TelaCaixa });
const rotaAutorizacao = createRoute({
  getParentRoute: () => rotaApp,
  path: '/caixa/autorizacao-desconto',
  validateSearch: lerBuscaDaAutorizacao,
  component: TelaAutorizacaoDesconto,
});
const rotaEstornos = createRoute({ getParentRoute: () => rotaApp, path: '/caixa/estornos', validateSearch: lerBuscaDoEstorno, component: TelaEstornos });
const rotaEstudos = createRoute({ getParentRoute: () => rotaApp, path: '/estudos', validateSearch: lerBuscaDaAgenda, component: TelaEstudos });
const rotaContingencia = createRoute({ getParentRoute: () => rotaApp, path: '/contingencia', component: TelaContingencia });

/** Portal do paciente: sessão de paciente separada da equipe (docs/30 § 4), mesma aplicação e design system. */
const rotaPortal = createRoute({ getParentRoute: () => raiz, path: '/portal', component: CascaDoPortal });
const rotaPortalInicio = createRoute({ getParentRoute: () => rotaPortal, path: '/', component: TelaPortal });
const rotaPortalLaudo = createRoute({ getParentRoute: () => rotaPortal, path: '/laudos/$laudoId', component: TelaLaudoDoPortal });
const rotaPortalLink = createRoute({ getParentRoute: () => rotaPortal, path: '/link/$token', component: TelaLinkCompartilhado });

const arvore = raiz.addChildren([
  rotaEntrar,
  rotaPortal.addChildren([rotaPortalInicio, rotaPortalLaudo, rotaPortalLink]),
  rotaApp.addChildren([
    rotaInicio,
    rotaAgenda,
    rotaRecepcao,
    rotaNovoAgendamento,
    rotaSala,
    rotaTriagem,
    rotaFilaDeLaudos,
    rotaLaudo,
    rotaCaixa,
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
