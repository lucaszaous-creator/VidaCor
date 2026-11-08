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
import { consultaSessao } from './consultas';
import type { Dependencias } from './contexto';
import { TelaAgenda } from './telas/agenda/TelaAgenda';
import { TelaEntrar } from './telas/TelaEntrar';

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
  beforeLoad: () => {
    // eslint-disable-next-line @typescript-eslint/only-throw-error -- padrão do TanStack Router
    throw redirect({ to: '/agenda' });
  },
});

const rotaAgenda = createRoute({
  getParentRoute: () => rotaApp,
  path: '/agenda',
  validateSearch: lerBuscaDaAgenda,
  component: TelaAgenda,
});

const arvore = raiz.addChildren([rotaEntrar, rotaApp.addChildren([rotaInicio, rotaAgenda])]);

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
