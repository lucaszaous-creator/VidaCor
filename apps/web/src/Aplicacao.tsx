import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider, type RouterHistory } from '@tanstack/react-router';
import { useState } from 'react';
import { ContextoDeDependencias, type Dependencias } from './contexto';
import { deveTentarDeNovo } from './erros';
import { criarRoteador } from './rotas';

export function criarQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: deveTentarDeNovo, refetchOnWindowFocus: true },
      mutations: { retry: false },
    },
  });
}

export interface PropsDaAplicacao {
  dependencias: Dependencias;
  queryClient?: QueryClient;
  history?: RouterHistory;
}

export function Aplicacao({ dependencias, queryClient, history }: PropsDaAplicacao) {
  const [qc] = useState(() => queryClient ?? criarQueryClient());
  const [roteador] = useState(() => criarRoteador({ queryClient: qc, dependencias }, history));
  return (
    <ContextoDeDependencias.Provider value={dependencias}>
      <QueryClientProvider client={qc}>
        <RouterProvider router={roteador} />
      </QueryClientProvider>
    </ContextoDeDependencias.Provider>
  );
}
